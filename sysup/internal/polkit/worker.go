package polkit

import (
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"strings"
	"time"

	"sysup/internal/workerproto"
)

// WorkerClient is a handle to a live sysup-worker process, authorized once
// via pkexec and reused for every privileged step in this run.
type WorkerClient struct {
	socketPath string
	cmd        *exec.Cmd
	stdinW     io.WriteCloser
}

// StartWorker spawns sysup-worker via pkexec exactly once, before any TUI
// takes over the terminal (same timing PrimeSudo uses today) — the one
// polkit authorization that covers the whole run. Returns (nil, nil) when
// polkit hasn't been set up on this machine or dryRun is set; callers fall
// back to the classic sudo path in that case.
func StartWorker(dryRun bool) (*WorkerClient, error) {
	if dryRun || !Configured() {
		return nil, nil
	}

	socketPath := workerproto.SocketPath()

	stdinR, stdinW, err := os.Pipe()
	if err != nil {
		return nil, err
	}

	cmd := exec.Command("pkexec", helperInstallPath, "--socket", socketPath)
	cmd.Stdin = stdinR
	cmd.Stderr = os.Stderr
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		stdinW.Close()
		return nil, err
	}

	if err := cmd.Start(); err != nil {
		stdinW.Close()
		return nil, err
	}
	stdinR.Close()

	ready := make(chan error, 1)
	go func() {
		buf := make([]byte, 32)
		n, rerr := stdout.Read(buf)
		if rerr != nil && n == 0 {
			ready <- rerr
			return
		}
		if !strings.HasPrefix(string(buf[:n]), "READY") {
			ready <- fmt.Errorf("resposta inesperada do worker: %q", buf[:n])
			return
		}
		ready <- nil
	}()

	select {
	case err := <-ready:
		if err != nil {
			stdinW.Close()
			_ = cmd.Wait()
			return nil, fmt.Errorf("autorização do worker falhou: %w", err)
		}
	case <-time.After(5 * time.Minute):
		stdinW.Close()
		_ = cmd.Process.Kill()
		return nil, fmt.Errorf("timeout esperando autorização do polkit")
	}

	return &WorkerClient{socketPath: socketPath, cmd: cmd, stdinW: stdinW}, nil
}

// Run sends one exact command (already resolved to argv form, no shell) to
// the already-authorized worker and streams its output into out — same
// shape as a plain shell runner, so callers' output capture needs no
// per-source changes.
func (w *WorkerClient) Run(argv []string, out io.Writer) error {
	if w == nil {
		return fmt.Errorf("worker privilegiado não está rodando")
	}
	conn, err := net.DialTimeout("unix", w.socketPath, 5*time.Second)
	if err != nil {
		return fmt.Errorf("conectando no worker: %w", err)
	}
	defer conn.Close()

	if err := workerproto.WriteRequest(conn, argv); err != nil {
		return fmt.Errorf("mandando pedido pro worker: %w", err)
	}

	status, msg, err := workerproto.RelayOutput(out, conn)
	if err != nil {
		return err
	}
	if status != workerproto.StatusOK {
		return fmt.Errorf("%s", msg)
	}
	return nil
}

// Close signals the worker to shut down (closing its stdin pipe, which it's
// blocked reading from) and reaps the process. Safe on a nil client.
func (w *WorkerClient) Close() {
	if w == nil {
		return
	}
	_ = w.stdinW.Close()
	_ = w.cmd.Wait()
}

// SocketEnv returns the KEY=VALUE pair to add to a step's environment (e.g.
// paru, via its sysup-authbridge [bin] Sudo hook) so it can reach the
// already-authorized worker directly instead of prompting itself.
func (w *WorkerClient) SocketEnv() string {
	if w == nil {
		return ""
	}
	return "SYSUP_WORKER_SOCKET=" + w.socketPath
}
