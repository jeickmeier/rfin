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

use crate::capital_structure::types::*;
use crate::error::Result;
use crate::evaluator::EvaluationContext;
use finstack_core::dates::PeriodId;
use finstack_core::money::Money;
use indexmap::IndexMap;
use std::collections::HashSet;

/// Execute waterfall logic for a single period.
///
/// This function:
/// 1. Checks PIK toggle conditions and updates interest mode
/// 2. Calculates contractual flows (interest, amortization)
/// 3. Calculates ECF and applies sweep if configured
/// 4. Allocates available cash according to priority stack
///
/// # Arguments
/// * `period_id` - Current period being evaluated
/// * `context` - Evaluation context with model results
/// * `waterfall_spec` - Waterfall configuration
/// * `state` - Current capital structure state (opening balances, etc.)
/// * `contractual_flows` - Pre-calculated contractual flows by instrument
///
/// # Returns
/// Updated cashflow breakdown for the period and updated state
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
    let interest_priority = priority_index(
        &waterfall_spec.priority_of_payments,
        PaymentPriority::Interest,
    );
    let fees_priority = priority_index(&waterfall_spec.priority_of_payments, PaymentPriority::Fees);
    let _amortization_priority = priority_index(
        &waterfall_spec.priority_of_payments,
        PaymentPriority::Amortization,
    );
    let extra_principal_priority = extra_principal_priority(&waterfall_spec.priority_of_payments);
    let equity_priority = priority_index(
        &waterfall_spec.priority_of_payments,
        PaymentPriority::Equity,
    );

    // Step 1: Check PIK toggle conditions
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

    // Step 2: Calculate ECF and sweep amount
    let sweep_amount = if let Some(ecf_spec) = &waterfall_spec.ecf_sweep {
        calculate_ecf_sweep(context, ecf_spec, state, contractual_flows)?
    } else {
        Money::new(0.0, cash_currency)
    };
    let available_cash = if let Some(available_cash_node) = &waterfall_spec.available_cash_node {
        let cash = eval_value_or_formula(context, available_cash_node)?;
        Some(Money::new(cash.max(0.0), cash_currency))
    } else {
        None
    };

    // Step 3: Allocate cash according to priority stack
    //
    // Execution order per standard loan documentation:
    //   1. Determine sweep amount (already computed above)
    //   2. Apply sweep as additional principal prepayment
    //   3. Update balance after sweep + scheduled amortization
    //   4. Apply PIK mode — PIK accrues on post-sweep balance
    //
    // This ensures PIK interest accrues on the reduced (post-sweep) balance,
    // matching standard LPA (Loan and Purchase Agreement) conventions.
    let min_periods_in_pik = waterfall_spec
        .pik_toggle
        .as_ref()
        .map(|spec| spec.min_periods_in_pik)
        .unwrap_or(0);
    let mut staged: Vec<(String, CashflowBreakdown, Money, Money, Money)> =
        Vec::with_capacity(contractual_flows.len());
    for (instrument_id, breakdown) in contractual_flows {
        let currency = breakdown.interest_expense_cash.currency();
        let opening_balance = state.get_opening_balance(instrument_id, currency);

        if let Some(enable_pik) = pik_enable {
            let should_apply = pik_targets
                .as_ref()
                .map(|set| set.contains(instrument_id))
                .unwrap_or(true);
            if should_apply {
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

        let mut staged_breakdown = breakdown.clone();
        if is_pik_enabled_for_priority(state, instrument_id)
            && interest_priority < extra_principal_priority
            && extra_principal_priority != usize::MAX
        {
            staged_breakdown.interest_expense_pik += staged_breakdown.interest_expense_cash;
            staged_breakdown.interest_expense_cash = Money::new(0.0, currency);
        }
        staged.push((
            instrument_id.clone(),
            staged_breakdown,
            opening_balance,
            Money::new(0.0, currency),
            breakdown.principal_payment,
        ));
    }

    let mut remaining_sweep = if equity_priority < extra_principal_priority {
        Money::new(0.0, sweep_amount.currency())
    } else {
        sweep_amount
    };
    // When there is no ECF sweep, the sweep amount is zero and these deductions
    // are moot. When ECF is configured, cash interest is already deducted inside
    // calculate_ecf_sweep, so we must not deduct it again here.
    // Fees are never part of the ECF formula, so always deduct them if they
    // rank before extra-principal actions.
    if fees_priority < extra_principal_priority {
        let total_fees = staged
            .iter()
            .map(|(_, breakdown, _, _, _)| breakdown.fees.amount())
            .sum::<f64>();
        remaining_sweep = Money::new(
            (remaining_sweep.amount() - total_fees).max(0.0),
            remaining_sweep.currency(),
        );
    }
    if interest_priority < extra_principal_priority && waterfall_spec.ecf_sweep.is_none() {
        let total_cash_interest = staged
            .iter()
            .map(|(_, breakdown, _, _, _)| breakdown.interest_expense_cash.amount())
            .sum::<f64>();
        remaining_sweep = Money::new(
            (remaining_sweep.amount() - total_cash_interest).max(0.0),
            remaining_sweep.currency(),
        );
    }

    let target_instrument_id = waterfall_spec
        .ecf_sweep
        .as_ref()
        .and_then(|spec| spec.target_instrument_id.as_deref());
    let mut extra_capacity: IndexMap<String, f64> = IndexMap::new();
    let mut total_extra_capacity = 0.0;
    for (instrument_id, breakdown, opening_balance, _, scheduled_principal) in &staged {
        let eligible = if let Some(target_id) = target_instrument_id {
            target_id == instrument_id
        } else {
            true
        };
        if !eligible || extra_principal_priority == usize::MAX {
            extra_capacity.insert(instrument_id.clone(), 0.0);
            continue;
        }

        let mut capacity = (opening_balance.amount() - scheduled_principal.amount()).max(0.0);
        if is_pik_enabled_for_priority(state, instrument_id)
            && interest_priority < extra_principal_priority
        {
            capacity += breakdown.interest_expense_pik.amount();
        }

        total_extra_capacity += capacity;
        extra_capacity.insert(instrument_id.clone(), capacity);
    }

    let staged_len = staged.len();
    for (idx, entry) in staged.iter_mut().enumerate() {
        let instrument_id = &entry.0;
        let breakdown = &mut entry.1;
        let currency = breakdown.interest_expense_cash.currency();

        let sweep_for_instrument =
            if extra_principal_priority == usize::MAX || remaining_sweep.currency() != currency {
                Money::new(0.0, currency)
            } else if let Some(target_id) = target_instrument_id {
                if target_id == instrument_id {
                    let capacity = *extra_capacity.get(instrument_id.as_str()).unwrap_or(&0.0);
                    Money::new(remaining_sweep.amount().min(capacity), currency)
                } else {
                    Money::new(0.0, currency)
                }
            } else {
                let capacity = *extra_capacity.get(instrument_id.as_str()).unwrap_or(&0.0);
                if total_extra_capacity <= 0.0 || capacity <= 0.0 {
                    Money::new(0.0, currency)
                } else if idx + 1 == staged_len {
                    Money::new(remaining_sweep.amount().min(capacity), currency)
                } else {
                    let proportional = remaining_sweep.amount() * (capacity / total_extra_capacity);
                    Money::new(proportional.min(capacity), currency)
                }
            };

        entry.3 = sweep_for_instrument;
        remaining_sweep = remaining_sweep.checked_sub(sweep_for_instrument)?;

        breakdown.principal_payment = entry.4.checked_add(entry.3)?;
    }

    if let Some(mut remaining_cash) = available_cash {
        for priority in &waterfall_spec.priority_of_payments {
            match priority {
                PaymentPriority::Fees => {
                    apply_cash_cap_to_category(&mut staged, &mut remaining_cash, |entry| {
                        &mut entry.1.fees
                    });
                }
                PaymentPriority::Interest => {
                    apply_cash_cap_to_category(&mut staged, &mut remaining_cash, |entry| {
                        &mut entry.1.interest_expense_cash
                    });
                }
                PaymentPriority::Amortization => {
                    let planned: Vec<f64> = staged
                        .iter()
                        .map(|entry| entry.4.amount().max(0.0))
                        .collect();
                    let allocations = allocate_pro_rata(&planned, &mut remaining_cash);
                    for (entry, allocated) in staged.iter_mut().zip(allocations.into_iter()) {
                        entry.4 = Money::new(allocated, entry.4.currency());
                    }
                }
                PaymentPriority::MandatoryPrepayment
                | PaymentPriority::VoluntaryPrepayment
                | PaymentPriority::Sweep => {
                    let planned: Vec<f64> = staged
                        .iter()
                        .map(|entry| entry.3.amount().max(0.0))
                        .collect();
                    let allocations = allocate_pro_rata(&planned, &mut remaining_cash);
                    for (entry, allocated) in staged.iter_mut().zip(allocations.into_iter()) {
                        entry.3 = Money::new(allocated, entry.3.currency());
                    }
                }
                PaymentPriority::Equity => {}
            }
        }
    }

    for (instrument_id, mut breakdown, opening_balance, extra_principal, scheduled_principal) in
        staged
    {
        let currency = breakdown.interest_expense_cash.currency();
        breakdown.principal_payment = scheduled_principal.checked_add(extra_principal)?;
        let post_sweep_balance = opening_balance.checked_sub(breakdown.principal_payment)?;

        let is_pik_enabled = is_pik_enabled_for_priority(state, &instrument_id);
        if is_pik_enabled
            && !(interest_priority < extra_principal_priority
                && extra_principal_priority != usize::MAX)
        {
            breakdown.interest_expense_pik += breakdown.interest_expense_cash;
            breakdown.interest_expense_cash = Money::new(0.0, currency);
        }

        // Step 3d: Closing balance = post-sweep + PIK accrual
        let closing_balance = post_sweep_balance.checked_add(breakdown.interest_expense_pik)?;
        state.set_closing_balance(instrument_id.to_string(), closing_balance);
        breakdown.debt_balance = closing_balance;

        // Update cumulative metrics
        update_cumulative_metrics(state, &instrument_id, &breakdown, currency)?;

        result.insert(instrument_id.to_string(), breakdown);
    }

    Ok(result)
}

fn apply_cash_cap_to_category<F>(
    staged: &mut [(String, CashflowBreakdown, Money, Money, Money)],
    remaining_cash: &mut Money,
    mut field: F,
) where
    F: FnMut(&mut (String, CashflowBreakdown, Money, Money, Money)) -> &mut Money,
{
    let planned: Vec<f64> = staged
        .iter_mut()
        .map(|entry| field(entry).amount().max(0.0))
        .collect();
    let allocations = allocate_pro_rata(&planned, remaining_cash);
    for (entry, allocated) in staged.iter_mut().zip(allocations.into_iter()) {
        let currency = field(entry).currency();
        *field(entry) = Money::new(allocated, currency);
    }
}

fn allocate_pro_rata(planned: &[f64], remaining_cash: &mut Money) -> Vec<f64> {
    let total_planned: f64 = planned.iter().sum();
    if total_planned <= 0.0 || remaining_cash.amount() <= 0.0 {
        return vec![0.0; planned.len()];
    }
    if remaining_cash.amount() >= total_planned {
        *remaining_cash = Money::new(
            remaining_cash.amount() - total_planned,
            remaining_cash.currency(),
        );
        return planned.to_vec();
    }

    let cash_before = remaining_cash.amount();
    let mut allocations = Vec::with_capacity(planned.len());
    for (idx, planned_value) in planned.iter().enumerate() {
        if idx + 1 == planned.len() {
            let allocated_so_far: f64 = allocations.iter().sum();
            allocations.push(
                (cash_before - allocated_so_far)
                    .max(0.0)
                    .min(*planned_value),
            );
        } else {
            allocations.push((cash_before * (*planned_value / total_planned)).min(*planned_value));
        }
    }
    *remaining_cash = Money::new(0.0, remaining_cash.currency());
    allocations
}

fn priority_index(priorities: &[PaymentPriority], target: PaymentPriority) -> usize {
    priorities
        .iter()
        .position(|priority| *priority == target)
        .unwrap_or(usize::MAX)
}

fn waterfall_currency(
    flows: &IndexMap<String, CashflowBreakdown>,
) -> Result<finstack_core::currency::Currency> {
    let mut currencies = flows
        .values()
        .map(|cf| cf.interest_expense_cash.currency())
        .collect::<Vec<_>>();
    currencies.sort();
    currencies.dedup();
    match currencies.as_slice() {
        [currency] => Ok(*currency),
        [] => Ok(finstack_core::currency::Currency::USD),
        _ => Err(crate::error::Error::capital_structure(
            "Waterfall execution currently requires a single cash currency. \
             Use one currency per waterfall or add explicit FX allocation semantics.",
        )),
    }
}

fn extra_principal_priority(priorities: &[PaymentPriority]) -> usize {
    [
        PaymentPriority::MandatoryPrepayment,
        PaymentPriority::VoluntaryPrepayment,
        PaymentPriority::Sweep,
    ]
    .into_iter()
    .map(|priority| priority_index(priorities, priority))
    .min()
    .unwrap_or(usize::MAX)
}

fn is_pik_enabled_for_priority(state: &CapitalStructureState, instrument_id: &str) -> bool {
    state.pik_mode.get(instrument_id).copied().unwrap_or(false)
}

fn eval_value_or_formula(context: &EvaluationContext, expr: &str) -> Result<f64> {
    // Fast path: treat as a node reference.
    if let Ok(value) = context.get_value(expr) {
        return Ok(value);
    }

    // Otherwise, treat as a DSL expression and evaluate against the current context.
    let compiled = crate::dsl::parse_and_compile(expr)?;
    let mut scratch = context.clone();
    crate::evaluator::formula::evaluate_formula(&compiled, &mut scratch, None)
}

/// Evaluate PIK toggle conditions and return whether PIK should be enabled.
fn evaluate_pik_toggle(context: &EvaluationContext, pik_spec: &PikToggleSpec) -> Result<bool> {
    let metric_value = eval_value_or_formula(context, &pik_spec.liquidity_metric)?;

    let enable_pik = metric_value < pik_spec.threshold;
    Ok(enable_pik)
}

/// Calculate Excess Cash Flow and determine sweep amount.
fn calculate_ecf_sweep(
    context: &EvaluationContext,
    ecf_spec: &EcfSweepSpec,
    _state: &CapitalStructureState,
    contractual_flows: &IndexMap<String, CashflowBreakdown>,
) -> Result<Money> {
    // Get EBITDA
    let ebitda = eval_value_or_formula(context, &ecf_spec.ebitda_node)?;

    // Get taxes (if specified)
    let taxes = ecf_spec
        .taxes_node
        .as_ref()
        .map(|expr| eval_value_or_formula(context, expr))
        .transpose()?
        .unwrap_or(0.0);

    // Get capex (if specified)
    let capex = ecf_spec
        .capex_node
        .as_ref()
        .map(|expr| eval_value_or_formula(context, expr))
        .transpose()?
        .unwrap_or(0.0);

    // Get working capital change (if specified)
    let wc_change = ecf_spec
        .working_capital_node
        .as_ref()
        .map(|expr| eval_value_or_formula(context, expr))
        .transpose()?
        .unwrap_or(0.0);

    // Get cash interest paid. Per S&P LCD / standard LPA definitions, ECF should
    // deduct cash interest paid. When not explicitly provided, use the period's
    // contractual cash interest so ECF is not overstated.
    let cash_interest = if let Some(ref expr) = ecf_spec.cash_interest_node {
        eval_value_or_formula(context, expr)?
    } else {
        contractual_flows
            .values()
            .map(|cf| cf.interest_expense_cash.amount())
            .sum()
    };

    // Calculate ECF: EBITDA - Taxes - Capex - Working Capital Change - Cash Interest
    let ecf = ebitda - taxes - capex - wc_change - cash_interest;

    // Apply sweep percentage
    let sweep_amount = ecf * ecf_spec.sweep_percentage;

    // Get base currency from contractual flows
    let currency = get_base_currency(contractual_flows);

    Ok(Money::new(sweep_amount.max(0.0), currency))
}

/// Get base currency from contractual flows (assumes all same currency for now).
fn get_base_currency(
    flows: &IndexMap<String, CashflowBreakdown>,
) -> finstack_core::currency::Currency {
    flows
        .values()
        .next()
        .map(|cf| cf.interest_expense_cash.currency())
        .unwrap_or(finstack_core::currency::Currency::USD)
}

/// Update cumulative metrics in state.
fn update_cumulative_metrics(
    state: &mut CapitalStructureState,
    instrument_id: &str,
    breakdown: &CashflowBreakdown,
    currency: finstack_core::currency::Currency,
) -> Result<()> {
    // Update cumulative interest cash
    let current_cash = state
        .cumulative_interest_cash
        .get(instrument_id)
        .copied()
        .unwrap_or_else(|| Money::new(0.0, currency));
    state.cumulative_interest_cash.insert(
        instrument_id.to_string(),
        current_cash.checked_add(breakdown.interest_expense_cash)?,
    );

    // Update cumulative interest PIK
    let current_pik = state
        .cumulative_interest_pik
        .get(instrument_id)
        .copied()
        .unwrap_or_else(|| Money::new(0.0, currency));
    state.cumulative_interest_pik.insert(
        instrument_id.to_string(),
        current_pik.checked_add(breakdown.interest_expense_pik)?,
    );

    // Update cumulative principal
    let current_principal = state
        .cumulative_principal
        .get(instrument_id)
        .copied()
        .unwrap_or_else(|| Money::new(0.0, currency));
    state.cumulative_principal.insert(
        instrument_id.to_string(),
        current_principal.checked_add(breakdown.principal_payment)?,
    );

    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::capital_structure::types::{
        CapitalStructureState, CashflowBreakdown, EcfSweepSpec, PaymentPriority, WaterfallSpec,
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
                sweep_percentage: 0.5, // 50% of ECF
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
        // ECF = 1_000_000 - 200_000 - 50_000 = 750,000 => sweep 375,000
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

        // Update liquidity above threshold and ensure toggle switches off
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

        // ECF = 1_000_000 - 200_000 - 50_000 = 750,000 => sweep 375,000
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
        // With ECF configured, cash interest is already deducted in ECF, so the
        // waterfall does not subtract it again from sweep capacity. Both orderings
        // now produce the same balance when PIK is active and ECF handles interest.
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
        // ECF = 1000 - 100 - 50 - 200 (auto cash interest) = 650
        // Sweep = 650 * 0.5 = 325
        assert_eq!(tl.principal_payment.amount(), 325.0);
    }
}
