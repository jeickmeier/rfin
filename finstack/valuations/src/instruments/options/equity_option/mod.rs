//! Equity option instrument implementation using Black-Scholes model.

pub mod metrics;

use crate::instruments::traits::Attributes;
// use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;

use finstack_core::dates::Date;

use super::models::{d1, d2};
use super::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::math::{norm_cdf, norm_pdf};

/// Equity option instrument
#[derive(Clone, Debug)]
pub struct EquityOption {
    /// Unique instrument identifier
    pub id: String,
    /// Underlying stock ticker
    pub underlying_ticker: String,
    /// Strike price
    pub strike: Money,
    /// Option type (Call or Put)
    pub option_type: OptionType,
    /// Exercise style
    pub exercise_style: ExerciseStyle,
    /// Expiry date
    pub expiry: Date,
    /// Contract size (number of shares per contract)
    pub contract_size: F,
    /// Day count convention for time calculations
    pub day_count: finstack_core::dates::DayCount,
    /// Settlement type
    pub settlement: SettlementType,
    /// Discount curve identifier
    pub disc_id: &'static str,
    /// Spot price identifier for underlying
    pub spot_id: &'static str,
    /// Volatility surface identifier
    pub vol_id: &'static str,
    /// Dividend yield curve identifier (optional)
    pub div_yield_id: Option<&'static str>,
    /// Implied volatility (if known, overrides vol surface)
    pub implied_vol: Option<F>,
    /// Additional attributes
    pub attributes: Attributes,
}

impl EquityOption {
    /// Create a new equity option builder.
    pub fn builder() -> EquityOptionBuilder {
        EquityOptionBuilder::new()
    }

    /// Create a new equity option
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        underlying_ticker: impl Into<String>,
        strike: Money,
        option_type: OptionType,
        expiry: Date,
        contract_size: F,
        disc_id: &'static str,
        spot_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            underlying_ticker: underlying_ticker.into(),
            strike,
            option_type,
            exercise_style: ExerciseStyle::European,
            expiry,
            contract_size,
            day_count: finstack_core::dates::DayCount::Act365F,
            settlement: SettlementType::Physical,
            disc_id,
            spot_id,
            vol_id,
            div_yield_id: None,
            implied_vol: None,
            attributes: Attributes::new(),
        }
    }

    /// Calculate option price using Black-Scholes model
    pub fn black_scholes_price(
        &self,
        spot: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
    ) -> finstack_core::Result<Money> {
        if t <= 0.0 {
            // Option expired
            let intrinsic = match self.option_type {
                OptionType::Call => (spot - self.strike.amount()).max(0.0),
                OptionType::Put => (self.strike.amount() - spot).max(0.0),
            };
            return Ok(Money::new(
                intrinsic * self.contract_size,
                self.strike.currency(),
            ));
        }

        let k = self.strike.amount();
        let d1 = d1(spot, k, r, sigma, t, q);
        let d2 = d2(spot, k, r, sigma, t, q);

        let price = match self.option_type {
            OptionType::Call => {
                spot * (-q * t).exp() * norm_cdf(d1) - k * (-r * t).exp() * norm_cdf(d2)
            }
            OptionType::Put => {
                k * (-r * t).exp() * norm_cdf(-d2) - spot * (-q * t).exp() * norm_cdf(-d1)
            }
        };

        Ok(Money::new(
            price * self.contract_size,
            self.strike.currency(),
        ))
    }

    /// Calculate option delta
    pub fn delta(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 {
            return match self.option_type {
                OptionType::Call => {
                    if spot > self.strike.amount() {
                        1.0
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if spot < self.strike.amount() {
                        -1.0
                    } else {
                        0.0
                    }
                }
            };
        }

        let d1 = d1(spot, self.strike.amount(), r, sigma, t, q);
        let exp_q_t = (-q * t).exp();

        match self.option_type {
            OptionType::Call => exp_q_t * norm_cdf(d1),
            OptionType::Put => -exp_q_t * norm_cdf(-d1),
        }
    }

    /// Calculate option gamma
    pub fn gamma(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 || sigma <= 0.0 {
            return 0.0;
        }

        let d1 = d1(spot, self.strike.amount(), r, sigma, t, q);
        let exp_q_t = (-q * t).exp();

        exp_q_t * norm_pdf(d1) / (spot * sigma * t.sqrt())
    }

    /// Calculate option vega
    pub fn vega(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let d1 = d1(spot, self.strike.amount(), r, sigma, t, q);
        let exp_q_t = (-q * t).exp();

        spot * exp_q_t * norm_pdf(d1) * t.sqrt() / 100.0 // Divide by 100 for 1% vega
    }

    /// Calculate option theta
    pub fn theta(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let k = self.strike.amount();
        let d1 = d1(spot, k, r, sigma, t, q);
        let d2 = d2(spot, k, r, sigma, t, q);
        let sqrt_t = t.sqrt();

        match self.option_type {
            OptionType::Call => {
                let term1 = -spot * norm_pdf(d1) * sigma * (-q * t).exp() / (2.0 * sqrt_t);
                let term2 = q * spot * norm_cdf(d1) * (-q * t).exp();
                let term3 = -r * k * (-r * t).exp() * norm_cdf(d2);
                (term1 + term2 + term3) / 365.0 // Daily theta
            }
            OptionType::Put => {
                let term1 = -spot * norm_pdf(d1) * sigma * (-q * t).exp() / (2.0 * sqrt_t);
                let term2 = -q * spot * norm_cdf(-d1) * (-q * t).exp();
                let term3 = r * k * (-r * t).exp() * norm_cdf(-d2);
                (term1 + term2 + term3) / 365.0 // Daily theta
            }
        }
    }

    /// Calculate option rho
    pub fn rho(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let k = self.strike.amount();
        let d2 = d2(spot, k, r, sigma, t, q);

        match self.option_type {
            OptionType::Call => k * t * (-r * t).exp() * norm_cdf(d2) / 100.0, // Per 1% rate change
            OptionType::Put => -k * t * (-r * t).exp() * norm_cdf(-d2) / 100.0,
        }
    }
}

