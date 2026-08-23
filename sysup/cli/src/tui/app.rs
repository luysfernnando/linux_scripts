// sysup update's full-screen live dashboard: the step list with
// spinner/progress, driven by ratatui/crossterm instead of Go's Bubble Tea.

use std::io::Write;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event as CEvent, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};
use ratatui::Terminal;
use regex::Regex;

use crate::pipeline::{Step, StepResult};
use crate::style;

/// Reports whether it's safe to hand the terminal over to a full-screen
/// dashboard: needs an actual tty (same check color already uses) and isn't
/// worth it for a dry-run, which is just a plan printout, not something
/// worth animating.
pub fn available(dry_run: bool) -> bool {
    style::color_enabled() && !dry_run
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StepStatus {
    Pending,
    Running,
    Ok,
    Failed,
    Skipped,
}

struct TuiStep {
    name: String,
    status: StepStatus,
    dur: Duration,
    err: Option<String>,
    progress: Option<(u32, u32)>,
    color: Color,
}

const TUI_PALETTE: &[Color] = &[
    Color::Indexed(38),
    Color::Indexed(213),
    Color::Indexed(220),
    Color::Indexed(75),
    Color::Indexed(84),
    Color::Indexed(203),
    Color::Indexed(141),
    Color::Indexed(44),
];

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Lightweight descriptor for the dashboard about to run — mirrors Go's
/// `NewUpdateModel(family, names)`, but the actual entry point callers drive
/// is `run_steps_tui` below, which opens/renders/tears down its own session
/// per call rather than keeping one long-lived model around.
pub struct UpdateApp {
    pub family: String,
    pub names: Vec<String>,
}

impl UpdateApp {
    pub fn new(family: &str, names: &[String]) -> Self {
        Self {
            family: family.to_string(),
            names: names.to_vec(),
        }
    }
}

enum TuiEvent {
    Start(usize),
    Progress(usize, u32, u32),
    Done(usize, Duration, Option<String>, String),
}

// pkgProgressRe matches the "(N/TOTAL)" prefix pacman (and paru, which
// shells out to pacman for the actual install) already prints on its own
// while installing/upgrading packages — e.g. "(42/115) upgrading firefox".
// Nothing needs to ask pacman for this; it's already in the output, we just
// have to notice it live instead of only seeing it after the fact in the
// captured buffer.
pub(crate) fn pkg_progress_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\((\d+)/(\d+)\)").unwrap())
}

fn parse_pkg_progress(line: &[u8]) -> Option<(u32, u32)> {
    let text = String::from_utf8_lossy(line);
    let caps = pkg_progress_re().captures(&text)?;
    let cur: u32 = caps.get(1)?.as_str().parse().ok()?;
    let total: u32 = caps.get(2)?.as_str().parse().ok()?;
    if total > 0 {
        Some((cur, total))
    } else {
        None
    }
}

pub(crate) fn round_ms(d: Duration) -> Duration {
    Duration::from_millis((d.as_secs_f64() * 1000.0).round() as u64)
}

fn round_secs(d: Duration) -> Duration {
    Duration::from_secs(d.as_secs_f64().round() as u64)
}

// Forwards everything written to it unchanged (so the captured output used
// for the post-failure report is untouched), while also scanning for the
// "(N/TOTAL)" pattern to report live install progress back to the
// dashboard. Output uses '\r' for in-place progress bars as well as '\n'
// for real line breaks, so both count as separators here.
struct ProgressWriter {
    buf: Arc<Mutex<Vec<u8>>>,
    leftover: Vec<u8>,
    tx: mpsc::Sender<TuiEvent>,
    index: usize,
}

