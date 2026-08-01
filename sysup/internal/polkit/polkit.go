// Package polkit is the client side of sysup's polkit privilege-escalation
// feature: detecting whether `sysup polkit-setup` has run, rendering the
// policy XML, and running the setup flow itself. See sysup/README.md for
// the full architecture and cmd/sysup-worker for the privileged half.
package polkit

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"sysup/internal/detect"
	"sysup/internal/download"
	"sysup/internal/style"
)

// repoSlug identifies the GitHub repo release assets are fetched from —
// duplicated as a tiny local constant here and in internal/selfupdate
// rather than shared, since it's a single string literal and sharing it
// would mean an otherwise-pointless inter-package dependency.
const repoSlug = "luysfernnando/linux_scripts"

const (
	polkitActionID    = "io.github.luysfernnando.sysup.worker"
	policyPath        = "/usr/share/polkit-1/actions/" + polkitActionID + ".policy"
	helperInstallDir  = "/usr/lib/sysup"
	helperInstallPath = helperInstallDir + "/sysup-worker"
	authbridgePath    = helperInstallDir + "/sysup-authbridge"
)

// Available reports whether this machine even has a polkit stack (pkexec +
// the actions directory) — separate from Configured, which checks whether
// sysup's OWN one-time setup ran.
func Available() bool {
	if !detect.HasTool("pkexec") {
		return false
	}
	_, err := os.Stat("/usr/share/polkit-1/actions")
	return err == nil
}

// Configured reports whether `sysup polkit-setup` has already installed the
// root-owned helper and its policy. BuildPipeline uses this (not Available
// alone) to decide whether privileged steps go through the worker or fall
// back to classic sudo.
func Configured() bool {
	if !Available() {
		return false
	}
	info, err := os.Stat(helperInstallPath)
	if err != nil || info.Mode().IsDir() {
		return false
	}
	// Reject if group/other can write it — a helper that anyone but root
	// could modify would make the whole polkit action pointless.
	if info.Mode().Perm()&0o022 != 0 {
		return false
	}
	if _, err := os.Stat(policyPath); err != nil {
		return false
	}
	return true
}

// RenderPolicyXML builds the single polkit action that authorizes running
// sysup-worker. This doesn't vary per machine — the worker re-detects
// family/tools itself and enforces its own whitelist, so the policy only
// ever needs to answer "can this user run the worker."
func RenderPolicyXML() string {
	return fmt.Sprintf(`<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC
 "-//freedesktop//DTD polkit Policy Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/polkit/1/policyconfig.dtd">
<policyconfig>
  <vendor>sysup</vendor>
  <vendor_url>https://github.com/%s</vendor_url>

  <action id="%s">
    <description>Executar operações de atualização do sistema (sysup)</description>
    <message>sysup precisa de privilégios administrativos para rodar o update</message>
    <icon_name>system-software-update</icon_name>
    <defaults>
      <allow_any>no</allow_any>
      <allow_inactive>no</allow_inactive>
      <allow_active>auth_admin_keep</allow_active>
    </defaults>
    <annotate key="org.freedesktop.policykit.exec.path">%s</annotate>
    <annotate key="org.freedesktop.policykit.exec.allow_gui">true</annotate>
  </action>
</policyconfig>
`, repoSlug, polkitActionID, helperInstallPath)
}

