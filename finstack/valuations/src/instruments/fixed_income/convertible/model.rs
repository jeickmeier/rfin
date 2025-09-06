//! Convertible bond pricing model using binomial/trinomial trees.
//!
//! Implements a hybrid debt-equity pricing model that:
//! 1. Uses CashflowBuilder to generate the bond's coupon schedule
//! 2. Applies tree-based pricing to capture the equity conversion option
//! 3. Handles call/put provisions and various conversion policies

use finstack_core::market_data::context::MarketContext;
use finstack_core::prelude::*;
use finstack_core::{Error, Result, F};
use std::collections::HashMap;

use crate::cashflow::builder::{cf, CashFlowSchedule};
use crate::cashflow::primitives::CFKind;
use crate::instruments::options::models::{
    single_factor_equity_state, BinomialTree, NodeState, TreeGreeks, TreeModel, TreeValuator,
    TrinomialTree,
};

use super::{ConversionPolicy, ConvertibleBond};

/// Tree model type selection for convertible bond pricing
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvertibleTreeType {
    /// Use binomial tree (CRR)
    Binomial(usize), // number of steps
    /// Use trinomial tree
    Trinomial(usize), // number of steps
}

impl Default for ConvertibleTreeType {
    fn default() -> Self {
        Self::Binomial(100) // Default to 100-step binomial
    }
}

/// Convertible bond valuator implementing the TreeValuator trait
pub struct ConvertibleBondValuator {
    /// Conversion ratio (shares per bond)
    conversion_ratio: F,
    /// Face value of the bond
    face_value: F,
    /// Coupon cashflows mapped to tree steps
    coupon_map: HashMap<usize, F>,
    /// Call prices mapped to tree steps
    call_map: HashMap<usize, F>,
    /// Put prices mapped to tree steps
    put_map: HashMap<usize, F>,
    /// Conversion policy
    conversion_policy: ConversionPolicy,
    /// Time steps for the tree (in years)
    time_steps: Vec<F>,
    /// Currency for consistency checks
    #[allow(dead_code)]
    currency: Currency,
    /// Base date for time calculations
    base_date: Date,
}

impl ConvertibleBondValuator {
    /// Create a new convertible bond valuator
    pub fn new(
        bond: &ConvertibleBond,
        cashflow_schedule: &CashFlowSchedule,
        time_to_maturity: F,
        steps: usize,
        base_date: Date,
    ) -> Result<Self> {
        // Calculate conversion ratio from conversion spec
        let conversion_ratio = if let Some(ratio) = bond.conversion.ratio {
            ratio
        } else if let Some(price) = bond.conversion.price {
            bond.notional.amount() / price
        } else {
            return Err(Error::Internal); // Must have either ratio or price
        };

        // Map cashflows to tree steps
        let dt = time_to_maturity / steps as F;
        let mut time_steps = Vec::with_capacity(steps + 1);

        for i in 0..=steps {
            time_steps.push(i as F * dt);
        }

        // Process coupon and principal cashflows using proper time mapping
        let mut coupon_map = HashMap::new();
        for cf in &cashflow_schedule.flows {
            if matches!(cf.kind, CFKind::Fixed | CFKind::Stub | CFKind::FloatReset) {
                // Calculate actual time from base date to cashflow date
                let cf_time = finstack_core::dates::DayCount::Act365F
                    .year_fraction(
                        base_date,
                        cf.date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);

                // Map to tree step with bounds checking
                let step_index = ((cf_time / time_to_maturity) * steps as F).round() as usize;
                let bounded_step = step_index.min(steps); // Bound within [0, steps]

                *coupon_map.entry(bounded_step).or_insert(0.0) += cf.amount.amount();
            }
        }

        // Map call/put schedules to tree steps
        let mut call_map = HashMap::new();
        let mut put_map = HashMap::new();

        if let Some(ref call_put) = bond.call_put {
            // Map call schedule
            for call in &call_put.calls {
                if call.date > base_date && call.date <= bond.maturity {
                    let time_frac = finstack_core::dates::DayCount::Act365F
                        .year_fraction(
                            base_date,
                            call.date,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0);
                    let step = ((time_frac / time_to_maturity) * steps as F).round() as usize;
                    let bounded_step = step.min(steps);
                    let call_price = bond.notional.amount() * (call.price_pct_of_par / 100.0);
                    call_map.insert(bounded_step, call_price);
                }
            }

            // Map put schedule
            for put in &call_put.puts {
                if put.date > base_date && put.date <= bond.maturity {
                    let time_frac = finstack_core::dates::DayCount::Act365F
                        .year_fraction(
                            base_date,
                            put.date,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0);
                    let step = ((time_frac / time_to_maturity) * steps as F).round() as usize;
                    let bounded_step = step.min(steps);
                    let put_price = bond.notional.amount() * (put.price_pct_of_par / 100.0);
                    put_map.insert(bounded_step, put_price);
                }
            }
        }

        Ok(Self {
            conversion_ratio,
            face_value: bond.notional.amount(),
            coupon_map,
            call_map,
            put_map,
            conversion_policy: bond.conversion.policy.clone(),
            time_steps,
            currency: bond.notional.currency(),
            base_date,
        })
    }

