// Installs a native periodic trigger (systemd timer, launchd agent, or
// schtasks job) that runs `sysup mirrors` weekly.

use anyhow::{anyhow, Context};
use std::path::Path;
use std::process::Command;

use crate::detect;

const SYSTEMD_UNIT_NAME: &str = "sysup-mirrors";

// InstallSchedule sets up a weekly background trigger for `sysup mirrors`
// on whichever scheduler this OS actually has, so mirror ranking stays
// fresh even for machines where `sysup update` isn't run every week.
pub fn install_schedule() -> anyhow::Result<()> {
    let self_path = std::env::current_exe().with_context(|| "resolvendo executável atual")?;

    if cfg!(target_os = "linux") && detect::has_tool("systemctl") {
        install_systemd_timer(&self_path)
    } else if cfg!(target_os = "macos") {
        install_launchd_agent(&self_path)
    } else if cfg!(target_os = "windows") && detect::has_tool("schtasks") {
        install_schtasks(&self_path)
    } else {
        Err(anyhow!(
            "nenhum agendador suportado encontrado neste sistema (systemd/launchd/schtasks)"
        ))
    }
}

#[cfg(target_os = "linux")]
fn install_systemd_timer(self_path: &Path) -> anyhow::Result<()> {
    let home = std::env::home_dir().ok_or_else(|| anyhow!("$HOME não definido"))?;
    let dir = home.join(".config").join("systemd").join("user");
    std::fs::create_dir_all(&dir)?;

    let service = format!(
        "[Unit]\nDescription=sysup mirror ranking\n\n[Service]\nType=oneshot\nExecStart={} mirrors\n",
        self_path.display()
    );

    let timer = "[Unit]\nDescription=Weekly sysup mirror ranking\n\n[Timer]\nOnCalendar=weekly\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n";

    std::fs::write(dir.join(format!("{SYSTEMD_UNIT_NAME}.service")), service)?;
    std::fs::write(dir.join(format!("{SYSTEMD_UNIT_NAME}.timer")), timer)?;

    run_ok(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
    run_ok(Command::new("systemctl").args([
        "--user",
        "enable",
        "--now",
        &format!("{SYSTEMD_UNIT_NAME}.timer"),
    ]))?;

    println!("==> timer systemd instalado: {SYSTEMD_UNIT_NAME}.timer (semanal)");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn install_systemd_timer(_self_path: &Path) -> anyhow::Result<()> {
    Err(anyhow!("systemd não é suportado neste sistema"))
}

#[cfg(target_os = "macos")]
fn install_launchd_agent(self_path: &Path) -> anyhow::Result<()> {
    let home = std::env::home_dir().ok_or_else(|| anyhow!("$HOME não definido"))?;
    let dir = home.join("Library").join("LaunchAgents");
    std::fs::create_dir_all(&dir)?;

    let plist_path = dir.join("com.sysup.mirrors.plist");
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>com.sysup.mirrors</string>
	<key>ProgramArguments</key>
	<array>
		<string>{}</string>
		<string>mirrors</string>
	</array>
	<key>StartInterval</key>
	<integer>604800</integer>
</dict>
</plist>
"#,
        self_path.display()
    );

    std::fs::write(&plist_path, plist)?;
    run_ok(Command::new("launchctl").args(["load", &plist_path.display().to_string()]))?;

    println!(
        "==> launchd agent instalado: {} (semanal)",
        plist_path.display()
    );
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn install_launchd_agent(_self_path: &Path) -> anyhow::Result<()> {
    Err(anyhow!("launchd não é suportado neste sistema"))
}

#[cfg(target_os = "windows")]
fn install_schtasks(self_path: &Path) -> anyhow::Result<()> {
    run_ok(Command::new("schtasks").args([
        "/Create",
        "/SC",
        "WEEKLY",
        "/TN",
        "SysupMirrors",
        "/TR",
        &format!("\"{}\" mirrors", self_path.display()),
        "/F",
    ]))?;
    println!("==> tarefa agendada instalada: SysupMirrors (semanal)");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn install_schtasks(_self_path: &Path) -> anyhow::Result<()> {
    Err(anyhow!("schtasks não é suportado neste sistema"))
}

fn run_ok(cmd: &mut Command) -> anyhow::Result<()> {
    let status = cmd.status()?;
    if !status.success() {
        return Err(anyhow!("comando falhou: {:?} ({})", cmd, status));
    }
    Ok(())
}
