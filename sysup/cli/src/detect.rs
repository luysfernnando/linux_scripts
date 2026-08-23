// Figures out the OS/distro family and which optional external tools are
// present — used to decide which pipeline steps apply to this machine.
// Leaf module: no internal deps.

use std::fs;
use std::io::BufRead;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Arch,
    Debian,
    Fedora,
    Suse,
    Darwin,
    Windows,
    Unknown,
}

pub fn detect_family() -> Family {
    if cfg!(target_os = "macos") {
        return Family::Darwin;
    }
    if cfg!(target_os = "windows") {
        return Family::Windows;
    }
    if cfg!(target_os = "linux") {
        return detect_linux_family("/etc/os-release");
    }
    Family::Unknown
}

fn detect_linux_family(os_release_path: &str) -> Family {
    let file = match fs::File::open(os_release_path) {
        Ok(f) => f,
        Err(_) => return Family::Unknown,
    };

    let mut id = String::new();
    let mut id_like = String::new();
    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        if let Some(rest) = line.strip_prefix("ID=") {
            id = unquote(rest);
        } else if let Some(rest) = line.strip_prefix("ID_LIKE=") {
            id_like = unquote(rest);
        }
    }

    let haystack = format!("{id} {id_like}");
    if contains_any(&haystack, &["arch", "cachyos", "manjaro", "endeavouros"]) {
        Family::Arch
    } else if contains_any(&haystack, &["debian", "ubuntu", "mint", "pop"]) {
        Family::Debian
    } else if contains_any(&haystack, &["fedora", "rhel", "centos"]) {
        Family::Fedora
    } else if contains_any(&haystack, &["suse"]) {
        Family::Suse
    } else {
        Family::Unknown
    }
}

/// WSL never assigns a systemd-logind seat (`loginctl` shows an empty
/// `Seat:`), which leaves polkit's interactive auth broken even when the
/// session itself is active, D-Bus is up, and plain `sudo` works fine —
/// pkexec prompts for the password, then unconditionally reports "Not
/// authorized" regardless of the password being correct. Detected so
/// `polkit::run_setup` can warn before the user hits that wall.
pub fn is_wsl() -> bool {
    if std::env::var_os("WSL_DISTRO_NAME").is_some() || std::env::var_os("WSL_INTEROP").is_some() {
        return true;
    }
    fs::read_to_string("/proc/version").is_ok_and(|v| v.to_ascii_lowercase().contains("microsoft"))
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

pub fn has_tool(name: &str) -> bool {
    which::which(name)
        .map(|path| !is_wsl_interop_path(&path))
        .unwrap_or(false)
}

// WSL mounts the Windows filesystem under /mnt/<drive-letter>/ — a tool
// resolved there (e.g. npm/composer installed only on the Windows side) is
// a Windows executable reachable via WSL interop, not a real Linux binary.
// It works fine in an interactive user shell but is never invokable
// through `sudo` (secure_path never includes /mnt/*), so treating it as
// "present" would make the pipeline detect a tool it can never actually
// run as root — exactly the "sudo: npm: command not found" failure this
// guards against.
fn is_wsl_interop_path(path: &std::path::Path) -> bool {
    use std::path::Component;
    let mut components = path.components();
    matches!(components.next(), Some(Component::RootDir))
        && matches!(components.next(), Some(Component::Normal(c)) if c == "mnt")
        && matches!(components.next(), Some(Component::Normal(c)) if c.len() == 1)
}

// Bundles the presence checks the pipeline needs, computed once per run.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tools {
    pub yay: bool,
    pub paru: bool,
    pub pacman: bool,
    pub apt: bool,
    pub dnf: bool,
    pub zypper: bool,
    pub brew: bool,
    pub flatpak: bool,
    pub composer: bool,
    pub npm: bool,
    pub bun: bool,
    pub fwupdmgr: bool,
    pub choco: bool,
    pub winget: bool,
    pub reflector: bool,
    pub rate_mirror: bool, // cachyos-rate-mirrors
    pub notify_send: bool,
    pub pkexec: bool,
    pub paccache: bool, // pacman-contrib
}

pub fn detect_tools() -> Tools {
    Tools {
        yay: has_tool("yay"),
        paru: has_tool("paru"),
        pacman: has_tool("pacman"),
        apt: has_tool("apt-get"),
        dnf: has_tool("dnf"),
        zypper: has_tool("zypper"),
        brew: has_tool("brew"),
        flatpak: has_tool("flatpak"),
        composer: has_tool("composer"),
        npm: has_tool("npm"),
        bun: has_tool("bun"),
        fwupdmgr: has_tool("fwupdmgr"),
        choco: has_tool("choco"),
        winget: has_tool("winget"),
        reflector: has_tool("reflector"),
        rate_mirror: has_tool("cachyos-rate-mirrors"),
        notify_send: has_tool("notify-send"),
        pkexec: has_tool("pkexec"),
        paccache: has_tool("paccache"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn rejects_windows_interop_paths() {
        assert!(is_wsl_interop_path(Path::new(
            "/mnt/c/Program Files/nodejs/npm"
        )));
        assert!(is_wsl_interop_path(Path::new("/mnt/d/tools/npm")));
    }

    #[test]
    fn accepts_real_linux_paths() {
        assert!(!is_wsl_interop_path(Path::new("/usr/bin/npm")));
        assert!(!is_wsl_interop_path(Path::new("/home/user/.local/bin/npm")));
        // "/mnt/something-else" with a multi-char segment isn't a WSL
        // drive mount (e.g. a manually mounted /mnt/data).
        assert!(!is_wsl_interop_path(Path::new("/mnt/data/npm")));
    }
}