    /// Check if conversion is allowed at a given time step
    fn conversion_allowed(&self, step: usize) -> bool {
        let time = self.time_steps.get(step).copied().unwrap_or(0.0);

        match &self.conversion_policy {
            ConversionPolicy::Voluntary => true,
            ConversionPolicy::MandatoryOn(date) => {
                // Allow conversion only when time matches the mandatory conversion date
                let target_time = finstack_core::dates::DayCount::Act365F
                    .year_fraction(
                        self.base_date,
                        *date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                let tolerance = 1e-6; // Small tolerance for floating point comparison
                (time - target_time).abs() < tolerance
            }
            ConversionPolicy::Window { start, end } => {
                // Allow conversion when time falls within the window
                let start_time = finstack_core::dates::DayCount::Act365F
                    .year_fraction(
                        self.base_date,
                        *start,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                let end_time = finstack_core::dates::DayCount::Act365F
                    .year_fraction(
                        self.base_date,
                        *end,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                time >= start_time && time <= end_time
            }
            ConversionPolicy::UponEvent(_event) => {
                // For event-triggered conversion, would need metadata in NodeState
                // For now, conservatively disable unless explicitly handled
                false
            }
        }
    }

    /// Get call price at a given step (if callable)
    fn call_price_at_step(&self, step: usize) -> Option<F> {
        self.call_map.get(&step).copied()
    }

    /// Get put price at a given step (if puttable)
    fn put_price_at_step(&self, step: usize) -> Option<F> {
        self.put_map.get(&step).copied()
    }
}

impl TreeValuator for ConvertibleBondValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<F> {
        let spot = state.spot().ok_or(Error::Internal)?;

        // At maturity, choose between conversion and redemption
        let conversion_value = spot * self.conversion_ratio;
        let redemption_value = self.face_value;

        // Add any final coupon payment
        let final_coupon = self.coupon_map.get(&state.step).copied().unwrap_or(0.0);

        Ok(conversion_value.max(redemption_value) + final_coupon)
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: F) -> Result<F> {
        let spot = state.spot().ok_or(Error::Internal)?;

        // Start with continuation value plus any coupon at this step
        let coupon = self.coupon_map.get(&state.step).copied().unwrap_or(0.0);
        let hold_value = continuation_value + coupon;

        // Check conversion option
        let mut optimal_value = hold_value;
        if self.conversion_allowed(state.step) {
            let conversion_value = spot * self.conversion_ratio;
            optimal_value = optimal_value.max(conversion_value);
        }

        // Apply issuer call option (issuer forces redemption if beneficial)
        if let Some(call_price) = self.call_price_at_step(state.step) {
            optimal_value = optimal_value.min(call_price);
        }

        // Apply holder put option (holder forces redemption if beneficial)
        if let Some(put_price) = self.put_price_at_step(state.step) {
            optimal_value = optimal_value.max(put_price);
        }

        Ok(optimal_value)
    }
}

/// Extract equity market state from market context
fn extract_equity_state(
    ctx: &MarketContext,
    underlying_id: &str,
    disc_id: &'static str,
    maturity: Date,
    expected_currency: Currency,
) -> Result<(F, F, F, F, F)> {
    // Get spot price
    let spot_price = ctx.price(underlying_id)?;
    let spot = match spot_price {
        finstack_core::market_data::primitives::MarketScalar::Price(money) => {
            // Enforce currency safety
            if money.currency() != expected_currency {
                return Err(Error::Internal);
            }
            // For currency safety, we extract the amount but currency checks should be done by caller
            money.amount()
        }
        finstack_core::market_data::primitives::MarketScalar::Unitless(value) => *value,
    };

    // Get volatility (must be unitless)
    let vol_id = format!("{}-VOL", underlying_id);
    let volatility = match ctx.price(&vol_id)? {
        finstack_core::market_data::primitives::MarketScalar::Unitless(vol) => *vol,
        _ => return Err(Error::Internal),
    };

    // Get dividend yield (default to 0 if not available, must be unitless)
    let div_yield_id = format!("{}-DIVYIELD", underlying_id);
    let dividend_yield = ctx
        .price(&div_yield_id)
        .map(|scalar| match scalar {
            finstack_core::market_data::primitives::MarketScalar::Unitless(yield_val) => *yield_val,
            _ => 0.0,
        })
        .unwrap_or(0.0);

    // Get risk-free rate from discount curve
    let discount_curve = ctx.disc(disc_id)?;
    let base_date = discount_curve.base_date();

    // Calculate time to maturity
    let time_to_maturity = finstack_core::dates::DayCount::Act365F
        .year_fraction(
            base_date,
            maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap_or(0.0);

    // Extract risk-free rate (approximate from maturity point)
    let risk_free_rate = if time_to_maturity > 0.0 {
        -discount_curve.df(time_to_maturity).ln() / time_to_maturity
    } else {
        0.05 // Fallback rate
    };

    Ok((
        spot,
        volatility,
        dividend_yield,
        risk_free_rate,
        time_to_maturity,
    ))
}

/// Main pricing function for convertible bonds
pub fn price_convertible_bond(
    bond: &ConvertibleBond,
    market_context: &MarketContext,
    tree_type: ConvertibleTreeType,
) -> Result<Money> {
    // Step 1: Generate cashflow schedule using CashflowBuilder
    let mut builder = cf();
    builder.principal(bond.notional, bond.issue, bond.maturity);

    // Add fixed coupon if specified
    if let Some(fixed_spec) = bond.fixed_coupon {
        builder.fixed_cf(fixed_spec);
    }

    // Add floating coupon if specified
    if let Some(floating_spec) = bond.floating_coupon {
        builder.floating_cf(floating_spec);
    }

    let cashflow_schedule = builder.build()?;

    // Step 2: Extract market data using helper
    let underlying_id = bond.underlying_equity_id.as_ref().ok_or(Error::Internal)?;
    let (spot, volatility, dividend_yield, risk_free_rate, time_to_maturity) =
        extract_equity_state(
            market_context,
            underlying_id,
            bond.disc_id,
            bond.maturity,
            bond.notional.currency(),
        )?;

    if time_to_maturity <= 0.0 {
        return Ok(Money::new(0.0, bond.notional.currency()));
    }

    // Step 3: Create valuator
    let steps = match tree_type {
        ConvertibleTreeType::Binomial(n) => n,
        ConvertibleTreeType::Trinomial(n) => n,
    };

    let base_date = market_context.disc(bond.disc_id)?.base_date();
    let valuator =
        ConvertibleBondValuator::new(bond, &cashflow_schedule, time_to_maturity, steps, base_date)?;

    // Step 4: Create initial state variables
    let initial_vars = single_factor_equity_state(spot, risk_free_rate, dividend_yield, volatility);

    // Step 5: Price using selected tree model
    let pv_amount = match tree_type {
        ConvertibleTreeType::Binomial(steps) => {
            let tree = BinomialTree::crr(steps);
            tree.price(initial_vars, time_to_maturity, market_context, &valuator)?
        }
        ConvertibleTreeType::Trinomial(steps) => {
            let tree = TrinomialTree::standard(steps);
            tree.price(initial_vars, time_to_maturity, market_context, &valuator)?
        }
    };

    Ok(Money::new(pv_amount, bond.notional.currency()))
}

/// Calculate Greeks for a convertible bond
pub fn calculate_convertible_greeks(
    bond: &ConvertibleBond,
    market_context: &MarketContext,
    tree_type: ConvertibleTreeType,
    bump_size: Option<F>,
) -> Result<TreeGreeks> {
    // Generate cashflow schedule
    let mut builder = cf();
    builder.principal(bond.notional, bond.issue, bond.maturity);

    if let Some(fixed_spec) = bond.fixed_coupon {
        builder.fixed_cf(fixed_spec);
    }

    if let Some(floating_spec) = bond.floating_coupon {
        builder.floating_cf(floating_spec);
    }

    let cashflow_schedule = builder.build()?;

    // Extract market data using helper
    let underlying_id = bond.underlying_equity_id.as_ref().ok_or(Error::Internal)?;
    let (spot, volatility, dividend_yield, risk_free_rate, time_to_maturity) =
        extract_equity_state(
            market_context,
            underlying_id,
            bond.disc_id,
            bond.maturity,
            bond.notional.currency(),
        )?;

    // Create valuator and initial state
    let steps = match tree_type {
        ConvertibleTreeType::Binomial(n) => n,
        ConvertibleTreeType::Trinomial(n) => n,
    };

    let base_date = market_context.disc(bond.disc_id)?.base_date();
    let valuator =
        ConvertibleBondValuator::new(bond, &cashflow_schedule, time_to_maturity, steps, base_date)?;

    let initial_vars = single_factor_equity_state(spot, risk_free_rate, dividend_yield, volatility);

    // Calculate Greeks using selected tree model
    match tree_type {
        ConvertibleTreeType::Binomial(steps) => {
            let tree = BinomialTree::crr(steps);
            TreeModel::calculate_greeks(
                &tree,
                initial_vars,
                time_to_maturity,
                market_context,
                &valuator,
                bump_size,
            )
        }
        ConvertibleTreeType::Trinomial(steps) => {
            let tree = TrinomialTree::standard(steps);
            TreeModel::calculate_greeks(
                &tree,
                initial_vars,
                time_to_maturity,
                market_context,
                &valuator,
                bump_size,
            )
        }
    }
}

/// Calculate convertible bond parity
pub fn calculate_parity(bond: &ConvertibleBond, current_spot: F) -> F {
    let conversion_ratio = if let Some(ratio) = bond.conversion.ratio {
        ratio
    } else if let Some(price) = bond.conversion.price {
        bond.notional.amount() / price
    } else {
        return 0.0;
    };

    (current_spot * conversion_ratio) / bond.notional.amount()
}

/// Calculate conversion premium
pub fn calculate_conversion_premium(bond_price: F, current_spot: F, conversion_ratio: F) -> F {
    let conversion_value = current_spot * conversion_ratio;
    if conversion_value > 0.0 {
        (bond_price / conversion_value) - 1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::types::{CouponType, FixedCouponSpec};
    use crate::instruments::fixed_income::convertible::{
        AntiDilutionPolicy, ConversionPolicy, ConversionSpec, DividendAdjustment,
    };
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
    use finstack_core::market_data::primitives::MarketScalar;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use time::Month;

    fn create_test_bond() -> ConvertibleBond {
        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        let conversion_spec = ConversionSpec {
            ratio: Some(10.0), // 10 shares per bond
            price: None,
            policy: ConversionPolicy::Voluntary,
            anti_dilution: AntiDilutionPolicy::None,
            dividend_adjustment: DividendAdjustment::None,
        };

        let fixed_coupon = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05, // 5% coupon
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        ConvertibleBond {
            id: "TEST_CONVERTIBLE".to_string(),
            notional: Money::new(1000.0, Currency::USD),
            issue,
            maturity,
            disc_id: "USD-OIS",
            conversion: conversion_spec,
            underlying_equity_id: Some("AAPL".to_string()),
            call_put: None,
            fixed_coupon: Some(fixed_coupon),
            floating_coupon: None,
            attributes: Default::default(),
        }
    }

    fn create_test_market_context() -> MarketContext {
        // Create a simple discount curve that covers beyond the bond maturity
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (10.0, 0.90)]) // Extended to 10 years
            .set_interp(finstack_core::market_data::interp::InterpStyle::Linear)
            .build()
            .unwrap();

        MarketContext::new()
            .insert_discount(discount_curve)
            .insert_price("AAPL", MarketScalar::Unitless(150.0)) // $150 stock price
            .insert_price("AAPL-VOL", MarketScalar::Unitless(0.25)) // 25% volatility
            .insert_price("AAPL-DIVYIELD", MarketScalar::Unitless(0.02)) // 2% dividend yield
    }

    #[test]
    fn test_convertible_bond_parity() {
        let bond = create_test_bond();
        let parity = calculate_parity(&bond, 150.0);

        // With 10 shares per bond and $150 stock price:
        // Conversion value = 10 * 150 = $1,500
        // Parity = $1,500 / $1,000 = 1.5 (150%)
        assert!((parity - 1.5).abs() < 1e-9);
    }

    #[test]
    fn test_convertible_bond_pricing() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();

        let price =
            price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(50));

        assert!(price.is_ok());
        let price = price.unwrap();

        // Should be worth at least the conversion value
        let conversion_value = 150.0 * 10.0; // $1,500
        assert!(price.amount() >= conversion_value);

        // Should be in a reasonable range
        assert!(price.amount() > 1000.0 && price.amount() < 2000.0);
    }

    #[test]
    fn test_convertible_greeks_calculation() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();

        let greeks = calculate_convertible_greeks(
            &bond,
            &market_context,
            ConvertibleTreeType::Binomial(50),
            Some(0.01),
        );

        assert!(greeks.is_ok());
        let greeks = greeks.unwrap();

        // Delta should be positive for convertible bonds (increases with stock price)
        assert!(greeks.delta > 0.0);

        // Gamma should be positive
        assert!(greeks.gamma >= 0.0);

        // Price should be reasonable
        assert!(greeks.price > 1000.0);
    }
}
