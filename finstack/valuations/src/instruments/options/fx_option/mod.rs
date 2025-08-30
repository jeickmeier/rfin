//! FX option instrument implementation using Garman-Kohlhagen model.

pub mod metrics;

use crate::impl_attributable;
use crate::pricing::result::ValuationResult;
use crate::traits::{Attributes, Priceable};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::money::Money;
use finstack_core::F;

use super::{black_scholes_common, ExerciseStyle, OptionType, SettlementType};

/// FX option instrument (Garman-Kohlhagen model)
#[derive(Clone, Debug)]
pub struct FxOption {
    /// Unique instrument identifier
    pub id: String,
    /// Base currency (foreign currency)
    pub base_currency: Currency,
    /// Quote currency (domestic currency)
    pub quote_currency: Currency,
    /// Strike price (units of quote per base)
    pub strike: F,
    /// Option type (Call or Put)
    pub option_type: OptionType,
    /// Exercise style
    pub exercise_style: ExerciseStyle,
    /// Expiry date
    pub expiry: Date,
    /// Notional amount in base currency
    pub notional: Money,
    /// Settlement type
    pub settlement: SettlementType,
    /// Domestic discount curve identifier
    pub domestic_disc_id: &'static str,
    /// Foreign discount curve identifier
    pub foreign_disc_id: &'static str,
    /// Implied volatility (if known)
    pub implied_vol: Option<F>,
    /// Additional attributes
    pub attributes: Attributes,
}

