// pipeline builds and runs the per-machine update pipeline: which steps
// apply, whether they route through the polkit worker or classic sudo, and
// how they're executed (parallel ecosystems + a serialized privileged
// lane).

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;

use crate::detect::{Family, Tools};
use crate::polkit::WorkerClient;
use crate::style;

/// Step is one stage of the update pipeline. Steps that don't apply to this
/// machine (tool not installed) are simply omitted by `build_pipeline`
/// rather than failing the whole run.
///
/// `needs_privilege` marks steps that need root via the classic sudo path
/// (yay/paru's own internal escalation, or the npm -g step, or any sudo
/// fallback when no polkit worker is configured). However well primed,
/// sudo's credential cache is keyed per terminal/session in ways that vary
/// by system config (tty_tickets, use_pty, timestamp_timeout, even PAM 2FA
/// modules that never cache at all) — running two sudo invocations at once
/// is not reliably safe on every machine. Every needs_privilege step is
/// serialized into a single lane that runs one at a time, while everything
/// else (including steps routed through the polkit worker, which handles
/// its own sequencing) still runs fully parallel alongside it.
pub struct Step {
    pub name: String,
    pub needs_privilege: bool,
    // `+ Sync` (beyond Go's closure, which needs no such bound) is required
    // here only because `run_parallel` shares steps across threads via
    // `Arc<Vec<Step>>` instead of Go's implicit closure-over-shared-slice
    // capture — every closure built by `build_pipeline` only captures
    // owned/Arc data, so this is free in practice.
    pub run: StepRun,
}

/// A step's executable body: `(dry_run, out) -> Result<()>`.
pub type StepRun = Box<dyn Fn(bool, &mut dyn Write) -> anyhow::Result<()> + Send + Sync>;

/// Executes (or, in dry-run mode, just prints) a shell command line, writing
/// combined stdout/stderr to out. Steps are expressed as shell strings
/// because most of them are already "cmd1 && cmd2"-style pipelines ported
/// straight from the old .zshrc aliases.
pub fn run_shell(dry_run: bool, line: &str, out: &mut dyn Write) -> anyhow::Result<()> {
    run_shell_env(dry_run, line, &[], out)
}

/// `run_shell` plus extra environment variables — used to hand paru's step
/// SYSUP_WORKER_SOCKET so its configured sysup-authbridge (see
/// crate::polkit) can reach the already-authorized worker.
fn run_shell_env(
    dry_run: bool,
    line: &str,
    extra_env: &[String],
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    writeln!(out, "{}", style::dim(&format!("==> {line}")))?;
    if dry_run {
        return Ok(());
    }
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(line);
    if !extra_env.is_empty() {
        cmd.envs(extra_env.iter().filter_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            Some((k.to_string(), v.to_string()))
        }));
    }
    // Go sets cmd.Stdout and cmd.Stderr to the very same `out` writer,
    // which Go's exec.Cmd internally satisfies by handing the child one
    // real fd (it special-cases Stdout == Stderr to skip the pipe+copy
    // machinery it'd otherwise need). We get the same single-fd behavior
    // here by duping one OS pipe's write end into both slots: both streams
    // land in one fd, so there is exactly one interleaved byte stream to
    // read back and relay into `out` — no second thread, no `Send` bound
    // needed on `out`, and no risk of the child blocking on a full pipe
    // while we're stuck draining the other one.
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    let (read_end, write_end) = nix::unistd::pipe().context("criando pipe")?;
    let write_end_dup = nix::unistd::dup(write_end.as_raw_fd()).context("duplicando pipe")?;
    // SAFETY: `write_end_dup` is a fresh fd just returned by `dup(2)`, not
    // owned or tracked by anything else yet — wrapping it in an OwnedFd
    // here is what gives it an owner (whoever drops this OwnedFd, or the
    // Stdio it's about to be moved into, closes it exactly once).
    let write_end_dup = unsafe { OwnedFd::from_raw_fd(write_end_dup) };
    cmd.stdin(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::from(write_end));
    cmd.stderr(std::process::Stdio::from(write_end_dup));
    let mut child = cmd.spawn()?;
    // Drop our own copies of the write end now that the child has its own
    // (duplicated at spawn) — otherwise our read loop below would never
    // see EOF, since the pipe would still have a writer open (us).
    drop(cmd);

    let mut reader = std::fs::File::from(read_end);
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => out.write_all(&buf[..n])?,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("comando falhou: {} ({})", line, status);
    }
    Ok(())
}

