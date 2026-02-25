//! Credit option specific parameters.
//!
//! # Validation
//!
//! All constructors validate inputs at creation time to ensure market-standard compliance:
//! - Strike spread must be positive (≤0 is invalid)
//! - Option expiry must precede underlying CDS maturity
//! - Recovery rate (in parent CreditParams) must be in (0, 1)
//! - Index factor must be in (0, 1] when specified

use crate::instruments::common_impl::parameters::OptionType;
use finstack_core::dates::DayCount;
use finstack_core::types::Bps;
use finstack_core::{dates::Date, money::Money};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// Minimum valid strike as a decimal rate (exclusive lower bound).
pub const MIN_STRIKE: f64 = 0.0;

/// Maximum valid strike as a decimal rate (inclusive upper bound).
/// 1.0 decimal = 10000bp = 100% spread, which is extremely rare.
pub const MAX_STRIKE: f64 = 1.0;

/// Credit option specific parameters.
///
/// Deal-level inputs for an option on a CDS spread.
/// Ownership clarifications to avoid duplication with `CreditParams`:
/// - This struct holds strike (decimal rate), expiry, underlying CDS maturity, notional, option type.
/// - Reference entity, recovery rate, and hazard `credit_id` live in `CreditParams`.
/// - Discount `discount_curve_id` and vol `vol_surface_id` are instrument-level market IDs passed to `CDSOption::try_new`.
///
/// # Validation
///
/// All inputs are validated at construction:
/// - `strike`: Must be in (0, 1.0] as a decimal rate
/// - `expiry`: Must be before `cds_maturity`
/// - `index_factor`: Must be in (0, 1] when specified
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CDSOptionParams {
    /// Strike spread as a decimal rate (e.g., 0.01 = 100bp)
    #[serde(alias = "strike_spread_bp")]
    pub strike: Decimal,
    /// Option expiry date (must be before cds_maturity)
    pub expiry: Date,
    /// Underlying CDS maturity date
    pub cds_maturity: Date,
    /// Notional amount
    pub notional: Money,
    /// Option type (Call/Put)
    pub option_type: OptionType,
    /// Whether the underlying is a CDS index (vs single-name CDS)
    #[serde(default)]
    pub underlying_is_index: bool,
    /// Optional index factor scaling for index underlyings (e.g., 0.8). Must be in (0, 1].
    pub index_factor: Option<f64>,
    /// Forward spread adjustment as a decimal rate (e.g., 0.0025 = 25bp)
    #[serde(alias = "forward_spread_adjust_bp")]
    #[serde(default)]
    pub forward_spread_adjust: Decimal,
    /// Day count convention for time calculations (defaults to Act/360 per ISDA)
    pub day_count: DayCount,
}

