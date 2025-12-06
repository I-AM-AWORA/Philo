// cli/main.rs
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::Style;
use indicatif::{ProgressBar, ProgressStyle};

/// Philo CLI — zengin help, version ve küçük animasyonlar içerir.
#[derive(Parser, Debug)]
#[command(name = "philo", about = "Felsefi kitap yöneticisi CLI", version)]
struct PhiloCli {
    /// JSON formatında çıktı
    #[arg(long)]
    json: bool,

    /// İnternet aramasını kapat (yalnızca yerel)
    #[arg(long)]
    no_internet: bool,

    /// Repoya giden yol (varsayılan: config veya ./philo_repo)
    #[arg(long)]
    repo: Option<PathBuf>,

    /// Tercih edilen format sırası (örn: pdf,epub)
    #[arg(long, value_delimiter = ',')]
    prefer: Option<Vec<String>>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Yerel + internet araması (paket oluşturma)
    Search {
        /// Kitap veya yazar sorgusu (tırnaklı yazman iyi olur)
        query: String,

        /// Otomatik seçim için indeks numarası
        #[arg(long)]
        select: Option<usize>,
    },

    /// Klasörü tara ve tahmini meta verileri göster
    Scan {
        /// Klasör yolu
        folder: PathBuf,
    },

    /// Yerel dosyayı paket olarak ekle (metadata çıkarımı basit)
    Add {
        /// Eklenecek dosyanın yolu
        file: PathBuf,

        /// Başlık (opsiyonel)
        #[arg(long)]
        title: Option<String>,

        /// Yazar (opsiyonel)
        #[arg(long)]
        author: Option<String>,
    },

    /// Varsayılan config ve repo dizinlerini oluştur
    InitConfig {},

    /// Repo güncelle (git pull + indeks)
    Update {},
}

fn print_header() {
    let cyan = Style::new().cyan();
    let bold = Style::new().bold();
    println!("{}", cyan.apply_to("Philo — Felsefi Kitap Yöneticisi"));
    println!("{}", bold.apply_to(format!("Version: {}", env!("CARGO_PKG_VERSION"))));
    println!();
}

fn resolve_repo(cli_repo: &Option<PathBuf>, cfg_repo: &str) -> PathBuf {
    cli_repo.clone().unwrap_or_else(|| PathBuf::from(cfg_repo))
}

fn spinner_start(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(msg.to_string());
    pb
}

