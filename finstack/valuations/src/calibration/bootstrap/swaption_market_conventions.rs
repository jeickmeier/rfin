//! Market conventions for swaption volatility calibration.
//!
//! Provides currency-specific market conventions and configurable parameters
//! for swaption calibration to align with market standards.

use finstack_core::dates::{DayCount, Frequency};
use finstack_core::prelude::Currency;
use finstack_core::{F};

/// Market convention configuration for swaption calibration
#[derive(Clone, Debug)]
pub struct SwaptionMarketConvention {
    /// Default day count for the currency
    pub day_count: DayCount,
    /// Fixed leg frequency
    pub fixed_freq: Frequency,
    /// Float leg frequency  
    pub float_freq: Frequency,
    /// Standard expiry points (in years)
    pub standard_expiries: Vec<F>,
    /// Standard tenor points (in years)
    pub standard_tenors: Vec<F>,
    /// Minimum points for SABR calibration
    pub min_sabr_points: usize,
    /// Default volatility for missing data
    pub default_vol: F,
    /// Zero threshold for rate checks
    pub zero_threshold: F,
    /// Payment estimation method
    pub payment_estimation: PaymentEstimation,
}

/// Method for estimating swap payments
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaymentEstimation {
    /// Use proper schedule generation
    ProperSchedule,
    /// Simple approximation (legacy, not recommended)
    SimpleApproximation,
}

impl SwaptionMarketConvention {
    /// USD market conventions
    pub fn usd() -> Self {
        Self {
            day_count: DayCount::Act360,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0
            ],
            standard_tenors: vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0
            ],
            min_sabr_points: 3,
            default_vol: 0.2,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }
    
    /// EUR market conventions
    pub fn eur() -> Self {
        Self {
            day_count: DayCount::Thirty360,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::semi_annual(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0
            ],
            standard_tenors: vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0
            ],
            min_sabr_points: 3,
            default_vol: 0.2,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }
    
    /// GBP market conventions
    pub fn gbp() -> Self {
        Self {
            day_count: DayCount::Act365F,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0
            ],
            standard_tenors: vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0
            ],
            min_sabr_points: 3,
            default_vol: 0.2,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }
    
    /// JPY market conventions
    pub fn jpy() -> Self {
        Self {
            day_count: DayCount::Act365F,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0
            ],
            standard_tenors: vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0
            ],
            min_sabr_points: 3,
            default_vol: 0.2,
            zero_threshold: 1e-8,
            payment_estimation: PaymentEstimation::ProperSchedule,
        }
    }
    
    /// CHF market conventions
    pub fn chf() -> Self {
        Self {
            day_count: DayCount::Thirty360,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::semi_annual(),
            standard_expiries: vec![
                0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0
            ],
            standard_tenors: vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0
            ],
            min_sabr_points: 3,
            default_vol: 0.2,
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
    pub fn with_expiries(mut self, expiries: Vec<F>) -> Self {
        self.standard_expiries = expiries;
        self
    }
    
    pub fn with_tenors(mut self, tenors: Vec<F>) -> Self {
        self.standard_tenors = tenors;
        self
    }
    
    pub fn with_day_count(mut self, day_count: DayCount) -> Self {
        self.day_count = day_count;
        self
    }
    
    pub fn with_fixed_freq(mut self, freq: Frequency) -> Self {
        self.fixed_freq = freq;
        self
    }
    
    pub fn with_float_freq(mut self, freq: Frequency) -> Self {
        self.float_freq = freq;
        self
    }
    
    pub fn with_default_vol(mut self, vol: F) -> Self {
        self.default_vol = vol;
        self
    }
    
    pub fn with_zero_threshold(mut self, threshold: F) -> Self {
        self.zero_threshold = threshold;
        self
    }
    
    pub fn with_min_sabr_points(mut self, min_points: usize) -> Self {
        self.min_sabr_points = min_points;
        self
    }
    
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
        assert_eq!(conv.day_count, DayCount::Act360);
        assert_eq!(conv.fixed_freq, Frequency::semi_annual());
        assert_eq!(conv.float_freq, Frequency::quarterly());
    }
    
    #[test]
    fn test_eur_conventions() {
        let conv = SwaptionMarketConvention::eur();
        assert_eq!(conv.day_count, DayCount::Thirty360);
        assert_eq!(conv.fixed_freq, Frequency::annual());
        assert_eq!(conv.float_freq, Frequency::semi_annual());
    }
    
    #[test]
    fn test_from_currency() {
        let usd_conv = SwaptionMarketConvention::from_currency(Currency::USD);
        assert_eq!(usd_conv.day_count, DayCount::Act360);
        
        let eur_conv = SwaptionMarketConvention::from_currency(Currency::EUR);
        assert_eq!(eur_conv.day_count, DayCount::Thirty360);
        
        let gbp_conv = SwaptionMarketConvention::from_currency(Currency::GBP);
        assert_eq!(gbp_conv.day_count, DayCount::Act365F);
    }
    
    #[test]
    fn test_builder_pattern() {
        let custom_conv = SwaptionMarketConvention::usd()
            .with_day_count(DayCount::ActAct)
            .with_default_vol(0.15)
            .with_zero_threshold(1e-10)
            .with_min_sabr_points(5);
        
        assert_eq!(custom_conv.day_count, DayCount::ActAct);
        assert_eq!(custom_conv.default_vol, 0.15);
        assert_eq!(custom_conv.zero_threshold, 1e-10);
        assert_eq!(custom_conv.min_sabr_points, 5);
    }
}
