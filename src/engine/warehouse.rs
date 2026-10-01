use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use color_eyre::eyre::{Result, WrapErr};

use crate::types::ProduceType;

/// Warehouse tracks accumulated produce from completed agent tasks.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Warehouse {
    /// Per-type produce counts.
    pub counts: HashMap<ProduceType, u32>,
}

/// Warehouse filename within the config directory.
const WAREHOUSE_FILENAME: &str = "warehouse.json";

/// Name a warehouse file that failed to parse is moved to.
const CORRUPT_FILENAME: &str = "warehouse.json.corrupt";

/// Name of the file written before the atomic rename onto the warehouse file.
const TEMP_FILENAME: &str = "warehouse.json.tmp";

impl Warehouse {
    /// Add one unit of produce.
    pub fn add(&mut self, produce: ProduceType) {
        let count = self.counts.entry(produce).or_insert(0);
        *count = count.saturating_add(1);
    }

    /// Get count for a produce type.
    pub fn count(&self, produce: ProduceType) -> u32 {
        self.counts.get(&produce).copied().unwrap_or(0)
    }

    /// Total produce across all types.
    pub fn total(&self) -> u32 {
        self.counts
            .values()
            .fold(0, |sum, n| u32::saturating_add(sum, *n))
    }

    /// Size tier for a produce type: 0 (empty), 1 (small), 2 (medium), 3 (large).
    pub fn tier(&self, produce: ProduceType) -> u8 {
        match self.count(produce) {
            0 => 0,
            1..=3 => 1,
            4..=9 => 2,
            _ => 3,
        }
    }
}

/// Load warehouse from `config_dir/warehouse.json`.
///
/// A missing file yields an empty warehouse. A file that does not parse is renamed to
/// `warehouse.json.corrupt` so the next save cannot destroy it, and an empty warehouse is
/// returned.
///
/// # Errors
///
/// Returns an error when the file exists but cannot be read, or when a corrupt file cannot be
/// renamed.
pub fn load_warehouse(config_dir: &Path) -> Result<Warehouse> {
    let path = config_dir.join(WAREHOUSE_FILENAME);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Warehouse::default()),
        Err(e) => {
            return Err(e)
                .wrap_err_with(|| format!("failed to read warehouse at {}", path.display()));
        }
    };
    match serde_json::from_str(&contents) {
        Ok(warehouse) => Ok(warehouse),
        Err(e) => {
            let backup = config_dir.join(CORRUPT_FILENAME);
            fs::rename(&path, &backup).wrap_err_with(|| {
                format!("failed to move corrupt warehouse {} aside", path.display())
            })?;
            tracing::warn!(
                path = %path.display(),
                backup = %backup.display(),
                "warehouse file is corrupt, starting empty: {e}"
            );
            Ok(Warehouse::default())
        }
    }
}

/// Save warehouse to `config_dir/warehouse.json` through a temporary file and a rename, so a
/// crash never leaves a truncated file.
///
/// # Errors
///
/// Returns an error when the directory, temporary file, or rename fails.
pub fn save_warehouse(config_dir: &Path, warehouse: &Warehouse) -> Result<()> {
    let path = config_dir.join(WAREHOUSE_FILENAME);
    let tmp = config_dir.join(TEMP_FILENAME);
    let json = serde_json::to_string_pretty(warehouse).wrap_err("failed to serialize warehouse")?;
    fs::create_dir_all(config_dir)
        .wrap_err_with(|| format!("failed to create {}", config_dir.display()))?;
    fs::write(&tmp, json.as_bytes())
        .wrap_err_with(|| format!("failed to write warehouse at {}", tmp.display()))?;
    fs::rename(&tmp, &path)
        .wrap_err_with(|| format!("failed to replace warehouse at {}", path.display()))?;
    Ok(())
}

