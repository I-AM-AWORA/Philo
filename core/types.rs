use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchItem {
    pub title: String,
    pub author: String,
    pub year: Option<u16>,
    pub format: Option<String>,
    pub source: String,
    pub url: Option<String>,
    pub isbn: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceLink {
    pub name: String,                // Örn: "openlibrary", "archive"
    pub url: String,
    pub mime: Option<String>,
    pub quality: f32,
    pub checksum_sha256: Option<String>,
    // Global registry bilgisi (config.source'dan)
    pub registry_source_id: Option<String>,
    pub registry_source_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Package {
    pub id: String,          // namespace ile: "<namespace>:<slug>-<yıl|unknown>"
    pub title: String,
    pub author: String,
    pub year: Option<u16>,
    pub language: Option<String>,
    pub publisher: Option<String>,
    pub format: Option<String>,
    pub sources: Vec<SourceLink>,
    pub isbn: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexEntry {
    pub id: String,
    pub title: String,
    pub author: String,
    pub year: Option<u16>,
    pub format: Option<String>,
    pub packages_path: String,
}
