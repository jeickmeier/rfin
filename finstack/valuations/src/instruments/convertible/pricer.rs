//! Convertible bond pricing model using binomial/trinomial trees.
//!
//! Implements a hybrid debt-equity pricing model that:
//! 1. Uses CashflowBuilder to generate the bond's coupon schedule
//! 2. Applies tree-based pricing to capture the equity conversion option
//! 3. Handles call/put provisions and various conversion policies
//!
//! Public API:
//! - `price_convertible_bond`: Present value using selected tree type
//! - `calculate_convertible_greeks`: Tree-based Greeks and price
//! - `calculate_parity`: Equity parity ratio
//! - `calculate_conversion_premium`: Conversion premium versus equity value
//!
//! Future enhancements
//! - Tsiveriotis–Fernandes-style split of cash-only vs equity components, with
//!   cash flows discounted at risk-free plus issuer credit spread and equity flows
//!   at risk-free, to better reflect credit risk in the lattice framework.
//! - Optional credit-spread factor (or curve) integration to align with market
//!   practice when valuing credit-sensitive convertibles.
//!
//! # Known Limitations
//!
//! TODO: Add credit-equity correlation infrastructure for ConvertibleBond pricing.
//! Currently uses single-factor equity model without credit-equity correlation.
//! Future work requires:
//! - Two-factor model (equity + credit) in tree framework
//! - Correlation parameter in pricing inputs
//! - Tree pricing framework updates to handle correlated factors
//! - Greeks calculation for correlation risk

use finstack_core::error::InputError;
use finstack_core::market_data::context::MarketContext;
use finstack_core::prelude::*;
use finstack_core::{Error, Result};
use std::collections::HashMap;

use crate::cashflow::builder::CashFlowSchedule;
use crate::instruments::common::models::trees::tree_framework::map_date_to_step;
use crate::instruments::common::models::{
    single_factor_equity_state, BinomialTree, NodeState, TreeGreeks, TreeModel, TreeValuator,
    TrinomialTree,
};
use crate::instruments::common::traits::Attributes;
use crate::instruments::common::traits::Instrument;
use crate::instruments::convertible::{ConversionPolicy, ConvertibleBond};

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
    conversion_ratio: f64,
    /// Face value of the bond
    face_value: f64,
    /// Coupon cashflows mapped to tree steps
    coupon_map: HashMap<usize, f64>,
    /// Call prices mapped to tree steps
    call_map: HashMap<usize, f64>,
    /// Put prices mapped to tree steps
    put_map: HashMap<usize, f64>,
    /// Conversion policy
    conversion_policy: ConversionPolicy,
    /// Time steps for the tree (in years)
    time_steps: Vec<f64>,
    /// Base date for time calculations
    base_date: Date,
}

