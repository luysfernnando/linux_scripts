// sysup's command dispatch and the `update` orchestration (self-update,
// pipeline build, TUI/plain runner, summary, notification) — the thin
// main.rs just calls run().

use std::io::Write as _;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};

use crate::detect::{self, Family};
use crate::mirrors;
use crate::notify;
use crate::pipeline;
use crate::polkit;
use crate::schedule;
use crate::selfupdate;
use crate::style;
use crate::tools;
use crate::tui;

#[derive(Parser)]
#[command(name = "sysup")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Mostra a versão instalada
    Version,
    /// Detecta distro/SO e roda o pipeline de update completo
    Update {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        no_self_update: bool,
    },
    /// Reranqueia mirrors do gerenciador de pacotes
    Mirrors {
        #[arg(long)]
        dry_run: bool,
    },
    /// Instala um agendador nativo (systemd timer/launchd/schtasks) pros mirrors
    Schedule,
    /// Instala/atualiza o GitKraken
    #[command(name = "gitkraken")]
    GitKraken {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Instala/atualiza/roda o Tidewave
    Tidewave {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Configura elevação via polkit (1 prompt por run em vez de repetir sudo)
    #[command(name = "polkit-setup")]
    PolkitSetup {
        #[arg(long)]
        dry_run: bool,
    },
}

/// sysup's entrypoint logic — parses argv and dispatches to the matching
/// subcommand.
pub fn run() -> anyhow::Result<()> {
    let mut argv: Vec<String> = std::env::args().collect();
    // No subcommand at all defaults to `update` — the most common invocation
    // (a cron/menu entry just runs `sysup`).
    if argv.len() < 2 {
        argv.push("update".to_string());
    } else if argv[1] == "--version" || argv[1] == "-v" {
        // clap's derive Parser only wires up --version/-v when asked to via
        // `#[command(version)]`; we don't want clap's own version string
        // (it'd print the Cargo package version, not selfupdate::VERSION),
        // so these are aliased to the `version` subcommand by hand instead.
        argv[1] = "version".to_string();
    }

    let cli = Cli::parse_from(argv);

    match cli.command {
        Command::Version => {
            println!("sysup {}", selfupdate::VERSION);
            Ok(())
        }
        Command::Update { dry_run, no_self_update } => run_update(dry_run, no_self_update),
        Command::Mirrors { dry_run } => {
            let family = detect::detect_family();
            let t = detect::detect_tools();
            if let Err(e) = mirrors::run_check(&family, &t, dry_run, &mut std::io::stdout()) {
                eprintln!("erro no ranking de mirrors: {e}");
                std::process::exit(1);
            }
            Ok(())
        }
        Command::Schedule => {
            if let Err(e) = schedule::install_schedule() {
                eprintln!("erro instalando agendamento: {e}");
                std::process::exit(1);
            }
            Ok(())
        }
        Command::GitKraken { args } => {
            if let Err(e) = tools::run_gitkraken(&args) {
                eprintln!("erro: {e}");
                std::process::exit(1);
            }
            Ok(())
        }
        Command::Tidewave { args } => {
            if let Err(e) = tools::run_tidewave(&args) {
                eprintln!("erro: {e}");
                std::process::exit(1);
            }
            Ok(())
        }
        Command::PolkitSetup { dry_run } => {
            if let Err(e) = polkit::run_setup(dry_run) {
                eprintln!("erro: {e}");
                std::process::exit(1);
            }
            Ok(())
        }
    }
}

// family_str/REPO_SLUG-style small duplication: polkit.rs keeps its own
// private copy of this same mapping for its policy/log text, and sharing a
// single string-to-string function across modules isn't worth the coupling
// for something this small.
fn family_str(family: Family) -> &'static str {
    match family {
        Family::Arch => "arch",
        Family::Debian => "debian",
        Family::Fedora => "fedora",
        Family::Suse => "suse",
        Family::Darwin => "darwin",
        Family::Windows => "windows",
        Family::Unknown => "unknown",
    }
}

