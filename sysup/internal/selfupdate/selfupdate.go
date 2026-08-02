// Package selfupdate handles sysup replacing its own running binary from
// the latest GitHub release, plus the best-effort dotfiles repo git-pull
// that piggybacks on the same "check GitHub" moment.
package selfupdate

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"

	"github.com/blang/semver"
	ghselfupdate "github.com/rhysd/go-github-selfupdate/selfupdate"

	"sysup/internal/style"
)

// repoSlug identifies the GitHub repo release assets are fetched from —
// duplicated as a tiny local constant here and in internal/polkit rather
// than shared, since it's a single string literal.
const repoSlug = "luysfernnando/linux_scripts"

// Version is stamped at release-build time via
// -ldflags "-X sysup/internal/selfupdate.Version=vX.Y.Z" (GoReleaser does
// this from the git tag). A "dev" value means a locally-built binary —
// never self-update those, there is no meaningful "newer" to compare
// against.
var Version = "dev"

// SelfUpdate checks the latest GitHub release against the running version
// and, if newer, downloads+verifies+replaces the current executable. Any
// failure (offline, GitHub down, rate-limited) is swallowed as a warning —
// this must never be the reason `sysup update` aborts.
//
// Returns true if the binary was replaced (caller should ReExec).
func SelfUpdate(dryRun bool) bool {
	if Version == "dev" {
		return false
	}
	if dryRun {
		fmt.Println("==> self-update: checando última release do GitHub (dry-run, não substitui nada)")
		return false
	}

	current, err := semver.Parse(strings.TrimPrefix(Version, "v"))
	if err != nil {
		fmt.Fprintln(os.Stderr, style.Warn("aviso: versão embutida inválida, pulando self-update: "+err.Error()))
		return false
	}

	// Filter required: assets are matched by OS/arch suffix only (no cmd-name
	// check), so "sysup-worker_linux_amd64.tar.gz" also matches the
	// "linux_amd64.tar.gz" suffix and can shadow "sysup_linux_amd64.tar.gz"
	// in the release asset list, downloading the wrong binary.
	updater, err := ghselfupdate.NewUpdater(ghselfupdate.Config{
		Filters: []string{`^sysup_`},
	})
	if err != nil {
		fmt.Fprintln(os.Stderr, style.Warn("aviso: self-update falhou ao configurar updater: "+err.Error()))
		return false
	}
	latest, err := updater.UpdateSelf(current, repoSlug)
	if err != nil {
		fmt.Fprintln(os.Stderr, style.Warn("aviso: self-update falhou, seguindo com a versão atual: "+err.Error()))
		return false
	}
	if latest.Version.Equals(current) {
		return false
	}
	fmt.Println(style.Header("==> sysup atualizado: %s -> %s", Version, latest.Version))
	return true
}

// TryUpdateDotfilesRepo does a best-effort `git pull --ff-only` on the repo
// clone that install.sh recorded, so dotfiles stay fresh for people who
// cloned the repo. It's entirely optional: no marker file, no repo, or a
// dirty/diverged tree just means we skip silently — never blocks `update`.
func TryUpdateDotfilesRepo(dryRun bool) {
	home, err := os.UserHomeDir()
	if err != nil {
		return
	}
	marker := filepath.Join(home, ".config", "sysup", "repo-path")
	data, err := os.ReadFile(marker)
	if err != nil {
		return
	}
	repo := strings.TrimSpace(string(data))
	if repo == "" {
		return
	}
	if out, err := exec.Command("git", "-C", repo, "status", "--porcelain").Output(); err != nil || len(out) > 0 {
		return
	}

	line := fmt.Sprintf("git -C %s pull --ff-only", repo)
	fmt.Println(style.Dim("==> repo dotfiles: " + line))
	if dryRun {
		return
	}
	if err := exec.Command("git", "-C", repo, "pull", "--ff-only").Run(); err != nil {
		fmt.Fprintln(os.Stderr, style.Warn("aviso: git pull do repo dotfiles falhou: "+err.Error()))
	}
}

// ReExec replaces the current process with a fresh invocation of the
// (now-updated) binary at the same path, so the rest of `sysup update`
// runs the new code instead of whatever is still loaded in memory.
func ReExec() error {
	self, err := os.Executable()
	if err != nil {
		return err
	}
	return syscall.Exec(self, os.Args, os.Environ())
}
