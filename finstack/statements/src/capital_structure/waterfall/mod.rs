//! Cash Flow Waterfall & Sweep Mechanics
//!
//! This module implements dynamic cash flow allocation according to priority of payments,
//! excess cash flow sweeps, and PIK toggles based on model results.
//!
//! # Sign Conventions
//!
//! The ECF (Excess Cash Flow) sweep calculation follows standard LBO model conventions:
//!
//! ## Input Nodes
//!
//! - **EBITDA** (`ebitda_node`): Positive value representing operating cash generation.
//!   Example: $10M EBITDA → use `10_000_000.0`
//!
//! - **Taxes** (`taxes_node`): Positive value representing cash tax payments (outflow).
//!   Example: $2M taxes paid → use `2_000_000.0` (not negative)
//!
//! - **CapEx** (`capex_node`): Positive value representing capital expenditures (outflow).
//!   Example: $1.5M capex → use `1_500_000.0` (not negative)
//!
//! - **Working Capital** (`working_capital_node`): Signed value representing change in NWC.
//!   - Positive = cash consumed (increase in receivables/inventory)
//!   - Negative = cash released (increase in payables)
//!   - Example: $500K increase in NWC → use `500_000.0`
//!
//! ## ECF Calculation
//!
//! ```text
//! ECF = EBITDA - Taxes - CapEx - Working_Capital_Change - Cash_Interest
//! Sweep = max(0, ECF × sweep_percentage)
//! ```
//!
//! The `cash_interest_node` is optional. Per S&P LCD / standard LPA definitions,
//! ECF should include a cash interest deduction. Set it to include this deduction.
//!
//! The sweep is floored at zero (cannot sweep negative cash flow) and then
//! applied as additional principal prepayment to the target instrument.
//!
//! ## Example
//!
//! ```text
//! EBITDA:    $10,000,000  (positive)
//! Taxes:     $ 2,000,000  (positive = outflow)
//! CapEx:     $ 1,500,000  (positive = outflow)
//! ΔWC:       $   500,000  (positive = cash used)
//! ─────────────────────────────────
//! ECF:       $ 6,000,000
//! Sweep @50%: $ 3,000,000 → applied to debt prepayment
//! ```

mod cash_distribution;
mod excess_cash_flow;
mod payment_in_kind;
mod payment_stack;
mod period_close;

use crate::capital_structure::cashflows::CashflowBreakdown;
use crate::capital_structure::state::CapitalStructureState;
use crate::capital_structure::waterfall_spec::{PaymentPriority, WaterfallSpec};
use crate::error::Result;
use crate::evaluator::EvaluationContext;
use finstack_core::dates::PeriodId;
use finstack_core::money::Money;
use indexmap::IndexMap;
use std::collections::HashSet;

use cash_distribution::{allocate_pro_rata, apply_cash_cap_to_category, StagedInstrumentFlow};
use excess_cash_flow::calculate_ecf_sweep;
use payment_in_kind::{evaluate_pik_toggle, is_pik_enabled};
use payment_stack::{extra_principal_priority, priority_index, waterfall_currency};
use period_close::update_cumulative_metrics;

/// Evaluate a node reference or inline DSL expression against the current context.
fn eval_value_or_formula(context: &EvaluationContext, expr: &str) -> Result<f64> {
    if let Ok(value) = context.get_value(expr) {
        return Ok(value);
    }
    let compiled = crate::dsl::parse_and_compile(expr)?;
    let mut scratch = context.clone();
    crate::evaluator::formula::evaluate_formula(&compiled, &mut scratch, None)
}

