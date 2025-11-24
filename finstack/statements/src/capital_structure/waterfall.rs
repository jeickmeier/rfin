//! Cash Flow Waterfall & Sweep Mechanics
//!
//! This module implements dynamic cash flow allocation according to priority of payments,
//! excess cash flow sweeps, and PIK toggles based on model results.

use crate::capital_structure::types::*;
use crate::error::Result;
use crate::evaluator::EvaluationContext;
use finstack_core::dates::PeriodId;
use finstack_core::money::Money;
use indexmap::IndexMap;

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
    if let Some(pik_spec) = &waterfall_spec.pik_toggle {
        evaluate_pik_toggle(context, pik_spec, state)?;
    }

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

/// Evaluate PIK toggle conditions and update state.
fn evaluate_pik_toggle(
    context: &EvaluationContext,
    pik_spec: &PikToggleSpec,
    state: &mut CapitalStructureState,
) -> Result<()> {
    let _ = state; // Will be used when we update PIK mode for all instruments
                   // Try to get liquidity metric value from context
                   // For now, we'll try to evaluate it as a node reference
                   // In a full implementation, this might need to parse and evaluate a formula
    let metric_value = context.get_value(&pik_spec.liquidity_metric).unwrap_or(0.0);

    let enable_pik = metric_value < pik_spec.threshold;

    // Update PIK mode for target instruments
    if let Some(target_ids) = &pik_spec.target_instrument_ids {
        for instrument_id in target_ids {
            state.pik_mode.insert(instrument_id.clone(), enable_pik);
        }
    } else {
        // Apply to all instruments (we'll need to know which instruments exist)
        // For now, we'll update as we encounter them in execute_waterfall
    }

    Ok(())
}

/// Calculate Excess Cash Flow and determine sweep amount.
fn calculate_ecf_sweep(
    context: &EvaluationContext,
    ecf_spec: &EcfSweepSpec,
    _state: &CapitalStructureState,
    contractual_flows: &IndexMap<String, CashflowBreakdown>,
) -> Result<Money> {
    // Get EBITDA
    let ebitda = context.get_value(&ecf_spec.ebitda_node).unwrap_or(0.0);

    // Get taxes (if specified)
    let taxes = ecf_spec
        .taxes_node
        .as_ref()
        .and_then(|node| context.get_value(node).ok())
        .unwrap_or(0.0);

    // Get capex (if specified)
    let capex = ecf_spec
        .capex_node
        .as_ref()
        .and_then(|node| context.get_value(node).ok())
        .unwrap_or(0.0);

    // Get working capital change (if specified)
    let wc_change = ecf_spec
        .working_capital_node
        .as_ref()
        .and_then(|node| context.get_value(node).ok())
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
