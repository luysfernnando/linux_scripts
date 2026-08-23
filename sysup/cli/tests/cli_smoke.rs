// Lightweight CLI smoke tests: no network, no mock server, never touch a
// real system-mutating pipeline. Complements selfupdate_e2e.rs's heavier
// self-update path with quick checks on the everyday commands and their
// messages.

use std::process::Command;

#[test]
fn version_command_prints_embedded_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_sysup"))
        .arg("version")
        .output()
        .expect("spawn sysup version");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stdout: {stdout}");
    assert!(
        stdout.contains(&format!("sysup {}", sysup::selfupdate::VERSION)),
        "unexpected version output: {stdout}"
    );
}

#[test]
fn update_dry_run_prints_header_without_mutating_system() {
    // --no-self-update + --dry-run: no network call, no real package
    // manager/sudo invocation — pipeline steps under dry-run only print
    // the command they'd run, never execute it.
    let output = Command::new(env!("CARGO_BIN_EXE_sysup"))
        .args(["update", "--dry-run", "--no-self-update"])
        .output()
        .expect("spawn sysup update --dry-run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exit status: {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status
    );
    assert!(
        stdout.contains("==> sysup update ("),
        "missing update header line.\nstdout: {stdout}\nstderr: {stderr}"
    );
}
