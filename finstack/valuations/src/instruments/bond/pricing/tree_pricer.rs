// Moved from bond/oas_pricer.rs without changes other than path adjustments.
// See original for detailed comments.

#![allow(clippy::module_inception)]

use super::super::types::Bond;

#[cfg(test)]
use super::super::types::CallPut;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::models::trees::tree_framework::state_keys as tf_keys;
use crate::instruments::common::models::trees::two_factor_rates_credit::{
    RatesCreditConfig, RatesCreditTree,
};
use crate::instruments::common::models::{
    short_rate_keys, NodeState, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
    TreeValuator,
};
#[cfg(test)]
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;
use std::collections::HashMap;

#[cfg(test)]
use finstack_core::money::Money;

/// Configuration for tree-based bond pricing (callable/putable bonds, OAS).
#[derive(Clone, Debug)]
pub struct TreePricerConfig {
    /// Number of time steps in the interest rate tree
    pub tree_steps: usize,
    /// Short rate volatility (annualized)
    pub volatility: f64,
    /// Convergence tolerance for iterative solvers (OAS, YTM)
    pub tolerance: f64,
    /// Maximum iterations for root finding algorithms
    pub max_iterations: usize,
    /// Optional initial bracket size (in basis points) for the root solver.
    /// Defaults to 1000 bps when `None`.
    pub initial_bracket_size_bp: Option<f64>,
}

impl Default for TreePricerConfig {
    fn default() -> Self {
        Self {
            tree_steps: 100,
            volatility: 0.01,
            tolerance: 1e-6,
            max_iterations: 50,
            initial_bracket_size_bp: Some(1000.0),
        }
    }
}

/// Bond valuator for tree-based pricing of callable/putable bonds.
///
/// Implements TreeValuator trait for backward induction pricing with embedded options.
pub struct BondValuator {
    bond: Bond,
    /// Coupon amounts indexed by time step
    coupon_map: HashMap<usize, f64>,
    /// Call prices indexed by time step (if callable)
    call_map: HashMap<usize, f64>,
    /// Put prices indexed by time step (if putable)
    put_map: HashMap<usize, f64>,
    /// Time steps for tree pricing
    time_steps: Vec<f64>,
    /// Optional recovery rate sourced from a hazard curve in MarketContext
    recovery_rate: Option<f64>,
}

