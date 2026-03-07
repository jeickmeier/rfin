//! Swaption-specific parameters.

use crate::instruments::rates::irs::PayReceive;
use finstack_core::dates::Date;
use finstack_core::dates::{DayCount, Tenor};
use finstack_core::money::Money;
use finstack_core::types::Rate;
use rust_decimal::Decimal;

/// Swaption-specific parameters.
///
/// Groups swaption parameters beyond basic option parameters.
#[derive(Debug, Clone)]
pub struct SwaptionParams {
    /// Notional amount
    pub notional: Money,
    /// Strike rate (fixed rate)
    pub strike: Decimal,
    /// Swaption expiry date
    pub expiry: Date,
    /// Underlying swap start date
    pub swap_start: Date,
    /// Underlying swap end date
    pub swap_end: Date,
    /// Payer/receiver side
    pub side: PayReceive,
    /// Optional override: fixed leg payment frequency
    pub fixed_freq: Option<Tenor>,
    /// Optional override: float leg payment frequency
    pub float_freq: Option<Tenor>,
    /// Optional override: day count convention for year fractions
    pub day_count: Option<DayCount>,
    /// Optional override: volatility model
    pub vol_model: Option<crate::instruments::rates::swaption::types::VolatilityModel>,
}

impl SwaptionParams {
    /// Create payer swaption parameters
    pub fn payer(
        notional: Money,
        strike: f64,
        expiry: Date,
        swap_start: Date,
        swap_end: Date,
    ) -> Self {
        Self {
            notional,
            // Safety: `Decimal::try_from(f64)` only fails for NaN/Inf, which are
            // not valid strike values. Panicking on invalid input is intentional.
            #[allow(clippy::expect_used)]
            strike: Decimal::try_from(strike)
                .expect("strike must be a finite f64 (not NaN or Inf)"),
            expiry,
            swap_start,
            swap_end,
            side: PayReceive::PayFixed,
            fixed_freq: None,
            float_freq: None,
            day_count: None,
            vol_model: None,
        }
    }

    /// Create payer swaption parameters using a typed strike rate.
    pub fn payer_rate(
        notional: Money,
        strike: Rate,
        expiry: Date,
        swap_start: Date,
        swap_end: Date,
    ) -> Self {
        Self {
            notional,
            // Safety: `Rate::as_decimal()` returns an f64; `try_from` only fails for
            // NaN/Inf, which are not valid rates. Panicking on invalid input is intentional.
            #[allow(clippy::expect_used)]
            strike: Decimal::try_from(strike.as_decimal())
                .expect("strike must be a finite rate (not NaN or Inf)"),
            expiry,
            swap_start,
            swap_end,
            side: PayReceive::PayFixed,
            fixed_freq: None,
            float_freq: None,
            day_count: None,
            vol_model: None,
        }
    }

    /// Create receiver swaption parameters
    pub fn receiver(
        notional: Money,
        strike: f64,
        expiry: Date,
        swap_start: Date,
        swap_end: Date,
    ) -> Self {
        Self {
            notional,
            // Safety: `Decimal::try_from(f64)` only fails for NaN/Inf, which are
            // not valid strike values. Panicking on invalid input is intentional.
            #[allow(clippy::expect_used)]
            strike: Decimal::try_from(strike)
                .expect("strike must be a finite f64 (not NaN or Inf)"),
            expiry,
            swap_start,
            swap_end,
            side: PayReceive::ReceiveFixed,
            fixed_freq: None,
            float_freq: None,
            day_count: None,
            vol_model: None,
        }
    }

    /// Create receiver swaption parameters using a typed strike rate.
    pub fn receiver_rate(
        notional: Money,
        strike: Rate,
        expiry: Date,
        swap_start: Date,
        swap_end: Date,
    ) -> Self {
        Self {
            notional,
            // Safety: `Rate::as_decimal()` returns an f64; `try_from` only fails for
            // NaN/Inf, which are not valid rates. Panicking on invalid input is intentional.
            #[allow(clippy::expect_used)]
            strike: Decimal::try_from(strike.as_decimal())
                .expect("strike must be a finite rate (not NaN or Inf)"),
            expiry,
            swap_start,
            swap_end,
            side: PayReceive::ReceiveFixed,
            fixed_freq: None,
            float_freq: None,
            day_count: None,
            vol_model: None,
        }
    }

    /// Override fixed leg payment frequency
    pub fn with_fixed_frequency(mut self, freq: Tenor) -> Self {
        self.fixed_freq = Some(freq);
        self
    }

    /// Override float leg payment frequency
    pub fn with_float_frequency(mut self, freq: Tenor) -> Self {
        self.float_freq = Some(freq);
        self
    }

    /// Override day count convention
    pub fn with_day_count(mut self, dc: DayCount) -> Self {
        self.day_count = Some(dc);
        self
    }

    /// Override volatility model
    pub fn with_vol_model(
        mut self,
        model: crate::instruments::rates::swaption::types::VolatilityModel,
    ) -> Self {
        self.vol_model = Some(model);
        self
    }
}
