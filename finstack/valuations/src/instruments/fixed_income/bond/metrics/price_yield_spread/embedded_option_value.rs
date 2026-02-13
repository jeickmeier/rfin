//! Embedded option value calculator for callable/putable bonds.
//!
//! Computes the theoretical value of embedded call or put options by pricing
//! the bond twice using a short-rate tree:
//! 1. With call/put constraints → P_embedded
//! 2. Without call/put constraints → P_straight
//!
//! The embedded option value is the difference between these prices.
//!
//! # Option Value Decomposition
#![allow(dead_code)] // Public API items may be used by external bindings
//!
//! ## Callable Bonds (Issuer Owns the Call)
//!
//! ```text
//! P_callable = P_straight - V_call
//! V_call = P_straight - P_callable  (positive)
//! ```
//!
//! The call option has positive value to the issuer, reducing the price
//! the investor pays.
//!
//! ## Putable Bonds (Investor Owns the Put)
//!
//! ```text
//! P_putable = P_straight + V_put
//! V_put = P_putable - P_straight  (positive)
//! ```
//!
//! The put option has positive value to the investor, increasing the price.
//!
//! ## Bonds with Both Options
//!
//! For bonds with both calls and puts, the metric returns the **net** option
//! value from the investor's perspective:
//!
//! ```text
//! V_net = P_embedded - P_straight
//! ```
//!
//! - Positive: Put value dominates (benefits investor)
//! - Negative: Call value dominates (benefits issuer)
//!
//! # Examples
//!
//! ```text
//! use finstack_valuations::instruments::fixed_income::bond::Bond;
//! use finstack_valuations::metrics::{MetricRegistry, MetricId};
//!
//! # let bond = Bond::example();
//! // Register metrics and compute
//! // V_call will be positive for callable bonds
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::instruments::common_impl::models::{
    short_rate_keys, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
};
use crate::instruments::fixed_income::bond::pricing::tree_engine::{
    bond_tree_config, BondValuator,
};
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::DayCountCtx;

/// Calculates the embedded option value for callable/putable bonds.
///
/// Uses tree-based pricing to compute the difference between:
/// - The option-embedded bond price (with call/put exercise decisions)
/// - The straight bond price (without embedded options)
///
/// # Returns
///
/// - For **callable bonds**: Positive value (call reduces holder value)
/// - For **putable bonds**: Positive value (put increases holder value)
/// - For **bonds with both**: Net option value from investor perspective
/// - For **straight bonds**: Zero (no embedded options)
///
/// The value is returned in **currency units** (same as bond notional).
///
/// # Dependencies
///
/// None - this is a standalone metric using tree pricing.
///
/// # Examples
///
/// ```text
/// use finstack_valuations::instruments::fixed_income::bond::metrics::price_yield_spread::EmbeddedOptionValueCalculator;
/// use finstack_valuations::metrics::MetricCalculator;
///
/// let calculator = EmbeddedOptionValueCalculator::new();
/// // Use via MetricRegistry for proper context management
/// ```
#[derive(Debug, Clone, Default)]
pub struct EmbeddedOptionValueCalculator {
    /// Number of tree steps (default: 100)
    tree_steps: usize,
    /// Short rate volatility (default: 1% = 100 bps normal vol for Ho-Lee)
    volatility: f64,
}

impl EmbeddedOptionValueCalculator {
    /// Create a calculator with default settings.
    ///
    /// Uses 100 tree steps and 1% (100 bps) normal volatility.
    pub fn new() -> Self {
        Self {
            tree_steps: 100,
            volatility: 0.01,
        }
    }

    /// Create a calculator with custom tree configuration.
    ///
    /// # Arguments
    ///
    /// * `tree_steps` - Number of steps in the short-rate tree (50-200 typical)
    /// * `volatility` - Short rate volatility (normal vol in decimal, e.g., 0.01 = 100 bps)
    ///
    /// # Examples
    ///
    /// ```text
    /// use finstack_valuations::instruments::fixed_income::bond::metrics::price_yield_spread::EmbeddedOptionValueCalculator;
    ///
    /// // High precision with calibrated volatility
    /// let calc = EmbeddedOptionValueCalculator::with_config(200, 0.012);
    /// ```
    pub fn with_config(tree_steps: usize, volatility: f64) -> Self {
        Self {
            tree_steps,
            volatility,
        }
    }
}

