use anyhow::{anyhow, Context};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::download;

const GITKRAKEN_URL: &str = "https://release.gitkraken.com/linux/gitkraken-amd64.tar.gz";
const GITKRAKEN_APP_DIR: &str = "/opt/gitkraken";
const GITKRAKEN_BIN_LINK: &str = "/usr/bin/gitkraken";
const GITKRAKEN_WRAPPER: &str = "/usr/local/bin/gitkraken";

// Ports gitkraken/gitkraken-install-or-update.sh: download the latest linux
// tarball, safe-swap it into /opt/gitkraken, symlink the binary, and create
// the UPDATE_ON_START-aware wrapper if missing. Same privilege requirements
// as the original script (root-owned paths), just orchestrated from Rust
// instead of bash.
//
// Stays on plain sudo (not the polkit worker) — this installer isn't part
// of `sysup update`'s pipeline, it's a standalone subcommand, and GitKraken
// publishes no checksums.txt to verify the download against the way
// sysup-worker's release assets do (see internal/polkit), so it doesn't
// route through internal/download's checksum path either.
pub fn run_gitkraken(_args: &[String]) -> anyhow::Result<()> {
    let tmp_dir = tempfile_dir()?;

    let archive_path = tmp_dir.join("gitkraken.tar.gz");
    println!("Baixando: {GITKRAKEN_URL}");
    download::get_to_file(GITKRAKEN_URL, &archive_path).context("download falhou")?;

    println!("Extraindo...");
    download::extract_tar_gz(&archive_path, &tmp_dir).context("extração falhou")?;

    let src = tmp_dir.join("gitkraken");
    if !src.is_dir() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(anyhow!(
            "formato inesperado: pasta 'gitkraken' não encontrada no tarball"
        ));
    }

    println!("Instalando em {GITKRAKEN_APP_DIR} (requer sudo)...");
    let new_dir = format!("{GITKRAKEN_APP_DIR}.new");
    let old_dir = format!("{GITKRAKEN_APP_DIR}.old");

    let result = install_gitkraken(&src, &new_dir, &old_dir);
    let _ = std::fs::remove_dir_all(&tmp_dir);
    result
}

fn install_gitkraken(src: &Path, new_dir: &str, old_dir: &str) -> anyhow::Result<()> {
    sudo_run(&["rm", "-rf", new_dir, old_dir])?;
    sudo_run(&["cp", "-a", &src.to_string_lossy(), new_dir])?;
    if sudo_test("-d", GITKRAKEN_APP_DIR) {
        sudo_run(&["mv", GITKRAKEN_APP_DIR, old_dir])?;
    }
    sudo_run(&["mv", new_dir, GITKRAKEN_APP_DIR])?;
    sudo_run(&["rm", "-rf", old_dir])?;

    let bin_target = format!("{GITKRAKEN_APP_DIR}/gitkraken");
    println!("Criando symlink {GITKRAKEN_BIN_LINK} -> {bin_target} (requer sudo)...");
    sudo_run(&["ln", "-sf", &bin_target, GITKRAKEN_BIN_LINK])?;

    if !sudo_test("-x", GITKRAKEN_WRAPPER) {
        println!("Criando wrapper em {GITKRAKEN_WRAPPER} (requer sudo)...");
        let wrapper_dir = Path::new(GITKRAKEN_WRAPPER)
            .parent()
            .ok_or_else(|| anyhow!("caminho de wrapper inválido"))?
            .to_string_lossy()
            .into_owned();
        sudo_run(&["install", "-d", "-m", "0755", &wrapper_dir])?;

        let sysup_path = std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "sysup".to_string());

        let script = format!(
            r#"#!/usr/bin/env bash
set -euo pipefail

# Se quiser atualizar sempre ao abrir, deixe UPDATE_ON_START=1 no ambiente.
UPDATE_ON_START="${{UPDATE_ON_START:-0}}"
BIN="{GITKRAKEN_APP_DIR}/gitkraken"

if [[ "$UPDATE_ON_START" == "1" ]]; then
  "{sysup_path}" gitkraken >/dev/null 2>&1 || true
fi

exec "$BIN" "$@"
"#
        );

        sudo_tee(GITKRAKEN_WRAPPER, &script)?;
        sudo_run(&["chmod", "0755", GITKRAKEN_WRAPPER])?;
    }

    println!("OK: GitKraken atualizado.");
    println!("Abrir: gitkraken");
    println!("Atualizar ao abrir (opcional): UPDATE_ON_START=1 gitkraken");
    Ok(())
}

fn tempfile_dir() -> anyhow::Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!("gitkraken-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn sudo_run(args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new("sudo")
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("falha ao executar sudo {}", args.join(" ")))?;
    if !status.success() {
        return Err(anyhow!("sudo {} falhou: {}", args.join(" "), status));
    }
    Ok(())
}

fn sudo_test(flag: &str, path: &str) -> bool {
    Command::new("sudo")
        .args(["test", flag, path])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn sudo_tee(path: &str, content: &str) -> anyhow::Result<()> {
    let mut child = Command::new("sudo")
        .arg("tee")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .context("falha ao executar sudo tee")?;

    child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("stdin de sudo tee indisponível"))?
        .write_all(content.as_bytes())?;

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("sudo tee {} falhou: {}", path, status));
    }
    Ok(())
}