/// Asks the user, once per run, whether to install pacman-contrib when
/// paccache is missing — without it the orphan/cache cleanup step silently
/// skips cache trimming. Runs before the polkit worker starts, same spot as
/// the classic sudo prompt in polkit::run_setup.
fn maybe_install_paccache(dry_run: bool, t: &mut detect::Tools) {
    if dry_run {
        println!(
            "{}",
            style::dim("==> paccache ausente (pacman-contrib) — pulando prompt de instalação em --dry-run")
        );
        return;
    }

    print!(
        "pacman-contrib (paccache) não está instalado — instalar agora pra limpeza de cache funcionar? [y/N] "
    );
    std::io::stdout().flush().ok();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return;
    }
    let answer = answer.trim().to_lowercase();
    if answer != "y" && answer != "yes" && answer != "s" && answer != "sim" {
        println!("{}", style::dim("==> ok, limpeza de cache vai ser pulada nesta run"));
        return;
    }

    let status = std::process::Command::new("sudo")
        .args(["pacman", "-S", "--noconfirm", "pacman-contrib"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();
    match status {
        Ok(s) if s.success() => t.paccache = true,
        Ok(s) => eprintln!(
            "{}",
            style::warn(&format!(
                "aviso: falha instalando pacman-contrib, limpeza de cache vai ser pulada: exit status {s}"
            ))
        ),
        Err(e) => eprintln!(
            "{}",
            style::warn(&format!(
                "aviso: falha instalando pacman-contrib, limpeza de cache vai ser pulada: {e}"
            ))
        ),
    }
}

// Go's `time.Since(start).Round(time.Second)` — used only for the elapsed
// time shown in the final summary, where sub-second precision is just noise.
fn round_secs(d: Duration) -> Duration {
    Duration::from_secs(d.as_secs_f64().round() as u64)
}

fn run_update(dry_run: bool, no_self_update: bool) -> anyhow::Result<()> {
    if !no_self_update && selfupdate::self_update(dry_run) {
        if let Err(e) = selfupdate::re_exec() {
            // re_exec only returns on failure — a successful call replaces
            // the process image and never comes back here at all.
            eprintln!(
                "{}",
                style::warn(&format!(
                    "aviso: self-update rodou mas re-exec falhou, continuando nesta versão: {e}"
                ))
            );
        }
    }
    // Best-effort: a stale/dirty dotfiles clone, or no clone at all, should
    // never block the actual system update.
    selfupdate::try_update_dotfiles_repo(dry_run);

    let start = Instant::now();
    let family = detect::detect_family();
    let mut t = detect::detect_tools();

    if family == Family::Arch && !t.paccache {
        maybe_install_paccache(dry_run, &mut t);
    }

    // Whether the worker is actually authorized and running — separate from
    // whatever WorkerClient::start happens to hand back, since it always
    // returns a (possibly inactive) client rather than an Option: inactive
    // either because `sysup polkit-setup` never ran, or because starting it
    // just failed below. Either way build_pipeline must fall back to
    // classic sudo instead of routing steps into a client that will only
    // bail when actually asked to run something.
    let mut worker_active = !dry_run && polkit::configured();
    let mut worker = match polkit::WorkerClient::start(dry_run) {
        Ok(w) => w,
        Err(e) => {
            eprintln!(
                "{}",
                style::warn(&format!(
                    "aviso: worker privilegiado do polkit falhou, caindo pro sudo clássico: {e}"
                ))
            );
            worker_active = false;
            polkit::WorkerClient::start(true)
                .expect("start(dry_run=true) is infallible: returns an inactive client with no side effects")
        }
    };

    let worker_ref = if worker_active { Some(&worker) } else { None };
    let (mut parallel_steps, cleanup_steps) = pipeline::build_pipeline(&family, &t, worker_ref);

    if mirrors::due_for_check() {
        let family_for_step = family;
        let tools_for_step = t;
        parallel_steps.insert(
            0,
            pipeline::Step {
                name: "Mirrors".to_string(),
                needs_privilege: false,
                run: Box::new(move |dry_run, out| {
                    mirrors::run_check(&family_for_step, &tools_for_step, dry_run, out)
                }),
            },
        );
    }

    let used_tui = tui::available(dry_run);
    let results = if used_tui {
        run_update_tui(parallel_steps, cleanup_steps)
    } else {
        run_update_plain(family, parallel_steps, cleanup_steps, dry_run)
    };

    let elapsed = round_secs(start.elapsed());

    if !dry_run {
        println!();
        if used_tui {
            println!("{}", tui::render_summary_box(&results, elapsed));
        } else {
            println!("{}", style::header("==> resumo"));
            for r in &results {
                let status = if r.err.is_some() {
                    style::fail("✘ falhou")
                } else if r.dur.is_zero() {
                    style::dim("− pulado")
                } else {
                    style::ok("✔ ok")
                };
                println!(
                    "  {:<42} {}  {}",
                    r.name,
                    status,
                    style::dim(&format!("{:?}", crate::tui::app::round_ms(r.dur)))
                );
            }
        }
    }

    // Worker close happens after the summary is printed (not in a defer-at-
    // top-of-function style) so its own shutdown/reap never delays getting
    // the report on screen — same ordering Go's `defer worker.Close()`
    // achieved implicitly by running last regardless.
    worker.close();

    if let Some(failed) = results.iter().find(|r| r.err.is_some()) {
        if !failed.output.trim().is_empty() {
            eprintln!(
                "{}",
                style::fail(&format!("\n── saída de \"{}\" ──", failed.name))
            );
            eprintln!("{}", failed.output.trim_end_matches('\n'));
        }
        eprintln!(
            "{} \"{}\": {}",
            style::fail("erro em"),
            failed.name,
            failed.err.as_ref().expect("checked is_some above")
        );
        notify::notify("Erro no update", &format!("Falhou em: {}", failed.name));
        std::process::exit(1);
    }

    notify::notify(
        "Update completo",
        &format!("Sistema atualizado e limpo em {elapsed:?}"),
    );
    Ok(())
}

/// The fallback path for non-tty output (piped/logged runs, NO_COLOR) and
/// --dry-run, where a full-screen dashboard wouldn't render (or wouldn't be
/// worth it) — plain colored log lines, same as before the TUI existed.
fn run_update_plain(
    family: Family,
    parallel_steps: Vec<pipeline::Step>,
    cleanup_steps: Vec<pipeline::Step>,
    dry_run: bool,
) -> Vec<pipeline::StepResult> {
    println!(
        "{}",
        style::header(&format!("==> sysup update ({})", family_str(family)))
    );

    let mut results = match pipeline::run_parallel(parallel_steps, dry_run) {
        Ok(r) => r,
        // run_parallel's own contract is to always return Ok (see its doc
        // comment) — this only guards the Result signature defensively.
        Err(e) => {
            return vec![pipeline::StepResult {
                name: "pipeline".to_string(),
                dur: Duration::ZERO,
                err: Some(e),
                output: String::new(),
            }];
        }
    };

    let parallel_failed = results.iter().any(|r| r.err.is_some());
    if !parallel_failed {
        for step in cleanup_steps {
            let cstart = Instant::now();
            let run_result = (step.run)(dry_run, &mut std::io::stdout());
            let cdur = cstart.elapsed();
            let failed = run_result.is_err();
            results.push(pipeline::StepResult {
                name: step.name,
                dur: cdur,
                err: run_result.err(),
                output: String::new(),
            });
            if failed {
                break;
            }
        }
    }

    results
}

/// Drives the same pipeline through the full-screen ratatui dashboard: sudo
/// is primed up front across BOTH phases combined (must happen before the
/// alt screen takes over the terminal, so it can still prompt normally),
/// then each phase runs through its own `tui::run_steps_tui` call/alt-screen
/// session in turn.
fn run_update_tui(
    parallel_steps: Vec<pipeline::Step>,
    cleanup_steps: Vec<pipeline::Step>,
) -> Vec<pipeline::StepResult> {
    let n_parallel = parallel_steps.len();

    // Combined into one Vec purely so prime_sudo can see privileged steps
    // from both phases at once (it only needs a borrowed slice) — split
    // straight back apart by the length captured above before either phase
    // actually runs.
    let mut all_steps = parallel_steps;
    all_steps.extend(cleanup_steps);
    let stop_sudo = pipeline::prime_sudo(&all_steps, false);
    let cleanup_steps = all_steps.split_off(n_parallel);
    let parallel_steps = all_steps;

    let mut results = tui::run_steps_tui(parallel_steps, false, 0);
    if tui::first_err(&results).is_some() {
        results.extend(tui::skip_steps(&cleanup_steps));
    } else {
        let cleanup_results = tui::run_steps_tui(cleanup_steps, false, n_parallel);
        results.extend(cleanup_results);
    }

    stop_sudo();
    results
}
