package main

import (
	"fmt"
	"io"
	"os"
	"time"
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
				fmt.Fprintln(os.Stderr, "aviso: self-update rodou mas re-exec falhou, continuando nesta versão:", err)
			}
			// ReExec only returns on error; on success the process image is replaced.
		}
	}
	TryUpdateDotfilesRepo(dryRun)

	start := time.Now()
	family := DetectFamily()
	tools := DetectTools()
	fmt.Println(header("==> sysup update (%s)", family))

	parallelSteps, cleanupSteps := BuildPipeline(family, tools)
	if dueForMirrorCheck() {
		mirrorCheck := Step{Name: "mirrors", Run: func(dryRun bool, out io.Writer) error {
			return RunMirrorCheck(family, tools, dryRun, out)
		}}
		parallelSteps = append([]Step{mirrorCheck}, parallelSteps...)
	}

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

	elapsed := time.Since(start).Round(time.Second)

	if !dryRun {
		fmt.Println()
		fmt.Println(header("==> resumo"))
		for _, r := range results {
			status := ok("✔ ok")
			if r.Err != nil {
				status = fail("✘ falhou")
			}
			fmt.Printf("  %-42s %s  %s\n", r.Name, status, dim(r.Dur.Round(time.Millisecond).String()))
		}
	}

	if err != nil {
		failedName := "desconhecido"
		for _, r := range results {
			if r.Err != nil {
				failedName = r.Name
				break
			}
		}
		fmt.Fprintf(os.Stderr, "%s %q: %v\n", fail("erro em"), failedName, err)
		Notify("Erro no update", fmt.Sprintf("Falhou em: %s", failedName))
		os.Exit(1)
	}
	Notify("Update completo", fmt.Sprintf("Sistema atualizado e limpo em %s", elapsed))
}
