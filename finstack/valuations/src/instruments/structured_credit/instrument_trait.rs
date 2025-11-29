//! Common trait for structured credit instruments.
//!
//! This trait provides a shared interface for CLO, ABS, RMBS, and CMBS instruments
//! to generate cashflows using a consistent waterfall engine.
//!
//! # Module Organization
//!
//! The simulation helpers (RecoveryLagBuffer, PeriodFlows, update_tranche_balance)
//! are in the [`simulation_helpers`](super::simulation_helpers) module.

use super::components::{
    AssetPool, CreditFactors, DefaultModelSpec, MarketConditions, PaymentRecipient,
    PrepaymentModelSpec, RecoveryModelSpec, TrancheCashflowResult, TrancheStructure,
    WaterfallEngine,
};
use super::config::POOL_BALANCE_CLEANUP_THRESHOLD;
use super::simulation_helpers::{update_tranche_balance, PeriodFlows, RecoveryLagBuffer};
use crate::cashflow::traits::DatedFlows;
use finstack_core::dates::months_between;
use finstack_core::dates::{Date, Frequency, ScheduleBuilder};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use std::collections::HashMap;

/// Common trait for structured credit instruments (internal implementation detail)
pub(crate) trait StructuredCreditInstrument {
    /// Get reference to asset pool
    fn pool(&self) -> &AssetPool;

    /// Get reference to tranche structure
    fn tranches(&self) -> &TrancheStructure;

    /// Get closing date
    fn closing_date(&self) -> Date;

    /// Get first payment date
    fn first_payment_date(&self) -> Date;

    /// Get legal maturity date
    fn legal_maturity(&self) -> Date;

    /// Get payment frequency
    fn payment_frequency(&self) -> Frequency;

    /// Get prepayment model specification
    fn prepayment_spec(&self) -> &PrepaymentModelSpec;

    /// Get default model specification
    fn default_spec_ref(&self) -> &DefaultModelSpec;

    /// Get recovery model specification
    fn recovery_spec_ref(&self) -> &RecoveryModelSpec;

    /// Get market conditions
    #[allow(dead_code)]
    fn market_conditions(&self) -> &MarketConditions;

    /// Get credit factors
    #[allow(dead_code)]
    fn credit_factors(&self) -> &CreditFactors;

    /// Get default behavioral assumptions
    #[allow(dead_code)]
    fn default_assumptions(&self) -> &super::config::DefaultAssumptions;

    /// Create instrument-specific waterfall engine
    fn create_waterfall_engine(&self) -> WaterfallEngine;

    /// Get instrument-specific prepayment rate override (if any)
    fn prepayment_rate_override(&self, _pay_date: Date, _seasoning: u32) -> Option<f64> {
        None
    }

    /// Get instrument-specific default rate override (if any)
    fn default_rate_override(&self, _pay_date: Date, _seasoning: u32) -> Option<f64> {
        None
    }

    /// Calculate prepayment rate (SMM)
    fn calculate_prepayment_rate(&self, pay_date: Date, seasoning_months: u32) -> f64 {
        if let Some(override_rate) = self.prepayment_rate_override(pay_date, seasoning_months) {
            return override_rate;
        }

        self.prepayment_spec().smm(seasoning_months).max(0.0)
    }

    /// Calculate default rate (MDR)
    fn calculate_default_rate(&self, pay_date: Date, seasoning_months: u32) -> f64 {
        if let Some(override_rate) = self.default_rate_override(pay_date, seasoning_months) {
            return override_rate;
        }

        self.default_spec_ref().mdr(seasoning_months).max(0.0)
    }