// RunSetup is `sysup polkit-setup`: shows exactly what will be
// installed/changed, asks for confirmation, then applies it behind a single
// elevation prompt (this bootstrap step, not the worker itself, so it's
// fine to just use whatever of pkexec/sudo is present). This is a one-time
// setup action — it never installs a standing NOPASSWD rule of any kind.
func RunSetup(dryRun bool) error {
	if !detect.HasTool("pkexec") {
		fmt.Println(style.Warn("aviso: pkexec não encontrado — este sistema não tem polkit, nada a fazer. `sysup update` continua usando sudo."))
		return nil
	}

	family := detect.DetectFamily()
	tools := detect.DetectTools()

	fmt.Println(style.Header("==> sysup polkit-setup (%s)", family))
	fmt.Println("Isso instala:")
	fmt.Printf("  - %s (binário root, só roda via pkexec, vive só durante um `sysup update`)\n", helperInstallPath)
	fmt.Println("  - " + policyPath + " (autoriza esse binário via polkit)")

	var paruPath, paruNewContent string
	if tools.Paru {
		fmt.Println("  - symlink " + authbridgePath + " -> sysup-worker")
		var err error
		paruPath, paruNewContent, err = buildParuConfUpdate()
		if err != nil {
			fmt.Println(style.Warn("aviso: não consegui preparar a edição de paru.conf: " + err.Error()))
			paruPath = ""
		} else {
			fmt.Printf("  - %s: adiciona/atualiza `[bin] Sudo = %s` (backup .bak antes; SudoLoop não é tocado)\n", paruPath, authbridgePath)
		}
	}
	if tools.Yay {
		fmt.Println(style.Warn("aviso: yay detectado — ele chama sudo por conta própria pra instalar pacotes AUR e não tem opção de configuração pra trocar isso (paru tem; yay não). Nessas máquinas o update ainda vai pedir um segundo prompt (sudo clássico), disparado junto no início, não no meio do dashboard. Ver sysup/README.md."))
	}

	policyXML := RenderPolicyXML()
	fmt.Println(style.Header("\n==> conteúdo proposto para %s", policyPath))
	fmt.Print(policyXML)

	if paruPath != "" {
		fmt.Println(style.Header("\n==> conteúdo proposto para %s", paruPath))
		fmt.Print(paruNewContent)
	}

	if dryRun {
		return nil
	}

	fmt.Print("\nAplicar essas mudanças agora? [y/N] ")
	reader := bufio.NewReader(os.Stdin)
	answer, _ := reader.ReadString('\n')
	answer = strings.ToLower(strings.TrimSpace(answer))
	if answer != "y" && answer != "yes" && answer != "s" && answer != "sim" {
		fmt.Println("cancelado")
		return nil
	}

	helperBin, err := acquireHelperBinary()
	if err != nil {
		return fmt.Errorf("obtendo sysup-worker: %w", err)
	}
	defer os.Remove(helperBin)

	if err := installElevated(helperBin, policyXML, tools.Paru); err != nil {
		return err
	}

	if paruPath != "" {
		if err := applyParuConf(paruPath, paruNewContent); err != nil {
			return fmt.Errorf("editando %s: %w", paruPath, err)
		}
	}

	fmt.Println(style.Ok("✔ polkit configurado — próximo `sysup update` pede a senha uma única vez por execução"))
	return nil
}

// installElevated writes the helper binary + policy (and, if requested, the
// sysup-authbridge symlink) via one elevated shell script, so the whole
// install happens behind a single sudo/pkexec prompt instead of one per
// file. This is a one-time bootstrap script, not something that runs again
// after setup — the actual update pipeline never invokes a shell.
func installElevated(helperBin, policyXML string, includeAuthbridge bool) error {
	tmpPolicy, err := os.CreateTemp("", "sysup-policy-*.xml")
	if err != nil {
		return err
	}
	defer os.Remove(tmpPolicy.Name())
	if _, err := tmpPolicy.WriteString(policyXML); err != nil {
		tmpPolicy.Close()
		return err
	}
	tmpPolicy.Close()

	script := fmt.Sprintf("set -e\n"+
		"install -o root -g root -m 0755 -D %s %s\n"+
		"install -o root -g root -m 0644 -D %s %s\n",
		shellQuote(helperBin), shellQuote(helperInstallPath),
		shellQuote(tmpPolicy.Name()), shellQuote(policyPath))
	if includeAuthbridge {
		script += fmt.Sprintf("ln -sf sysup-worker %s\n", shellQuote(authbridgePath))
	}

	tmpScript, err := os.CreateTemp("", "sysup-polkit-install-*.sh")
	if err != nil {
		return err
	}
	defer os.Remove(tmpScript.Name())
	if _, err := tmpScript.WriteString(script); err != nil {
		tmpScript.Close()
		return err
	}
	tmpScript.Close()

	cmd := exec.Command("pkexec", "sh", tmpScript.Name())
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func shellQuote(s string) string {
	return "'" + strings.ReplaceAll(s, "'", `'\''`) + "'"
}

// acquireHelperBinary prefers building from a local repo clone (matching
// the ~/.config/sysup/repo-path marker install.sh already maintains) over
// downloading a release asset — mirrors install.sh's own "build from
// source if we can, download otherwise" ordering, just reversed priority
// since a dev iterating on this code wants their own build, not last
// release's.
func acquireHelperBinary() (string, error) {
	if path, ok := buildHelperFromRepo(); ok {
		return path, nil
	}
	return downloadHelperBinary()
}

func buildHelperFromRepo() (string, bool) {
	if !detect.HasTool("go") {
		return "", false
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return "", false
	}
	marker := filepath.Join(home, ".config", "sysup", "repo-path")
	data, err := os.ReadFile(marker)
	if err != nil {
		return "", false
	}
	repo := strings.TrimSpace(string(data))
	srcDir := filepath.Join(repo, "sysup")
	if _, err := os.Stat(filepath.Join(srcDir, "cmd", "sysup-worker")); err != nil {
		return "", false
	}

	out := filepath.Join(os.TempDir(), fmt.Sprintf("sysup-worker-build-%d", os.Getpid()))
	cmd := exec.Command("go", "build", "-o", out, "./cmd/sysup-worker")
	cmd.Dir = srcDir
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		return "", false
	}
	return out, true
}

