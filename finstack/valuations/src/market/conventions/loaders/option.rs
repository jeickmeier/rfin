//! Loader for option conventions embedded in JSON registries.

use super::json::{build_lookup_map_mapped, RegistryFile};
use crate::market::conventions::defs::OptionConventions;
use crate::market::conventions::ids::OptionConventionId;
use finstack_core::dates::BusinessDayConvention;
use finstack_core::Error;
use finstack_core::collections::HashMap;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OptionConventionRecord {
    calendar_id: String,
    settlement_days: i32,
    business_day_convention: BusinessDayConvention,
}

impl OptionConventionRecord {
    fn into_conventions(self) -> Result<OptionConventions, Error> {
        Ok(OptionConventions {
            calendar_id: self.calendar_id,
            settlement_days: self.settlement_days,
            business_day_convention: self.business_day_convention,
        })
    }
}

fn normalize_registry_id(id: &str) -> String {
    id.trim().to_string()
}

/// Load the Option conventions from the embedded JSON registry.
pub fn load_registry() -> Result<HashMap<OptionConventionId, OptionConventions>, Error> {
    let json = include_str!("../../../../data/conventions/option_conventions.json");
    let file: RegistryFile<OptionConventionRecord> = serde_json::from_str(json).map_err(|e| {
        Error::Validation(format!(
            "Failed to parse embedded Option conventions registry JSON: {e}"
        ))
    })?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| {
        rec.clone().into_conventions()
    })?;
    let mut final_map = HashMap::default();
    for (k, v) in string_map {
        final_map.insert(OptionConventionId::new(k), v?);
    }
    Ok(final_map)
}
