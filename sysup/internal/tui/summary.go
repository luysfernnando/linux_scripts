package tui

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"
)

// peakPkgTotal returns the largest TOTAL seen in any "(N/TOTAL)" line in
// output (pkgProgressRe, tui.go) — that's the number of packages pacman
// actually touched, whether installing or removing.
func peakPkgTotal(output string) int {
	max := 0
	for _, m := range pkgProgressRe.FindAllStringSubmatch(output, -1) {
		if n, err := strconv.Atoi(m[2]); err == nil && n > max {
			max = n
		}
	}
	return max
}

var npmChangedRe = regexp.MustCompile(`changed (\d+) package`)

// summarizeStep turns a step's raw captured output into one short
// declarative line for the final summary ("42 pacote(s) atualizado(s)", "Já
// estava tudo atualizado") instead of a bare "ok". Returns "" when nothing
// tool-specific could be inferred — callers fall back to a generic label.
func summarizeStep(name, output string) string {
	switch {
	case strings.Contains(name, "Pacotes do sistema"), strings.Contains(name, "+ AUR"):
		return summarizePkgUpdate(output)
	case strings.HasPrefix(name, "Órfãos"):
		return summarizeCleanup(output)
	case strings.HasPrefix(name, "Npm"):
		return summarizeNpm(output)
	case name == "Flatpak":
		return summarizeFlatpak(output)
	}
	return ""
}

func summarizePkgUpdate(output string) string {
	if n := peakPkgTotal(output); n > 0 {
		return fmt.Sprintf("%d pacote(s) atualizado(s)", n)
	}
	if strings.Contains(output, "there is nothing to do") {
		return "Já estava tudo atualizado"
	}
	return ""
}

func summarizeCleanup(output string) string {
	if n := peakPkgTotal(output); n > 0 {
		return fmt.Sprintf("%d pacote(s) removido(s)", n)
	}
	if strings.Contains(output, "sem pacotes órfãos") {
		return "Nenhum pacote órfão"
	}
	return ""
}

func summarizeNpm(output string) string {
	if m := npmChangedRe.FindStringSubmatch(output); m != nil {
		return m[1] + " pacote(s) atualizado(s)"
	}
	if strings.Contains(output, "up to date") {
		return "Já estava tudo atualizado"
	}
	return ""
}

func summarizeFlatpak(output string) string {
	if strings.Contains(output, "Nothing to update") {
		return "Já estava tudo atualizado"
	}
	return ""
}
