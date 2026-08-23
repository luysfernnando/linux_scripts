//! polkit is the client side of sysup's polkit privilege-escalation
//! feature: detecting whether `sysup polkit-setup` has run, rendering the
//! policy XML, and running the setup flow itself. See sysup/README.md for
//! the full architecture and sysup-worker for the privileged half.

use anyhow::{bail, Context};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use ipc::Status;

use crate::detect::{self, Family};
use crate::download;
use crate::style;

// repoSlug identifies the GitHub repo release assets are fetched from —
// duplicated as a tiny local constant here and in selfupdate rather than
// shared, since it's a single string literal and sharing it would mean an
// otherwise-pointless inter-module dependency.
const REPO_SLUG: &str = "luysfernnando/linux_scripts";

const POLKIT_ACTION_ID: &str = "io.github.luysfernnando.sysup.worker";
const HELPER_INSTALL_DIR: &str = "/usr/lib/sysup";

fn policy_path() -> String {
    format!("/usr/share/polkit-1/actions/{POLKIT_ACTION_ID}.policy")
}

fn helper_install_path() -> String {
    format!("{HELPER_INSTALL_DIR}/sysup-worker")
}

fn authbridge_path() -> String {
    format!("{HELPER_INSTALL_DIR}/sysup-authbridge")
}

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

/// Reports whether this machine even has a polkit stack (pkexec + the
/// actions directory) — separate from `configured`, which checks whether
/// sysup's OWN one-time setup ran.
pub fn available() -> bool {
    if !detect::has_tool("pkexec") {
        return false;
    }
    Path::new("/usr/share/polkit-1/actions").is_dir()
}

/// Reports whether `sysup polkit-setup` has already installed the
/// root-owned helper and its policy. BuildPipeline uses this (not
/// `available` alone) to decide whether privileged steps go through the
/// worker or fall back to classic sudo.
pub fn configured() -> bool {
    if !available() {
        return false;
    }
    let meta = match std::fs::metadata(helper_install_path()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if meta.is_dir() {
        return false;
    }
    // Reject if group/other can write it — a helper that anyone but root
    // could modify would make the whole polkit action pointless.
    if meta.permissions().mode() & 0o022 != 0 {
        return false;
    }
    if std::fs::metadata(policy_path()).is_err() {
        return false;
    }
    true
}

/// Builds the single polkit action that authorizes running sysup-worker.
/// This doesn't vary per machine — the worker re-detects family/tools
/// itself and enforces its own whitelist, so the policy only ever needs to
/// answer "can this user run the worker."
pub fn render_policy_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC
 "-//freedesktop//DTD polkit Policy Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/polkit/1/policyconfig.dtd">
<policyconfig>
  <vendor>sysup</vendor>
  <vendor_url>https://github.com/{REPO_SLUG}</vendor_url>

  <action id="{POLKIT_ACTION_ID}">
    <description>Executar operações de atualização do sistema (sysup)</description>
    <message>sysup precisa de privilégios administrativos para rodar o update</message>
    <icon_name>system-software-update</icon_name>
    <defaults>
      <allow_any>no</allow_any>
      <allow_inactive>no</allow_inactive>
      <allow_active>auth_admin_keep</allow_active>
    </defaults>
    <annotate key="org.freedesktop.policykit.exec.path">{helper}</annotate>
    <annotate key="org.freedesktop.policykit.exec.allow_gui">true</annotate>
  </action>
</policyconfig>
"#,
        helper = helper_install_path()
    )
}

