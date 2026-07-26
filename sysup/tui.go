package main

import (
	"bytes"
	"fmt"
	"strings"
	"sync"
	"time"

	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// tuiAvailable reports whether it's safe to hand the terminal over to a
// full-screen Bubble Tea program: needs an actual tty (same check color
// already uses) and isn't worth it for a dry-run, which is just a plan
// printout, not something worth animating.
func tuiAvailable(dryRun bool) bool {
	return colorEnabled && !dryRun
}

type stepStatus int

const (
	stepPending stepStatus = iota
	stepRunning
	stepOK
	stepFailed
	stepSkipped
)

type tuiStep struct {
	name   string
	status stepStatus
	dur    time.Duration
	err    error
	output string
	color  lipgloss.Color
}

type stepStartMsg struct{ i int }
type stepDoneMsg struct {
	i      int
	dur    time.Duration
	err    error
	output string
}
type stepSkipMsg struct{ i int }
type pipelineDoneMsg struct{}

// updateModel is the Bubble Tea model for `sysup update`'s live dashboard:
// a fixed list of steps (parallel steps first, then cleanup), each ticking
// through pending -> running -> ok/failed/skipped as the real pipeline
// (driven from a separate goroutine via Program.Send) reports progress.
type updateModel struct {
	family string
	steps  []tuiStep
	spin   spinner.Model
	start  time.Time
}

var tuiPalette = []lipgloss.Color{"38", "213", "220", "75", "84", "203", "141", "44"}

func newUpdateModel(family string, names []string) updateModel {
	steps := make([]tuiStep, len(names))
	for i, n := range names {
		steps[i] = tuiStep{name: n, status: stepPending, color: tuiPalette[i%len(tuiPalette)]}
	}
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("212"))
	return updateModel{family: family, steps: steps, spin: s, start: time.Now()}
}

func (m updateModel) Init() tea.Cmd {
	return m.spin.Tick
}

func (m updateModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		if msg.String() == "ctrl+c" {
			return m, tea.Quit
		}
	case spinner.TickMsg:
		var cmd tea.Cmd
		m.spin, cmd = m.spin.Update(msg)
		return m, cmd
	case stepStartMsg:
		m.steps[msg.i].status = stepRunning
	case stepDoneMsg:
		m.steps[msg.i].dur = msg.dur
		m.steps[msg.i].err = msg.err
		m.steps[msg.i].output = msg.output
		if msg.err != nil {
			m.steps[msg.i].status = stepFailed
		} else {
			m.steps[msg.i].status = stepOK
		}
	case stepSkipMsg:
		m.steps[msg.i].status = stepSkipped
	case pipelineDoneMsg:
		return m, tea.Quit
	}
	return m, nil
}

var (
	tuiTitleStyle = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("212"))
	tuiBoxStyle   = lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(lipgloss.Color("240")).Padding(1, 2)
	tuiOKMark     = lipgloss.NewStyle().Foreground(lipgloss.Color("42")).Render("✔")
	tuiFailMark   = lipgloss.NewStyle().Foreground(lipgloss.Color("203")).Render("✘")
	tuiSkipMark   = lipgloss.NewStyle().Foreground(lipgloss.Color("240")).Render("−")
	tuiPendMark   = lipgloss.NewStyle().Foreground(lipgloss.Color("240")).Render("·")
	tuiDimStyle   = lipgloss.NewStyle().Foreground(lipgloss.Color("240"))
	tuiHintStyle  = lipgloss.NewStyle().Foreground(lipgloss.Color("240")).Italic(true)
)

func (m updateModel) View() string {
	var b strings.Builder
	b.WriteString(tuiTitleStyle.Render(fmt.Sprintf("sysup update (%s)", m.family)))
	b.WriteString("\n\n")
	for _, s := range m.steps {
		nameStyle := lipgloss.NewStyle().Foreground(s.color)
		var mark string
		switch s.status {
		case stepPending:
			mark = tuiPendMark
		case stepRunning:
			mark = m.spin.View()
		case stepOK:
			mark = tuiOKMark
		case stepFailed:
			mark = tuiFailMark
		case stepSkipped:
			mark = tuiSkipMark
		}
		name := nameStyle.Render(s.name)
		if s.status == stepSkipped {
			name = tuiDimStyle.Render(s.name)
		}
		line := fmt.Sprintf("%s %s", mark, name)
		if s.status == stepOK || s.status == stepFailed {
			line += tuiDimStyle.Render(fmt.Sprintf("  (%s)", s.dur.Round(time.Millisecond)))
		}
		b.WriteString(line)
		b.WriteString("\n")
	}
	b.WriteString("\n")
	b.WriteString(tuiHintStyle.Render(fmt.Sprintf("decorrido: %s", time.Since(m.start).Round(time.Second))))
	return tuiBoxStyle.Render(b.String())
}

// syncBuffer is an io.Writer that just accumulates output — used to capture
// each step's combined stdout/stderr for the failure report printed after
// the TUI exits, instead of streaming raw shell output into the fixed-size
// dashboard (which would just scramble the layout).
type syncBuffer struct {
	mu  sync.Mutex
	buf bytes.Buffer
}

func (w *syncBuffer) Write(p []byte) (int, error) {
	w.mu.Lock()
	defer w.mu.Unlock()
	return w.buf.Write(p)
}

func (w *syncBuffer) String() string {
	w.mu.Lock()
	defer w.mu.Unlock()
	return w.buf.String()
}

// runStepsTUI runs steps with the same parallel/sudo-serialized semantics as
// RunParallel, but reports progress via Program.Send instead of printing —
// index i in steps maps to model index offset+i. sudo priming is the
// caller's responsibility (it must happen before the alt screen takes over).
func runStepsTUI(p *tea.Program, steps []Step, dryRun bool, offset int) []StepResult {
	results := make([]StepResult, len(steps))
	if len(steps) == 0 {
		return results
	}

	var sudoSteps, plainSteps []int
	for i, s := range steps {
		if s.NeedsSudo {
			sudoSteps = append(sudoSteps, i)
		} else {
			plainSteps = append(plainSteps, i)
		}
	}

	run := func(i int) {
		p.Send(stepStartMsg{i: offset + i})
		out := &syncBuffer{}
		start := time.Now()
		stepErr := steps[i].Run(dryRun, out)
		dur := time.Since(start)
		output := out.String()
		results[i] = StepResult{Name: steps[i].Name, Dur: dur, Err: stepErr, Output: output}
		p.Send(stepDoneMsg{i: offset + i, dur: dur, err: stepErr, output: output})
	}

	var wg sync.WaitGroup
	for _, i := range plainSteps {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			run(i)
		}(i)
	}
	if len(sudoSteps) > 0 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for _, i := range sudoSteps {
				run(i)
			}
		}()
	}
	wg.Wait()
	return results
}

// skipSteps marks a batch of not-yet-run steps as skipped (used for cleanup
// steps when an earlier phase already failed) and fills in matching
// zero-duration, no-error results so the summary table still lists them.
func skipSteps(p *tea.Program, steps []Step, offset int) []StepResult {
	results := make([]StepResult, len(steps))
	for i, s := range steps {
		p.Send(stepSkipMsg{i: offset + i})
		results[i] = StepResult{Name: s.Name}
	}
	return results
}

// firstErr returns the first non-nil error among results, if any.
func firstErr(results []StepResult) error {
	for _, r := range results {
		if r.Err != nil {
			return r.Err
		}
	}
	return nil
}