fn spinner_stop(pb: ProgressBar, done_msg: &str) {
    pb.finish_with_message(done_msg.to_string());
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let cli = PhiloCli::parse();

    // Başlık / versiyon
    print_header();

    // Config yükle (core/config::load_config kullanılıyor)
    let cfg = match philo_core::config::load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config yüklenemedi: {}", e);
            // Yine de devam etmek için default repo kullan
            philo_core::config::load_config().unwrap_or_else(|_| {
                // minimal fallback
                philo_core::config::Config {
                    repo_path: "./philo_repo".into(),
                    prefer_formats: vec!["pdf".into(), "epub".into()],
                    max_size_mb: 200,
                    providers: vec!["openlibrary".into(), "googlebooks".into(), "archive".into()],
                    user_agent: Some("Philo/0.1".into()),
                    source: philo_core::config::SourceConfig {
                        id: "philo".into(),
                        name: "Philo Registry".into(),
                        namespace: "philo".into(),
                        default_provider: "openlibrary".into(),
                        providers: vec!["openlibrary".into(), "googlebooks".into(), "archive".into()],
                    },
                }
            })
        }
    };

    let repo_path = resolve_repo(&cli.repo, &cfg.repo_path);

    match &cli.command {
        Commands::Search { query, select } => {
            // Spinner başlat
            let pb = spinner_start("Yerel arama yapılıyor...");
            // Yerel arama (senkron)
            let local = philo_core::search::search_local(&repo_path, query)?;
            spinner_stop(pb, "Yerel arama tamamlandı.");

            if !local.is_empty() {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&local)?);
                } else {
                    println!("Yerel eşleşmeler:");
                    for m in &local {
                        println!("- {} — {} [{}] ({:.2})", m.title, m.author, m.source, m.confidence);
                    }
                }
                return Ok(ExitCode::SUCCESS);
            }

            if cli.no_internet {
                println!("Yerelde bulunamadı ve internet araması kapalı.");
                return Ok(ExitCode::SUCCESS);
            }

            // İnternette arama — spinner ile
            let pb2 = spinner_start("İnternette arama yapılıyor...");
            let candidates = philo_core::search::search_providers(query, &cfg.source.providers).await?;
            spinner_stop(pb2, "İnternet araması tamamlandı.");

            if candidates.is_empty() {
                println!("Hiç çevrimiçi eşleşme bulunamadı.");
                return Ok(ExitCode::SUCCESS);
            }

            // Seçim: otomatik select varsa onu kullan
            let chosen = if let Some(idx) = select {
                candidates.get(*idx).cloned()
            } else {
                // Prompt ile seçim
                philo_core::search::prompt_select(&candidates)
            };

            if let Some(item) = chosen {
                // Paket oluşturma (cfg geçiriliyor)
                let pb3 = spinner_start("Paket oluşturuluyor...");
                let pkg_path = philo_core::search::create_package(&repo_path, &cfg, &item, None, None)?;
                spinner_stop(pb3, "Paket oluşturuldu.");
                println!("Paket oluşturuldu: {}", pkg_path.display());

                // Opsiyonel commit/push
                if let Err(e) = philo_core::git::commit_and_push(&repo_path, &format!("Add package {}", item.title)) {
                    eprintln!("Git commit/push başarısız: {}", e);
                }
            } else {
                println!("Seçim yapılmadı, işlem iptal edildi.");
            }
        }

        Commands::Scan { folder } => {
            let pb = spinner_start("Klasör taranıyor...");
            let records = philo_core::detect::scan_folder(folder)?;
            spinner_stop(pb, "Tarama tamamlandı.");
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&records)?);
            } else {
                for r in records {
                    println!("{}", serde_json::to_string_pretty(&r)?);
                }
            }
        }

        Commands::Add { file, title, author } => {
            let pb = spinner_start("Dosya paketleniyor...");
            philo_core::detect::add_file_as_package(&repo_path, &cfg, file.clone(), title.clone(), author.clone())?;
            spinner_stop(pb, "Paket oluşturuldu.");
            println!("Dosya paket olarak eklendi.");
            // commit
            if let Err(e) = philo_core::git::commit_and_push(&repo_path, "Add package from file") {
                eprintln!("Git commit/push başarısız: {}", e);
            }
        }

        Commands::InitConfig {} => {
            // Basit init: varsayılan config yaz ve repo dizinlerini oluştur
            let pb = spinner_start("Varsayılan config ve dizinler oluşturuluyor...");
            // reuse tools/init_config logic inline to avoid extra binary dependency
            if let Err(e) = philo_core::config::create_default_config_and_dirs() {
                spinner_stop(pb, "Başarısız.");
                eprintln!("InitConfig hatası: {}", e);
            } else {
                spinner_stop(pb, "Tamamlandı.");
                println!("Varsayılan config ve repo dizinleri oluşturuldu: {}", repo_path.display());
            }
        }

        Commands::Update {} => {
            let pb = spinner_start("Repo güncelleniyor...");
            if let Err(e) = philo_core::git::pull(&repo_path) {
                spinner_stop(pb, "Güncelleme hatası.");
                eprintln!("Git pull hatası: {}", e);
            } else {
                // indeks yeniden oluştur (opsiyonel)
                if let Err(e) = philo_core::index::rebuild_index(&repo_path) {
                    spinner_stop(pb, "İndeks yeniden oluşturulamadı.");
                    eprintln!("İndeks yeniden oluşturma hatası: {}", e);
                } else {
                    spinner_stop(pb, "Repo güncellendi.");
                    println!("Repo güncellendi.");
                }
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
