//! Runs unprivileged, as whoever paru is running as. It exists purely to
//! satisfy paru's expectation of a synchronous "sudo replacement" binary:
//! read $SYSUP_WORKER_SOCKET, forward argv, relay output/exit code.
//!
//! This is paru's ONLY privilege-escalation path once `sysup polkit-setup`
//! points paru.conf's [bin] Sudo at it -- including when paru is run by
//! hand, completely outside `sysup update` (no SYSUP_WORKER_SOCKET set, no
//! worker running). So whenever the worker isn't reachable, or rejects the
//! request before running anything, this falls back to real sudo -- paru
//! must keep working normally on its own, not only inside a sysup run.

use std::os::unix::net::UnixStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use ipc::Status;

use crate::whitelist::which;

pub fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    if let Ok(sock) = std::env::var("SYSUP_WORKER_SOCKET") {
        if !sock.is_empty() && try_worker(&sock, &argv) {
            return;
        }
    }
    exec_real_sudo(&argv);
}

/// tryWorker attempts the request against the worker. Returns true if it
/// handled the request start to finish (success or a real execution
/// failure -- either way, nothing left to fall back to). Returns false only
/// when nothing ran yet (connection failed, or the worker rejected the
/// request outright) -- safe for the caller to retry via sudo.
fn try_worker(sock: &str, argv: &[String]) -> bool {
    let (tx, rx) = mpsc::channel();
    let sock_owned = sock.to_string();
    thread::spawn(move || {
        let _ = tx.send(UnixStream::connect(sock_owned));
    });
    let conn = match rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(conn)) => conn,
        _ => return false,
    };

    if ipc::write_request(&conn, argv).is_err() {
        return false;
    }

    let (status, msg) = match ipc::relay_output(std::io::stdout(), &conn) {
        Ok(result) => result,
        Err(_) => return false,
    };

    match status {
        Status::Ok => true,
        Status::Failed => {
            // The worker DID run this and it failed -- surfacing it as-is
            // and stopping here, never falling back, since retrying via
            // sudo would re-run a command that may have already had side
            // effects.
            eprintln!("sysup-authbridge: {msg}");
            std::process::exit(1);
        }
        Status::Rejected => false,
    }
}

/// execRealSudo is the plain fallback: run the exact argv paru handed us
/// through the system's real sudo, prompting normally if needed. Resolved
/// fresh via PATH (never assumes a path), since this binary IS what
/// paru.conf's `Sudo =` now points to -- it must never call itself.
fn exec_real_sudo(argv: &[String]) -> ! {
    use std::os::unix::process::CommandExt;

    let sudo_path = match which("sudo") {
        Some(p) => p,
        None => {
            eprintln!("sysup-authbridge: sudo não encontrado no PATH");
            std::process::exit(1);
        }
    };

    let err = std::process::Command::new(sudo_path).args(argv).exec();
    eprintln!("sysup-authbridge: {err}");
    std::process::exit(1);
}
