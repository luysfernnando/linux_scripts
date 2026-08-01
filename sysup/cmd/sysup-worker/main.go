// Command sysup-worker is the privileged half of sysup's polkit integration.
// It has two personalities, chosen by the name it's invoked as (argv[0]) —
// see sysup/README.md for the full architecture:
//
//   - sysup-worker: run via `pkexec` as root. Binds a Unix socket, re-detects
//     the machine's family/tools itself (never trusts the caller), and
//     executes only an exact whitelist of update/cleanup commands received
//     over that socket. Lives only for the duration of one `sysup update`
//     run — its lifetime is tied to its inherited stdin pipe, not installed
//     as any kind of persistent service.
//   - sysup-authbridge: a symlink to this same binary, run unprivileged as
//     paru's configured `[bin] Sudo` replacement. Forwards the exact argv
//     paru would have handed to `sudo` over to the already-authorized
//     worker socket, so paru's own privilege escalation never prompts.
package main

import (
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"os/user"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"sysup/internal/workerproto"
)

func main() {
	if filepath.Base(os.Args[0]) == "sysup-authbridge" {
		bridgeMain(os.Args[1:])
		return
	}
	serverMain(os.Args[1:])
}

// bridgeMain runs unprivileged, as whoever paru is running as. It exists
// purely to satisfy paru's expectation of a synchronous "sudo replacement"
// binary: read $SYSUP_WORKER_SOCKET, forward argv, relay output/exit code.
//
// This is paru's ONLY privilege-escalation path once `sysup polkit-setup`
// points paru.conf's [bin] Sudo at it — including when paru is run by hand,
// completely outside `sysup update` (no SYSUP_WORKER_SOCKET set, no worker
// running). So whenever the worker isn't reachable, or rejects the request
// before running anything, this falls back to real sudo — paru must keep
// working normally on its own, not only inside a sysup run.
func bridgeMain(argv []string) {
	if sock := os.Getenv("SYSUP_WORKER_SOCKET"); sock != "" {
		if tryWorker(sock, argv) {
			return
		}
	}
	execRealSudo(argv)
}

// tryWorker attempts the request against the worker. Returns true if it
// handled the request start to finish (success or a real execution
// failure — either way, nothing left to fall back to). Returns false only
// when nothing ran yet (connection failed, or the worker rejected the
// request outright) — safe for the caller to retry via sudo.
func tryWorker(sock string, argv []string) bool {
	conn, err := net.DialTimeout("unix", sock, 3*time.Second)
	if err != nil {
		return false
	}
	defer conn.Close()

	if err := workerproto.WriteRequest(conn, argv); err != nil {
		return false
	}

	status, msg, err := workerproto.RelayOutput(os.Stdout, conn)
	if err != nil {
		return false
	}
	switch status {
	case workerproto.StatusOK:
		return true
	case workerproto.StatusFailed:
		// The worker DID run this and it failed — surfacing it as-is and
		// stopping here, never falling back, since retrying via sudo would
		// re-run a command that may have already had side effects.
		fmt.Fprintln(os.Stderr, "sysup-authbridge:", msg)
		os.Exit(1)
		return true
	default: // StatusRejected: nothing ran, safe to fall back
		return false
	}
}

// execRealSudo is the plain fallback: run the exact argv paru handed us
// through the system's real sudo, prompting normally if needed. Resolved
// fresh via PATH (never assumes a path), since this binary IS what
// paru.conf's `Sudo =` now points to — it must never call itself.
func execRealSudo(argv []string) {
	sudoPath, err := exec.LookPath("sudo")
	if err != nil {
		fmt.Fprintln(os.Stderr, "sysup-authbridge: sudo não encontrado no PATH:", err)
		os.Exit(1)
	}
	cmd := exec.Command(sudoPath, argv...)
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			os.Exit(exitErr.ExitCode())
		}
		fmt.Fprintln(os.Stderr, "sysup-authbridge:", err)
		os.Exit(1)
	}
}