impl BondValuator {
    /// Create a new bond valuator for tree pricing.
    ///
    /// Builds maps of coupons, call prices, and put prices indexed by tree step.
    pub fn new(
        bond: Bond,
        market_context: &MarketContext,
        time_to_maturity: f64,
        tree_steps: usize,
    ) -> Result<Self> {
        let dt = time_to_maturity / tree_steps as f64;
        let time_steps: Vec<f64> = (0..=tree_steps).map(|i| i as f64 * dt).collect();

        let curves = market_context;
        let discount_curve = market_context.get_discount(&bond.discount_curve_id)?;
        let dc_curve = discount_curve.day_count();
        let base_date = discount_curve.base_date();
        let flows = bond.build_schedule(curves, base_date)?;

        let mut coupon_map = HashMap::new();
        for (date, amount) in &flows {
            if *date > base_date {
                let time_frac = dc_curve.year_fraction(
                    base_date,
                    *date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                // Map to the nearest forward step using ceil and clamp to [1, tree_steps]
                let raw = (time_frac / time_to_maturity) * tree_steps as f64;
                let mut step = raw.ceil() as usize;
                if step == 0 {
                    step = 1;
                }
                if step > tree_steps {
                    step = tree_steps;
                }
                *coupon_map.entry(step).or_insert(0.0) += amount.amount();
            }
        }

        let mut call_map = HashMap::new();
        let mut put_map = HashMap::new();
        if let Some(ref call_put) = bond.call_put {
            for call in &call_put.calls {
                if call.date > base_date && call.date <= bond.maturity {
                    let time_frac = dc_curve.year_fraction(
                        base_date,
                        call.date,
                        finstack_core::dates::DayCountCtx::default(),
                    )?;
                    let raw = (time_frac / time_to_maturity) * tree_steps as f64;
                    let mut step = raw.ceil() as usize;
                    if step == 0 {
                        step = 1;
                    }
                    if step > tree_steps {
                        step = tree_steps;
                    }
                    let call_price = bond.notional.amount() * (call.price_pct_of_par / 100.0);
                    call_map.insert(step, call_price);
                }
            }
            for put in &call_put.puts {
                if put.date > base_date && put.date <= bond.maturity {
                    let time_frac = dc_curve.year_fraction(
                        base_date,
                        put.date,
                        finstack_core::dates::DayCountCtx::default(),
                    )?;
                    let raw = (time_frac / time_to_maturity) * tree_steps as f64;
                    let mut step = raw.ceil() as usize;
                    if step == 0 {
                        step = 1;
                    }
                    if step > tree_steps {
                        step = tree_steps;
                    }
                    let put_price = bond.notional.amount() * (put.price_pct_of_par / 100.0);
                    put_map.insert(step, put_price);
                }
            }
        }

        // Try to source recovery rate from a hazard curve whose ID matches the bond's credit curve ID.
        // Convention: credit (hazard) curve ID == hazard curve ID. For compatibility, we also try a
        // fallback suffix of "-CREDIT".
        let mut recovery_rate: Option<f64> = None;
        if let Ok(hc) = market_context.get_hazard(bond.discount_curve_id.as_str()) {
            recovery_rate = Some(hc.recovery_rate());
        } else if let Ok(hc) =
            market_context.get_hazard(format!("{}-CREDIT", bond.discount_curve_id.as_str()))
        {
            recovery_rate = Some(hc.recovery_rate());
        }

        Ok(Self {
            bond,
            coupon_map,
            call_map,
            put_map,
            time_steps,
            recovery_rate,
        })
    }
}

impl TreeValuator for BondValuator {
    fn value_at_maturity(&self, _state: &NodeState) -> Result<f64> {
        let final_step = self.time_steps.len() - 1;
        let cashflow = self.coupon_map.get(&final_step).copied().unwrap_or(0.0);
        Ok(cashflow)
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: f64) -> Result<f64> {
        let step = state.step;
        let coupon = self.coupon_map.get(&step).copied().unwrap_or(0.0);

        // Alive (no default) value at end of the step including coupon, with call/put decisions
        let mut alive_value = continuation_value + coupon;
        if let Some(&put_price) = self.put_map.get(&step) {
            alive_value = alive_value.max(put_price);
        }
        if let Some(&call_price) = self.call_map.get(&step) {
            alive_value = alive_value.min(call_price);
        }

        // Default handling: if hazard and dt are present, compute survival/default weighting
        if let (Some(hazard), Some(dt)) = (
            state.get_var(
                super::super::super::common::models::trees::tree_framework::state_keys::HAZARD_RATE,
            ),
            state.get_var("dt"),
        ) {
            let df = state.get_var("df").unwrap_or(1.0);
            let p_surv = (-hazard.max(0.0) * dt).exp();
            let default_prob = (1.0 - p_surv).clamp(0.0, 1.0);
            let recovery = self
                .recovery_rate
                .map(|rr| rr.clamp(0.0, 1.0) * self.bond.notional.amount())
                .unwrap_or(0.0);
            let node_value = p_surv * alive_value + default_prob * df * recovery;
            Ok(node_value)
        } else {
            // No hazard info at this node; return alive path value
            Ok(alive_value)
        }
    }
}

/// Tree-based pricer for bonds with embedded options and OAS calculations.
pub struct TreePricer {
    /// Pricer configuration (tree steps, volatility, convergence settings)
    config: TreePricerConfig,
}

impl TreePricer {
    /// Create a new tree pricer with default configuration.
    pub fn new() -> Self {
        Self {
            config: TreePricerConfig::default(),
        }
    }
    
    /// Create a tree pricer with custom configuration.
    pub fn with_config(config: TreePricerConfig) -> Self {
        Self { config }
    }

