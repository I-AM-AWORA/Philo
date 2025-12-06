use crate::types::{MatchItem, SourceLink, Package};
use crate::config::Config;
use crate::index::update_with_package;
use anyhow::Result;
use chrono::Utc;
use std::fs;
use std::io::{self, Write};
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

fn fuzzy_score(a: &str, b: &str) -> f32 {
    strsim::normalized_levenshtein(a.to_lowercase().trim(), b.to_lowercase().trim()) as f32
}

fn load_local_index(repo_path: &Path) -> Result<Vec<MatchItem>> {
    let idx_path = repo_path.join("index.json");
    let mut out = vec![];

    if idx_path.exists() {
        let s = fs::read_to_string(&idx_path)?;
        let entries: Vec<crate::types::IndexEntry> = serde_json::from_str(&s).unwrap_or_default();
        for e in entries {
            out.push(MatchItem {
                title: e.title,
                author: e.author,
                year: e.year,
                format: e.format,
                source: "local".into(),
                url: None,
                isbn: None,
                confidence: 1.0,
            });
        }
    }

    let pkg_dir = repo_path.join("packages");
    if pkg_dir.exists() {
        for entry in WalkDir::new(&pkg_dir).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(name) = p.file_stem().and_then(|s| s.to_str()) {
                    out.push(MatchItem {
                        title: name.to_string(),
                        author: "unknown".into(),
                        year: None,
                        format: None,
                        source: "local".into(),
                        url: None,
                        isbn: None,
                        confidence: 0.7,
                    });
                }
            }
        }
    }

    Ok(out)
}

pub fn search_local(repo_path: &Path, query: &str) -> Result<Vec<MatchItem>> {
    let mut res = vec![];
    for m in load_local_index(repo_path)? {
        let mut conf = 0.0;
        conf = conf.max(fuzzy_score(&m.title, query));
        conf = conf.max(fuzzy_score(&m.author, query));
        if conf > 0.75 {
            let mut mm = m.clone();
            mm.confidence = conf;
            res.push(mm);
        }
    }
    res.sort_by(|a,b| b.confidence.partial_cmp(&a.confidence).unwrap());
    Ok(res)
}

// Provider mock — gerçek API için reqwest ekleyebilirsin
async fn search_openlibrary(query: &str) -> Result<Vec<MatchItem>> {
    let mut out = vec![];
    if query.to_lowercase().contains("eleştirisi") {
        out.push(MatchItem {
            title: "Saf Aklın Eleştirisi".into(),
            author: "Immanuel Kant".into(),
            year: Some(1781),
            format: Some("pdf".into()),
            source: "openlibrary".into(),
            url: Some("https://example.org/kant.pdf".into()),
            isbn: Some("978-...".into()),
            confidence: 0.88,
        });
    }
    Ok(out)
}
async fn search_googlebooks(_query: &str) -> Result<Vec<MatchItem>> { Ok(vec![]) }
async fn search_internet_archive(query: &str) -> Result<Vec<MatchItem>> {
    let mut out = vec![];
    if query.to_lowercase().contains("hegel") {
        out.push(MatchItem {
            title: "Tinin Fenomenolojisi".into(),
            author: "G. W. F. Hegel".into(),
            year: Some(1807),
            format: Some("pdf".into()),
            source: "archive".into(),
            url: Some("https://example.org/hegel.pdf".into()),
            isbn: None,
            confidence: 0.83,
        });
    }
    Ok(out)
}
async fn search_providers(query: &str, providers: &[String]) -> Result<Vec<MatchItem>> {
    let mut all = vec![];
    for p in providers {
        let mut res = match p.as_str() {
            "openlibrary" => search_openlibrary(query).await?,
            "googlebooks" => search_googlebooks(query).await?,
            "archive" => search_internet_archive(query).await?,
            _ => vec![],
        };
        all.append(&mut res);
    }
    all.sort_by(|a,b| b.confidence.partial_cmp(&a.confidence).unwrap());
    Ok(all)
}

