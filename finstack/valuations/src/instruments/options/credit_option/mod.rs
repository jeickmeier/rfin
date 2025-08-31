//! Credit option instrument implementation for options on credit default swaps.

pub mod metrics;

use crate::instruments::traits::Attributes;
use crate::metrics::MetricId;
use finstack_core::money::Money;
use finstack_core::F;

use finstack_core::dates::{Date, DayCount};

use super::models;
use super::{ExerciseStyle, OptionType, SettlementType};

/// Credit option instrument (option on CDS spread)
#[derive(Clone, Debug)]
pub struct CreditOption {
    /// Unique instrument identifier
    pub id: String,
    /// Reference entity (underlying credit)
    pub reference_entity: String,
    /// Strike spread in basis points
    pub strike_spread_bp: F,
    /// Option type (Call = right to buy protection, Put = right to sell protection)
    pub option_type: OptionType,
    /// Exercise style
    pub exercise_style: ExerciseStyle,
    /// Option expiry date
    pub expiry: Date,
    /// Underlying CDS maturity date
    pub cds_maturity: Date,
    /// Day count convention for time calculations
    pub day_count: finstack_core::dates::DayCount,
    /// Notional amount
    pub notional: Money,
    /// Settlement type
    pub settlement: SettlementType,
    /// Recovery rate assumption
    pub recovery_rate: F,
    /// Discount curve identifier
    pub disc_id: &'static str,
    /// Credit curve identifier
    pub credit_id: &'static str,
    /// Volatility surface identifier
    pub vol_id: &'static str,
    /// Implied volatility of credit spread (if known, overrides vol surface)
    pub implied_vol: Option<F>,
    /// Additional attributes
    pub attributes: Attributes,
}

