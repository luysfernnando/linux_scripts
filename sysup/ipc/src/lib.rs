//! Wire protocol between sysup (or sysup-authbridge) and sysup-worker: one
//! JSON request line naming the exact command to run, followed by the
//! command's raw combined stdout/stderr, then a trailer line marking
//! success/failure.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;

/// Per-user runtime location of the worker's Unix socket. `$XDG_RUNTIME_DIR`
/// is systemd-logind's 0700-per-user directory — the socket inherits that as
/// its only access control, appropriate for a personal single-user machine,
/// not a shared/multi-tenant one.
pub fn socket_path() -> PathBuf {
    let dir = match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => {
            let uid = unsafe { libc::getuid() };
            let dir = std::env::temp_dir().join(format!("sysup-{uid}"));
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
            dir
        }
    };
    dir.join("sysup-worker.sock")
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(serde::Serialize, serde::Deserialize)]
struct Request {
    argv: Vec<String>,
}

/// Sends one command (already split into argv form, no shell involved
/// anywhere in this protocol) as a single JSON line.
pub fn write_request<W: Write>(mut w: W, argv: &[String]) -> io::Result<()> {
    let req = Request {
        argv: argv.to_vec(),
    };
    let mut line = serde_json::to_vec(&req).map_err(io::Error::other)?;
    line.push(b'\n');
    w.write_all(&line)
}

/// Reads back what write_request sent.
pub fn read_request<R: BufRead>(mut r: R) -> io::Result<Vec<String>> {
    let mut line = String::new();
    r.read_line(&mut line)?;
    let req: Request = serde_json::from_str(&line).map_err(io::Error::other)?;
    Ok(req.argv)
}

/// A NUL byte essentially never appears in legitimate pacman/apt/dnf/paccache
/// output, so it's a safe-enough sentinel for this local, single-purpose
/// socket — this isn't a general-purpose framing protocol.
const TRAILER_MARKER: u8 = 0x00;

/// Distinguishes WHY a request didn't succeed, because that changes what's
/// safe for a caller to do next:
/// - `Rejected`: the worker never ran anything (bad request, or the argv
///   didn't match the whitelist) — safe to retry some other way (e.g.
///   sysup-authbridge falling back to real sudo).
/// - `Failed`: the worker DID execute the command and it returned an error —
///   retrying via another path would risk re-running something with side
///   effects (installing/removing packages twice). Callers must surface this
///   as a hard failure, never silently retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Rejected,
    Failed,
}

impl Status {
    fn to_byte(self) -> u8 {
        match self {
            Status::Ok => b'O',
            Status::Rejected => b'R',
            Status::Failed => b'F',
        }
    }

    fn from_byte(b: u8) -> Option<Status> {
        match b {
            b'O' => Some(Status::Ok),
            b'R' => Some(Status::Rejected),
            b'F' => Some(Status::Failed),
            _ => None,
        }
    }
}

/// Sends the final status line.
pub fn write_trailer<W: Write>(mut w: W, status: Status, msg: &str) -> io::Result<()> {
    writeln!(
        w,
        "{}{}{}",
        TRAILER_MARKER as char,
        status.to_byte() as char,
        msg
    )
}

/// Copies src to dst until the trailer marker shows up, then returns the
/// trailer's status/message. Used by both client sides to turn the raw byte
/// stream back into "did it work, and is it safe to retry."
pub fn relay_output<W: Write, R: Read>(mut dst: W, src: R) -> io::Result<(Status, String)> {
    let mut r = BufReader::new(src);
    let mut buf = [0u8; 4096];
    loop {
        let n = r.read(&mut buf)?;
        if n > 0 {
            let chunk = &buf[..n];
            if let Some(idx) = chunk.iter().position(|&b| b == TRAILER_MARKER) {
                if idx > 0 {
                    dst.write_all(&chunk[..idx])?;
                }
                let mut tail = chunk[idx + 1..].to_vec();
                let mut line = Vec::new();
                r.read_until(b'\n', &mut line)?;
                tail.extend_from_slice(&line);
                while tail.last() == Some(&b'\n') {
                    tail.pop();
                }
                if tail.is_empty() {
                    return Err(io::Error::other("trailer de status vazio"));
                }
                let status = Status::from_byte(tail[0])
                    .ok_or_else(|| io::Error::other("trailer de status invalido"))?;
                let msg = String::from_utf8_lossy(&tail[1..]).into_owned();
                return Ok((status, msg));
            }
            dst.write_all(chunk)?;
        } else {
            return Err(io::Error::other(
                "conexão do worker encerrada sem trailer de status",
            ));
        }
    }
}
