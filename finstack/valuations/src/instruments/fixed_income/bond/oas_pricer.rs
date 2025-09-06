//! Option-Adjusted Spread (OAS) pricer for bonds with embedded options.
//!
//! Implements industry-standard OAS calculation using short-rate trees and
//! the tree framework for valuing callable/putable bonds.

use super::Bond;

#[cfg(test)]
use super::CallPut;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::options::models::{
    short_rate_keys, NodeState, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
    TreeValuator,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::root_finding::brent;
use finstack_core::{Error, Result, F};
use std::collections::HashMap;

#[cfg(test)]
use finstack_core::money::Money;

/// Configuration for OAS pricing
#[derive(Clone, Debug)]
pub struct OASPricerConfig {
    /// Number of tree steps
    pub tree_steps: usize,
    /// Interest rate volatility (annualized)
    pub volatility: F,
    /// Solver tolerance
    pub tolerance: F,
    /// Maximum solver iterations
    pub max_iterations: usize,
}

impl Default for OASPricerConfig {
    fn default() -> Self {
        Self {
            tree_steps: 100,
            volatility: 0.01, // 1% default volatility
            tolerance: 1e-6,
            max_iterations: 50,
        }
    }
}

/// Bond valuator implementing TreeValuator for the short-rate tree framework
pub struct BondValuator {
    /// Bond being valued
    bond: Bond,
    /// Coupon amounts mapped to tree steps
    coupon_map: HashMap<usize, F>,
    /// Call schedule mapped to tree steps
    call_map: HashMap<usize, F>,
    /// Put schedule mapped to tree steps
    put_map: HashMap<usize, F>,
    /// Time steps in years
    time_steps: Vec<F>,
}

impl BondValuator {
    /// Create a new bond valuator
    pub fn new(
        bond: Bond,
        market_context: &MarketContext,
        time_to_maturity: F,
        tree_steps: usize,
    ) -> Result<Self> {
        let dt = time_to_maturity / tree_steps as F;
        let time_steps: Vec<F> = (0..=tree_steps).map(|i| i as F * dt).collect();

        // Build cashflow schedule
        let curves = market_context; // Use MarketContext directly
        let base_date = market_context.disc(bond.disc_id)?.base_date();
        let flows = bond.build_schedule(curves, base_date)?;

        // Map cashflows to tree steps
        let mut coupon_map = HashMap::new();
        for (date, amount) in &flows {
            if *date > base_date {
                let time_frac = bond
                    .dc
                    .year_fraction(
                        base_date,
                        *date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                let step = ((time_frac / time_to_maturity) * tree_steps as F).round() as usize;
                if step <= tree_steps {
                    *coupon_map.entry(step).or_insert(0.0) += amount.amount();
                }
            }
        }

        // Map call/put schedules to tree steps
        let mut call_map = HashMap::new();
        let mut put_map = HashMap::new();

        if let Some(ref call_put) = bond.call_put {
            // Map call schedule
            for call in &call_put.calls {
                if call.date > base_date && call.date <= bond.maturity {
                    let time_frac = bond
                        .dc
                        .year_fraction(
                            base_date,
                            call.date,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0);
                    let step = ((time_frac / time_to_maturity) * tree_steps as F).round() as usize;
                    if step <= tree_steps {
                        let call_price = bond.notional.amount() * (call.price_pct_of_par / 100.0);
                        call_map.insert(step, call_price);
                    }
                }
            }

            // Map put schedule
            for put in &call_put.puts {
                if put.date > base_date && put.date <= bond.maturity {
                    let time_frac = bond
                        .dc
                        .year_fraction(
                            base_date,
                            put.date,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0);
                    let step = ((time_frac / time_to_maturity) * tree_steps as F).round() as usize;
                    if step <= tree_steps {
                        let put_price = bond.notional.amount() * (put.price_pct_of_par / 100.0);
                        put_map.insert(step, put_price);
                    }
                }
            }
        }

        Ok(Self {
            bond,
            coupon_map,
            call_map,
            put_map,
            time_steps,
        })
    }
}

impl TreeValuator for BondValuator {
    fn value_at_maturity(&self, _state: &NodeState) -> Result<F> {
        // At maturity, bond pays face value plus any final coupon
        let final_step = self.time_steps.len() - 1;
        let coupon = self.coupon_map.get(&final_step).copied().unwrap_or(0.0);
        let face_value = self.bond.notional.amount();

        Ok(face_value + coupon)
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: F) -> Result<F> {
        let step = state.step;

        // Add any coupon payment at this step
        let coupon = self.coupon_map.get(&step).copied().unwrap_or(0.0);
        let mut value = continuation_value + coupon;

        // Apply put option (holder can force redemption - max operation)
        if let Some(&put_price) = self.put_map.get(&step) {
            value = value.max(put_price);
        }

        // Apply call option (issuer can force redemption - min operation)
        if let Some(&call_price) = self.call_map.get(&step) {
            value = value.min(call_price);
        }

        Ok(value)
    }
}

/// OAS calculator using short-rate trees
pub struct OASCalculator {
    config: OASPricerConfig,
}

impl OASCalculator {
    /// Create new OAS calculator
    pub fn new() -> Self {
        Self {
            config: OASPricerConfig::default(),
        }
    }

    /// Create calculator with custom config
    pub fn with_config(config: OASPricerConfig) -> Self {
        Self { config }
    }

    /// Calculate OAS for a bond given market price
    pub fn calculate_oas(
        &self,
        bond: &Bond,
        market_context: &MarketContext,
        as_of: Date,
        market_price: F, // Clean price
    ) -> Result<F> {
        // Get accrued interest to calculate dirty price target
        let accrued = self.calculate_accrued_interest(bond, market_context, as_of)?;
        let dirty_price_pct = market_price + accrued;

        // Convert percentage price to notional terms (model returns notional amounts)
        let dirty_target = dirty_price_pct * bond.notional.amount() / 100.0;

        // Calculate time to maturity
        let time_to_maturity = bond
            .dc
            .year_fraction(
                as_of,
                bond.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        if time_to_maturity <= 0.0 {
            return Ok(0.0); // Bond has matured
        }

        // Create and calibrate short-rate tree
        let tree_config = ShortRateTreeConfig {
            steps: self.config.tree_steps,
            volatility: self.config.volatility,
            ..Default::default()
        };

        let mut tree = ShortRateTree::new(tree_config);
        let discount_curve = market_context.disc(bond.disc_id)?;
        tree.calibrate(discount_curve.as_ref(), time_to_maturity)?;

        // Create bond valuator
        let valuator = BondValuator::new(
            bond.clone(),
            market_context,
            time_to_maturity,
            self.config.tree_steps,
        )?;

        // Define objective function for OAS solver
        let objective_fn = |oas: F| -> F {
            // Create state variables with OAS
            let mut vars = StateVariables::new();
            vars.insert(short_rate_keys::OAS, oas);

            // Price bond with this OAS
            match tree.price(vars, time_to_maturity, market_context, &valuator) {
                Ok(model_price) => model_price - dirty_target,
                Err(_) => {
                    // If pricing fails, return a large number to guide solver away
                    if oas > 0.0 {
                        1000000.0
                    } else {
                        -1000000.0
                    }
                }
            }
        };

        // Test objective function at endpoints before calling Brent
        let f_low = objective_fn(-500.0);
        let f_high = objective_fn(2000.0);

        // Check if there's a sign change (required for Brent)
        if f_low * f_high > 0.0 {
            // Try wider range
            let f_very_low = objective_fn(-2000.0);
            let f_very_high = objective_fn(5000.0);

            if f_very_low * f_very_high > 0.0 {
                return Err(Error::Internal);
            }

            // Use expanded range for Brent
            return brent(
                objective_fn,
                -2000.0,
                5000.0,
                self.config.tolerance,
                self.config.max_iterations,
            );
        }

        // Solve for OAS using Brent's method
        // Typical OAS range is -500bp to +2000bp
        let oas_bp = brent(
            objective_fn,
            -500.0, // -5% in bps
            2000.0, // +20% in bps
            self.config.tolerance,
            self.config.max_iterations,
        )?;

        Ok(oas_bp)
    }

    /// Calculate accrued interest for the bond
    fn calculate_accrued_interest(
        &self,
        bond: &Bond,
        _market_context: &MarketContext,
        as_of: Date,
    ) -> Result<F> {
        // Use the existing accrued interest calculation logic
        // This is a simplified version - in practice would use the full metric calculator

        if let Some(ref custom) = bond.custom_cashflows {
            // Use coupon flows from custom schedule
            let mut coupon_dates = Vec::new();
            for cf in &custom.flows {
                if matches!(
                    cf.kind,
                    crate::cashflow::primitives::CFKind::Fixed
                        | crate::cashflow::primitives::CFKind::Stub
                ) {
                    coupon_dates.push((cf.date, cf.amount));
                }
            }

            if coupon_dates.len() < 2 {
                return Ok(0.0);
            }

            // Find accrual period
            for window in coupon_dates.windows(2) {
                let (start_date, _) = window[0];
                let (end_date, coupon_amount) = window[1];

                if start_date <= as_of && as_of < end_date {
                    let total_period = bond
                        .dc
                        .year_fraction(
                            start_date,
                            end_date,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0);
                    let elapsed = bond
                        .dc
                        .year_fraction(
                            start_date,
                            as_of,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0)
                        .max(0.0);

                    if total_period > 0.0 {
                        return Ok(coupon_amount.amount() * (elapsed / total_period));
                    }
                }
            }
        } else {
            // Use standard bond coupon calculation
            let sched = crate::cashflow::builder::build_dates(
                bond.issue,
                bond.maturity,
                bond.freq,
                finstack_core::dates::StubKind::None,
                finstack_core::dates::BusinessDayConvention::Following,
                None,
            );

            for window in sched.dates.windows(2) {
                let start_date = window[0];
                let end_date = window[1];

                if start_date <= as_of && as_of < end_date {
                    let yf = bond
                        .dc
                        .year_fraction(
                            start_date,
                            end_date,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0);
                    let period_coupon = bond.notional.amount() * bond.coupon * yf;
                    let elapsed = bond
                        .dc
                        .year_fraction(
                            start_date,
                            as_of,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0)
                        .max(0.0);

                    if yf > 0.0 {
                        return Ok(period_coupon * (elapsed / yf));
                    }
                }
            }
        }

        Ok(0.0)
    }
}

impl Default for OASCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to calculate OAS with default settings
pub fn calculate_oas(
    bond: &Bond,
    market_context: &MarketContext,
    as_of: Date,
    clean_price: F,
) -> Result<F> {
    let calculator = OASCalculator::new();
    calculator.calculate_oas(bond, market_context, as_of, clean_price)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::bond::CallPutSchedule;
    use finstack_core::market_data::interp::InterpStyle;
    use time::Month;

    fn create_test_bond() -> Bond {
        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        Bond {
            id: "TEST_BOND".to_string(),
            notional: Money::new(1000.0, finstack_core::currency::Currency::USD),
            coupon: 0.05, // 5% coupon
            freq: finstack_core::dates::Frequency::semi_annual(),
            dc: finstack_core::dates::DayCount::Act365F,
            issue,
            maturity,
            disc_id: "USD-OIS",
            quoted_clean: Some(98.5), // Slightly below par
            call_put: None,
            amortization: None,
            custom_cashflows: None,
            attributes: Default::default(),
        }
    }

    fn create_callable_bond() -> Bond {
        let mut bond = create_test_bond();

        // Add call schedule
        let call_date = Date::from_calendar_date(2027, Month::January, 1).unwrap();
        let mut call_put = CallPutSchedule::default();
        call_put.calls.push(CallPut {
            date: call_date,
            price_pct_of_par: 102.0, // Callable at 102% of par
        });

        bond.call_put = Some(call_put);
        bond
    }

    fn create_test_market_context() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let discount_curve =
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(
                "USD-OIS",
            )
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.85), (10.0, 0.70)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .unwrap();

        MarketContext::new().insert_discount(discount_curve)
    }

    #[test]
    fn test_bond_valuator_creation() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();

        let valuator = BondValuator::new(
            bond,
            &market_context,
            5.0, // 5 years to maturity
            50,  // 50 steps
        );

        assert!(valuator.is_ok());
        let valuator = valuator.unwrap();
        assert!(!valuator.coupon_map.is_empty());

        // Verify market context was used properly
        assert!(market_context.disc("USD-OIS").is_ok());
    }

    #[test]
    fn test_oas_calculator_plain_bond() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let calculator = OASCalculator::new();

        // For a plain bond (no options), OAS should be close to Z-spread
        let oas = calculator.calculate_oas(&bond, &market_context, as_of, 98.5);

        assert!(oas.is_ok());
        let oas_bp = oas.unwrap();

        // OAS should be reasonable (some positive spread for below-par bond)
        assert!(oas_bp > 0.0);
        assert!(oas_bp < 5000.0); // Less than 50% (very generous for below-par bond)
    }

    #[test]
    fn test_oas_calculator_callable_bond() {
        let bond = create_callable_bond();
        let market_context = create_test_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let calculator = OASCalculator::new();

        // Callable bond should have higher OAS than equivalent non-callable
        let oas = calculator.calculate_oas(&bond, &market_context, as_of, 98.5);

        assert!(oas.is_ok());
        let oas_bp = oas.unwrap();

        // Callable bond OAS should be positive (call option has negative value for holder)
        assert!(oas_bp > 0.0);
    }

    #[test]
    fn test_bond_valuator_with_calls() {
        let bond = create_callable_bond();
        let market_context = create_test_market_context();

        let valuator = BondValuator::new(bond, &market_context, 5.0, 50).unwrap();

        // Should have call options mapped
        assert!(!valuator.call_map.is_empty());
        assert!(valuator.put_map.is_empty()); // No puts in this bond
    }

    #[test]
    fn test_accrued_interest_calculation() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();
        let calculator = OASCalculator::new();

        // Test accrued interest on a coupon date (should be 0)
        let coupon_date = Date::from_calendar_date(2025, Month::July, 1).unwrap();
        let accrued = calculator
            .calculate_accrued_interest(&bond, &market_context, coupon_date)
            .unwrap();
        assert!(accrued.abs() < 1e-6); // Should be very close to 0

        // Test accrued interest halfway through period
        let mid_period = Date::from_calendar_date(2025, Month::April, 1).unwrap();
        let accrued_mid = calculator
            .calculate_accrued_interest(&bond, &market_context, mid_period)
            .unwrap();
        assert!(accrued_mid > 0.0); // Should have some accrued interest
    }
}
