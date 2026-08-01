// Package style holds the ANSI color helpers shared across the whole
// sysup CLI (pipeline output, polkit-setup, self-update, mirrors, tool
// installers) — kept as its own leaf package (no internal deps) so that
// internal/pipeline and internal/tui can both depend on it without risking
// a cycle between them.
package style

import (
	"fmt"
	"os"
)

// ColorEnabled follows the usual conventions: NO_COLOR/TERM=dumb disable it
// outright, otherwise it's on only when stdout is an actual terminal (not a
// pipe/file, where escape codes would just be noise).
var ColorEnabled = detectColorEnabled()

func detectColorEnabled() bool {
	if os.Getenv("NO_COLOR") != "" || os.Getenv("TERM") == "dumb" {
		return false
	}
	fi, err := os.Stdout.Stat()
	if err != nil {
		return false
	}
	return (fi.Mode() & os.ModeCharDevice) != 0
}

const (
	ansiReset   = "\033[0m"
	ansiBold    = "\033[1m"
	ansiDim     = "\033[2m"
	ansiRed     = "\033[31m"
	ansiGreen   = "\033[32m"
	ansiYellow  = "\033[33m"
	ansiBlue    = "\033[34m"
	ansiMagenta = "\033[35m"
	ansiCyan    = "\033[36m"
)

// StepPalette rotates across step tags so concurrent steps are visually
// distinguishable in interleaved output, purely by index — deterministic
// across runs since BuildPipeline always emits steps in the same order.
var StepPalette = []string{ansiCyan, ansiMagenta, ansiYellow, ansiBlue, ansiGreen, ansiRed}

func Colorize(code, s string) string {
	if !ColorEnabled {
		return s
	}
	return code + s + ansiReset
}

func StepColor(i int) string {
	return StepPalette[i%len(StepPalette)]
}

func Header(format string, a ...any) string {
	return Colorize(ansiBold+ansiCyan, fmt.Sprintf(format, a...))
}

func Ok(s string) string   { return Colorize(ansiGreen, s) }
func Fail(s string) string { return Colorize(ansiRed, s) }
func Warn(s string) string { return Colorize(ansiYellow, s) }
func Dim(s string) string  { return Colorize(ansiDim, s) }
