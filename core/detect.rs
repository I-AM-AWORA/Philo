use crate::config::Config;
use crate::types::{Package, SourceLink};
use crate::index::update_with_package;
use anyhow::Result;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn slugify(s: &str) -> String {
    let s = s.trim().to_lowercase();
    s.chars().map(|c| match c {
        ' ' => '_',
        '/' | '\\' | ':' | ';' | ',' | '.' | '"' | '\'' => '-',
        _ if c.is_alphanumeric() || c == '_' || c == '-' => c,
        _ => '-',
    }).collect::<String>().trim_matches('-').to_string()
}
fn make_package_id_with_ns(namespace: &str, author: &str, title: &str, year: Option<u16>) -> String {
    let base = match year {
        Some(y) => format!("{}-{}-{}", slugify(author), slugify(title), y),
        None => format!("{}-{}-unknown", slugify(author), slugify(title)),
    };
    format!("{namespace}:{base}")
}
fn package_path(base: &Path, author: &str, title: &str, year: Option<u16>) -> PathBuf {
    let author_dir = slugify(author);
    let title_slug = slugify(title);
    let fname = match year {
        Some(y) => format!("{title_slug}-{y}.json"),
        None => format!("{title_slug}-unknown.json"),
    };
    base.join("packages").join(author_dir).join(fname)
}

// Basit tahmin: "Author - Title"
fn infer_from_filename(name: &str) -> (String, String) {
    let parts: Vec<&str> = name.split('-').map(|s| s.trim()).collect();
    if parts.len() >= 2 { (parts[0].to_string(), parts[1].to_string()) } else { ("unknown".into(), name.to_string()) }
}

pub fn scan_folder(folder: &Path) -> Result<Vec<serde_json::Value>> {
    let mut out = vec![];
    for entry in WalkDir::new(folder).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_file() {
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                let (author, title) = infer_from_filename(stem);
                out.push(serde_json::json!({
                    "path": p.display().to_string(),
                    "author_guess": author,
                    "title_guess": title,
                    "ext": p.extension().and_then(|e| e.to_str()).unwrap_or(""),
                }));
            }
        }
    }
    Ok(out)
}

pub fn add_file_as_package(
    repo_path: &Path,
    cfg: &Config,
    file: std::path::PathBuf,
    title_override: Option<String>,
    author_override: Option<String>,
) -> Result<()> {
    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
    let (mut author, mut title) = infer_from_filename(stem);
    if let Some(a) = author_override { author = a; }
    if let Some(t) = title_override { title = t; }

    let now = Utc::now().to_rfc3339();
    let pkg = Package {
        id: make_package_id_with_ns(&cfg.source.namespace, &author, &title, None),
        title,
        author: author.clone(),
        year: None,
        language: None,
        publisher: None,
        format: file.extension().and_then(|e| e.to_str()).map(|s| s.to_string()),
        sources: vec![SourceLink {
            name: "local-file".into(),
            url: file.display().to_string(),
            mime: None,
            quality: 0.7,
            checksum_sha256: None,
            registry_source_id: Some(cfg.source.id.clone()),
            registry_source_name: Some(cfg.source.name.clone()),
        }],
        isbn: vec![],
        tags: vec!["felsefe".into()],
        created_at: now.clone(),
        updated_at: now,
    };

    let path = package_path(repo_path, &pkg.author, &pkg.title, pkg.year);
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, serde_json::to_string_pretty(&pkg)?)?;
    update_with_package(repo_path, &pkg, &path)?;
    Ok(())
}