/// Sends one exact argv to the already-authorized polkit worker instead of a
/// shell string — no `sh -c`, no sudo.
fn run_worker_cmd(
    dry_run: bool,
    worker: &WorkerClient,
    out: &mut dyn Write,
    argv: &[String],
) -> anyhow::Result<()> {
    writeln!(out, "{}", style::dim(&format!("==> {}", argv.join(" "))))?;
    if dry_run {
        return Ok(());
    }
    worker.run(argv, out)
}

fn run_worker_sequence(
    dry_run: bool,
    worker: &WorkerClient,
    out: &mut dyn Write,
    cmds: &[&[&str]],
) -> anyhow::Result<()> {
    for argv in cmds {
        let argv: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
        run_worker_cmd(dry_run, worker, out, &argv)?;
    }
    Ok(())
}

fn add_step(dst: &mut Vec<Step>, name: &str, shell_line: &str, needs_privilege: bool) {
    let line = shell_line.to_string();
    dst.push(Step {
        name: name.to_string(),
        needs_privilege,
        run: Box::new(move |dry_run, out| run_shell(dry_run, &line, out)),
    });
}

fn add_worker_step(dst: &mut Vec<Step>, name: &str, argv: &[&str], worker: Arc<WorkerClientHandle>) {
    let argv: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    dst.push(Step {
        name: name.to_string(),
        needs_privilege: false,
        run: Box::new(move |dry_run, out| run_worker_cmd(dry_run, worker.get(), out, &argv)),
    });
}

/// `WorkerClient` doesn't implement `Clone`/`Sync`, but `Step::run` closures
/// need `Send` and may be invoked from a spawned thread while `build_pipeline`
/// itself only ever holds a borrowed `&WorkerClient`. We can't stash a
/// borrowed reference in a `'static`-ish boxed closure, so instead every
/// worker-routed step closure captures an `Arc` to this thin handle, which
/// wraps the raw pointer for the run's duration — sound because the
/// `WorkerClient` passed into `build_pipeline` is guaranteed by every caller
/// to outlive the steps it built (they're run and dropped within the same
/// update invocation, before the worker client itself is dropped/closed).
struct WorkerClientHandle(*const WorkerClient);

// SAFETY: the pointee outlives all steps built against it (see doc comment
// above), and WorkerClient::run only performs its own internal
// synchronization (a fresh UnixStream connect per call) — concurrent calls
// through this handle from different threads are as safe as calling `run`
// directly on a shared `&WorkerClient` would be.
unsafe impl Send for WorkerClientHandle {}
unsafe impl Sync for WorkerClientHandle {}

impl WorkerClientHandle {
    fn get(&self) -> &WorkerClient {
        // SAFETY: see struct doc comment — pointee is guaranteed to be alive
        // for as long as any Step built with this handle exists.
        unsafe { &*self.0 }
    }
}

