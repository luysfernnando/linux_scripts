// ANSI color helpers shared across the whole sysup CLI (pipeline output,
// polkit-setup, self-update, mirrors, tool installers) — kept as its own
// leaf module (no internal deps) so pipeline and tui can both depend on it
// without risking a cycle between them.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_DIM: &str = "\x1b[2m";
const ANSI_RED: &str = "\x1b[31m";
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_YELLOW: &str = "\x1b[33m";
const ANSI_BLUE: &str = "\x1b[34m";
const ANSI_MAGENTA: &str = "\x1b[35m";
const ANSI_CYAN: &str = "\x1b[36m";

// Rotates across step tags so concurrent steps are visually distinguishable
// in interleaved output, purely by index — deterministic across runs since
// BuildPipeline always emits steps in the same order.
pub const STEP_PALETTE: &[&str] = &[
    ANSI_CYAN,
    ANSI_MAGENTA,
    ANSI_YELLOW,
    ANSI_BLUE,
    ANSI_GREEN,
    ANSI_RED,
];

static COLOR_ENABLED: OnceLock<AtomicBool> = OnceLock::new();

// Follows the usual conventions: NO_COLOR/TERM=dumb disable it outright,
// otherwise it's on only when stdout is an actual terminal (not a
// pipe/file, where escape codes would just be noise).
fn detect_color_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
        || std::env::var("TERM").is_ok_and(|v| v == "dumb")
    {
        return false;
    }
    std::io::stdout().is_terminal()
}

pub fn color_enabled() -> bool {
    COLOR_ENABLED
        .get_or_init(|| AtomicBool::new(detect_color_enabled()))
        .load(Ordering::Relaxed)
}

pub fn set_color_enabled(enabled: bool) {
    COLOR_ENABLED
        .get_or_init(|| AtomicBool::new(detect_color_enabled()))
        .store(enabled, Ordering::Relaxed);
}

pub fn colorize(code: &str, s: &str) -> String {
    if !color_enabled() {
        return s.to_string();
    }
    format!("{code}{s}{ANSI_RESET}")
}

pub fn step_color(i: usize) -> &'static str {
    STEP_PALETTE[i % STEP_PALETTE.len()]
}

pub fn header(s: &str) -> String {
    colorize(&format!("{ANSI_BOLD}{ANSI_CYAN}"), s)
}

pub fn ok(s: &str) -> String {
    colorize(ANSI_GREEN, s)
}

pub fn fail(s: &str) -> String {
    colorize(ANSI_RED, s)
}

pub fn warn(s: &str) -> String {
    colorize(ANSI_YELLOW, s)
}

pub fn dim(s: &str) -> String {
    colorize(ANSI_DIM, s)
}
