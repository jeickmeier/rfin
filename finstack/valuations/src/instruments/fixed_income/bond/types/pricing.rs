//! Bond pricing methods, validation, and cashflow projection.

use crate::instruments::common_impl::validation;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::Result;
use rust_decimal::prelude::ToPrimitive;

use super::definitions::Bond;
use super::CashflowSpec;

impl Bond {
    /// Pricing-oriented dated cashflows: coupons, amortization, and positive
    /// notional (redemption). Negative notionals (initial draw) and pure PIK
    /// accretion are excluded because they are not discounted receipt flows.
    ///
    /// Internal pricing engines (discount, hazard, spread solvers) should use
    /// this instead of the public [`CashflowProvider::dated_cashflows`] which
    /// now returns the full signed canonical schedule.
    pub(crate) fn pricing_dated_cashflows(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<Vec<(finstack_core::dates::Date, finstack_core::money::Money)>> {
        use finstack_core::cashflow::CFKind;

        let schedule = self.full_cashflow_schedule(curves)?;
        Ok(schedule
            .flows
            .into_iter()
            .filter(|cf| {
                cf.date >= as_of
                    && cf.kind != CFKind::PIK
                    && !(cf.kind == CFKind::Notional && cf.amount.amount() < 0.0)
            })
            .map(|cf| (cf.date, cf.amount))
            .collect())
    }

    /// Cashflow schedule enriched with discount factors, survival probabilities, and PVs.
    ///
    /// Builds the bond's full internal cashflow schedule
    /// and computes per-cashflow discount factors and (when a credit curve is configured)
    /// survival probabilities, returning a
    /// [`crate::cashflow::builder::PeriodDataFrame`] that is ready for tabular
    /// export or further analysis.
    ///
    /// # Arguments
    /// * `market` - Market context containing discount and optional hazard curves
    /// * `as_of` - Valuation date; defaults to the discount curve's base date when `None`
    ///
    /// # Returns
    /// A [`crate::cashflow::builder::PeriodDataFrame`] with `discount_factors`,
    /// optional `survival_probs`, and `pvs`.
    pub fn pricing_cashflows(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Option<Date>,
    ) -> Result<crate::cashflow::builder::PeriodDataFrame> {
        use crate::cashflow::builder::PeriodDataFrameOptions;
        use finstack_core::dates::{Period, PeriodId};

        let schedule = self.full_cashflow_schedule(market)?;

        let periods: Vec<Period> =
            if let (Some(first), Some(last)) = (schedule.flows.first(), schedule.flows.last()) {
                vec![Period {
                    id: PeriodId::annual(first.date.year()),
                    start: first.date,
                    end: last.date,
                    is_actual: true,
                }]
            } else {
                Vec::new()
            };

        let options = PeriodDataFrameOptions {
            credit_curve_id: self.credit_curve_id.as_ref().map(|id| id.as_str()),
            as_of,
            ..Default::default()
        };

        schedule.to_period_dataframe(&periods, market, self.discount_curve_id.as_str(), options)
    }

    /// Price bond using tree-based pricing for embedded options (calls/puts).
    ///
    /// This method is automatically called by `value()` when the bond has a non-empty
    /// call/put schedule. It uses a short-rate tree model to properly value the
    /// embedded optionality via backward induction.
    ///
    /// # Arguments
    /// * `market` - Market context with discount curve (and optionally hazard curve)
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// Option-adjusted present value of the bond
    pub(crate) fn value_with_tree(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::common_impl::models::{
            short_rate_keys, state_keys, ShortRateTree, ShortRateTreeConfig, StateVariables,
            TreeModel,
        };
        use crate::instruments::fixed_income::bond::pricing::engine::tree::{
            bond_tree_config, BondValuator,
        };

        // Calculate time to maturity from the valuation date (as_of) using the
        // discount curve's day-count convention to ensure consistency with tree calibration.
        let discount_curve = market.get_discount(&self.discount_curve_id)?;
        let time_to_maturity = discount_curve.day_count().year_fraction(
            as_of,
            self.maturity,
            finstack_core::dates::DayCountContext::default(),
        )?;

        if time_to_maturity <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        // Use centralized tree config from pricing_overrides (or defaults)
        let config = bond_tree_config(self);
        let tree_steps = config.tree_steps;
        let volatility = config.volatility;

        let tree_config = ShortRateTreeConfig {
            steps: tree_steps,
            volatility,
            mean_reversion: config.mean_reversion,
            ..Default::default()
        };

        // Initialize and calibrate short-rate tree to match discount curve
        let mut tree = ShortRateTree::new(tree_config);
        tree.calibrate(discount_curve.as_ref(), time_to_maturity)?;

        // Create bond valuator with call/put schedule mapped to tree steps
        let valuator =
            BondValuator::new(self.clone(), market, as_of, time_to_maturity, tree_steps)?;

        // Set up initial state variables (no OAS for vanilla pricing)
        let initial_rate = tree
            .rate_at_node(0, 0)
            .unwrap_or_else(|_| discount_curve.zero(0.0));
        let mut vars = StateVariables::default();
        vars.insert(state_keys::INTEREST_RATE, initial_rate);
        vars.insert(short_rate_keys::OAS, 0.0);

        // Price via tree with backward induction applying call/put constraints
        let price_amount = tree.price(vars, time_to_maturity, market, &valuator)?;

        Ok(Money::new(price_amount, self.notional.currency()))
    }

    /// Validate all bond parameters.
    ///
    /// Performs comprehensive validation of the bond instrument:
    /// - Issue date must be before maturity date
    /// - Notional must be positive
    /// - Coupon rate must be non-negative (for fixed-rate bonds)
    /// - Call/put prices must be positive
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` with a descriptive message if any validation fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let bond = Bond::fixed(...)?;
    /// bond.validate()?; // Validates all parameters
    /// ```
    pub fn validate(&self) -> Result<()> {
        // Validate date ordering
        validation::validate_date_range_strict_with(
            self.issue_date,
            self.maturity,
            |start, end| {
                format!(
                    "Bond issue date ({}) must be before maturity date ({})",
                    start, end
                )
            },
        )?;

        // Validate notional is finite and positive
        validation::validate_money_finite(self.notional, "bond notional")?;
        validation::validate_money_gt_with(self.notional, 0.0, |amount| {
            format!("Bond notional must be positive, got {}", amount)
        })?;

        // Validate coupon rate for fixed-rate bonds (including amortizing with fixed base)
        Self::validate_coupon_rate(&self.cashflow_spec)?;

        // Validate call/put prices and exercise date ranges
        if let Some(ref call_put) = self.call_put {
            for call in &call_put.calls {
                if call.price_pct_of_par <= 0.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "Bond call price must be positive, got {} on {}",
                        call.price_pct_of_par, call.date
                    )));
                }
                if call.date < self.issue_date || call.date > self.maturity {
                    return Err(finstack_core::Error::Validation(format!(
                        "Call exercise date {} is outside bond life [{}, {}]",
                        call.date, self.issue_date, self.maturity
                    )));
                }
                if let Some(end) = call.end_date {
                    if end > self.maturity {
                        return Err(finstack_core::Error::Validation(format!(
                            "Call exercise end date {} is after maturity {}",
                            end, self.maturity
                        )));
                    }
                    if call.date > end {
                        return Err(finstack_core::Error::Validation(format!(
                            "Call exercise start date {} is after end date {}",
                            call.date, end
                        )));
                    }
                }
            }
            for put in &call_put.puts {
                if put.price_pct_of_par <= 0.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "Bond put price must be positive, got {} on {}",
                        put.price_pct_of_par, put.date
                    )));
                }
                if put.date < self.issue_date || put.date > self.maturity {
                    return Err(finstack_core::Error::Validation(format!(
                        "Put exercise date {} is outside bond life [{}, {}]",
                        put.date, self.issue_date, self.maturity
                    )));
                }
                if let Some(end) = put.end_date {
                    if end > self.maturity {
                        return Err(finstack_core::Error::Validation(format!(
                            "Put exercise end date {} is after maturity {}",
                            end, self.maturity
                        )));
                    }
                    if put.date > end {
                        return Err(finstack_core::Error::Validation(format!(
                            "Put exercise start date {} is after end date {}",
                            put.date, end
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns `true` when coupon cashflows depend on forward curve projection (floating FRNs).
    ///
    /// True for [`CashflowSpec::Floating`] and for [`CashflowSpec::Amortizing`] when the
    /// base specification is floating.
    pub fn has_floating_coupons(&self) -> bool {
        match &self.cashflow_spec {
            CashflowSpec::Floating(_) => true,
            CashflowSpec::Amortizing { base, .. } => {
                matches!(base.as_ref(), CashflowSpec::Floating(_))
            }
            _ => false,
        }
    }

    /// Recursively validate that fixed coupon rates are non-negative.
    ///
    /// Handles `Fixed`, `Floating` (no coupon rate to validate), and
    /// `Amortizing` (recurses into the base spec).
    fn validate_coupon_rate(spec: &CashflowSpec) -> Result<()> {
        match spec {
            CashflowSpec::Fixed(s) => {
                let rate = s.rate.to_f64().unwrap_or(0.0);
                if rate < 0.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "Bond fixed coupon rate must be non-negative, got {}",
                        rate
                    )));
                }
            }
            CashflowSpec::StepUp(s) => {
                let rate = s.initial_rate.to_f64().unwrap_or(0.0);
                if rate < 0.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "Bond step-up initial coupon rate must be non-negative, got {}",
                        rate
                    )));
                }
                for (_, step_rate) in &s.step_schedule {
                    let r = step_rate.to_f64().unwrap_or(0.0);
                    if r < 0.0 {
                        return Err(finstack_core::Error::Validation(format!(
                            "Bond step-up coupon rate must be non-negative, got {}",
                            r
                        )));
                    }
                }
            }
            CashflowSpec::Amortizing { base, .. } => {
                Self::validate_coupon_rate(base)?;
            }
            CashflowSpec::Floating(_) => {
                // No fixed coupon rate to validate
            }
        }
        Ok(())
    }
}
