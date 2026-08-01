// Package cli is sysup's command dispatch and the `update` orchestration
// (self-update, pipeline build, TUI/plain runner, summary, notification) —
// the thin cmd/sysup/main.go just calls Run().
package cli

import (
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"

	"sysup/internal/detect"
	"sysup/internal/mirrors"
	"sysup/internal/notify"
	"sysup/internal/pipeline"
	"sysup/internal/polkit"
	"sysup/internal/schedule"
	"sysup/internal/selfupdate"
	"sysup/internal/style"
	"sysup/internal/tools"
	"sysup/internal/tui"
)

// Run is sysup's entrypoint logic — parses argv and dispatches to the
// matching subcommand.
func Run() {
	args := os.Args[1:]
	if len(args) == 0 {
		args = []string{"update"}
	}
	cmd := args[0]
	rest := args[1:]

	switch cmd {
	case "version", "--version", "-v":
		fmt.Println("sysup", selfupdate.Version)
	case "update":
		dryRun := false
		noSelfUpdate := false
		for _, a := range rest {
			switch a {
			case "--dry-run":
				dryRun = true
			case "--no-self-update":
				noSelfUpdate = true
			}
		}
		runUpdate(dryRun, noSelfUpdate)
	case "mirrors":
		dryRun := hasFlag(rest, "--dry-run")
		family, t := detect.DetectFamily(), detect.DetectTools()
		if err := mirrors.RunCheck(family, t, dryRun, os.Stdout); err != nil {
			fmt.Fprintln(os.Stderr, "erro no ranking de mirrors:", err)
			os.Exit(1)
		}
	case "schedule":
		if err := schedule.InstallSchedule(); err != nil {
			fmt.Fprintln(os.Stderr, "erro instalando agendamento:", err)
			os.Exit(1)
		}
	case "gitkraken":
		if err := tools.RunGitKraken(rest); err != nil {
			fmt.Fprintln(os.Stderr, "erro:", err)
			os.Exit(1)
		}
	case "tidewave":
		if err := tools.RunTidewave(rest); err != nil {
			fmt.Fprintln(os.Stderr, "erro:", err)
			os.Exit(1)
		}
	case "polkit-setup":
		if err := polkit.RunSetup(hasFlag(rest, "--dry-run")); err != nil {
			fmt.Fprintln(os.Stderr, "erro:", err)
			os.Exit(1)
		}
	default:
		fmt.Fprintf(os.Stderr, "uso: sysup [update|mirrors|schedule|gitkraken|tidewave|polkit-setup|version] [--dry-run] [--no-self-update]\n")
		os.Exit(2)
	}
}

func hasFlag(args []string, flag string) bool {
	for _, a := range args {
		if a == flag {
			return true
		}
	}
	return false
}

func runUpdate(dryRun, noSelfUpdate bool) {
	if !noSelfUpdate {
		if selfupdate.SelfUpdate(dryRun) {
			if err := selfupdate.ReExec(); err != nil {
				fmt.Fprintln(os.Stderr, style.Warn("aviso: self-update rodou mas re-exec falhou, continuando nesta versão: "+err.Error()))
			}
			// ReExec only returns on error; on success the process image is replaced.
		}
	}
	selfupdate.TryUpdateDotfilesRepo(dryRun)

	start := time.Now()
	family := detect.DetectFamily()
	t := detect.DetectTools()

	worker, werr := polkit.StartWorker(dryRun)
	if werr != nil {
		fmt.Fprintln(os.Stderr, style.Warn("aviso: worker privilegiado do polkit falhou, caindo pro sudo clássico: "+werr.Error()))
		worker = nil
	}
	defer worker.Close()

	parallelSteps, cleanupSteps := pipeline.BuildPipeline(family, t, worker)
	if mirrors.DueForCheck() {
		mirrorCheck := pipeline.Step{Name: "Mirrors", Run: func(dryRun bool, out io.Writer) error {
			return mirrors.RunCheck(family, t, dryRun, out)
		}}
		parallelSteps = append([]pipeline.Step{mirrorCheck}, parallelSteps...)
	}

	usedTUI := tui.Available(dryRun)
	var results []pipeline.StepResult
	var err error
	if usedTUI {
		results, err = runUpdateTUI(family, parallelSteps, cleanupSteps)
	} else {
		results, err = runUpdatePlain(family, parallelSteps, cleanupSteps, dryRun)
	}

	elapsed := time.Since(start).Round(time.Second)

	if !dryRun {
		fmt.Println()
		if usedTUI {
			fmt.Println(tui.RenderSummaryBox(results, elapsed))
		} else {
			fmt.Println(style.Header("==> resumo"))
			for _, r := range results {
				status := style.Ok("✔ ok")
				switch {
				case r.Err != nil:
					status = style.Fail("✘ falhou")
				case r.Dur == 0:
					status = style.Dim("− pulado")
				}
				fmt.Printf("  %-42s %s  %s\n", r.Name, status, style.Dim(r.Dur.Round(time.Millisecond).String()))
			}
		}
	}

	if err != nil {
		failedName := "desconhecido"
		for _, r := range results {
			if r.Err != nil {
				failedName = r.Name
				if strings.TrimSpace(r.Output) != "" {
					fmt.Fprintln(os.Stderr, style.Fail(fmt.Sprintf("\n── saída de %q ──", r.Name)))
					fmt.Fprintln(os.Stderr, strings.TrimRight(r.Output, "\n"))
				}
				break
			}
		}
		fmt.Fprintf(os.Stderr, "%s %q: %v\n", style.Fail("erro em"), failedName, err)
		notify.Notify("Erro no update", fmt.Sprintf("Falhou em: %s", failedName))
		os.Exit(1)
	}
	notify.Notify("Update completo", fmt.Sprintf("Sistema atualizado e limpo em %s", elapsed))
}

