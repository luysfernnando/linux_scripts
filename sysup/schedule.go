package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
)

const systemdUnitName = "sysup-mirrors"

// InstallSchedule sets up a weekly background trigger for `sysup mirrors`
// on whichever scheduler this OS actually has, so mirror ranking stays
// fresh even for machines where `sysup update` isn't run every week.
func InstallSchedule() error {
	self, err := os.Executable()
	if err != nil {
		return err
	}

	switch {
	case runtime.GOOS == "linux" && HasTool("systemctl"):
		return installSystemdTimer(self)
	case runtime.GOOS == "darwin":
		return installLaunchdAgent(self)
	case runtime.GOOS == "windows" && HasTool("schtasks"):
		return installSchtasks(self)
	default:
		return fmt.Errorf("nenhum agendador suportado encontrado neste sistema (systemd/launchd/schtasks)")
	}
}

func installSystemdTimer(self string) error {
	home, err := os.UserHomeDir()
	if err != nil {
		return err
	}
	dir := filepath.Join(home, ".config", "systemd", "user")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}

	service := fmt.Sprintf(`[Unit]
Description=sysup mirror ranking

[Service]
Type=oneshot
ExecStart=%s mirrors
`, self)

	timer := `[Unit]
Description=Weekly sysup mirror ranking

[Timer]
OnCalendar=weekly
Persistent=true

[Install]
WantedBy=timers.target
`

	if err := os.WriteFile(filepath.Join(dir, systemdUnitName+".service"), []byte(service), 0o644); err != nil {
		return err
	}
	if err := os.WriteFile(filepath.Join(dir, systemdUnitName+".timer"), []byte(timer), 0o644); err != nil {
		return err
	}

	if err := exec.Command("systemctl", "--user", "daemon-reload").Run(); err != nil {
		return err
	}
	if err := exec.Command("systemctl", "--user", "enable", "--now", systemdUnitName+".timer").Run(); err != nil {
		return err
	}
	fmt.Printf("==> timer systemd instalado: %s.timer (semanal)\n", systemdUnitName)
	return nil
}

func installLaunchdAgent(self string) error {
	home, err := os.UserHomeDir()
	if err != nil {
		return err
	}
	dir := filepath.Join(home, "Library", "LaunchAgents")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}

	plistPath := filepath.Join(dir, "com.sysup.mirrors.plist")
	plist := fmt.Sprintf(`<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>com.sysup.mirrors</string>
	<key>ProgramArguments</key>
	<array>
		<string>%s</string>
		<string>mirrors</string>
	</array>
	<key>StartInterval</key>
	<integer>604800</integer>
</dict>
</plist>
`, self)

	if err := os.WriteFile(plistPath, []byte(plist), 0o644); err != nil {
		return err
	}
	if err := exec.Command("launchctl", "load", plistPath).Run(); err != nil {
		return err
	}
	fmt.Printf("==> launchd agent instalado: %s (semanal)\n", plistPath)
	return nil
}

func installSchtasks(self string) error {
	cmd := exec.Command("schtasks", "/Create", "/SC", "WEEKLY", "/TN", "SysupMirrors",
		"/TR", fmt.Sprintf(`"%s" mirrors`, self), "/F")
	if err := cmd.Run(); err != nil {
		return err
	}
	fmt.Println("==> tarefa agendada instalada: SysupMirrors (semanal)")
	return nil
}
