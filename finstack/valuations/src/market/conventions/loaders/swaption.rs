//! Loader for swaption conventions embedded in JSON registries.

use super::json::{build_lookup_map_mapped, RegistryFile};
use crate::market::conventions::defs::SwaptionConventions;
use crate::market::conventions::ids::SwaptionConventionId;
use finstack_core::collections::HashMap;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::Error;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SwaptionConventionRecord {
    calendar_id: String,
    settlement_days: i32,
    business_day_convention: BusinessDayConvention,
    fixed_leg_frequency: String,
    fixed_leg_day_count: DayCount,
    float_leg_index: String,
}

impl SwaptionConventionRecord {
    fn into_conventions(self) -> Result<SwaptionConventions, Error> {
        let fixed_leg_freq = Tenor::parse(&self.fixed_leg_frequency).map_err(|e| {
            Error::Validation(format!(
                "Invalid `fixed_leg_frequency` in Swaption conventions: '{}': {}",
                self.fixed_leg_frequency, e
            ))
        })?;
        Ok(SwaptionConventions {
            calendar_id: self.calendar_id,
            settlement_days: self.settlement_days,
            business_day_convention: self.business_day_convention,
            fixed_leg_frequency: fixed_leg_freq,
            fixed_leg_day_count: self.fixed_leg_day_count,
            float_leg_index: self.float_leg_index,
        })
    }
}

fn normalize_registry_id(id: &str) -> String {
    id.trim().to_string()
}

/// Load the Swaption conventions from the embedded JSON registry.
pub fn load_registry() -> Result<HashMap<SwaptionConventionId, SwaptionConventions>, Error> {
    let json = include_str!("../../../../data/conventions/swaption_conventions.json");
    let file: RegistryFile<SwaptionConventionRecord> = serde_json::from_str(json).map_err(|e| {
        Error::Validation(format!(
            "Failed to parse embedded Swaption conventions registry JSON: {e}"
        ))
    })?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| {
        rec.clone().into_conventions()
    })?;
    let mut final_map = HashMap::default();
    for (k, v) in string_map {
        final_map.insert(SwaptionConventionId::new(k), v?);
    }
    Ok(final_map)
}
