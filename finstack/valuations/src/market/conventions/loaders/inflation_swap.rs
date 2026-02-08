//! Loader for inflation swap conventions embedded in JSON registries.

use super::json::{build_lookup_map_mapped, RegistryFile};
use crate::market::conventions::defs::InflationSwapConventions;
use crate::market::conventions::ids::InflationSwapConventionId;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::Error;
use finstack_core::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InflationSwapConventionRecord {
    calendar_id: String,
    settlement_days: i32,
    business_day_convention: BusinessDayConvention,
    day_count: DayCount,
    inflation_lag: String,
}

impl InflationSwapConventionRecord {
    fn into_conventions(self) -> Result<InflationSwapConventions, Error> {
        let lag = Tenor::parse(&self.inflation_lag).map_err(|e| {
            Error::Validation(format!(
                "Invalid `inflation_lag` in Inflation Swap conventions: '{}': {}",
                self.inflation_lag, e
            ))
        })?;
        Ok(InflationSwapConventions {
            calendar_id: self.calendar_id,
            settlement_days: self.settlement_days,
            business_day_convention: self.business_day_convention,
            day_count: self.day_count,
            inflation_lag: lag,
        })
    }
}

fn normalize_registry_id(id: &str) -> String {
    id.trim().to_string()
}

/// Load the Inflation Swap conventions from the embedded JSON registry.
pub fn load_registry() -> Result<HashMap<InflationSwapConventionId, InflationSwapConventions>, Error>
{
    let json = include_str!("../../../../data/conventions/inflation_swap_conventions.json");
    let file: RegistryFile<InflationSwapConventionRecord> =
        serde_json::from_str(json).map_err(|e| {
            Error::Validation(format!(
                "Failed to parse embedded Inflation Swap conventions registry JSON: {e}"
            ))
        })?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| {
        rec.clone().into_conventions()
    })?;
    let mut final_map = HashMap::default();
    for (k, v) in string_map {
        final_map.insert(InflationSwapConventionId::new(k), v?);
    }
    Ok(final_map)
}