// runUpdatePlain is the fallback path for non-tty output (piped/logged runs,
// NO_COLOR) and --dry-run, where a full-screen dashboard wouldn't render
// (or wouldn't be worth it) — plain colored log lines, same as before the
// TUI existed.
func runUpdatePlain(family detect.Family, parallelSteps, cleanupSteps []pipeline.Step, dryRun bool) ([]pipeline.StepResult, error) {
	fmt.Println(style.Header("==> sysup update (%s)", family))

	results, err := pipeline.RunParallel(parallelSteps, dryRun)
	if err == nil {
		for i := range cleanupSteps {
			s := &cleanupSteps[i]
			cstart := time.Now()
			runErr := s.Run(dryRun, os.Stdout)
			cdur := time.Since(cstart)
			results = append(results, pipeline.StepResult{Name: s.Name, Dur: cdur, Err: runErr})
			if runErr != nil {
				err = runErr
				break
			}
		}
	}
	return results, err
}

// runUpdateTUI drives the same pipeline through a full-screen Bubble Tea
// dashboard: sudo is primed up front (must happen before the alt screen
// takes over the terminal, so it can still prompt normally), then the
// pipeline runs in a background goroutine sending progress messages while
// Program.Run blocks in the foreground rendering them.
func runUpdateTUI(family detect.Family, parallelSteps, cleanupSteps []pipeline.Step) ([]pipeline.StepResult, error) {
	allSteps := make([]pipeline.Step, 0, len(parallelSteps)+len(cleanupSteps))
	allSteps = append(allSteps, parallelSteps...)
	allSteps = append(allSteps, cleanupSteps...)

	names := make([]string, len(allSteps))
	for i, s := range allSteps {
		names[i] = s.Name
	}

	stopSudo := pipeline.PrimeSudo(allSteps, false)
	defer stopSudo()

	p := tea.NewProgram(tui.NewUpdateModel(string(family), names), tea.WithAltScreen())

	var results []pipeline.StepResult
	var pipelineErr error
	go func() {
		parallelResults := tui.RunStepsTUI(p, parallelSteps, false, 0)
		results = append(results, parallelResults...)
		if perr := tui.FirstErr(parallelResults); perr != nil {
			pipelineErr = perr
			results = append(results, tui.SkipSteps(p, cleanupSteps, len(parallelSteps))...)
		} else {
			cleanupResults := tui.RunStepsTUI(p, cleanupSteps, false, len(parallelSteps))
			results = append(results, cleanupResults...)
			pipelineErr = tui.FirstErr(cleanupResults)
		}
		p.Send(tui.PipelineDoneMsg{})
	}()

	if _, err := p.Run(); err != nil {
		fmt.Fprintln(os.Stderr, style.Warn("aviso: dashboard falhou, saída pode estar incompleta: "+err.Error()))
	}

	return results, pipelineErr
}