    /// Calculate option-adjusted spread (OAS) for a bond.
    ///
    /// Solves for the constant spread that equates the tree price to the market price.
    pub fn calculate_oas(
        &self,
        bond: &Bond,
        market_context: &MarketContext,
        as_of: Date,
        clean_price_pct_of_par: f64,
    ) -> Result<f64> {
        // clean_price_pct_of_par is expected to be the CLEAN price quoted in percent of par.
        // Convert to currency and add accrued interest (currency) to form the dirty target.
        let accrued_ccy = self.calculate_accrued_interest(bond, market_context, as_of)?;
        let dirty_target = (clean_price_pct_of_par * bond.notional.amount() / 100.0) + accrued_ccy;
        // Choose model: if a hazard curve is present in MarketContext whose ID matches the bond's
        // discount ID (preferred) or the fallback pattern "{discount_curve_id}-CREDIT", use the rates+credit
        // two-factor tree; otherwise, fall back to short-rate.
        let mut use_rates_credit = false;
        let mut rc_tree: Option<RatesCreditTree> = None;
        let discount_curve = market_context.get_discount(&bond.discount_curve_id)?;
        // Align tree time basis with the discount curve's own day-count.
        if as_of >= bond.maturity {
            return Ok(0.0);
        }
        let dc_curve = discount_curve.day_count();
        let time_to_maturity = dc_curve.year_fraction(
            as_of,
            bond.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if time_to_maturity <= 0.0 {
            return Ok(0.0);
        }
        let hazard_curve = if let Some(hid) = bond.credit_curve_id.as_ref() {
            market_context.get_hazard(hid.as_str()).ok()
        } else {
            market_context
                .get_hazard(bond.discount_curve_id.as_str())
                .ok()
                .or_else(|| {
                    market_context
                        .get_hazard(format!("{}-CREDIT", bond.discount_curve_id.as_str()))
                        .ok()
                })
        };
        if let Some(hc) = hazard_curve.as_ref() {
            let cfg = RatesCreditConfig {
                steps: self.config.tree_steps,
                ..Default::default()
            };
            let mut tree = RatesCreditTree::new(cfg);
            let _recovery = tree.align_hazard_from_curve(hc);
            rc_tree = Some(tree);
            use_rates_credit = true;
        }

        let mut sr_tree: Option<ShortRateTree> = None;
        if !use_rates_credit {
            let tree_config = ShortRateTreeConfig {
                steps: self.config.tree_steps,
                volatility: self.config.volatility,
                ..Default::default()
            };
            let mut tree = ShortRateTree::new(tree_config);
            tree.calibrate(discount_curve.as_ref(), time_to_maturity)?;
            sr_tree = Some(tree);
        }

        let valuator = BondValuator::new(
            bond.clone(),
            market_context,
            time_to_maturity,
            self.config.tree_steps,
        )?;

        let objective_fn = |oas: f64| -> f64 {
            // `oas` is treated in basis points (bp) to match `short_rate_keys::OAS`
            // semantics in the short-rate tree. When using the rates+credit tree,
            // we convert bp → decimal and add it to the short rate passed via
            // `INTEREST_RATE`.
            let mut vars = StateVariables::new();
            if use_rates_credit {
                let base_rate = discount_curve.zero(0.0);
                let oas_bp = oas;
                let rate_with_oas = base_rate + oas_bp / 10_000.0;
                vars.insert(tf_keys::INTEREST_RATE, rate_with_oas);
                if let Some(hc) = hazard_curve.as_ref() {
                    // Use first knot hazard as base
                    if let Some((_, lambda0)) = hc.knot_points().next() {
                        vars.insert(tf_keys::HAZARD_RATE, lambda0.max(0.0));
                    } else {
                        vars.insert(tf_keys::HAZARD_RATE, 0.01);
                    }
                } else {
                    vars.insert(tf_keys::HAZARD_RATE, 0.01);
                }
                // Let valuator handle call/put; OAS is not used here (credit spread embedded via hazard)
                if let Some(tree) = rc_tree.as_ref() {
                    match tree.price(vars, time_to_maturity, market_context, &valuator) {
                        Ok(model_price) => model_price - dirty_target,
                        Err(_) => 1.0e6,
                    }
                } else {
                    1.0e6
                }
            } else {
                vars.insert(short_rate_keys::OAS, oas);
                if let Some(tree) = sr_tree.as_ref() {
                    match tree.price(vars, time_to_maturity, market_context, &valuator) {
                        Ok(model_price) => model_price - dirty_target,
                        Err(_) => {
                            if oas > 0.0 {
                                1.0e6
                            } else {
                                -1.0e6
                            }
                        }
                    }
                } else {
                    1.0e6
                }
            }
        };

        let mut solver = BrentSolver::new()
            .with_tolerance(self.config.tolerance)
            .with_initial_bracket_size(self.config.initial_bracket_size_bp);
        // Respect the configured maximum iteration cap for OAS root-finding.
        solver.max_iterations = self.config.max_iterations;
        let initial_guess = 0.0;
        let oas_bp = solver.solve(objective_fn, initial_guess)?;
        Ok(oas_bp)
    }

    fn calculate_accrued_interest(
        &self,
        bond: &Bond,
        _market_context: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        // Prefer context-aware helper for FRNs; MarketContext available here
        super::helpers::compute_accrued_interest_with_context(bond, _market_context, as_of)
    }
}

impl Default for TreePricer {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate option-adjusted spread for a bond given market price.
///
/// Convenience function using default tree configuration.
pub fn calculate_oas(
    bond: &Bond,
    market_context: &MarketContext,
    as_of: Date,
    clean_price: f64,
) -> Result<f64> {
    let calculator = TreePricer::new();
    calculator.calculate_oas(bond, market_context, as_of, clean_price)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::bond::CallPutSchedule;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;
    fn create_test_bond() -> Bond {
        use crate::instruments::bond::CashflowSpec;

        let issue = Date::from_calendar_date(2025, Month::January, 1)
            .expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1)
            .expect("Valid test date");

        Bond::builder()
            .id("TEST_BOND".into())
            .notional(Money::new(1000.0, finstack_core::currency::Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                finstack_core::dates::Frequency::semi_annual(),
                finstack_core::dates::DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default().with_clean_price(98.5))
            .call_put_opt(None)
            .custom_cashflows_opt(None)
            .attributes(Default::default())
            .settlement_days_opt(Some(2))
            .ex_coupon_days_opt(Some(0))
            .build()
            .expect("Bond builder should succeed with valid test data")
    }
    fn create_callable_bond() -> Bond {
        let mut bond = create_test_bond();
        let call_date = Date::from_calendar_date(2027, Month::January, 1)
            .expect("Valid test date");
        let mut call_put = CallPutSchedule::default();
        call_put.calls.push(CallPut {
            date: call_date,
            price_pct_of_par: 102.0,
        });
        bond.call_put = Some(call_put);
        bond
    }
    fn create_test_market_context() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1)
            .expect("Valid test date");
        let discount_curve =
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(
                "USD-OIS",
            )
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.85), (10.0, 0.70)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");
        MarketContext::new().insert_discount(discount_curve)
    }
    #[test]
    fn test_bond_valuator_creation() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();
        let valuator = BondValuator::new(bond, &market_context, 5.0, 50);
        assert!(valuator.is_ok());
        let valuator = valuator.expect("BondValuator creation should succeed in test");
        assert!(!valuator.coupon_map.is_empty());
        assert!(market_context.get_discount("USD-OIS").is_ok());
    }
    #[test]
    #[cfg(feature = "slow")]
    fn test_oas_calculator_plain_bond() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1)
            .expect("Valid test date");
        let calculator = TreePricer::new();
        let oas = calculator.calculate_oas(&bond, &market_context, as_of, 98.5);
        assert!(oas.is_ok());
        let oas_bp = oas.expect("OAS calculation should succeed in test");
        assert!(oas_bp > 0.0);
        assert!(oas_bp < 5000.0);
    }
    #[test]
    #[cfg(feature = "slow")]
    fn test_oas_calculator_callable_bond() {
        let bond = create_callable_bond();
        let market_context = create_test_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1)
            .expect("Valid test date");
        let calculator = TreePricer::new();
        let oas = calculator.calculate_oas(&bond, &market_context, as_of, 98.5);
        assert!(oas.is_ok());
        let oas_bp = oas.expect("OAS calculation should succeed in test");
        assert!(oas_bp > 0.0);
    }
    #[test]
    fn test_bond_valuator_with_calls() {
        let bond = create_callable_bond();
        let market_context = create_test_market_context();
        let valuator = BondValuator::new(bond, &market_context, 5.0, 50)
            .expect("BondValuator creation should succeed in test");
        assert!(!valuator.call_map.is_empty());
        assert!(valuator.put_map.is_empty());
    }

    #[test]
    #[cfg(feature = "slow")]
    fn test_rates_credit_default_lowers_price() {
        use crate::instruments::common::models::trees::tree_framework::state_keys as tf_keys;
        use crate::instruments::common::models::trees::two_factor_rates_credit::{
            RatesCreditConfig, RatesCreditTree,
        };
        use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;

        let bond = create_test_bond();
        let base_date = Date::from_calendar_date(2025, Month::January, 1)
            .expect("Valid test date");

        // Curves: discount + two hazard scenarios
        let discount_curve =
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(
                "USD-OIS",
            )
            .base_date(base_date)
            .knots([(0.0, 1.0), (5.0, 0.85)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("Curve builder should succeed with valid test data");

        let low_hazard = HazardCurve::builder("HAZ-LOW")
            .base_date(base_date)
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (5.0, 0.01)])
            .build()
            .expect("Curve builder should succeed with valid test data");
        let _high_hazard = HazardCurve::builder("HAZ-HIGH")
            .base_date(base_date)
            .recovery_rate(0.4)
            .knots([(0.0, 0.05), (5.0, 0.05)])
            .build()
            .expect("Curve builder should succeed with valid test data");

        let ctx_low = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_hazard(low_hazard);
        // Recreate for high scenario to avoid cloning requirements
        let discount_curve2 =
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(
                "USD-OIS",
            )
            .base_date(base_date)
            .knots([(0.0, 1.0), (5.0, 0.85)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("Curve builder should succeed with valid test data");
        let high_hazard2 =
            finstack_core::market_data::term_structures::hazard_curve::HazardCurve::builder(
                "HAZ-HIGH",
            )
            .base_date(base_date)
            .recovery_rate(0.4)
            .knots([(0.0, 0.05), (5.0, 0.05)])
            .build()
            .expect("Curve builder should succeed with valid test data");
        let ctx_high = MarketContext::new()
            .insert_discount(discount_curve2)
            .insert_hazard(high_hazard2);

        // Time grid
        let as_of = base_date;
        let time_to_maturity = bond
            .cashflow_spec
            .day_count()
            .year_fraction(
                as_of,
                bond.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let steps = 40usize;

        // Valuator
        let valuator_low =
            BondValuator::new(bond.clone(), &ctx_low, time_to_maturity, steps).expect("valuator");
        let valuator_high =
            BondValuator::new(bond.clone(), &ctx_high, time_to_maturity, steps).expect("valuator");

        // Two-factor rates+credit trees aligned to each hazard curve
        let mut tree_low = RatesCreditTree::new(RatesCreditConfig {
            steps,
            ..Default::default()
        });
        // Align to the hazard curve stored in the context
        let low_hc_ref = ctx_low
            .get_hazard_ref("HAZ-LOW")
            .expect("Hazard curve should exist in test context");
        tree_low.align_hazard_from_curve(low_hc_ref);
        let mut tree_high = RatesCreditTree::new(RatesCreditConfig {
            steps,
            ..Default::default()
        });
        let high_hc_ref = ctx_high
            .get_hazard_ref("HAZ-HIGH")
            .expect("Hazard curve should exist in test context");
        tree_high.align_hazard_from_curve(high_hc_ref);

        // Initial state
        let mut vars = StateVariables::new();
        vars.insert(tf_keys::INTEREST_RATE, 0.03);
        vars.insert(tf_keys::HAZARD_RATE, 0.01);

        let pv_low = tree_low
            .price(vars.clone(), time_to_maturity, &ctx_low, &valuator_low)
            .expect("price low");

        // Use higher base hazard for the high scenario
        let mut vars_high = vars.clone();
        vars_high.insert(tf_keys::HAZARD_RATE, 0.05);
        let pv_high = tree_high
            .price(vars_high, time_to_maturity, &ctx_high, &valuator_high)
            .expect("price high");

        // With higher hazard, price should be lower (all else equal)
        assert!(pv_high < pv_low, "pv_high={} pv_low={}", pv_high, pv_low);
    }
    #[test]
    fn test_accrued_interest_calculation() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();
        let calculator = TreePricer::new();
        let coupon_date = Date::from_calendar_date(2025, Month::July, 1)
            .expect("Valid test date");
        let accrued = calculator
            .calculate_accrued_interest(&bond, &market_context, coupon_date)
            .expect("Accrued interest calculation should succeed in test");
        assert!(accrued.abs() < 1e-6);
        let mid_period = Date::from_calendar_date(2025, Month::April, 1)
            .expect("Valid test date");
        let accrued_mid = calculator
            .calculate_accrued_interest(&bond, &market_context, mid_period)
            .expect("Accrued interest calculation should succeed in test");
        assert!(accrued_mid > 0.0);
    }
}