impl CreditOption {
    /// Create a new credit option
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        reference_entity: impl Into<String>,
        strike_spread_bp: F,
        option_type: OptionType,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
        recovery_rate: F,
        disc_id: &'static str,
        credit_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            reference_entity: reference_entity.into(),
            strike_spread_bp,
            option_type,
            exercise_style: ExerciseStyle::European,
            expiry,
            cds_maturity,
            day_count: DayCount::Act360,
            notional,
            settlement: SettlementType::Cash,
            recovery_rate,
            disc_id,
            credit_id,
            vol_id,
            implied_vol: None,
            attributes: Attributes::new(),
        }
    }

    /// Calculate option price using modified Black model for credit spreads
    ///
    /// # Arguments
    /// * `forward_spread_bp` - Forward CDS spread in basis points
    /// * `df` - Discount factor to option expiry
    /// * `risky_annuity` - Risky annuity of underlying CDS
    /// * `sigma` - Implied volatility of credit spread
    /// * `t` - Time to option expiry in years
    pub fn credit_option_price(
        &self,
        forward_spread_bp: F,
        df: F,
        risky_annuity: F,
        sigma: F,
        t: F,
    ) -> finstack_core::Result<Money> {
        if t <= 0.0 {
            // Option expired
            let intrinsic = match self.option_type {
                OptionType::Call => (forward_spread_bp - self.strike_spread_bp).max(0.0),
                OptionType::Put => (self.strike_spread_bp - forward_spread_bp).max(0.0),
            };
            return Ok(Money::new(
                intrinsic * risky_annuity * self.notional.amount() / 10000.0,
                self.notional.currency(),
            ));
        }

        // Use Black's formula with spreads
        let forward = forward_spread_bp / 10000.0; // Convert to decimal
        let strike = self.strike_spread_bp / 10000.0; // Convert to decimal

        if forward <= 0.0 || strike <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let d1 = ((forward / strike).ln() + 0.5 * sigma * sigma * t) / (sigma * t.sqrt());
        let d2 = d1 - sigma * t.sqrt();

        let option_value = match self.option_type {
            OptionType::Call => {
                // Call option on CDS spread (right to buy protection at strike spread)
                df * risky_annuity
                    * self.notional.amount()
                    * (forward * models::norm_cdf(d1) - strike * models::norm_cdf(d2))
            }
            OptionType::Put => {
                // Put option on CDS spread (right to sell protection at strike spread)
                df * risky_annuity
                    * self.notional.amount()
                    * (strike * models::norm_cdf(-d2) - forward * models::norm_cdf(-d1))
            }
        };

        Ok(Money::new(option_value, self.notional.currency()))
    }

    /// Calculate option delta (sensitivity to credit spread)
    pub fn delta(&self, forward_spread_bp: F, sigma: F, t: F) -> F {
        if t <= 0.0 || sigma <= 0.0 {
            return match self.option_type {
                OptionType::Call => {
                    if forward_spread_bp > self.strike_spread_bp {
                        1.0
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if forward_spread_bp < self.strike_spread_bp {
                        -1.0
                    } else {
                        0.0
                    }
                }
            };
        }

        let forward = forward_spread_bp / 10000.0;
        let strike = self.strike_spread_bp / 10000.0;

        if forward <= 0.0 || strike <= 0.0 {
            return 0.0;
        }

        let d1 = ((forward / strike).ln() + 0.5 * sigma * sigma * t) / (sigma * t.sqrt());

        match self.option_type {
            OptionType::Call => models::norm_cdf(d1),
            OptionType::Put => -models::norm_cdf(-d1),
        }
    }

    /// Calculate option gamma
    pub fn gamma(&self, forward_spread_bp: F, sigma: F, t: F) -> F {
        if t <= 0.0 || sigma <= 0.0 {
            return 0.0;
        }

        let forward = forward_spread_bp / 10000.0;
        let strike = self.strike_spread_bp / 10000.0;

        if forward <= 0.0 || strike <= 0.0 {
            return 0.0;
        }

        let d1 = ((forward / strike).ln() + 0.5 * sigma * sigma * t) / (sigma * t.sqrt());

        models::norm_pdf(d1) / (forward * 10000.0 * sigma * t.sqrt())
    }

    /// Calculate option vega (sensitivity to credit spread volatility)
    pub fn vega(&self, forward_spread_bp: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let forward = forward_spread_bp / 10000.0;
        let strike = self.strike_spread_bp / 10000.0;

        if forward <= 0.0 || strike <= 0.0 {
            return 0.0;
        }

        let d1 = if sigma > 0.0 {
            ((forward / strike).ln() + 0.5 * sigma * sigma * t) / (sigma * t.sqrt())
        } else {
            0.0
        };

        forward * 10000.0 * models::norm_pdf(d1) * t.sqrt() / 100.0
        // Per 1% vega
    }

    /// Calculate option theta (time decay)
    pub fn theta(&self, forward_spread_bp: F, r: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }

        let forward = forward_spread_bp / 10000.0;
        let strike = self.strike_spread_bp / 10000.0;

        if forward <= 0.0 || strike <= 0.0 {
            return 0.0;
        }

        let d1 = ((forward / strike).ln() + 0.5 * sigma * sigma * t) / (sigma * t.sqrt());
        let d2 = d1 - sigma * t.sqrt();
        let sqrt_t = t.sqrt();

        match self.option_type {
            OptionType::Call => {
                let term1 = -forward * models::norm_pdf(d1) * sigma / (2.0 * sqrt_t);
                let term2 = -r * strike * (-r * t).exp() * models::norm_cdf(d2);
                (term1 + term2) * 10000.0 / 365.0 // Daily theta in bp
            }
            OptionType::Put => {
                let term1 = -forward * models::norm_pdf(d1) * sigma / (2.0 * sqrt_t);
                let term2 = r * strike * (-r * t).exp() * models::norm_cdf(-d2);
                (term1 + term2) * 10000.0 / 365.0 // Daily theta in bp
            }
        }
    }
}

