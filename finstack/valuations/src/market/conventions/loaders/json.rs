//! Shared JSON registry loader helpers.

use finstack_core::Error;
use finstack_core::HashMap;

/// A registry JSON file containing entries with alias IDs.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryFile<R> {
    /// Optional schema identifier.
    #[serde(default)]
    #[allow(dead_code)]
    pub schema: Option<String>,
    /// Optional namespace identifier.
    #[serde(default)]
    #[allow(dead_code)]
    pub namespace: Option<String>,
    /// Version number.
    #[serde(default)]
    #[allow(dead_code)]
    pub version: Option<u32>,
    /// Registry entries.
    pub entries: Vec<RegistryEntry<R>>,
}

/// One registry record plus its set of alias IDs.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryEntry<R> {
    /// List of alias IDs.
    pub ids: Vec<String>,
    /// The record content.
    pub record: R,
}

/// Normalize a registry ID by trimming whitespace.
pub fn normalize_registry_id(id: &str) -> String {
    id.trim().to_string()
}

/// Build a lookup map from a registry file while mapping records to a derived value.
pub fn build_lookup_map_mapped<R, K, V: Clone>(
    file: RegistryFile<R>,
    normalize_id: impl Fn(&str) -> K,
    map_record: impl Fn(&R) -> V,
) -> Result<HashMap<K, V>, Error>
where
    K: std::hash::Hash + Eq + std::fmt::Display,
{
    let mut map: HashMap<K, V> = HashMap::default();
    for entry in file.entries {
        let value = map_record(&entry.record);
        for id in entry.ids {
            let key = normalize_id(&id);
            if map.contains_key(&key) {
                return Err(Error::Validation(format!(
                    "Duplicate registry id after normalization: '{}' (from '{}')",
                    key, id
                )));
            }
            map.insert(key, value.clone());
        }
    }
    Ok(map)
}