// downloadHelperBinary fetches sysup-worker from the latest GitHub release,
// verifying it against checksums.txt from the same release before trusting
// it — same trust chain SelfUpdate already relies on for the main binary.
func downloadHelperBinary() (string, error) {
	if runtime.GOOS != "linux" {
		return "", fmt.Errorf("polkit só é suportado no Linux")
	}
	assetName := fmt.Sprintf("sysup-worker_linux_%s.tar.gz", runtime.GOARCH)
	base := fmt.Sprintf("https://github.com/%s/releases/latest/download/", repoSlug)

	tgz, err := download.Get(base + assetName)
	if err != nil {
		return "", err
	}
	sums, err := download.Get(base + "checksums.txt")
	if err != nil {
		return "", err
	}
	if err := download.VerifyChecksum(string(sums), assetName, tgz); err != nil {
		return "", err
	}
	return download.ExtractSingleFile(tgz, "sysup-worker")
}

// buildParuConfUpdate computes the [bin] Sudo = <authbridge> edit for the
// user's paru.conf without touching anything else in the file — creates the
// file/section if missing, replaces an existing Sudo line if present.
func buildParuConfUpdate() (path, newContent string, err error) {
	path, err = paruConfPath()
	if err != nil {
		return "", "", err
	}
	original := ""
	if data, err := os.ReadFile(path); err == nil {
		original = string(data)
	}
	return path, patchParuConfSudo(original, authbridgePath), nil
}

func paruConfPath() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	base := os.Getenv("XDG_CONFIG_HOME")
	if base == "" {
		base = filepath.Join(home, ".config")
	}
	return filepath.Join(base, "paru", "paru.conf"), nil
}

func patchParuConfSudo(original, authbridge string) string {
	sudoLine := "Sudo = " + authbridge
	if strings.TrimSpace(original) == "" {
		return "[bin]\n" + sudoLine + "\n"
	}

	lines := strings.Split(original, "\n")
	var out []string
	inBin := false
	binSeen := false
	sudoSet := false
	closeBin := func() {
		if inBin && !sudoSet {
			out = append(out, sudoLine)
			sudoSet = true
		}
	}
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "[") && strings.HasSuffix(trimmed, "]") {
			closeBin()
			inBin = trimmed == "[bin]"
			if inBin {
				binSeen = true
			}
			out = append(out, line)
			continue
		}
		if inBin && strings.HasPrefix(trimmed, "Sudo") && strings.Contains(trimmed, "=") {
			out = append(out, sudoLine)
			sudoSet = true
			continue
		}
		out = append(out, line)
	}
	closeBin()
	if !binSeen {
		if len(out) > 0 && strings.TrimSpace(out[len(out)-1]) != "" {
			out = append(out, "")
		}
		out = append(out, "[bin]", sudoLine)
	}
	return strings.Join(out, "\n")
}

// applyParuConf backs up the existing file (if any) to .bak, same pattern
// dotfiles/install.sh already uses before symlinking, then writes the
// patched content.
func applyParuConf(path, newContent string) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	if data, err := os.ReadFile(path); err == nil {
		if err := os.WriteFile(path+".bak", data, 0o644); err != nil {
			return err
		}
	}
	return os.WriteFile(path, []byte(newContent), 0o644)
}
