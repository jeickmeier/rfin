//! Convertible bond pricing model using binomial/trinomial trees.
//!
//! Implements a hybrid debt-equity pricing model that:
//! 1. Uses CashflowBuilder to generate the bond's coupon schedule
//! 2. Applies tree-based pricing to capture the equity conversion option
//! 3. Handles call/put provisions and various conversion policies

use finstack_core::prelude::*;
use finstack_core::market_data::context::MarketContext;
use finstack_core::{Error, Result, F};

use crate::cashflow::builder::{cf, CashFlowSchedule};
use crate::cashflow::primitives::CFKind;
use crate::instruments::options::models::{
    NodeState, TreeModel, TreeValuator, TreeGreeks, 
    BinomialTree, TrinomialTree, single_factor_equity_state
};
use crate::instruments::fixed_income::bond::CallPutSchedule;

use super::{ConvertibleBond, ConversionPolicy};

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
    cashflows_at_step: Vec<F>,
    /// Call schedule (if any)
    #[allow(dead_code)]
    call_schedule: Option<CallPutSchedule>,
    /// Put schedule (if any) 
    #[allow(dead_code)]
    put_schedule: Option<CallPutSchedule>,
    /// Conversion policy
    conversion_policy: ConversionPolicy,
    /// Time steps for the tree (in years)
    time_steps: Vec<F>,
    /// Currency for consistency checks
    #[allow(dead_code)]
    currency: Currency,
}

impl ConvertibleBondValuator {
    /// Create a new convertible bond valuator
    pub fn new(
        bond: &ConvertibleBond,
        cashflow_schedule: &CashFlowSchedule,
        time_to_maturity: F,
        steps: usize,
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
        let mut cashflows_at_step = vec![0.0; steps + 1];
        let mut time_steps = Vec::with_capacity(steps + 1);
        
        for i in 0..=steps {
            time_steps.push(i as F * dt);
        }

        // Process coupon and principal cashflows
        for cf in &cashflow_schedule.flows {
            if matches!(cf.kind, CFKind::Fixed | CFKind::Stub | CFKind::FloatReset) {
                // Map cashflow date to tree step
                let cf_time = 0.0; // TODO: Calculate actual time from issue to cf.date
                let step_index = ((cf_time / time_to_maturity) * steps as F).round() as usize;
                if step_index < cashflows_at_step.len() {
                    cashflows_at_step[step_index] += cf.amount.amount();
                }
            }
        }

        Ok(Self {
            conversion_ratio,
            face_value: bond.notional.amount(),
            cashflows_at_step,
            call_schedule: bond.call_put.clone(),
            put_schedule: None, // TODO: Separate call and put schedules
            conversion_policy: bond.conversion.policy.clone(),
            time_steps,
            currency: bond.notional.currency(),
        })
    }

    /// Check if conversion is allowed at a given time step
    fn conversion_allowed(&self, step: usize) -> bool {
        let _time = self.time_steps.get(step).copied().unwrap_or(0.0);
        
        match &self.conversion_policy {
            ConversionPolicy::Voluntary => true,
            ConversionPolicy::MandatoryOn(_date) => {
                // TODO: Check if time matches the mandatory conversion date
                step == self.time_steps.len() - 1 // For now, only at maturity
            }
            ConversionPolicy::Window { start: _, end: _ } => {
                // TODO: Check if time falls within the conversion window
                true // For now, allow conversion always
            }
            ConversionPolicy::UponEvent(_event) => {
                // TODO: Check event conditions
                false // For now, disable event-triggered conversion
            }
        }
    }

    /// Get call price at a given step (if callable)
    fn call_price_at_step(&self, _step: usize) -> Option<F> {
        // TODO: Implement call schedule lookup
        // For now, return None (no call provisions)
        None
    }

    /// Get put price at a given step (if puttable)
    fn put_price_at_step(&self, _step: usize) -> Option<F> {
        // TODO: Implement put schedule lookup
        // For now, return None (no put provisions)
        None
    }
}

impl TreeValuator for ConvertibleBondValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<F> {
        let spot = state.spot().ok_or(Error::Internal)?;
        
        // At maturity, choose between conversion and redemption
        let conversion_value = spot * self.conversion_ratio;
        let redemption_value = self.face_value;
        
        // Add any final coupon payment
        let final_coupon = self.cashflows_at_step.get(state.step).copied().unwrap_or(0.0);
        
