// Package notify fires a native desktop notification when possible.
package notify

import (
	"fmt"
	"os/exec"
	"runtime"

	"sysup/internal/detect"
)

// Notify fires a native desktop notification when possible; otherwise it
// just prints, so a run over SSH or on a bare system never fails on this.
func Notify(title, body string) {
	switch {
	case runtime.GOOS == "linux" && detect.HasTool("notify-send"):
		_ = exec.Command("notify-send", title, body).Run()
	case runtime.GOOS == "darwin":
		script := fmt.Sprintf(`display notification %q with title %q`, body, title)
		_ = exec.Command("osascript", "-e", script).Run()
	case runtime.GOOS == "windows" && detect.HasTool("msg"):
		_ = exec.Command("msg", "*", fmt.Sprintf("%s: %s", title, body)).Run()
	default:
		fmt.Printf("%s: %s\n", title, body)
	}
}
