// Moved from bond/oas_pricer.rs without changes other than path adjustments.
// See original for detailed comments.

#![allow(clippy::module_inception)]

use super::super::types::Bond;

#[cfg(test)]
use super::super::types::CallPut;
use crate::cashflow::traits::CashflowProvider;
#[cfg(test)]
use crate::instruments::PricingOverrides;
use crate::instruments::options::models::{
    short_rate_keys, NodeState, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
    TreeValuator,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::{Result, F};
use std::collections::HashMap;

#[cfg(test)]
use finstack_core::money::Money;

#[derive(Clone, Debug)]
pub struct OASPricerConfig {
    pub tree_steps: usize,
    pub volatility: F,
    pub tolerance: F,
    pub max_iterations: usize,
}

impl Default for OASPricerConfig {
    fn default() -> Self {
        Self { tree_steps: 100, volatility: 0.01, tolerance: 1e-6, max_iterations: 50 }
    }
}

pub struct BondValuator {
    bond: Bond,
    coupon_map: HashMap<usize, F>,
    call_map: HashMap<usize, F>,
    put_map: HashMap<usize, F>,
    time_steps: Vec<F>,
}

impl BondValuator {
    pub fn new(bond: Bond, market_context: &MarketContext, time_to_maturity: F, tree_steps: usize) -> Result<Self> {
        let dt = time_to_maturity / tree_steps as F;
        let time_steps: Vec<F> = (0..=tree_steps).map(|i| i as F * dt).collect();

        let curves = market_context;
        let base_date = market_context
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                bond.disc_id.clone(),
            )?
            .base_date();
        let flows = bond.build_schedule(curves, base_date)?;

        let mut coupon_map = HashMap::new();
        for (date, amount) in &flows {
            if *date > base_date {
                let time_frac = bond
                    .dc
                    .year_fraction(base_date, *date, finstack_core::dates::DayCountCtx::default())
                    .unwrap_or(0.0);
                let step = ((time_frac / time_to_maturity) * tree_steps as F).round() as usize;
                if step <= tree_steps {
                    *coupon_map.entry(step).or_insert(0.0) += amount.amount();
                }
            }
        }

        let mut call_map = HashMap::new();
        let mut put_map = HashMap::new();
        if let Some(ref call_put) = bond.call_put {
            for call in &call_put.calls {
                if call.date > base_date && call.date <= bond.maturity {
                    let time_frac = bond
                        .dc
                        .year_fraction(base_date, call.date, finstack_core::dates::DayCountCtx::default())
                        .unwrap_or(0.0);
                    let step = ((time_frac / time_to_maturity) * tree_steps as F).round() as usize;
                    if step <= tree_steps {
                        let call_price = bond.notional.amount() * (call.price_pct_of_par / 100.0);
                        call_map.insert(step, call_price);
                    }
                }
            }
            for put in &call_put.puts {
                if put.date > base_date && put.date <= bond.maturity {
                    let time_frac = bond
                        .dc
                        .year_fraction(base_date, put.date, finstack_core::dates::DayCountCtx::default())
                        .unwrap_or(0.0);
                    let step = ((time_frac / time_to_maturity) * tree_steps as F).round() as usize;
                    if step <= tree_steps {
                        let put_price = bond.notional.amount() * (put.price_pct_of_par / 100.0);
                        put_map.insert(step, put_price);
                    }
                }
            }
        }

        Ok(Self { bond, coupon_map, call_map, put_map, time_steps })
    }
}

impl TreeValuator for BondValuator {
    fn value_at_maturity(&self, _state: &NodeState) -> Result<F> {
        let final_step = self.time_steps.len() - 1;
        let coupon = self.coupon_map.get(&final_step).copied().unwrap_or(0.0);
        let face_value = self.bond.notional.amount();
        Ok(face_value + coupon)
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: F) -> Result<F> {
        let step = state.step;
        let coupon = self.coupon_map.get(&step).copied().unwrap_or(0.0);
        let mut value = continuation_value + coupon;
        if let Some(&put_price) = self.put_map.get(&step) { value = value.max(put_price); }
        if let Some(&call_price) = self.call_map.get(&step) { value = value.min(call_price); }
        Ok(value)
    }
}

pub struct OASCalculator {
    config: OASPricerConfig,
}

impl OASCalculator {
    pub fn new() -> Self { Self { config: OASPricerConfig::default() } }
    pub fn with_config(config: OASPricerConfig) -> Self { Self { config } }

    pub fn calculate_oas(&self, bond: &Bond, market_context: &MarketContext, as_of: Date, market_price: F) -> Result<F> {
        let accrued = self.calculate_accrued_interest(bond, market_context, as_of)?;
        let dirty_price_pct = market_price + accrued;
        let dirty_target = dirty_price_pct * bond.notional.amount() / 100.0;
        let time_to_maturity = bond
            .dc
            .year_fraction(as_of, bond.maturity, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        if time_to_maturity <= 0.0 { return Ok(0.0); }

        let tree_config = ShortRateTreeConfig { steps: self.config.tree_steps, volatility: self.config.volatility, ..Default::default() };
        let mut tree = ShortRateTree::new(tree_config);
        let discount_curve = market_context
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                bond.disc_id.clone(),
            )?;
        tree.calibrate(discount_curve.as_ref(), time_to_maturity)?;

        let valuator = BondValuator::new(bond.clone(), market_context, time_to_maturity, self.config.tree_steps)?;

        let objective_fn = |oas: F| -> F {
            let mut vars = StateVariables::new();
            vars.insert(short_rate_keys::OAS, oas);
            match tree.price(vars, time_to_maturity, market_context, &valuator) {
                Ok(model_price) => model_price - dirty_target,
                Err(_) => if oas > 0.0 { 1000000.0 } else { -1000000.0 },
            }
        };

        let solver = BrentSolver::new().with_tolerance(self.config.tolerance).with_initial_bracket_size(Some(1000.0));
        let initial_guess = 0.0;
        let oas_bp = solver.solve(objective_fn, initial_guess)?;
        Ok(oas_bp)
    }

