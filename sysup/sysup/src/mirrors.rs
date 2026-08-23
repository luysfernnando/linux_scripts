// Ranks package-manager mirrors (reflector/rate-mirrors on Arch, apt-select
// on Debian) and tracks when the last check ran so it only happens roughly
// weekly.

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::detect::{self, Family, Tools};
use crate::style;

const MIRROR_CHECK_INTERVAL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

fn state_dir() -> anyhow::Result<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(xdg).join("sysup"));
    }
    let home = std::env::home_dir().ok_or_else(|| anyhow::anyhow!("$HOME não definido"))?;
    Ok(home.join(".local").join("state").join("sysup"))
}

fn mirror_state_file() -> anyhow::Result<PathBuf> {
    Ok(state_dir()?.join("last-mirror-check"))
}

// DueForCheck reports whether it's been MIRROR_CHECK_INTERVAL (or more, or
// ever) since the last mirror ranking. Any read/parse error is treated as
// "yes, check now" — a missing or corrupt state file shouldn't block us.
pub fn due_for_check() -> bool {
    let Ok(path) = mirror_state_file() else {
        return true;
    };
    let Ok(data) = std::fs::read_to_string(&path) else {
        return true;
    };
    let Ok(unix_secs) = data.trim().parse::<u64>() else {
        return true;
    };
    let last = UNIX_EPOCH + Duration::from_secs(unix_secs);
    match SystemTime::now().duration_since(last) {
        Ok(elapsed) => elapsed >= MIRROR_CHECK_INTERVAL,
        Err(_) => true,
    }
}

fn record_mirror_check() -> anyhow::Result<()> {
    let dir = state_dir()?;
    std::fs::create_dir_all(&dir)?;
    let path = mirror_state_file()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    std::fs::write(path, now.to_string())?;
    Ok(())
}

// mirror_country reads an optional per-machine country filter for mirror
// ranking from ~/.config/sysup/mirror-country (comma-separated, e.g.
// "Brazil,Argentina,Chile" — same spelling reflector's --country expects).
// Unset by default: sysup has no way to auto-detect location, and this repo
// targets any system, so we never hardcode a region here. Not part of the
// repo — it's a local, per-machine preference, same as repo-path.
fn mirror_country() -> String {
    let Some(home) = std::env::home_dir() else {
        return String::new();
    };
    std::fs::read_to_string(home.join(".config").join("sysup").join("mirror-country"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

// shell_single_quote wraps s so it's passed through `sh -c` as one literal
// argument, regardless of spaces or shell metacharacters in it.
fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

// RankCommand returns the shell command that best ranks mirrors for this
// family/toolset, or "" if nothing suitable is installed.
pub fn rank_command(family: &Family, t: &Tools) -> String {
    match family {
        Family::Arch => {
            if t.rate_mirror {
                // cachyos-rate-mirrors auto-detects region on its own; no
                // country filter needed.
                "sudo cachyos-rate-mirrors".to_string()
            } else if t.reflector {
                // --protocol https: pacman only speaks http(s)/ftp — without
                // this, reflector happily fills the list with rsync://
                // mirrors that pacman can never actually download from (dead
                // weight it still has to probe every run).
                // --download-timeout keeps a single slow mirror from
                // stalling the whole ranking.
                let mut cmd =
                    "sudo reflector --protocol https --latest 20 --sort rate --download-timeout 8"
                        .to_string();
                let country = mirror_country();
                if !country.is_empty() {
                    cmd.push_str(" --country ");
                    cmd.push_str(&shell_single_quote(&country));
                }
                cmd.push_str(" --save /etc/pacman.d/mirrorlist");
                cmd
            } else {
                String::new()
            }
        }
        Family::Debian => {
            if detect::has_tool("apt-select") {
                "apt-select -t 5 && sudo mv sources.list /etc/apt/sources.list".to_string()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

// RunCheck ranks mirrors if a command is available for this system, then
// always stamps the state file (even when there's nothing to do) so we
// don't keep retrying every single run on machines with no ranking tool.
pub fn run_check(
    family: &Family,
    t: &Tools,
    dry_run: bool,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let cmd = rank_command(family, t);
    if cmd.is_empty() {
        writeln!(
            out,
            "{}",
            style::dim("==> nenhuma ferramenta de ranking de mirrors encontrada, pulando")
        )?;
        if !dry_run {
            return record_mirror_check();
        }
        return Ok(());
    }

    // reflector logs one WARNING line per unreachable mirror it probes — on
    // a bad network that's dozens of lines of noise for something that's
    // expected and self-correcting. Collapse them into a single count.
    let result = if cmd.contains("reflector") {
        let mut filter = MirrorNoiseFilter::new(out);
        let r = crate::pipeline::run_shell(dry_run, &cmd, &mut filter);
        filter.flush()?;
        r
    } else {
        crate::pipeline::run_shell(dry_run, &cmd, out)
    };
    result?;

    if dry_run {
        return Ok(());
    }
    record_mirror_check()
}

// MirrorNoiseFilter drops reflector's per-mirror "WARNING: failed to rate
// ..." lines and reports a single summary count instead of letting them
// flood the terminal.
struct MirrorNoiseFilter<'a> {
    out: &'a mut dyn Write,
    buf: Vec<u8>,
    suppressed: usize,
}

impl<'a> MirrorNoiseFilter<'a> {
    fn new(out: &'a mut dyn Write) -> Self {
        Self {
            out,
            buf: Vec::new(),
            suppressed: 0,
        }
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        if self.suppressed > 0 {
            writeln!(
                self.out,
                "{}",
                style::dim(&format!(
                    "   ({} mirrors indisponíveis/timeout, ignorados)",
                    self.suppressed
                ))
            )?;
        }
        Ok(())
    }
}

impl Write for MirrorNoiseFilter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(buf);
        while let Some(i) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=i).collect();
            let line = &line[..line.len() - 1];
            if line
                .windows(b"WARNING: failed to rate".len())
                .any(|w| w == b"WARNING: failed to rate")
            {
                self.suppressed += 1;
                continue;
            }
            self.out.write_all(line)?;
            self.out.write_all(b"\n")?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.out.flush()
    }
}
