//! Market conventions for swaption volatility calibration.
//!
//! Provides currency-specific market conventions and configurable parameters
//! for swaption calibration to align with market standards.

use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::prelude::Currency;
use serde::{Deserialize, Serialize};

/// Market convention configuration for swaption calibration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwaptionMarketConvention {
    /// Fixed leg day count (used for swap annuity / PV01).
    pub fixed_day_count: DayCount,
    /// Floating leg day count (used for float PV in multi-curve mode).
    pub float_day_count: DayCount,
    /// Fixed leg business day convention.
    pub fixed_bdc: BusinessDayConvention,
    /// Floating leg business day convention.
    pub float_bdc: BusinessDayConvention,
    /// Fixed leg frequency
    pub fixed_freq: Tenor,
    /// Float leg frequency  
    pub float_freq: Tenor,
    /// Standard expiry points (in years)
    pub standard_expiries: Vec<f64>,
    /// Standard tenor points (in years)
    pub standard_tenors: Vec<f64>,
    /// Minimum points for SABR calibration
    pub min_sabr_points: usize,
    /// Default volatility for missing data
    pub default_vol: f64,
    /// Reporting tolerance used to determine calibration success.
    ///
    /// This is distinct from solver tolerance: it is a *quality threshold* for
    /// residuals (vol units).
    pub vol_tolerance: f64,
    /// Zero threshold for rate checks
    pub zero_threshold: f64,
    /// Payment estimation method
    pub payment_estimation: PaymentEstimation,
}

/// Method for estimating swap payments
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaymentEstimation {
    /// Use proper schedule generation
    ProperSchedule,
}

impl SwaptionMarketConvention {
    /// USD market conventions
    pub fn usd() -> Self {
        Self {
            fixed_day_count: DayCount::Thirty360,
            float_day_count: DayCount::Act360,
            fixed_bdc: BusinessDayConvention::ModifiedFollowing,
            float_bdc: BusinessDayConvention::ModifiedFollowing,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
            ],
            standard_tenors: vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            min_sabr_points: 3,
            default_vol: 0.2,
            vol_tolerance: 0.0015, // 15bp in decimal vol units
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }

