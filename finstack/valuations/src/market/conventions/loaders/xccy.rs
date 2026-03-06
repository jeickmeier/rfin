//! Loader for cross-currency swap conventions embedded in JSON registries.

use super::json::{build_lookup_map_mapped, normalize_registry_id, RegistryFile};
use crate::market::conventions::defs::XccyConventions;
use crate::market::conventions::ids::{IndexId, XccyConventionId};
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::Error;
use finstack_core::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct XccyConventionRecord {
    base_currency: Currency,
    quote_currency: Currency,
    base_index_id: String,
    quote_index_id: String,
    spot_lag_days: i32,
    payment_frequency: String,
    day_count: DayCount,
    business_day_convention: BusinessDayConvention,
    base_calendar_id: String,
    quote_calendar_id: String,
}

impl XccyConventionRecord {
    fn into_conventions(self) -> Result<XccyConventions, Error> {
        if self.base_currency == self.quote_currency {
            return Err(Error::Validation(
                "XCCY conventions must specify different base and quote currencies".to_string(),
            ));
        }
        if self.spot_lag_days < 0 || self.spot_lag_days > 7 {
            return Err(Error::Validation(format!(
                "XCCY spot_lag_days exceeds reasonable bound: {}",
                self.spot_lag_days
            )));
        }

        let payment_frequency = Tenor::parse(&self.payment_frequency).map_err(|e| {
            Error::Validation(format!(
                "Invalid `payment_frequency` in XCCY conventions registry: '{}': {}",
                self.payment_frequency, e
            ))
        })?;

        Ok(XccyConventions {
            base_currency: self.base_currency,
            quote_currency: self.quote_currency,
            base_index_id: IndexId::new(self.base_index_id),
            quote_index_id: IndexId::new(self.quote_index_id),
            spot_lag_days: self.spot_lag_days,
            payment_frequency,
            day_count: self.day_count,
            business_day_convention: self.business_day_convention,
            base_calendar_id: self.base_calendar_id,
            quote_calendar_id: self.quote_calendar_id,
        })
    }
}

/// Load XCCY conventions from the embedded JSON registry.
pub fn load_registry() -> Result<HashMap<XccyConventionId, XccyConventions>, Error> {
    let json = include_str!("../../../../data/conventions/xccy_conventions.json");
    let file: RegistryFile<XccyConventionRecord> = serde_json::from_str(json).map_err(|e| {
        Error::Validation(format!(
            "Failed to parse embedded XCCY conventions registry JSON: {e}"
        ))
    })?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| {
        rec.clone().into_conventions()
    })?;

    let mut final_map = HashMap::default();
    for (k, v) in string_map {
        final_map.insert(XccyConventionId::new(k), v?);
    }
    Ok(final_map)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn eurusd_xccy_conventions_are_available() {
        let registry = load_registry().expect("xccy registry");
        let conv = registry
            .get(&XccyConventionId::new("EUR/USD-XCCY"))
            .expect("EUR/USD-XCCY conventions");

        assert_eq!(conv.base_currency, Currency::EUR);
        assert_eq!(conv.quote_currency, Currency::USD);
        assert_eq!(conv.base_index_id, IndexId::new("EUR-ESTR-OIS"));
        assert_eq!(conv.quote_index_id, IndexId::new("USD-SOFR-OIS"));
        assert_eq!(conv.spot_lag_days, 2);
        assert_eq!(conv.base_calendar_id, "target2");
        assert_eq!(conv.quote_calendar_id, "usny");
    }
}
