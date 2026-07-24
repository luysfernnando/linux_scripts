package main

import (
	"archive/tar"
	"compress/gzip"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

const (
	gitkrakenURL      = "https://release.gitkraken.com/linux/gitkraken-amd64.tar.gz"
	gitkrakenAppDir   = "/opt/gitkraken"
	gitkrakenBinLink  = "/usr/bin/gitkraken"
	gitkrakenWrapper  = "/usr/local/bin/gitkraken"
)

// RunGitKraken ports gitkraken/gitkraken-install-or-update.sh: download the
// latest linux tarball, safe-swap it into /opt/gitkraken, symlink the
// binary, and create the UPDATE_ON_START-aware wrapper if missing. Same
// privilege requirements as the original script (root-owned paths), just
// orchestrated from Go instead of bash.
func RunGitKraken(args []string) error {
	tmpDir, err := os.MkdirTemp("", "gitkraken-*")
	if err != nil {
		return err
	}
	defer os.RemoveAll(tmpDir)

	archivePath := filepath.Join(tmpDir, "gitkraken.tar.gz")
	fmt.Println("Baixando:", gitkrakenURL)
	if err := downloadFile(gitkrakenURL, archivePath); err != nil {
		return fmt.Errorf("download falhou: %w", err)
	}

	fmt.Println("Extraindo...")
	if err := extractTarGz(archivePath, tmpDir); err != nil {
		return fmt.Errorf("extração falhou: %w", err)
	}

	src := filepath.Join(tmpDir, "gitkraken")
	if stat, err := os.Stat(src); err != nil || !stat.IsDir() {
		return fmt.Errorf("formato inesperado: pasta 'gitkraken' não encontrada no tarball")
	}

	fmt.Printf("Instalando em %s (requer sudo)...\n", gitkrakenAppDir)
	newDir := gitkrakenAppDir + ".new"
	oldDir := gitkrakenAppDir + ".old"

	if err := sudoRun("rm", "-rf", newDir, oldDir); err != nil {
		return err
	}
	if err := sudoRun("cp", "-a", src, newDir); err != nil {
		return err
	}
	if sudoTest("-d", gitkrakenAppDir) {
		if err := sudoRun("mv", gitkrakenAppDir, oldDir); err != nil {
			return err
		}
	}
	if err := sudoRun("mv", newDir, gitkrakenAppDir); err != nil {
		return err
	}
	if err := sudoRun("rm", "-rf", oldDir); err != nil {
		return err
	}

	fmt.Printf("Criando symlink %s -> %s/gitkraken (requer sudo)...\n", gitkrakenBinLink, gitkrakenAppDir)
	if err := sudoRun("ln", "-sf", filepath.Join(gitkrakenAppDir, "gitkraken"), gitkrakenBinLink); err != nil {
		return err
	}

	if !sudoTest("-x", gitkrakenWrapper) {
		fmt.Printf("Criando wrapper em %s (requer sudo)...\n", gitkrakenWrapper)
		if err := sudoRun("install", "-d", "-m", "0755", filepath.Dir(gitkrakenWrapper)); err != nil {
			return err
		}
		sysupPath, err := os.Executable()
		if err != nil {
			sysupPath = "sysup"
		}
		script := fmt.Sprintf(`#!/usr/bin/env bash
set -euo pipefail

# Se quiser atualizar sempre ao abrir, deixe UPDATE_ON_START=1 no ambiente.
UPDATE_ON_START="${UPDATE_ON_START:-0}"
BIN="%s/gitkraken"

if [[ "$UPDATE_ON_START" == "1" ]]; then
  "%s" gitkraken >/dev/null 2>&1 || true
fi

exec "$BIN" "$@"
`, gitkrakenAppDir, sysupPath)

		if err := sudoTee(gitkrakenWrapper, script); err != nil {
			return err
		}
		if err := sudoRun("chmod", "0755", gitkrakenWrapper); err != nil {
			return err
		}
	}

	fmt.Println("OK: GitKraken atualizado.")
	fmt.Println("Abrir: gitkraken")
	fmt.Println("Atualizar ao abrir (opcional): UPDATE_ON_START=1 gitkraken")
	return nil
}

func downloadFile(url, dest string) error {
	resp, err := http.Get(url)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("status %s", resp.Status)
	}
	f, err := os.Create(dest)
	if err != nil {
		return err
	}
	defer f.Close()
	_, err = io.Copy(f, resp.Body)
	return err
}

func extractTarGz(archivePath, destDir string) error {
	f, err := os.Open(archivePath)
	if err != nil {
		return err
	}
	defer f.Close()

	gz, err := gzip.NewReader(f)
	if err != nil {
		return err
	}
	defer gz.Close()

	tr := tar.NewReader(gz)
	for {
		hdr, err := tr.Next()
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
		target := filepath.Join(destDir, hdr.Name)

		switch hdr.Typeflag {
		case tar.TypeDir:
			if err := os.MkdirAll(target, 0o755); err != nil {
				return err
			}
		case tar.TypeReg:
			if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
				return err
			}
			out, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, os.FileMode(hdr.Mode))
			if err != nil {
				return err
			}
			if _, err := io.Copy(out, tr); err != nil {
				out.Close()
				return err
			}
			out.Close()
		case tar.TypeSymlink:
			if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
				return err
			}
			_ = os.Remove(target)
			if err := os.Symlink(hdr.Linkname, target); err != nil {
				return err
			}
		}
	}
}

func sudoRun(args ...string) error {
	cmd := exec.Command("sudo", args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func sudoTest(flag, path string) bool {
	return exec.Command("sudo", "test", flag, path).Run() == nil
}

func sudoTee(path, content string) error {
	cmd := exec.Command("sudo", "tee", path)
	cmd.Stdin = strings.NewReader(content)
	cmd.Stderr = os.Stderr
	return cmd.Run()
}
