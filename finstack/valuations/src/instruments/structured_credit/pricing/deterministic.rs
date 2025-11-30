//! Deterministic cashflow simulation for structured credit instruments.
//!
//! This module provides pure functions for running period-by-period
//! cashflow simulation through the waterfall engine.

use super::waterfall::execute_waterfall;
use crate::cashflow::traits::DatedFlows;
use crate::instruments::structured_credit::types::constants::POOL_BALANCE_CLEANUP_THRESHOLD;
use crate::instruments::structured_credit::types::{
    Pool, RecipientType, StructuredCredit, TrancheCashflows, TrancheStructure, Waterfall,
};
use crate::instruments::structured_credit::utils::simulation::RecoveryQueue;
use finstack_core::cashflow::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::months_between;
use finstack_core::dates::{Date, DayCount, DayCountCtx, ScheduleBuilder};
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

    if pool.total_balance().amount() <= 0.0 {
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

/// Internal state for period-by-period simulation.
struct SimulationState<'a> {
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

        Self {
            pool_outstanding: pool.total_balance(),
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
    waterfall: &Waterfall,
    pay_date: Date,
    context: &MarketContext,
    months_per_period: f64,
) -> Result<()> {
    let seasoning_months = months_between(state.closing_date, pay_date);

    // Capture period start before updating prev_date (for accrual calculations)
    let period_start = state.prev_date.unwrap_or(state.closing_date);

    // Step 1: Calculate pool cashflows for the period
    let interest_collections = calculate_period_interest_collections(
        &instrument.pool,
        pay_date,
        Some(period_start),
        months_per_period,
        context,
    )?;

    state.prev_date = Some(pay_date);

    let (prepay_amt, default_amt, recovery_amt) = calculate_period_prepayments_and_defaults(
        instrument,
        pay_date,
        seasoning_months,
        state.pool_outstanding,
        months_per_period,
    )?;

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

    // Step 2: Run waterfall to distribute cash
    let waterfall_result = execute_waterfall(
        waterfall,
        total_cash_for_waterfall,
        interest_collections,
        pay_date,
        period_start,
        state.tranches,
        state.pool_outstanding,
        state.pool,
        context,
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
    if is_reinvestment_active {
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

/// Calculate period interest collections from pool assets.
fn calculate_period_interest_collections(
    pool: &Pool,
    pay_date: Date,
    prev_date: Option<Date>,
    months_per_period: f64,
    context: &MarketContext,
) -> Result<Money> {
    let base_ccy = pool.base_currency();
    let mut interest_collections = Money::new(0.0, base_ccy);

    for asset in &pool.assets {
        let asset_rate = if let Some(idx) = &asset.index_id {
            match context.get_forward_ref(idx.as_str()) {
                Ok(fwd) => {
                    let base = fwd.base_date();
                    let dc = fwd.day_count();
                    let t2 = dc.year_fraction(base, pay_date, DayCountCtx::default())?;
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

        // Use asset's day-count if available, otherwise default to ACT/360 (market standard for loans)
        let accrual_factor = match (prev_date, asset.day_count) {
            (Some(prev), Some(dc)) => dc.year_fraction(prev, pay_date, DayCountCtx::default())?,
            (Some(prev), None) => {
                DayCount::Act360.year_fraction(prev, pay_date, DayCountCtx::default())?
            }
            _ => months_per_period / 12.0, // Fallback only when no prev_date
        };

        let ir = Money::new(
            asset.balance.amount() * asset_rate * accrual_factor,
            base_ccy,
        );
        interest_collections = interest_collections.checked_add(ir)?;
    }

    Ok(interest_collections)
}

/// Calculate prepayments and defaults for a period.
fn calculate_period_prepayments_and_defaults(
    instrument: &StructuredCredit,
    pay_date: Date,
    seasoning_months: u32,
    pool_outstanding: Money,
    months_per_period: f64,
) -> Result<(Money, Money, Money)> {
    if months_per_period <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "months_per_period must be positive, got {}",
            months_per_period
        )));
    }

    let base_ccy = pool_outstanding.currency();

    // Calculate rates using the instrument's behavioral logic (respect overrides)
    let smm = instrument.calculate_prepayment_rate(pay_date, seasoning_months);
    let mdr = instrument.calculate_default_rate(pay_date, seasoning_months);

    // Adjust for payment period frequency
    let period_smm = 1.0 - (1.0 - smm).powf(months_per_period);
    let period_mdr = 1.0 - (1.0 - mdr).powf(months_per_period);

    let prepay_amt = Money::new(pool_outstanding.amount() * period_smm, base_ccy);
    let default_amt = Money::new(pool_outstanding.amount() * period_mdr, base_ccy);

    let recovery_rate = instrument.recovery_spec.rate;
    let recovery_amt = Money::new(default_amt.amount() * recovery_rate, base_ccy);

    Ok((prepay_amt, default_amt, recovery_amt))
}
