package main

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"
	"sync"
	"time"
)

// Step is one stage of the update pipeline. Steps that don't apply to this
// machine (tool not installed) are simply omitted by BuildPipeline rather
// than failing the whole run.
//
// NeedsSudo marks steps whose shell command invokes sudo. However well
// primed, sudo's credential cache is keyed per terminal/session in ways
// that vary by system config (tty_tickets, use_pty, timestamp_timeout,
// even PAM 2FA modules that never cache at all) — running two sudo
// invocations at once is not reliably safe on every machine. Rather than
// depend on that, every NeedsSudo step is serialized into a single lane
// that runs one at a time, while non-sudo steps still run fully parallel
// alongside it. This trades a little parallelism for a guarantee: sudo is
// only ever prompting for one thing at a time, so it can never race or
// garble output.
type Step struct {
	Name      string
	NeedsSudo bool
	Run       func(dryRun bool, out io.Writer) error
}

// runShell executes (or, in dry-run mode, just prints) a shell command line,
// writing combined stdout/stderr to out. Steps are expressed as shell
// strings because most of them are already "cmd1 && cmd2"-style pipelines
// ported straight from the old .zshrc aliases.
func runShell(dryRun bool, line string, out io.Writer) error {
	fmt.Fprintln(out, dim("==> "+line))
	if dryRun {
		return nil
	}
	cmd := exec.Command("sh", "-c", line)
	cmd.Stdout = out
	cmd.Stderr = out
	if out == io.Writer(os.Stdout) {
		cmd.Stdin = os.Stdin
	}
	return cmd.Run()
}

// BuildPipeline assembles the steps for this machine, split into:
//   - parallel: independent package ecosystems (system pkg manager, flatpak,
//     composer, npm, bun, firmware) that don't touch each other's state and
//     can run concurrently to cut wall-clock time.
//   - cleanup: orphan removal + cache cleaning, which depend on the system
//     package manager step having finished, so they always run serially
//     after every parallel step completes.
//
// Nothing here is hardcoded to a single distro — that's the whole point.
func addStep(dst *[]Step, name, shellLine string, needsSudo bool) {
	line := shellLine
	*dst = append(*dst, Step{
		Name:      name,
		NeedsSudo: needsSudo,
		Run: func(dryRun bool, out io.Writer) error {
			return runShell(dryRun, line, out)
		},
	})
}

func BuildPipeline(family Family, t Tools) (parallel []Step, cleanup []Step) {
	add := func(dst *[]Step, name, shellLine string) {
		addStep(dst, name, shellLine, strings.Contains(shellLine, "sudo"))
	}

	switch family {
	case FamilyArch:
		switch {
		case t.Yay:
			// yay/paru don't literally contain "sudo" in the command line,
			// but they invoke it internally (via a helper) to install
			// packages — still needs the serialized sudo lane.
			addStep(&parallel, "pacotes do sistema + AUR (yay)", "yay -Syu --noconfirm", true)
		case t.Paru:
			addStep(&parallel, "pacotes do sistema + AUR (paru)", "paru -Syu --noconfirm", true)
		case t.Pacman:
			add(&parallel, "pacotes do sistema (pacman)", "sudo pacman -Syu --noconfirm")
		}
	case FamilyDebian:
		if t.Apt {
			add(&parallel, "pacotes do sistema (apt)", "sudo apt-get update && sudo apt-get full-upgrade -y")
		}
	case FamilyFedora:
		if t.Dnf {
			add(&parallel, "pacotes do sistema (dnf)", "sudo dnf upgrade -y")
		}
	case FamilySuse:
		if t.Zypper {
			add(&parallel, "pacotes do sistema (zypper)", "sudo zypper update -y")
		}
	case FamilyDarwin:
		if t.Brew {
			add(&parallel, "Homebrew", "brew update && brew upgrade")
		}
	case FamilyWindows:
		switch {
		case t.Choco:
			add(&parallel, "Chocolatey", "choco upgrade all -y")
		case t.Winget:
			add(&parallel, "winget", "winget upgrade --all")
		}
	}

	// Homebrew on Linux is common alongside pacman/apt/dnf, so check it
	// independently of family instead of only under FamilyDarwin.
	if family != FamilyDarwin && t.Brew {
		add(&parallel, "Homebrew (linux)", "brew update && brew upgrade")
	}

	if t.Flatpak {
		add(&parallel, "Flatpak", "flatpak update -y && flatpak uninstall --unused -y")
	}
	if t.Composer && hasComposerGlobalProject() {
		add(&parallel, "Composer (global)", "composer global update --no-interaction")
	}
	if t.Npm {
		add(&parallel, "npm (global)", "sudo npm install -g npm@latest && sudo npm update -g")
	}
	if t.Bun {
		if bunPath, err := exec.LookPath("bun"); err == nil && isWritable(bunPath) {
			add(&parallel, "Bun", "bun upgrade")
		}
		// If bun isn't writable by us, it's almost certainly installed via
		// the system package manager (e.g. pacman's `bun` package, owned by
		// root) — `bun upgrade` would try to overwrite a root-owned file
		// (fails with EACCES) and, even with sudo, would silently break the
		// package manager's file tracking. Let the system pkg manager own
		// bun's upgrades instead; just skip this step.
	}
	if t.Fwupdmgr {
		add(&parallel, "Firmware (fwupdmgr)", "fwupdmgr refresh --force && fwupdmgr update --no-reboot-check -y")
	}

	switch family {
	case FamilyArch:
		add(&cleanup, "órfãos + cache (pacman)", "{ pacman -Qtdq | xargs -r sudo pacman -Rns --noconfirm; }; yes | sudo paccache -r")
	case FamilyDebian:
		add(&cleanup, "órfãos + cache (apt)", "sudo apt-get autoremove -y && sudo apt-get autoclean -y")
	case FamilyFedora:
		add(&cleanup, "órfãos + cache (dnf)", "sudo dnf autoremove -y && sudo dnf clean all")
	}

	return parallel, cleanup
}

