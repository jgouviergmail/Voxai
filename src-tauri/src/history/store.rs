use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

const MAX_HISTORY_ENTRIES: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub raw_text: String,
    pub final_text: String,
    pub engine: String,
    pub duration_ms: u64,
    pub created_at: DateTime<Utc>,
}

impl HistoryEntry {
    pub fn new(raw_text: String, final_text: String, engine: String, duration_ms: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            raw_text,
            final_text,
            engine,
            duration_ms,
            created_at: Utc::now(),
        }
    }
}

pub struct HistoryStore {
    entries: Vec<HistoryEntry>,
    path: PathBuf,
}

impl HistoryStore {
    pub fn new(path: PathBuf) -> Result<Self, AppError> {
        let entries = if path.exists() {
            let content = fs::read_to_string(&path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(Self { entries, path })
    }

    pub fn add(&mut self, entry: HistoryEntry) -> Result<(), AppError> {
        self.entries.insert(0, entry);
        self.entries.truncate(MAX_HISTORY_ENTRIES);
        self.persist()
    }

    pub fn get_all(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn clear(&mut self) -> Result<(), AppError> {
        self.entries.clear();
        self.persist()
    }

    fn persist(&self) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.path, content)?;
        Ok(())
    }
}

pub fn history_path() -> Result<PathBuf, AppError> {
    let dir = dirs::config_dir()
        .ok_or_else(|| AppError::Config("Cannot determine config directory".to_string()))?
        .join("Voxai");
    fs::create_dir_all(&dir)?;
    Ok(dir.join("history.json"))
}