impl Write for ProgressWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.buf.lock().unwrap().extend_from_slice(data);
        self.leftover.extend_from_slice(data);
        while let Some(pos) = self.leftover.iter().position(|&b| b == b'\r' || b == b'\n') {
            let line: Vec<u8> = self.leftover.drain(..=pos).collect();
            let line = &line[..line.len() - 1];
            if let Some((cur, total)) = parse_pkg_progress(line) {
                let _ = self.tx.send(TuiEvent::Progress(self.index, cur, total));
            }
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Runs steps with the same parallel/sudo-serialized semantics as
/// `pipeline::run_parallel`, but reports progress into a live dashboard
/// instead of printing. `offset` lets a later phase (e.g. cleanup steps,
/// run in a second call after the parallel phase) keep the same row
/// numbering scheme Go's shared dashboard used — here it sizes this
/// session's own row list with `offset` blank placeholder rows up front, so
/// event indices line up the same way even though each call opens and tears
/// down its own alt-screen session rather than keeping one Bubble Tea
/// program alive across both phases. sudo priming is the caller's
/// responsibility (it must happen before the alt screen takes over).
pub fn run_steps_tui(steps: Vec<Step>, dry_run: bool, offset: usize) -> Vec<StepResult> {
    if steps.is_empty() {
        return Vec::new();
    }

    let names: Vec<String> = steps.iter().map(|s| s.name.clone()).collect();
    let n = steps.len();

    let mut plain_indices = Vec::new();
    let mut privileged_indices = Vec::new();
    for (i, s) in steps.iter().enumerate() {
        if s.needs_privilege {
            privileged_indices.push(i);
        } else {
            plain_indices.push(i);
        }
    }

    let (tx, rx) = mpsc::channel::<TuiEvent>();
    let steps = Arc::new(steps);
    let results: Arc<Mutex<Vec<Option<StepResult>>>> =
        Arc::new(Mutex::new((0..n).map(|_| None).collect()));

    let render_handle = std::thread::spawn(move || {
        let _ = render_loop(names, offset, rx, n);
    });

    let run_one = {
        let steps = steps.clone();
        let results = results.clone();
        let tx = tx.clone();
        move |i: usize| {
            let _ = tx.send(TuiEvent::Start(offset + i));
            let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
            let mut out = ProgressWriter {
                buf: buf.clone(),
                leftover: Vec::new(),
                tx: tx.clone(),
                index: offset + i,
            };
            let start = Instant::now();
            let step_err = (steps[i].run)(dry_run, &mut out);
            let dur = start.elapsed();
            let output = String::from_utf8_lossy(&buf.lock().unwrap()).into_owned();
            let err_msg = step_err.as_ref().err().map(|e| e.to_string());
            results.lock().unwrap()[i] = Some(StepResult {
                name: steps[i].name.clone(),
                dur,
                err: step_err.err(),
                output: output.clone(),
            });
            let _ = tx.send(TuiEvent::Done(offset + i, dur, err_msg, output));
        }
    };

    std::thread::scope(|scope| {
        for &i in &plain_indices {
            let run_one = run_one.clone();
            scope.spawn(move || run_one(i));
        }
        if !privileged_indices.is_empty() {
            let run_one = run_one.clone();
            let privileged_indices = privileged_indices.clone();
            scope.spawn(move || {
                for i in privileged_indices {
                    run_one(i);
                }
            });
        }
    });

    drop(run_one);
    drop(tx);
    let _ = render_handle.join();

    Arc::try_unwrap(results)
        .map(|m| m.into_inner().unwrap())
        .unwrap_or_else(|arc| arc.lock().unwrap().drain(..).collect())
        .into_iter()
        .map(|r| r.expect("every step index is filled by run_one"))
        .collect()
}

/// Marks a batch of not-yet-run steps as skipped (used for cleanup steps
/// when an earlier phase already failed) and fills in matching
/// zero-duration, no-error results so the summary table still lists them.
pub fn skip_steps(steps: &[Step]) -> Vec<StepResult> {
    steps
        .iter()
        .map(|s| StepResult {
            name: s.name.clone(),
            dur: Duration::ZERO,
            err: None,
            output: String::new(),
        })
        .collect()
}

/// Returns the first error among results, if any.
pub fn first_err(results: &[StepResult]) -> Option<&anyhow::Error> {
    results.iter().find_map(|r| r.err.as_ref())
}

fn render_loop(
    names: Vec<String>,
    offset: usize,
    rx: mpsc::Receiver<TuiEvent>,
    expected_done: usize,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut rows: Vec<TuiStep> = Vec::with_capacity(offset + names.len());
    for _ in 0..offset {
        rows.push(TuiStep {
            name: String::new(),
            status: StepStatus::Pending,
            dur: Duration::ZERO,
            err: None,
            progress: None,
            color: Color::Indexed(240),
        });
    }
    for (i, name) in names.iter().enumerate() {
        rows.push(TuiStep {
            name: name.clone(),
            status: StepStatus::Pending,
            dur: Duration::ZERO,
            err: None,
            progress: None,
            color: TUI_PALETTE[i % TUI_PALETTE.len()],
        });
    }

    let start = Instant::now();
    let mut spinner_frame = 0usize;
    let mut done_count = 0usize;

    let result = loop {
        terminal.draw(|f| draw_ui(f, &rows, spinner_frame, start.elapsed()))?;

        if done_count >= expected_done {
            break Ok(());
        }

        if event::poll(Duration::from_millis(80))? {
            if let CEvent::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break Ok(());
                }
            }
        }
        spinner_frame = (spinner_frame + 1) % SPINNER_FRAMES.len();

        while let Ok(ev) = rx.try_recv() {
            match ev {
                TuiEvent::Start(i) => rows[i].status = StepStatus::Running,
                TuiEvent::Progress(i, cur, total) => rows[i].progress = Some((cur, total)),
                TuiEvent::Done(i, dur, err, _output) => {
                    rows[i].dur = dur;
                    rows[i].status = if err.is_some() {
                        StepStatus::Failed
                    } else {
                        StepStatus::Ok
                    };
                    rows[i].err = err;
                    done_count += 1;
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    result
}

fn draw_ui(frame: &mut ratatui::Frame, rows: &[TuiStep], spinner_frame: usize, elapsed: Duration) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "sysup update",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Indexed(212)),
    )));
    lines.push(Line::from(""));

    for row in rows {
        let mark = match row.status {
            StepStatus::Pending => Span::styled("·", Style::default().fg(Color::Indexed(240))),
            StepStatus::Running => Span::raw(SPINNER_FRAMES[spinner_frame % SPINNER_FRAMES.len()]),
            StepStatus::Ok => Span::styled("✔", Style::default().fg(Color::Indexed(42))),
            StepStatus::Failed => Span::styled("✘", Style::default().fg(Color::Indexed(203))),
            StepStatus::Skipped => Span::styled("−", Style::default().fg(Color::Indexed(240))),
        };
        let name_style = if row.status == StepStatus::Skipped {
            Style::default().fg(Color::Indexed(240))
        } else {
            Style::default().fg(row.color)
        };
        let mut spans = vec![
            mark,
            Span::raw(" "),
            Span::styled(row.name.clone(), name_style),
        ];
        if row.status == StepStatus::Running {
            if let Some((cur, total)) = row.progress {
                spans.push(Span::styled(
                    format!(" ({cur}/{total})"),
                    Style::default().fg(Color::Indexed(240)),
                ));
            }
        }
        if matches!(row.status, StepStatus::Ok | StepStatus::Failed) {
            spans.push(Span::styled(
                format!("  ({:?})", round_ms(row.dur)),
                Style::default().fg(Color::Indexed(240)),
            ));
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("decorrido: {:?}", round_secs(elapsed)),
        Style::default()
            .fg(Color::Indexed(240))
            .add_modifier(Modifier::ITALIC),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Indexed(240)))
        .padding(Padding::new(2, 2, 1, 1));
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, frame.area());
}
