use crate::api::Snapshot;
use anyhow::Result;
use std::{fs, path::PathBuf};

fn cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("mstatui")
        .join("snapshot.json")
}

pub fn load() -> Option<Snapshot> {
    let path = cache_path();
    let body = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&body).ok()
}

pub fn save(snap: &Snapshot) -> Result<()> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string(snap)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}
