//! sysup-worker is the privileged half of sysup's polkit integration. It
//! has two personalities, chosen by the name it's invoked as (argv[0]) --
//! see sysup/README.md for the full architecture:
//!
//!   - sysup-worker: run via `pkexec` as root. Binds a Unix socket,
//!     re-detects the machine's family/tools itself (never trusts the
//!     caller), and executes only an exact whitelist of update/cleanup
//!     commands received over that socket. Lives only for the duration of
//!     one `sysup update` run -- its lifetime is tied to its inherited
//!     stdin pipe, not installed as any kind of persistent service.
//!   - sysup-authbridge: a symlink to this same binary, run unprivileged as
//!     paru's configured `[bin] Sudo` replacement. Forwards the exact argv
//!     paru would have handed to `sudo` over to the already-authorized
//!     worker socket, so paru's own privilege escalation never prompts.

mod bridge;
mod server;
mod whitelist;

fn main() {
    let arg0 = std::env::args().next().unwrap_or_default();
    let base = std::path::Path::new(&arg0)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if base == "sysup-authbridge" {
        bridge::main();
        return;
    }
    server::main();
}