// serverMain runs as root (via pkexec). It never trusts anything about the
// caller's environment beyond PKEXEC_UID (set by polkit itself, not the
// caller's own env) and only ever executes commands it independently
// resolves and validates against allowedCommand's whitelist.
func serverMain(args []string) {
	socketPath := workerproto.SocketPath()
	for i := 0; i < len(args); i++ {
		if args[i] == "--socket" && i+1 < len(args) {
			socketPath = args[i+1]
			i++
		}
	}

	if err := os.Remove(socketPath); err != nil && !os.IsNotExist(err) {
		fmt.Fprintln(os.Stderr, "sysup-worker: não consegui limpar socket antigo:", err)
		os.Exit(1)
	}

	ln, err := net.Listen("unix", socketPath)
	if err != nil {
		fmt.Fprintln(os.Stderr, "sysup-worker: bind falhou:", err)
		os.Exit(1)
	}
	defer ln.Close()
	if err := os.Chmod(socketPath, 0o600); err != nil {
		fmt.Fprintln(os.Stderr, "sysup-worker: chmod do socket falhou:", err)
		os.Exit(1)
	}
	// The socket is created by us while running as root, so it starts out
	// root:root — 0600 in that state means only root could ever connect,
	// locking out the very user pkexec authorized this for. Chown it to
	// PKEXEC_UID (set by polkit itself, not the caller) so "owner-only"
	// actually means the real user, not root; root can still connect
	// regardless, since the kernel exempts root from file permission
	// checks entirely.
	if uidStr := os.Getenv("PKEXEC_UID"); uidStr != "" {
		if u, err := user.LookupId(uidStr); err == nil {
			uid, uerr := strconv.Atoi(u.Uid)
			gid, gerr := strconv.Atoi(u.Gid)
			if uerr == nil && gerr == nil {
				if err := os.Chown(socketPath, uid, gid); err != nil {
					fmt.Fprintln(os.Stderr, "sysup-worker: chown do socket falhou:", err)
					os.Exit(1)
				}
			}
		}
	}

	// Tie our lifetime to the parent sysup process: it holds the write end
	// of a pipe wired to our stdin for exactly as long as `sysup update`
	// runs. When it exits (normally or crashed), we get EOF here and shut
	// down — this worker never outlives a single run. The timer is a
	// defense-in-depth fallback only, in case the pipe is somehow never
	// closed (e.g. spawned by hand for testing).
	done := make(chan struct{})
	go func() {
		_, _ = io.Copy(io.Discard, os.Stdin)
		close(done)
	}()
	go func() {
		select {
		case <-done:
		case <-time.After(15 * time.Minute):
		}
		_ = ln.Close()
		os.Exit(0)
	}()

	fmt.Println("READY")

	for {
		conn, err := ln.Accept()
		if err != nil {
			return
		}
		go handleConn(conn)
	}
}

func handleConn(conn net.Conn) {
	defer conn.Close()

	argv, err := workerproto.ReadRequest(conn)
	if err != nil {
		_ = workerproto.WriteTrailer(conn, workerproto.StatusRejected, "pedido inválido: "+err.Error())
		return
	}

	resolved, ok := allowedCommand(argv)
	if !ok {
		_ = workerproto.WriteTrailer(conn, workerproto.StatusRejected, "comando não permitido: "+strings.Join(argv, " "))
		return
	}

	cmd := exec.Command(resolved[0], resolved[1:]...)
	cmd.Stdout = conn
	cmd.Stderr = conn
	runErr := cmd.Run()
	if runErr != nil {
		_ = workerproto.WriteTrailer(conn, workerproto.StatusFailed, runErr.Error())
		return
	}
	_ = workerproto.WriteTrailer(conn, workerproto.StatusOK, "")
}

// allowedCommand re-resolves argv[0] on PATH itself (never trusts a path
// string arriving over the socket) and checks the full argument list
// against an exact whitelist — no shell, no wildcards beyond the one
// deliberately scoped case (AUR packages already built into the caller's
// own yay/paru cache dir). Anything else is rejected.
func allowedCommand(argv []string) (resolved []string, ok bool) {
	if len(argv) == 0 {
		return nil, false
	}
	base := filepath.Base(argv[0])
	args := argv[1:]

	path, err := exec.LookPath(base)
	if err != nil {
		return nil, false
	}

	switch base {
	case "pacman":
		if resolvedArgs, ok := allowedPacmanArgs(args); ok {
			return prepend(path, resolvedArgs), true
		}
	case "paccache":
		if argsEqual(args, "-r") {
			return prepend(path, args), true
		}
	case "apt-get":
		switch {
		case argsEqual(args, "update"),
			argsEqual(args, "full-upgrade", "-y"),
			argsEqual(args, "autoremove", "-y"),
			argsEqual(args, "autoclean", "-y"):
			return prepend(path, args), true
		}
	case "dnf":
		switch {
		case argsEqual(args, "upgrade", "-y"),
			argsEqual(args, "autoremove", "-y"),
			argsEqual(args, "clean", "all"):
			return prepend(path, args), true
		}
	case "zypper":
		if argsEqual(args, "update", "-y") {
			return prepend(path, args), true
		}
	case "npm":
		// Exact commands sysup's own pipeline ever issues for the global
		// npm step — not a general "npm can do anything as root" grant.
		switch {
		case argsEqual(args, "install", "-g", "npm@latest"),
			argsEqual(args, "update", "-g"):
			return prepend(path, args), true
		}
	}
	return nil, false
}

