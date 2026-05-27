use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerProfile {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub nationality: String,
    pub created_at: String,
    #[serde(default)]
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerProfileIndex {
    pub version: u32,
    pub profiles: Vec<ManagerProfile>,
}

impl ManagerProfileIndex {
    pub fn new() -> Self {
        Self {
            version: 1,
            profiles: Vec::new(),
        }
    }
}

impl Default for ManagerProfileIndex {
    fn default() -> Self {
        Self::new()
    }
}

pub fn load_profiles(path: &Path) -> Result<ManagerProfileIndex, String> {
    if !path.exists() {
        return Ok(ManagerProfileIndex::new());
    }
    let data = fs::read_to_string(path).map_err(|_| "be.error.profiles.loadFailed".to_string())?;
    serde_json::from_str(&data).map_err(|_| "be.error.profiles.loadFailed".to_string())
}

pub fn write_profiles(path: &Path, index: &ManagerProfileIndex) -> Result<(), String> {
    let data = serde_json::to_string_pretty(index)
        .map_err(|_| "be.error.profiles.saveFailed".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| "be.error.profiles.saveFailed".to_string())?;
    }
    fs::write(path, data).map_err(|_| "be.error.profiles.saveFailed".to_string())
}

pub fn add_profile(
    path: &Path,
    first_name: String,
    last_name: String,
    date_of_birth: String,
    nationality: String,
) -> Result<ManagerProfile, String> {
    let mut index = load_profiles(path)?;

    if let Some(existing) = index.profiles.iter_mut().find(|p| {
        p.first_name == first_name
            && p.last_name == last_name
            && p.date_of_birth == date_of_birth
            && p.nationality == nationality
    }) {
        existing.last_used_at = Some(Utc::now().to_rfc3339());
        let updated = existing.clone();
        write_profiles(path, &index)?;
        return Ok(updated);
    }

    let profile = ManagerProfile {
        id: Uuid::new_v4().to_string(),
        first_name,
        last_name,
        date_of_birth,
        nationality,
        created_at: Utc::now().to_rfc3339(),
        last_used_at: None,
    };
    index.profiles.push(profile.clone());
    write_profiles(path, &index)?;
    Ok(profile)
}

/// Add a new profile unconditionally, bypassing dedup. Used when the caller explicitly
/// wants a separate profile entry (e.g. "Save as New" from the edit-confirm modal).
pub fn add_profile_force(
    path: &Path,
    first_name: String,
    last_name: String,
    date_of_birth: String,
    nationality: String,
) -> Result<ManagerProfile, String> {
    let mut index = load_profiles(path)?;
    let profile = ManagerProfile {
        id: Uuid::new_v4().to_string(),
        first_name,
        last_name,
        date_of_birth,
        nationality,
        created_at: Utc::now().to_rfc3339(),
        last_used_at: None,
    };
    index.profiles.push(profile.clone());
    write_profiles(path, &index)?;
    Ok(profile)
}

/// Update an existing profile's identity fields in-place, preserving its id and created_at.
/// Returns the updated profile, or None if no profile with that id was found.
pub fn update_profile(
    path: &Path,
    id: &str,
    first_name: String,
    last_name: String,
    date_of_birth: String,
    nationality: String,
) -> Result<Option<ManagerProfile>, String> {
    let mut index = load_profiles(path)?;
    if let Some(profile) = index.profiles.iter_mut().find(|p| p.id == id) {
        profile.first_name = first_name;
        profile.last_name = last_name;
        profile.date_of_birth = date_of_birth;
        profile.nationality = nationality;
        let updated = profile.clone();
        write_profiles(path, &index)?;
        Ok(Some(updated))
    } else {
        Ok(None)
    }
}

