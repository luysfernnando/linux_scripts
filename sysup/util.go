package main

import (
	"io"
	"os"
	"os/exec"
	"path/filepath"
)

func execLookPath(name string) (string, error) {
	return exec.LookPath(name)
}

func copyFile(src, dst string, mode os.FileMode) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()

	out, err := os.OpenFile(dst, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, mode)
	if err != nil {
		return err
	}
	defer out.Close()

	_, err = io.Copy(out, in)
	return err
}

// isWritable reports whether path's containing directory actually accepts
// new files from us — the real test for "can we replace this binary in
// place", more reliable than parsing permission bits (covers root-owned
// dirs, read-only mounts, ACLs, etc). Used to detect binaries installed by
// a system package manager (e.g. pacman's `bun`), which self-updaters like
// `bun upgrade` can't and shouldn't try to overwrite.
func isWritable(path string) bool {
	dir := filepath.Dir(path)
	f, err := os.CreateTemp(dir, ".sysup-writetest-*")
	if err != nil {
		return false
	}
	name := f.Name()
	f.Close()
	os.Remove(name)
	return true
}

// hasComposerGlobalProject reports whether there's actually a global
// composer.json to update. `composer global update` exits 1 when there
// isn't one — a normal state for anyone who's never installed a global
// composer package, not a real failure — so we skip the step entirely
// instead of letting it abort the whole pipeline.
func hasComposerGlobalProject() bool {
	dir := os.Getenv("COMPOSER_HOME")
	if dir == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			return false
		}
		dir = filepath.Join(home, ".config", "composer")
	}
	_, err := os.Stat(filepath.Join(dir, "composer.json"))
	return err == nil
}

// runNamed runs a named executable with no captured output (or fully
// silenced, when silent is true) — used for delegating to optional
// tidewave-update / tidewave-fix-codex-acp helper scripts if present.
func runNamed(silent bool, name string, args ...string) error {
	cmd := exec.Command(name, args...)
	if !silent {
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
	}
	return cmd.Run()
}
