//! Loader for rate index conventions embedded in JSON registries.

use super::json::{build_lookup_map_mapped, RegistryFile};
use crate::instruments::irs::FloatingLegCompounding;
use crate::market::conventions::defs::{RateIndexConventions, RateIndexKind};
use crate::market::conventions::ids::IndexId; // Used for normalization if needed, or string
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::Currency;
use finstack_core::Error;
use finstack_core::HashMap;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RateIndexConventionsRecord {
    currency: Currency,
    kind: RateIndexKind,
    #[serde(default)]
    tenor: Option<String>,
    day_count: DayCount,
    default_payment_frequency: String,
    default_payment_delay_days: i32,
    default_reset_lag_days: i32,
    #[serde(default)]
    ois_compounding: Option<OisCompoundingSpec>,
    market_calendar_id: String,
    market_settlement_days: i32,
    market_business_day_convention: BusinessDayConvention,
    default_fixed_leg_day_count: DayCount,
    default_fixed_leg_frequency: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum OisCompoundingSpec {
    Sofr,
    Sonia,
    Estr,
    Tona,
    Fedfunds,
    Saron,
    CompoundedInArrears {
        lookback_days: i32,
        observation_shift: Option<i32>,
    },
}

impl OisCompoundingSpec {
    fn to_compounding(&self) -> FloatingLegCompounding {
        match self {
            Self::Sofr => FloatingLegCompounding::sofr(),
            Self::Sonia => FloatingLegCompounding::sonia(),
            Self::Estr => FloatingLegCompounding::estr(),
            Self::Tona => FloatingLegCompounding::tona(),
            Self::Fedfunds => FloatingLegCompounding::fedfunds(),
            Self::Saron => FloatingLegCompounding::CompoundedInArrears {
                lookback_days: 2,
                observation_shift: None,
            },
            Self::CompoundedInArrears {
                lookback_days,
                observation_shift,
            } => FloatingLegCompounding::CompoundedInArrears {
                lookback_days: *lookback_days,
                observation_shift: *observation_shift,
            },
        }
    }
}

impl RateIndexConventionsRecord {
    fn into_conventions(self) -> Result<RateIndexConventions, Error> {
        let tenor = match self.tenor {
            Some(s) => Some(Tenor::parse(&s).map_err(|e| {
                Error::Validation(format!(
                    "Invalid `tenor` in rate index conventions registry: '{}': {}",
                    s, e
                ))
            })?),
            None => None,
        };

        let default_payment_frequency = Tenor::parse(&self.default_payment_frequency).map_err(
            |e| {
                Error::Validation(format!(
                    "Invalid `default_payment_frequency` in rate index conventions registry: '{}': {}",
                    self.default_payment_frequency, e
                ))
            },
        )?;

        let default_fixed_leg_frequency =
            Tenor::parse(&self.default_fixed_leg_frequency).map_err(|e| {
                Error::Validation(format!(
                    "Invalid `default_fixed_leg_frequency` in rate index conventions registry: '{}': {}",
                    self.default_fixed_leg_frequency, e
                ))
            })?;

        let ois_compounding = self.ois_compounding.as_ref().map(|s| s.to_compounding());

        // Basic invariants
        match self.kind {
            RateIndexKind::OvernightRfr => {
                if tenor.is_some() {
                    return Err(Error::Validation(
                        "Overnight RFR index conventions must not specify a tenor".to_string(),
                    ));
                }
                if ois_compounding.is_none() {
                    return Err(Error::Validation(
                        "Overnight RFR index conventions must specify `ois_compounding`"
                            .to_string(),
                    ));
                }
            }
            RateIndexKind::Term => {
                if ois_compounding.is_some() {
                    return Err(Error::Validation(
                        "Term index conventions must not specify `ois_compounding`".to_string(),
                    ));
                }
            }
        }

        Ok(RateIndexConventions {
            currency: self.currency,
            kind: self.kind,
            tenor,
            day_count: self.day_count,
            default_payment_frequency,
            default_payment_delay_days: self.default_payment_delay_days,
            default_reset_lag_days: self.default_reset_lag_days,
            ois_compounding,
            market_calendar_id: self.market_calendar_id,
            market_settlement_days: self.market_settlement_days,
            market_business_day_convention: self.market_business_day_convention,
            default_fixed_leg_day_count: self.default_fixed_leg_day_count,
            default_fixed_leg_frequency,
        })
    }
}

fn normalize_registry_id(id: &str) -> String {
    id.trim().to_string()
}

/// Load the rate index conventions from the embedded JSON registry.
pub fn load_registry() -> Result<HashMap<IndexId, RateIndexConventions>, Error> {
    let json = include_str!("../../../../data/conventions/rate_index_conventions.json");
    let file: RegistryFile<RateIndexConventionsRecord> =
        serde_json::from_str(json).map_err(|e| {
            Error::Validation(format!(
                "Failed to parse embedded rate index conventions registry JSON: {e}"
            ))
        })?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| {
        rec.clone().into_conventions()
    })?;

    // Convert keys to IndexId
    let mut final_map = HashMap::default();
    for (k, v) in string_map {
        final_map.insert(IndexId::new(k), v?);
    }
    Ok(final_map)
}
