package main

import (
	"bytes"
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

// mirrorCountry reads an optional per-machine country filter for mirror
// ranking from ~/.config/sysup/mirror-country (comma-separated, e.g.
// "Brazil,Argentina,Chile" — same spelling reflector's --country expects).
// Unset by default: sysup has no way to auto-detect location, and this repo
// targets any system, so we never hardcode a region here. Not part of the
// repo — it's a local, per-machine preference, same as repo-path.
func mirrorCountry() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return ""
	}
	data, err := os.ReadFile(filepath.Join(home, ".config", "sysup", "mirror-country"))
	if err != nil {
		return ""
	}
	return strings.TrimSpace(string(data))
}

// shellSingleQuote wraps s so it's passed through `sh -c` as one literal
// argument, regardless of spaces or shell metacharacters in it.
func shellSingleQuote(s string) string {
	return "'" + strings.ReplaceAll(s, "'", `'\''`) + "'"
}

// MirrorRankCommand returns the shell command that best ranks mirrors for
// this family/toolset, or "" if nothing suitable is installed.
func MirrorRankCommand(family Family, t Tools) string {
	switch family {
	case FamilyArch:
		switch {
		case t.RateMirror:
			// cachyos-rate-mirrors auto-detects region on its own; no
			// country filter needed.
			return "sudo cachyos-rate-mirrors"
		case t.Reflector:
			// --protocol https: pacman only speaks http(s)/ftp — without this,
			// reflector happily fills the list with rsync:// mirrors that
			// pacman can never actually download from (dead weight it still
			// has to probe every run). --download-timeout keeps a single slow
			// mirror from stalling the whole ranking.
			cmd := "sudo reflector --protocol https --latest 20 --sort rate --download-timeout 8"
			if country := mirrorCountry(); country != "" {
				cmd += " --country " + shellSingleQuote(country)
			}
			return cmd + " --save /etc/pacman.d/mirrorlist"
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
		fmt.Fprintln(out, dim("==> nenhuma ferramenta de ranking de mirrors encontrada, pulando"))
		if !dryRun {
			return recordMirrorCheck()
		}
		return nil
	}

	// reflector logs one WARNING line per unreachable mirror it probes —
	// on a bad network that's dozens of lines of noise for something that's
	// expected and self-correcting. Collapse them into a single count.
	effOut := out
	var mf *mirrorNoiseFilter
	if strings.Contains(cmd, "reflector") {
		mf = &mirrorNoiseFilter{out: out}
		effOut = mf
	}

	err := runShell(dryRun, cmd, effOut)
	if mf != nil {
		mf.Flush()
	}
	if err != nil {
		return err
	}
	if dryRun {
		return nil
	}
	return recordMirrorCheck()
}

// mirrorNoiseFilter drops reflector's per-mirror "WARNING: failed to rate
// ..." lines and reports a single summary count instead of letting them
// flood the terminal.
type mirrorNoiseFilter struct {
	out        io.Writer
	buf        []byte
	suppressed int
}

func (w *mirrorNoiseFilter) Write(p []byte) (int, error) {
	w.buf = append(w.buf, p...)
	for {
		i := bytes.IndexByte(w.buf, '\n')
		if i < 0 {
			break
		}
		line := w.buf[:i]
		w.buf = w.buf[i+1:]
		if bytes.Contains(line, []byte("WARNING: failed to rate")) {
			w.suppressed++
			continue
		}
		w.out.Write(line)
		w.out.Write([]byte("\n"))
	}
	return len(p), nil
}

func (w *mirrorNoiseFilter) Flush() {
	if w.suppressed > 0 {
		fmt.Fprintln(w.out, dim(fmt.Sprintf("   (%d mirrors indisponíveis/timeout, ignorados)", w.suppressed)))
	}
}
