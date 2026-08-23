//! allowed_command re-resolves argv[0] on PATH itself (never trusts a path
//! string arriving over the socket) and checks the full argument list
//! against an exact whitelist -- no shell, no wildcards beyond the one
//! deliberately scoped case (AUR packages already built into the caller's
//! own yay/paru cache dir). Anything else is rejected.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn allowed_command(argv: &[String]) -> Option<Vec<String>> {
    if argv.is_empty() {
        return None;
    }
    let base = Path::new(&argv[0]).file_name()?.to_str()?.to_string();
    let args = &argv[1..];

    let path = which(&base)?;
    let path_str = path.to_string_lossy().into_owned();

    let resolved_args = match base.as_str() {
        "pacman" => allowed_pacman_args(args)?,
        "paccache" => paccache_ok(args)?,
        "apt-get" => apt_get_ok(args)?,
        "dnf" => dnf_ok(args)?,
        "zypper" => zypper_ok(args)?,
        // Exact commands sysup's own pipeline ever issues for the global
        // npm step -- not a general "npm can do anything as root" grant.
        "npm" => npm_ok(args)?,
        _ => return None,
    };

    Some(prepend(&path_str, &resolved_args))
}

fn prepend(path: &str, args: &[String]) -> Vec<String> {
    let mut v = Vec::with_capacity(args.len() + 1);
    v.push(path.to_string());
    v.extend(args.iter().cloned());
    v
}

fn args_equal(args: &[String], want: &[&str]) -> bool {
    args.len() == want.len() && args.iter().zip(want.iter()).all(|(a, w)| a == w)
}

fn paccache_ok(args: &[String]) -> Option<Vec<String>> {
    if args_equal(args, &["-r"]) {
        Some(args.to_vec())
    } else {
        None
    }
}

fn apt_get_ok(args: &[String]) -> Option<Vec<String>> {
    if args_equal(args, &["update"])
        || args_equal(args, &["full-upgrade", "-y"])
        || args_equal(args, &["autoremove", "-y"])
        || args_equal(args, &["autoclean", "-y"])
    {
        Some(args.to_vec())
    } else {
        None
    }
}

fn dnf_ok(args: &[String]) -> Option<Vec<String>> {
    if args_equal(args, &["upgrade", "-y"])
        || args_equal(args, &["autoremove", "-y"])
        || args_equal(args, &["clean", "all"])
    {
        Some(args.to_vec())
    } else {
        None
    }
}

fn zypper_ok(args: &[String]) -> Option<Vec<String>> {
    if args_equal(args, &["update", "-y"]) {
        Some(args.to_vec())
    } else {
        None
    }
}

fn npm_ok(args: &[String]) -> Option<Vec<String>> {
    if args_equal(args, &["install", "-g", "npm@latest"]) || args_equal(args, &["update", "-g"]) {
        Some(args.to_vec())
    } else {
        None
    }
}