/// Assembles the steps for this machine, split into:
///   - parallel: independent package ecosystems (system pkg manager, flatpak,
///     composer, npm, bun, firmware) that don't touch each other's state and
///     can run concurrently to cut wall-clock time.
///   - cleanup: orphan removal + cache cleaning, which depend on the system
///     package manager step having finished, so they always run serially
///     after every parallel step completes.
///
/// `worker` is the already-authorized polkit worker (`None` if `sysup
/// polkit-setup` hasn't run, or dry-run) — when `Some`, the system
/// package-manager and cleanup steps route through it instead of sudo. yay
/// always stays on classic sudo (no config hook to redirect it); paru is
/// pointed at the worker via its own `[bin] Sudo` config (see crate::polkit),
/// so its step doesn't need `needs_privilege` either.
///
/// Nothing here is hardcoded to a single distro — that's the whole point.
pub fn build_pipeline(
    family: &Family,
    t: &Tools,
    worker: Option<&WorkerClient>,
) -> (Vec<Step>, Vec<Step>) {
    let mut parallel: Vec<Step> = Vec::new();
    let mut cleanup: Vec<Step> = Vec::new();

    // Only ever handed to closures that run within this same call's
    // lifetime — see WorkerClientHandle's doc comment for the safety
    // argument.
    let handle: Option<Arc<WorkerClientHandle>> =
        worker.map(|w| Arc::new(WorkerClientHandle(w as *const WorkerClient)));

    let add = |dst: &mut Vec<Step>, name: &str, shell_line: &str| {
        add_step(dst, name, shell_line, shell_line.contains("sudo"));
    };

    match family {
        Family::Arch => {
            if t.yay {
                // yay doesn't expose any config/flag to change its privilege
                // escalation binary, so it always calls the real sudo itself
                // — no way to route it through the worker without shadowing
                // the system's sudo globally, which reopens exactly the kind
                // of broad hole this design avoids. Stays on classic sudo.
                add_step(
                    &mut parallel,
                    "Pacotes do sistema + AUR (yay)",
                    "yay -Syu --noconfirm",
                    true,
                );
            } else if t.paru {
                if let Some(worker) = worker {
                    // paru DOES support a custom [bin] Sudo binary (see
                    // crate::polkit's paru.conf patching) — when polkit-setup
                    // ran, paru is configured to call sysup-authbridge, which
                    // forwards straight to this worker. No sudo prompt at
                    // all.
                    let socket_env = worker.socket_env();
                    parallel.push(Step {
                        name: "Pacotes do sistema + AUR (paru)".to_string(),
                        needs_privilege: false,
                        run: Box::new(move |dry_run, out| {
                            run_shell_env(
                                dry_run,
                                "paru -Syu --noconfirm",
                                std::slice::from_ref(&socket_env),
                                out,
                            )
                        }),
                    });
                } else {
                    add_step(
                        &mut parallel,
                        "Pacotes do sistema + AUR (paru)",
                        "paru -Syu --noconfirm",
                        true,
                    );
                }
            } else if t.pacman {
                if let Some(handle) = &handle {
                    add_worker_step(
                        &mut parallel,
                        "Pacotes do sistema (pacman)",
                        &["pacman", "-Syu", "--noconfirm"],
                        handle.clone(),
                    );
                } else {
                    add(
                        &mut parallel,
                        "Pacotes do sistema (pacman)",
                        "sudo pacman -Syu --noconfirm",
                    );
                }
            }
        }
        Family::Debian => {
            if t.apt {
                if let Some(handle) = handle.clone() {
                    parallel.push(Step {
                        name: "Pacotes do sistema (apt)".to_string(),
                        needs_privilege: false,
                        run: Box::new(move |dry_run, out| {
                            run_worker_sequence(
                                dry_run,
                                handle.get(),
                                out,
                                &[&["apt-get", "update"], &["apt-get", "full-upgrade", "-y"]],
                            )
                        }),
                    });
                } else {
                    add(
                        &mut parallel,
                        "Pacotes do sistema (apt)",
                        "sudo apt-get update && sudo apt-get full-upgrade -y",
                    );
                }
            }
        }
        Family::Fedora => {
            if t.dnf {
                if let Some(handle) = &handle {
                    add_worker_step(
                        &mut parallel,
                        "Pacotes do sistema (dnf)",
                        &["dnf", "upgrade", "-y"],
                        handle.clone(),
                    );
                } else {
                    add(&mut parallel, "Pacotes do sistema (dnf)", "sudo dnf upgrade -y");
                }
            }
        }
        Family::Suse => {
            if t.zypper {
                if let Some(handle) = &handle {
                    add_worker_step(
                        &mut parallel,
                        "Pacotes do sistema (zypper)",
                        &["zypper", "update", "-y"],
                        handle.clone(),
                    );
                } else {
                    add(&mut parallel, "Pacotes do sistema (zypper)", "sudo zypper update -y");
                }
            }
        }
        Family::Darwin => {
            if t.brew {
                add(&mut parallel, "Homebrew", "brew update && brew upgrade");
            }
        }
        Family::Windows => {
            if t.choco {
                add(&mut parallel, "Chocolatey", "choco upgrade all -y");
            } else if t.winget {
                add(&mut parallel, "winget", "winget upgrade --all");
            }
        }
        Family::Unknown => {}
    }

    // Homebrew on Linux is common alongside pacman/apt/dnf, so check it
    // independently of family instead of only under Family::Darwin.
    if *family != Family::Darwin && t.brew {
        add(&mut parallel, "Homebrew (linux)", "brew update && brew upgrade");
    }

    if t.flatpak {
        add(
            &mut parallel,
            "Flatpak",
            "flatpak update -y && flatpak uninstall --unused -y",
        );
    }
    if t.composer && has_composer_global_project() {
        add(
            &mut parallel,
            "Composer (global)",
            "composer global update --no-interaction",
        );
    }
    if t.npm {
        if let Some(handle) = handle.clone() {
            parallel.push(Step {
                name: "Npm (global)".to_string(),
                needs_privilege: false,
                run: Box::new(move |dry_run, out| {
                    run_worker_sequence(
                        dry_run,
                        handle.get(),
                        out,
                        &[
                            &["npm", "install", "-g", "npm@latest"],
                            &["npm", "update", "-g"],
                        ],
                    )
                }),
            });
        } else {
            add(
                &mut parallel,
                "Npm (global)",
                "sudo npm install -g npm@latest && sudo npm update -g",
            );
        }
    }
    if t.bun {
        if let Ok(bun_path) = which::which("bun") {
            if is_writable(&bun_path) {
                add(&mut parallel, "Bun (global)", "bun upgrade");
            }
        }
        // If bun isn't writable by us, it's almost certainly installed via
        // the system package manager (e.g. pacman's `bun` package, owned by
        // root) — `bun upgrade` would try to overwrite a root-owned file
        // (fails with EACCES) and, even with sudo, would silently break the
        // package manager's file tracking. Let the system pkg manager own
        // bun's upgrades instead; just skip this step.
    }
    if t.fwupdmgr {
        add(
            &mut parallel,
            "Firmware (fwupdmgr)",
            "fwupdmgr refresh --force && fwupdmgr update --no-reboot-check -y",
        );
    }

    match family {
        Family::Arch => {
            if let Some(handle) = handle.clone() {
                let has_paccache = t.paccache;
                cleanup.push(Step {
                    name: "Órfãos + cache (pacman)".to_string(),
                    needs_privilege: false,
                    run: Box::new(move |dry_run, out| {
                        arch_cleanup_via_worker(dry_run, handle.get(), out, has_paccache)
                    }),
                });
            } else if t.paccache {
                add_step(
                    &mut cleanup,
                    "Órfãos + cache (pacman)",
                    "{ pacman -Qtdq | xargs -r sudo pacman -Rns --noconfirm; }; yes | sudo paccache -r",
                    true,
                );
            } else {
                add_step(
                    &mut cleanup,
                    "Órfãos (pacman)",
                    "pacman -Qtdq | xargs -r sudo pacman -Rns --noconfirm",
                    true,
                );
            }
        }
        Family::Debian => {
            if let Some(handle) = handle.clone() {
                cleanup.push(Step {
                    name: "Órfãos + cache (apt)".to_string(),
                    needs_privilege: false,
                    run: Box::new(move |dry_run, out| {
                        run_worker_sequence(
                            dry_run,
                            handle.get(),
                            out,
                            &[
                                &["apt-get", "autoremove", "-y"],
                                &["apt-get", "autoclean", "-y"],
                            ],
                        )
                    }),
                });
            } else {
                add_step(
                    &mut cleanup,
                    "Órfãos + cache (apt)",
                    "sudo apt-get autoremove -y && sudo apt-get autoclean -y",
                    true,
                );
            }
        }
        Family::Fedora => {
            if let Some(handle) = handle.clone() {
                cleanup.push(Step {
                    name: "Órfãos + cache (dnf)".to_string(),
                    needs_privilege: false,
                    run: Box::new(move |dry_run, out| {
                        run_worker_sequence(
                            dry_run,
                            handle.get(),
                            out,
                            &[&["dnf", "autoremove", "-y"], &["dnf", "clean", "all"]],
                        )
                    }),
                });
            } else {
                add_step(
                    &mut cleanup,
                    "Órfãos + cache (dnf)",
                    "sudo dnf autoremove -y && sudo dnf clean all",
                    true,
                );
            }
        }
        _ => {}
    }

    (parallel, cleanup)
}

