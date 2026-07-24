package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"syscall"
)

const (
	tidewaveDefaultPort = "8000"
	tidewaveCLIURL      = "https://github.com/tidewave-ai/tidewave_app/releases/latest/download/tidewave-cli-x86_64-unknown-linux-gnu"
)

func tidewaveAppBinDir() string {
	home, _ := os.UserHomeDir()
	return filepath.Join(home, ".local", "bin")
}

func tidewaveCLIPath() string {
	return filepath.Join(tidewaveAppBinDir(), "tidewave-cli")
}

// RunTidewave ports tidewave/tidewave.sh: install/update/fix-codex-acp
// subcommands, or (with no recognized subcommand) auto-update+fix silently
// and exec the real CLI, injecting a default -p 8000 if none was passed.
func RunTidewave(args []string) error {
	sub := ""
	if len(args) > 0 {
		sub = args[0]
	}
	switch sub {
	case "install":
		return tidewaveCmdInstall()
	case "update":
		return tidewaveCmdUpdate(false)
	case "fix-codex-acp":
		return tidewaveCmdFixCodexACP(false)
	case "help", "-h", "--help":
		tidewaveUsage()
		return nil
	default:
		return tidewaveDefaultRun(args)
	}
}

func tidewaveCmdInstall() error {
	dir := tidewaveAppBinDir()
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	sysupPath, err := os.Executable()
	if err != nil {
		sysupPath = "sysup"
	}
	wrapper := fmt.Sprintf("#!/usr/bin/env bash\nexec \"%s\" tidewave \"$@\"\n", sysupPath)
	target := filepath.Join(dir, "tidewave")
	if err := os.WriteFile(target, []byte(wrapper), 0o755); err != nil {
		return err
	}
	fmt.Println("OK: wrapper instalado em", target)
	return nil
}

func tidewaveCmdUpdate(silent bool) error {
	if HasTool("tidewave-update") {
		return runSilenceable(silent, "tidewave-update")
	}

	dir := tidewaveAppBinDir()
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	tmp, err := os.CreateTemp("", "tidewave-cli-*")
	if err != nil {
		return err
	}
	tmpPath := tmp.Name()
	tmp.Close()
	defer os.Remove(tmpPath)

	if !silent {
		fmt.Println("Baixando:", tidewaveCLIURL)
	}
	if err := downloadFile(tidewaveCLIURL, tmpPath); err != nil {
		return err
	}
	if err := os.Chmod(tmpPath, 0o755); err != nil {
		return err
	}
	if err := copyFile(tmpPath, tidewaveCLIPath(), 0o755); err != nil {
		return err
	}
	if !silent {
		fmt.Println("OK:", tidewaveCLIPath(), "atualizado.")
	}
	return nil
}

func tidewaveCmdFixCodexACP(silent bool) error {
	if HasTool("tidewave-fix-codex-acp") {
		return runSilenceable(silent, "tidewave-fix-codex-acp")
	}

	acp, err := lookPathQuiet("codex-acp")
	if err != nil {
		if silent {
			return err
		}
		return fmt.Errorf("codex-acp não encontrado no PATH")
	}

	cacheHome := os.Getenv("XDG_CACHE_HOME")
	if cacheHome == "" {
		home, _ := os.UserHomeDir()
		cacheHome = filepath.Join(home, ".cache")
	}
	downloads := filepath.Join(cacheHome, "tidewave", "downloads")
	if err := os.MkdirAll(downloads, 0o755); err != nil {
		return err
	}

	entries, _ := os.ReadDir(downloads)
	for _, e := range entries {
		if strings.HasPrefix(e.Name(), "codex-acp-linux-x64-") {
			link := filepath.Join(downloads, e.Name())
			os.Remove(link)
			_ = os.Symlink(acp, link)
		}
	}
	pinned := filepath.Join(downloads, "codex-acp-linux-x64-0-8-2")
	os.Remove(pinned)
	_ = os.Symlink(acp, pinned)

	if !silent {
		fmt.Println("OK: codex-acp linkado em", downloads)
	}
	return nil
}

func tidewaveDefaultRun(args []string) error {
	if HasTool("tidewave-update") {
		_ = runSilenceable(true, "tidewave-update")
	} else {
		_ = tidewaveCmdUpdate(true)
	}

	if HasTool("tidewave-fix-codex-acp") {
		_ = runSilenceable(true, "tidewave-fix-codex-acp")
	} else {
		_ = tidewaveCmdFixCodexACP(true)
	}

	cliBin := tidewaveCLIPath()
	if stat, err := os.Stat(cliBin); err != nil || stat.Mode()&0o111 == 0 {
		fmt.Println("tidewave-cli não encontrado. Executando update...")
		if err := tidewaveCmdUpdate(false); err != nil {
			return err
		}
	}

	hasPort := false
	for _, a := range args {
		if a == "-p" || a == "--port" {
			hasPort = true
			break
		}
	}
	finalArgs := args
	if !hasPort {
		finalArgs = append([]string{"-p", tidewaveDefaultPort}, args...)
	}

	return syscall.Exec(cliBin, append([]string{cliBin}, finalArgs...), os.Environ())
}

func tidewaveUsage() {
	fmt.Print(`Uso: sysup tidewave [comando] [args]

Comandos:
  install         Instala o wrapper "tidewave" em ~/.local/bin
  update          Atualiza o binário tidewave-cli
  fix-codex-acp   Linka/arruma o codex-acp no cache
  help            Mostra esta ajuda

Sem comando, executa o Tidewave (com update/fix silenciosos)
`)
}

func runSilenceable(silent bool, name string, args ...string) error {
	return runNamed(silent, name, args...)
}

func lookPathQuiet(name string) (string, error) {
	return execLookPath(name)
}
