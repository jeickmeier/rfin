//! Risk factor extraction for VaR calculations.
//!
//! This module defines the types of risk factors that drive VaR calculations
//! and provides utilities to extract them from instruments based on their
//! market data dependencies.

use crate::instruments::common::traits::{CurveDependencies, Instrument};
use crate::metrics::sensitivities::dv01::standard_ir_dv01_buckets;
use finstack_core::market_data::MarketContext;
use finstack_core::types::CurveId;
use finstack_core::Result;

/// Risk factor categories for VaR calculation.
///
/// Each risk factor represents a market variable that can shift and impact
/// portfolio valuations. Risk factors are bucketed at standard tenors/strikes
/// to enable historical simulation.
#[derive(Clone, Debug, PartialEq)]
pub enum RiskFactorType {
    /// Discount curve rate at a specific tenor (in years).
    DiscountRate {
        /// Curve identifier
        curve_id: CurveId,
        /// Tenor in years (e.g., 1.0, 5.0, 10.0)
        tenor_years: f64,
    },

    /// Forward curve rate at a specific tenor (in years).
    ForwardRate {
        /// Curve identifier
        curve_id: CurveId,
        /// Tenor in years
        tenor_years: f64,
    },

    /// Credit spread (hazard rate) at a specific tenor (in years).
    CreditSpread {
        /// Curve identifier
        curve_id: CurveId,
        /// Tenor in years
        tenor_years: f64,
    },

    /// Equity spot price.
    EquitySpot {
        /// Equity ticker or identifier
        ticker: String,
    },

    /// Implied volatility at specific expiry and strike.
    ImpliedVol {
        /// Volatility surface identifier
        surface_id: CurveId,
        /// Expiry in years
        expiry_years: f64,
        /// Strike level (absolute or moneyness)
        strike: f64,
    },
}

impl RiskFactorType {
    /// Get a human-readable label for this risk factor.
    pub fn label(&self) -> String {
        match self {
            Self::DiscountRate {
                curve_id,
                tenor_years,
            } => format!("{}::disc::{:.1}y", curve_id.as_str(), tenor_years),
            Self::ForwardRate {
                curve_id,
                tenor_years,
            } => format!("{}::fwd::{:.1}y", curve_id.as_str(), tenor_years),
            Self::CreditSpread {
                curve_id,
                tenor_years,
            } => format!("{}::credit::{:.1}y", curve_id.as_str(), tenor_years),
            Self::EquitySpot { ticker } => format!("{}::spot", ticker),
            Self::ImpliedVol {
                surface_id,
                expiry_years,
                strike,
            } => format!(
                "{}::vol::{:.1}y::{}",
                surface_id.as_str(),
                expiry_years,
                strike
            ),
        }
    }
}

