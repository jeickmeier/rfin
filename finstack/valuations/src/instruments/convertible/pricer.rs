//! Convertible bond pricing model using Tsiveriotis-Zhang tree.
//!
//! Implements a hybrid debt-equity pricing model that:
//! 1. Uses `CashFlowBuilder` to generate the bond's coupon schedule
//! 2. Applies Tsiveriotis-Zhang tree decomposition to capture the equity conversion option
//!    while accounting for credit risk on the cash-only component.
//! 3. Handles call/put provisions and various conversion policies
//!
//! Public API:
//! - `price_convertible_bond`: Present value using selected tree type
//! - `calculate_convertible_greeks`: Tree-based Greeks and price
//! - `calculate_parity`: Equity parity ratio
//! - `calculate_conversion_premium`: Conversion premium versus equity value

use finstack_core::dates::Date;
use finstack_core::error::InputError;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::{Error, Result};
use finstack_core::collections::HashMap;

use crate::cashflow::builder::CashFlowSchedule;
use crate::instruments::common::models::trees::tree_framework::map_date_to_step;
use crate::instruments::common::models::{
    single_factor_equity_state, EvolutionParams, StateVariables, TreeGreeks,
};
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

/// Convertible bond valuator implementing the TZ logic
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
    /// Credit spread for the debt component
    credit_spread: f64,
}