impl FxOption {
    /// Create a new FX option
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        base_currency: Currency,
        quote_currency: Currency,
        strike: F,
        option_type: OptionType,
        expiry: Date,
        notional: Money,
        domestic_disc_id: &'static str,
        foreign_disc_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            base_currency,
            quote_currency,
            strike,
            option_type,
            exercise_style: ExerciseStyle::European,
            expiry,
            notional,
            settlement: SettlementType::Cash,
            domestic_disc_id,
            foreign_disc_id,
            implied_vol: None,
            attributes: Attributes::new(),
        }
    }

    /// Calculate option price using Garman-Kohlhagen model
    ///
    /// # Arguments
    /// * `spot` - Current FX rate (quote per base)
    /// * `r_d` - Domestic risk-free rate
    /// * `r_f` - Foreign risk-free rate
    /// * `sigma` - Implied volatility
    /// * `t` - Time to maturity in years
    pub fn garman_kohlhagen_price(
        &self,
        spot: F,
        r_d: F,
        r_f: F,
        sigma: F,
        t: F,
    ) -> finstack_core::Result<Money> {
        if t <= 0.0 {
            // Option expired
            let intrinsic = match self.option_type {
                OptionType::Call => (spot - self.strike).max(0.0),
                OptionType::Put => (self.strike - spot).max(0.0),
            };
            return Ok(Money::new(
                intrinsic * self.notional.amount(),
                self.quote_currency,
            ));
        }

        // Garman-Kohlhagen is Black-Scholes with foreign rate as dividend yield
        let d1 = black_scholes_common::d1(spot, self.strike, r_d, sigma, t, r_f);
        let d2 = black_scholes_common::d2(spot, self.strike, r_d, sigma, t, r_f);

        let price = match self.option_type {
            OptionType::Call => {
                spot * (-r_f * t).exp() * black_scholes_common::norm_cdf(d1)
                    - self.strike * (-r_d * t).exp() * black_scholes_common::norm_cdf(d2)
            }
            OptionType::Put => {
                self.strike * (-r_d * t).exp() * black_scholes_common::norm_cdf(-d2)
                    - spot * (-r_f * t).exp() * black_scholes_common::norm_cdf(-d1)
            }
        };

        Ok(Money::new(
            price * self.notional.amount(),
            self.quote_currency,
        ))
    }

    /// Calculate option delta (with respect to spot FX rate)
    pub fn delta(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return match self.option_type {
                OptionType::Call => {
                    if spot > self.strike {
                        1.0
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if spot < self.strike {
                        -1.0
                    } else {
                        0.0
                    }
                }
            };
        }

        let d1 = black_scholes_common::d1(spot, self.strike, r_d, sigma, t, r_f);
        let exp_rf_t = (-r_f * t).exp();

        match self.option_type {
            OptionType::Call => exp_rf_t * black_scholes_common::norm_cdf(d1),
            OptionType::Put => -exp_rf_t * black_scholes_common::norm_cdf(-d1),
        }
    }

    /// Calculate option gamma
    pub fn gamma(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 || sigma <= 0.0 {
            return 0.0;
        }

        let d1 = black_scholes_common::d1(spot, self.strike, r_d, sigma, t, r_f);
        let exp_rf_t = (-r_f * t).exp();

        exp_rf_t * black_scholes_common::norm_pdf(d1) / (spot * sigma * t.sqrt())
    }

    /// Calculate option vega
    pub fn vega(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let d1 = black_scholes_common::d1(spot, self.strike, r_d, sigma, t, r_f);
        let exp_rf_t = (-r_f * t).exp();

        spot * exp_rf_t * black_scholes_common::norm_pdf(d1) * t.sqrt() / 100.0 // Divide by 100 for 1% vega
    }

    /// Calculate option theta
    pub fn theta(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let d1 = black_scholes_common::d1(spot, self.strike, r_d, sigma, t, r_f);
        let d2 = black_scholes_common::d2(spot, self.strike, r_d, sigma, t, r_f);
        let sqrt_t = t.sqrt();

        match self.option_type {
            OptionType::Call => {
                let term1 = -spot * black_scholes_common::norm_pdf(d1) * sigma * (-r_f * t).exp()
                    / (2.0 * sqrt_t);
                let term2 = r_f * spot * black_scholes_common::norm_cdf(d1) * (-r_f * t).exp();
                let term3 =
                    -r_d * self.strike * (-r_d * t).exp() * black_scholes_common::norm_cdf(d2);
                (term1 + term2 + term3) / 365.0 // Daily theta
            }
            OptionType::Put => {
                let term1 = -spot * black_scholes_common::norm_pdf(d1) * sigma * (-r_f * t).exp()
                    / (2.0 * sqrt_t);
                let term2 = -r_f * spot * black_scholes_common::norm_cdf(-d1) * (-r_f * t).exp();
                let term3 =
                    r_d * self.strike * (-r_d * t).exp() * black_scholes_common::norm_cdf(-d2);
                (term1 + term2 + term3) / 365.0 // Daily theta
            }
        }
    }

    /// Calculate option rho (domestic rate sensitivity)
    pub fn rho_domestic(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let d2 = black_scholes_common::d2(spot, self.strike, r_d, sigma, t, r_f);

        match self.option_type {
            OptionType::Call => {
                self.strike * t * (-r_d * t).exp() * black_scholes_common::norm_cdf(d2) / 100.0
            }
            OptionType::Put => {
                -self.strike * t * (-r_d * t).exp() * black_scholes_common::norm_cdf(-d2) / 100.0
            }
        }
    }

    /// Calculate option rho (foreign rate sensitivity)
    pub fn rho_foreign(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let d1 = black_scholes_common::d1(spot, self.strike, r_d, sigma, t, r_f);

        match self.option_type {
            OptionType::Call => {
                -spot * t * (-r_f * t).exp() * black_scholes_common::norm_cdf(d1) / 100.0
            }
            OptionType::Put => {
                spot * t * (-r_f * t).exp() * black_scholes_common::norm_cdf(-d1) / 100.0
            }
        }
    }
}