/// Extract risk factors from an instrument's market data dependencies.
///
/// This function inspects the instrument's curve dependencies and extracts
/// risk factors at standard tenor buckets. The risk factors can then be used
/// to apply historical market shifts for VaR calculation.
///
/// # Arguments
///
/// * `instrument` - The instrument to analyze
/// * `market` - Current market context (used to verify curve existence)
///
/// # Returns
///
/// Vector of risk factors that affect this instrument's valuation
///
/// # Examples
///
/// ```ignore
/// use finstack_valuations::instruments::Bond;
/// use finstack_valuations::metrics::risk::extract_risk_factors;
///
/// let bond = Bond::fixed(...);
/// let market = MarketContext::new()...;
/// let factors = extract_risk_factors(&bond, &market)?;
/// // factors will include discount curve rates at standard tenors
/// ```
pub fn extract_risk_factors<I>(
    instrument: &I,
    market: &MarketContext,
) -> Result<Vec<RiskFactorType>>
where
    I: Instrument + CurveDependencies,
{
    let mut factors = Vec::new();

    // Get instrument's curve dependencies
    let deps = instrument.curve_dependencies();

    // Standard tenors for IR/credit curve factors
    let standard_tenors = standard_ir_dv01_buckets();

    // Extract discount curve factors
    for curve_id in &deps.discount_curves {
        // Verify curve exists in market
        if market.get_discount_ref(curve_id.as_str()).is_ok() {
            for &tenor_years in &standard_tenors {
                factors.push(RiskFactorType::DiscountRate {
                    curve_id: curve_id.clone(),
                    tenor_years,
                });
            }
        }
    }

    // Extract forward curve factors
    for curve_id in &deps.forward_curves {
        if market.get_forward_ref(curve_id.as_str()).is_ok() {
            for &tenor_years in &standard_tenors {
                factors.push(RiskFactorType::ForwardRate {
                    curve_id: curve_id.clone(),
                    tenor_years,
                });
            }
        }
    }

    // Extract credit spread factors
    for curve_id in &deps.credit_curves {
        if market.get_hazard_ref(curve_id.as_str()).is_ok() {
            for &tenor_years in &standard_tenors {
                factors.push(RiskFactorType::CreditSpread {
                    curve_id: curve_id.clone(),
                    tenor_years,
                });
            }
        }
    }

    // TODO: Extract equity spot factors (requires equity price lookup in market)
    // TODO: Extract volatility surface factors (requires vol surface analysis)

    Ok(factors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::money::Money;
    use finstack_core::types::Currency;
    use time::macros::date;

    #[test]
    fn test_risk_factor_label() {
        let factor = RiskFactorType::DiscountRate {
            curve_id: CurveId::from("USD-OIS"),
            tenor_years: 5.0,
        };
        assert_eq!(factor.label(), "USD-OIS::disc::5.0y");

        let factor = RiskFactorType::CreditSpread {
            curve_id: CurveId::from("AAPL"),
            tenor_years: 10.0,
        };
        assert_eq!(factor.label(), "AAPL::credit::10.0y");

        let factor = RiskFactorType::ForwardRate {
            curve_id: CurveId::from("USD-LIBOR-3M"),
            tenor_years: 2.0,
        };
        assert_eq!(factor.label(), "USD-LIBOR-3M::fwd::2.0y");
    }

    #[test]
    fn test_extract_discount_factors_from_bond() -> Result<()> {
        use crate::instruments::Bond;

        let as_of = date!(2024 - 01 - 01);

        // Use Bond::fixed factory method
        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(100_000.0, Currency::USD),
            0.05,
            as_of,
            date!(2029 - 01 - 01),
            "USD-OIS",
        );

        // Create market with discount curve
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots(vec![(0.0, 1.0), (1.0, 0.96), (5.0, 0.85), (10.0, 0.70)])
            .build()?;

        let market = MarketContext::new().insert_discount(curve);

        // Extract risk factors
        let factors = extract_risk_factors(&bond, &market)?;

        // Should have discount rate factors at standard tenors
        assert!(!factors.is_empty(), "Should extract risk factors");

        // Verify we have discount rate factors
        let discount_factors: Vec<_> = factors
            .iter()
            .filter_map(|f| match f {
                RiskFactorType::DiscountRate {
                    curve_id,
                    tenor_years,
                } => Some((curve_id.as_str(), *tenor_years)),
                _ => None,
            })
            .collect();

        assert!(
            !discount_factors.is_empty(),
            "Should have discount rate factors"
        );
        assert!(
            discount_factors.iter().all(|(cid, _)| *cid == "USD-OIS"),
            "All factors should be for USD-OIS curve"
        );

        // Verify we're using standard tenors
        let standard_tenors = standard_ir_dv01_buckets();
        for (_, tenor) in &discount_factors {
            assert!(
                standard_tenors.contains(tenor),
                "Tenor {} should be in standard bucket list",
                tenor
            );
        }

        Ok(())
    }

    #[test]
    fn test_extract_factors_empty_market() -> Result<()> {
        use crate::instruments::Bond;

        let as_of = date!(2024 - 01 - 01);
        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(100_000.0, Currency::USD),
            0.05,
            as_of,
            date!(2029 - 01 - 01),
            "USD-OIS",
        );

        // Empty market - curve exists in instrument but not in market
        let market = MarketContext::new();

        let factors = extract_risk_factors(&bond, &market)?;

        // Should return empty vector when curves don't exist
        assert!(
            factors.is_empty(),
            "Should have no factors for empty market"
        );

        Ok(())
    }
}