impl_instrument!(
    EquityOption,
    "EquityOption",
    pv = |s, curves, as_of| {
        // Calculate time to expiry in years
        let time_to_expiry = s.day_count.year_fraction(as_of, s.expiry, finstack_core::dates::DayCountCtx::default())?;

        if time_to_expiry <= 0.0 {
            // Option expired - return intrinsic value
            let spot_scalar = curves.price(s.spot_id)?;
            let spot = match spot_scalar {
                finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
                finstack_core::market_data::primitives::MarketScalar::Price(money) => {
                    money.amount()
                }
            };

            let intrinsic = match s.option_type {
                OptionType::Call => (spot - s.strike.amount()).max(0.0),
                OptionType::Put => (s.strike.amount() - spot).max(0.0),
            };

            return Ok(finstack_core::money::Money::new(
                intrinsic * s.contract_size,
                s.strike.currency(),
            ));
        }

        // Get market data
        let disc_curve = curves.disc(s.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = curves.price(s.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
        };

        // Get dividend yield (default to 0 if not specified)
        let q = if let Some(div_id) = s.div_yield_id {
            match curves.price(div_id) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::primitives::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        // Get volatility (use implied_vol if set, otherwise fetch from surface)
        let sigma = if let Some(impl_vol) = s.implied_vol {
            impl_vol
        } else {
            let vol_surface = curves.surface(s.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, s.strike.amount())
        };

        // Price using Black-Scholes
        s.black_scholes_price(spot, r, sigma, time_to_expiry, q)
    }
);

// Conversions and Attributable provided by macro

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_equity_option_creation() {
        let strike = Money::new(100.0, Currency::USD);
        let expiry = Date::from_calendar_date(2025, Month::December, 31).unwrap();

        let option = EquityOption::new(
            "AAPL_CALL_100",
            "AAPL",
            strike,
            OptionType::Call,
            expiry,
            100.0,
            "USD-OIS",
            "AAPL-SPOT",
            "AAPL-VOL",
        );

        assert_eq!(option.id, "AAPL_CALL_100");
        assert_eq!(option.underlying_ticker, "AAPL");
        assert_eq!(option.strike.amount(), 100.0);
    }

    #[test]
    fn test_black_scholes_call() {
        let strike = Money::new(100.0, Currency::USD);
        let expiry = Date::from_calendar_date(2025, Month::December, 31).unwrap();

        let option = EquityOption::new(
            "CALL",
            "TEST",
            strike,
            OptionType::Call,
            expiry,
            1.0,
            "USD-OIS",
            "TEST-SPOT",
            "TEST-VOL",
        );

        // Test parameters
        let spot = 110.0;
        let r = 0.05;
        let sigma = 0.25;
        let t = 1.0;
        let q = 0.02;

        let price = option.black_scholes_price(spot, r, sigma, t, q).unwrap();

        // Call should have positive value when spot > strike
        assert!(price.amount() > 0.0);

        // Test Greeks
        let delta = option.delta(spot, r, sigma, t, q);
        assert!(delta > 0.0 && delta < 1.0); // Call delta should be between 0 and 1

        let gamma = option.gamma(spot, r, sigma, t, q);
        assert!(gamma > 0.0); // Gamma should be positive
    }

    #[test]
    fn test_black_scholes_put() {
        let strike = Money::new(100.0, Currency::USD);
        let expiry = Date::from_calendar_date(2025, Month::December, 31).unwrap();

        let option = EquityOption::new(
            "PUT",
            "TEST",
            strike,
            OptionType::Put,
            expiry,
            1.0,
            "USD-OIS",
            "TEST-SPOT",
            "TEST-VOL",
        );

        // Test parameters
        let spot = 90.0;
        let r = 0.05;
        let sigma = 0.25;
        let t = 1.0;
        let q = 0.02;

        let price = option.black_scholes_price(spot, r, sigma, t, q).unwrap();

        // Put should have positive value when strike > spot
        assert!(price.amount() > 0.0);

        // Test Greeks
        let delta = option.delta(spot, r, sigma, t, q);
        assert!(delta < 0.0 && delta > -1.0); // Put delta should be between -1 and 0
    }
}

/// Builder pattern for EquityOption instruments
#[derive(Default)]
pub struct EquityOptionBuilder {
    id: Option<String>,
    underlying_ticker: Option<String>,
    strike: Option<Money>,
    option_type: Option<OptionType>,
    exercise_style: Option<ExerciseStyle>,
    expiry: Option<Date>,
    contract_size: Option<F>,
    day_count: Option<finstack_core::dates::DayCount>,
    settlement: Option<SettlementType>,
    disc_id: Option<&'static str>,
    spot_id: Option<&'static str>,
    vol_id: Option<&'static str>,
    div_yield_id: Option<&'static str>,
    implied_vol: Option<F>,
}

impl EquityOptionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    pub fn underlying_ticker(mut self, value: impl Into<String>) -> Self {
        self.underlying_ticker = Some(value.into());
        self
    }

    pub fn strike(mut self, value: Money) -> Self {
        self.strike = Some(value);
        self
    }

    pub fn option_type(mut self, value: OptionType) -> Self {
        self.option_type = Some(value);
        self
    }

    pub fn exercise_style(mut self, value: ExerciseStyle) -> Self {
        self.exercise_style = Some(value);
        self
    }

    pub fn expiry(mut self, value: Date) -> Self {
        self.expiry = Some(value);
        self
    }

    pub fn contract_size(mut self, value: F) -> Self {
        self.contract_size = Some(value);
        self
    }

    pub fn day_count(mut self, value: finstack_core::dates::DayCount) -> Self {
        self.day_count = Some(value);
        self
    }

    pub fn settlement(mut self, value: SettlementType) -> Self {
        self.settlement = Some(value);
        self
    }

    pub fn disc_id(mut self, value: &'static str) -> Self {
        self.disc_id = Some(value);
        self
    }

    pub fn spot_id(mut self, value: &'static str) -> Self {
        self.spot_id = Some(value);
        self
    }

    pub fn vol_id(mut self, value: &'static str) -> Self {
        self.vol_id = Some(value);
        self
    }

    pub fn div_yield_id(mut self, value: &'static str) -> Self {
        self.div_yield_id = Some(value);
        self
    }

    pub fn implied_vol(mut self, value: F) -> Self {
        self.implied_vol = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<EquityOption> {
        Ok(EquityOption {
            id: self.id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            underlying_ticker: self.underlying_ticker.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            strike: self.strike.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            option_type: self.option_type.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            exercise_style: self.exercise_style.unwrap_or(ExerciseStyle::European),
            expiry: self.expiry.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            contract_size: self.contract_size.unwrap_or(1.0),
            day_count: self
                .day_count
                .unwrap_or(finstack_core::dates::DayCount::Act365F),
            settlement: self.settlement.unwrap_or(SettlementType::Physical),
            disc_id: self.disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            spot_id: self.spot_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            vol_id: self.vol_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            div_yield_id: self.div_yield_id,
            implied_vol: self.implied_vol,
            attributes: Attributes::new(),
        })
    }
}
