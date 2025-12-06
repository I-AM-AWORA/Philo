use anyhow::{Result, anyhow, Context};
use std::process::Command;
use std::path::Path;

pub fn commit_and_push(repo_path: &Path, message: &str) -> Result<()> {
    let repo_str = repo_path.to_string_lossy().to_string();
    let run = |args: &[&str]| -> Result<()> {
        let status = Command::new("git")
            .current_dir(&repo_str)
            .args(args)
            .status()
            .with_context(|| format!("git çalıştırılamadı: {:?}", args))?;
        if !status.success() { return Err(anyhow!("git başarısız: {:?}", args)); }
        Ok(())
    };
    if !repo_path.join(".git").exists() { run(&["init"])?; }
    run(&["add", "."])?;
    let _ = run(&["commit", "-m", message]); // boş commit olabilir
    let _ = run(&["push"]); // remote yoksa atlar
    Ok(())
}

pub fn pull(repo_path: &Path) -> Result<()> {
    let repo_str = repo_path.to_string_lossy().to_string();
    let status = Command::new("git").current_dir(&repo_str).args(&["pull"]).status()?;
    if !status.success() { /* remote yoksa sorun değil */ }
    Ok(())
}
