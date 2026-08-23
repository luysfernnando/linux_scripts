// End-to-end regression test for the full self-update chain (fetch latest
// release -> download asset -> verify checksum -> extract -> replace_binary
// -> re_exec), run fully offline against a local mock of the GitHub API.
//
// This is exactly the manual scenario used during development to find and
// fix two real bugs: re_exec resolving its own path *after* self_update had
// already replaced the binary (Linux then reports "<path> (deleted)"), and
// replace_binary's cross-filesystem fallback copying straight onto the
// currently-executing target (ETXTBSY). Codifying it here means neither
// regresses silently.
//
// Requires SYSUP_VERSION to be set (to something below "9.9.9") when
// `cargo test` itself is invoked, since sysup::selfupdate::VERSION is baked
// in at compile time via option_env!. sysup/check.sh does this. A plain
// `cargo test` run without it compiles VERSION as "dev", which makes
// self_update() a guaranteed no-op — this test detects that and skips with
// an explanatory message instead of failing confusingly.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::thread;

use sha2::{Digest, Sha256};

fn os_arch() -> (&'static str, &'static str) {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };
    (os, arch)
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn build_release_tar_gz(fake_binary: &[u8]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(fake_binary.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    builder
        .append_data(&mut header, "sysup", fake_binary)
        .unwrap();
    let tar_bytes = builder.into_inner().unwrap();

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&tar_bytes).unwrap();
    encoder.finish().unwrap()
}

fn write_response(mut stream: &TcpStream, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

// Starts a minimal HTTP/1.1 mock server on 127.0.0.1 serving the three
// requests self_update makes: the releases/latest JSON, the release
// tar.gz, and checksums.txt. Runs as a detached background thread for the
// lifetime of the test process — no graceful shutdown needed for a
// single-test binary.
fn start_mock_release_server(
    listener: TcpListener,
    tar_gz: Vec<u8>,
    checksums_txt: String,
    release_json: String,
) {
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
                continue;
            }
            // Drain the rest of the request headers so the client doesn't
            // see a broken pipe before it finishes writing its request.
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) if line == "\r\n" || line.is_empty() => break,
                    Ok(_) => continue,
                }
            }

            if request_line.contains("releases/latest") {
                write_response(&stream, "application/json", release_json.as_bytes());
            } else if request_line.contains("checksums.txt") {
                write_response(&stream, "text/plain", checksums_txt.as_bytes());
            } else if request_line.contains(".tar.gz") {
                write_response(&stream, "application/octet-stream", &tar_gz);
            } else {
                let _ = (&stream).write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
            }
        }
    });
}

#[test]
fn selfupdate_downloads_verifies_and_reexecs_into_new_binary() {
    if sysup::selfupdate::VERSION == "dev" {
        eprintln!(
            "pulando: rode com SYSUP_VERSION=0.0.1 cargo test (sysup/check.sh já faz isso) \
             para exercitar o self-update de verdade"
        );
        return;
    }

    let fake_binary = b"#!/bin/sh\necho \"REEXEC_OK $@\"\n".to_vec();
    let tar_gz = build_release_tar_gz(&fake_binary);
    let (os, arch) = os_arch();
    let asset_name = format!("sysup_{os}_{arch}.tar.gz");
    let checksum = sha256_hex(&tar_gz);

    // Bind first so the real port is known before formatting the JSON that
    // needs to embed it in the asset URLs.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let port = listener.local_addr().unwrap().port();

    let checksums_txt = format!("{checksum}  {asset_name}\n");
    let release_json = format!(
        r#"{{"tag_name":"v9.9.9","assets":[
            {{"name":"{asset_name}","browser_download_url":"http://127.0.0.1:{port}/assets/{asset_name}"}},
            {{"name":"checksums.txt","browser_download_url":"http://127.0.0.1:{port}/assets/checksums.txt"}}
        ]}}"#
    );

    start_mock_release_server(listener, tar_gz, checksums_txt, release_json);

    let output = Command::new(env!("CARGO_BIN_EXE_sysup"))
        .env(
            "SYSUP_SELFUPDATE_API_BASE",
            format!("http://127.0.0.1:{port}"),
        )
        .args(["update", "--self-update-only"])
        .output()
        .expect("spawn sysup child process");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("sysup atualizado:") && stdout.contains("9.9.9"),
        "self-update message missing.\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("REEXEC_OK update --self-update-only"),
        "re_exec didn't hand off to the new binary with the right args.\nstdout: {stdout}\nstderr: {stderr}"
    );
}
