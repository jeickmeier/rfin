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
//! ECF = EBITDA - Taxes - CapEx - Working_Capital_Change
//! Sweep = max(0, ECF × sweep_percentage)
//! ```
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
    let mut result = IndexMap::new();

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
        Money::new(0.0, get_base_currency(contractual_flows))
    };

    // Step 3: Allocate cash according to priority stack
    for (instrument_id, mut breakdown) in contractual_flows.clone() {
        let currency = breakdown.interest_expense_cash.currency();

        // Get opening balance for this instrument
        let opening_balance = state.get_opening_balance(&instrument_id, currency);

        // Apply PIK mode update for this instrument (if configured)
        if let Some(enable_pik) = pik_enable {
            let should_apply = pik_targets
                .as_ref()
                .map(|set| set.contains(&instrument_id))
                .unwrap_or(true);
            if should_apply {
                state.pik_mode.insert(instrument_id.clone(), enable_pik);
            }
        }

        // If PIK mode is enabled, move cash interest into PIK bucket
        let is_pik_enabled = state.pik_mode.get(&instrument_id).copied().unwrap_or(false);
        if is_pik_enabled {
            breakdown.interest_expense_pik += breakdown.interest_expense_cash;
            breakdown.interest_expense_cash = Money::new(0.0, currency);
        }

        // Apply sweep if this is the target instrument
        let sweep_for_instrument = if let Some(ecf_spec) = &waterfall_spec.ecf_sweep {
            if ecf_spec
                .target_instrument_id
                .as_ref()
                .map(|id| id == &instrument_id)
                .unwrap_or(true)
            {
                // Limit sweep to outstanding balance
                let max_sweep = opening_balance;
                if sweep_amount.currency() == currency {
                    Money::new(sweep_amount.amount().min(max_sweep.amount()), currency)
                } else {
                    Money::new(0.0, currency)
                }
            } else {
                Money::new(0.0, currency)
            }
        } else {
            Money::new(0.0, currency)
        };

        // Add sweep to principal payment
        breakdown.principal_payment = breakdown
            .principal_payment
            .checked_add(sweep_for_instrument)?;

        // Update closing balance
        let closing_balance = opening_balance
            .checked_sub(breakdown.principal_payment)?
            .checked_add(breakdown.interest_expense_pik)?; // PIK increases balance
        state.set_closing_balance(instrument_id.clone(), closing_balance);
        breakdown.debt_balance = closing_balance;

        // Update cumulative metrics
        update_cumulative_metrics(state, &instrument_id, &breakdown, currency)?;

        result.insert(instrument_id, breakdown);
    }

    Ok(result)
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

    // Calculate ECF: EBITDA - Taxes - Capex - Working Capital Change
    let ecf = ebitda - taxes - capex - wc_change;

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
            node_to_column.insert((*name).to_string(), idx);
        }
        let mut ctx = EvaluationContext::new(period, node_to_column, IndexMap::new());
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
            ecf_sweep: Some(EcfSweepSpec {
                ebitda_node: "ebitda".into(),
                taxes_node: Some("taxes".into()),
                capex_node: Some("capex".into()),
                working_capital_node: None,
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
            ecf_sweep: None,
            pik_toggle: Some(PikToggleSpec {
                liquidity_metric: "liquidity".into(),
                threshold: 100.0,
                target_instrument_ids: Some(vec!["TL-PIK".into()]),
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
}