impl CDSOptionParams {
    /// Validate the parameters and return an error if invalid.
    fn validate(&self) -> finstack_core::Result<()> {
        let strike_f64 = self.strike.to_f64().unwrap_or(0.0);

        if strike_f64 <= MIN_STRIKE {
            return Err(finstack_core::Error::Validation(format!(
                "strike must be positive, got {}",
                self.strike
            )));
        }
        if strike_f64 > MAX_STRIKE {
            return Err(finstack_core::Error::Validation(format!(
                "strike {} exceeds maximum {}",
                self.strike, MAX_STRIKE
            )));
        }

        // Date validation: expiry must be before CDS maturity
        if self.expiry >= self.cds_maturity {
            return Err(finstack_core::Error::Validation(format!(
                "option expiry ({}) must be before CDS maturity ({})",
                self.expiry, self.cds_maturity
            )));
        }

        // Index factor validation
        if let Some(factor) = self.index_factor {
            if factor <= 0.0 || factor > 1.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "index_factor must be in (0, 1], got {}",
                    factor
                )));
            }
        }

        Ok(())
    }

    /// Create new credit option parameters with validation.
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike spread as a decimal rate (e.g., `Decimal::new(1, 2)` for 100bp = 0.01)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `strike` is not positive or exceeds 1.0 (10000bp)
    /// - `expiry` is not before `cds_maturity`
    pub fn new(
        strike: Decimal,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
        option_type: OptionType,
    ) -> finstack_core::Result<Self> {
        let params = Self {
            strike,
            expiry,
            cds_maturity,
            notional,
            option_type,
            underlying_is_index: false,
            index_factor: None,
            forward_spread_adjust: Decimal::ZERO,
            day_count: DayCount::Act360, // ISDA standard
        };
        params.validate()?;
        Ok(params)
    }

    /// Create new credit option parameters using typed basis points.
    ///
    /// Converts from basis points to decimal rate internally.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `strike_bps` is not positive or exceeds 10000bp
    /// - `expiry` is not before `cds_maturity`
    pub fn new_bps(
        strike_bps: Bps,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
        option_type: OptionType,
    ) -> finstack_core::Result<Self> {
        let bp_value = strike_bps.as_bps() as f64;
        let strike = Decimal::try_from(bp_value / 10000.0).map_err(|e| {
            finstack_core::Error::Validation(format!("Invalid strike from bps: {}", e))
        })?;
        Self::new(strike, expiry, cds_maturity, notional, option_type)
    }

    /// Create credit call option parameters with validation.
    pub fn call(
        strike: Decimal,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
    ) -> finstack_core::Result<Self> {
        Self::new(strike, expiry, cds_maturity, notional, OptionType::Call)
    }

    /// Create credit call option parameters using typed basis points.
    pub fn call_bps(
        strike_bps: Bps,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
    ) -> finstack_core::Result<Self> {
        Self::new_bps(strike_bps, expiry, cds_maturity, notional, OptionType::Call)
    }

    /// Create credit put option parameters with validation.
    pub fn put(
        strike: Decimal,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
    ) -> finstack_core::Result<Self> {
        Self::new(strike, expiry, cds_maturity, notional, OptionType::Put)
    }

    /// Create credit put option parameters using typed basis points.
    pub fn put_bps(
        strike_bps: Bps,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
    ) -> finstack_core::Result<Self> {
        Self::new_bps(strike_bps, expiry, cds_maturity, notional, OptionType::Put)
    }

    /// Mark this option as referencing a CDS index and set an index factor.
    ///
    /// # Arguments
    ///
    /// * `index_factor` - Scale factor in (0, 1]. E.g., 0.85 means 85% of original index notional.
    ///
    /// # Errors
    ///
    /// Returns an error if `index_factor` is not in (0, 1].
    pub fn as_index(mut self, index_factor: f64) -> finstack_core::Result<Self> {
        self.underlying_is_index = true;
        self.index_factor = Some(index_factor);
        self.validate()?;
        Ok(self)
    }

    /// Apply a forward spread adjustment as a decimal rate (e.g., 0.0025 = 25bp).
    #[must_use]
    pub fn with_forward_spread_adjust(mut self, adjust: Decimal) -> Self {
        self.forward_spread_adjust = adjust;
        self
    }

    /// Apply a forward spread adjustment using typed basis points.
    #[must_use]
    pub fn with_forward_spread_adjust_bps(mut self, adjust_bp: Bps) -> Self {
        let bp_value = adjust_bp.as_bps() as f64;
        self.forward_spread_adjust = Decimal::try_from(bp_value / 10000.0).unwrap_or(Decimal::ZERO);
        self
    }

    /// Set the day count convention for time calculations.
    ///
    /// Defaults to Act/360 per ISDA CDS standard conventions.
    #[must_use]
    pub fn with_day_count(mut self, day_count: DayCount) -> Self {
        self.day_count = day_count;
        self
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::macros::date;

    #[test]
    fn test_valid_params_creation() {
        let result = CDSOptionParams::call(
            Decimal::new(1, 2), // 0.01 = 100bp
            date!(2025 - 06 - 20),
            date!(2030 - 06 - 20),
            Money::new(10_000_000.0, Currency::USD),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_strike_zero() {
        let result = CDSOptionParams::call(
            Decimal::ZERO,
            date!(2025 - 06 - 20),
            date!(2030 - 06 - 20),
            Money::new(10_000_000.0, Currency::USD),
        );
        assert!(result.is_err());
        assert!(result
            .expect_err("Expected error for zero strike")
            .to_string()
            .contains("strike must be positive"));
    }

    #[test]
    fn test_invalid_strike_negative() {
        let result = CDSOptionParams::call(
            Decimal::new(-5, 3), // -0.005 = -50bp
            date!(2025 - 06 - 20),
            date!(2030 - 06 - 20),
            Money::new(10_000_000.0, Currency::USD),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_expiry_after_maturity() {
        let result = CDSOptionParams::call(
            Decimal::new(1, 2),    // 0.01 = 100bp
            date!(2030 - 06 - 21), // After maturity
            date!(2030 - 06 - 20),
            Money::new(10_000_000.0, Currency::USD),
        );
        assert!(result.is_err());
        assert!(result
            .expect_err("Expected error for expiry after maturity")
            .to_string()
            .contains("must be before CDS maturity"));
    }

    #[test]
    fn test_invalid_index_factor() {
        let params = CDSOptionParams::call(
            Decimal::new(1, 2), // 0.01 = 100bp
            date!(2025 - 06 - 20),
            date!(2030 - 06 - 20),
            Money::new(10_000_000.0, Currency::USD),
        )
        .expect("Valid CDS option params");

        // Index factor > 1 is invalid
        let result = params.as_index(1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_index_factor() {
        let params = CDSOptionParams::call(
            Decimal::new(1, 2), // 0.01 = 100bp
            date!(2025 - 06 - 20),
            date!(2030 - 06 - 20),
            Money::new(10_000_000.0, Currency::USD),
        )
        .expect("Valid CDS option params");

        let result = params.as_index(0.85);
        assert!(result.is_ok());
        let indexed = result.expect("Valid index conversion");
        assert!(indexed.underlying_is_index);
        assert_eq!(indexed.index_factor, Some(0.85));
    }
}