    /// Calculate period interest collections from pool assets
    ///
    /// For floating rate assets, uses the forward curve's day count convention
    /// and actual tenor for year fraction calculations to ensure consistency
    /// with how the curve was calibrated.
    fn calculate_period_interest_collections(
        &self,
        pay_date: Date,
        prev_date: Option<Date>,
        months_per_period: f64,
        context: &MarketContext,
    ) -> finstack_core::Result<Money> {
        let pool = self.pool();
        let base_ccy = pool.base_currency();
        let mut interest_collections = Money::new(0.0, base_ccy);

        for asset in &pool.assets {
            // Determine asset rate
            let asset_rate = if let Some(idx) = &asset.index_id {
                match context.get_forward_ref(idx.as_str()) {
                    Ok(fwd) => {
                        let base = fwd.base_date();
                        // Use the curve's own day count for consistency with calibration
                        let dc = fwd.day_count();
                        let t2 = dc
                            .year_fraction(
                                base,
                                pay_date,
                                finstack_core::dates::DayCountCtx::default(),
                            )
                            .unwrap_or(0.0);
                        // Use the curve's actual tenor instead of hardcoded 0.25
                        let tenor = fwd.tenor();
                        let t1 = (t2 - tenor).max(0.0);
                        let idx_rate = fwd.rate_period(t1, t2);
                        idx_rate + (asset.spread_bps().max(0.0) / 10_000.0)
                    }
                    Err(_) => asset.rate,
                }
            } else {
                asset.rate
            };

            // Calculate accrual factor based on asset day count or fallback
            let accrual_factor = if let (Some(prev), Some(dc)) = (prev_date, asset.day_count) {
                dc.year_fraction(prev, pay_date, finstack_core::dates::DayCountCtx::default())?
            } else {
                months_per_period / 12.0
            };

            let ir = Money::new(
                asset.balance.amount() * asset_rate * accrual_factor,
                base_ccy,
            );
            interest_collections = interest_collections.checked_add(ir)?;
        }

        Ok(interest_collections)
    }

    /// Calculate prepayments and defaults for a period
    fn calculate_period_prepayments_and_defaults(
        &self,
        pay_date: Date,
        seasoning_months: u32,
        pool_outstanding: Money,
        months_per_period: f64,
    ) -> finstack_core::Result<(Money, Money, Money)> {
        let base_ccy = pool_outstanding.currency();
        let smm = self.calculate_prepayment_rate(pay_date, seasoning_months);
        let mdr = self.calculate_default_rate(pay_date, seasoning_months);

        // Adjust for payment period frequency using powf to correctly handle
        // fractional periods (e.g., long/short first periods of 1.5 months)
        let period_smm = 1.0 - (1.0 - smm).powf(months_per_period);
        let period_mdr = 1.0 - (1.0 - mdr).powf(months_per_period);

        let prepay_amt = Money::new(pool_outstanding.amount() * period_smm, base_ccy);
        let default_amt = Money::new(pool_outstanding.amount() * period_mdr, base_ccy);

        let recovery_rate = self.recovery_spec_ref().rate;
        let recovery_amt = Money::new(default_amt.amount() * recovery_rate, base_ccy);

        Ok((prepay_amt, default_amt, recovery_amt))
    }