impl Priceable for FxOption {
    /// Compute the present value of the option
    fn value(&self, curves: &CurveSet, _as_of: Date) -> finstack_core::Result<Money> {
        // Get market data
        let _domestic_disc = curves.discount(self.domestic_disc_id)?;
        let _foreign_disc = curves.discount(self.foreign_disc_id)?;

        // Get FX spot from market context (would need to be extended)
        // For now, return error as we need spot FX rate
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
            Arc::new(Instrument::FxOption(self.clone())),
            Arc::new(curves.clone()),
            as_of,
            base_value,
        );

        crate::pricing::build_with_metrics(
            Instrument::FxOption(self.clone()),
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
impl_attributable!(FxOption);

impl From<FxOption> for crate::instruments::Instrument {
    fn from(value: FxOption) -> Self {
        crate::instruments::Instrument::FxOption(value)
    }
}

impl std::convert::TryFrom<crate::instruments::Instrument> for FxOption {
    type Error = finstack_core::Error;

    fn try_from(value: crate::instruments::Instrument) -> finstack_core::Result<Self> {
        match value {
            crate::instruments::Instrument::FxOption(v) => Ok(v),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_fx_option_creation() {
        let notional = Money::new(1_000_000.0, Currency::EUR);
        let expiry = Date::from_calendar_date(2025, Month::December, 31).unwrap();

        let option = FxOption::new(
            "EURUSD_CALL_1.20",
            Currency::EUR,
            Currency::USD,
            1.20,
            OptionType::Call,
            expiry,
            notional,
            "USD-OIS",
            "EUR-OIS",
        );

        assert_eq!(option.id, "EURUSD_CALL_1.20");
        assert_eq!(option.base_currency, Currency::EUR);
        assert_eq!(option.quote_currency, Currency::USD);
        assert_eq!(option.strike, 1.20);
    }

    #[test]
    fn test_garman_kohlhagen_call() {
        let notional = Money::new(1_000_000.0, Currency::EUR);
        let expiry = Date::from_calendar_date(2025, Month::December, 31).unwrap();

        let option = FxOption::new(
            "CALL",
            Currency::EUR,
            Currency::USD,
            1.20,
            OptionType::Call,
            expiry,
            notional,
            "USD-OIS",
            "EUR-OIS",
        );

        // Test parameters
        let spot = 1.25; // EUR/USD
        let r_d = 0.05; // USD rate
        let r_f = 0.03; // EUR rate
        let sigma = 0.10;
        let t = 1.0;

        let price = option
            .garman_kohlhagen_price(spot, r_d, r_f, sigma, t)
            .unwrap();

        // Call should have positive value when spot > strike
        assert!(price.amount() > 0.0);
        assert_eq!(price.currency(), Currency::USD);

        // Test Greeks
        let delta = option.delta(spot, r_d, r_f, sigma, t);
        assert!(delta > 0.0 && delta < 1.0);

        let gamma = option.gamma(spot, r_d, r_f, sigma, t);
        assert!(gamma > 0.0);
    }

    #[test]
    fn test_garman_kohlhagen_put() {
        let notional = Money::new(1_000_000.0, Currency::EUR);
        let expiry = Date::from_calendar_date(2025, Month::December, 31).unwrap();

        let option = FxOption::new(
            "PUT",
            Currency::EUR,
            Currency::USD,
            1.20,
            OptionType::Put,
            expiry,
            notional,
            "USD-OIS",
            "EUR-OIS",
        );

        // Test parameters
        let spot = 1.15; // EUR/USD
        let r_d = 0.05; // USD rate
        let r_f = 0.03; // EUR rate
        let sigma = 0.10;
        let t = 1.0;

        let price = option
            .garman_kohlhagen_price(spot, r_d, r_f, sigma, t)
            .unwrap();

        // Put should have positive value when strike > spot
        assert!(price.amount() > 0.0);
        assert_eq!(price.currency(), Currency::USD);

        // Test Greeks
        let delta = option.delta(spot, r_d, r_f, sigma, t);
        assert!(delta < 0.0 && delta > -1.0);
    }
}
