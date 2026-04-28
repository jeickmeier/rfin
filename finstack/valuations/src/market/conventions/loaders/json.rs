//! Shared JSON registry loader helpers.

use finstack_core::Error;
use finstack_core::HashMap;

const SUPPORTED_REGISTRY_SCHEMA_MAJOR: u32 = 2;

/// A registry JSON file containing entries with alias IDs.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistryFile<R> {
    /// Schema identifier.
    pub(crate) schema: Option<String>,
    /// Namespace identifier.
    pub(crate) namespace: Option<String>,
    /// Version number.
    pub(crate) version: Option<u32>,
    /// Registry entries.
    pub(crate) entries: Vec<RegistryEntry<R>>,
}

impl<R> RegistryFile<R> {
    /// Validate registry metadata before consuming embedded convention data.
    pub(crate) fn validate_metadata(&self, registry_name: &str) -> Result<(), Error> {
        let schema = self.schema.as_deref().ok_or_else(|| {
            Error::Validation(format!(
                "Embedded {registry_name} conventions registry is missing `schema`"
            ))
        })?;
        if schema.trim().is_empty() {
            return Err(Error::Validation(format!(
                "Embedded {registry_name} conventions registry has an empty `schema`"
            )));
        }

        let schema_major = parse_schema_major(schema).ok_or_else(|| {
            Error::Validation(format!(
                "Embedded {registry_name} conventions registry has unsupported schema identifier `{schema}`"
            ))
        })?;
        if schema_major > SUPPORTED_REGISTRY_SCHEMA_MAJOR {
            return Err(Error::Validation(format!(
                "Embedded {registry_name} conventions registry schema major version {} exceeds supported version {}",
                schema_major, SUPPORTED_REGISTRY_SCHEMA_MAJOR
            )));
        }

        let namespace = self.namespace.as_deref().ok_or_else(|| {
            Error::Validation(format!(
                "Embedded {registry_name} conventions registry is missing `namespace`"
            ))
        })?;
        if namespace.trim().is_empty() {
            return Err(Error::Validation(format!(
                "Embedded {registry_name} conventions registry has an empty `namespace`"
            )));
        }

        let version = self.version.ok_or_else(|| {
            Error::Validation(format!(
                "Embedded {registry_name} conventions registry is missing `version`"
            ))
        })?;
        if version == 0 {
            return Err(Error::Validation(format!(
                "Embedded {registry_name} conventions registry `version` must be positive"
            )));
        }

        Ok(())
    }
}

fn parse_schema_major(schema: &str) -> Option<u32> {
    schema.rsplit_once(".registry.v")?.1.parse().ok()
}

/// One registry record plus its set of alias IDs.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistryEntry<R> {
    /// List of alias IDs.
    pub(crate) ids: Vec<String>,
    /// The record content.
    pub(crate) record: R,
}

/// Normalize a registry ID by trimming whitespace.
pub(crate) fn normalize_registry_id(id: &str) -> String {
    id.trim().to_string()
}

/// Build a lookup map from a registry file while mapping records to a derived value.
pub(crate) fn build_lookup_map_mapped<R, K, V: Clone>(
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

/// Parse a JSON convention registry, convert each record, and re-key using a domain ID wrapper.
///
/// This is the canonical helper for all simple convention loaders. It handles:
/// 1. Deserializing `RegistryFile<R>` from JSON
/// 2. Converting each `R` record via `map_record` (which may return `Result<V>`)
/// 3. Re-keying from `String` to a typed domain ID via `make_id`
///
/// # Errors
///
/// Returns [`Error::Validation`] if JSON parsing fails, if any record conversion fails,
/// or if duplicate IDs are found after normalization.
pub(crate) fn parse_and_rekey<R, Id, V>(
    json: &str,
    registry_name: &str,
    make_id: impl Fn(String) -> Id,
    map_record: impl Fn(&R) -> Result<V, Error>,
) -> Result<HashMap<Id, V>, Error>
where
    R: Clone + for<'de> serde::Deserialize<'de>,
    Id: std::hash::Hash + Eq,
    V: Clone,
{
    let file: RegistryFile<R> = serde_json::from_str(json).map_err(|e| {
        Error::Validation(format!(
            "Failed to parse embedded {registry_name} conventions registry JSON: {e}"
        ))
    })?;
    file.validate_metadata(registry_name)?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| map_record(rec))?;

    let mut final_map = HashMap::default();
    for (k, v) in string_map {
        final_map.insert(make_id(k), v?);
    }
    Ok(final_map)
}