impl_instrument!(
    CreditOption,
    "CreditOption",
    pv = |s, curves, as_of| {
        // Calculate time to expiry in years
        let time_to_expiry = s.day_count.year_fraction(as_of, s.expiry)?;
        
        // Get market curves
        let disc_curve = curves.discount(s.disc_id)?;
        let credit_curve = curves.credit(s.credit_id)?;
        
        // Calculate risky annuity (RPV01) of the underlying CDS
        // This is a simplified calculation - in practice would need full CDS schedule
        let cds_tenor = s.day_count.year_fraction(s.expiry, s.cds_maturity)?;
        let mut risky_annuity = 0.0;
        
        // Approximate quarterly payments for CDS premium leg
        let num_payments = (cds_tenor * 4.0).ceil() as usize;
        for i in 1..=num_payments {
            let t = cds_tenor * (i as f64) / (num_payments as f64);
            let df = disc_curve.df(time_to_expiry + t);
            let survival = credit_curve.survival_probability(
                as_of.checked_add(time::Duration::days(((time_to_expiry + t) * 365.25) as i64)).unwrap_or(as_of)
            );
            risky_annuity += 0.25 * df * survival; // 0.25 = quarterly accrual
        }
        
        // Calculate forward CDS spread (simplified)
        let current_tenor = s.day_count.year_fraction(as_of, s.cds_maturity)?;
        let forward_spread_bp = if current_tenor > 0.0 {
            credit_curve.spread_bp(current_tenor)
        } else {
            s.strike_spread_bp // Fallback if CDS has expired
        };
        
        // Get discount factor to option expiry
        let df_expiry = disc_curve.df(time_to_expiry);
        
        // Get volatility (use implied_vol if set, otherwise fetch from surface)
        let sigma = if let Some(impl_vol) = s.implied_vol {
            impl_vol
        } else {
            let vol_surface = curves.vol_surface(s.vol_id)?;
            vol_surface.value_checked(time_to_expiry, s.strike_spread_bp)?
        };
        
        // Price using Black model on credit spreads
        s.credit_option_price(forward_spread_bp, df_expiry, risky_annuity, sigma, time_to_expiry)
    },
    metrics = |_s| vec![
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::Theta,
        MetricId::Rho
    ]
);

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_credit_option_creation() {
        let notional = Money::new(10_000_000.0, Currency::USD);
        let expiry = Date::from_calendar_date(2025, Month::June, 30).unwrap();
        let cds_maturity = Date::from_calendar_date(2030, Month::June, 30).unwrap();

        let option = CreditOption::new(
            "ABC_CDS_CALL_200",
            "ABC Corp",
            200.0, // 200bp strike
            OptionType::Call,
            expiry,
            cds_maturity,
            notional,
            0.4, // 40% recovery
            "USD-OIS",
            "ABC-SENIOR",
            "ABC-CDS-VOL",
        );

        assert_eq!(option.id, "ABC_CDS_CALL_200");
        assert_eq!(option.reference_entity, "ABC Corp");
        assert_eq!(option.strike_spread_bp, 200.0);
        assert_eq!(option.recovery_rate, 0.4);
    }

    #[test]
    fn test_credit_option_pricing() {
        let notional = Money::new(10_000_000.0, Currency::USD);
        let expiry = Date::from_calendar_date(2025, Month::June, 30).unwrap();
        let cds_maturity = Date::from_calendar_date(2030, Month::June, 30).unwrap();

        let option = CreditOption::new(
            "CALL",
            "XYZ Corp",
            150.0, // 150bp strike
            OptionType::Call,
            expiry,
            cds_maturity,
            notional,
            0.4,
            "USD-OIS",
            "XYZ-SENIOR",
            "XYZ-CDS-VOL",
        );

        // Test parameters
        let forward_spread_bp = 200.0; // 200bp forward spread
        let df = 0.98; // Discount factor
        let risky_annuity = 4.5; // Risky annuity
        let sigma = 0.30; // 30% credit spread volatility
        let t = 0.5; // 6 months to expiry

        let price = option
            .credit_option_price(forward_spread_bp, df, risky_annuity, sigma, t)
            .unwrap();

        // Call should have positive value when forward > strike
        assert!(price.amount() > 0.0);

        // Test Greeks
        let delta = option.delta(forward_spread_bp, sigma, t);
        assert!(delta > 0.0 && delta < 1.0);

        let gamma = option.gamma(forward_spread_bp, sigma, t);
        assert!(gamma > 0.0);
    }

    #[test]
    fn test_credit_put_option() {
        let notional = Money::new(10_000_000.0, Currency::USD);
        let expiry = Date::from_calendar_date(2025, Month::June, 30).unwrap();
        let cds_maturity = Date::from_calendar_date(2030, Month::June, 30).unwrap();

        let option = CreditOption::new(
            "PUT",
            "XYZ Corp",
            250.0, // 250bp strike
            OptionType::Put,
            expiry,
            cds_maturity,
            notional,
            0.4,
            "USD-OIS",
            "XYZ-SENIOR",
            "XYZ-CDS-VOL",
        );

        // Test parameters
        let forward_spread_bp = 200.0; // 200bp forward spread
        let df = 0.98;
        let risky_annuity = 4.5;
        let sigma = 0.30;
        let t = 0.5;

        let price = option
            .credit_option_price(forward_spread_bp, df, risky_annuity, sigma, t)
            .unwrap();

        // Put should have positive value when strike > forward
        assert!(price.amount() > 0.0);

        // Test Greeks
        let delta = option.delta(forward_spread_bp, sigma, t);
        assert!(delta < 0.0 && delta > -1.0);
    }
}