/// `sysup polkit-setup`: shows exactly what will be installed/changed,
/// asks for confirmation, then applies it behind a single elevation prompt
/// (this bootstrap step, not the worker itself, so it's fine to just use
/// whatever of pkexec/sudo is present). This is a one-time setup action —
/// it never installs a standing NOPASSWD rule of any kind.
pub fn run_setup(dry_run: bool) -> anyhow::Result<()> {
    if !detect::has_tool("pkexec") {
        println!(
            "{}",
            style::warn(
                "aviso: pkexec não encontrado — este sistema não tem polkit, nada a fazer. `sysup update` continua usando sudo."
            )
        );
        return Ok(());
    }

    let family = detect::detect_family();
    let tools = detect::detect_tools();

    println!(
        "{}",
        style::header(&format!("==> sysup polkit-setup ({})", family_str(family)))
    );
    println!("Isso instala:");
    println!(
        "  - {} (binário root, só roda via pkexec, vive só durante um `sysup update`)",
        helper_install_path()
    );
    println!("  - {} (autoriza esse binário via polkit)", policy_path());

    let mut paru_path = String::new();
    let mut paru_new_content = String::new();
    if tools.paru {
        println!("  - symlink {} -> sysup-worker", authbridge_path());
        match build_paru_conf_update() {
            Ok((path, content)) => {
                println!(
                    "  - {}: adiciona/atualiza `[bin] Sudo = {}` (backup .bak antes; SudoLoop não é tocado)",
                    path,
                    authbridge_path()
                );
                paru_path = path;
                paru_new_content = content;
            }
            Err(e) => {
                println!(
                    "{}",
                    style::warn(&format!(
                        "aviso: não consegui preparar a edição de paru.conf: {e}"
                    ))
                );
            }
        }
    }
    if tools.yay {
        println!(
            "{}",
            style::warn(
                "aviso: yay detectado — ele chama sudo por conta própria pra instalar pacotes AUR e não tem opção de configuração pra trocar isso (paru tem; yay não). Nessas máquinas o update ainda vai pedir um segundo prompt (sudo clássico), disparado junto no início, não no meio do dashboard. Ver sysup/README.md."
            )
        );
    }

    if detect::is_wsl() {
        println!(
            "{}",
            style::warn(
                "aviso: WSL detectado — WSL não atribui seat ao systemd-logind, e a autenticação interativa do polkit costuma falhar mesmo com senha correta e sudo funcionando normal (pkexec sempre reporta \"Not authorized\"). Se `polkit-setup` terminar com esse erro, use `sysup update` sem polkit configurado — ele já cai automaticamente pro sudo clássico."
            )
        );
    }

    let policy_xml = render_policy_xml();
    println!(
        "{}",
        style::header(&format!("\n==> conteúdo proposto para {}", policy_path()))
    );
    print!("{policy_xml}");

    if !paru_path.is_empty() {
        println!(
            "{}",
            style::header(&format!("\n==> conteúdo proposto para {paru_path}"))
        );
        print!("{paru_new_content}");
    }

    if dry_run {
        return Ok(());
    }

    print!("\nAplicar essas mudanças agora? [y/N] ");
    std::io::stdout().flush().ok();
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    let answer = answer.trim().to_lowercase();
    if answer != "y" && answer != "yes" && answer != "s" && answer != "sim" {
        println!("cancelado");
        return Ok(());
    }

    let helper_bin = acquire_helper_binary().context("obtendo sysup-worker")?;
    let _cleanup = RemoveOnDrop(helper_bin.clone());

    install_elevated(&helper_bin, &policy_xml, tools.paru)?;

    if !paru_path.is_empty() {
        apply_paru_conf(&paru_path, &paru_new_content)
            .with_context(|| format!("editando {paru_path}"))?;
    }

    println!(
        "{}",
        style::ok(
            "✔ polkit configurado — próximo `sysup update` pede a senha uma única vez por execução"
        )
    );
    Ok(())
}

