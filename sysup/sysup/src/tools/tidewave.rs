use anyhow::{anyhow, Context};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::detect;
use crate::download;

const TIDEWAVE_DEFAULT_PORT: &str = "8000";
const TIDEWAVE_CLI_URL: &str =
    "https://github.com/tidewave-ai/tidewave_app/releases/latest/download/tidewave-cli-x86_64-unknown-linux-gnu";

fn tidewave_app_bin_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".local").join("bin")
}

fn tidewave_cli_path() -> PathBuf {
    tidewave_app_bin_dir().join("tidewave-cli")
}

// Ports tidewave/tidewave.sh: install/update/fix-codex-acp subcommands, or
// (with no recognized subcommand) auto-update+fix silently and exec the
// real CLI, injecting a default -p 8000 if none was passed.
pub fn run_tidewave(args: &[String]) -> anyhow::Result<()> {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "install" => tidewave_cmd_install(),
        "update" => tidewave_cmd_update(false),
        "fix-codex-acp" => tidewave_fix_codex_acp(false),
        "help" | "-h" | "--help" => {
            tidewave_usage();
            Ok(())
        }
        _ => tidewave_default_run(args),
    }
}

fn tidewave_cmd_install() -> anyhow::Result<()> {
    let dir = tidewave_app_bin_dir();
    std::fs::create_dir_all(&dir)?;
    let sysup_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "sysup".to_string());
    let wrapper = format!("#!/usr/bin/env bash\nexec \"{sysup_path}\" tidewave \"$@\"\n");
    let target = dir.join("tidewave");
    write_executable(&target, wrapper.as_bytes())?;
    println!("OK: wrapper instalado em {}", target.display());
    Ok(())
}

fn tidewave_cmd_update(silent: bool) -> anyhow::Result<()> {
    if detect::has_tool("tidewave-update") {
        return run_named(silent, "tidewave-update", &[]);
    }

    let dir = tidewave_app_bin_dir();
    std::fs::create_dir_all(&dir)?;

    let tmp_path =
        std::env::temp_dir().join(format!("tidewave-cli-{}", std::process::id()));

    if !silent {
        println!("Baixando: {TIDEWAVE_CLI_URL}");
    }
    download::get_to_file(TIDEWAVE_CLI_URL, &tmp_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))?;
    }

    let dest = tidewave_cli_path();
    let copy_result = copy_file(&tmp_path, &dest, 0o755);
    let _ = std::fs::remove_file(&tmp_path);
    copy_result?;

    if !silent {
        println!("OK: {} atualizado.", dest.display());
    }
    Ok(())
}

fn tidewave_fix_codex_acp(silent: bool) -> anyhow::Result<()> {
    if detect::has_tool("tidewave-fix-codex-acp") {
        return run_named(silent, "tidewave-fix-codex-acp", &[]);
    }

    let acp = which::which("codex-acp").map_err(|_| {
        if silent {
            anyhow!("codex-acp não encontrado no PATH (silent)")
        } else {
            anyhow!("codex-acp não encontrado no PATH")
        }
    })?;

    let cache_home = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join(".cache")
        });
    let downloads = cache_home.join("tidewave").join("downloads");
    std::fs::create_dir_all(&downloads)?;

    if let Ok(entries) = std::fs::read_dir(&downloads) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("codex-acp-linux-x64-") {
                let link = downloads.join(entry.file_name());
                let _ = std::fs::remove_file(&link);
                let _ = std::os::unix::fs::symlink(&acp, &link);
            }
        }
    }
    let pinned = downloads.join("codex-acp-linux-x64-0-8-2");
    let _ = std::fs::remove_file(&pinned);
    let _ = std::os::unix::fs::symlink(&acp, &pinned);

    if !silent {
        println!("OK: codex-acp linkado em {}", downloads.display());
    }
    Ok(())
}

// Uses real process::exec (not spawn) so the tidewave CLI replaces this
// process in place — matching Go's syscall.Exec: no lingering sysup parent,
// signals/tty go straight to the CLI, exit code propagates untouched.
fn tidewave_default_run(args: &[String]) -> anyhow::Result<()> {
    if detect::has_tool("tidewave-update") {
        let _ = run_named(true, "tidewave-update", &[]);
    } else {
        let _ = tidewave_cmd_update(true);
    }

    if detect::has_tool("tidewave-fix-codex-acp") {
        let _ = run_named(true, "tidewave-fix-codex-acp", &[]);
    } else {
        let _ = tidewave_fix_codex_acp(true);
    }

    let cli_bin = tidewave_cli_path();
    let executable = std::fs::metadata(&cli_bin)
        .map(|m| {
            use std::os::unix::fs::PermissionsExt;
            m.permissions().mode() & 0o111 != 0
        })
        .unwrap_or(false);
    if !executable {
        println!("tidewave-cli não encontrado. Executando update...");
        tidewave_cmd_update(false)?;
    }

    let has_port = args.iter().any(|a| a == "-p" || a == "--port");
    let mut final_args: Vec<String> = Vec::new();
    if !has_port {
        final_args.push("-p".to_string());
        final_args.push(TIDEWAVE_DEFAULT_PORT.to_string());
    }
    final_args.extend(args.iter().cloned());

    let err = Command::new(&cli_bin).args(&final_args).exec();
    Err(anyhow!("exec {} falhou: {}", cli_bin.display(), err))
}

fn tidewave_usage() {
    print!(
        r#"Uso: sysup tidewave [comando] [args]

Comandos:
  install         Instala o wrapper "tidewave" em ~/.local/bin
  update          Atualiza o binário tidewave-cli
  fix-codex-acp   Linka/arruma o codex-acp no cache
  help            Mostra esta ajuda

Sem comando, executa o Tidewave (com update/fix silenciosos)
"#
    );
}

// Runs a named executable with no captured output (or fully silenced, when
// silent is true) — used for delegating to optional tidewave-update /
// tidewave-fix-codex-acp helper scripts if present.
fn run_named(silent: bool, name: &str, args: &[String]) -> anyhow::Result<()> {
    let mut cmd = Command::new(name);
    cmd.args(args);
    if silent {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    } else {
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    }
    let status = cmd
        .status()
        .with_context(|| format!("falha ao executar {name}"))?;
    if !status.success() {
        return Err(anyhow!("{} falhou: {}", name, status));
    }
    Ok(())
}

fn copy_file(src: &std::path::Path, dst: &std::path::Path, mode: u32) -> anyhow::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut input = std::fs::File::open(src)?;
    let mut output = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(mode)
        .open(dst)?;
    std::io::copy(&mut input, &mut output)?;
    Ok(())
}

fn write_executable(path: &std::path::Path, content: &[u8]) -> anyhow::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o755)
        .open(path)?;
    std::io::Write::write_all(&mut f, content)?;
    Ok(())
}