/// Map an AnimType to produce. Returns None for non-productive animations.
pub fn produce_for_anim(anim: crate::types::AnimType) -> Option<ProduceType> {
    match anim {
        crate::types::AnimType::Farm => Some(ProduceType::Wheat),
        crate::types::AnimType::Harvest => Some(ProduceType::Fruit),
        crate::types::AnimType::Fish => Some(ProduceType::Fish),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warehouse_add_and_count() {
        let mut wh = Warehouse::default();
        assert_eq!(wh.count(ProduceType::Wheat), 0);
        wh.add(ProduceType::Wheat);
        wh.add(ProduceType::Wheat);
        assert_eq!(wh.count(ProduceType::Wheat), 2);
        assert_eq!(wh.total(), 2);
    }

    #[test]
    fn warehouse_tier() {
        let mut wh = Warehouse::default();
        assert_eq!(wh.tier(ProduceType::Fish), 0);
        wh.add(ProduceType::Fish);
        assert_eq!(wh.tier(ProduceType::Fish), 1);
        for _ in 0..5 {
            wh.add(ProduceType::Fish);
        }
        assert_eq!(wh.tier(ProduceType::Fish), 2);
        for _ in 0..10 {
            wh.add(ProduceType::Fish);
        }
        assert_eq!(wh.tier(ProduceType::Fish), 3);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut wh = Warehouse::default();
        wh.add(ProduceType::Wheat);
        wh.add(ProduceType::Fruit);
        wh.add(ProduceType::Fruit);
        save_warehouse(dir.path(), &wh).unwrap();

        let loaded = load_warehouse(dir.path()).unwrap();
        assert_eq!(loaded.count(ProduceType::Wheat), 1);
        assert_eq!(loaded.count(ProduceType::Fruit), 2);
    }

    #[test]
    fn load_missing_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let wh = load_warehouse(dir.path()).unwrap();
        assert_eq!(wh.total(), 0);
    }

    #[test]
    fn load_corrupt_file_is_preserved_and_replaced_by_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("warehouse.json");
        fs::write(&path, "{\"counts\": {\"Wheat\": 12").unwrap();

        let wh = load_warehouse(dir.path()).unwrap();

        assert_eq!(wh.total(), 0);
        assert!(!path.exists());
        let kept = fs::read_to_string(dir.path().join("warehouse.json.corrupt")).unwrap();
        assert!(kept.contains("12"));
    }

    #[test]
    fn load_unreadable_file_reports_the_path() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("warehouse.json")).unwrap();

        let err = load_warehouse(dir.path()).unwrap_err();

        assert!(format!("{err:#}").contains("warehouse.json"));
    }

    #[test]
    fn save_creates_missing_directory_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nested").join("config");
        let mut wh = Warehouse::default();
        wh.add(ProduceType::Fish);

        save_warehouse(&config, &wh).unwrap();

        assert_eq!(load_warehouse(&config).unwrap().count(ProduceType::Fish), 1);
        assert!(!config.join("warehouse.json.tmp").exists());
    }

    #[test]
    fn add_saturates_at_the_maximum_count() {
        let mut wh = Warehouse::default();
        wh.counts.insert(ProduceType::Wheat, u32::MAX);
        wh.add(ProduceType::Wheat);
        assert_eq!(wh.count(ProduceType::Wheat), u32::MAX);
    }

    #[test]
    fn total_does_not_overflow() {
        let mut wh = Warehouse::default();
        wh.counts.insert(ProduceType::Wheat, u32::MAX);
        wh.counts.insert(ProduceType::Fish, u32::MAX);
        assert_eq!(wh.total(), u32::MAX);
    }

    #[test]
    fn produce_for_anim_mapping() {
        use crate::types::AnimType;
        assert_eq!(produce_for_anim(AnimType::Farm), Some(ProduceType::Wheat));
        assert_eq!(
            produce_for_anim(AnimType::Harvest),
            Some(ProduceType::Fruit)
        );
        assert_eq!(produce_for_anim(AnimType::Fish), Some(ProduceType::Fish));
        assert_eq!(produce_for_anim(AnimType::Walk), None);
    }
}
