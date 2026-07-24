package main

import "os/exec"

// HasTool reports whether an executable is present on PATH.
func HasTool(name string) bool {
	_, err := exec.LookPath(name)
	return err == nil
}

// Tools bundles the presence checks the pipeline needs, computed once per run.
type Tools struct {
	Yay        bool
	Paru       bool
	Pacman     bool
	Apt        bool
	Dnf        bool
	Zypper     bool
	Brew       bool
	Flatpak    bool
	Composer   bool
	Npm        bool
	Bun        bool
	Fwupdmgr   bool
	Choco      bool
	Winget     bool
	Reflector  bool
	RateMirror bool // cachyos-rate-mirrors
	NotifySend bool
}

func DetectTools() Tools {
	return Tools{
		Yay:        HasTool("yay"),
		Paru:       HasTool("paru"),
		Pacman:     HasTool("pacman"),
		Apt:        HasTool("apt-get"),
		Dnf:        HasTool("dnf"),
		Zypper:     HasTool("zypper"),
		Brew:       HasTool("brew"),
		Flatpak:    HasTool("flatpak"),
		Composer:   HasTool("composer"),
		Npm:        HasTool("npm"),
		Bun:        HasTool("bun"),
		Fwupdmgr:   HasTool("fwupdmgr"),
		Choco:      HasTool("choco"),
		Winget:     HasTool("winget"),
		Reflector:  HasTool("reflector"),
		RateMirror: HasTool("cachyos-rate-mirrors"),
		NotifySend: HasTool("notify-send"),
	}
}