// Mirrors Go's `defer os.Remove(helperBin)`.
struct RemoveOnDrop(PathBuf);
impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Writes the helper binary + policy (and, if requested, the
/// sysup-authbridge symlink) via one elevated shell script, so the whole
/// install happens behind a single sudo/pkexec prompt instead of one per
/// file. This is a one-time bootstrap script, not something that runs
/// again after setup — the actual update pipeline never invokes a shell.
fn install_elevated(
    helper_bin: &Path,
    policy_xml: &str,
    include_authbridge: bool,
) -> anyhow::Result<()> {
    let tmp_policy = std::env::temp_dir().join(format!("sysup-policy-{}.xml", std::process::id()));
    std::fs::write(&tmp_policy, policy_xml)?;
    let _tmp_policy_cleanup = RemoveOnDrop(tmp_policy.clone());

    let mut script = format!(
        "set -e\ninstall -o root -g root -m 0755 -D {} {}\ninstall -o root -g root -m 0644 -D {} {}\n",
        shell_quote(&helper_bin.to_string_lossy()),
        shell_quote(&helper_install_path()),
        shell_quote(&tmp_policy.to_string_lossy()),
        shell_quote(&policy_path()),
    );
    if include_authbridge {
        script += &format!("ln -sf sysup-worker {}\n", shell_quote(&authbridge_path()));
    }

    let tmp_script =
        std::env::temp_dir().join(format!("sysup-polkit-install-{}.sh", std::process::id()));
    std::fs::write(&tmp_script, &script)?;
    let _tmp_script_cleanup = RemoveOnDrop(tmp_script.clone());

    let status = Command::new("pkexec")
        .arg("sh")
        .arg(&tmp_script)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !status.success() {
        bail!("pkexec sh {} falhou: {status}", tmp_script.display());
    }
    Ok(())
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Prefers building from a local repo clone (matching the
/// ~/.config/sysup/repo-path marker install.sh already maintains) over
/// downloading a release asset — mirrors install.sh's own "build from
/// source if we can, download otherwise" ordering, just reversed priority
/// since a dev iterating on this code wants their own build, not last
/// release's.
fn acquire_helper_binary() -> anyhow::Result<PathBuf> {
    if let Some(path) = build_helper_from_repo() {
        return Ok(path);
    }
    download_helper_binary()
}

fn build_helper_from_repo() -> Option<PathBuf> {
    if !detect::has_tool("cargo") {
        return None;
    }
    let home = std::env::var_os("HOME")?;
    let marker = Path::new(&home).join(".config/sysup/repo-path");
    let data = std::fs::read_to_string(&marker).ok()?;
    let repo = data.trim();
    if repo.is_empty() {
        return None;
    }
    let src_dir = Path::new(repo).join("sysup-rs");
    if !src_dir.join("sysup-worker").join("Cargo.toml").is_file() {
        return None;
    }

    let status = Command::new("cargo")
        .args(["build", "--release", "-p", "sysup-worker"])
        .current_dir(&src_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }

    let built = src_dir.join("target/release/sysup-worker");
    let out = std::env::temp_dir().join(format!("sysup-worker-build-{}", std::process::id()));
    std::fs::copy(&built, &out).ok()?;
    let mut perms = std::fs::metadata(&out).ok()?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&out, perms).ok()?;
    Some(out)
}

/// Fetches sysup-worker from the latest GitHub release, verifying it
/// against checksums.txt from the same release before trusting it — same
/// trust chain SelfUpdate already relies on for the main binary.
fn download_helper_binary() -> anyhow::Result<PathBuf> {
    if !cfg!(target_os = "linux") {
        bail!("polkit só é suportado no Linux");
    }
    let asset_name = format!("sysup-worker_linux_{}.tar.gz", go_arch());
    let base = format!("https://github.com/{REPO_SLUG}/releases/latest/download/");

    let tgz = download::get(&format!("{base}{asset_name}"))?;
    let sums = download::get(&format!("{base}checksums.txt"))?;
    download::verify_checksum(&String::from_utf8_lossy(&sums), &asset_name, &tgz)?;
    download::extract_single_file(&tgz, "sysup-worker")
}

// Matches Go's GOARCH naming for release assets (amd64/arm64), since the
// release pipeline still publishes assets under those names.
fn go_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    }
}

/// Computes the `[bin] Sudo = <authbridge>` edit for the user's paru.conf
/// without touching anything else in the file — creates the file/section
/// if missing, replaces an existing Sudo line if present.
fn build_paru_conf_update() -> anyhow::Result<(String, String)> {
    let path = paru_conf_path()?;
    let original = std::fs::read_to_string(&path).unwrap_or_default();
    let new_content = patch_paru_conf_sudo(&original, &authbridge_path());
    Ok((path.to_string_lossy().into_owned(), new_content))
}

fn paru_conf_path() -> anyhow::Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME não definido")?;
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => Path::new(&home).join(".config"),
    };
    Ok(base.join("paru/paru.conf"))
}

fn patch_paru_conf_sudo(original: &str, authbridge: &str) -> String {
    let sudo_line = format!("Sudo = {authbridge}");
    if original.trim().is_empty() {
        return format!("[bin]\n{sudo_line}\n");
    }

    let mut out: Vec<String> = Vec::new();
    let mut in_bin = false;
    let mut bin_seen = false;
    let mut sudo_set = false;

    let close_bin = |in_bin: bool, sudo_set: &mut bool, out: &mut Vec<String>| {
        if in_bin && !*sudo_set {
            out.push(sudo_line.clone());
            *sudo_set = true;
        }
    };

    for line in original.split('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            close_bin(in_bin, &mut sudo_set, &mut out);
            in_bin = trimmed == "[bin]";
            if in_bin {
                bin_seen = true;
            }
            out.push(line.to_string());
            continue;
        }
        if in_bin && trimmed.starts_with("Sudo") && trimmed.contains('=') {
            out.push(sudo_line.clone());
            sudo_set = true;
            continue;
        }
        out.push(line.to_string());
    }
    close_bin(in_bin, &mut sudo_set, &mut out);
    if !bin_seen {
        if let Some(last) = out.last() {
            if !last.trim().is_empty() {
                out.push(String::new());
            }
        }
        out.push("[bin]".to_string());
        out.push(sudo_line);
    }
    out.join("\n")
}

/// Backs up the existing file (if any) to .bak, same pattern
/// dotfiles/install.sh already uses before symlinking, then writes the
/// patched content.
fn apply_paru_conf(path: &str, new_content: &str) -> anyhow::Result<()> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Ok(data) = std::fs::read(path) {
        let backup = format!("{}.bak", path.display());
        std::fs::write(backup, data)?;
    }
    std::fs::write(path, new_content)?;
    Ok(())
}