fn prompt_select(matches: &[MatchItem]) -> Option<MatchItem> {
    println!("\nBulunan adaylar:");
    for (i, m) in matches.iter().enumerate() {
        println!(
            "[{}] {} — {} {} [{}] ({:.2})",
            i,
            m.title,
            m.author,
            m.year.map(|y| format!("({})", y)).unwrap_or_default(),
            m.source,
            m.confidence
        );
    }
    print!("\nSeçmek istediğiniz numara (iptal için boş): ");
    io::stdout().flush().ok()?;
    let mut s = String::new();
    io::stdin().read_line(&mut s).ok()?;
    let s = s.trim();
    if s.is_empty() { return None; }
    let idx: usize = s.parse().ok()?;
    matches.get(idx).cloned()
}

fn create_package(
    repo_path: &Path,
    cfg: &Config,
    item: &MatchItem,
    language: Option<String>,
    publisher: Option<String>,
) -> Result<std::path::PathBuf> {
    let now = Utc::now().to_rfc3339();
    let id = {
        let base = match item.year {
            Some(y) => format!("{}-{}-{}", slugify(&item.author), slugify(&item.title), y),
            None => format!("{}-{}-unknown", slugify(&item.author), slugify(&item.title)),
        };
        format!("{}:{}", cfg.source.namespace, base)
    };

    let mut sources = vec![];
    if let Some(url) = &item.url {
        sources.push(SourceLink {
            name: item.source.clone(),
            url: url.clone(),
            mime: item.format.as_ref().map(|f| {
                if f.eq_ignore_ascii_case("pdf") { "application/pdf".to_string() }
                else if f.eq_ignore_ascii_case("epub") { "application/epub+zip".to_string() }
                else { "application/octet-stream".to_string() }
            }),
            quality: item.confidence,
            checksum_sha256: None,
            registry_source_id: Some(cfg.source.id.clone()),
            registry_source_name: Some(cfg.source.name.clone()),
        });
    }

    let pkg = Package {
        id,
        title: item.title.clone(),
        author: item.author.clone(),
        year: item.year,
        language,
        publisher,
        format: item.format.clone(),
        sources,
        isbn: item.isbn.clone().into_iter().collect(),
        tags: vec!["felsefe".into()],
        created_at: now.clone(),
        updated_at: now,
    };

    let path = package_path(repo_path, &pkg.author, &pkg.title, pkg.year);
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, serde_json::to_string_pretty(&pkg)?)?;
    update_with_package(repo_path, &pkg, &path)?;
    Ok(path)
}

pub async fn run_search_and_package(
    query: &str,
    repo_path: &Path,
    cfg: &Config,
    select: Option<usize>,
    internet_enabled: bool,
    json_out: bool,
) -> Result<()> {
    // Yerel
    let local = search_local(repo_path, query)?;
    if !local.is_empty() {
        if json_out { println!("{}", serde_json::to_string_pretty(&local)?); }
        else {
            println!("Yerel eşleşmeler:");
            for m in &local {
                println!("- {} — {} [{}] ({:.2})", m.title, m.author, m.source, m.confidence);
            }
        }
        return Ok(());
    }

    // İnternet kapalıysa bitir
    if !internet_enabled {
        println!("Yerelde bulunamadı ve internet araması kapalı.");
        return Ok(());
    }

    // Çevrimiçi
    println!("Yerelde yok. İnternette aranıyor: {}", query);
    let candidates = search_providers(query, &cfg.source.providers).await?;
    if candidates.is_empty() { println!("Çevrimiçi sonuç yok."); return Ok(()); }

    let chosen = if let Some(i) = select { candidates.get(i).cloned() } else { prompt_select(&candidates) };
    if let Some(item) = chosen {
        let pkg_path = create_package(repo_path, cfg, &item, None, None)?;
        println!("Paket oluşturuldu: {}", pkg_path.display());
    } else {
        println!("Seçim yapılmadı.");
    }
    Ok(())
}