/// which re-implements exec.LookPath's PATH search: no shell involved, just
/// a plain directory scan for an executable regular file.
pub fn which(cmd: &str) -> Option<PathBuf> {
    if cmd.contains('/') {
        let p = PathBuf::from(cmd);
        return if is_executable(&p) { Some(p) } else { None };
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(cmd);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && meta.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

/// pacman_flag_alias maps pacman's long-form flags to a canonical name also
/// reachable via the equivalent short-form letter (see parse_pacman_args) --
/// so "--sync -y -u --noconfirm" and "-Syu --noconfirm" resolve to the exact
/// same flag set. paru calls pacman with the long/split form; sysup's own
/// pipeline uses the short combined form -- both need to match the same
/// whitelist entry without listing every literal spelling.
fn pacman_flag_alias(flag: &str) -> Option<&'static str> {
    match flag {
        "--sync" => Some("S"),
        "--refresh" => Some("y"),
        "--sysupgrade" => Some("u"),
        "--remove" => Some("R"),
        "--nosave" => Some("n"),
        "--recursive" => Some("s"),
        "--upgrade" => Some("U"),
        "--noconfirm" => Some("noconfirm"),
        _ => None,
    }
}

/// PacmanRequest is argv[1:] decomposed into its flag set and non-flag
/// operands (package names, file paths) -- order-independent, so it doesn't
/// matter whether pacman was invoked with combined short flags, split short
/// flags, or long flags.
struct PacmanRequest {
    flags: HashSet<String>,
    operands: Vec<String>,
}

impl PacmanRequest {
    fn flags_only(&self, want: &[&str]) -> bool {
        if self.flags.len() != want.len() {
            return false;
        }
        want.iter().all(|f| self.flags.contains(*f))
    }
}

/// parse_pacman_args decodes args into flags+operands, trimming the single
/// trailing "--" separator paru always appends. Any token it doesn't
/// recognize (an unknown flag) fails closed -- returns None rather than
/// guessing, since a misparsed flag here would mean the whitelist check
/// below it is checking the wrong thing.
fn parse_pacman_args(args: &[String]) -> Option<PacmanRequest> {
    let mut args = args.to_vec();
    if args.last().map(String::as_str) == Some("--") {
        args.pop();
    }

    let mut req = PacmanRequest {
        flags: HashSet::new(),
        operands: Vec::new(),
    };

    for a in &args {
        if a.is_empty() {
            continue;
        }
        if !a.starts_with('-') {
            req.operands.push(a.clone());
            continue;
        }
        if let Some(canon) = pacman_flag_alias(a) {
            req.flags.insert(canon.to_string());
            continue;
        }
        let bytes = a.as_bytes();
        if a.len() > 1 && bytes[1] != b'-' {
            for ch in a[1..].chars() {
                match ch {
                    'S' | 'y' | 'u' | 'R' | 'n' | 's' | 'U' => {
                        req.flags.insert(ch.to_string());
                    }
                    _ => return None,
                }
            }
            continue;
        }
        return None;
    }
    Some(req)
}

/// allowed_pacman_args is the actual pacman whitelist: exactly "refresh the
/// databases and upgrade everything," "remove this exact list of orphan
/// packages," or "install this one AUR package already built into the
/// caller's own cache dir" -- nothing else. Returns the args pacman should
/// actually be invoked with (always the canonical short form, regardless of
/// which form the caller used) so the executed command line is predictable
/// and log-able, independent of how it arrived.
fn allowed_pacman_args(args: &[String]) -> Option<Vec<String>> {
    let req = parse_pacman_args(args)?;

    if req.flags_only(&["S", "y", "u", "noconfirm"]) && req.operands.is_empty() {
        return Some(vec!["-Syu".to_string(), "--noconfirm".to_string()]);
    }
    if req.flags_only(&["R", "n", "s", "noconfirm"]) && !req.operands.is_empty() {
        let mut out = vec!["-Rns".to_string(), "--noconfirm".to_string()];
        out.extend(req.operands.iter().cloned());
        return Some(out);
    }
    if req.flags_only(&["U", "noconfirm"])
        && req.operands.len() == 1
        && aur_cache_path_ok(&req.operands[0])
    {
        return Some(vec![
            "-U".to_string(),
            "--noconfirm".to_string(),
            req.operands[0].clone(),
        ]);
    }
    None
}

/// abs_clean mirrors Go's filepath.Abs+filepath.Clean: a purely lexical
/// join+normalize against the current directory, no symlink resolution and
/// no filesystem access -- so it works the same whether or not the path
/// actually exists.
fn abs_clean(path: &str) -> PathBuf {
    let p = Path::new(path);
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(p)
    };

    let mut out: Vec<std::ffi::OsString> = Vec::new();
    for comp in joined.components() {
        match comp {
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {}
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::Normal(s) => out.push(s.to_os_string()),
        }
    }
    let mut result = PathBuf::from("/");
    for c in out {
        result.push(c);
    }
    result
}