struct WorkerInner {
    socket_path: PathBuf,
    child: Child,
    // Held open for exactly as long as the worker should run — the
    // worker's blocking stdin read detects EOF, closing this pipe, and
    // exits. Wrapped in Option so `close` can drop it explicitly.
    stdin: Option<ChildStdin>,
}

/// A handle to a live sysup-worker process, authorized once via pkexec and
/// reused for every privileged step in this run. Mirrors Go's nilable
/// `*WorkerClient`: when polkit hasn't been set up (or dry-run), `inner` is
/// `None` and every method degrades to the classic-sudo-fallback behavior
/// callers already expect from a nil client.
pub struct WorkerClient {
    inner: Option<WorkerInner>,
}

impl WorkerClient {
    /// Spawns sysup-worker via pkexec exactly once, before any TUI takes
    /// over the terminal (same timing PrimeSudo uses today) — the one
    /// polkit authorization that covers the whole run. Yields an inactive
    /// client when polkit hasn't been set up on this machine or `dry_run`
    /// is set; callers fall back to the classic sudo path in that case.
    pub fn start(dry_run: bool) -> anyhow::Result<WorkerClient> {
        if dry_run || !configured() {
            return Ok(WorkerClient { inner: None });
        }

        let socket_path = ipc::socket_path();

        let mut child = Command::new("pkexec")
            .arg(helper_install_path())
            .arg("--socket")
            .arg(&socket_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take();
        let mut stdout = child.stdout.take().expect("stdout piped");

        let (tx, rx) = mpsc::channel::<anyhow::Result<()>>();
        std::thread::spawn(move || {
            let mut buf = [0u8; 32];
            let result = match stdout.read(&mut buf) {
                Ok(0) => Err(anyhow::anyhow!("worker encerrou sem responder")),
                Ok(n) if buf[..n].starts_with(b"READY") => Ok(()),
                Ok(n) => Err(anyhow::anyhow!(
                    "resposta inesperada do worker: {:?}",
                    String::from_utf8_lossy(&buf[..n])
                )),
                Err(e) => Err(anyhow::Error::from(e)),
            };
            let _ = tx.send(result);
        });

        match rx.recv_timeout(Duration::from_secs(5 * 60)) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                drop(stdin);
                let _ = child.wait();
                bail!("autorização do worker falhou: {e}");
            }
            Err(_) => {
                let _ = child.kill();
                bail!("timeout esperando autorização do polkit");
            }
        }

        Ok(WorkerClient {
            inner: Some(WorkerInner {
                socket_path,
                child,
                stdin,
            }),
        })
    }

    /// Sends one exact command (already resolved to argv form, no shell)
    /// to the already-authorized worker and streams its output into out —
    /// same shape as a plain shell runner, so callers' output capture
    /// needs no per-source changes.
    pub fn run(&self, argv: &[String], out: &mut dyn Write) -> anyhow::Result<()> {
        let inner = match &self.inner {
            Some(inner) => inner,
            None => bail!("worker privilegiado não está rodando"),
        };

        let conn = UnixStream::connect(&inner.socket_path).context("conectando no worker")?;

        ipc::write_request(&conn, argv).context("mandando pedido pro worker")?;

        let (status, msg) = ipc::relay_output(out, &conn)?;
        match status {
            Status::Ok => Ok(()),
            // Rejected (bad request / not whitelisted) and Failed (ran but
            // errored) are both hard failures at this call site — the
            // Rejected-vs-Failed distinction only matters to callers who
            // might retry via a different path (e.g. sysup-authbridge
            // falling back to real sudo), which this method never does.
            Status::Rejected | Status::Failed => bail!(msg),
        }
    }

    /// Signals the worker to shut down (closing its stdin pipe, which it's
    /// blocked reading from) and reaps the process. Safe to call on an
    /// inactive client.
    pub fn close(&mut self) {
        if let Some(inner) = &mut self.inner {
            inner.stdin.take();
            let _ = inner.child.wait();
        }
    }

    /// Returns the KEY=VALUE pair to add to a step's environment (e.g.
    /// paru, via its sysup-authbridge `[bin] Sudo` hook) so it can reach
    /// the already-authorized worker directly instead of prompting itself.
    pub fn socket_env(&self) -> String {
        match &self.inner {
            Some(inner) => format!("SYSUP_WORKER_SOCKET={}", inner.socket_path.display()),
            None => String::new(),
        }
    }
}

/// Spawns sysup-worker via pkexec exactly once. See `WorkerClient::start`.
pub fn start_worker(dry_run: bool) -> anyhow::Result<WorkerClient> {
    WorkerClient::start(dry_run)
}
