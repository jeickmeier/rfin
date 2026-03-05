//! Deterministic cashflow simulation for structured credit instruments.
//!
//! This module provides pure functions for running period-by-period
//! cashflow simulation through the waterfall engine.

use crate::cashflow::traits::DatedFlows;
use crate::instruments::fixed_income::structured_credit::types::constants::POOL_BALANCE_CLEANUP_THRESHOLD;
use crate::instruments::fixed_income::structured_credit::types::{
    Pool, PoolState, RecipientType, StructuredCredit, TrancheCashflows, TrancheSeniority,
    TrancheStructure, Waterfall,
};
use crate::instruments::fixed_income::structured_credit::utils::simulation::RecoveryQueue;
use finstack_core::cashflow::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::CalendarRegistry;
use finstack_core::dates::HolidayCalendar;
use finstack_core::dates::{
    adjust, BusinessDayConvention, Date, DateExt, DayCount, DayCountCtx, ScheduleBuilder,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::HashMap;
use finstack_core::Result;

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
        return Ok(HashMap::default());
    }

    // Validate and extract months per period
    let months_per_period = match instrument.frequency.months() {
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
        instrument.credit_model.recovery_spec.recovery_lag,
        instrument.credit_model.recovery_spec.rate,
    );

    // Create waterfall
    let waterfall = instrument.create_waterfall();

    // Resolve payment calendar - required for structured credit deals.
    // Silent fallback to weekends-only would shift coupons around holidays,
    // breaking WAC/WAL and OC tests.
    let calendar: &dyn HolidayCalendar = match instrument.payment_calendar_id.as_deref() {
        Some(cal_id) => CalendarRegistry::global()
            .resolve_str(cal_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::InputError::NotFound {
                    id: format!(
                        "payment_calendar_id:{} (available: {})",
                        cal_id,
                        CalendarRegistry::global().available_ids().join(", ")
                    ),
                })
            })?,
        None => {
            return Err(finstack_core::Error::Validation(
                "Structured credit instruments require a payment_calendar_id for accurate \
                     schedule generation. Specify a valid calendar ID (e.g., 'nyse', 'target2') \
                     to ensure payment dates are adjusted correctly for business days."
                    .to_string(),
            ));
        }
    };

    let convention = instrument
        .payment_bdc
        .unwrap_or(BusinessDayConvention::ModifiedFollowing);

    // Generate payment schedule with calendar-adjusted dates
    let schedule = ScheduleBuilder::new(
        instrument.first_payment_date.max(as_of),
        instrument.maturity,
    )?
    .frequency(instrument.frequency)
    .build()?;

    let mut adjusted_schedule = schedule;
    for date in &mut adjusted_schedule.dates {
        *date = adjust(*date, convention, calendar)?;
    }
    let schedule = adjusted_schedule;

    // Simulate period-by-period
    for pay_date in schedule.dates {
        if state.is_pool_exhausted() {
            break;
        }

        // Clean-up call: if pool factor drops below threshold, redeem tranches.
        //
        // INTEX/Bloomberg convention: when pool factor (current / original total
        // balance) drops below the cleanup threshold (typically 10%), the equity
        // holder may exercise an optional redemption. Redemption pays tranches
        // in seniority order (senior first), bounded by the remaining pool value.
        if let Some(cleanup_threshold) = instrument.cleanup_call_pct {
            let pool_factor = if state.total_pool_balance.amount() > 0.0 {
                state.pool_outstanding.amount() / state.total_pool_balance.amount()
            } else {
                0.0
            };
            if pool_factor < cleanup_threshold && pool_factor > 0.0 {
                // Available cash for redemption = remaining pool outstanding.
                // The equity holder purchases remaining collateral and uses proceeds
                // to redeem notes in seniority order.
                let mut available_for_redemption = state.pool_outstanding.amount();

                // Pay tranches in seniority order (Senior=0 first, Equity=3 last)
                let mut redemption_order: Vec<usize> = (0..state.tranches.tranches.len()).collect();
                redemption_order.sort_by_key(|&i| state.tranches.tranches[i].seniority);

                for &idx in &redemption_order {
                    if available_for_redemption <= WRITEDOWN_DE_MINIMIS {
                        break;
                    }
                    let tranche = &state.tranches.tranches[idx];
                    let tranche_id_str = tranche.id.as_str();
                    let balance = state
                        .tranche_balances
                        .get(tranche_id_str)
                        .copied()
                        .unwrap_or(Money::new(0.0, state.base_ccy));

                    if balance.amount() <= WRITEDOWN_DE_MINIMIS {
                        continue;
                    }

                    let redemption = Money::new(
                        balance.amount().min(available_for_redemption),
                        state.base_ccy,
                    );
                    available_for_redemption -= redemption.amount();

                    if let Some(res) = state.results.get_mut(tranche_id_str) {
                        res.principal_flows.push((pay_date, redemption));
                        res.cashflows.push((pay_date, redemption));
                        res.total_principal = res.total_principal.checked_add(redemption)?;
                    }
                    if let Some(bal) = state.tranche_balances.get_mut(tranche_id_str) {
                        *bal = bal
                            .checked_sub(redemption)
                            .unwrap_or(Money::new(0.0, state.base_ccy));
                    }
                }
                break; // Terminate simulation after cleanup call
            }
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
    let mut flow_map: HashMap<Date, Money> = {
        let mut m = HashMap::default();
        m.reserve(estimated_dates);
        m
    };

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
        finstack_core::Error::from(finstack_core::InputError::NotFound {
            id: format!("tranche:{}", tranche_id),
        })
    })
}

