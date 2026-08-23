use anyhow::{anyhow, Context};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn get(url: &str) -> anyhow::Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let resp = client.get(url).send()?;
    if !resp.status().is_success() {
        return Err(anyhow!("GET {}: status {}", url, resp.status()));
    }
    Ok(resp.bytes()?.to_vec())
}

pub fn get_to_file(url: &str, dest: &Path) -> anyhow::Result<()> {
    let resp = reqwest::blocking::get(url)?;
    if !resp.status().is_success() {
        return Err(anyhow!("GET {}: status {}", url, resp.status()));
    }
    let bytes = resp.bytes()?;
    let mut f = File::create(dest)?;
    f.write_all(&bytes)?;
    Ok(())
}

pub fn verify_checksum(checksums_txt: &str, asset_name: &str, data: &[u8]) -> anyhow::Result<()> {
    let mut want: Option<&str> = None;
    for line in checksums_txt.split('\n') {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() == 2 && fields[1] == asset_name {
            want = Some(fields[0]);
            break;
        }
    }
    let want =
        want.ok_or_else(|| anyhow!("checksum de {} não encontrado em checksums.txt", asset_name))?;

    let mut hasher = Sha256::new();
    hasher.update(data);
    let got = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    if got != want {
        return Err(anyhow!(
            "checksum de {} não bate (esperado {}, obtido {})",
            asset_name,
            want,
            got
        ));
    }
    Ok(())
}

pub fn extract_single_file(tgz: &[u8], want_name: &str) -> anyhow::Result<PathBuf> {
    let gz = flate2::read::GzDecoder::new(tgz);
    let mut tr = tar::Archive::new(gz);

    for entry in tr.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().into_owned(),
            None => continue,
        };
        if name != want_name {
            continue;
        }
        let out = std::env::temp_dir().join(format!("{}-{}", want_name, std::process::id()));
        let mut f = open_with_mode(&out, 0o755)?;
        std::io::copy(&mut entry, &mut f)?;
        return Ok(out);
    }
    Err(anyhow!("{} não encontrado no arquivo baixado", want_name))
}

pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> anyhow::Result<()> {
    let f = File::open(archive_path)?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut tr = tar::Archive::new(gz);

    for entry in tr.entries()? {
        let mut entry = entry?;
        let header = entry.header().clone();
        let name = entry.path()?.into_owned();
        let target = dest_dir.join(&name);

        match header.entry_type() {
            tar::EntryType::Directory => {
                std::fs::create_dir_all(&target)?;
            }
            tar::EntryType::Regular => {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mode = header.mode().unwrap_or(0o644);
                let mut out = open_with_mode(&target, mode)?;
                std::io::copy(&mut entry, &mut out)?;
            }
            tar::EntryType::Symlink => {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let _ = std::fs::remove_file(&target);
                if let Some(link_name) = entry.link_name()? {
                    std::os::unix::fs::symlink(link_name, &target)
                        .with_context(|| format!("symlink {}", target.display()))?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn open_with_mode(path: &Path, mode: u32) -> anyhow::Result<File> {
    let f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(mode)
        .open(path)?;
    Ok(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn build_tar_gz(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, content) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, *content).unwrap();
        }
        let tar_bytes = builder.into_inner().unwrap();

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&tar_bytes).unwrap();
        encoder.finish().unwrap()
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

    #[test]
    fn verify_checksum_accepts_matching_hash() {
        let data = b"hello world";
        let hash = sha256_hex(data);
        let checksums = format!("{hash}  sysup_linux_amd64.tar.gz\n");
        assert!(verify_checksum(&checksums, "sysup_linux_amd64.tar.gz", data).is_ok());
    }

    #[test]
    fn verify_checksum_rejects_mismatched_hash() {
        let checksums = format!("{}  sysup_linux_amd64.tar.gz\n", "0".repeat(64));
        let result = verify_checksum(&checksums, "sysup_linux_amd64.tar.gz", b"hello world");
        assert!(result.is_err());
    }

    #[test]
    fn verify_checksum_rejects_missing_entry() {
        let checksums = format!("{}  sysup_darwin_amd64.tar.gz\n", "0".repeat(64));
        let result = verify_checksum(&checksums, "sysup_linux_amd64.tar.gz", b"hello world");
        assert!(result.is_err());
    }

    #[test]
    fn extract_single_file_finds_named_entry_and_sets_mode() {
        let tgz = build_tar_gz(&[
            ("checksums.txt", b"irrelevant"),
            ("sysup", b"fake binary content"),
        ]);

        let out = extract_single_file(&tgz, "sysup").expect("should extract sysup entry");
        let content = std::fs::read(&out).expect("read extracted file");
        assert_eq!(content, b"fake binary content");

        let mode = std::fs::metadata(&out).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);

        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn extract_single_file_errors_when_name_absent() {
        let tgz = build_tar_gz(&[("checksums.txt", b"irrelevant")]);
        let result = extract_single_file(&tgz, "sysup");
        assert!(result.is_err());
    }

    #[test]
    fn extract_tar_gz_writes_files_with_header_mode() {
        let tgz_bytes = build_tar_gz(&[("bin/sysup-worker", b"worker content")]);
        let archive_dir = tempfile::tempdir().expect("tempdir for archive file");
        let archive_path = archive_dir.path().join("archive.tar.gz");
        std::fs::write(&archive_path, &tgz_bytes).expect("write archive to disk");

        let dest_dir = tempfile::tempdir().expect("tempdir for extraction output");
        extract_tar_gz(&archive_path, dest_dir.path()).expect("extract archive");

        let extracted = dest_dir.path().join("bin/sysup-worker");
        let content = std::fs::read(&extracted).expect("read extracted file");
        assert_eq!(content, b"worker content");
    }
}