// prefixWriter line-buffers writes and emits each line prefixed with a
// colored step name, serialized through a shared mutex — needed so
// concurrent steps' output doesn't interleave mid-line into unreadable
// garbage.
type prefixWriter struct {
	name  string
	color string
	mu    *sync.Mutex
	buf   []byte
}

func (w *prefixWriter) Write(p []byte) (int, error) {
	w.mu.Lock()
	defer w.mu.Unlock()
	w.buf = append(w.buf, p...)
	tag := colorize(w.color, "["+w.name+"]")
	for {
		i := bytes.IndexByte(w.buf, '\n')
		if i < 0 {
			break
		}
		fmt.Printf("%s %s\n", tag, w.buf[:i])
		w.buf = w.buf[i+1:]
	}
	return len(p), nil
}

// StepResult records how one pipeline step went, for the end-of-run summary.
// Output is only populated by the TUI runner (which captures each step's
// combined stdout/stderr instead of streaming it live) — empty elsewhere.
type StepResult struct {
	Name   string
	Dur    time.Duration
	Err    error
	Output string
}

// RunParallel primes sudo once against the real terminal (so it can
// actually prompt, instead of the no-op it'd be against /dev/null), keeps
// it alive for the duration, then runs steps: non-sudo steps all at once
// in their own goroutines, sudo steps one at a time in a single dedicated
// lane so no two sudo invocations ever run concurrently. Returns one
// StepResult per step (same order as the input) plus the first error
// encountered, if any.
func RunParallel(steps []Step, dryRun bool) (results []StepResult, err error) {
	if len(steps) == 0 {
		return nil, nil
	}

	stopSudo := primeSudo(steps, dryRun)
	defer stopSudo()

	var mu sync.Mutex
	var wg sync.WaitGroup

	var sudoSteps, plainSteps []int
	for i, s := range steps {
		if s.NeedsSudo {
			sudoSteps = append(sudoSteps, i)
		} else {
			plainSteps = append(plainSteps, i)
		}
	}

	results = make([]StepResult, len(steps))
	run := func(i int) {
		color := stepColor(i)
		out := io.Writer(&prefixWriter{name: steps[i].Name, color: color, mu: &mu})
		start := time.Now()
		stepErr := steps[i].Run(dryRun, out)
		dur := time.Since(start)
		results[i] = StepResult{Name: steps[i].Name, Dur: dur, Err: stepErr}

		mu.Lock()
		tag := colorize(color, "["+steps[i].Name+"]")
		if stepErr != nil {
			fmt.Printf("%s %s (%s)\n", tag, fail("✘ falhou"), dur.Round(time.Millisecond))
		} else if !dryRun {
			fmt.Printf("%s %s (%s)\n", tag, ok("✔ concluído"), dur.Round(time.Millisecond))
		}
		mu.Unlock()
	}

	for _, i := range plainSteps {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			run(i)
		}(i)
	}
	if len(sudoSteps) > 0 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for _, i := range sudoSteps {
				run(i)
			}
		}()
	}
	wg.Wait()

	for _, r := range results {
		if r.Err != nil && err == nil {
			err = r.Err
		}
	}
	return results, err
}

// primeSudo checks whether any step needs root and, if so, primes sudo's
// credential cache against the real terminal (so it can actually prompt,
// instead of the no-op it'd be against /dev/null) and keeps it alive with a
// background ticker. Callers must invoke the returned stop func once done —
// safe to call even when nothing was primed (steps have no sudo, or dryRun).
func primeSudo(steps []Step, dryRun bool) (stop func()) {
	hasSudoStep := false
	for _, s := range steps {
		if s.NeedsSudo {
			hasSudoStep = true
			break
		}
	}
	if dryRun || !hasSudoStep {
		return func() {}
	}

	primeCmd := exec.Command("sudo", "-v")
	primeCmd.Stdin = os.Stdin
	primeCmd.Stdout = os.Stdout
	primeCmd.Stderr = os.Stderr
	if err := primeCmd.Run(); err != nil {
		fmt.Fprintln(os.Stderr, warn("aviso: sudo -v falhou, passos que precisam de root podem pedir senha individualmente: "+err.Error()))
	}
	done := make(chan struct{})
	go sudoKeepAlive(done)
	return func() { close(done) }
}

func sudoKeepAlive(done <-chan struct{}) {
	ticker := time.NewTicker(60 * time.Second)
	defer ticker.Stop()
	for {
		select {
		case <-done:
			return
		case <-ticker.C:
			_ = exec.Command("sudo", "-v").Run()
		}
	}
}
