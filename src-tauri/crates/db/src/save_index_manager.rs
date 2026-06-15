use std::path::{Path, PathBuf};

use crate::save_index::{SaveEntry, SaveIndex, load_index, rebuild_index, write_index};

const SAVE_INDEX_WRITE_ERROR: &str = "be.error.saveIndex.writeFailed";

pub struct SaveIndexManager {
    saves_dir: PathBuf,
    index_path: PathBuf,
    index: SaveIndex,
    index_needs_rebuild: bool,
}

impl SaveIndexManager {
    pub fn init(saves_dir: &Path) -> Result<Self, String> {
        let index_path = saves_dir.join("save_index.json");
        let (index, index_needs_rebuild) = match load_index(&index_path)? {
            Some(index) => (index, false),
            None => (SaveIndex::new(), true),
        };

        Ok(Self {
            saves_dir: saves_dir.to_path_buf(),
            index_path,
            index,
            index_needs_rebuild,
        })
    }

    pub fn ensure_loaded(&mut self) -> Result<(), String> {
        if !self.index_needs_rebuild {
            return Ok(());
        }

        let (index, _) = rebuild_index(&self.saves_dir)?;

        write_index(&self.index_path, &index)?;
        self.index = index;
        self.index_needs_rebuild = false;
        Ok(())
    }

    pub fn list_saves(&self) -> &[SaveEntry] {
        &self.index.saves
    }

    pub fn find(&self, save_id: &str) -> Option<&SaveEntry> {
        self.index.find(save_id)
    }

    pub fn record_new_save(&mut self, entry: SaveEntry) -> Result<(), String> {
        self.index.add(entry);
        self.persist()
    }

    pub fn update_save(&mut self, entry: SaveEntry) -> Result<(), String> {
        if !self.index.update(&entry) {
            return Err(SAVE_INDEX_WRITE_ERROR.to_string());
        }

        self.persist()
    }

    pub fn remove_save(&mut self, save_id: &str) -> Result<bool, String> {
        let removed = self.index.remove(save_id);
        self.persist()?;
        Ok(removed)
    }

    fn persist(&self) -> Result<(), String> {
        write_index(&self.index_path, &self.index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> SaveEntry {
        SaveEntry {
            id: "save-1".to_string(),
            name: "Career".to_string(),
            manager_name: "John Smith".to_string(),
            team_name: "London FC".to_string(),
            db_filename: "save-1.db".to_string(),
            checksum: "checksum".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_played_at: "2026-01-02T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn update_save_returns_backend_key_when_entry_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        let mut manager = SaveIndexManager::init(&saves_dir).unwrap();

        let result = manager.update_save(sample_entry());

        assert_eq!(result.unwrap_err(), SAVE_INDEX_WRITE_ERROR);
    }
}
