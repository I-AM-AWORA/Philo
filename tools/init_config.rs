// tools/init_config.rs
// Basit yardımcı: varsayılan config oluşturur ve repo dizinlerini hazırlar.
//
// Derleme/çalıştırma notu:
// - Bu dosyayı proje içinde tools/ dizinine koyup ayrı bir binary olarak Cargo.toml'a ekleyebilirsiniz.
// - Alternatif: tek başına çalıştırmak için `cargo script` veya küçük bir Cargo workspace kullanın.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use anyhow::{Context, Result};

fn default_config_yaml(repo_path: &str) -> String {
    format!(
r#"repo_path: "{repo}"
prefer_formats: ["pdf", "epub"]
max_size_mb: 200
providers: ["openlibrary", "googlebooks", "archive"]
user_agent: "Philo/0.1"

source:
  id: "philo"
  name: "Philo Registry"
  namespace: "philo"
  default_provider: "openlibrary"
  providers: ["openlibrary", "googlebooks", "archive"]
"#,
        repo = repo_path
    )
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("philo").join("config.yaml"))
}

fn ensure_repo_dirs(repo_path: &PathBuf) -> Result<()> {
    // Temel dizinler: packages, quarantine, store
    let packages = repo_path.join("packages");
    let quarantine = repo_path.join("quarantine");
    let store = repo_path.join("store");
    fs::create_dir_all(&packages).with_context(|| format!("packages dizini oluşturulamadı: {}", packages.display()))?;
    fs::create_dir_all(&quarantine).with_context(|| format!("quarantine dizini oluşturulamadı: {}", quarantine.display()))?;
    fs::create_dir_all(&store).with_context(|| format!("store dizini oluşturulamadı: {}", store.display()))?;
    Ok(())
}

fn write_default_config(path: &PathBuf, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("Config dizini oluşturulamadı: {}", parent.display()))?;
    }
    let mut file = fs::File::create(path).with_context(|| format!("Config dosyası oluşturulamadı: {}", path.display()))?;
    file.write_all(content.as_bytes()).with_context(|| format!("Config dosyası yazılamadı: {}", path.display()))?;
    Ok(())
}

fn main() -> Result<()> {
    // 1) Hedef config yolu
    let cfg_path = config_path().ok_or_else(|| anyhow::anyhow!("Kullanıcı config dizini bulunamadı"))?;

    // 2) Eğer config zaten varsa kullanıcıyı bilgilendir
    if cfg_path.exists() {
        println!("Config zaten mevcut: {}", cfg_path.display());
        println!("Eğer varsayılanı yeniden yazmak istiyorsanız önce mevcut dosyayı silin veya taşıyın.");
        return Ok(());
    }

    // 3) Varsayılan repo yolu (kullanıcı ev dizininde philo_repo)
    let default_repo = dirs::home_dir()
        .map(|h| h.join("philo_repo"))
        .unwrap_or_else(|| PathBuf::from("./philo_repo"));
    let default_repo_str = default_repo.to_string_lossy();

    // 4) Config içeriğini hazırla ve yaz
    let yaml = default_config_yaml(&default_repo_str);
    write_default_config(&cfg_path, &yaml)?;
    println!("Varsayılan config oluşturuldu: {}", cfg_path.display());

    // 5) Repo dizinlerini oluştur
    ensure_repo_dirs(&default_repo)?;
    println!("Varsayılan repo dizinleri oluşturuldu: {}", default_repo.display());
    println!("- packages/, quarantine/, store/ hazır.");

    // 6) Bilgilendirici çıktı
    println!("\nKullanım notu:");
    println!(" - Config dosyasını düzenleyerek repo_path, providers vb. değiştirebilirsiniz.");
    println!(" - CLI çalıştırırken farklı repo kullanmak isterseniz `--repo <yol>` ile geçersiniz.");

    Ok(())
}