/// Lists orphans unprivileged first (`pacman -Qtdq` needs no root) and only
/// sends the actual removal through the worker if there's anything to
/// remove, then always clears the cache.
fn arch_cleanup_via_worker(
    dry_run: bool,
    worker: &WorkerClient,
    out: &mut dyn Write,
    has_paccache: bool,
) -> anyhow::Result<()> {
    let list_out = std::process::Command::new("pacman")
        .arg("-Qtdq")
        .output()
        .map(|o| o.stdout)
        .unwrap_or_default(); // non-zero exit here just means "no orphans"
    let orphans: Vec<String> = String::from_utf8_lossy(&list_out)
        .trim()
        .split('\n')
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    if !orphans.is_empty() {
        let mut argv = vec!["pacman".to_string(), "-Rns".to_string(), "--noconfirm".to_string()];
        argv.extend(orphans);
        run_worker_cmd(dry_run, worker, out, &argv)?;
    } else {
        writeln!(out, "{}", style::dim("==> sem pacotes órfãos"))?;
    }

    if !has_paccache {
        writeln!(
            out,
            "{}",
            style::dim("==> paccache ausente (instale pacman-contrib) — pulando limpeza de cache")
        )?;
        return Ok(());
    }
    run_worker_cmd(
        dry_run,
        worker,
        out,
        &["paccache".to_string(), "-r".to_string()],
    )
}