func prepend(path string, args []string) []string {
	return append([]string{path}, args...)
}

func argsEqual(args []string, want ...string) bool {
	if len(args) != len(want) {
		return false
	}
	for i := range args {
		if args[i] != want[i] {
			return false
		}
	}
	return true
}

// pacmanFlagAliases maps pacman's long-form flags to a canonical name also
// reachable via the equivalent short-form letter (see decodeShortFlags) —
// so "--sync -y -u --noconfirm" and "-Syu --noconfirm" resolve to the exact
// same flag set. paru calls pacman with the long/split form; sysup's own
// pipeline (pipeline.go) uses the short combined form — both need to match
// the same whitelist entry without listing every literal spelling.
var pacmanFlagAliases = map[string]string{
	"--sync":       "S",
	"--refresh":    "y",
	"--sysupgrade": "u",
	"--remove":     "R",
	"--nosave":     "n",
	"--recursive":  "s",
	"--upgrade":    "U",
	"--noconfirm":  "noconfirm",
}

// pacmanRequest is argv[1:] decomposed into its flag set and non-flag
// operands (package names, file paths) — order-independent, so it doesn't
// matter whether pacman was invoked with combined short flags, split short
// flags, or long flags.
type pacmanRequest struct {
	flags    map[string]bool
	operands []string
}

// parsePacmanArgs decodes args into flags+operands, trimming the single
// trailing "--" separator paru always appends. Any token it doesn't
// recognize (an unknown flag) fails closed — returns ok=false rather than
// guessing, since a misparsed flag here would mean the whitelist check
// below it is checking the wrong thing.
func parsePacmanArgs(args []string) (pacmanRequest, bool) {
	if len(args) > 0 && args[len(args)-1] == "--" {
		args = args[:len(args)-1]
	}
	req := pacmanRequest{flags: map[string]bool{}}
	for _, a := range args {
		if a == "" {
			continue
		}
		if a[0] != '-' {
			req.operands = append(req.operands, a)
			continue
		}
		if canon, ok := pacmanFlagAliases[a]; ok {
			req.flags[canon] = true
			continue
		}
		if len(a) > 1 && a[1] != '-' {
			for _, ch := range a[1:] {
				switch ch {
				case 'S', 'y', 'u', 'R', 'n', 's', 'U':
					req.flags[string(ch)] = true
				default:
					return pacmanRequest{}, false
				}
			}
			continue
		}
		return pacmanRequest{}, false
	}
	return req, true
}

func (r pacmanRequest) flagsOnly(want ...string) bool {
	if len(r.flags) != len(want) {
		return false
	}
	for _, f := range want {
		if !r.flags[f] {
			return false
		}
	}
	return true
}

// allowedPacmanArgs is the actual pacman whitelist: exactly "refresh the
// databases and upgrade everything," "remove this exact list of orphan
// packages," or "install this one AUR package already built into the
// caller's own cache dir" — nothing else. Returns the args pacman should
// actually be invoked with (always the canonical short form, regardless of
// which form the caller used) so the executed command line is predictable
// and log-able, independent of how it arrived.
func allowedPacmanArgs(args []string) ([]string, bool) {
	req, ok := parsePacmanArgs(args)
	if !ok {
		return nil, false
	}
	switch {
	case req.flagsOnly("S", "y", "u", "noconfirm") && len(req.operands) == 0:
		return []string{"-Syu", "--noconfirm"}, true
	case req.flagsOnly("R", "n", "s", "noconfirm") && len(req.operands) > 0:
		return append([]string{"-Rns", "--noconfirm"}, req.operands...), true
	case req.flagsOnly("U", "noconfirm") && len(req.operands) == 1 && aurCachePathOK(req.operands[0]):
		return []string{"-U", "--noconfirm", req.operands[0]}, true
	}
	return nil, false
}

// aurCachePathOK validates that path resolves inside ~/.cache/yay/ or
// ~/.cache/paru/ of the UID polkit says actually authenticated (PKEXEC_UID,
// set by polkit itself) — never the caller-supplied environment, which
// could otherwise be forged to point anywhere.
func aurCachePathOK(path string) bool {
	uidStr := os.Getenv("PKEXEC_UID")
	if uidStr == "" {
		return false
	}
	u, err := user.LookupId(uidStr)
	if err != nil {
		return false
	}
	abs, err := filepath.Abs(path)
	if err != nil {
		return false
	}
	abs = filepath.Clean(abs)
	for _, sub := range []string{"yay", "paru"} {
		root := filepath.Clean(filepath.Join(u.HomeDir, ".cache", sub))
		if abs == root || strings.HasPrefix(abs, root+string(filepath.Separator)) {
			return true
		}
	}
	return false
}