impl ConvertibleBondValuator {
    /// Create a new convertible bond valuator
    pub fn new(
        bond: &ConvertibleBond,
        cashflow_schedule: &CashFlowSchedule,
        time_to_maturity: f64,
        steps: usize,
        base_date: Date,
        credit_spread: f64,
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
        let mut coupon_map = HashMap::default();
        for cf in cashflow_schedule.coupons() {
            if cf.date < base_date {
                continue;
            }
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
        let mut call_map = HashMap::default();
        let mut put_map = HashMap::default();

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
            credit_spread,
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

/// Implementation of Tsiveriotis-Zhang tree pricing logic
struct TsiveriotisZhangEngine<'a> {
    valuator: &'a ConvertibleBondValuator,
    steps: usize,
    time_to_maturity: f64,
}

impl<'a> TsiveriotisZhangEngine<'a> {
    fn price(
        &self,
        initial_vars: StateVariables,
        tree_type: ConvertibleTreeType,
    ) -> Result<(f64, f64)> {
        // Extract parameters
        let spot = *initial_vars
            .get("spot")
            .ok_or(Error::Input(InputError::NotFound {
                id: "spot price".to_string(),
            }))?;
        let volatility =
            *initial_vars
                .get("volatility")
                .ok_or(Error::Input(InputError::NotFound {
                    id: "volatility".to_string(),
                }))?;
        let risk_free_rate =
            *initial_vars
                .get("interest_rate")
                .ok_or(Error::Input(InputError::NotFound {
                    id: "interest_rate".to_string(),
                }))?;
        let dividend_yield =
            *initial_vars
                .get("dividend_yield")
                .ok_or(Error::Input(InputError::NotFound {
                    id: "dividend_yield".to_string(),
                }))?;

        let dt = self.time_to_maturity / self.steps as f64;

        // Use EvolutionParams to get tree factors
        // Note: We use CRR for Binomial, standard for Trinomial
        // For TZ, we need separate discount factors for risk-free and risky rates
        let df_rf = (-risk_free_rate * dt).exp();
        let df_risky = (-(risk_free_rate + self.valuator.credit_spread) * dt).exp();

        // Evolution parameters
        let params = match tree_type {
            ConvertibleTreeType::Binomial(_) => {
                EvolutionParams::equity_crr(volatility, risk_free_rate, dividend_yield, dt)
            }
            ConvertibleTreeType::Trinomial(_) => {
                EvolutionParams::equity_trinomial(volatility, risk_free_rate, dividend_yield, dt)
            }
        };

        // State tracking: (Total Value, Cash Component)
        // Cash Component is the value of the liability if it were not convertible.
        // It is subject to credit risk.

        // Initialize terminal nodes
        let mut values: Vec<(f64, f64)> = Vec::with_capacity(2 * self.steps + 1);

        // Helper to get spot at (step, node)
        let get_spot = |step: usize, node: usize| -> f64 {
            match tree_type {
                ConvertibleTreeType::Binomial(_) => {
                    let ups = node as i32;
                    let downs = step as i32 - node as i32;
                    spot * params.up_factor.powi(ups) * params.down_factor.powi(downs)
                }
                ConvertibleTreeType::Trinomial(_) => {
                    let net_moves = node as i32 - step as i32;
                    spot * params.up_factor.powi(net_moves.max(0))
                        * params.down_factor.powi((-net_moves).max(0))
                }
            }
        };

        // 1. Terminal Step
        let num_nodes = match tree_type {
            ConvertibleTreeType::Binomial(n) => n + 1,
            ConvertibleTreeType::Trinomial(n) => 2 * n + 1,
        };

        for i in 0..num_nodes {
            let node_spot = get_spot(self.steps, i);

            // At maturity:
            // Conversion Value = Ratio * Spot
            let conversion_val = node_spot * self.valuator.conversion_ratio;

            // Redemption Value = Face + Coupon
            let coupon = self
                .valuator
                .coupon_map
                .get(&self.steps)
                .copied()
                .unwrap_or(0.0);
            let redemption_val = self.valuator.face_value + coupon;

            // Decision: Max(Conversion, Redemption)
            // If Converted: Cash Component = 0 (it's all equity)
            // If Redeemed: Cash Component = Redemption Value (it's all debt)

            let (total_val, cash_val) = if conversion_val > redemption_val {
                (conversion_val, 0.0)
            } else {
                (redemption_val, redemption_val)
            };

            values.push((total_val, cash_val));
        }

        // 2. Backward Induction
        for step in (0..self.steps).rev() {
            let _next_num_nodes = values.len();
            // In binomial: step N has N+1 nodes. step N-1 has N nodes.
            // In trinomial: step N has 2N+1 nodes. step N-1 has 2N-1 nodes.

            let current_num_nodes = match tree_type {
                ConvertibleTreeType::Binomial(_) => step + 1,
                ConvertibleTreeType::Trinomial(_) => 2 * step + 1,
            };

            let mut next_values = Vec::with_capacity(current_num_nodes);

            for i in 0..current_num_nodes {
                // Calculate expected values
                let (exp_total, exp_cash) = match tree_type {
                    ConvertibleTreeType::Binomial(_) => {
                        let (v_up, c_up) = values[i + 1];
                        let (v_down, c_down) = values[i];

                        (
                            params.prob_up * v_up + params.prob_down * v_down,
                            params.prob_up * c_up + params.prob_down * c_down,
                        )
                    }
                    ConvertibleTreeType::Trinomial(_) => {
                        // Child indices: up=i+2, mid=i+1, down=i
                        let (v_up, c_up) = values[i + 2];
                        let (v_mid, c_mid) = values[i + 1];
                        let (v_down, c_down) = values[i];

                        let pm = params.prob_middle.unwrap_or(0.0);
                        (
                            params.prob_up * v_up + pm * v_mid + params.prob_down * v_down,
                            params.prob_up * c_up + pm * c_mid + params.prob_down * c_down,
                        )
                    }
                };

                // Discounting (The TZ Magic)
                // Cash component discounted at risky rate
                // Equity component (Total - Cash) discounted at risk-free rate

                let equity_part = (exp_total - exp_cash) * df_rf;
                let cash_part = exp_cash * df_risky;
                let mut continuation_total = equity_part + cash_part;
                let mut continuation_cash = cash_part;

                // Add coupons at this node
                let coupon = self.valuator.coupon_map.get(&step).copied().unwrap_or(0.0);
                continuation_total += coupon;
                continuation_cash += coupon;

                // Node decision logic
                let node_spot = get_spot(step, i);

                // 1. Conversion
                let conversion_val = node_spot * self.valuator.conversion_ratio;

                // Check conversion allowed
                let can_convert = self.valuator.conversion_allowed(step);

                let mut final_total = continuation_total;
                let mut final_cash = continuation_cash;

                if can_convert && conversion_val > final_total {
                    final_total = conversion_val;
                    final_cash = 0.0; // Converted to equity
                }

                // 2. Call (Issuer minimizes value)
                if let Some(call_price) = self.valuator.call_price_at_step(step) {
                    if final_total > call_price {
                        // Issuer calls
                        final_total = call_price;
                        // If called, it's redeemed in cash

                        let val_if_called = if can_convert {
                            conversion_val.max(call_price)
                        } else {
                            call_price
                        };

                        if final_total > val_if_called {
                            final_total = val_if_called;
                            if final_total == conversion_val {
                                final_cash = 0.0;
                            } else {
                                final_cash = final_total; // Redeemed in cash
                            }
                        }
                    }
                }

                // 3. Put (Holder maximizes value)
                if let Some(put_price) = self.valuator.put_price_at_step(step) {
                    if final_total < put_price {
                        final_total = put_price;
                        final_cash = final_total; // Put back for cash
                    }
                }

                next_values.push((final_total, final_cash));
            }
            values = next_values;
        }

        Ok(values[0])
    }

    // Greeks calculation helper
    fn calculate_greeks(
        &self,
        initial_vars: StateVariables,
        tree_type: ConvertibleTreeType,
        bump_size: Option<f64>,
    ) -> Result<TreeGreeks> {
        let bump = bump_size.unwrap_or(0.01);

        // Base price
        let (base_price, _) = self.price(initial_vars.clone(), tree_type)?;

        let mut greeks = TreeGreeks {
            price: base_price,
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
        };

        if let Some(&spot) = initial_vars.get("spot") {
            let h = bump * spot;

            // Up
            let mut vars_up = initial_vars.clone();
            vars_up.insert("spot", spot + h);
            let (price_up, _) = self.price(vars_up, tree_type)?;

            // Down
            let mut vars_down = initial_vars.clone();
            vars_down.insert("spot", spot - h);
            let (price_down, _) = self.price(vars_down, tree_type)?;

            greeks.delta = (price_up - price_down) / (2.0 * h);
            greeks.gamma = (price_up - 2.0 * base_price + price_down) / (h * h);
        }

        // Vega
        if let Some(&vol) = initial_vars.get("volatility") {
            let h = 0.01;
            let mut vars_vol_up = initial_vars.clone();
            vars_vol_up.insert("volatility", vol + h);
            let (price_vol_up, _) = self.price(vars_vol_up, tree_type)?;
            greeks.vega = price_vol_up - base_price;
        }

        // Rho
        if let Some(&rate) = initial_vars.get("interest_rate") {
            let h = 0.0001;
            let mut vars_rate_up = initial_vars.clone();
            vars_rate_up.insert("interest_rate", rate + h);
            let (price_rate_up, _) = self.price(vars_rate_up, tree_type)?;
            greeks.rho = price_rate_up - base_price;
        }

        // Theta
        let dt_bump = 1.0 / 365.25;
        if self.time_to_maturity > dt_bump {
            // Skip theta for now
        }

        Ok(greeks)
    }
}

/// Extract equity market state from market context
fn extract_equity_state(
    bond: &ConvertibleBond,
    ctx: &MarketContext,
    as_of: Date,
) -> Result<(f64, f64, f64, f64, f64, f64)> {
    let underlying_id = bond
        .underlying_equity_id
        .as_deref()
        .ok_or(Error::Internal)?;

    // Get spot price
    let spot_price = ctx.price(underlying_id)?;
    let spot = match spot_price {
        finstack_core::market_data::scalars::MarketScalar::Price(money) => {
            // Enforce currency safety
            if money.currency() != bond.notional.currency() {
                return Err(Error::Internal);
            }
            money.amount()
        }
        finstack_core::market_data::scalars::MarketScalar::Unitless(value) => *value,
    };

    // Get risk-free rate from discount curve
    let discount_curve = ctx.get_discount_ref(bond.discount_curve_id.as_str())?;

    // Calculate time to maturity using the provided as_of date
    let time_to_maturity = finstack_core::dates::DayCount::Act365F
        .year_fraction(
            as_of,
            bond.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap_or(0.0);

    // Extract instantaneous-equivalent risk-free rate
    let risk_free_rate = if time_to_maturity > 0.0 {
        -discount_curve.df(time_to_maturity).ln() / time_to_maturity
    } else {
        0.0
    };

    // Extract credit spread
    let credit_spread = if let Some(credit_id) = &bond.credit_curve_id {
        if credit_id == &bond.discount_curve_id {
            0.0
        } else {
            let credit_curve = ctx.get_discount_ref(credit_id.as_str())?;
            let risky_rate = if time_to_maturity > 0.0 {
                -credit_curve.df(time_to_maturity).ln() / time_to_maturity
            } else {
                0.0
            };
            risky_rate - risk_free_rate
        }
    } else {
        0.0
    };

    // Resolve volatility
    let mut vol_candidates: Vec<String> = Vec::new();
    if let Some(id) = bond.attributes.get_meta("vol_surface_id") {
        vol_candidates.push(id.to_string());
    }
    vol_candidates.push(format!("{}-VOL", underlying_id));
    if let Some(stripped) = underlying_id.strip_suffix("-SPOT") {
        vol_candidates.push(format!("{}-VOL", stripped));
    }
    let volatility = resolve_volatility(ctx, &vol_candidates, time_to_maturity, spot)?;

    // Resolve dividend yield
    let mut dividend_candidates: Vec<String> = Vec::new();
    if let Some(id) = bond.attributes.get_meta("div_yield_id") {
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
        credit_spread,
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

/// Aggregated data required for tree pricing
struct PricingInputs {
    cashflow_schedule: CashFlowSchedule,
    spot: f64,
    volatility: f64,
    dividend_yield: f64,
    risk_free_rate: f64,
    credit_spread: f64,
    time_to_maturity: f64,
}

/// Prepare all necessary inputs for pricing and greeks calculation.
fn prepare_for_pricing(
    bond: &ConvertibleBond,
    market_context: &MarketContext,
    as_of: Date,
) -> Result<PricingInputs> {
    let cashflow_schedule = build_convertible_schedule(bond)?;
    let (spot, volatility, dividend_yield, risk_free_rate, credit_spread, time_to_maturity) =
        extract_equity_state(bond, market_context, as_of)?;

    Ok(PricingInputs {
        cashflow_schedule,
        spot,
        volatility,
        dividend_yield,
        risk_free_rate,
        credit_spread,
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
    if as_of > bond.maturity {
        return Ok(Money::new(0.0, bond.notional.currency()));
    }

    // Step 1: Prepare all inputs
    let inputs = prepare_for_pricing(bond, market_context, as_of)?;

    if inputs.time_to_maturity <= 0.0 {
        let conversion_ratio = if let Some(ratio) = bond.conversion.ratio {
            ratio
        } else if let Some(price) = bond.conversion.price {
            bond.notional.amount() / price
        } else {
            return Err(Error::Internal);
        };

        let maturity_coupon: f64 = inputs
            .cashflow_schedule
            .coupons()
            .filter(|cf| cf.date == bond.maturity)
            .map(|cf| cf.amount.amount())
            .sum();

        let redemption_value = bond.notional.amount() + maturity_coupon;
        let conversion_value = inputs.spot * conversion_ratio;
        let payoff = redemption_value.max(conversion_value);

        return Ok(Money::new(payoff, bond.notional.currency()));
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
        inputs.credit_spread,
    )?;

    // Step 3: Create initial state variables
    let initial_vars = single_factor_equity_state(
        inputs.spot,
        inputs.risk_free_rate,
        inputs.dividend_yield,
        inputs.volatility,
    );

    // Step 4: Price using Tsiveriotis-Zhang Engine
    let engine = TsiveriotisZhangEngine {
        valuator: &valuator,
        steps,
        time_to_maturity: inputs.time_to_maturity,
    };

    let (pv_amount, _) = engine.price(initial_vars, tree_type)?;

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
        inputs.credit_spread,
    )?;

    let initial_vars = single_factor_equity_state(
        inputs.spot,
        inputs.risk_free_rate,
        inputs.dividend_yield,
        inputs.volatility,
    );

    // Calculate Greeks using TZ Engine
    let engine = TsiveriotisZhangEngine {
        valuator: &valuator,
        steps,
        time_to_maturity: inputs.time_to_maturity,
    };

    let mut greeks = engine.calculate_greeks(initial_vars, tree_type, bump_size)?;

    // Finite-difference theta using a 1-day roll of the valuation date.
    let dt_bump = 1.0 / 365.25;
    if inputs.time_to_maturity > dt_bump {
        if let Some(next_day) = as_of.next_day() {
            let fwd_price = price_convertible_bond(bond, market_context, tree_type, next_day)?;
            greeks.theta = (fwd_price.amount() - greeks.price) / dt_bump;
        }
    }

    Ok(greeks)
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
    use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
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
            freq: Tenor::semi_annual(),
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
            credit_curve_id: None,
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
    fn test_convertible_pricing_at_maturity_uses_payoff() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();
        let as_of = bond.maturity;

        let price = price_convertible_bond(
            &bond,
            &market_context,
            ConvertibleTreeType::Binomial(10),
            as_of,
        )
        .expect("should price");

        // At maturity, value should match the greater of redemption or conversion.
        let conversion_value = 150.0 * 10.0;
        assert!((price.amount() - conversion_value).abs() < 1e-6);
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

        // Gamma should be positive (or close to zero if deep ITM)
        assert!(greeks.gamma >= -1e-6);

        // Price should be reasonable
        assert!(greeks.price > 1000.0);
    }
}