// ============================================================================
// SIMULATION STATE
// ============================================================================

/// De minimis threshold for write-down recording (avoids noise from fp rounding).
const WRITEDOWN_DE_MINIMIS: f64 = 0.01;

/// Internal state for period-by-period simulation.
struct SimulationState<'a> {
    /// Pool state (SoA layout)
    pool_state: PoolState,
    /// Total pool outstanding (sum of balances)
    pool_outstanding: Money,
    recovery_queue: RecoveryQueue,
    tranche_balances: HashMap<String, Money>,
    /// Deferred (PIK) interest per tranche, carried forward to next period.
    deferred_interest: HashMap<String, Money>,
    results: HashMap<String, TrancheCashflows>,
    prev_date: Option<Date>,
    base_ccy: Currency,
    recovery_lag_months: u32,
    pool: &'a Pool,
    tranches: &'a TrancheStructure,
    closing_date: Date,
    tranche_recipient_keys: Vec<RecipientType>,
    /// Whether reinvestment was active in the previous period.
    /// Used to detect the reinvestment-end transition and reconcile pool_outstanding.
    was_reinvestment_active: bool,
    /// Cumulative expected net losses (default_amount * (1 - recovery_rate)).
    ///
    /// Uses the expected recovery at the point of default rather than lagged
    /// realized recoveries. This is the INTEX/Moody's Analytics convention:
    /// loss allocation should reflect economic loss at default, not cash-timing
    /// of recovery receipts. Lagged recoveries affect waterfall cash, not
    /// loss allocation.
    cumulative_expected_loss: f64,
    /// Total pool balance at simulation start (including defaulted assets).
    /// Used for cleanup call pool factor calculation.
    total_pool_balance: Money,
    /// Performing pool balance at simulation start (excluding pre-defaulted assets).
    /// Used as denominator for loss allocation percentage.
    performing_pool_balance: Money,
    /// Pre-computed tranche indices sorted by loss allocation order:
    /// equity (first loss) → subordinated → mezzanine → senior.
    /// Computed once, reused every period.
    loss_alloc_order: Vec<usize>,
    /// Recovery rate from the deal's recovery specification.
    recovery_rate: f64,
    /// Current reserve account balance.
    reserve_balance: Money,
}

