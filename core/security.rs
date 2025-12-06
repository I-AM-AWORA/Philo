use anyhow::{Result, Context, anyhow};
use sha2::{Sha256, Digest};
use std::fs::{self, File};
use std::io::{Read};
use std::path::{Path, PathBuf};
use std::process::Command;

// MIME tespiti için: infer crate’i kullan
pub fn detect_mime(path: &Path) -> Result<String> {
    let data = fs::read(path)?;
    let kind = infer::get(&data);
    Ok(kind.map(|k| k.mime_type().to_string()).unwrap_or_else(|| "application/octet-stream".into()))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

// ClamAV taraması (clamscan kurulu olmalı)
pub fn scan_with_clamav(path: &Path) -> Result<()> {
    let status = Command::new("clamscan")
        .args(["--no-summary", path.to_string_lossy().as_ref()])
        .status()
        .with_context(|| "clamscan çalıştırılamadı")?;
    if !status.success() {
        return Err(anyhow!("Virüs taraması başarısız (enfekte veya hata)"));
    }
    Ok(())
}

// Karantina dizinine güvenli indirme/kopyalama
pub fn put_in_quarantine(repo: &Path, temp_name: &str, bytes: &[u8]) -> Result<PathBuf> {
    let qdir = repo.join("quarantine");
    fs::create_dir_all(&qdir)?;
    let qpath = qdir.join(temp_name);
    fs::write(&qpath, bytes)?;
    Ok(qpath)
}

// Doğrulama: MIME/uzantı uyumu, boyut sınırı, tarama
pub fn validate_quarantine_file(qpath: &Path, expect_ext: Option<&str>, max_size_mb: u64) -> Result<(String, String)> {
    let meta = fs::metadata(qpath)?;
    let size_mb = meta.len() / (1024*1024) as u64;
    if size_mb > max_size_mb {
        return Err(anyhow!("Dosya çok büyük: {} MB", size_mb));
    }

    let mime = detect_mime(qpath)?;
    if let Some(ext) = expect_ext {
        match (ext, mime.as_str()) {
            ("pdf", "application/pdf") |
            ("epub", "application/epub+zip") => {},
            _ => return Err(anyhow!("MIME/uzantı uyuşmazlığı: .{} vs {}", ext, mime)),
        }
    }

    scan_with_clamav(qpath)?;
    let sha = sha256_file(qpath)?;
    Ok((mime, sha))
}

// Temize taşıma (store/)
pub fn promote_to_store(repo: &Path, qpath: &Path, target_name: &str) -> Result<PathBuf> {
    let sdir = repo.join("store");
    fs::create_dir_all(&sdir)?;
    let dest = sdir.join(target_name);
    fs::rename(qpath, &dest)?;
    Ok(dest)
}