/// Update `last_used_at` to now for the given profile id.
/// Returns `true` if found and updated, `false` if no profile with that id exists.
pub fn touch_profile(path: &Path, id: &str) -> Result<bool, String> {
    let mut index = load_profiles(path)?;
    if let Some(profile) = index.profiles.iter_mut().find(|p| p.id == id) {
        profile.last_used_at = Some(Utc::now().to_rfc3339());
        write_profiles(path, &index)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn remove_profile(path: &Path, id: &str) -> Result<bool, String> {
    let mut index = load_profiles(path)?;
    let before = index.profiles.len();
    index.profiles.retain(|p| p.id != id);
    let removed = index.profiles.len() < before;
    if removed {
        write_profiles(path, &index)?;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_load_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");

        let profile = add_profile(
            &path,
            "José".to_string(),
            "Mourinho".to_string(),
            "1963-01-26".to_string(),
            "PT".to_string(),
        )
        .unwrap();

        assert_eq!(profile.first_name, "José");
        assert_eq!(profile.last_name, "Mourinho");

        let index = load_profiles(&path).unwrap();
        assert_eq!(index.profiles.len(), 1);
        assert_eq!(index.profiles[0].id, profile.id);
    }

    #[test]
    fn test_add_profile_deduplication() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");

        let p1 = add_profile(
            &path,
            "José".to_string(),
            "Mourinho".to_string(),
            "1963-01-26".to_string(),
            "PT".to_string(),
        )
        .unwrap();

        let p2 = add_profile(
            &path,
            "José".to_string(),
            "Mourinho".to_string(),
            "1963-01-26".to_string(),
            "PT".to_string(),
        )
        .unwrap();

        assert_eq!(p1.id, p2.id);
        let index = load_profiles(&path).unwrap();
        assert_eq!(index.profiles.len(), 1);
    }

    #[test]
    fn test_remove_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");

        let profile = add_profile(
            &path,
            "Pep".to_string(),
            "Guardiola".to_string(),
            "1971-01-18".to_string(),
            "ES".to_string(),
        )
        .unwrap();

        let removed = remove_profile(&path, &profile.id).unwrap();
        assert!(removed);

        let index = load_profiles(&path).unwrap();
        assert!(index.profiles.is_empty());

        let not_removed = remove_profile(&path, &profile.id).unwrap();
        assert!(!not_removed);
    }

    #[test]
    fn test_update_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");

        let profile = add_profile(
            &path,
            "José".to_string(),
            "Mourinho".to_string(),
            "1963-01-26".to_string(),
            "PT".to_string(),
        )
        .unwrap();

        let updated = update_profile(
            &path,
            &profile.id,
            "Jose".to_string(),
            "Mourinho".to_string(),
            "1963-01-26".to_string(),
            "ES".to_string(),
        )
        .unwrap()
        .expect("profile should exist");

        assert_eq!(updated.id, profile.id);
        assert_eq!(updated.created_at, profile.created_at);
        assert_eq!(updated.first_name, "Jose");
        assert_eq!(updated.nationality, "ES");

        let index = load_profiles(&path).unwrap();
        assert_eq!(index.profiles.len(), 1);
        assert_eq!(index.profiles[0].nationality, "ES");

        let not_found = update_profile(
            &path,
            "nonexistent-id",
            "A".to_string(),
            "B".to_string(),
            "2000-01-01".to_string(),
            "DE".to_string(),
        )
        .unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_touch_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");

        let profile = add_profile(
            &path,
            "Jürgen".to_string(),
            "Klopp".to_string(),
            "1967-06-16".to_string(),
            "DE".to_string(),
        )
        .unwrap();

        assert!(profile.last_used_at.is_none());

        let touched = touch_profile(&path, &profile.id).unwrap();
        assert!(touched);

        let index = load_profiles(&path).unwrap();
        assert!(index.profiles[0].last_used_at.is_some());

        let not_touched = touch_profile(&path, "nonexistent-id").unwrap();
        assert!(!not_touched);
    }

    #[test]
    fn test_add_profile_force_skips_dedup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");

        let p1 = add_profile(
            &path,
            "Alex".to_string(),
            "Ferguson".to_string(),
            "1941-12-31".to_string(),
            "GB-SCT".to_string(),
        )
        .unwrap();

        let p2 = add_profile_force(
            &path,
            "Alex".to_string(),
            "Ferguson".to_string(),
            "1941-12-31".to_string(),
            "GB-SCT".to_string(),
        )
        .unwrap();

        assert_ne!(p1.id, p2.id);
        let index = load_profiles(&path).unwrap();
        assert_eq!(index.profiles.len(), 2);
    }

    #[test]
    fn test_load_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let index = load_profiles(&path).unwrap();
        assert!(index.profiles.is_empty());
    }
}
