package main

import (
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

func main() {
	args := os.Args[1:]
	if len(args) == 0 {
		args = []string{"update"}
	}
	cmd := args[0]
	rest := args[1:]

	switch cmd {
	case "version", "--version", "-v":
		fmt.Println("sysup", version)
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
		family, tools := DetectFamily(), DetectTools()
		if err := RunMirrorCheck(family, tools, dryRun, os.Stdout); err != nil {
			fmt.Fprintln(os.Stderr, "erro no ranking de mirrors:", err)
			os.Exit(1)
		}
	case "schedule":
		if err := InstallSchedule(); err != nil {
			fmt.Fprintln(os.Stderr, "erro instalando agendamento:", err)
			os.Exit(1)
		}
	case "gitkraken":
		if err := RunGitKraken(rest); err != nil {
			fmt.Fprintln(os.Stderr, "erro:", err)
			os.Exit(1)
		}
	case "tidewave":
		if err := RunTidewave(rest); err != nil {
			fmt.Fprintln(os.Stderr, "erro:", err)
			os.Exit(1)
		}
	default:
		fmt.Fprintf(os.Stderr, "uso: sysup [update|mirrors|schedule|gitkraken|tidewave|version] [--dry-run] [--no-self-update]\n")
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
		if SelfUpdate(dryRun) {
			if err := ReExec(); err != nil {
				fmt.Fprintln(os.Stderr, warn("aviso: self-update rodou mas re-exec falhou, continuando nesta versão: "+err.Error()))
			}
			// ReExec only returns on error; on success the process image is replaced.
		}
	}
	TryUpdateDotfilesRepo(dryRun)

	start := time.Now()
	family := DetectFamily()
	tools := DetectTools()

	parallelSteps, cleanupSteps := BuildPipeline(family, tools)
	if dueForMirrorCheck() {
		mirrorCheck := Step{Name: "mirrors", Run: func(dryRun bool, out io.Writer) error {
			return RunMirrorCheck(family, tools, dryRun, out)
		}}
		parallelSteps = append([]Step{mirrorCheck}, parallelSteps...)
	}

	var results []StepResult
	var err error
	if tuiAvailable(dryRun) {
		results, err = runUpdateTUI(family, parallelSteps, cleanupSteps)
	} else {
		results, err = runUpdatePlain(family, parallelSteps, cleanupSteps, dryRun)
	}

	elapsed := time.Since(start).Round(time.Second)

	if !dryRun {
		fmt.Println()
		fmt.Println(header("==> resumo"))
		for _, r := range results {
			status := ok("✔ ok")
			switch {
			case r.Err != nil:
				status = fail("✘ falhou")
			case r.Dur == 0:
				status = dim("− pulado")
			}
			fmt.Printf("  %-42s %s  %s\n", r.Name, status, dim(r.Dur.Round(time.Millisecond).String()))
		}
	}

	if err != nil {
		failedName := "desconhecido"
		for _, r := range results {
			if r.Err != nil {
				failedName = r.Name
				if strings.TrimSpace(r.Output) != "" {
					fmt.Fprintln(os.Stderr, fail(fmt.Sprintf("\n── saída de %q ──", r.Name)))
					fmt.Fprintln(os.Stderr, strings.TrimRight(r.Output, "\n"))
				}
				break
			}
		}
		fmt.Fprintf(os.Stderr, "%s %q: %v\n", fail("erro em"), failedName, err)
		Notify("Erro no update", fmt.Sprintf("Falhou em: %s", failedName))
		os.Exit(1)
	}
	Notify("Update completo", fmt.Sprintf("Sistema atualizado e limpo em %s", elapsed))
}

// runUpdatePlain is the fallback path for non-tty output (piped/logged runs,
// NO_COLOR) and --dry-run, where a full-screen dashboard wouldn't render
// (or wouldn't be worth it) — plain colored log lines, same as before the
// TUI existed.
func runUpdatePlain(family Family, parallelSteps, cleanupSteps []Step, dryRun bool) ([]StepResult, error) {
	fmt.Println(header("==> sysup update (%s)", family))

	results, err := RunParallel(parallelSteps, dryRun)
	if err == nil {
		for i := range cleanupSteps {
			s := &cleanupSteps[i]
			cstart := time.Now()
			runErr := s.Run(dryRun, os.Stdout)
			cdur := time.Since(cstart)
			results = append(results, StepResult{Name: s.Name, Dur: cdur, Err: runErr})
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
func runUpdateTUI(family Family, parallelSteps, cleanupSteps []Step) ([]StepResult, error) {
	allSteps := make([]Step, 0, len(parallelSteps)+len(cleanupSteps))
	allSteps = append(allSteps, parallelSteps...)
	allSteps = append(allSteps, cleanupSteps...)

	names := make([]string, len(allSteps))
	for i, s := range allSteps {
		names[i] = s.Name
	}

	stopSudo := primeSudo(allSteps, false)
	defer stopSudo()

	p := tea.NewProgram(newUpdateModel(string(family), names), tea.WithAltScreen())

	var results []StepResult
	var pipelineErr error
	go func() {
		parallelResults := runStepsTUI(p, parallelSteps, false, 0)
		results = append(results, parallelResults...)
		if perr := firstErr(parallelResults); perr != nil {
			pipelineErr = perr
			results = append(results, skipSteps(p, cleanupSteps, len(parallelSteps))...)
		} else {
			cleanupResults := runStepsTUI(p, cleanupSteps, false, len(parallelSteps))
			results = append(results, cleanupResults...)
			pipelineErr = firstErr(cleanupResults)
		}
		p.Send(pipelineDoneMsg{})
	}()

	if _, err := p.Run(); err != nil {
		fmt.Fprintln(os.Stderr, warn("aviso: dashboard falhou, saída pode estar incompleta: "+err.Error()))
	}

	return results, pipelineErr
}
