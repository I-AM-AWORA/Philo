use crate::types::{IndexEntry, Package};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_index(repo_path: &Path) -> Result<Vec<IndexEntry>> {
    let path = repo_path.join("index.json");
    if !path.exists() { return Ok(vec![]); }
    let s = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&s).unwrap_or_default())
}

pub fn update_with_package(repo_path: &Path, pkg: &Package, pkg_path: &Path) -> Result<()> {
    let mut entries = load_index(repo_path)?;
    let rel = pkg_path.strip_prefix(repo_path).unwrap_or(pkg_path).to_string_lossy().to_string();

    if let Some(e) = entries.iter_mut().find(|e| e.id == pkg.id) {
        e.title = pkg.title.clone();
        e.author = pkg.author.clone();
        e.year = pkg.year;
        e.format = pkg.format.clone();
        e.packages_path = rel.clone();
    } else {
        entries.push(IndexEntry {
            id: pkg.id.clone(),
            title: pkg.title.clone(),
            author: pkg.author.clone(),
            year: pkg.year,
            format: pkg.format.clone(),
            packages_path: rel,
        });
    }

    fs::write(repo_path.join("index.json"), serde_json::to_string_pretty(&entries)?)?;
    Ok(())
}

pub fn rebuild_index(_repo_path: &Path) -> Result<()> {
    // İleride packages/ üzerinden tam yeniden oluşturma eklenebilir.
    Ok(())
}