    /// Runs the full cashflow simulation and returns detailed results for all tranches.
    /// This is the core simulation engine that is reused by the public cashflow methods.
    fn run_full_simulation(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<HashMap<String, TrancheCashflowResult>> {
        let pool = self.pool();
        let tranches = self.tranches();
        let base_ccy = pool.base_currency();
        let mut pool_outstanding = pool.total_balance();

        if pool_outstanding.amount() <= 0.0 {
            return Ok(HashMap::new());
        }

        // Initialize results map for each tranche
        let mut results: HashMap<String, TrancheCashflowResult> = tranches
            .tranches
            .iter()
            .map(|t| {
                (
                    t.id.to_string(),
                    TrancheCashflowResult {
                        tranche_id: t.id.to_string(),
                        cashflows: Vec::new(),
                        detailed_flows: Vec::new(),
                        interest_flows: Vec::new(),
                        principal_flows: Vec::new(),
                        pik_flows: Vec::new(),
                        final_balance: t.current_balance,
                        total_interest: Money::new(0.0, base_ccy),
                        total_principal: Money::new(0.0, base_ccy),
                        total_pik: Money::new(0.0, base_ccy),
                    },
                )
            })
            .collect();

        // Get date configurations
        let dates_closing_date = self.closing_date();
        let dates_first_payment_date = self.first_payment_date();
        let dates_legal_maturity = self.legal_maturity();
        let dates_payment_frequency = self.payment_frequency();

        // Track tranche balances over time
        let mut tranche_balances: HashMap<String, Money> = tranches
            .tranches
            .iter()
            .map(|t| (t.id.to_string(), t.current_balance))
            .collect();

        // Initialize waterfall engine
        let waterfall_engine = self.create_waterfall_engine();
        let months_per_period = dates_payment_frequency.months().unwrap_or(3) as f64;

        // Initialize recovery lag buffer for delayed recovery processing
        let recovery_lag_months = self.recovery_spec_ref().recovery_lag;
        let mut recovery_buffer = RecoveryLagBuffer::new();

        // Generate payment schedule
        let schedule =
            ScheduleBuilder::try_new(dates_first_payment_date.max(as_of), dates_legal_maturity)?
                .frequency(dates_payment_frequency)
                .build()?;

        // Simulate period-by-period
        let mut prev_date = Some(dates_closing_date);

        for pay_date in schedule.dates {
            if pool_outstanding.amount() <= POOL_BALANCE_CLEANUP_THRESHOLD {
                break;
            }

            let seasoning_months = months_between(dates_closing_date, pay_date);

            // Step 1: Calculate pool cashflows for the period
            let interest_collections = self.calculate_period_interest_collections(
                pay_date,
                prev_date,
                months_per_period,
                context,
            )?;

            // Update prev_date for next iteration
            prev_date = Some(pay_date);

            let (prepay_amt, default_amt, recovery_amt) = self
                .calculate_period_prepayments_and_defaults(
                    pay_date,
                    seasoning_months,
                    pool_outstanding,
                    months_per_period,
                )?;

            // Add new recoveries to the lag buffer (they'll be released after the lag period)
            recovery_buffer.add_recovery(pay_date, recovery_amt);

            // Release matured recoveries from the buffer
            let released_recoveries =
                recovery_buffer.release_matured(pay_date, recovery_lag_months, base_ccy)?;

            // Reinvestment Logic:
            // If in reinvestment period, retain principal (prepay + released recoveries) to buy new assets.
            // We assume reinvestment at Par (maintaining pool balance), so we don't pass principal to waterfall.
            // Note: Defaults reduce the pool balance permanently by the loss amount.
            // With recovery lag, the actual recovery comes later, so we track it separately.
            let is_reinvestment_active = if let Some(period) = &pool.reinvestment_period {
                pay_date <= period.end_date
            } else {
                false
            };

            let (principal_available_for_waterfall, _reinvested_amount) = if is_reinvestment_active
            {
                // In reinvestment, we keep principal collections.
                // Available for waterfall is only interest (plus maybe some specialized leakage, ignored here).
                (
                    Money::new(0.0, base_ccy),
                    prepay_amt.checked_add(released_recoveries)?,
                )
            } else {
                // Not reinvesting, all principal goes to waterfall
                (
                    prepay_amt.checked_add(released_recoveries)?,
                    Money::new(0.0, base_ccy),
                )
            };

            let period_flows = PeriodFlows {
                interest_collections,
                prepayments: prepay_amt,
                defaults: default_amt,
                recoveries: released_recoveries, // Use released (lagged) recoveries
            };

            let total_cash_for_waterfall =
                interest_collections.checked_add(principal_available_for_waterfall)?;

            // Step 2: Run waterfall to distribute cash
            let waterfall_result = waterfall_engine.execute_waterfall(
                total_cash_for_waterfall,
                period_flows.interest_collections,
                pay_date,
                tranches,
                pool_outstanding,
                pool,
                context,
            )?;

            // Step 3: Record flows and update balances for all tranches
            for tranche in &tranches.tranches {
                let tranche_id = tranche.id.to_string();
                if let Some(payment) = waterfall_result
                    .distributions
                    .get(&PaymentRecipient::Tranche(tranche_id.clone()))
                {
                    if payment.amount() > 0.0 {
                        let current_balance = tranche_balances
                            .get(&tranche_id)
                            .copied()
                            .unwrap_or(Money::new(0.0, base_ccy));
                        let coupon_rate = tranche.coupon.current_rate_with_index(pay_date, context);

                        let interest_portion = Money::new(
                            current_balance.amount() * coupon_rate * (months_per_period / 12.0),
                            base_ccy,
                        );

                        let principal_payment = payment
                            .checked_sub(interest_portion)
                            .unwrap_or(Money::new(0.0, base_ccy));

                        // Update the results for this tranche
                        if let Some(res) = results.get_mut(&tranche_id) {
                            res.cashflows.push((pay_date, *payment));
                            if interest_portion.amount() > 0.0 {
                                res.interest_flows.push((pay_date, interest_portion));
                                res.total_interest =
                                    res.total_interest.checked_add(interest_portion)?;
                            }
                            if principal_payment.amount() > 0.0 {
                                res.principal_flows.push((pay_date, principal_payment));
                                res.total_principal =
                                    res.total_principal.checked_add(principal_payment)?;
                            }
                        }

                        // Update tranche balance for next period's interest calc
                        update_tranche_balance(
                            &mut tranche_balances,
                            &tranche_id,
                            *payment,
                            interest_portion,
                        )?;
                    }
                }
            }

            // Step 4: Update pool balance
            //
            // With recovery lag, the accounting is:
            // - Defaults: Pool balance decreases immediately by full default amount
            // - Prepayments: Pool balance decreases immediately
            // - Recoveries: Cash comes back after the lag period (handled via released_recoveries)
            //
            // During reinvestment period:
            // - We reinvest prepayments and released recoveries to buy new assets
            // - Defaults still reduce pool balance immediately
            // - Released recoveries that come in get reinvested
            //
            // The key difference with recovery lag:
            // - Without lag: loss = defaults - recoveries (immediate offset)
            // - With lag: defaults reduce balance immediately, recoveries add cash later

            if is_reinvestment_active {
                // During reinvestment, defaults reduce pool immediately
                // Prepayments and released recoveries get reinvested (balance maintained)
                // So net change = -defaults + reinvested_recoveries
                // Note: prepayments don't reduce balance during reinvestment (they're reinvested)
                pool_outstanding = pool_outstanding
                    .checked_sub(default_amt)?
                    .checked_add(released_recoveries)?;
            } else {
                // Normal amortization: balance reduces by prepays and defaults
                pool_outstanding = pool_outstanding
                    .checked_sub(period_flows.prepayments)?
                    .checked_sub(period_flows.defaults)?;
            }
        }

        // Final step: update final balances and detailed flows in results
        for (tranche_id, res) in results.iter_mut() {
            res.final_balance = tranche_balances
                .get(tranche_id)
                .copied()
                .unwrap_or(Money::new(0.0, base_ccy));

            for (date, amount) in &res.interest_flows {
                if amount.amount() > 0.0 {
                    let cf = finstack_core::cashflow::CashFlow {
                        date: *date,
                        reset_date: None,
                        amount: *amount,
                        kind: finstack_core::cashflow::CFKind::Fixed,
                        accrual_factor: 0.0,
                        rate: None,
                    };
                    res.detailed_flows.push(cf);
                }
            }
            for (date, amount) in &res.principal_flows {
                if amount.amount() > 0.0 {
                    let cf = finstack_core::cashflow::CashFlow {
                        date: *date,
                        reset_date: None,
                        amount: *amount,
                        kind: finstack_core::cashflow::CFKind::Amortization,
                        accrual_factor: 0.0,
                        rate: None,
                    };
                    res.detailed_flows.push(cf);
                }
            }
        }

        Ok(results)
    }

    /// Generate complete tranche-specific cashflows using waterfall engine
    ///
    /// This is the shared implementation that eliminates duplication across
    /// CLO, ABS, CMBS, and RMBS instruments.
    fn generate_tranche_cashflows(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        let full_results = self.run_full_simulation(context, as_of)?;

        // Aggregate all tranche cashflows into a single schedule
        // Pre-allocate based on estimated number of unique payment dates
        let estimated_dates = full_results
            .values()
            .next()
            .map(|r| r.cashflows.len())
            .unwrap_or(0);
        let mut all_flows: DatedFlows = Vec::with_capacity(estimated_dates);
        let mut flow_map: HashMap<Date, Money> = HashMap::with_capacity(estimated_dates);

        for (_tranche_id, result) in full_results {
            for (date, amount) in result.cashflows {
                flow_map
                    .entry(date)
                    .and_modify(|existing| {
                        *existing = existing.checked_add(amount).unwrap_or(*existing)
                    })
                    .or_insert(amount);
            }
        }

        for (date, amount) in flow_map {
            all_flows.push((date, amount));
        }
        all_flows.sort_by_key(|(d, _)| *d);

        Ok(all_flows)
    }

    /// Generate cashflows for a specific tranche after waterfall allocation
    ///
    /// This method runs the full waterfall simulation but returns only the
    /// cashflows allocated to the specified tranche, properly separating
    /// interest and principal components.
    fn generate_specific_tranche_cashflows(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<super::components::TrancheCashflowResult> {
        let mut full_results = self.run_full_simulation(context, as_of)?;

        full_results.remove(tranche_id).ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: format!("tranche:{}", tranche_id),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    // Tests would go here
}