impl ConvertibleBondValuator {
    /// Create a new convertible bond valuator
    pub fn new(
        bond: &ConvertibleBond,
        cashflow_schedule: &CashFlowSchedule,
        time_to_maturity: f64,
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
        let dt = time_to_maturity / steps as f64;
        let mut time_steps = Vec::with_capacity(steps + 1);

        for i in 0..=steps {
            time_steps.push(i as f64 * dt);
        }

        // Process coupon cashflows (exclude reset-only events) using schedule day count
        let mut coupon_map = HashMap::new();
        for cf in cashflow_schedule.coupons() {
            let bounded_step = map_date_to_step(
                base_date,
                cf.date,
                bond.maturity,
                steps,
                cashflow_schedule.day_count,
            );
            *coupon_map.entry(bounded_step).or_insert(0.0) += cf.amount.amount();
        }

        // Map call/put schedules to tree steps
        let mut call_map = HashMap::new();
        let mut put_map = HashMap::new();

        if let Some(ref call_put) = bond.call_put {
            // Map call schedule using shared helper
            for call in &call_put.calls {
                if call.date > base_date && call.date <= bond.maturity {
                    let bounded_step = map_date_to_step(
                        base_date,
                        call.date,
                        bond.maturity,
                        steps,
                        cashflow_schedule.day_count,
                    );
                    let call_price = bond.notional.amount() * (call.price_pct_of_par / 100.0);
                    call_map.insert(bounded_step, call_price);
                }
            }

            // Map put schedule using shared helper
            for put in &call_put.puts {
                if put.date > base_date && put.date <= bond.maturity {
                    let bounded_step = map_date_to_step(
                        base_date,
                        put.date,
                        bond.maturity,
                        steps,
                        cashflow_schedule.day_count,
                    );
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
    fn call_price_at_step(&self, step: usize) -> Option<f64> {
        self.call_map.get(&step).copied()
    }

    /// Get put price at a given step (if puttable)
    fn put_price_at_step(&self, step: usize) -> Option<f64> {
        self.put_map.get(&step).copied()
    }
}

impl TreeValuator for ConvertibleBondValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
        let spot = state.spot().ok_or(Error::Internal)?;

        // At maturity, choose between conversion and redemption
        let conversion_value = spot * self.conversion_ratio;
        let redemption_value = self.face_value;

        // Add any final coupon payment
        let final_coupon = self.coupon_map.get(&state.step).copied().unwrap_or(0.0);

        Ok(conversion_value.max(redemption_value) + final_coupon)
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: f64, _dt: f64) -> Result<f64> {
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
    discount_curve_id: &finstack_core::types::CurveId,
    maturity: Date,
    expected_currency: Currency,
    attributes: &Attributes,
    as_of: Date,
) -> Result<(f64, f64, f64, f64, f64)> {
    // Get spot price
    let spot_price = ctx.price(underlying_id)?;
    let spot = match spot_price {
        finstack_core::market_data::scalars::MarketScalar::Price(money) => {
            // Enforce currency safety
            if money.currency() != expected_currency {
                return Err(Error::Internal);
            }
            // For currency safety, we extract the amount but currency checks should be done by caller
            money.amount()
        }
        finstack_core::market_data::scalars::MarketScalar::Unitless(value) => *value,
    };

    // Get risk-free rate from discount curve
    let discount_curve = ctx.get_discount_ref(discount_curve_id.as_str())?;

    // Calculate time to maturity using the provided as_of date
    let time_to_maturity = finstack_core::dates::DayCount::Act365F
        .year_fraction(
            as_of,
            maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap_or(0.0);

    // Extract instantaneous-equivalent risk-free rate from discount factor at maturity.
    // If time_to_maturity is zero (already matured), return zero to avoid division by zero.
    let risk_free_rate = if time_to_maturity > 0.0 {
        -discount_curve.df(time_to_maturity).ln() / time_to_maturity
    } else {
        0.0
    };

    // Resolve volatility (unitless) via metadata or naming heuristics.
    let mut vol_candidates: Vec<String> = Vec::new();
    if let Some(id) = attributes.get_meta("vol_surface_id") {
        vol_candidates.push(id.to_string());
    }
    if let Some(id) = attributes.get_meta("vol_surface_id") {
        vol_candidates.push(id.to_string());
    }
    if let Some(id) = attributes.get_meta("vol_scalar_id") {
        vol_candidates.push(id.to_string());
    }
    vol_candidates.push(format!("{}-VOL", underlying_id));
    if let Some(stripped) = underlying_id.strip_suffix("-SPOT") {
        vol_candidates.push(format!("{}-VOL", stripped));
    }
    let volatility = resolve_volatility(ctx, &vol_candidates, time_to_maturity, spot)?;

    // Resolve dividend yield (unitless), defaulting to zero if unavailable.
    let mut dividend_candidates: Vec<String> = Vec::new();
    if let Some(id) = attributes.get_meta("div_yield_id") {
        dividend_candidates.push(id.to_string());
    }
    dividend_candidates.push(format!("{}-DIVYIELD", underlying_id));
    if let Some(stripped) = underlying_id.strip_suffix("-SPOT") {
        dividend_candidates.push(format!("{}-DIVYIELD", stripped));
    }
    let dividend_yield = resolve_unitless_scalar(ctx, &dividend_candidates)?.unwrap_or(0.0);

    Ok((
        spot,
        volatility,
        dividend_yield,
        risk_free_rate,
        time_to_maturity,
    ))
}

fn resolve_unitless_scalar(ctx: &MarketContext, candidate_ids: &[String]) -> Result<Option<f64>> {
    for id in candidate_ids {
        match ctx.price(id) {
            Ok(finstack_core::market_data::scalars::MarketScalar::Unitless(value)) => {
                return Ok(Some(*value));
            }
            Ok(_) => {}
            Err(err) => {
                if matches!(err, Error::Input(InputError::NotFound { .. })) {
                    continue;
                }
                return Err(err);
            }
        }
    }
    Ok(None)
}

fn resolve_volatility(
    ctx: &MarketContext,
    candidate_ids: &[String],
    time_to_maturity: f64,
    spot: f64,
) -> Result<f64> {
    let mut first_missing: Option<String> = None;

    for id in candidate_ids {
        match ctx.price(id) {
            Ok(finstack_core::market_data::scalars::MarketScalar::Unitless(vol)) => {
                return Ok(*vol);
            }
            Ok(_) => {}
            Err(err) => {
                if matches!(err, Error::Input(InputError::NotFound { .. })) {
                    if first_missing.is_none() {
                        first_missing = Some(id.clone());
                    }
                } else {
                    return Err(err);
                }
            }
        }

        match ctx.surface_ref(id) {
            Ok(surface) => {
                let vol = surface.value_clamped(time_to_maturity, spot);
                return Ok(vol);
            }
            Err(err) => {
                if matches!(err, Error::Input(InputError::NotFound { .. })) {
                    if first_missing.is_none() {
                        first_missing = Some(id.clone());
                    }
                    continue;
                }
                return Err(err);
            }
        }
    }

    let missing_id = first_missing.unwrap_or_else(|| "volatility".to_string());
    Err(Error::from(InputError::NotFound { id: missing_id }))
}

/// Aggregated data required for tree pricing, prepared once to avoid duplication.
struct PricingInputs {
    cashflow_schedule: CashFlowSchedule,
    spot: f64,
    volatility: f64,
    dividend_yield: f64,
    risk_free_rate: f64,
    time_to_maturity: f64,
}

/// Prepare all necessary inputs for pricing and greeks calculation.
fn prepare_for_pricing(
    bond: &ConvertibleBond,
    market_context: &MarketContext,
    as_of: Date,
) -> Result<PricingInputs> {
    let cashflow_schedule = build_convertible_schedule(bond)?;
    let underlying_id = bond.underlying_equity_id.as_ref().ok_or(Error::Internal)?;
    let (spot, volatility, dividend_yield, risk_free_rate, time_to_maturity) =
        extract_equity_state(
            market_context,
            underlying_id,
            &bond.discount_curve_id,
            bond.maturity,
            bond.notional.currency(),
            &bond.attributes,
            as_of,
        )?;

    Ok(PricingInputs {
        cashflow_schedule,
        spot,
        volatility,
        dividend_yield,
        risk_free_rate,
        time_to_maturity,
    })
}

/// Main pricing function for convertible bonds
pub fn price_convertible_bond(
    bond: &ConvertibleBond,
    market_context: &MarketContext,
    tree_type: ConvertibleTreeType,
    as_of: Date,
) -> Result<Money> {
    // Step 1: Prepare all inputs
    let inputs = prepare_for_pricing(bond, market_context, as_of)?;

    if inputs.time_to_maturity <= 0.0 {
        return Ok(Money::new(0.0, bond.notional.currency()));
    }

    // Step 2: Create valuator
    let steps = match tree_type {
        ConvertibleTreeType::Binomial(n) => n,
        ConvertibleTreeType::Trinomial(n) => n,
    };

    let valuator = ConvertibleBondValuator::new(
        bond,
        &inputs.cashflow_schedule,
        inputs.time_to_maturity,
        steps,
        as_of,
    )?;

    // Step 3: Create initial state variables
    let initial_vars = single_factor_equity_state(
        inputs.spot,
        inputs.risk_free_rate,
        inputs.dividend_yield,
        inputs.volatility,
    );

    // Step 4: Price using selected tree model
    let pv_amount = match tree_type {
        ConvertibleTreeType::Binomial(steps) => {
            let tree = BinomialTree::crr(steps);
            tree.price(
                initial_vars,
                inputs.time_to_maturity,
                market_context,
                &valuator,
            )?
        }
        ConvertibleTreeType::Trinomial(steps) => {
            let tree = TrinomialTree::standard(steps);
            tree.price(
                initial_vars,
                inputs.time_to_maturity,
                market_context,
                &valuator,
            )?
        }
    };

    Ok(Money::new(pv_amount, bond.notional.currency()))
}

/// Calculate Greeks for a convertible bond
pub fn calculate_convertible_greeks(
    bond: &ConvertibleBond,
    market_context: &MarketContext,
    tree_type: ConvertibleTreeType,
    bump_size: Option<f64>,
    as_of: Date,
) -> Result<TreeGreeks> {
    // Prepare all inputs
    let inputs = prepare_for_pricing(bond, market_context, as_of)?;

    // Create valuator and initial state
    let steps = match tree_type {
        ConvertibleTreeType::Binomial(n) => n,
        ConvertibleTreeType::Trinomial(n) => n,
    };

    let valuator = ConvertibleBondValuator::new(
        bond,
        &inputs.cashflow_schedule,
        inputs.time_to_maturity,
        steps,
        as_of,
    )?;

    let initial_vars = single_factor_equity_state(
        inputs.spot,
        inputs.risk_free_rate,
        inputs.dividend_yield,
        inputs.volatility,
    );

    // Calculate Greeks using selected tree model
    match tree_type {
        ConvertibleTreeType::Binomial(steps) => {
            let tree = BinomialTree::crr(steps);
            TreeModel::calculate_greeks(
                &tree,
                initial_vars,
                inputs.time_to_maturity,
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
                inputs.time_to_maturity,
                market_context,
                &valuator,
                bump_size,
            )
        }
    }
}

/// Build the convertible bond cashflow schedule using common builder flow.
fn build_convertible_schedule(bond: &ConvertibleBond) -> Result<CashFlowSchedule> {
    let mut builder = CashFlowSchedule::builder();
    builder.principal(bond.notional, bond.issue, bond.maturity);
    if let Some(fixed_spec) = &bond.fixed_coupon {
        builder.fixed_cf(fixed_spec.clone());
    }
    if let Some(floating_spec) = &bond.floating_coupon {
        builder.floating_cf(floating_spec.clone());
    }
    builder.build()
}

/// Calculate convertible bond parity
pub fn calculate_parity(bond: &ConvertibleBond, current_spot: f64) -> f64 {
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
pub fn calculate_conversion_premium(
    bond_price: f64,
    current_spot: f64,
    conversion_ratio: f64,
) -> f64 {
    let conversion_value = current_spot * conversion_ratio;
    if conversion_value > 0.0 {
        (bond_price / conversion_value) - 1.0
    } else {
        0.0
    }
}

// ========================= REGISTRY PRICER =========================

/// Registry pricer for Convertible Bond using tree-based pricing
pub struct SimpleConvertibleDiscountingPricer;

impl SimpleConvertibleDiscountingPricer {
    /// Create a new convertible bond discounting pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleConvertibleDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pricer::Pricer for SimpleConvertibleDiscountingPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(
            crate::pricer::InstrumentType::Convertible,
            crate::pricer::ModelKey::Discounting,
        )
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        use crate::instruments::common::traits::Instrument;

        // Type-safe downcasting
        let convertible = instrument
            .as_any()
            .downcast_ref::<crate::instruments::convertible::ConvertibleBond>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::Convertible,
                    instrument.key(),
                )
            })?;

        // Use the provided as_of date for valuation
        // Compute present value using the engine with binomial tree
        let pv = price_convertible_bond(
            convertible,
            market,
            ConvertibleTreeType::Binomial(100),
            as_of,
        )
        .map_err(|e| crate::pricer::PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(crate::results::ValuationResult::stamped(
            convertible.id(),
            as_of,
            pv,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::specs::{CouponType, FixedCouponSpec};
    use crate::instruments::convertible::{
        AntiDilutionPolicy, ConversionPolicy, ConversionSpec, DividendAdjustment,
    };
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use time::Month;

    fn create_test_bond() -> ConvertibleBond {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("valid date");

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
            id: "TEST_CONVERTIBLE".to_string().into(),
            notional: Money::new(1000.0, Currency::USD),
            issue,
            maturity,
            discount_curve_id: "USD-OIS".into(),
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
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (10.0, 0.90)]) // Extended to 10 years
            .set_interp(finstack_core::math::interp::InterpStyle::Linear)
            .build()
            .expect("should succeed");

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
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let price = price_convertible_bond(
            &bond,
            &market_context,
            ConvertibleTreeType::Binomial(50),
            as_of,
        );

        assert!(price.is_ok());
        let price = price.expect("should succeed");

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

        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let greeks = calculate_convertible_greeks(
            &bond,
            &market_context,
            ConvertibleTreeType::Binomial(50),
            Some(0.01),
            as_of,
        );

        assert!(greeks.is_ok());
        let greeks = greeks.expect("should succeed");

        // Delta should be positive for convertible bonds (increases with stock price)
        assert!(greeks.delta > 0.0);

        // Gamma should be positive
        assert!(greeks.gamma >= 0.0);

        // Price should be reasonable
        assert!(greeks.price > 1000.0);
    }
}
