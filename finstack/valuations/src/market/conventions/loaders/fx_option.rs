//! Loader for FX option conventions embedded in JSON registries.

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

pub(crate) fn load_registry() -> Result<HashMap<FxOptionConventionId, FxOptionConventions>, Error> {
    let json = include_str!("../../../../data/conventions/fx_option_conventions.json");
    super::json::parse_and_rekey(
        json,
        "FX option",
        FxOptionConventionId::new,
        |rec: &FxOptionConventionRecord| rec.clone().into_conventions(),
    )
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
