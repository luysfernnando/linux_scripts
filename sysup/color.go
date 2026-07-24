package main

import (
	"fmt"
	"os"
)

var colorEnabled = detectColorEnabled()

// detectColorEnabled follows the usual conventions: NO_COLOR/TERM=dumb
// disable it outright, otherwise it's on only when stdout is an actual
// terminal (not a pipe/file, where escape codes would just be noise).
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

// stepPalette rotates across step tags so concurrent steps are visually
// distinguishable in interleaved output, purely by index — deterministic
// across runs since BuildPipeline always emits steps in the same order.
var stepPalette = []string{ansiCyan, ansiMagenta, ansiYellow, ansiBlue, ansiGreen, ansiRed}

func colorize(code, s string) string {
	if !colorEnabled {
		return s
	}
	return code + s + ansiReset
}

func stepColor(i int) string {
	return stepPalette[i%len(stepPalette)]
}

func header(format string, a ...any) string {
	return colorize(ansiBold+ansiCyan, fmt.Sprintf(format, a...))
}

func ok(s string) string   { return colorize(ansiGreen, s) }
func fail(s string) string { return colorize(ansiRed, s) }
func warn(s string) string { return colorize(ansiYellow, s) }
func dim(s string) string  { return colorize(ansiDim, s) }