    /// EUR market conventions
    pub fn eur() -> Self {
        Self {
            fixed_day_count: DayCount::ThirtyE360,
            float_day_count: DayCount::Act360,
            fixed_bdc: BusinessDayConvention::ModifiedFollowing,
            float_bdc: BusinessDayConvention::ModifiedFollowing,
            fixed_freq: Tenor::annual(),
            float_freq: Tenor::semi_annual(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
            ],
            standard_tenors: vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            min_sabr_points: 3,
            default_vol: 0.2,
            vol_tolerance: 0.0015,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }

    /// GBP market conventions
    pub fn gbp() -> Self {
        Self {
            fixed_day_count: DayCount::Act365F,
            float_day_count: DayCount::Act365F,
            fixed_bdc: BusinessDayConvention::ModifiedFollowing,
            float_bdc: BusinessDayConvention::ModifiedFollowing,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
            ],
            standard_tenors: vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            min_sabr_points: 3,
            default_vol: 0.2,
            vol_tolerance: 0.0015,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }

    /// JPY market conventions
    pub fn jpy() -> Self {
        Self {
            fixed_day_count: DayCount::Act365F,
            float_day_count: DayCount::Act365F,
            fixed_bdc: BusinessDayConvention::ModifiedFollowing,
            float_bdc: BusinessDayConvention::ModifiedFollowing,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
            ],
            standard_tenors: vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            min_sabr_points: 3,
            default_vol: 0.2,
            vol_tolerance: 0.0015,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }

    /// CHF market conventions
    pub fn chf() -> Self {
        Self {
            fixed_day_count: DayCount::ThirtyE360,
            float_day_count: DayCount::Act360,
            fixed_bdc: BusinessDayConvention::ModifiedFollowing,
            float_bdc: BusinessDayConvention::ModifiedFollowing,
            fixed_freq: Tenor::annual(),
            float_freq: Tenor::semi_annual(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
            ],
            standard_tenors: vec![1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
            min_sabr_points: 3,
            default_vol: 0.2,
            vol_tolerance: 0.0015,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }

    /// Create from currency using standard market conventions
    pub fn from_currency(currency: Currency) -> Self {
        match currency {
            Currency::USD => Self::usd(),
            Currency::EUR => Self::eur(),
            Currency::GBP => Self::gbp(),
            Currency::JPY => Self::jpy(),
            Currency::CHF => Self::chf(),
            _ => Self::usd(), // Default to USD conventions for unknown currencies
        }
    }

    /// Builder pattern for customization
    pub fn with_expiries(mut self, expiries: Vec<f64>) -> Self {
        self.standard_expiries = expiries;
        self
    }

    /// Set standard tenors for swaption matrix
    pub fn with_tenors(mut self, tenors: Vec<f64>) -> Self {
        self.standard_tenors = tenors;
        self
    }

    /// Set day count convention for both fixed and floating legs.
    pub fn with_day_count(mut self, day_count: DayCount) -> Self {
        self.fixed_day_count = day_count;
        self.float_day_count = day_count;
        self
    }

    /// Set fixed leg day count convention.
    pub fn with_fixed_day_count(mut self, day_count: DayCount) -> Self {
        self.fixed_day_count = day_count;
        self
    }

    /// Set floating leg day count convention.
    pub fn with_float_day_count(mut self, day_count: DayCount) -> Self {
        self.float_day_count = day_count;
        self
    }

    /// Set fixed leg frequency
    pub fn with_fixed_freq(mut self, freq: Tenor) -> Self {
        self.fixed_freq = freq;
        self
    }

    /// Set floating leg frequency
    pub fn with_float_freq(mut self, freq: Tenor) -> Self {
        self.float_freq = freq;
        self
    }

    /// Set fixed leg business day convention.
    pub fn with_fixed_bdc(mut self, bdc: BusinessDayConvention) -> Self {
        self.fixed_bdc = bdc;
        self
    }

    /// Set floating leg business day convention.
    pub fn with_float_bdc(mut self, bdc: BusinessDayConvention) -> Self {
        self.float_bdc = bdc;
        self
    }

    /// Set default volatility for missing points
    pub fn with_default_vol(mut self, vol: f64) -> Self {
        self.default_vol = vol;
        self
    }

    /// Set the reporting tolerance used to declare calibration success.
    pub fn with_vol_tolerance(mut self, tol: f64) -> Self {
        self.vol_tolerance = tol;
        self
    }

    /// Set threshold below which volatilities are treated as zero
    pub fn with_zero_threshold(mut self, threshold: f64) -> Self {
        self.zero_threshold = threshold;
        self
    }

    /// Set minimum number of points required for SABR calibration
    pub fn with_min_sabr_points(mut self, min_points: usize) -> Self {
        self.min_sabr_points = min_points;
        self
    }

    /// Set payment estimation method for swaption pricing
    pub fn with_payment_estimation(mut self, method: PaymentEstimation) -> Self {
        self.payment_estimation = method;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usd_conventions() {
        let conv = SwaptionMarketConvention::usd();
        assert_eq!(conv.fixed_day_count, DayCount::Thirty360);
        assert_eq!(conv.float_day_count, DayCount::Act360);
        assert_eq!(conv.fixed_freq, Tenor::semi_annual());
        assert_eq!(conv.float_freq, Tenor::quarterly());
    }

    #[test]
    fn test_eur_conventions() {
        let conv = SwaptionMarketConvention::eur();
        assert_eq!(conv.fixed_day_count, DayCount::ThirtyE360);
        assert_eq!(conv.float_day_count, DayCount::Act360);
        assert_eq!(conv.fixed_freq, Tenor::annual());
        assert_eq!(conv.float_freq, Tenor::semi_annual());
    }

    #[test]
    fn test_from_currency() {
        let usd_conv = SwaptionMarketConvention::from_currency(Currency::USD);
        assert_eq!(usd_conv.fixed_day_count, DayCount::Thirty360);

        let eur_conv = SwaptionMarketConvention::from_currency(Currency::EUR);
        assert_eq!(eur_conv.fixed_day_count, DayCount::ThirtyE360);

        let gbp_conv = SwaptionMarketConvention::from_currency(Currency::GBP);
        assert_eq!(gbp_conv.fixed_day_count, DayCount::Act365F);
    }

    #[test]
    fn test_builder_pattern() {
        let custom_conv = SwaptionMarketConvention::usd()
            .with_day_count(DayCount::ActAct)
            .with_default_vol(0.15)
            .with_zero_threshold(1e-10)
            .with_min_sabr_points(5);

        assert_eq!(custom_conv.fixed_day_count, DayCount::ActAct);
        assert_eq!(custom_conv.float_day_count, DayCount::ActAct);
        assert_eq!(custom_conv.default_vol, 0.15);
        assert_eq!(custom_conv.zero_threshold, 1e-10);
        assert_eq!(custom_conv.min_sabr_points, 5);
    }
}
