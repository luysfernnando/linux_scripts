//! Runs as root (via pkexec). It never trusts anything about the caller's
//! environment beyond PKEXEC_UID (set by polkit itself, not the caller's
//! own env) and only ever executes commands it independently resolves and
//! validates against whitelist::allowed_command.

use std::io::{BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use sysup_workerproto::Status;

use crate::whitelist::allowed_command;

pub fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut socket_path = sysup_workerproto::socket_path();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--socket" && i + 1 < args.len() {
            socket_path = PathBuf::from(&args[i + 1]);
            i += 1;
        }
        i += 1;
    }

    if let Err(e) = std::fs::remove_file(&socket_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("sysup-worker: não consegui limpar socket antigo: {e}");
            std::process::exit(1);
        }
    }

    let listener = match UnixListener::bind(&socket_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("sysup-worker: bind falhou: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))
    {
        eprintln!("sysup-worker: chmod do socket falhou: {e}");
        std::process::exit(1);
    }

    // The socket is created by us while running as root, so it starts out
    // root:root -- 0600 in that state means only root could ever connect,
    // locking out the very user pkexec authorized this for. Chown it to
    // PKEXEC_UID (set by polkit itself, not the caller) so "owner-only"
    // actually means the real user, not root; root can still connect
    // regardless, since the kernel exempts root from file permission
    // checks entirely.
    if let Ok(uid_str) = std::env::var("PKEXEC_UID") {
        if !uid_str.is_empty() {
            if let Ok(uid_num) = uid_str.parse::<u32>() {
                if let Ok(Some(u)) =
                    nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(uid_num))
                {
                    if let Err(e) = nix::unistd::chown(&socket_path, Some(u.uid), Some(u.gid)) {
                        eprintln!("sysup-worker: chown do socket falhou: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    // Tie our lifetime to the parent sysup process: it holds the write end
    // of a pipe wired to our stdin for exactly as long as `sysup update`
    // runs. When it exits (normally or crashed), we get EOF here and shut
    // down -- this worker never outlives a single run. The timer is a
    // defense-in-depth fallback only, in case the pipe is somehow never
    // closed (e.g. spawned by hand for testing).
    let (done_tx, done_rx) = mpsc::channel::<()>();
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 4096];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        let _ = done_tx.send(());
    });
    thread::spawn(move || {
        let _ = done_rx.recv_timeout(Duration::from_secs(15 * 60));
        std::process::exit(0);
    });

    println!("READY");
    let _ = std::io::stdout().flush();

    for stream in listener.incoming() {
        match stream {
            Ok(conn) => {
                thread::spawn(move || handle_conn(conn));
            }
            Err(_) => return,
        }
    }
}

fn handle_conn(conn: UnixStream) {
    let mut reader = BufReader::new(&conn);
    let argv = match sysup_workerproto::read_request(&mut reader) {
        Ok(argv) => argv,
        Err(e) => {
            let _ = sysup_workerproto::write_trailer(
                &conn,
                Status::Rejected,
                &format!("pedido inválido: {e}"),
            );
            return;
        }
    };

    let resolved = match allowed_command(&argv) {
        Some(resolved) => resolved,
        None => {
            let _ = sysup_workerproto::write_trailer(
                &conn,
                Status::Rejected,
                &format!("comando não permitido: {}", argv.join(" ")),
            );
            return;
        }
    };

    let stdout_fd = match conn.try_clone() {
        Ok(c) => c,
        Err(e) => {
            let _ = sysup_workerproto::write_trailer(&conn, Status::Failed, &e.to_string());
            return;
        }
    };
    let stderr_fd = match conn.try_clone() {
        Ok(c) => c,
        Err(e) => {
            let _ = sysup_workerproto::write_trailer(&conn, Status::Failed, &e.to_string());
            return;
        }
    };

    let status = std::process::Command::new(&resolved[0])
        .args(&resolved[1..])
        .stdout(Stdio::from(std::os::fd::OwnedFd::from(stdout_fd)))
        .stderr(Stdio::from(std::os::fd::OwnedFd::from(stderr_fd)))
        .status();

    match status {
        Ok(status) if status.success() => {
            let _ = sysup_workerproto::write_trailer(&conn, Status::Ok, "");
        }
        Ok(status) => {
            let msg = match status.code() {
                Some(code) => format!("exit status {code}"),
                None => "exit status desconhecido (sinal)".to_string(),
            };
            let _ = sysup_workerproto::write_trailer(&conn, Status::Failed, &msg);
        }
        Err(e) => {
            let _ = sysup_workerproto::write_trailer(&conn, Status::Failed, &e.to_string());
        }
    }
}
