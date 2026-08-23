// Handles sysup replacing its own running binary from the latest GitHub
// release, plus the best-effort dotfiles repo git-pull that piggybacks on
// the same "check GitHub" moment.

use anyhow::{anyhow, Context};
use serde::Deserialize;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::style;

// repoSlug identifies the GitHub repo release assets are fetched from —
// duplicated as a tiny local constant here and in polkit rather than
// shared, since it's a single string literal.
const REPO_SLUG: &str = "luysfernnando/linux_scripts";

// VERSION is stamped at release-build time (the Go build used
// `-ldflags "-X ...selfupdate.Version=vX.Y.Z"` from the git tag; Cargo has
// no direct equivalent, so this reads an env var a release build can set
// via `SYSUP_VERSION=vX.Y.Z cargo build`, falling back to Cargo's own
// package version). A "dev" value means a locally-built binary — never
// self-update those, there is no meaningful "newer" to compare against.
pub static VERSION: &str = match option_env!("SYSUP_VERSION") {
    Some(v) => v,
    None => "dev",
};

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn target_os() -> &'static str {
    // GoReleaser's name_template uses Go's GOOS spelling ("darwin", not
    // "macos"), so map Rust's std::env::consts::OS to match the asset names
    // actually published in releases.
    match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    }
}

fn target_arch() -> &'static str {
    // Likewise, GOARCH spells x86_64/aarch64 as "amd64"/"arm64".
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    }
}

fn fetch_latest_release() -> anyhow::Result<Release> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("sysup-selfupdate")
        .build()?;
    let url = format!("https://api.github.com/repos/{REPO_SLUG}/releases/latest");
    let resp = client.get(url).send()?;
    if !resp.status().is_success() {
        return Err(anyhow!("GitHub releases/latest: status {}", resp.status()));
    }
    Ok(resp.json()?)
}

// SelfUpdate checks the latest GitHub release against the running version
// and, if newer, downloads+verifies+replaces the current executable. Any
// failure (offline, GitHub down, rate-limited) is swallowed as a warning —
// this must never be the reason `sysup update` aborts.
//
// Returns true if the binary was replaced (caller should re_exec).
pub fn self_update(dry_run: bool) -> bool {
    if VERSION == "dev" {
        return false;
    }
    if dry_run {
        println!(
            "==> self-update: checando última release do GitHub (dry-run, não substitui nada)"
        );
        return false;
    }

    let current = match semver::Version::parse(VERSION.trim_start_matches('v')) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "{}",
                style::warn(&format!(
                    "aviso: versão embutida inválida, pulando self-update: {e}"
                ))
            );
            return false;
        }
    };

    let release = match fetch_latest_release() {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "{}",
                style::warn(&format!(
                    "aviso: self-update falhou ao consultar GitHub: {e}"
                ))
            );
            return false;
        }
    };

    let latest = match semver::Version::parse(release.tag_name.trim_start_matches('v')) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "{}",
                style::warn(&format!(
                    "aviso: self-update falhou, versão da release inválida: {e}"
                ))
            );
            return false;
        }
    };
    if latest <= current {
        return false;
    }

    match do_update(&release, &latest) {
        Ok(()) => {
            println!(
                "{}",
                style::header(&format!("==> sysup atualizado: {} -> {}", VERSION, latest))
            );
            true
        }
        Err(e) => {
            eprintln!(
                "{}",
                style::warn(&format!(
                    "aviso: self-update falhou, seguindo com a versão atual: {e}"
                ))
            );
            false
        }
    }
}

fn do_update(release: &Release, latest: &semver::Version) -> anyhow::Result<()> {
    let os = target_os();
    let arch = target_arch();

    // Filter required: assets are matched by an exact "sysup_<os>_<arch>"
    // name (not just an OS/arch suffix), so "sysup-worker_linux_amd64.tar.gz"
    // never shadows "sysup_linux_amd64.tar.gz" in the release asset list.
    let ext = if os == "windows" { "zip" } else { "tar.gz" };
    let asset_name = format!("sysup_{os}_{arch}.{ext}");

    if ext == "zip" {
        // Not worth pulling in a zip crate for a rarely-hit path (only
        // windows/amd64 ships as zip) — self-update is best-effort and this
        // just falls back to "no update" on that platform for now.
        return Err(anyhow!(
            "self-update de artefatos .zip (windows) ainda não é suportado"
        ));
    }

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            anyhow!(
                "nenhum asset {} encontrado na release {}",
                asset_name,
                latest
            )
        })?;
    let checksums = release
        .assets
        .iter()
        .find(|a| a.name == "checksums.txt")
        .ok_or_else(|| anyhow!("checksums.txt não encontrado na release {}", latest))?;

    let data = crate::download::get(&asset.browser_download_url)
        .with_context(|| format!("baixando {}", asset.name))?;
    let checksums_txt = crate::download::get(&checksums.browser_download_url)
        .with_context(|| "baixando checksums.txt")?;
    let checksums_txt =
        String::from_utf8(checksums_txt).with_context(|| "checksums.txt não é UTF-8 válido")?;
    crate::download::verify_checksum(&checksums_txt, &asset.name, &data)?;

    let extracted = crate::download::extract_single_file(&data, "sysup")
        .with_context(|| "extraindo binário sysup do arquivo baixado")?;

    let current_exe = std::env::current_exe().with_context(|| "resolvendo executável atual")?;
    replace_binary(&extracted, &current_exe)?;
    Ok(())
}