/// Reports whether path's containing directory actually accepts new files
/// from us — the real test for "can we replace this binary in place", more
/// reliable than parsing permission bits (covers root-owned dirs, read-only
/// mounts, ACLs, etc). Used to detect binaries installed by a system package
/// manager (e.g. pacman's `bun`), which self-updaters like `bun upgrade`
/// can't and shouldn't try to overwrite.
fn is_writable(path: &std::path::Path) -> bool {
    let Some(dir) = path.parent() else {
        return false;
    };
    let candidate = dir.join(format!(".sysup-writetest-{}", std::process::id()));
    match std::fs::File::create(&candidate) {
        Ok(_) => {
            let _ = std::fs::remove_file(&candidate);
            true
        }
        Err(_) => false,
    }
}

/// Reports whether there's actually a global composer.json to update.
/// `composer global update` exits 1 when there isn't one — a normal state
/// for anyone who's never installed a global composer package, not a real
/// failure — so we skip the step entirely instead of letting it abort the
/// whole pipeline.
fn has_composer_global_project() -> bool {
    let dir = match std::env::var_os("COMPOSER_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => match std::env::home_dir() {
            Some(home) => home.join(".config").join("composer"),
            None => return false,
        },
    };
    dir.join("composer.json").is_file()
}

/// Line-buffers writes and emits each line prefixed with a colored step
/// name, serialized through a shared mutex — needed so concurrent steps'
/// output doesn't interleave mid-line into unreadable garbage.
struct PrefixWriter {
    name: String,
    color: &'static str,
    mu: Arc<Mutex<()>>,
    buf: Vec<u8>,
}

