use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_ENTRIES: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    pub text: String,
    pub timestamp: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TranslationHistory {
    pub entries: Vec<HistoryEntry>,
    next_id: u64,
}

impl TranslationHistory {
    pub fn load(dir: &PathBuf) -> Self {
        let path = dir.join("history.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self, dir: &PathBuf) {
        if let Ok(json) = serde_json::to_string(self) {
            let _ = std::fs::write(dir.join("history.json"), json);
        }
    }

    pub fn push(&mut self, text: String, dir: &PathBuf) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.entries.insert(0, HistoryEntry { id: self.next_id, text, timestamp });
        self.next_id += 1;
        self.entries.truncate(MAX_ENTRIES);
        self.save(dir);
    }

    pub fn remove(&mut self, id: u64, dir: &PathBuf) {
        self.entries.retain(|e| e.id != id);
        self.save(dir);
    }

    pub fn clear(&mut self, dir: &PathBuf) {
        self.entries.clear();
        self.save(dir);
    }
}