impl<'a> SimulationState<'a> {
    fn new(
        pool: &'a Pool,
        tranches: &'a TrancheStructure,
        closing_date: Date,
        recovery_lag_months: u32,
        recovery_rate: f64,
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
                        writedown_flows: Vec::new(),
                        final_balance: t.current_balance,
                        total_interest: Money::new(0.0, base_ccy),
                        total_principal: Money::new(0.0, base_ccy),
                        total_pik: Money::new(0.0, base_ccy),
                        total_writedown: Money::new(0.0, base_ccy),
                    },
                )
            })
            .collect();

        let tranche_balances: HashMap<String, Money> = tranches
            .tranches
            .iter()
            .map(|t| (t.id.to_string(), t.current_balance))
            .collect();

        // Map each tranche to its waterfall distribution key.
        // Equity tranches receive residual via RecipientType::Equity in the
        // standard waterfall, so their key must match that variant.
        let tranche_recipient_keys: Vec<RecipientType> = tranches
            .tranches
            .iter()
            .map(|t| {
                if t.seniority == TrancheSeniority::Equity {
                    RecipientType::Equity
                } else {
                    RecipientType::Tranche(t.id.to_string())
                }
            })
            .collect();

        // Initialize PoolState
        // Note: For now we convert the full asset list to PoolState.
        // Future optimization: Support RepLine conversion to PoolState.
        let pool_state = PoolState::from_pool(pool);

        let deferred_interest: HashMap<String, Money> = tranches
            .tranches
            .iter()
            .map(|t| (t.id.to_string(), Money::new(0.0, base_ccy)))
            .collect();

        // Determine if reinvestment is initially active
        let initial_reinvestment_active = pool
            .reinvestment_period
            .as_ref()
            .is_some_and(|period| closing_date <= period.end_date);

        let total_pool_balance = pool.total_balance().unwrap_or(Money::new(0.0, base_ccy));

        // Performing balance excludes pre-defaulted assets. Used as denominator
        // for loss allocation — pre-defaulted assets are already priced into the
        // deal structure and should not trigger additional write-downs.
        let performing_pool_balance = pool.performing_balance().unwrap_or(total_pool_balance);

        // Pre-compute loss allocation order once: equity first → senior last.
        // Seniority enum: Senior=0, Mezzanine=1, Subordinated=2, Equity=3.
        // Sort descending so Equity (3) comes first.
        let mut loss_alloc_order: Vec<usize> = (0..tranches.tranches.len()).collect();
        loss_alloc_order.sort_by(|&a, &b| {
            tranches.tranches[b]
                .seniority
                .cmp(&tranches.tranches[a].seniority)
        });

        Self {
            pool_state,
            pool_outstanding: total_pool_balance,
            recovery_queue: RecoveryQueue::new(),
            tranche_balances,
            deferred_interest,
            results,
            prev_date: Some(closing_date),
            base_ccy,
            recovery_lag_months,
            pool,
            tranches,
            closing_date,
            tranche_recipient_keys,
            was_reinvestment_active: initial_reinvestment_active,
            cumulative_expected_loss: 0.0,
            total_pool_balance,
            performing_pool_balance,
            loss_alloc_order,
            recovery_rate,
            reserve_balance: pool.reserve_account,
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
                    res.detailed_flows.push(CashFlow {
                        date: *date,
                        reset_date: None,
                        amount: *amount,
                        kind: CFKind::Fixed,
                        accrual_factor: 0.0,
                        rate: None,
                    });
                }
            }
            for (date, amount) in &res.principal_flows {
                if amount.amount() > 0.0 {
                    res.detailed_flows.push(CashFlow {
                        date: *date,
                        reset_date: None,
                        amount: *amount,
                        kind: CFKind::Amortization,
                        accrual_factor: 0.0,
                        rate: None,
                    });
                }
            }
            // M5: Include write-down flows in detailed_flows so NPV and
            // risk analytics capture the full economic picture.
            // Write-downs represent permanent loss of notional and are
            // classified as DefaultedNotional (negative = loss to holder).
            for (date, amount) in &res.writedown_flows {
                if amount.amount() > 0.0 {
                    res.detailed_flows.push(CashFlow {
                        date: *date,
                        reset_date: None,
                        amount: Money::new(-amount.amount(), amount.currency()),
                        kind: CFKind::DefaultedNotional,
                        accrual_factor: 0.0,
                        rate: None,
                    });
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
///
/// Period execution order matches INTEX/Bloomberg convention:
///   1. Calculate pool cashflows (interest, principal, default, recovery)
///   2. Allocate losses through capital structure (using expected loss at default)
///   3. Execute waterfall on post-loss tranche balances
///   4. Record cashflows and update tranche balances
///   5. Update pool balance
///
/// Loss allocation uses **expected net loss** = default * (1 - recovery_rate),
/// applied at the point of default. This decouples loss recognition from cash
/// timing of recovery receipts (which are lagged). Recoveries still flow through
/// the waterfall as cash when they mature from the recovery queue.
fn simulate_period(
    state: &mut SimulationState,
    instrument: &StructuredCredit,
    waterfall: &Waterfall,
    pay_date: Date,
    context: &MarketContext,
    months_per_period: f64,
) -> Result<()> {
    let seasoning_months = state.closing_date.months_until(pay_date);

    // Capture period start before updating prev_date (for accrual calculations)
    let period_start = state.prev_date.unwrap_or(state.closing_date);

    // Reinvestment logic -- determined before pool flows so reconciliation
    // can snap pool_outstanding to the correct pre-flow asset balances.
    let is_reinvestment_active = state
        .pool
        .reinvestment_period
        .as_ref()
        .is_some_and(|period| pay_date <= period.end_date);

    // Reconciliation: When reinvestment transitions from active → inactive,
    // snap pool_outstanding to the actual sum of asset balances BEFORE this
    // period's flows are applied. During the reinvestment period,
    // pool_outstanding is reduced only by defaults (gross), which can cause
    // it to diverge from the true sum of asset-level balances (e.g. due to
    // matured assets, rounding, or partial defaults). This one-time
    // reconciliation eliminates the phantom balance at the transition point.
    //
    // Must happen before calculate_pool_flows so that Step 4's normal
    // subtraction of this period's flows is applied to the correct base.
    if state.was_reinvestment_active && !is_reinvestment_active {
        let actual_sum: f64 = state.pool_state.balances.iter().sum();
        state.pool_outstanding = Money::new(actual_sum.max(0.0), state.base_ccy);
    }
    state.was_reinvestment_active = is_reinvestment_active;

    // ── Step 1: Calculate pool cashflows for the period ──────────────
    let pool_flows = calculate_pool_flows(
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
    state
        .recovery_queue
        .add_recovery(pay_date, pool_flows.recovery);

    // Release matured recoveries (these become cash for waterfall distribution)
    let released_recoveries = state.recovery_queue.release_matured(
        pay_date,
        state.recovery_lag_months,
        state.base_ccy,
    )?;

    // ── Step 2: Loss allocation through capital structure ────────────
    //
    // INTEX/Moody's Analytics convention: allocate expected net loss at the
    // point of default, NOT when lagged recoveries arrive. This ensures:
    //   - Tranche balances reflect economic reality before the waterfall runs
    //   - Interest accrues only on non-impaired notional
    //   - OC/IC coverage tests see correct post-loss balances
    //   - No risk of paying interest on subsequently written-down principal
    //
    // Expected net loss = default_amount * (1 - recovery_rate)
    // This is a permanent, irreversible write-down.
    let period_expected_loss = pool_flows.default.amount() * (1.0 - state.recovery_rate);
    state.cumulative_expected_loss += period_expected_loss;

    if state.cumulative_expected_loss > WRITEDOWN_DE_MINIMIS
        && state.performing_pool_balance.amount() > 0.0
    {
        // Allocate cumulative expected loss bottom-up using pre-computed order
        let mut remaining_loss = state.cumulative_expected_loss;
        // Clone loss_alloc_order to avoid borrow conflict with state
        let loss_order = state.loss_alloc_order.clone();
        for &idx in &loss_order {
            if remaining_loss <= WRITEDOWN_DE_MINIMIS {
                break;
            }
            let tranche = &state.tranches.tranches[idx];
            let tranche_id_str = tranche.id.as_str();
            let original_balance = tranche.original_balance.amount();

            // This tranche's share of cumulative loss (capped at original face)
            let target_loss = remaining_loss.min(original_balance);
            remaining_loss -= target_loss;

            // Incremental write-down: only record the increase over prior periods
            let already_written_down = state
                .results
                .get(tranche_id_str)
                .map(|r| r.total_writedown.amount())
                .unwrap_or(0.0);

            let incremental = (target_loss - already_written_down).max(0.0);
            if incremental > WRITEDOWN_DE_MINIMIS {
                // Reduce tranche balance BEFORE waterfall execution
                if let Some(current_balance) = state.tranche_balances.get_mut(tranche_id_str) {
                    let new_balance = (current_balance.amount() - incremental).max(0.0);
                    *current_balance = Money::new(new_balance, state.base_ccy);
                }

                let writedown = Money::new(incremental, state.base_ccy);
                if let Some(res) = state.results.get_mut(tranche_id_str) {
                    res.writedown_flows.push((pay_date, writedown));
                    res.total_writedown = res.total_writedown.checked_add(writedown)?;
                }
            }
        }
    }

    // ── Step 3: Prepare waterfall inputs ─────────────────────────────
    // Total principal from pool (scheduled + prepayment)
    let total_principal_from_pool = pool_flows
        .scheduled_principal
        .checked_add(pool_flows.prepayment)?;

    // During reinvestment, principal collections are reinvested into new assets.
    // Recoveries are CASH and always flow through the waterfall.
    let principal_available_for_waterfall = if is_reinvestment_active {
        released_recoveries
    } else {
        total_principal_from_pool.checked_add(released_recoveries)?
    };

    let total_cash_for_waterfall = pool_flows
        .interest
        .checked_add(principal_available_for_waterfall)?;

    // ── Step 4: Execute Waterfall on post-loss balances ──────────────
    let waterfall_context =
        crate::instruments::fixed_income::structured_credit::pricing::waterfall::WaterfallContext {
            available_cash: total_cash_for_waterfall,
            interest_collections: pool_flows.interest,
            payment_date: pay_date,
            period_start,
            pool_balance: state.pool_outstanding,
            market: context,
            tranche_balances: Some(&state.tranche_balances),
            reserve_balance: state.reserve_balance,
            recovery_proceeds: released_recoveries,
        };

    let waterfall_result =
        crate::instruments::fixed_income::structured_credit::pricing::waterfall::execute_waterfall(
            waterfall,
            state.tranches,
            state.pool,
            waterfall_context,
        )?;

    // Update reserve balance from waterfall distributions to ReserveAccount recipients.
    for (recipient, amount) in &waterfall_result.distributions {
        if let RecipientType::ReserveAccount(_) = recipient {
            state.reserve_balance = state.reserve_balance.checked_add(*amount)?;
        }
    }

    // ── Step 5: Record flows and update balances ─────────────────────
    for (idx, tranche) in state.tranches.tranches.iter().enumerate() {
        let recipient_key = &state.tranche_recipient_keys[idx];
        let tranche_id_str = tranche.id.as_str();

        let current_balance = state
            .tranche_balances
            .get(tranche_id_str)
            .copied()
            .unwrap_or(Money::new(0.0, state.base_ccy));
        let coupon_rate = tranche
            .coupon
            .try_current_rate_with_index(pay_date, context)?;

        // Use tranche's day-count convention for proper accrual calculation
        let accrual_factor =
            tranche
                .day_count
                .year_fraction(period_start, pay_date, DayCountCtx::default())?;

        // Interest due on post-writedown balance (correct: no interest on
        // written-down notional). Deferred interest is tracked separately
        // and already reflected in the tranche balance via PIK accretion.
        let interest_due = Money::new(
            current_balance.amount() * coupon_rate * accrual_factor,
            state.base_ccy,
        );

        let payment_received = waterfall_result
            .distributions
            .get(recipient_key)
            .copied()
            .unwrap_or(Money::new(0.0, state.base_ccy));

        // Determine how much interest was actually paid vs. shortfall (PIK)
        let interest_paid = if payment_received.amount() >= interest_due.amount() {
            interest_due
        } else {
            payment_received
        };

        let interest_shortfall = Money::new(
            (interest_due.amount() - interest_paid.amount()).max(0.0),
            state.base_ccy,
        );

        let principal_payment = payment_received
            .checked_sub(interest_paid)
            .unwrap_or(Money::new(0.0, state.base_ccy));

        if let Some(res) = state.results.get_mut(tranche_id_str) {
            if payment_received.amount() > 0.0 {
                res.cashflows.push((pay_date, payment_received));
            }
            if interest_paid.amount() > 0.0 {
                res.interest_flows.push((pay_date, interest_paid));
                res.total_interest = res.total_interest.checked_add(interest_paid)?;
            }
            if principal_payment.amount() > 0.0 {
                res.principal_flows.push((pay_date, principal_payment));
                res.total_principal = res.total_principal.checked_add(principal_payment)?;
            }
            // Record PIK (interest shortfall deferred to future periods)
            if interest_shortfall.amount() > 0.0 {
                res.pik_flows.push((pay_date, interest_shortfall));
                res.total_pik = res.total_pik.checked_add(interest_shortfall)?;
            }
        }

        // Update deferred interest: accumulate shortfall, clear if fully paid
        let existing_deferred = state
            .deferred_interest
            .get(tranche_id_str)
            .copied()
            .unwrap_or(Money::new(0.0, state.base_ccy));
        state.deferred_interest.insert(
            tranche_id_str.to_string(),
            existing_deferred.checked_add(interest_shortfall)?,
        );

        // Update tranche balance:
        // - Always reduce by principal payment
        // - Only accrete shortfall if PIK is explicitly enabled for this tranche
        //
        // Standard CLO/ABS indenture: shortfalls are tracked as deferred interest
        // and paid from future interest collections, NOT capitalized into balance.
        // PIK accretion (capitalizing shortfall) is an explicit structural feature
        // that must be opted into per tranche.
        if let Some(current) = state.tranche_balances.get_mut(tranche_id_str) {
            let after_principal = current.checked_sub(principal_payment).unwrap_or(*current);
            if tranche.pik_enabled && interest_shortfall.amount() > 0.0 {
                *current = after_principal.checked_add(interest_shortfall)?;
            } else {
                *current = after_principal;
            }
        }
    }

    // ── Step 6: Update pool balance ──────────────────────────────────
    if is_reinvestment_active {
        // During reinvestment, principal is recycled into new assets.
        // Pool balance drops only by defaults (gross).
        state.pool_outstanding = state.pool_outstanding.checked_sub(pool_flows.default)?;
    } else {
        // After reinvestment, all principal reductions hit pool balance.
        state.pool_outstanding = state
            .pool_outstanding
            .checked_sub(total_principal_from_pool)?
            .checked_sub(pool_flows.default)?;
    }

    Ok(())
}

// ============================================================================
// CALCULATION HELPERS
// ============================================================================

/// Pool flow results for a single period.
struct PoolFlows {
    interest: Money,
    scheduled_principal: Money,
    prepayment: Money,
    default: Money,
    recovery: Money,
}

/// Calculate all pool flows for the period.
///
/// Implements:
/// - M1: Scheduled amortization for amortizing assets (mortgages, auto, etc.)
/// - M3: Maturity/balloon payment when an asset reaches maturity
/// - m2: Sequential default-then-prepay application (market convention)
fn calculate_pool_flows(
    state: &mut SimulationState,
    instrument: &StructuredCredit,
    pay_date: Date,
    prev_date: Date,
    seasoning_months: u32,
    months_per_period: f64,
    context: &MarketContext,
) -> Result<PoolFlows> {
    let base_ccy = state.base_ccy;
    let mut total_interest = Money::new(0.0, base_ccy);
    let mut total_scheduled = Money::new(0.0, base_ccy);
    let mut total_prepay = Money::new(0.0, base_ccy);
    let mut total_default = Money::new(0.0, base_ccy);
    let mut total_recovery = Money::new(0.0, base_ccy);

    // Pre-calculate global rates
    let smm = instrument.calculate_prepayment_rate(pay_date, seasoning_months);
    let mdr = instrument.calculate_default_rate(pay_date, seasoning_months);
    let recovery_rate = instrument.credit_model.recovery_spec.rate;

    let global_period_smm = 1.0 - (1.0 - smm).powf(months_per_period);
    let global_period_mdr = 1.0 - (1.0 - mdr).powf(months_per_period);

    // Pre-resolve all curves
    let mut resolved_rates = Vec::with_capacity(state.pool_state.unique_curves.len());
    for idx_str in &state.pool_state.unique_curves {
        let fwd = context.get_forward(idx_str)?;
        let base = fwd.base_date();
        let dc = fwd.day_count();
        let t2 = dc.year_fraction(base, pay_date, DayCountCtx::default())?;
        let tenor = fwd.tenor();
        let t1 = (t2 - tenor).max(0.0);
        let r = if t2 > 0.0 && t1 < t2 {
            fwd.rate_period(t1, t2)
        } else {
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

        // Skip already-defaulted assets: prevents pre-existing defaulted assets
        // (e.g. assets that entered the pool in workout) from accruing interest,
        // defaulting again, or prepaying. Also guards against assets marked as
        // fully defaulted during simulation.
        if state.pool_state.is_defaulted[i] {
            continue;
        }

        // 1. Interest -- computed first so matured assets still pay their final coupon
        let rate = if let Some(curve_idx) = state.pool_state.curve_indices[i] {
            let base_rate = resolved_rates[curve_idx];
            base_rate + (state.pool_state.spread_bps[i].unwrap_or(0.0).max(0.0) / 10_000.0)
        } else {
            state.pool_state.rates[i]
        };

        // m-FINAL-1: Cap interest accrual at asset maturity for mid-period maturities.
        // If the asset matures between prev_date and pay_date, accrue interest only up
        // to the maturity date, not the full period end.
        let interest_end = state.pool_state.maturities[i].min(pay_date);

        let accrual_factor = state.pool_state.day_counts[i]
            .unwrap_or(DayCount::Act360)
            .year_fraction(prev_date, interest_end, DayCountCtx::default())?;

        let interest = Money::new(balance * rate * accrual_factor, base_ccy);
        total_interest = total_interest.checked_add(interest)?;

        // M3: Check maturity -- if asset has matured, return remaining balance as
        // a balloon payment and zero out the asset. Interest was already computed above
        // (capped at maturity date).
        if pay_date >= state.pool_state.maturities[i] {
            let balloon = Money::new(balance, base_ccy);
            total_scheduled = total_scheduled.checked_add(balloon)?;
            state.pool_state.balances[i] = 0.0;
            continue;
        }

        // M1: Scheduled amortization for amortizing assets
        let scheduled_principal = if state.pool_state.is_amortizing[i] && rate > 0.0 {
            // Compute level payment using period-native amortization math:
            // period_rate = annual_rate * months_per_period / 12
            // remaining_periods = remaining_months / months_per_period
            // level_payment = P * r_p / (1 - (1+r_p)^-n_p)
            let remaining_days = (state.pool_state.maturities[i] - pay_date)
                .whole_days()
                .max(1) as f64;
            let remaining_months = (remaining_days / 30.44).round().max(1.0);

            let period_rate = rate * months_per_period / 12.0;
            let remaining_periods_f64 = remaining_months / months_per_period;
            let denom = 1.0 - (1.0 + period_rate).powf(-remaining_periods_f64);

            let period_payment = if denom.abs() > 1e-12 && remaining_periods_f64 > 0.0 {
                balance * period_rate / denom
            } else {
                // If denominator is ~0 (very short term), return full balance
                balance
            };

            // Scheduled principal = level payment - interest (for this period)
            (period_payment - balance * period_rate)
                .max(0.0)
                .min(balance)
        } else {
            0.0
        };

        total_scheduled = total_scheduled.checked_add(Money::new(scheduled_principal, base_ccy))?;

        // Balance after scheduled amortization
        let balance_after_sched = balance - scheduled_principal;

        // m2 fix: Apply default first, then prepayment to post-default balance
        // (market convention per Intex/Moody's Analytics)
        let period_mdr = if let Some(mdr) = state.pool_state.mdr_overrides[i] {
            1.0 - (1.0 - mdr).powf(months_per_period)
        } else {
            global_period_mdr
        };

        let default_amt = balance_after_sched * period_mdr;
        let balance_after_default = balance_after_sched - default_amt;

        let period_smm = if let Some(smm) = state.pool_state.smm_overrides[i] {
            1.0 - (1.0 - smm).powf(months_per_period)
        } else {
            global_period_smm
        };

        let prepay_amt = balance_after_default * period_smm;
        let recovery_amt = default_amt * recovery_rate;

        total_prepay = total_prepay.checked_add(Money::new(prepay_amt, base_ccy))?;
        total_default = total_default.checked_add(Money::new(default_amt, base_ccy))?;
        total_recovery = total_recovery.checked_add(Money::new(recovery_amt, base_ccy))?;

        // Mark asset as fully defaulted if default consumed (nearly) all remaining balance.
        // Uses relative tolerance: 1 - 1e-10 catches floating-point imprecision when
        // the MDR is effectively 100%, without false positives from small balances.
        // Guard on balance_after_sched > 0 prevents marking fully-amortized assets
        // (where both sides are 0.0) as "defaulted" when they actually paid down normally.
        if balance_after_sched > 0.0 && default_amt >= balance_after_sched * (1.0 - 1e-10) {
            state.pool_state.is_defaulted[i] = true;
        }

        // Update balance
        let new_balance = balance_after_default - prepay_amt;
        state.pool_state.balances[i] = new_balance.max(0.0);
    }

    Ok(PoolFlows {
        interest: total_interest,
        scheduled_principal: total_scheduled,
        prepayment: total_prepay,
        default: total_default,
        recovery: total_recovery,
    })
}