impl Write for PrefixWriter {
    fn write(&mut self, p: &[u8]) -> std::io::Result<usize> {
        let _guard = self.mu.lock().unwrap();
        self.buf.extend_from_slice(p);
        let tag = style::colorize(self.color, &format!("[{}]", self.name));
        while let Some(i) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=i).collect();
            let line = &line[..line.len() - 1];
            println!("{tag} {}", String::from_utf8_lossy(line));
        }
        Ok(p.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Records how one pipeline step went, for the end-of-run summary. `output`
/// is only populated by the TUI runner (which captures each step's combined
/// stdout/stderr instead of streaming it live) — empty elsewhere.
pub struct StepResult {
    pub name: String,
    pub dur: Duration,
    pub err: Option<anyhow::Error>,
    pub output: String,
}

/// Primes sudo once against the real terminal (so it can actually prompt,
/// instead of the no-op it'd be against /dev/null), keeps it alive for the
/// duration, then runs steps: non-sudo steps all at once on their own
/// threads, sudo steps one at a time in a single dedicated lane so no two
/// sudo invocations ever run concurrently. Returns one `StepResult` per step
/// (same order as the input) plus the first error encountered, if any.
pub fn run_parallel(steps: Vec<Step>, dry_run: bool) -> anyhow::Result<Vec<StepResult>> {
    if steps.is_empty() {
        return Ok(Vec::new());
    }

    let stop_sudo = prime_sudo(&steps, dry_run);

    let mu = Arc::new(Mutex::new(()));
    let n = steps.len();
    let results: Arc<Mutex<Vec<Option<StepResult>>>> =
        Arc::new(Mutex::new((0..n).map(|_| None).collect()));

    let mut plain_indices = Vec::new();
    let mut privileged_indices = Vec::new();
    for (i, s) in steps.iter().enumerate() {
        if s.needs_privilege {
            privileged_indices.push(i);
        } else {
            plain_indices.push(i);
        }
    }

    // Steps are stored behind an Arc<Vec<Step>> so both the plain-lane
    // threads and the single privileged lane thread can reference them by
    // index without moving ownership around — mirrors Go's closures over
    // the shared `steps` slice.
    let steps = Arc::new(steps);

    let run_one = {
        let steps = steps.clone();
        let mu = mu.clone();
        let results = results.clone();
        move |i: usize| {
            let color = style::step_color(i);
            let mut out = PrefixWriter {
                name: steps[i].name.clone(),
                color,
                mu: mu.clone(),
                buf: Vec::new(),
            };
            let start = Instant::now();
            let step_err = (steps[i].run)(dry_run, &mut out);
            let dur = start.elapsed();

            {
                let _guard = mu.lock().unwrap();
                let tag = style::colorize(color, &format!("[{}]", steps[i].name));
                if step_err.is_err() {
                    println!("{tag} {} ({:?})", style::fail("✘ falhou"), dur);
                } else if !dry_run {
                    println!("{tag} {} ({:?})", style::ok("✔ concluído"), dur);
                }
            }

            results.lock().unwrap()[i] = Some(StepResult {
                name: steps[i].name.clone(),
                dur,
                err: step_err.err(),
                output: String::new(),
            });
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

    stop_sudo();

    let results: Vec<StepResult> = Arc::try_unwrap(results)
        .map(|m| m.into_inner().unwrap())
        .unwrap_or_else(|arc| arc.lock().unwrap().drain(..).collect())
        .into_iter()
        .map(|r| r.expect("every step index is filled by run_one"))
        .collect();

    // Go's RunParallel returns (results, err) as two separate values — every
    // StepResult always comes back, plus the first error seen, so the
    // caller can both render the per-step summary and decide whether to
    // treat the whole run as failed. Rust's `anyhow::Result<Vec<StepResult>>`
    // can't carry both at once, so we always return the full results here
    // (each with its own `err` field intact) and leave "was the run a
    // success" to the caller via `results.iter().any(|r| r.err.is_some())`
    // — that's the same check Go's caller (cli.go) ultimately made with the
    // returned err.
    Ok(results)
}

/// Checks whether any step needs classic sudo and, if so, primes sudo's
/// credential cache against the real terminal (so it can actually prompt,
/// instead of the no-op it'd be against /dev/null) and keeps it alive with a
/// background ticker. Callers must invoke the returned stop closure once
/// done — safe to call even when nothing was primed (no privileged steps, or
/// dry_run). This has no equivalent needed on the polkit worker path: the
/// worker's own single pkexec call is what prompts, whenever it happens,
/// with no keepalive required since the worker just stays alive (and
/// authorized) for the run's duration instead of relying on a refreshable
/// credential cache.
pub fn prime_sudo(steps: &[Step], dry_run: bool) -> Box<dyn FnOnce()> {
    let has_privileged_step = steps.iter().any(|s| s.needs_privilege);
    if dry_run || !has_privileged_step {
        return Box::new(|| {});
    }

    let status = std::process::Command::new("sudo")
        .arg("-v")
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();
    if let Err(e) = status.and_then(|s| {
        if s.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(format!("exit status {s}")))
        }
    }) {
        eprintln!(
            "{}",
            style::warn(&format!(
                "aviso: sudo -v falhou, passos que precisam de root podem pedir senha individualmente: {e}"
            ))
        );
    }

    let (tx, rx) = mpsc::channel::<()>();
    let handle = std::thread::spawn(move || sudo_keep_alive(rx));
    Box::new(move || {
        let _ = tx.send(());
        let _ = handle.join();
    })
}

fn sudo_keep_alive(done: mpsc::Receiver<()>) {
    loop {
        match done.recv_timeout(Duration::from_secs(60)) {
            Ok(()) => return,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = std::process::Command::new("sudo").arg("-v").status();
            }
        }
    }
}
