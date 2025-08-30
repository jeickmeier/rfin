//! Equity option instrument implementation using Black-Scholes model.

pub mod metrics;

use crate::impl_attributable;
use crate::pricing::result::ValuationResult;
use crate::traits::{Attributes, Priceable};
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::money::Money;
use finstack_core::F;

use finstack_core::dates::Date;

use super::{black_scholes_common, ExerciseStyle, OptionType, SettlementType};

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
    /// Settlement type
    pub settlement: SettlementType,
    /// Discount curve identifier
    pub disc_id: &'static str,
    /// Dividend yield curve identifier (optional)
    pub div_yield_id: Option<&'static str>,
    /// Implied volatility (if known)
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
    pub fn new(
        id: impl Into<String>,
        underlying_ticker: impl Into<String>,
        strike: Money,
        option_type: OptionType,
        expiry: Date,
        contract_size: F,
        disc_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            underlying_ticker: underlying_ticker.into(),
            strike,
            option_type,
            exercise_style: ExerciseStyle::European,
            expiry,
            contract_size,
            settlement: SettlementType::Physical,
            disc_id,
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
        let d1 = black_scholes_common::d1(spot, k, r, sigma, t, q);
        let d2 = black_scholes_common::d2(spot, k, r, sigma, t, q);

        let price = match self.option_type {
            OptionType::Call => {
                spot * (-q * t).exp() * black_scholes_common::norm_cdf(d1)
                    - k * (-r * t).exp() * black_scholes_common::norm_cdf(d2)
            }
            OptionType::Put => {
                k * (-r * t).exp() * black_scholes_common::norm_cdf(-d2)
                    - spot * (-q * t).exp() * black_scholes_common::norm_cdf(-d1)
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

        let d1 = black_scholes_common::d1(spot, self.strike.amount(), r, sigma, t, q);
        let exp_q_t = (-q * t).exp();

        match self.option_type {
            OptionType::Call => exp_q_t * black_scholes_common::norm_cdf(d1),
            OptionType::Put => -exp_q_t * black_scholes_common::norm_cdf(-d1),
        }
    }

    /// Calculate option gamma
    pub fn gamma(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 || sigma <= 0.0 {
            return 0.0;
        }

        let d1 = black_scholes_common::d1(spot, self.strike.amount(), r, sigma, t, q);
        let exp_q_t = (-q * t).exp();

        exp_q_t * black_scholes_common::norm_pdf(d1) / (spot * sigma * t.sqrt())
    }

    /// Calculate option vega
    pub fn vega(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let d1 = black_scholes_common::d1(spot, self.strike.amount(), r, sigma, t, q);
        let exp_q_t = (-q * t).exp();

        spot * exp_q_t * black_scholes_common::norm_pdf(d1) * t.sqrt() / 100.0 // Divide by 100 for 1% vega
    }

    /// Calculate option theta
    pub fn theta(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let k = self.strike.amount();
        let d1 = black_scholes_common::d1(spot, k, r, sigma, t, q);
        let d2 = black_scholes_common::d2(spot, k, r, sigma, t, q);
        let sqrt_t = t.sqrt();

        match self.option_type {
            OptionType::Call => {
                let term1 = -spot * black_scholes_common::norm_pdf(d1) * sigma * (-q * t).exp()
                    / (2.0 * sqrt_t);
                let term2 = q * spot * black_scholes_common::norm_cdf(d1) * (-q * t).exp();
                let term3 = -r * k * (-r * t).exp() * black_scholes_common::norm_cdf(d2);
                (term1 + term2 + term3) / 365.0 // Daily theta
            }
            OptionType::Put => {
                let term1 = -spot * black_scholes_common::norm_pdf(d1) * sigma * (-q * t).exp()
                    / (2.0 * sqrt_t);
                let term2 = -q * spot * black_scholes_common::norm_cdf(-d1) * (-q * t).exp();
                let term3 = r * k * (-r * t).exp() * black_scholes_common::norm_cdf(-d2);
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
        let d2 = black_scholes_common::d2(spot, k, r, sigma, t, q);

        match self.option_type {
            OptionType::Call => k * t * (-r * t).exp() * black_scholes_common::norm_cdf(d2) / 100.0, // Per 1% rate change
            OptionType::Put => {
                -k * t * (-r * t).exp() * black_scholes_common::norm_cdf(-d2) / 100.0
            }
        }
    }
}

impl Priceable for EquityOption {
    /// Compute the present value of the option
    fn value(&self, curves: &CurveSet, _as_of: Date) -> finstack_core::Result<Money> {
        // Get market data
        let _disc = curves.discount(self.disc_id)?;

        // Get spot price from market context (would need to be extended)
        // For now, return error as we need spot price
        Err(finstack_core::Error::from(
            finstack_core::error::InputError::NotFound,
        ))
    }

    /// Compute value with specific metrics
    fn price_with_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        use crate::instruments::Instrument;
        use crate::metrics::MetricContext;
        use std::sync::Arc;

        // Compute base value
        let base_value = self.value(curves, as_of)?;

        // Create metric context
        let _context = MetricContext::new(
            Arc::new(Instrument::EquityOption(self.clone())),
            Arc::new(curves.clone()),
            as_of,
            base_value,
        );

        crate::pricing::build_with_metrics(
            crate::instruments::Instrument::EquityOption(self.clone()),
            curves,
            as_of,
            base_value,
            metrics,
        )
    }

    /// Compute full valuation with all standard option metrics
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        use crate::metrics::MetricId;

        let standard_metrics = vec![
            MetricId::Delta,
            MetricId::Gamma,
            MetricId::Vega,
            MetricId::Theta,
            MetricId::Rho,
        ];

        self.price_with_metrics(curves, as_of, &standard_metrics)
    }
}

// Generate standard Attributable implementation using macro
impl_attributable!(EquityOption);

impl From<EquityOption> for crate::instruments::Instrument {
    fn from(value: EquityOption) -> Self {
        crate::instruments::Instrument::EquityOption(value)
    }
}

impl std::convert::TryFrom<crate::instruments::Instrument> for EquityOption {
    type Error = finstack_core::Error;

    fn try_from(value: crate::instruments::Instrument) -> finstack_core::Result<Self> {
        match value {
            crate::instruments::Instrument::EquityOption(v) => Ok(v),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}

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
    settlement: Option<SettlementType>,
    disc_id: Option<&'static str>,
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

    pub fn settlement(mut self, value: SettlementType) -> Self {
        self.settlement = Some(value);
        self
    }

    pub fn disc_id(mut self, value: &'static str) -> Self {
        self.disc_id = Some(value);
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
            settlement: self.settlement.unwrap_or(SettlementType::Physical),
            disc_id: self.disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            div_yield_id: self.div_yield_id,
            implied_vol: self.implied_vol,
            attributes: Attributes::new(),
        })
    }
}
