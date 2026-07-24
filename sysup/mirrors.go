package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

const mirrorCheckInterval = 7 * 24 * time.Hour

func stateDir() (string, error) {
	if xdg := os.Getenv("XDG_STATE_HOME"); xdg != "" {
		return filepath.Join(xdg, "sysup"), nil
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(home, ".local", "state", "sysup"), nil
}

func mirrorStateFile() (string, error) {
	dir, err := stateDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dir, "last-mirror-check"), nil
}

// dueForMirrorCheck reports whether it's been mirrorCheckInterval (or more,
// or ever) since the last mirror ranking. Any read/parse error is treated
// as "yes, check now" — a missing or corrupt state file shouldn't block us.
func dueForMirrorCheck() bool {
	path, err := mirrorStateFile()
	if err != nil {
		return true
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return true
	}
	unixSecs, err := strconv.ParseInt(strings.TrimSpace(string(data)), 10, 64)
	if err != nil {
		return true
	}
	return time.Since(time.Unix(unixSecs, 0)) >= mirrorCheckInterval
}

func recordMirrorCheck() error {
	dir, err := stateDir()
	if err != nil {
		return err
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	path, err := mirrorStateFile()
	if err != nil {
		return err
	}
	now := strconv.FormatInt(time.Now().Unix(), 10)
	return os.WriteFile(path, []byte(now), 0o644)
}

// MirrorRankCommand returns the shell command that best ranks mirrors for
// this family/toolset, or "" if nothing suitable is installed.
func MirrorRankCommand(family Family, t Tools) string {
	switch family {
	case FamilyArch:
		switch {
		case t.RateMirror:
			return "sudo cachyos-rate-mirrors"
		case t.Reflector:
			return "sudo reflector --latest 20 --sort rate --save /etc/pacman.d/mirrorlist"
		}
	case FamilyDebian:
		if HasTool("apt-select") {
			return "apt-select -t 5 && sudo mv sources.list /etc/apt/sources.list"
		}
	}
	return ""
}

// RunMirrorCheck ranks mirrors if a command is available for this system,
// then always stamps the state file (even when there's nothing to do) so
// we don't keep retrying every single run on machines with no ranking tool.
func RunMirrorCheck(family Family, t Tools, dryRun bool, out io.Writer) error {
	cmd := MirrorRankCommand(family, t)
	if cmd == "" {
		fmt.Fprintln(out, "==> nenhuma ferramenta de ranking de mirrors encontrada, pulando")
		if !dryRun {
			return recordMirrorCheck()
		}
		return nil
	}
	if err := runShell(dryRun, cmd, out); err != nil {
		return err
	}
	if dryRun {
		return nil
	}
	return recordMirrorCheck()
}