    fn calculate_accrued_interest(&self, bond: &Bond, _market_context: &MarketContext, as_of: Date) -> Result<F> {
        if let Some(ref custom) = bond.custom_cashflows {
            let mut coupon_dates = Vec::new();
            for cf in &custom.flows {
                if matches!(cf.kind, crate::cashflow::primitives::CFKind::Fixed | crate::cashflow::primitives::CFKind::Stub) {
                    coupon_dates.push((cf.date, cf.amount));
                }
            }
            if coupon_dates.len() < 2 { return Ok(0.0); }
            for window in coupon_dates.windows(2) {
                let (start_date, _) = window[0];
                let (end_date, coupon_amount) = window[1];
                if start_date <= as_of && as_of < end_date {
                    let total_period = bond.dc.year_fraction(start_date, end_date, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
                    let elapsed = bond.dc.year_fraction(start_date, as_of, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0).max(0.0);
                    if total_period > 0.0 { return Ok(coupon_amount.amount() * (elapsed / total_period)); }
                }
            }
        } else {
            let sched = crate::cashflow::builder::build_dates(
                bond.issue, bond.maturity, bond.freq, finstack_core::dates::StubKind::None, finstack_core::dates::BusinessDayConvention::Following, None,
            );
            for window in sched.dates.windows(2) {
                let start_date = window[0];
                let end_date = window[1];
                if start_date <= as_of && as_of < end_date {
                    let yf = bond.dc.year_fraction(start_date, end_date, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
                    let period_coupon = bond.notional.amount() * bond.coupon * yf;
                    let elapsed = bond.dc.year_fraction(start_date, as_of, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0).max(0.0);
                    if yf > 0.0 { return Ok(period_coupon * (elapsed / yf)); }
                }
            }
        }
        Ok(0.0)
    }
}

impl Default for OASCalculator { fn default() -> Self { Self::new() } }

pub fn calculate_oas(bond: &Bond, market_context: &MarketContext, as_of: Date, clean_price: F) -> Result<F> {
    let calculator = OASCalculator::new();
    calculator.calculate_oas(bond, market_context, as_of, clean_price)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::bond::CallPutSchedule;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;
    fn create_test_bond() -> Bond {
        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
        Bond { id: "TEST_BOND".to_string().into(), notional: Money::new(1000.0, finstack_core::currency::Currency::USD), coupon: 0.05, freq: finstack_core::dates::Frequency::semi_annual(), dc: finstack_core::dates::DayCount::Act365F, issue, maturity, disc_id: "USD-OIS".into(), pricing_overrides: PricingOverrides::default().with_clean_price(98.5), call_put: None, amortization: None, custom_cashflows: None, attributes: Default::default() }
    }
    fn create_callable_bond() -> Bond { let mut bond = create_test_bond(); let call_date = Date::from_calendar_date(2027, Month::January, 1).unwrap(); let mut call_put = CallPutSchedule::default(); call_put.calls.push(CallPut { date: call_date, price_pct_of_par: 102.0 }); bond.call_put = Some(call_put); bond }
    fn create_test_market_context() -> MarketContext { let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap(); let discount_curve = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder("USD-OIS").base_date(base_date).knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.85), (10.0, 0.70)]).set_interp(InterpStyle::LogLinear).build().unwrap(); MarketContext::new().insert_discount(discount_curve) }
    #[test] fn test_bond_valuator_creation() { let bond = create_test_bond(); let market_context = create_test_market_context(); let valuator = BondValuator::new(bond, &market_context, 5.0, 50); assert!(valuator.is_ok()); let valuator = valuator.unwrap(); assert!(!valuator.coupon_map.is_empty()); assert!(market_context.get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>("USD-OIS").is_ok()); }
    #[test] fn test_oas_calculator_plain_bond() { let bond = create_test_bond(); let market_context = create_test_market_context(); let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap(); let calculator = OASCalculator::new(); let oas = calculator.calculate_oas(&bond, &market_context, as_of, 98.5); assert!(oas.is_ok()); let oas_bp = oas.unwrap(); assert!(oas_bp > 0.0); assert!(oas_bp < 5000.0); }
    #[test] fn test_oas_calculator_callable_bond() { let bond = create_callable_bond(); let market_context = create_test_market_context(); let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap(); let calculator = OASCalculator::new(); let oas = calculator.calculate_oas(&bond, &market_context, as_of, 98.5); assert!(oas.is_ok()); let oas_bp = oas.unwrap(); assert!(oas_bp > 0.0); }
    #[test] fn test_bond_valuator_with_calls() { let bond = create_callable_bond(); let market_context = create_test_market_context(); let valuator = BondValuator::new(bond, &market_context, 5.0, 50).unwrap(); assert!(!valuator.call_map.is_empty()); assert!(valuator.put_map.is_empty()); }
    #[test] fn test_accrued_interest_calculation() { let bond = create_test_bond(); let market_context = create_test_market_context(); let calculator = OASCalculator::new(); let coupon_date = Date::from_calendar_date(2025, Month::July, 1).unwrap(); let accrued = calculator.calculate_accrued_interest(&bond, &market_context, coupon_date).unwrap(); assert!(accrued.abs() < 1e-6); let mid_period = Date::from_calendar_date(2025, Month::April, 1).unwrap(); let accrued_mid = calculator.calculate_accrued_interest(&bond, &market_context, mid_period).unwrap(); assert!(accrued_mid > 0.0); }
}


