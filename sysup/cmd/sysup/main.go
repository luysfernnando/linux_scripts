// Command sysup is a cross-distro/cross-OS system-update CLI. See
// sysup/README.md and the top-level CLAUDE.md for the full architecture —
// this file is intentionally just an entrypoint into internal/cli.
package main

import "sysup/internal/cli"

func main() {
	cli.Run()
}