// Swaps the running binary's file for the freshly downloaded one.
// std::fs::rename is used first since it's atomic and safe to do on an
// executing file on Unix (the running process keeps its old inode open
// until it re-execs). It fails with EXDEV when the extracted temp file
// (under std::env::temp_dir(), often tmpfs) lives on a different
// filesystem than target — copying straight onto target as a fallback
// would hit ETXTBSY instead (Linux refuses to open+truncate a binary
// that's currently executing), so the fallback copies into a sibling
// temp file in target's own directory (same filesystem, so it never
// touches target's live inode) and renames that onto target instead.
fn replace_binary(new_path: &Path, target: &Path) -> anyhow::Result<()> {
    if std::fs::rename(new_path, target).is_ok() {
        return Ok(());
    }

    let target_dir = target
        .parent()
        .ok_or_else(|| anyhow!("{} não tem diretório pai", target.display()))?;
    let target_name = target
        .file_name()
        .ok_or_else(|| anyhow!("{} não tem nome de arquivo", target.display()))?
        .to_string_lossy();
    let staged = target_dir.join(format!(".{target_name}.new"));

    std::fs::copy(new_path, &staged)
        .with_context(|| format!("copiando binário atualizado para {}", staged.display()))?;
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("ajustando permissão de {}", staged.display()))?;
    std::fs::rename(&staged, target)
        .with_context(|| format!("substituindo {} pelo binário atualizado", target.display()))?;
    let _ = std::fs::remove_file(new_path);
    Ok(())
}

// TryUpdateDotfilesRepo does a best-effort `git pull --ff-only` on the repo
// clone that install.sh recorded, so dotfiles stay fresh for people who
// cloned the repo. It's entirely optional: no marker file, no repo, or a
// dirty/diverged tree just means we skip silently — never blocks `update`.
pub fn try_update_dotfiles_repo(dry_run: bool) {
    let Some(home) = std::env::home_dir() else {
        return;
    };
    let marker = home.join(".config").join("sysup").join("repo-path");
    let Ok(data) = std::fs::read_to_string(&marker) else {
        return;
    };
    let repo = data.trim();
    if repo.is_empty() {
        return;
    }

    let status = Command::new("git")
        .args(["-C", repo, "status", "--porcelain"])
        .output();
    match status {
        Ok(out) if out.status.success() && out.stdout.is_empty() => {}
        _ => return,
    }

    let line = format!("git -C {repo} pull --ff-only");
    println!("{}", style::dim(&format!("==> repo dotfiles: {line}")));
    if dry_run {
        return;
    }
    if let Err(e) = Command::new("git")
        .args(["-C", repo, "pull", "--ff-only"])
        .status()
    {
        eprintln!(
            "{}",
            style::warn(&format!("aviso: git pull do repo dotfiles falhou: {e}"))
        );
    }
}

// ReExec replaces the current process with a fresh invocation of the
// (now-updated) binary at the same path, so the rest of `sysup update`
// runs the new code instead of whatever is still loaded in memory.
//
// Takes the exe path as a parameter rather than resolving it itself: once
// self_update has replaced the file on disk (via rename over the running
// binary), the kernel treats the process's original inode as unlinked, so a
// fresh std::env::current_exe() call returns "<path> (deleted)" instead of
// the real path — exec() on that string then fails with ENOENT. Callers
// must capture the path *before* calling self_update.
pub fn re_exec(self_path: &Path) -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let err = Command::new(self_path).args(args).exec();
    // exec() only returns on failure — a successful call never comes back.
    Err(anyhow::Error::from(err))
}
