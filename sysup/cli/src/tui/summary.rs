use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;

use crate::pipeline::StepResult;
use crate::style;
use crate::tui::app::{pkg_progress_re, round_ms};

// peakPkgTotal returns the largest TOTAL seen in any "(N/TOTAL)" line in
// output (pkg_progress_re, app.rs) — that's the number of packages pacman
// actually touched, whether installing or removing.
fn peak_pkg_total(output: &str) -> u32 {
    let mut max = 0u32;
    for caps in pkg_progress_re().captures_iter(output) {
        if let Ok(n) = caps[2].parse::<u32>() {
            if n > max {
                max = n;
            }
        }
    }
    max
}

fn npm_changed_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"changed (\d+) package").unwrap())
}

// Turns a step's raw captured output into one short declarative line for
// the final summary ("42 pacote(s) atualizado(s)", "Já estava tudo
// atualizado") instead of a bare "ok". Returns "" when nothing tool-specific
// could be inferred — callers fall back to a generic label.
fn summarize_step(name: &str, output: &str) -> String {
    if name.contains("Pacotes do sistema") || name.contains("+ AUR") {
        return summarize_pkg_update(output);
    }
    if name.starts_with("Órfãos") {
        return summarize_cleanup(output);
    }
    if name.starts_with("Npm") {
        return summarize_npm(output);
    }
    if name == "Flatpak" {
        return summarize_flatpak(output);
    }
    if name == "Repo dotfiles" {
        return summarize_dotfiles(output);
    }
    String::new()
}

fn summarize_pkg_update(output: &str) -> String {
    let n = peak_pkg_total(output);
    if n > 0 {
        return format!("{n} pacote(s) atualizado(s)");
    }
    if output.contains("there is nothing to do") {
        return "Já estava tudo atualizado".to_string();
    }
    String::new()
}

fn summarize_cleanup(output: &str) -> String {
    let n = peak_pkg_total(output);
    if n > 0 {
        return format!("{n} pacote(s) removido(s)");
    }
    if output.contains("sem pacotes órfãos") {
        return "Nenhum pacote órfão".to_string();
    }
    String::new()
}

fn summarize_npm(output: &str) -> String {
    if let Some(caps) = npm_changed_re().captures(output) {
        return format!("{} pacote(s) atualizado(s)", &caps[1]);
    }
    if output.contains("up to date") {
        return "Já estava tudo atualizado".to_string();
    }
    String::new()
}

fn summarize_flatpak(output: &str) -> String {
    if output.contains("Nothing to update") {
        return "Já estava tudo atualizado".to_string();
    }
    String::new()
}

fn summarize_dotfiles(output: &str) -> String {
    if output.contains("Already up to date.") {
        return "Já estava tudo atualizado".to_string();
    }
    if output.contains("sujo ou inacessível") {
        return "Pulado (repositório sujo)".to_string();
    }
    if !output.trim().is_empty() {
        return "Atualizado".to_string();
    }
    String::new()
}

// Strips ANSI SGR escape sequences so box padding/border alignment is based
// on the visible width of a line rather than its raw byte length.
fn visible_len(s: &str) -> usize {
    static ANSI_RE: OnceLock<Regex> = OnceLock::new();
    let re = ANSI_RE.get_or_init(|| Regex::new(r"\x1b\[[0-9;]*m").unwrap());
    re.replace_all(s, "").chars().count()
}

// Manual box-drawing equivalent of lipgloss's RoundedBorder + Padding(1, 2)
// — the summary box is returned as a plain string (not drawn live), so this
// avoids round-tripping through a ratatui widget just to stringify it.
fn render_box(content_lines: &[String]) -> String {
    let inner_width = content_lines
        .iter()
        .map(|l| visible_len(l))
        .max()
        .unwrap_or(0);
    let h_pad = "  "; // Padding(1, 2): 2 cols left/right
    let horizontal = "─".repeat(inner_width + h_pad.len() * 2);

    let mut out = String::new();
    out.push_str(&style::dim(&format!("╭{horizontal}╮")));
    out.push('\n');
    out.push_str(&style::dim(&format!(
        "│{}│",
        " ".repeat(inner_width + h_pad.len() * 2)
    )));
    out.push('\n');
    for line in content_lines {
        let pad_right = " ".repeat(inner_width - visible_len(line));
        out.push_str(&style::dim("│"));
        out.push_str(h_pad);
        out.push_str(line);
        out.push_str(&pad_right);
        out.push_str(h_pad);
        out.push_str(&style::dim("│"));
        out.push('\n');
    }
    out.push_str(&style::dim(&format!(
        "│{}│",
        " ".repeat(inner_width + h_pad.len() * 2)
    )));
    out.push('\n');
    out.push_str(&style::dim(&format!("╰{horizontal}╯")));
    out
}

/// The post-run report printed after the dashboard's alt-screen closes —
/// same bordered-box look as the live dashboard instead of a bare "==>
/// resumo" line, so the two don't visually clash. Each line tries to say
/// something declarative (`summarize_step`, above) instead of just "ok".
pub fn render_summary_box(results: &[StepResult], elapsed: Duration) -> String {
    let mut lines = vec![style::header("sysup update — resumo"), String::new()];

    let max_name = results
        .iter()
        .map(|r| r.name.chars().count())
        .max()
        .unwrap_or(0);

    for r in results {
        let (mark, detail) = if r.err.is_some() {
            (style::fail("✘"), "Falhou".to_string())
        } else if r.dur.is_zero() {
            (style::dim("−"), "Pulado".to_string())
        } else {
            let detail = summarize_step(&r.name, &r.output);
            (
                style::ok("✔"),
                if detail.is_empty() {
                    "Concluído".to_string()
                } else {
                    detail
                },
            )
        };

        let name_padded = format!("{:<width$}", r.name, width = max_name);
        let mut line = format!("{mark} {name_padded}  {}", style::dim(&detail));
        if !r.dur.is_zero() {
            line.push_str(&style::dim(&format!("  ({:?})", round_ms(r.dur))));
        }
        lines.push(line);
    }

    lines.push(String::new());
    lines.push(style::dim(&format!("concluído em {elapsed:?}")));

    render_box(&lines)
}