/// aur_cache_path_ok validates that path resolves inside ~/.cache/yay/ or
/// ~/.cache/paru/ of the UID polkit says actually authenticated
/// (PKEXEC_UID, set by polkit itself) -- never the caller-supplied
/// environment, which could otherwise be forged to point anywhere.
fn aur_cache_path_ok(path: &str) -> bool {
    let uid_str = match std::env::var("PKEXEC_UID") {
        Ok(s) if !s.is_empty() => s,
        _ => return false,
    };
    let uid: u32 = match uid_str.parse() {
        Ok(u) => u,
        Err(_) => return false,
    };
    let user = match nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(uid)) {
        Ok(Some(u)) => u,
        _ => return false,
    };

    let abs = abs_clean(path);
    for sub in ["yay", "paru"] {
        let root = abs_clean(&user.dir.join(".cache").join(sub).to_string_lossy());
        if abs == root || abs.starts_with(&root) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Guards every test that touches PKEXEC_UID / the current working
    // directory, since cargo test runs test fns concurrently within one
    // process and both are process-global state.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn pacman_full_sysupgrade_allowed() {
        assert_eq!(
            allowed_pacman_args(&s(&["-S", "-y", "-u", "--noconfirm"])),
            Some(s(&["-Syu", "--noconfirm"]))
        );
        assert_eq!(
            allowed_pacman_args(&s(&[
                "--sync",
                "--refresh",
                "--sysupgrade",
                "--noconfirm",
                "--"
            ])),
            Some(s(&["-Syu", "--noconfirm"]))
        );
    }

    #[test]
    fn pacman_full_sysupgrade_rejected_with_extra_operand() {
        assert_eq!(
            allowed_pacman_args(&s(&["-Syu", "--noconfirm", "extra-pkg"])),
            None
        );
    }

    #[test]
    fn pacman_orphan_removal_allowed() {
        assert_eq!(
            allowed_pacman_args(&s(&["-Rns", "--noconfirm", "orphan-a", "orphan-b"])),
            Some(s(&["-Rns", "--noconfirm", "orphan-a", "orphan-b"]))
        );
    }

    #[test]
    fn pacman_orphan_removal_rejected_without_operands() {
        assert_eq!(allowed_pacman_args(&s(&["-Rns", "--noconfirm"])), None);
    }

    #[test]
    fn pacman_aur_cache_install_and_traversal_rejection() {
        let _guard = ENV_LOCK.lock().unwrap();
        let uid = nix::unistd::getuid().as_raw().to_string();
        let home = nix::unistd::User::from_uid(nix::unistd::Uid::current())
            .unwrap()
            .unwrap()
            .dir;

        std::env::set_var("PKEXEC_UID", &uid);

        let good_path = home
            .join(".cache/paru/pkg-1.0.pkg.tar.zst")
            .to_string_lossy()
            .into_owned();
        assert!(aur_cache_path_ok(&good_path));
        assert_eq!(
            allowed_pacman_args(&s(&["-U", "--noconfirm", &good_path])),
            Some(vec![
                "-U".to_string(),
                "--noconfirm".to_string(),
                good_path.clone()
            ])
        );

        // Escapes the cache dir via ".." -- must resolve lexically to
        // outside ~/.cache/paru and be rejected.
        let escaping_path = home
            .join(".cache/paru/../../../etc/passwd")
            .to_string_lossy()
            .into_owned();
        assert!(!aur_cache_path_ok(&escaping_path));
        assert_eq!(
            allowed_pacman_args(&s(&["-U", "--noconfirm", &escaping_path])),
            None
        );

        std::env::remove_var("PKEXEC_UID");
    }

    #[test]
    fn pacman_aur_cache_install_rejected_without_pkexec_uid() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("PKEXEC_UID");
        assert!(!aur_cache_path_ok("/home/someone/.cache/paru/pkg.tar.zst"));
    }

    #[test]
    fn paccache_r_allowed() {
        assert_eq!(paccache_ok(&s(&["-r"])), Some(s(&["-r"])));
    }

    #[test]
    fn paccache_other_flag_rejected() {
        assert_eq!(paccache_ok(&s(&["-rk1"])), None);
    }

    #[test]
    fn apt_get_exact_invocations_allowed() {
        assert_eq!(apt_get_ok(&s(&["update"])), Some(s(&["update"])));
        assert_eq!(
            apt_get_ok(&s(&["full-upgrade", "-y"])),
            Some(s(&["full-upgrade", "-y"]))
        );
        assert_eq!(
            apt_get_ok(&s(&["autoremove", "-y"])),
            Some(s(&["autoremove", "-y"]))
        );
        assert_eq!(
            apt_get_ok(&s(&["autoclean", "-y"])),
            Some(s(&["autoclean", "-y"]))
        );
    }

    #[test]
    fn apt_get_arbitrary_invocation_rejected() {
        assert_eq!(apt_get_ok(&s(&["install", "-y", "netcat"])), None);
    }

    #[test]
    fn dnf_exact_invocations_allowed() {
        assert_eq!(dnf_ok(&s(&["upgrade", "-y"])), Some(s(&["upgrade", "-y"])));
        assert_eq!(
            dnf_ok(&s(&["autoremove", "-y"])),
            Some(s(&["autoremove", "-y"]))
        );
        assert_eq!(dnf_ok(&s(&["clean", "all"])), Some(s(&["clean", "all"])));
    }

    #[test]
    fn dnf_arbitrary_invocation_rejected() {
        assert_eq!(dnf_ok(&s(&["remove", "netcat"])), None);
    }

    #[test]
    fn zypper_exact_invocation_allowed() {
        assert_eq!(zypper_ok(&s(&["update", "-y"])), Some(s(&["update", "-y"])));
    }

    #[test]
    fn zypper_arbitrary_invocation_rejected() {
        assert_eq!(zypper_ok(&s(&["dup", "-y"])), None);
    }

    #[test]
    fn npm_exact_invocations_allowed() {
        assert_eq!(
            npm_ok(&s(&["install", "-g", "npm@latest"])),
            Some(s(&["install", "-g", "npm@latest"]))
        );
        assert_eq!(npm_ok(&s(&["update", "-g"])), Some(s(&["update", "-g"])));
    }

    #[test]
    fn npm_arbitrary_invocation_rejected() {
        assert_eq!(npm_ok(&s(&["install", "-g", "some-other-package"])), None);
    }

    #[test]
    fn allowed_command_end_to_end_resolves_pacman_on_path() {
        let resolved = allowed_command(&s(&["pacman", "-Syu", "--noconfirm"]))
            .expect("pacman is present on this system's PATH");
        assert!(resolved[0].ends_with("/pacman"));
        assert_eq!(&resolved[1..], &s(&["-Syu", "--noconfirm"])[..]);
    }

    #[test]
    fn allowed_command_rejects_unknown_tool() {
        assert_eq!(allowed_command(&s(&["rm", "-rf", "/"])), None);
    }

    #[test]
    fn allowed_command_rejects_empty_argv() {
        assert_eq!(allowed_command(&[]), None);
    }
}
