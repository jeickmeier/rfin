//! Deterministic cashflow simulation for structured credit instruments.
//!
//! This module provides pure functions for running period-by-period
//! cashflow simulation through the waterfall engine.

use crate::cashflow::traits::DatedFlows;
use crate::instruments::structured_credit::types::constants::POOL_BALANCE_CLEANUP_THRESHOLD;
use crate::instruments::structured_credit::types::{
    Pool, PoolState, RecipientType, StructuredCredit, TrancheCashflows, TrancheStructure, Waterfall,
};
use crate::instruments::structured_credit::utils::simulation::RecoveryQueue;
use finstack_core::cashflow::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::months_between;
use finstack_core::dates::{adjust, BusinessDayConvention, Date, DayCount, DayCountCtx, ScheduleBuilder};
use finstack_core::dates::calendar::types::Calendar;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

// ============================================================================
// PUBLIC API
// ============================================================================

/// Run full cashflow simulation for a structured credit instrument.
///
/// Returns detailed cashflow results for each tranche.
pub fn run_simulation(
    instrument: &StructuredCredit,
    context: &MarketContext,
    as_of: Date,
) -> Result<HashMap<String, TrancheCashflows>> {
    let pool = &instrument.pool;
    let tranches = &instrument.tranches;

    if pool.total_balance()?.amount() <= 0.0 {
        return Ok(HashMap::new());
    }

    // Validate and extract months per period
    let months_per_period = match instrument.payment_frequency.months() {
        Some(m) => m as f64,
        None => {
            return Err(finstack_core::Error::Validation(
                "Structured credit instruments require month-based payment frequencies".to_string(),
            ));
        }
    };

    // Initialize simulation state
    let mut state = SimulationState::new(
        pool,
        tranches,
        instrument.closing_date,
        months_per_period,
        instrument.recovery_spec.recovery_lag,
    );

    // Create waterfall
    let waterfall = instrument.create_waterfall();

    // Generate payment schedule
    let schedule = ScheduleBuilder::try_new(
        instrument.first_payment_date.max(as_of),
        instrument.legal_maturity,
    )?
    .frequency(instrument.payment_frequency)
    .build()?;

    // Adjust payment dates for business days
    // TODO: Add calendar to StructuredCredit. For now use weekends-only calendar.
    let calendar = Calendar::new("weekends_only", "Weekends Only", true, &[]);
    let convention = BusinessDayConvention::Following;
    let mut adjusted_schedule = schedule;
    for date in &mut adjusted_schedule.dates {
        *date = adjust(*date, convention, &calendar)?;
    }
    let schedule = adjusted_schedule;

    // Simulate period-by-period
    for pay_date in schedule.dates {
        if state.is_pool_exhausted() {
            break;
        }

        simulate_period(
            &mut state,
            instrument,
            &waterfall,
            pay_date,
            context,
            months_per_period,
        )?;
    }

    Ok(state.finalize())
}

/// Generate aggregated cashflows for all tranches.
pub fn generate_cashflows(
    instrument: &StructuredCredit,
    context: &MarketContext,
    as_of: Date,
) -> Result<DatedFlows> {
    let full_results = run_simulation(instrument, context, as_of)?;

    // Aggregate all tranche cashflows into a single schedule
    let estimated_dates = full_results
        .values()
        .next()
        .map(|r| r.cashflows.len())
        .unwrap_or(0);
    let mut flow_map: HashMap<Date, Money> = HashMap::with_capacity(estimated_dates);

    for result in full_results.values() {
        for (date, amount) in &result.cashflows {
            flow_map
                .entry(*date)
                .and_modify(|existing| {
                    *existing = existing.checked_add(*amount).unwrap_or(*existing)
                })
                .or_insert(*amount);
        }
    }

    let mut all_flows: DatedFlows = flow_map.into_iter().collect();
    all_flows.sort_by_key(|(d, _)| *d);

    Ok(all_flows)
}

/// Generate cashflows for a specific tranche.
pub fn generate_tranche_cashflows(
    instrument: &StructuredCredit,
    tranche_id: &str,
    context: &MarketContext,
    as_of: Date,
) -> Result<TrancheCashflows> {
    let mut full_results = run_simulation(instrument, context, as_of)?;

    full_results.remove(tranche_id).ok_or_else(|| {
        finstack_core::Error::from(finstack_core::error::InputError::NotFound {
            id: format!("tranche:{}", tranche_id),
        })
    })
}

