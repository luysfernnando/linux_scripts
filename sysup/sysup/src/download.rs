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
    let want = want.ok_or_else(|| {
        anyhow!("checksum de {} não encontrado em checksums.txt", asset_name)
    })?;

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
