//! Generic JSON-backed registries for calibration quote conventions.
//!
//! This module provides a small schema and loader for embedding deterministic
//! convention registries (rates/credit/vol/etc.) as JSON and building a lookup
//! map with alias support.
//!
//! Design goals:
//! - **Deterministic**: embedded JSON (no runtime filesystem dependence)
//! - **Generic**: reusable across convention domains (rate index, credit index, vol surfaces)
//! - **Strict**: deny unknown fields; fail fast on duplicate IDs

use std::collections::HashMap;

/// A registry JSON file containing entries with alias IDs.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistryFile<R> {
    /// Optional schema identifier (for humans / future migrations).
    #[allow(dead_code)]
    #[serde(default)]
    pub schema: Option<String>,
    /// Optional namespace identifier (for humans).
    #[allow(dead_code)]
    #[serde(default)]
    pub namespace: Option<String>,
    /// Version number for the file format (for humans / future migrations).
    #[allow(dead_code)]
    #[serde(default)]
    pub version: Option<u32>,
    /// Convention records with associated alias IDs.
    pub entries: Vec<RegistryEntry<R>>,
}

/// One registry record plus its set of alias IDs.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistryEntry<R> {
    /// Alias identifiers that should resolve to the same record.
    pub ids: Vec<String>,
    /// The record payload.
    pub record: R,
}

/// Build a lookup map from a registry file.
///
/// Each alias `id` is normalized by `normalize_id` and mapped to `record`.
/// Duplicate normalized IDs are treated as a hard error to prevent silent
/// convention shadowing.
#[allow(dead_code)]
pub(crate) fn build_lookup_map<R: Clone>(
    file: RegistryFile<R>,
    normalize_id: impl Fn(&str) -> String,
) -> HashMap<String, R> {
    build_lookup_map_mapped(file, normalize_id, |r| r.clone())
}

/// Build a lookup map from a registry file while mapping records to a derived value.
///
/// This is useful when the JSON record is a "raw" representation that needs to be
/// validated and converted into a richer in-memory type.
pub(crate) fn build_lookup_map_mapped<R, V: Clone>(
    file: RegistryFile<R>,
    normalize_id: impl Fn(&str) -> String,
    map_record: impl Fn(&R) -> V,
) -> HashMap<String, V> {
    let mut map: HashMap<String, V> = HashMap::new();
    for entry in file.entries {
        let value = map_record(&entry.record);
        for id in entry.ids {
            let key = normalize_id(&id);
            if map.contains_key(&key) {
                panic!(
                    "Duplicate registry id after normalization: '{}' (from '{}')",
                    key, id
                );
            }
            map.insert(key, value.clone());
        }
    }
    map
}


