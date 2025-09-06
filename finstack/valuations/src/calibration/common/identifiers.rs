//! Identifier construction helpers for type safety.
//!
//! This module provides standardized functions to construct curve and surface
//! identifiers, reducing manual string formatting and potential errors.

use finstack_core::prelude::Currency;
use finstack_core::types::CurveId;

/// Create a discount curve ID for a given currency.
///
/// Standard format: "{CURRENCY}-OIS"
pub fn discount_curve_id(currency: Currency) -> CurveId {
    CurveId::new(format!("{}-OIS", currency))
}

/// Create a forward curve ID for a given currency and tenor.
///
/// Standard format: "{CURRENCY}-{TENOR}"
pub fn forward_curve_id(currency: Currency, tenor: &str) -> CurveId {
    CurveId::new(format!("{}-{}", currency, tenor))
}

/// Create a volatility surface ID for a given underlying.
///
/// Standard format: "{UNDERLYING}-VOL"
pub fn vol_surface_id(underlying: &str) -> CurveId {
    CurveId::new(format!("{}-VOL", underlying))
}

/// Create a hazard curve ID for an entity and seniority.
///
/// Standard format: "{ENTITY}-{SENIORITY}"
pub fn hazard_curve_id(entity: &str, seniority: &str) -> CurveId {
    CurveId::new(format!("{}-{}", entity, seniority))
}

/// Create a base correlation curve ID for an index and maturity.
///
/// Standard format: "{INDEX}-CORR-{MATURITY}Y"
pub fn base_correlation_curve_id(index: &str, maturity_years: f64) -> CurveId {
    CurveId::new(format!("{}-CORR-{:.0}Y", index, maturity_years))
}

/// Create an inflation curve ID.
///
/// Standard format: "{INDEX}" (no transformation)
pub fn inflation_curve_id(index: &str) -> CurveId {
    CurveId::new(index)
}

/// Create a dividend yield scalar ID for an equity.
///
/// Standard format: "{UNDERLYING}-DIVYIELD"
pub fn dividend_yield_id(underlying: &str) -> CurveId {
    CurveId::new(format!("{}-DIVYIELD", underlying))
}

/// Create a temporary calibration curve ID.
///
/// Used for intermediate curves during calibration that won't be stored.
pub fn temp_calib_id(base: &str, suffix: &str) -> CurveId {
    CurveId::new(format!("CALIB_{}_{}", base, suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discount_curve_id() {
        let id = discount_curve_id(Currency::USD);
        assert_eq!(id.as_str(), "USD-OIS");

        let id = discount_curve_id(Currency::EUR);
        assert_eq!(id.as_str(), "EUR-OIS");
    }

    #[test]
    fn test_forward_curve_id() {
        let id = forward_curve_id(Currency::USD, "SOFR3M");
        assert_eq!(id.as_str(), "USD-SOFR3M");

        let id = forward_curve_id(Currency::EUR, "EURIBOR6M");
        assert_eq!(id.as_str(), "EUR-EURIBOR6M");
    }

    #[test]
    fn test_vol_surface_id() {
        let id = vol_surface_id("SPY");
        assert_eq!(id.as_str(), "SPY-VOL");

        let id = vol_surface_id("EURUSD");
        assert_eq!(id.as_str(), "EURUSD-VOL");
    }

    #[test]
    fn test_hazard_curve_id() {
        let id = hazard_curve_id("AAPL", "Senior");
        assert_eq!(id.as_str(), "AAPL-Senior");
    }

    #[test]
    fn test_base_correlation_curve_id() {
        let id = base_correlation_curve_id("CDX.NA.IG.42", 5.0);
        assert_eq!(id.as_str(), "CDX.NA.IG.42-CORR-5Y");
    }

    #[test]
    fn test_inflation_curve_id() {
        let id = inflation_curve_id("US-CPI-U");
        assert_eq!(id.as_str(), "US-CPI-U");
    }

    #[test]
    fn test_dividend_yield_id() {
        let id = dividend_yield_id("SPY");
        assert_eq!(id.as_str(), "SPY-DIVYIELD");
    }

    #[test]
    fn test_temp_calib_id() {
        let id = temp_calib_id("CURVE", "USD");
        assert_eq!(id.as_str(), "CALIB_CURVE_USD");
    }
}
