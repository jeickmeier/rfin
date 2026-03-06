//! Loader for FX option conventions embedded in JSON registries.

use super::json::{build_lookup_map_mapped, normalize_registry_id, RegistryFile};
use crate::instruments::{ExerciseStyle, SettlementType};
use crate::market::conventions::defs::FxOptionConventions;
use crate::market::conventions::ids::{FxConventionId, FxOptionConventionId};
use finstack_core::dates::DayCount;
use finstack_core::Error;
use finstack_core::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FxOptionConventionRecord {
    fx_convention_id: String,
    exercise_style: ExerciseStyle,
    settlement: SettlementType,
    day_count: DayCount,
}

impl FxOptionConventionRecord {
    fn into_conventions(self) -> Result<FxOptionConventions, Error> {
        Ok(FxOptionConventions {
            fx_convention_id: FxConventionId::new(self.fx_convention_id),
            exercise_style: self.exercise_style,
            settlement: self.settlement,
            day_count: self.day_count,
        })
    }
}

pub fn load_registry() -> Result<HashMap<FxOptionConventionId, FxOptionConventions>, Error> {
    let json = include_str!("../../../../data/conventions/fx_option_conventions.json");
    let file: RegistryFile<FxOptionConventionRecord> = serde_json::from_str(json).map_err(|e| {
        Error::Validation(format!(
            "Failed to parse embedded FX option conventions registry JSON: {e}"
        ))
    })?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| {
        rec.clone().into_conventions()
    })?;

    let mut final_map = HashMap::default();
    for (k, v) in string_map {
        final_map.insert(FxOptionConventionId::new(k), v?);
    }
    Ok(final_map)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn eurusd_vanilla_fx_option_conventions_are_available() {
        let registry = load_registry().expect("fx option registry");
        let conv = registry
            .get(&FxOptionConventionId::new("EUR/USD-VANILLA"))
            .expect("EUR/USD-VANILLA conventions");

        assert_eq!(conv.fx_convention_id, FxConventionId::new("EUR/USD"));
        assert_eq!(conv.exercise_style, ExerciseStyle::European);
        assert_eq!(conv.settlement, SettlementType::Cash);
        assert_eq!(conv.day_count, DayCount::Act365F);
    }
}
