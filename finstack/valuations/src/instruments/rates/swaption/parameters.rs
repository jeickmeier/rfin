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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::rates::swaption::types::VolatilityModel;
    use finstack_core::currency::Currency;
    use time::macros::date;

    fn sample_dates() -> (Date, Date, Date) {
        (
            date!(2026 - 01 - 15),
            date!(2026 - 07 - 15),
            date!(2031 - 07 - 15),
        )
    }

    #[test]
    fn payer_and_receiver_constructors_set_expected_defaults() {
        let (expiry, swap_start, swap_end) = sample_dates();
        let notional = Money::new(5_000_000.0, Currency::USD);

        let payer = SwaptionParams::payer(notional, 0.0325, expiry, swap_start, swap_end);
        let receiver = SwaptionParams::receiver(notional, 0.031, expiry, swap_start, swap_end);

        assert_eq!(payer.notional, notional);
        assert_eq!(payer.strike, Decimal::new(325, 4));
        assert_eq!(payer.side, PayReceive::PayFixed);
        assert_eq!(payer.expiry, expiry);
        assert_eq!(payer.swap_start, swap_start);
        assert_eq!(payer.swap_end, swap_end);
        assert_eq!(payer.fixed_freq, None);
        assert_eq!(payer.float_freq, None);
        assert_eq!(payer.day_count, None);
        assert_eq!(payer.vol_model, None);

        assert_eq!(receiver.strike, Decimal::new(31, 3));
        assert_eq!(receiver.side, PayReceive::ReceiveFixed);
    }

    #[test]
    fn typed_rate_constructors_preserve_decimal_strike() {
        let (expiry, swap_start, swap_end) = sample_dates();
        let notional = Money::new(1_250_000.0, Currency::EUR);
        let strike = Rate::from_bps(275);

        let payer = SwaptionParams::payer_rate(notional, strike, expiry, swap_start, swap_end);
        let receiver =
            SwaptionParams::receiver_rate(notional, strike, expiry, swap_start, swap_end);

        let expected = Decimal::new(275, 4);
        assert_eq!(payer.strike, expected);
        assert_eq!(payer.side, PayReceive::PayFixed);
        assert_eq!(receiver.strike, expected);
        assert_eq!(receiver.side, PayReceive::ReceiveFixed);
    }

    #[test]
    fn fluent_overrides_replace_optional_configuration() {
        let (expiry, swap_start, swap_end) = sample_dates();
        let params = SwaptionParams::payer(
            Money::new(2_000_000.0, Currency::GBP),
            0.04,
            expiry,
            swap_start,
            swap_end,
        )
        .with_fixed_frequency(Tenor::semi_annual())
        .with_float_frequency(Tenor::quarterly())
        .with_day_count(DayCount::Act365F)
        .with_vol_model(VolatilityModel::Normal);

        assert_eq!(params.fixed_freq, Some(Tenor::semi_annual()));
        assert_eq!(params.float_freq, Some(Tenor::quarterly()));
        assert_eq!(params.day_count, Some(DayCount::Act365F));
        assert_eq!(params.vol_model, Some(VolatilityModel::Normal));
    }

    #[test]
    fn constructors_reject_non_finite_strike_inputs() {
        let (expiry, swap_start, swap_end) = sample_dates();
        let notional = Money::new(1_000_000.0, Currency::USD);

        let payer = std::panic::catch_unwind(|| {
            SwaptionParams::payer(notional, f64::NAN, expiry, swap_start, swap_end)
        });
        let receiver = std::panic::catch_unwind(|| {
            SwaptionParams::receiver(notional, f64::INFINITY, expiry, swap_start, swap_end)
        });

        assert!(payer.is_err(), "NaN strike should panic");
        assert!(receiver.is_err(), "infinite strike should panic");
    }
}