/// Execute waterfall logic for a single period.
///
/// This function:
/// 1. Checks PIK toggle conditions and updates interest mode
/// 2. Calculates contractual flows (interest, amortization)
/// 3. Calculates ECF and applies sweep if configured
/// 4. Allocates available cash according to priority stack
///
/// # Arguments
///
/// * `period_id` - Current period being evaluated
/// * `context` - Evaluation context with model results
/// * `waterfall_spec` - Waterfall configuration
/// * `state` - Current capital structure state (opening balances, etc.)
/// * `contractual_flows` - Pre-calculated contractual flows by instrument
///
/// # Returns
///
/// Returns per-instrument cashflow breakdowns after sweep, PIK, and
/// priority-of-payments allocation have been applied. `state` is updated
/// in-place with opening/closing balances and cumulative tracking fields.
///
/// # Errors
///
/// Returns an error if required statement nodes are missing, if the waterfall
/// references inconsistent currencies, or if sweep / PIK calculations fail.
///
/// # References
///
/// - Fixed-income capital structure context: `docs/REFERENCES.md#tuckman-serrat-fixed-income`
pub fn execute_waterfall(
    _period_id: &PeriodId,
    context: &EvaluationContext,
    waterfall_spec: &WaterfallSpec,
    state: &mut CapitalStructureState,
    contractual_flows: &IndexMap<String, CashflowBreakdown>,
) -> Result<IndexMap<String, CashflowBreakdown>> {
    let _span = tracing::info_span!(
        "statements.capital_structure.waterfall",
        period = _period_id.to_string(),
        instruments = contractual_flows.len(),
        has_sweep = waterfall_spec.ecf_sweep.is_some(),
        has_pik_toggle = waterfall_spec.pik_toggle.is_some()
    )
    .entered();

    let mut result = IndexMap::new();
    let cash_currency = waterfall_currency(contractual_flows)?;

    // --- resolve priority positions ---
    let interest_priority = priority_index(
        &waterfall_spec.priority_of_payments,
        PaymentPriority::Interest,
    );
    let fees_priority = priority_index(&waterfall_spec.priority_of_payments, PaymentPriority::Fees);
    let extra_principal_priority = extra_principal_priority(&waterfall_spec.priority_of_payments);
    let equity_priority = priority_index(
        &waterfall_spec.priority_of_payments,
        PaymentPriority::Equity,
    );

    // --- Step 1: PIK toggle ---
    //
    // Evaluate PIK mode BEFORE ECF so that ECF cash-interest deduction
    // correctly reflects actual cash interest paid (instruments in PIK mode
    // pay zero cash interest that period).
    let (pik_enable, pik_targets): (Option<bool>, Option<HashSet<String>>) =
        if let Some(pik_spec) = &waterfall_spec.pik_toggle {
            (
                Some(evaluate_pik_toggle(context, pik_spec)?),
                pik_spec
                    .target_instrument_ids
                    .as_ref()
                    .map(|ids| ids.iter().cloned().collect()),
            )
        } else {
            (None, None)
        };

    // Apply PIK mode transitions to `state` before using it for ECF.
    let min_periods_in_pik = waterfall_spec
        .pik_toggle
        .as_ref()
        .map(|spec| spec.min_periods_in_pik)
        .unwrap_or(0);

    if let Some(enable_pik) = pik_enable {
        for instrument_id in contractual_flows.keys() {
            let should_apply = pik_targets
                .as_ref()
                .map(|set| set.contains(instrument_id))
                .unwrap_or(true);
            if !should_apply {
                continue;
            }
            let periods_active = state
                .pik_periods_active
                .get(instrument_id.as_str())
                .copied()
                .unwrap_or(0);
            let currently_pik = state
                .pik_mode
                .get(instrument_id.as_str())
                .copied()
                .unwrap_or(false);
            let effective_pik =
                if currently_pik && !enable_pik && periods_active < min_periods_in_pik {
                    true
                } else {
                    enable_pik
                };
            state
                .pik_mode
                .insert(instrument_id.to_string(), effective_pik);
            state.pik_periods_active.insert(
                instrument_id.to_string(),
                if effective_pik { periods_active + 1 } else { 0 },
            );
        }
    }

    // --- Step 2: ECF / sweep ---
    //
    // ECF is PIK-aware: when `cash_interest_node` is omitted, the fallback
    // deducts contractual cash interest only for instruments NOT in PIK mode
    // this period.
    let sweep_amount = if let Some(ecf_spec) = &waterfall_spec.ecf_sweep {
        calculate_ecf_sweep(context, ecf_spec, contractual_flows, state)?
    } else {
        Money::new(0.0, cash_currency)
    };
    let available_cash = if let Some(available_cash_node) = &waterfall_spec.available_cash_node {
        let cash = eval_value_or_formula(context, available_cash_node)?;
        Some(Money::new(cash.max(0.0), cash_currency))
    } else {
        None
    };

    // --- Step 3: Build staged per-instrument state ---
    //
    // Execution order per standard loan documentation:
    //   1. Determine sweep amount (already computed above)
    //   2. Apply sweep as additional principal prepayment
    //   3. Update balance after sweep + scheduled amortization
    //   4. Capitalize PIK interest into the closing balance when appropriate
    let mut staged: Vec<StagedInstrumentFlow> = Vec::with_capacity(contractual_flows.len());
    for (instrument_id, breakdown) in contractual_flows {
        let currency = breakdown.interest_expense_cash.currency();
        let opening_balance = state.get_opening_balance(instrument_id, currency);

        let staged_breakdown = breakdown.clone();
        staged.push(StagedInstrumentFlow {
            instrument_id: instrument_id.clone(),
            breakdown: staged_breakdown,
            opening_balance,
            extra_principal: Money::new(0.0, currency),
            scheduled_principal: breakdown.principal_payment,
        });
    }

    // --- Step 4: Distribute sweep across instruments ---
    let mut remaining_sweep = if equity_priority < extra_principal_priority {
        Money::new(0.0, sweep_amount.currency())
    } else {
        sweep_amount
    };

    if fees_priority < extra_principal_priority {
        let total_fees = staged
            .iter()
            .map(|s| s.breakdown.fees.amount())
            .sum::<f64>();
        remaining_sweep = Money::new(
            (remaining_sweep.amount() - total_fees).max(0.0),
            remaining_sweep.currency(),
        );
    }
    // Note: no separate interest-priority deduction from `remaining_sweep`
    // here. When an ECF sweep is configured, `calculate_ecf_sweep` already
    // deducts cash interest from EBITDA. When no ECF sweep is configured,
    // `sweep_amount` is zero, so `remaining_sweep` starts at zero and any
    // subtraction is a no-op. `interest_priority` is still used below in
    // Step 6 to decide whether PIK capitalization has already been applied.

    let target_instrument_id = waterfall_spec
        .ecf_sweep
        .as_ref()
        .and_then(|spec| spec.target_instrument_id.as_deref());
    let mut extra_capacity: IndexMap<String, f64> = IndexMap::new();
    let mut total_extra_capacity = 0.0;
    for s in &staged {
        let eligible = if let Some(target_id) = target_instrument_id {
            target_id == s.instrument_id
        } else {
            true
        };
        if !eligible || extra_principal_priority == usize::MAX {
            extra_capacity.insert(s.instrument_id.clone(), 0.0);
            continue;
        }

        let capacity = (s.opening_balance.amount() - s.scheduled_principal.amount()).max(0.0);

        total_extra_capacity += capacity;
        extra_capacity.insert(s.instrument_id.clone(), capacity);
    }

    // Two-pass approach: compute all proportional shares first, then apply.
    // This avoids the bug where mutating remaining_sweep during iteration
    // gives incorrect proportions to instruments after the first.
    let sweep_currency = remaining_sweep.currency();
    let sweep_total = remaining_sweep.amount();
    let staged_len = staged.len();
    let mut sweep_allocations: Vec<f64> = vec![0.0; staged_len];

    for (idx, s) in staged.iter().enumerate() {
        let currency = s.breakdown.interest_expense_cash.currency();

        sweep_allocations[idx] =
            if extra_principal_priority == usize::MAX || sweep_currency != currency {
                0.0
            } else if let Some(target_id) = target_instrument_id {
                if target_id == s.instrument_id {
                    let capacity = *extra_capacity.get(s.instrument_id.as_str()).unwrap_or(&0.0);
                    sweep_total.min(capacity)
                } else {
                    0.0
                }
            } else {
                let capacity = *extra_capacity.get(s.instrument_id.as_str()).unwrap_or(&0.0);
                if total_extra_capacity <= 0.0 || capacity <= 0.0 {
                    0.0
                } else {
                    let proportional = sweep_total * (capacity / total_extra_capacity);
                    proportional.min(capacity)
                }
            };
    }

    // Handle rounding residual: assign to the last eligible instrument
    let allocated_total: f64 = sweep_allocations.iter().sum();
    let residual = sweep_total - allocated_total;
    if residual.abs() > f64::EPSILON {
        for idx in (0..staged_len).rev() {
            let capacity = *extra_capacity
                .get(staged[idx].instrument_id.as_str())
                .unwrap_or(&0.0);
            if sweep_allocations[idx] > 0.0 || capacity > 0.0 {
                sweep_allocations[idx] = (sweep_allocations[idx] + residual).min(capacity).max(0.0);
                break;
            }
        }
    }

    // Second pass: apply computed shares
    for (idx, s) in staged.iter_mut().enumerate() {
        let currency = s.breakdown.interest_expense_cash.currency();
        s.extra_principal = Money::new(sweep_allocations[idx], currency);
        remaining_sweep = remaining_sweep.checked_sub(s.extra_principal)?;
        s.breakdown.principal_payment = s.scheduled_principal.checked_add(s.extra_principal)?;
    }

    // --- Step 4b: Apply PIK mode (post-sweep) ---
    // PIK staging is deferred until after sweep allocation so that PIK
    // accrues on the post-sweep balance, not the pre-sweep contractual amount.
    for s in &mut staged {
        let currency = s.breakdown.interest_expense_cash.currency();
        if is_pik_enabled(state, &s.instrument_id)
            && interest_priority < extra_principal_priority
            && extra_principal_priority != usize::MAX
        {
            s.breakdown.interest_expense_pik += s.breakdown.interest_expense_cash;
            s.breakdown.interest_expense_cash = Money::new(0.0, currency);
        }
    }

    // --- Step 5: Available cash caps ---
    //
    // The three prepayment priorities (MandatoryPrepayment, VoluntaryPrepayment,
    // Sweep) all share the single `extra_principal` bucket populated from the
    // ECF sweep in Step 4. Because there is only one bucket, the first of the
    // three that appears in `priority_of_payments` consumes the cash cap for
    // that bucket; later entries are no-ops. Modelers who need strict ordering
    // across distinct prepayment types should populate separate buckets
    // upstream (not currently supported by this engine) and distinguish them
    // via separate `target_instrument_id`s.
    if let Some(mut remaining_cash) = available_cash {
        let mut extra_principal_capped = false;
        for priority in &waterfall_spec.priority_of_payments {
            match priority {
                PaymentPriority::Fees => {
                    apply_cash_cap_to_category(&mut staged, &mut remaining_cash, |s| {
                        &mut s.breakdown.fees
                    });
                }
                PaymentPriority::Interest => {
                    apply_cash_cap_to_category(&mut staged, &mut remaining_cash, |s| {
                        &mut s.breakdown.interest_expense_cash
                    });
                }
                PaymentPriority::Amortization => {
                    let planned: Vec<f64> = staged
                        .iter()
                        .map(|s| s.scheduled_principal.amount().max(0.0))
                        .collect();
                    let allocations = allocate_pro_rata(&planned, &mut remaining_cash);
                    for (s, allocated) in staged.iter_mut().zip(allocations.into_iter()) {
                        s.scheduled_principal =
                            Money::new(allocated, s.scheduled_principal.currency());
                    }
                }
                PaymentPriority::MandatoryPrepayment
                | PaymentPriority::VoluntaryPrepayment
                | PaymentPriority::Sweep => {
                    if extra_principal_capped {
                        continue;
                    }
                    let planned: Vec<f64> = staged
                        .iter()
                        .map(|s| s.extra_principal.amount().max(0.0))
                        .collect();
                    let allocations = allocate_pro_rata(&planned, &mut remaining_cash);
                    for (s, allocated) in staged.iter_mut().zip(allocations.into_iter()) {
                        s.extra_principal = Money::new(allocated, s.extra_principal.currency());
                    }
                    extra_principal_capped = true;
                }
                PaymentPriority::Equity => {}
            }
        }
    }

    // --- Step 6: Period close ---
    //
    // For each instrument:
    //   (a) principal_payment = scheduled + extra, capped at opening_balance.
    //       If the cap truncates the sum, reduce `extra_principal` first
    //       (discretionary sweep is netted before scheduled amortization is
    //       reduced) so downstream accounting stays consistent.
    //   (b) post_sweep_balance = opening - principal_payment (with a small
    //       dust floor to avoid micro-residuals).
    //   (c) PIK capitalization: if the instrument was toggled into PIK mode
    //       AND the waterfall's ordering did not already move cash interest
    //       into the PIK bucket earlier, move it here. PIK interest accrues
    //       on the pre-waterfall opening balance and capitalizes at period
    //       close even when the principal was fully paid down during the
    //       period: the coupon still economically exists and gets rolled
    //       into the closing balance.
    //   (d) closing_balance = post_sweep_balance + PIK capitalized.
    //   (e) accrued_interest: cleared to zero when PIK capitalization
    //       absorbed the contractual coupon into principal, or when the
    //       debt was paid off and no further contractual accrual applies.
    //       Otherwise the field retains the contractual pre-waterfall
    //       accrual.
    for s in staged {
        let StagedInstrumentFlow {
            instrument_id,
            mut breakdown,
            opening_balance,
            extra_principal,
            scheduled_principal,
        } = s;
        let currency = breakdown.interest_expense_cash.currency();

        // (a) Principal cap. `extra_principal` (the discretionary sweep
        // bucket) is netted against the overshoot before scheduled
        // amortization is reduced, so the aggregate `principal_payment` is
        // never > opening_balance.
        let desired = scheduled_principal.checked_add(extra_principal)?;
        let principal_payment = if desired.amount() > opening_balance.amount() {
            opening_balance
        } else {
            desired
        };
        breakdown.principal_payment = principal_payment;

        let post_sweep_balance = opening_balance.checked_sub(principal_payment)?;
        // Dust floor: collapse sub-cent residuals on full paydown. Currency
        // agnostic fallback; modelers in JPY should override via explicit
        // rounding upstream.
        let post_sweep_balance = if post_sweep_balance.amount().abs() < 0.005 {
            Money::new(0.0, post_sweep_balance.currency())
        } else {
            post_sweep_balance
        };
        let fully_paid = post_sweep_balance.amount() == 0.0;

        // (c) PIK capitalization at close. Applies whenever the PIK toggle is
        // active and Step 4b did not already move the coupon into the PIK
        // bucket. Capitalization is independent of whether the principal was
        // fully repaid this period: the coupon accrued on the pre-waterfall
        // opening balance and must be recognized.
        let pik_enabled = is_pik_enabled(state, &instrument_id);
        let deferred_pik_applied = pik_enabled
            && !(interest_priority < extra_principal_priority
                && extra_principal_priority != usize::MAX);
        let pik_capitalized_this_step = if deferred_pik_applied {
            let cash_coupon = breakdown.interest_expense_cash;
            breakdown.interest_expense_pik += cash_coupon;
            breakdown.interest_expense_cash = Money::new(0.0, currency);
            true
        } else {
            // PIK already moved into the PIK bucket in Step 4b (or the
            // instrument is not in PIK mode).
            false
        };

        // (d) Closing balance. PIK capitalizes into the post-sweep balance.
        let closing_balance = post_sweep_balance.checked_add(breakdown.interest_expense_pik)?;
        state.set_closing_balance(instrument_id.to_string(), closing_balance);
        breakdown.debt_balance = closing_balance;

        // (e) Accrued interest bookkeeping after waterfall mutation.
        // The pre-waterfall `accrued_interest` was the contractual schedule's
        // accrual. It is cleared when the coupon was moved into PIK (the
        // accrual has been capitalized into principal) or when the debt was
        // fully paid off and there is no remaining balance to accrue on.
        if fully_paid || pik_capitalized_this_step {
            breakdown.accrued_interest = Money::new(0.0, currency);
        }

        breakdown.validate_currency_invariant().map_err(|e| {
            crate::error::Error::capital_structure(format!(
                "Currency invariant violated after waterfall mutation for {instrument_id}: {e}"
            ))
        })?;

        update_cumulative_metrics(state, &instrument_id, &breakdown, currency)?;

        result.insert(instrument_id.to_string(), breakdown);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capital_structure::{
        CapitalStructureState, CashflowBreakdown, EcfSweepSpec, PaymentPriority, PikToggleSpec,
        WaterfallSpec,
    };
    use finstack_core::currency::Currency;
    use finstack_core::dates::PeriodId;
    use finstack_core::money::Money;
    use indexmap::IndexMap;

    fn build_context(period: PeriodId, values: &[(&str, f64)]) -> EvaluationContext {
        let mut node_to_column = IndexMap::new();
        for (idx, (name, _)) in values.iter().enumerate() {
            node_to_column.insert(crate::types::NodeId::new(*name), idx);
        }
        let mut ctx = EvaluationContext::new(
            period,
            std::sync::Arc::new(node_to_column),
            std::sync::Arc::new(IndexMap::new()),
        );
        for (name, value) in values {
            ctx.set_value(name, *value)
                .expect("sample context should accept provided node values");
        }
        ctx
    }

    #[test]
    fn test_execute_waterfall_applies_ecf_sweep_and_updates_state() {
        let period = PeriodId::quarter(2025, 1);
        let context = build_context(
            period,
            &[
                ("ebitda", 1_000_000.0),
                ("taxes", 200_000.0),
                ("capex", 50_000.0),
            ],
        );

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        let mut tl_breakdown = CashflowBreakdown::with_currency(Currency::USD);
        tl_breakdown.principal_payment = Money::new(100_000.0, Currency::USD);
        contractual_flows.insert("TL-1".to_string(), tl_breakdown);

        let mut state = CapitalStructureState::new();
        state
            .opening_balances
            .insert("TL-1".to_string(), Money::new(10_000_000.0, Currency::USD));

        let waterfall = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Fees,
                PaymentPriority::Interest,
                PaymentPriority::Amortization,
                PaymentPriority::Sweep,
                PaymentPriority::Equity,
            ],
            available_cash_node: None,
            ecf_sweep: Some(EcfSweepSpec {
                ebitda_node: "ebitda".into(),
                taxes_node: Some("taxes".into()),
                capex_node: Some("capex".into()),
                working_capital_node: None,
                cash_interest_node: None,
                sweep_percentage: 0.5,
                target_instrument_id: Some("TL-1".into()),
            }),
            pik_toggle: None,
        };

        let results = execute_waterfall(
            &period,
            &context,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        let tl_result = results.get("TL-1").expect("instrument exists");
        assert_eq!(tl_result.principal_payment.amount(), 475_000.0);
        assert_eq!(
            state
                .closing_balances
                .get("TL-1")
                .expect("closing balance")
                .amount(),
            10_000_000.0 - 475_000.0
        );
        assert_eq!(
            state
                .cumulative_principal
                .get("TL-1")
                .expect("cumulative principal")
                .amount(),
            475_000.0
        );
    }

    #[test]
    fn test_pik_toggle_updates_state() {
        let period = PeriodId::quarter(2025, 2);
        let mut context = build_context(period, &[("liquidity", 50.0)]);

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        contractual_flows.insert(
            "TL-PIK".to_string(),
            CashflowBreakdown::with_currency(Currency::USD),
        );

        let mut state = CapitalStructureState::new();
        state
            .opening_balances
            .insert("TL-PIK".to_string(), Money::new(5_000_000.0, Currency::USD));

        let waterfall = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Fees,
                PaymentPriority::Interest,
                PaymentPriority::Amortization,
                PaymentPriority::Sweep,
                PaymentPriority::Equity,
            ],
            available_cash_node: None,
            ecf_sweep: None,
            pik_toggle: Some(PikToggleSpec {
                liquidity_metric: "liquidity".into(),
                threshold: 100.0,
                target_instrument_ids: Some(vec!["TL-PIK".into()]),
                min_periods_in_pik: 0,
            }),
        };

        execute_waterfall(
            &period,
            &context,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        assert_eq!(state.pik_mode.get("TL-PIK"), Some(&true));

        context
            .set_value("liquidity", 150.0)
            .expect("should update liquidity for second evaluation");
        execute_waterfall(
            &period,
            &context,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        assert_eq!(state.pik_mode.get("TL-PIK"), Some(&false));
    }

    #[test]
    fn test_pik_hysteresis_holds_pik_active_for_min_periods() {
        let period = PeriodId::quarter(2025, 1);

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        let mut tl_breakdown = CashflowBreakdown::with_currency(Currency::USD);
        tl_breakdown.interest_expense_cash = Money::new(10_000.0, Currency::USD);
        contractual_flows.insert("TL-PIK".to_string(), tl_breakdown);

        let mut state = CapitalStructureState::new();
        state
            .opening_balances
            .insert("TL-PIK".to_string(), Money::new(5_000_000.0, Currency::USD));

        let waterfall = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Fees,
                PaymentPriority::Interest,
                PaymentPriority::Amortization,
                PaymentPriority::Sweep,
                PaymentPriority::Equity,
            ],
            available_cash_node: None,
            ecf_sweep: None,
            pik_toggle: Some(PikToggleSpec {
                liquidity_metric: "liquidity".into(),
                threshold: 100.0,
                target_instrument_ids: Some(vec!["TL-PIK".into()]),
                min_periods_in_pik: 3,
            }),
        };

        // Period 1: liquidity < threshold => PIK activates
        let ctx_low = build_context(period, &[("liquidity", 50.0)]);
        execute_waterfall(
            &period,
            &ctx_low,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        assert_eq!(state.pik_mode.get("TL-PIK"), Some(&true));
        assert_eq!(state.pik_periods_active.get("TL-PIK"), Some(&1));

        // Period 2: liquidity recovers above threshold, but hysteresis holds PIK
        let ctx_high = build_context(period, &[("liquidity", 150.0)]);
        execute_waterfall(
            &period,
            &ctx_high,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        assert_eq!(
            state.pik_mode.get("TL-PIK"),
            Some(&true),
            "PIK should remain active due to hysteresis (periods_active=1 < 3)"
        );
        assert_eq!(state.pik_periods_active.get("TL-PIK"), Some(&2));

        // Period 3: still above threshold, hysteresis still holds (periods_active=2 < 3)
        execute_waterfall(
            &period,
            &ctx_high,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        assert_eq!(
            state.pik_mode.get("TL-PIK"),
            Some(&true),
            "PIK should remain active due to hysteresis (periods_active=2 < 3)"
        );
        assert_eq!(state.pik_periods_active.get("TL-PIK"), Some(&3));

        // Period 4: min_periods met (periods_active=3, which is not < 3), PIK releases
        execute_waterfall(
            &period,
            &ctx_high,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        assert_eq!(
            state.pik_mode.get("TL-PIK"),
            Some(&false),
            "PIK should release after min_periods_in_pik completed"
        );
        assert_eq!(
            state.pik_periods_active.get("TL-PIK"),
            Some(&0),
            "counter should reset on PIK exit"
        );
    }

    #[test]
    fn test_execute_waterfall_conserves_sweep_across_multiple_instruments() {
        let period = PeriodId::quarter(2025, 1);
        let context = build_context(
            period,
            &[
                ("ebitda", 1_000_000.0),
                ("taxes", 200_000.0),
                ("capex", 50_000.0),
            ],
        );

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        contractual_flows.insert(
            "TL-1".to_string(),
            CashflowBreakdown::with_currency(Currency::USD),
        );
        contractual_flows.insert(
            "TL-2".to_string(),
            CashflowBreakdown::with_currency(Currency::USD),
        );

        let mut state = CapitalStructureState::new();
        state
            .opening_balances
            .insert("TL-1".to_string(), Money::new(200_000.0, Currency::USD));
        state
            .opening_balances
            .insert("TL-2".to_string(), Money::new(300_000.0, Currency::USD));

        let waterfall = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Fees,
                PaymentPriority::Interest,
                PaymentPriority::Amortization,
                PaymentPriority::Sweep,
                PaymentPriority::Equity,
            ],
            available_cash_node: None,
            ecf_sweep: Some(EcfSweepSpec {
                ebitda_node: "ebitda".into(),
                taxes_node: Some("taxes".into()),
                capex_node: Some("capex".into()),
                working_capital_node: None,
                cash_interest_node: None,
                sweep_percentage: 0.5,
                target_instrument_id: None,
            }),
            pik_toggle: None,
        };

        let results = execute_waterfall(
            &period,
            &context,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        let total_principal = results
            .values()
            .map(|breakdown| breakdown.principal_payment.amount())
            .sum::<f64>();
        let tl1 = results.get("TL-1").expect("TL-1 result");
        let tl2 = results.get("TL-2").expect("TL-2 result");

        assert_eq!(total_principal, 375_000.0);
        assert!((tl1.principal_payment.amount() - 150_000.0).abs() < 1e-9);
        assert!((tl2.principal_payment.amount() - 225_000.0).abs() < 1e-9);
    }

    #[test]
    fn test_priority_of_payments_changes_pik_sweep_order() {
        let period = PeriodId::quarter(2025, 1);
        let context = build_context(period, &[("ebitda", 2_100.0), ("liquidity", 50.0)]);

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        let mut breakdown = CashflowBreakdown::with_currency(Currency::USD);
        breakdown.interest_expense_cash = Money::new(100.0, Currency::USD);
        contractual_flows.insert("TL-1".to_string(), breakdown);

        let mut sweep_first_state = CapitalStructureState::new();
        sweep_first_state
            .opening_balances
            .insert("TL-1".to_string(), Money::new(1_000.0, Currency::USD));
        let mut interest_first_state = sweep_first_state.clone();

        let sweep_first = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Sweep,
                PaymentPriority::Interest,
                PaymentPriority::Equity,
            ],
            available_cash_node: None,
            ecf_sweep: Some(EcfSweepSpec {
                ebitda_node: "ebitda".into(),
                taxes_node: None,
                capex_node: None,
                working_capital_node: None,
                cash_interest_node: None,
                sweep_percentage: 0.5,
                target_instrument_id: Some("TL-1".into()),
            }),
            pik_toggle: Some(PikToggleSpec {
                liquidity_metric: "liquidity".into(),
                threshold: 100.0,
                target_instrument_ids: Some(vec!["TL-1".into()]),
                min_periods_in_pik: 0,
            }),
        };
        let interest_first = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Interest,
                PaymentPriority::Sweep,
                PaymentPriority::Equity,
            ],
            available_cash_node: None,
            ecf_sweep: sweep_first.ecf_sweep.clone(),
            pik_toggle: sweep_first.pik_toggle.clone(),
        };

        let sweep_first_result = execute_waterfall(
            &period,
            &context,
            &sweep_first,
            &mut sweep_first_state,
            &contractual_flows,
        )
        .expect("waterfall should execute");
        let interest_first_result = execute_waterfall(
            &period,
            &context,
            &interest_first,
            &mut interest_first_state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        let sweep_first_balance = sweep_first_result["TL-1"].debt_balance.amount();
        let interest_first_balance = interest_first_result["TL-1"].debt_balance.amount();
        assert_eq!(sweep_first_balance, 100.0);
        assert_eq!(interest_first_balance, 100.0);
    }

    #[test]
    fn test_available_cash_caps_scheduled_payments_by_priority() {
        let period = PeriodId::quarter(2025, 1);
        let context = build_context(period, &[("cash_available", 150.0)]);

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        let mut breakdown = CashflowBreakdown::with_currency(Currency::USD);
        breakdown.fees = Money::new(20.0, Currency::USD);
        breakdown.interest_expense_cash = Money::new(100.0, Currency::USD);
        breakdown.principal_payment = Money::new(200.0, Currency::USD);
        contractual_flows.insert("TL-1".to_string(), breakdown);

        let mut state = CapitalStructureState::new();
        state
            .opening_balances
            .insert("TL-1".to_string(), Money::new(1_000.0, Currency::USD));

        let waterfall = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Fees,
                PaymentPriority::Interest,
                PaymentPriority::Amortization,
                PaymentPriority::Sweep,
                PaymentPriority::Equity,
            ],
            available_cash_node: Some("cash_available".into()),
            ecf_sweep: None,
            pik_toggle: None,
        };

        let results = execute_waterfall(
            &period,
            &context,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        let tl = results.get("TL-1").expect("TL-1");
        assert_eq!(tl.fees.amount(), 20.0);
        assert_eq!(tl.interest_expense_cash.amount(), 100.0);
        assert_eq!(tl.principal_payment.amount(), 30.0);
    }

    #[test]
    fn test_sweep_before_amortization_does_not_produce_negative_balance() {
        let period = PeriodId::quarter(2025, 1);
        let context = build_context(period, &[("ebitda", 5_000.0)]);

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        let mut breakdown = CashflowBreakdown::with_currency(Currency::USD);
        breakdown.principal_payment = Money::new(300.0, Currency::USD);
        contractual_flows.insert("TL-1".to_string(), breakdown);

        let mut state = CapitalStructureState::new();
        state
            .opening_balances
            .insert("TL-1".to_string(), Money::new(500.0, Currency::USD));

        let waterfall = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Sweep,
                PaymentPriority::Amortization,
                PaymentPriority::Equity,
            ],
            available_cash_node: None,
            ecf_sweep: Some(EcfSweepSpec {
                ebitda_node: "ebitda".into(),
                taxes_node: None,
                capex_node: None,
                working_capital_node: None,
                cash_interest_node: None,
                sweep_percentage: 1.0,
                target_instrument_id: Some("TL-1".into()),
            }),
            pik_toggle: None,
        };

        let results = execute_waterfall(
            &period,
            &context,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        let tl = results.get("TL-1").expect("TL-1");
        assert!(
            tl.debt_balance.amount() >= 0.0,
            "debt balance must never go negative, got {}",
            tl.debt_balance.amount()
        );
    }

    #[test]
    fn test_ecf_defaults_cash_interest_from_contractual_flows() {
        let period = PeriodId::quarter(2025, 1);
        let context = build_context(
            period,
            &[("ebitda", 1_000.0), ("taxes", 100.0), ("capex", 50.0)],
        );

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        let mut breakdown = CashflowBreakdown::with_currency(Currency::USD);
        breakdown.interest_expense_cash = Money::new(200.0, Currency::USD);
        contractual_flows.insert("TL-1".to_string(), breakdown);

        let mut state = CapitalStructureState::new();
        state
            .opening_balances
            .insert("TL-1".to_string(), Money::new(10_000.0, Currency::USD));

        let waterfall = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Fees,
                PaymentPriority::Interest,
                PaymentPriority::Amortization,
                PaymentPriority::Sweep,
                PaymentPriority::Equity,
            ],
            available_cash_node: None,
            ecf_sweep: Some(EcfSweepSpec {
                ebitda_node: "ebitda".into(),
                taxes_node: Some("taxes".into()),
                capex_node: Some("capex".into()),
                working_capital_node: None,
                cash_interest_node: None,
                sweep_percentage: 0.5,
                target_instrument_id: Some("TL-1".into()),
            }),
            pik_toggle: None,
        };

        let results = execute_waterfall(
            &period,
            &context,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        let tl = results.get("TL-1").expect("TL-1");
        assert_eq!(tl.principal_payment.amount(), 325.0);
    }

    #[test]
    fn test_ecf_negative_cash_interest_does_not_reduce_ecf() {
        let period = PeriodId::quarter(2025, 1);
        let context = build_context(
            period,
            &[("ebitda", 1_000.0), ("taxes", 100.0), ("capex", 50.0)],
        );

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        let mut breakdown = CashflowBreakdown::with_currency(Currency::USD);
        breakdown.interest_expense_cash = Money::new(-200.0, Currency::USD);
        contractual_flows.insert("TL-1".to_string(), breakdown);

        let mut state = CapitalStructureState::new();
        state
            .opening_balances
            .insert("TL-1".to_string(), Money::new(10_000.0, Currency::USD));

        let waterfall = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Fees,
                PaymentPriority::Interest,
                PaymentPriority::Amortization,
                PaymentPriority::Sweep,
                PaymentPriority::Equity,
            ],
            available_cash_node: None,
            ecf_sweep: Some(EcfSweepSpec {
                ebitda_node: "ebitda".into(),
                taxes_node: Some("taxes".into()),
                capex_node: Some("capex".into()),
                working_capital_node: None,
                cash_interest_node: None,
                sweep_percentage: 0.5,
                target_instrument_id: Some("TL-1".into()),
            }),
            pik_toggle: None,
        };

        let results = execute_waterfall(
            &period,
            &context,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        // ECF = 1000 - 100 - 50 - max(0, -200) = 1000 - 100 - 50 - 0 = 850
        // Sweep = 850 * 0.5 = 425
        let tl = results.get("TL-1").expect("TL-1");
        assert_eq!(tl.principal_payment.amount(), 425.0);
    }

    #[test]
    fn test_scheduled_amortization_exceeding_balance_is_clamped() {
        let period = PeriodId::quarter(2025, 1);
        let context = build_context(period, &[]);

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        let mut breakdown = CashflowBreakdown::with_currency(Currency::USD);
        breakdown.principal_payment = Money::new(300.0, Currency::USD);
        contractual_flows.insert("TL-1".to_string(), breakdown);

        let mut state = CapitalStructureState::new();
        state
            .opening_balances
            .insert("TL-1".to_string(), Money::new(200.0, Currency::USD));

        let waterfall = WaterfallSpec {
            priority_of_payments: vec![PaymentPriority::Amortization, PaymentPriority::Equity],
            available_cash_node: None,
            ecf_sweep: None,
            pik_toggle: None,
        };

        let results = execute_waterfall(
            &period,
            &context,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        let tl = results.get("TL-1").expect("TL-1");
        assert_eq!(
            tl.principal_payment.amount(),
            200.0,
            "principal should be clamped to opening balance"
        );
        assert_eq!(
            tl.debt_balance.amount(),
            0.0,
            "balance should be zero, not negative"
        );
    }

    #[test]
    fn test_sweep_plus_amortization_exceeding_balance_is_clamped() {
        let period = PeriodId::quarter(2025, 1);
        let context = build_context(period, &[("ebitda", 10_000.0)]);

        let mut contractual_flows: IndexMap<String, CashflowBreakdown> = IndexMap::new();
        let mut breakdown = CashflowBreakdown::with_currency(Currency::USD);
        breakdown.principal_payment = Money::new(500.0, Currency::USD);
        contractual_flows.insert("TL-1".to_string(), breakdown);

        let mut state = CapitalStructureState::new();
        state
            .opening_balances
            .insert("TL-1".to_string(), Money::new(400.0, Currency::USD));

        let waterfall = WaterfallSpec {
            priority_of_payments: vec![
                PaymentPriority::Amortization,
                PaymentPriority::Sweep,
                PaymentPriority::Equity,
            ],
            available_cash_node: None,
            ecf_sweep: Some(EcfSweepSpec {
                ebitda_node: "ebitda".into(),
                taxes_node: None,
                capex_node: None,
                working_capital_node: None,
                cash_interest_node: None,
                sweep_percentage: 1.0,
                target_instrument_id: Some("TL-1".into()),
            }),
            pik_toggle: None,
        };

        let results = execute_waterfall(
            &period,
            &context,
            &waterfall,
            &mut state,
            &contractual_flows,
        )
        .expect("waterfall should execute");

        let tl = results.get("TL-1").expect("TL-1");
        assert_eq!(
            tl.principal_payment.amount(),
            400.0,
            "principal should be clamped to opening balance"
        );
        assert_eq!(
            tl.debt_balance.amount(),
            0.0,
            "balance should be zero after full paydown"
        );
    }
}
