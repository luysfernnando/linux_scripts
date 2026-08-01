// Package pipeline builds and runs the per-machine update pipeline: which
// steps apply, whether they route through the polkit worker or classic
// sudo, and how they're executed (parallel ecosystems + a serialized
// privileged lane).
package pipeline

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"sysup/internal/detect"
	"sysup/internal/polkit"
	"sysup/internal/style"
)

// Step is one stage of the update pipeline. Steps that don't apply to this
// machine (tool not installed) are simply omitted by BuildPipeline rather
// than failing the whole run.
//
// NeedsPrivilege marks steps that need root via the classic sudo path
// (yay/paru's own internal escalation, or the npm -g step, or any sudo
// fallback when no polkit worker is configured). However well primed,
// sudo's credential cache is keyed per terminal/session in ways that vary
// by system config (tty_tickets, use_pty, timestamp_timeout, even PAM 2FA
// modules that never cache at all) — running two sudo invocations at once
// is not reliably safe on every machine. Every NeedsPrivilege step is
// serialized into a single lane that runs one at a time, while everything
// else (including steps routed through the polkit worker, which handles
// its own sequencing) still runs fully parallel alongside it.
type Step struct {
	Name           string
	NeedsPrivilege bool
	Run            func(dryRun bool, out io.Writer) error
}

// RunShell executes (or, in dry-run mode, just prints) a shell command line,
// writing combined stdout/stderr to out. Steps are expressed as shell
// strings because most of them are already "cmd1 && cmd2"-style pipelines
// ported straight from the old .zshrc aliases.
func RunShell(dryRun bool, line string, out io.Writer) error {
	return runShellEnv(dryRun, line, nil, out)
}

// runShellEnv is runShell plus extra environment variables — used to hand
// paru's step SYSUP_WORKER_SOCKET so its configured sysup-authbridge (see
// internal/polkit) can reach the already-authorized worker.
func runShellEnv(dryRun bool, line string, extraEnv []string, out io.Writer) error {
	fmt.Fprintln(out, style.Dim("==> "+line))
	if dryRun {
		return nil
	}
	cmd := exec.Command("sh", "-c", line)
	if len(extraEnv) > 0 {
		cmd.Env = append(os.Environ(), extraEnv...)
	}
	cmd.Stdout = out
	cmd.Stderr = out
	if out == io.Writer(os.Stdout) {
		cmd.Stdin = os.Stdin
	}
	return cmd.Run()
}

// runWorkerCmd sends one exact argv to the already-authorized polkit worker
// instead of a shell string — no `sh -c`, no sudo.
func runWorkerCmd(dryRun bool, worker *polkit.WorkerClient, out io.Writer, argv []string) error {
	fmt.Fprintln(out, style.Dim("==> "+strings.Join(argv, " ")))
	if dryRun {
		return nil
	}
	return worker.Run(argv, out)
}

func runWorkerSequence(dryRun bool, worker *polkit.WorkerClient, out io.Writer, cmds ...[]string) error {
	for _, argv := range cmds {
		if err := runWorkerCmd(dryRun, worker, out, argv); err != nil {
			return err
		}
	}
	return nil
}

// BuildPipeline assembles the steps for this machine, split into:
//   - parallel: independent package ecosystems (system pkg manager, flatpak,
//     composer, npm, bun, firmware) that don't touch each other's state and
//     can run concurrently to cut wall-clock time.
//   - cleanup: orphan removal + cache cleaning, which depend on the system
//     package manager step having finished, so they always run serially
//     after every parallel step completes.
//
// worker is the already-authorized polkit worker (nil if `sysup
// polkit-setup` hasn't run, or dry-run) — when non-nil, the system
// package-manager and cleanup steps route through it instead of sudo. yay
// always stays on classic sudo (no config hook to redirect it); paru is
// pointed at the worker via its own `[bin] Sudo` config (see
// internal/polkit), so its step doesn't need NeedsPrivilege either.
//
// Nothing here is hardcoded to a single distro — that's the whole point.
func addStep(dst *[]Step, name, shellLine string, needsPrivilege bool) {
	line := shellLine
	*dst = append(*dst, Step{
		Name:           name,
		NeedsPrivilege: needsPrivilege,
		Run: func(dryRun bool, out io.Writer) error {
			return RunShell(dryRun, line, out)
		},
	})
}

func addWorkerStep(dst *[]Step, name string, argv []string, worker *polkit.WorkerClient) {
	*dst = append(*dst, Step{
		Name: name,
		Run: func(dryRun bool, out io.Writer) error {
			return runWorkerCmd(dryRun, worker, out, argv)
		},
	})
}