        Ok(conversion_value.max(redemption_value) + final_coupon)
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: F) -> Result<F> {
        let spot = state.spot().ok_or(Error::Internal)?;
        
        // Start with continuation value plus any coupon at this step
        let coupon = self.cashflows_at_step.get(state.step).copied().unwrap_or(0.0);
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

    // Step 2: Extract market data
    let underlying_id = bond.underlying_equity_id
        .as_ref()
        .ok_or(Error::Internal)?;
    
    let spot_price = market_context
        .market_scalar(underlying_id)?
        .clone();
    
    let spot = match spot_price {
        finstack_core::market_data::primitives::MarketScalar::Price(money) => {
            if money.currency() != bond.notional.currency() {
                return Err(Error::Internal); // Currency mismatch
            }
            money.amount()
        }
        finstack_core::market_data::primitives::MarketScalar::Unitless(value) => value,
    };

    // Get volatility (assume it's stored as unitless scalar)
    let vol_id = format!("{}-VOL", underlying_id);
    let volatility = match market_context.market_scalar(&vol_id)? {
        finstack_core::market_data::primitives::MarketScalar::Unitless(vol) => *vol,
        _ => return Err(Error::Internal),
    };

    // Get dividend yield (default to 0 if not available)
    let div_yield_id = format!("{}-DIVYIELD", underlying_id);
    let dividend_yield = market_context
        .market_scalar(&div_yield_id)
        .map(|scalar| match scalar {
            finstack_core::market_data::primitives::MarketScalar::Unitless(yield_val) => *yield_val,
            _ => 0.0,
        })
        .unwrap_or(0.0);

    // Get risk-free rate from discount curve
    let discount_curve = market_context.discount(bond.disc_id)?;
    let base_date = discount_curve.base_date();
    
    // Calculate time to maturity
    let time_to_maturity = finstack_core::dates::DayCount::Act365F
        .year_fraction(base_date, bond.maturity)
        .unwrap_or(0.0);
    
    if time_to_maturity <= 0.0 {
        return Ok(Money::new(0.0, bond.notional.currency()));
    }

    // Extract risk-free rate (approximate from 1-year point)
    let risk_free_rate = if time_to_maturity > 0.0 {
        -discount_curve.df(time_to_maturity).ln() / time_to_maturity
    } else {
        0.05 // Fallback rate
    };

    // Step 3: Create valuator
    let steps = match tree_type {
        ConvertibleTreeType::Binomial(n) => n,
        ConvertibleTreeType::Trinomial(n) => n,
    };
    
    let valuator = ConvertibleBondValuator::new(
        bond,
        &cashflow_schedule,
        time_to_maturity,
        steps,
    )?;

    // Step 4: Create initial state variables
    let initial_vars = single_factor_equity_state(
        spot,
        risk_free_rate,
        dividend_yield,
        volatility,
    );

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

    // Extract market data (same logic as price_convertible_bond)
    let underlying_id = bond.underlying_equity_id
        .as_ref()
        .ok_or(Error::Internal)?;
    
    let spot_price = market_context.market_scalar(underlying_id)?;
    let spot = match spot_price {
        finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
        finstack_core::market_data::primitives::MarketScalar::Unitless(value) => *value,
    };

    let vol_id = format!("{}-VOL", underlying_id);
    let volatility = match market_context.market_scalar(&vol_id)? {
        finstack_core::market_data::primitives::MarketScalar::Unitless(vol) => *vol,
        _ => return Err(Error::Internal),
    };

    let div_yield_id = format!("{}-DIVYIELD", underlying_id);
    let dividend_yield = market_context
        .market_scalar(&div_yield_id)
        .map(|scalar| match scalar {
            finstack_core::market_data::primitives::MarketScalar::Unitless(yield_val) => *yield_val,
            _ => 0.0,
        })
        .unwrap_or(0.0);

    let discount_curve = market_context.discount(bond.disc_id)?;
    let base_date = discount_curve.base_date();
    let time_to_maturity = finstack_core::dates::DayCount::Act365F
        .year_fraction(base_date, bond.maturity)
        .unwrap_or(0.0);
    
    let risk_free_rate = if time_to_maturity > 0.0 {
        -discount_curve.df(time_to_maturity).ln() / time_to_maturity
    } else {
        0.05
    };

    // Create valuator and initial state
    let steps = match tree_type {
        ConvertibleTreeType::Binomial(n) => n,
        ConvertibleTreeType::Trinomial(n) => n,
    };
    
    let valuator = ConvertibleBondValuator::new(
        bond,
        &cashflow_schedule,
        time_to_maturity,
        steps,
    )?;

    let initial_vars = single_factor_equity_state(
        spot,
        risk_free_rate,
        dividend_yield,
        volatility,
    );

    // Calculate Greeks using selected tree model
    match tree_type {
        ConvertibleTreeType::Binomial(steps) => {
            let tree = BinomialTree::crr(steps);
            TreeModel::calculate_greeks(&tree, initial_vars, time_to_maturity, market_context, &valuator, bump_size)
        }
        ConvertibleTreeType::Trinomial(steps) => {
            let tree = TrinomialTree::standard(steps);
            TreeModel::calculate_greeks(&tree, initial_vars, time_to_maturity, market_context, &valuator, bump_size)
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
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount, Frequency, BusinessDayConvention, StubKind};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::primitives::MarketScalar;
    use crate::cashflow::builder::types::{FixedCouponSpec, CouponType};
    use crate::instruments::fixed_income::convertible::{ConversionPolicy, ConversionSpec, AntiDilutionPolicy, DividendAdjustment};
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
            .linear_df()
            .build()
            .unwrap();

        MarketContext::new()
            .with_discount(discount_curve)
            .with_price("AAPL", MarketScalar::Unitless(150.0)) // $150 stock price
            .with_price("AAPL-VOL", MarketScalar::Unitless(0.25)) // 25% volatility
            .with_price("AAPL-DIVYIELD", MarketScalar::Unitless(0.02)) // 2% dividend yield
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
        
        let price = price_convertible_bond(
            &bond,
            &market_context,
            ConvertibleTreeType::Binomial(50),
        );
        
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