// ============================================================================
// SIMULATION STATE
// ============================================================================

// ============================================================================
// SIMULATION STATE
// ============================================================================

/// Internal state for period-by-period simulation.
struct SimulationState<'a> {
    /// Pool state (SoA layout)
    pool_state: PoolState,
    /// Total pool outstanding (sum of balances)
    pool_outstanding: Money,
    recovery_queue: RecoveryQueue,
    tranche_balances: HashMap<String, Money>,
    results: HashMap<String, TrancheCashflows>,
    prev_date: Option<Date>,
    base_ccy: Currency,
    #[allow(dead_code)]
    months_per_period: f64,
    recovery_lag_months: u32,
    pool: &'a Pool,
    tranches: &'a TrancheStructure,
    closing_date: Date,
    tranche_recipient_keys: Vec<RecipientType>,
}

impl<'a> SimulationState<'a> {
    fn new(
        pool: &'a Pool,
        tranches: &'a TrancheStructure,
        closing_date: Date,
        months_per_period: f64,
        recovery_lag_months: u32,
    ) -> Self {
        let base_ccy = pool.base_currency();

        // Initialize results map for each tranche
        let results: HashMap<String, TrancheCashflows> = tranches
            .tranches
            .iter()
            .map(|t| {
                (
                    t.id.to_string(),
                    TrancheCashflows {
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

        let tranche_balances: HashMap<String, Money> = tranches
            .tranches
            .iter()
            .map(|t| (t.id.to_string(), t.current_balance))
            .collect();

        let tranche_recipient_keys: Vec<RecipientType> = tranches
            .tranches
            .iter()
            .map(|t| RecipientType::Tranche(t.id.to_string()))
            .collect();

        // Initialize PoolState
        // Note: For now we convert the full asset list to PoolState.
        // Future optimization: Support RepLine conversion to PoolState.
        let pool_state = PoolState::from_pool(pool);

        Self {
            pool_state,
            pool_outstanding: pool.total_balance().unwrap_or(Money::new(0.0, base_ccy)), // Safe fallback for init
            recovery_queue: RecoveryQueue::new(),
            tranche_balances,
            results,
            prev_date: Some(closing_date),
            base_ccy,
            months_per_period,
            recovery_lag_months,
            pool,
            tranches,
            closing_date,
            tranche_recipient_keys,
        }
    }

    fn is_pool_exhausted(&self) -> bool {
        self.pool_outstanding.amount() <= POOL_BALANCE_CLEANUP_THRESHOLD
    }

    fn finalize(mut self) -> HashMap<String, TrancheCashflows> {
        for (tranche_id, res) in self.results.iter_mut() {
            res.final_balance = self
                .tranche_balances
                .get(tranche_id)
                .copied()
                .unwrap_or(Money::new(0.0, self.base_ccy));

            for (date, amount) in &res.interest_flows {
                if amount.amount() > 0.0 {
                    let cf = CashFlow {
                        date: *date,
                        reset_date: None,
                        amount: *amount,
                        kind: CFKind::Fixed,
                        accrual_factor: 0.0,
                        rate: None,
                    };
                    res.detailed_flows.push(cf);
                }
            }
            for (date, amount) in &res.principal_flows {
                if amount.amount() > 0.0 {
                    let cf = CashFlow {
                        date: *date,
                        reset_date: None,
                        amount: *amount,
                        kind: CFKind::Amortization,
                        accrual_factor: 0.0,
                        rate: None,
                    };
                    res.detailed_flows.push(cf);
                }
            }
        }

        self.results
    }
}

// ============================================================================
// PERIOD SIMULATION
// ============================================================================

/// Simulate a single payment period.
fn simulate_period(
    state: &mut SimulationState,
    instrument: &StructuredCredit,
    _waterfall: &Waterfall,
    pay_date: Date,
    context: &MarketContext,
    months_per_period: f64,
) -> Result<()> {
    let seasoning_months = months_between(state.closing_date, pay_date);

    // Capture period start before updating prev_date (for accrual calculations)
    let period_start = state.prev_date.unwrap_or(state.closing_date);

    // Step 1: Calculate pool cashflows for the period (Interest + Principal Prepay/Default)
    // We now do this in a unified pass over the rep lines/assets to support line-level overrides
    let (interest_collections, prepay_amt, default_amt, recovery_amt) = calculate_pool_flows(
        state,
        instrument,
        pay_date,
        period_start,
        seasoning_months,
        months_per_period,
        context,
    )?;

    state.prev_date = Some(pay_date);

    // Add new recoveries to the lag queue
    state.recovery_queue.add_recovery(pay_date, recovery_amt);

    // Release matured recoveries
    let released_recoveries = state.recovery_queue.release_matured(
        pay_date,
        state.recovery_lag_months,
        state.base_ccy,
    )?;

    // Reinvestment logic
    let is_reinvestment_active = state
        .pool
        .reinvestment_period
        .as_ref()
        .is_some_and(|period| pay_date <= period.end_date);

    let principal_available_for_waterfall = if is_reinvestment_active {
        Money::new(0.0, state.base_ccy)
    } else {
        prepay_amt.checked_add(released_recoveries)?
    };

    let total_cash_for_waterfall =
        interest_collections.checked_add(principal_available_for_waterfall)?;

    // Step 2: Execute Waterfall
    let waterfall_context =
        crate::instruments::structured_credit::pricing::waterfall::WaterfallContext {
            available_cash: total_cash_for_waterfall,
            interest_collections,
            payment_date: pay_date,
            period_start,
            pool_balance: state.pool_outstanding,
            market: context,
        };

    let waterfall_result =
        crate::instruments::structured_credit::pricing::waterfall::execute_waterfall(
            &instrument.create_waterfall(),
            state.tranches,
            state.pool,
            waterfall_context,
        )?;

    // Step 3: Record flows and update balances for all tranches
    for (idx, tranche) in state.tranches.tranches.iter().enumerate() {
        let recipient_key = &state.tranche_recipient_keys[idx];

        if let Some(payment) = waterfall_result.distributions.get(recipient_key) {
            if payment.amount() > 0.0 {
                let tranche_id_str = tranche.id.as_str();

                let current_balance = state
                    .tranche_balances
                    .get(tranche_id_str)
                    .copied()
                    .unwrap_or(Money::new(0.0, state.base_ccy));
                let coupon_rate = tranche.coupon.current_rate_with_index(pay_date, context);

                // Use tranche's day-count convention for proper accrual calculation
                let accrual_factor = tranche
                    .day_count
                    .year_fraction(period_start, pay_date, DayCountCtx::default())
                    .unwrap_or(months_per_period / 12.0);

                let interest_portion = Money::new(
                    current_balance.amount() * coupon_rate * accrual_factor,
                    state.base_ccy,
                );

                let principal_payment = payment
                    .checked_sub(interest_portion)
                    .unwrap_or(Money::new(0.0, state.base_ccy));

                if let Some(res) = state.results.get_mut(tranche_id_str) {
                    res.cashflows.push((pay_date, *payment));
                    if interest_portion.amount() > 0.0 {
                        res.interest_flows.push((pay_date, interest_portion));
                        res.total_interest = res.total_interest.checked_add(interest_portion)?;
                    }
                    if principal_payment.amount() > 0.0 {
                        res.principal_flows.push((pay_date, principal_payment));
                        res.total_principal = res.total_principal.checked_add(principal_payment)?;
                    }
                }

                // Update tranche balance
                if let Some(current) = state.tranche_balances.get_mut(tranche_id_str) {
                    *current = current.checked_sub(principal_payment).unwrap_or(*current);
                }
            }
        }
    }

    // Step 4: Update pool balance
    // Note: rep_line_balances were already updated in calculate_pool_flows
    // We just need to update the total pool_outstanding to match
    if is_reinvestment_active {
        // Reinvestment assumes principal is recycled, so pool balance only drops by defaults (net of recoveries? No, usually gross defaults reduce pool, recoveries come back as cash)
        // Actually, in reinvestment, principal collections are used to buy new assets.
        // So pool balance stays constant unless defaults occur.
        // For simplicity here, we just update pool_outstanding based on the calculated flows.
        state.pool_outstanding = state
            .pool_outstanding
            .checked_sub(default_amt)?
            .checked_add(released_recoveries)?;
    } else {
        state.pool_outstanding = state
            .pool_outstanding
            .checked_sub(prepay_amt)?
            .checked_sub(default_amt)?;
    }

    Ok(())
}

// ============================================================================
// CALCULATION HELPERS
// ============================================================================

/// Calculate all pool flows (Interest, Prepay, Default) for the period.
/// Updates rep_line_balances in place.
fn calculate_pool_flows(
    state: &mut SimulationState,
    instrument: &StructuredCredit,
    pay_date: Date,
    prev_date: Date,
    seasoning_months: u32,
    months_per_period: f64,
    context: &MarketContext,
) -> Result<(Money, Money, Money, Money)> {
    let base_ccy = state.base_ccy;
    let mut total_interest = Money::new(0.0, base_ccy);
    let mut total_prepay = Money::new(0.0, base_ccy);
    let mut total_default = Money::new(0.0, base_ccy);
    let mut total_recovery = Money::new(0.0, base_ccy);

    // Pre-calculate global rates (optimization)
    // Future optimization: Handle per-asset overrides if added to PoolState
    let smm = instrument.calculate_prepayment_rate(pay_date, seasoning_months);
    let mdr = instrument.calculate_default_rate(pay_date, seasoning_months);
    let recovery_rate = instrument.recovery_spec.rate;

    let global_period_smm = 1.0 - (1.0 - smm).powf(months_per_period);
    let global_period_mdr = 1.0 - (1.0 - mdr).powf(months_per_period);

    // Pre-resolve all curves
    let mut resolved_rates = Vec::with_capacity(state.pool_state.unique_curves.len());
    for idx_str in &state.pool_state.unique_curves {
        let fwd = context.get_forward_ref(idx_str)?;
        let base = fwd.base_date();
        let dc = fwd.day_count();
        let t2 = dc.year_fraction(base, pay_date, DayCountCtx::default())?;
        let tenor = fwd.tenor();
        let t1 = (t2 - tenor).max(0.0);
        let r = if t2 > 0.0 && t1 < t2 {
            fwd.rate_period(t1, t2)
        } else {
            // For very short periods (t2 <= tenor or t2 == 0), use spot rate
            fwd.rate(0.0)
        };
        resolved_rates.push(r);
    }

    let n = state.pool_state.len();
    for i in 0..n {
        let balance = state.pool_state.balances[i];
        if balance <= 0.0 {
            continue;
        }

        // 1. Interest
        let rate = if let Some(curve_idx) = state.pool_state.curve_indices[i] {
            let base_rate = resolved_rates[curve_idx];
            base_rate + (state.pool_state.spread_bps[i].unwrap_or(0.0).max(0.0) / 10_000.0)
        } else {
            state.pool_state.rates[i]
        };

        let accrual_factor = state.pool_state.day_counts[i]
            .unwrap_or(DayCount::Act360) // Should be populated now, but safe fallback
            .year_fraction(prev_date, pay_date, DayCountCtx::default())?;

        let interest = Money::new(balance * rate * accrual_factor, base_ccy);
        total_interest = total_interest.checked_add(interest)?;

        // 2. Prepayment & Default
        let period_smm = if let Some(smm) = state.pool_state.smm_overrides[i] {
            1.0 - (1.0 - smm).powf(months_per_period)
        } else {
            global_period_smm
        };

        let period_mdr = if let Some(mdr) = state.pool_state.mdr_overrides[i] {
            1.0 - (1.0 - mdr).powf(months_per_period)
        } else {
            global_period_mdr
        };

        let prepay = Money::new(balance * period_smm, base_ccy);
        let default = Money::new(balance * period_mdr, base_ccy);
        let recovery = Money::new(default.amount() * recovery_rate, base_ccy);

        total_prepay = total_prepay.checked_add(prepay)?;
        total_default = total_default.checked_add(default)?;
        total_recovery = total_recovery.checked_add(recovery)?;

        // Update balance
        let new_balance = balance - prepay.amount() - default.amount();
        state.pool_state.balances[i] = new_balance.max(0.0);
    }

    Ok((total_interest, total_prepay, total_default, total_recovery))
}
