// Package workerproto is the tiny wire protocol between sysup (or
// sysup-authbridge) and sysup-worker: one JSON request line naming the exact
// command to run, followed by the command's raw combined stdout/stderr, then
// a trailer line marking success/failure. Shared by both client sides
// (sysup's own worker.go, and sysup-authbridge's paru-facing bridge) so the
// framing logic exists in exactly one place — worker.go/main packages can't
// import each other (both are `package main`), this can.
package workerproto

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
)

// SocketPath is the per-user runtime location of the worker's Unix socket.
// $XDG_RUNTIME_DIR is systemd-logind's 0700-per-user directory — the socket
// inherits that as its only access control, appropriate for a personal
// single-user machine, not a shared/multi-tenant one.
func SocketPath() string {
	dir := os.Getenv("XDG_RUNTIME_DIR")
	if dir == "" {
		dir = filepath.Join(os.TempDir(), fmt.Sprintf("sysup-%d", os.Getuid()))
		_ = os.MkdirAll(dir, 0o700)
	}
	return filepath.Join(dir, "sysup-worker.sock")
}

type request struct {
	Argv []string `json:"argv"`
}

// WriteRequest sends one command (already split into argv form, no shell
// involved anywhere in this protocol) as a single JSON line.
func WriteRequest(w io.Writer, argv []string) error {
	return json.NewEncoder(w).Encode(request{Argv: argv})
}

// ReadRequest reads back what WriteRequest sent.
func ReadRequest(r io.Reader) ([]string, error) {
	var req request
	if err := json.NewDecoder(r).Decode(&req); err != nil {
		return nil, err
	}
	return req.Argv, nil
}

// trailerMarker separates raw command output from the final status line. A
// NUL byte essentially never appears in legitimate pacman/apt/dnf/paccache
// output, so it's a safe-enough sentinel for this local, single-purpose
// socket — this isn't a general-purpose framing protocol.
const trailerMarker = 0x00

// Status distinguishes WHY a request didn't succeed, because that changes
// what's safe for a caller to do next:
//   - Rejected: the worker never ran anything (bad request, or the argv
//     didn't match the whitelist) — safe to retry some other way (e.g.
//     sysup-authbridge falling back to real sudo).
//   - Failed: the worker DID execute the command and it returned an error —
//     retrying via another path would risk re-running something with side
//     effects (installing/removing packages twice). Callers must surface
//     this as a hard failure, never silently retry.
type Status byte

const (
	StatusOK       Status = 'O'
	StatusRejected Status = 'R'
	StatusFailed   Status = 'F'
)

// WriteTrailer sends the final status line.
func WriteTrailer(w io.Writer, status Status, msg string) error {
	_, err := fmt.Fprintf(w, "%c%c%s\n", trailerMarker, status, msg)
	return err
}

// RelayOutput copies src to dst until the trailer marker shows up, then
// returns the trailer's status/message. Used by both client sides to turn
// the raw byte stream back into "did it work, and is it safe to retry."
func RelayOutput(dst io.Writer, src io.Reader) (status Status, msg string, err error) {
	r := bufio.NewReader(src)
	buf := make([]byte, 4096)
	for {
		n, rerr := r.Read(buf)
		if n > 0 {
			chunk := buf[:n]
			if idx := bytes.IndexByte(chunk, trailerMarker); idx >= 0 {
				if idx > 0 {
					if _, werr := dst.Write(chunk[:idx]); werr != nil {
						return 0, "", werr
					}
				}
				tail := append([]byte{}, chunk[idx+1:]...)
				line, lerr := r.ReadBytes('\n')
				if lerr != nil && lerr != io.EOF {
					return 0, "", lerr
				}
				tail = append(tail, line...)
				tail = bytes.TrimRight(tail, "\n")
				if len(tail) == 0 {
					return 0, "", fmt.Errorf("trailer de status vazio")
				}
				return Status(tail[0]), string(tail[1:]), nil
			}
			if _, werr := dst.Write(chunk); werr != nil {
				return 0, "", werr
			}
		}
		if rerr != nil {
			if rerr == io.EOF {
				return 0, "", fmt.Errorf("conexão do worker encerrada sem trailer de status")
			}
			return 0, "", rerr
		}
	}
}
