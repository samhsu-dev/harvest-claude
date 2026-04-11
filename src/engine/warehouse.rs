use std::collections::HashMap;
use std::fs;
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

impl Warehouse {
    /// Add one unit of produce.
    pub fn add(&mut self, produce: ProduceType) {
        *self.counts.entry(produce).or_insert(0) += 1;
    }

    /// Get count for a produce type.
    pub fn count(&self, produce: ProduceType) -> u32 {
        self.counts.get(&produce).copied().unwrap_or(0)
    }

    /// Total produce across all types.
    pub fn total(&self) -> u32 {
        self.counts.values().sum()
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

/// Load warehouse from `config_dir/warehouse.json`, returning default if missing.
pub fn load_warehouse(config_dir: &Path) -> Warehouse {
    let path = config_dir.join(WAREHOUSE_FILENAME);
    if !path.exists() {
        return Warehouse::default();
    }
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Warehouse::default(),
    }
}

/// Save warehouse to `config_dir/warehouse.json`.
pub fn save_warehouse(config_dir: &Path, warehouse: &Warehouse) -> Result<()> {
    let path = config_dir.join(WAREHOUSE_FILENAME);
    let json = serde_json::to_string_pretty(warehouse).wrap_err("failed to serialize warehouse")?;
    fs::write(&path, json.as_bytes())
        .wrap_err_with(|| format!("failed to write warehouse at {}", path.display()))?;
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

        let loaded = load_warehouse(dir.path());
        assert_eq!(loaded.count(ProduceType::Wheat), 1);
        assert_eq!(loaded.count(ProduceType::Fruit), 2);
    }

    #[test]
    fn load_missing_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let wh = load_warehouse(dir.path());
        assert_eq!(wh.total(), 0);
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