impl MetricCalculator for EmbeddedOptionValueCalculator {
    fn dependencies(&self) -> &[MetricId] {
        // No dependencies - standalone metric
        &[]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // If bond has no embedded options, return 0
        let has_options = bond
            .call_put
            .as_ref()
            .map(|cp| cp.has_options())
            .unwrap_or(false);

        if !has_options {
            return Ok(0.0);
        }

        let market = context.curves.as_ref();
        let as_of = context.as_of;

        // Get discount curve and compute time to maturity
        let discount_curve = market.get_discount(&bond.discount_curve_id)?;
        let dc = discount_curve.day_count();
        let time_to_maturity = dc.year_fraction(as_of, bond.maturity, DayCountCtx::default())?;

        if time_to_maturity <= 0.0 {
            return Ok(0.0);
        }

        // Use centralized tree config from bond.pricing_overrides,
        // falling back to calculator's defaults if bond has no overrides
        let bond_config = bond_tree_config(bond);
        let tree_config = ShortRateTreeConfig {
            steps: if bond.pricing_overrides.tree_steps.is_some() {
                bond_config.tree_steps
            } else {
                self.tree_steps
            },
            volatility: if bond.pricing_overrides.tree_volatility.is_some() {
                bond_config.volatility
            } else {
                self.volatility
            },
            ..Default::default()
        };
        let mut tree = ShortRateTree::new(tree_config);
        tree.calibrate(discount_curve.as_ref(), time_to_maturity)?;

        // Price 1: Bond WITH embedded options (call/put constraints applied)
        let valuator_with_options = BondValuator::new(
            bond.clone(),
            market,
            as_of,
            time_to_maturity,
            self.tree_steps,
        )?;

        let mut vars = StateVariables::default();
        vars.insert(short_rate_keys::OAS, 0.0);

        let price_with_options = tree.price(
            vars.clone(),
            time_to_maturity,
            market,
            &valuator_with_options,
        )?;

        // Price 2: Bond WITHOUT embedded options (straight bond)
        // Create a copy of the bond with call_put stripped out
        let mut straight_bond = bond.clone();
        straight_bond.call_put = None;

        let valuator_straight = BondValuator::new(
            straight_bond,
            market,
            as_of,
            time_to_maturity,
            self.tree_steps,
        )?;

        let price_straight = tree.price(vars, time_to_maturity, market, &valuator_straight)?;

        // Compute embedded option value:
        // For callable: V_call = P_straight - P_with_options (positive)
        // For putable:  V_put = P_with_options - P_straight (positive)
        //
        // We return the ABSOLUTE option value (always positive) with sign convention:
        // - Positive = option value exists (call or put)
        //
        // The decomposition is:
        // - Callable: P_callable = P_straight - V_call → V_call = P_straight - P_callable
        // - Putable:  P_putable = P_straight + V_put → V_put = P_putable - P_straight
        //
        // We determine direction based on which options exist
        // Safety: has_options check at top ensures call_put is Some
        let call_put = bond.call_put.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "call_put schedule".to_string(),
            })
        })?;
        let has_calls = !call_put.calls.is_empty();
        let has_puts = !call_put.puts.is_empty();

        let option_value = match (has_calls, has_puts) {
            (true, false) => {
                // Callable only: V_call = P_straight - P_callable (positive)
                price_straight - price_with_options
            }
            (false, true) => {
                // Putable only: V_put = P_putable - P_straight (positive)
                price_with_options - price_straight
            }
            (true, true) => {
                // Both: return net from holder perspective
                // Positive = puts dominate, Negative = calls dominate
                price_with_options - price_straight
            }
            (false, false) => {
                // No options (shouldn't reach here due to early return)
                0.0
            }
        };

        Ok(option_value)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Instrument;
    use crate::instruments::fixed_income::bond::CashflowSpec;
    #[cfg(feature = "slow")]
    use crate::instruments::fixed_income::bond::{CallPut, CallPutSchedule};
    use crate::instruments::PricingOverrides;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use std::sync::Arc;
    use time::Month;

    fn create_test_market() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.85), (10.0, 0.70)])
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("Valid curve");
        MarketContext::new().insert_discount(discount_curve)
    }

    #[cfg(feature = "slow")]
    fn create_callable_bond() -> Bond {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
        let call_date = Date::from_calendar_date(2027, Month::January, 1).expect("Valid date");

        let mut call_put = CallPutSchedule::default();
        call_put.calls.push(CallPut {
            date: call_date,
            price_pct_of_par: 100.0,
            end_date: None,
            make_whole: None,
        });

        Bond::builder()
            .id("CALLABLE_BOND".into())
            .notional(Money::new(1000.0, finstack_core::currency::Currency::USD))
            .issue_date(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                finstack_core::dates::Tenor::semi_annual(),
                finstack_core::dates::DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .call_put_opt(Some(call_put))
            .custom_cashflows_opt(None)
            .attributes(Default::default())
            .settlement_days_opt(Some(2))
            .ex_coupon_days_opt(Some(0))
            .build()
            .expect("Valid bond")
    }

    #[cfg(feature = "slow")]
    fn create_putable_bond() -> Bond {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
        let put_date = Date::from_calendar_date(2027, Month::January, 1).expect("Valid date");

        let mut call_put = CallPutSchedule::default();
        call_put.puts.push(CallPut {
            date: put_date,
            price_pct_of_par: 100.0,
            end_date: None,
            make_whole: None,
        });

        Bond::builder()
            .id("PUTABLE_BOND".into())
            .notional(Money::new(1000.0, finstack_core::currency::Currency::USD))
            .issue_date(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                finstack_core::dates::Tenor::semi_annual(),
                finstack_core::dates::DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .call_put_opt(Some(call_put))
            .custom_cashflows_opt(None)
            .attributes(Default::default())
            .settlement_days_opt(Some(2))
            .ex_coupon_days_opt(Some(0))
            .build()
            .expect("Valid bond")
    }

    fn create_straight_bond() -> Bond {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");

        Bond::builder()
            .id("STRAIGHT_BOND".into())
            .notional(Money::new(1000.0, finstack_core::currency::Currency::USD))
            .issue_date(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                finstack_core::dates::Tenor::semi_annual(),
                finstack_core::dates::DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .call_put_opt(None)
            .custom_cashflows_opt(None)
            .attributes(Default::default())
            .settlement_days_opt(Some(2))
            .ex_coupon_days_opt(Some(0))
            .build()
            .expect("Valid bond")
    }

    #[test]
    fn test_straight_bond_returns_zero() {
        let bond = create_straight_bond();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");

        let calc = EmbeddedOptionValueCalculator::new();
        let base_value = bond.value(&market, as_of).expect("Should price");

        let mut context = MetricContext::new(
            Arc::new(bond),
            Arc::new(market),
            as_of,
            base_value,
            MetricContext::default_config(),
        );
        let option_value = calc.calculate(&mut context).expect("Should calculate");

        assert!(
            option_value.abs() < 1e-10,
            "Straight bond should have zero option value, got {}",
            option_value
        );
    }

    #[test]
    #[cfg(feature = "slow")]
    fn test_callable_bond_positive_option_value() {
        let bond = create_callable_bond();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");

        let calc = EmbeddedOptionValueCalculator::new();
        let base_value = bond.value(&market, as_of).expect("Should price");

        let mut context = MetricContext::new(
            Arc::new(bond),
            Arc::new(market),
            as_of,
            base_value,
            MetricContext::default_config(),
        );
        let option_value = calc.calculate(&mut context).expect("Should calculate");

        assert!(
            option_value > 0.0,
            "Callable bond should have positive call option value, got {}",
            option_value
        );
    }

    #[test]
    #[cfg(feature = "slow")]
    fn test_putable_bond_positive_option_value() {
        let bond = create_putable_bond();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");

        let calc = EmbeddedOptionValueCalculator::new();
        let base_value = bond.value(&market, as_of).expect("Should price");

        let mut context = MetricContext::new(
            Arc::new(bond),
            Arc::new(market),
            as_of,
            base_value,
            MetricContext::default_config(),
        );
        let option_value = calc.calculate(&mut context).expect("Should calculate");

        assert!(
            option_value > 0.0,
            "Putable bond should have positive put option value, got {}",
            option_value
        );
    }
}
