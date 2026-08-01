// Package tui is sysup update's full-screen Bubble Tea dashboard: the live
// step list with spinner/progress, plus the bordered summary box printed
// after it exits.
package tui

import (
	"bytes"
	"fmt"
	"io"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"

	"sysup/internal/pipeline"
	"sysup/internal/style"
)

// Available reports whether it's safe to hand the terminal over to a
// full-screen Bubble Tea program: needs an actual tty (same check color
// already uses) and isn't worth it for a dry-run, which is just a plan
// printout, not something worth animating.
func Available(dryRun bool) bool {
	return style.ColorEnabled && !dryRun
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
	name     string
	status   stepStatus
	dur      time.Duration
	err      error
	output   string
	progress string
	color    lipgloss.Color
}

type stepStartMsg struct{ i int }
type stepDoneMsg struct {
	i      int
	dur    time.Duration
	err    error
	output string
}
type stepSkipMsg struct{ i int }
type stepProgressMsg struct {
	i          int
	cur, total int
}

// PipelineDoneMsg signals the dashboard the whole run (parallel + cleanup)
// has finished — sent by the caller once it's done driving RunStepsTUI.
type PipelineDoneMsg struct{}

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

// NewUpdateModel builds the initial dashboard model, one row per step name
// in the order they'll run (parallel steps first, then cleanup).
func NewUpdateModel(family string, names []string) tea.Model {
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
	case stepProgressMsg:
		m.steps[msg.i].progress = fmt.Sprintf("%d/%d", msg.cur, msg.total)
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
	case PipelineDoneMsg:
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
		if s.status == stepRunning && s.progress != "" {
			name += tuiDimStyle.Render(" (" + s.progress + ")")
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

// RenderSummaryBox is the post-run report printed after the TUI's
// alt-screen closes — same bordered-box look as the live dashboard instead
// of a bare "==> resumo" line, so the two don't visually clash. Each line
// tries to say something declarative (summarizeStep, summary.go) instead
// of just "ok".
func RenderSummaryBox(results []pipeline.StepResult, elapsed time.Duration) string {
	var b strings.Builder
	b.WriteString(tuiTitleStyle.Render("sysup update — resumo"))
	b.WriteString("\n\n")

	maxName := 0
	for _, r := range results {
		if len(r.Name) > maxName {
			maxName = len(r.Name)
		}
	}

	for _, r := range results {
		var mark, detail string
		switch {
		case r.Err != nil:
			mark = tuiFailMark
			detail = "Falhou"
		case r.Dur == 0:
			mark = tuiSkipMark
			detail = "Pulado"
		default:
			mark = tuiOKMark
			detail = summarizeStep(r.Name, r.Output)
			if detail == "" {
				detail = "Concluído"
			}
		}
		line := fmt.Sprintf("%s %-*s  %s", mark, maxName, r.Name, tuiDimStyle.Render(detail))
		if r.Dur > 0 {
			line += tuiDimStyle.Render(fmt.Sprintf("  (%s)", r.Dur.Round(time.Millisecond)))
		}
		b.WriteString(line)
		b.WriteString("\n")
	}

	b.WriteString("\n")
	b.WriteString(tuiHintStyle.Render(fmt.Sprintf("concluído em %s", elapsed)))
	return tuiBoxStyle.Render(strings.TrimRight(b.String(), "\n"))
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

// pkgProgressRe matches the "(N/TOTAL)" prefix pacman (and paru, which
// shells out to pacman for the actual install) already prints on its own
// while installing/upgrading packages — e.g. "(42/115) upgrading firefox".
// Nothing needs to ask pacman for this; it's already in the output, we
// just have to notice it live instead of only seeing it after the fact in
// the captured buffer.
var pkgProgressRe = regexp.MustCompile(`\((\d+)/(\d+)\)`)

// progressWriter forwards everything written to it unchanged (so the
// captured output used for the post-failure report is untouched), while
// also scanning for the "(N/TOTAL)" pattern to report live install
// progress back to the dashboard. Output uses '\r' for in-place progress
// bars as well as '\n' for real line breaks, so both count as separators
// here.
type progressWriter struct {
	inner   io.Writer
	buf     []byte
	onMatch func(cur, total int)
}

func (w *progressWriter) Write(p []byte) (int, error) {
	n, err := w.inner.Write(p)
	w.buf = append(w.buf, p...)
	for {
		idx := bytes.IndexAny(w.buf, "\r\n")
		if idx < 0 {
			break
		}
		line := w.buf[:idx]
		w.buf = w.buf[idx+1:]
		if m := pkgProgressRe.FindSubmatch(line); m != nil {
			cur, curErr := strconv.Atoi(string(m[1]))
			total, totalErr := strconv.Atoi(string(m[2]))
			if curErr == nil && totalErr == nil && total > 0 && w.onMatch != nil {
				w.onMatch(cur, total)
			}
		}
	}
	return n, err
}

// RunStepsTUI runs steps with the same parallel/sudo-serialized semantics as
// pipeline.RunParallel, but reports progress via Program.Send instead of
// printing — index i in steps maps to model index offset+i. sudo priming is
// the caller's responsibility (it must happen before the alt screen takes
// over).
func RunStepsTUI(p *tea.Program, steps []pipeline.Step, dryRun bool, offset int) []pipeline.StepResult {
	results := make([]pipeline.StepResult, len(steps))
	if len(steps) == 0 {
		return results
	}

	var sudoSteps, plainSteps []int
	for i, s := range steps {
		if s.NeedsPrivilege {
			sudoSteps = append(sudoSteps, i)
		} else {
			plainSteps = append(plainSteps, i)
		}
	}

	run := func(i int) {
		p.Send(stepStartMsg{i: offset + i})
		buf := &syncBuffer{}
		out := &progressWriter{inner: buf, onMatch: func(cur, total int) {
			p.Send(stepProgressMsg{i: offset + i, cur: cur, total: total})
		}}
		start := time.Now()
		stepErr := steps[i].Run(dryRun, out)
		dur := time.Since(start)
		output := buf.String()
		results[i] = pipeline.StepResult{Name: steps[i].Name, Dur: dur, Err: stepErr, Output: output}
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

// SkipSteps marks a batch of not-yet-run steps as skipped (used for cleanup
// steps when an earlier phase already failed) and fills in matching
// zero-duration, no-error results so the summary table still lists them.
func SkipSteps(p *tea.Program, steps []pipeline.Step, offset int) []pipeline.StepResult {
	results := make([]pipeline.StepResult, len(steps))
	for i, s := range steps {
		p.Send(stepSkipMsg{i: offset + i})
		results[i] = pipeline.StepResult{Name: s.Name}
	}
	return results
}

// FirstErr returns the first non-nil error among results, if any.
func FirstErr(results []pipeline.StepResult) error {
	for _, r := range results {
		if r.Err != nil {
			return r.Err
		}
	}
	return nil
}