func BuildPipeline(family detect.Family, t detect.Tools, worker *polkit.WorkerClient) (parallel []Step, cleanup []Step) {
	add := func(dst *[]Step, name, shellLine string) {
		addStep(dst, name, shellLine, strings.Contains(shellLine, "sudo"))
	}

	switch family {
	case detect.FamilyArch:
		switch {
		case t.Yay:
			// yay doesn't expose any config/flag to change its privilege
			// escalation binary, so it always calls the real sudo itself —
			// no way to route it through the worker without shadowing the
			// system's sudo globally, which reopens exactly the kind of
			// broad hole this design avoids. Stays on classic sudo.
			addStep(&parallel, "Pacotes do sistema + AUR (yay)", "yay -Syu --noconfirm", true)
		case t.Paru:
			if worker != nil {
				// paru DOES support a custom [bin] Sudo binary (see
				// internal/polkit's paru.conf patching) — when polkit-setup
				// ran, paru is configured to call sysup-authbridge, which
				// forwards straight to this worker. No sudo prompt at all.
				parallel = append(parallel, Step{
					Name: "Pacotes do sistema + AUR (paru)",
					Run: func(dryRun bool, out io.Writer) error {
						return runShellEnv(dryRun, "paru -Syu --noconfirm", []string{worker.SocketEnv()}, out)
					},
				})
			} else {
				addStep(&parallel, "Pacotes do sistema + AUR (paru)", "paru -Syu --noconfirm", true)
			}
		case t.Pacman:
			if worker != nil {
				addWorkerStep(&parallel, "Pacotes do sistema (pacman)", []string{"pacman", "-Syu", "--noconfirm"}, worker)
			} else {
				add(&parallel, "Pacotes do sistema (pacman)", "sudo pacman -Syu --noconfirm")
			}
		}
	case detect.FamilyDebian:
		if t.Apt {
			if worker != nil {
				parallel = append(parallel, Step{
					Name: "Pacotes do sistema (apt)",
					Run: func(dryRun bool, out io.Writer) error {
						return runWorkerSequence(dryRun, worker, out,
							[]string{"apt-get", "update"},
							[]string{"apt-get", "full-upgrade", "-y"},
						)
					},
				})
			} else {
				add(&parallel, "Pacotes do sistema (apt)", "sudo apt-get update && sudo apt-get full-upgrade -y")
			}
		}
	case detect.FamilyFedora:
		if t.Dnf {
			if worker != nil {
				addWorkerStep(&parallel, "Pacotes do sistema (dnf)", []string{"dnf", "upgrade", "-y"}, worker)
			} else {
				add(&parallel, "Pacotes do sistema (dnf)", "sudo dnf upgrade -y")
			}
		}
	case detect.FamilySuse:
		if t.Zypper {
			if worker != nil {
				addWorkerStep(&parallel, "Pacotes do sistema (zypper)", []string{"zypper", "update", "-y"}, worker)
			} else {
				add(&parallel, "Pacotes do sistema (zypper)", "sudo zypper update -y")
			}
		}
	case detect.FamilyDarwin:
		if t.Brew {
			add(&parallel, "Homebrew", "brew update && brew upgrade")
		}
	case detect.FamilyWindows:
		switch {
		case t.Choco:
			add(&parallel, "Chocolatey", "choco upgrade all -y")
		case t.Winget:
			add(&parallel, "winget", "winget upgrade --all")
		}
	}

	// Homebrew on Linux is common alongside pacman/apt/dnf, so check it
	// independently of family instead of only under FamilyDarwin.
	if family != detect.FamilyDarwin && t.Brew {
		add(&parallel, "Homebrew (linux)", "brew update && brew upgrade")
	}

	if t.Flatpak {
		add(&parallel, "Flatpak", "flatpak update -y && flatpak uninstall --unused -y")
	}
	if t.Composer && hasComposerGlobalProject() {
		add(&parallel, "Composer (global)", "composer global update --no-interaction")
	}
	if t.Npm {
		if worker != nil {
			parallel = append(parallel, Step{
				Name: "Npm (global)",
				Run: func(dryRun bool, out io.Writer) error {
					return runWorkerSequence(dryRun, worker, out,
						[]string{"npm", "install", "-g", "npm@latest"},
						[]string{"npm", "update", "-g"},
					)
				},
			})
		} else {
			add(&parallel, "Npm (global)", "sudo npm install -g npm@latest && sudo npm update -g")
		}
	}
	if t.Bun {
		if bunPath, err := exec.LookPath("bun"); err == nil && isWritable(bunPath) {
			add(&parallel, "Bun (global)", "bun upgrade")
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
	case detect.FamilyArch:
		if worker != nil {
			cleanup = append(cleanup, Step{
				Name: "Órfãos + cache (pacman)",
				Run: func(dryRun bool, out io.Writer) error {
					return archCleanupViaWorker(dryRun, worker, out)
				},
			})
		} else {
			add(&cleanup, "Órfãos + cache (pacman)", "{ pacman -Qtdq | xargs -r sudo pacman -Rns --noconfirm; }; yes | sudo paccache -r")
		}
	case detect.FamilyDebian:
		if worker != nil {
			cleanup = append(cleanup, Step{
				Name: "Órfãos + cache (apt)",
				Run: func(dryRun bool, out io.Writer) error {
					return runWorkerSequence(dryRun, worker, out,
						[]string{"apt-get", "autoremove", "-y"},
						[]string{"apt-get", "autoclean", "-y"},
					)
				},
			})
		} else {
			add(&cleanup, "Órfãos + cache (apt)", "sudo apt-get autoremove -y && sudo apt-get autoclean -y")
		}
	case detect.FamilyFedora:
		if worker != nil {
			cleanup = append(cleanup, Step{
				Name: "Órfãos + cache (dnf)",
				Run: func(dryRun bool, out io.Writer) error {
					return runWorkerSequence(dryRun, worker, out,
						[]string{"dnf", "autoremove", "-y"},
						[]string{"dnf", "clean", "all"},
					)
				},
			})
		} else {
			add(&cleanup, "Órfãos + cache (dnf)", "sudo dnf autoremove -y && sudo dnf clean all")
		}
	}

	return parallel, cleanup
}

// archCleanupViaWorker lists orphans unprivileged first (`pacman -Qtdq`
// needs no root) and only sends the actual removal through the worker if
// there's anything to remove, then always clears the cache.
func archCleanupViaWorker(dryRun bool, worker *polkit.WorkerClient, out io.Writer) error {
	listCmd := exec.Command("pacman", "-Qtdq")
	listOut, _ := listCmd.Output() // non-zero exit here just means "no orphans"
	var orphans []string
	for _, line := range strings.Split(strings.TrimSpace(string(listOut)), "\n") {
		if line != "" {
			orphans = append(orphans, line)
		}
	}
	if len(orphans) > 0 {
		argv := append([]string{"pacman", "-Rns", "--noconfirm"}, orphans...)
		if err := runWorkerCmd(dryRun, worker, out, argv); err != nil {
			return err
		}
	} else {
		fmt.Fprintln(out, style.Dim("==> sem pacotes órfãos"))
	}
	return runWorkerCmd(dryRun, worker, out, []string{"paccache", "-r"})
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
	tag := style.Colorize(w.color, "["+w.name+"]")
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

	stopSudo := PrimeSudo(steps, dryRun)
	defer stopSudo()

	var mu sync.Mutex
	var wg sync.WaitGroup

	var privilegedSteps, plainSteps []int
	for i, s := range steps {
		if s.NeedsPrivilege {
			privilegedSteps = append(privilegedSteps, i)
		} else {
			plainSteps = append(plainSteps, i)
		}
	}

	results = make([]StepResult, len(steps))
	run := func(i int) {
		color := style.StepColor(i)
		out := io.Writer(&prefixWriter{name: steps[i].Name, color: color, mu: &mu})
		start := time.Now()
		stepErr := steps[i].Run(dryRun, out)
		dur := time.Since(start)
		results[i] = StepResult{Name: steps[i].Name, Dur: dur, Err: stepErr}

		mu.Lock()
		tag := style.Colorize(color, "["+steps[i].Name+"]")
		if stepErr != nil {
			fmt.Printf("%s %s (%s)\n", tag, style.Fail("✘ falhou"), dur.Round(time.Millisecond))
		} else if !dryRun {
			fmt.Printf("%s %s (%s)\n", tag, style.Ok("✔ concluído"), dur.Round(time.Millisecond))
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
	if len(privilegedSteps) > 0 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for _, i := range privilegedSteps {
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

// PrimeSudo checks whether any step needs classic sudo and, if so, primes
// sudo's credential cache against the real terminal (so it can actually
// prompt, instead of the no-op it'd be against /dev/null) and keeps it
// alive with a background ticker. Callers must invoke the returned stop
// func once done — safe to call even when nothing was primed (no
// NeedsPrivilege steps, or dryRun). This has no equivalent needed on the
// polkit worker path: the worker's own single pkexec call is what prompts,
// whenever it happens, with no keepalive required since the worker just
// stays alive (and authorized) for the run's duration instead of relying
// on a refreshable credential cache.
func PrimeSudo(steps []Step, dryRun bool) (stop func()) {
	hasPrivilegedStep := false
	for _, s := range steps {
		if s.NeedsPrivilege {
			hasPrivilegedStep = true
			break
		}
	}
	if dryRun || !hasPrivilegedStep {
		return func() {}
	}

	primeCmd := exec.Command("sudo", "-v")
	primeCmd.Stdin = os.Stdin
	primeCmd.Stdout = os.Stdout
	primeCmd.Stderr = os.Stderr
	if err := primeCmd.Run(); err != nil {
		fmt.Fprintln(os.Stderr, style.Warn("aviso: sudo -v falhou, passos que precisam de root podem pedir senha individualmente: "+err.Error()))
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
