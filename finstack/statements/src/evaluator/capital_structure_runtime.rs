//! Capital-structure-specific evaluator runtime helpers.

use super::{EvaluationContext, Evaluator};
use crate::error::Result;
use crate::evaluator::{DependencyGraph, EvalWarning};
use crate::types::FinancialModelSpec;
use finstack_core::dates::{Date, Period, PeriodId};
use finstack_core::money::Money;
use indexmap::IndexMap;
use std::collections::HashSet;
use std::sync::Arc;

type Instruments =
    IndexMap<String, Arc<dyn finstack_valuations::cashflow::CashflowProvider + Send + Sync>>;

impl Evaluator {
    /// Build instruments from model specifications.
    pub(crate) fn build_instruments(
        &self,
        model: &FinancialModelSpec,
    ) -> Result<Option<Instruments>> {
        use crate::capital_structure::integration;
        use crate::types::DebtInstrumentSpec;
        use finstack_valuations::cashflow::CashflowProvider;

        let cs_spec = match &model.capital_structure {
            Some(cs) => cs,
            None => return Ok(None),
        };

        let mut instruments: IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>> =
            IndexMap::new();

        for debt_spec in &cs_spec.debt_instruments {
            let (id, instrument) = match debt_spec {
                DebtInstrumentSpec::Bond { id, .. }
                | DebtInstrumentSpec::Swap { id, .. }
                | DebtInstrumentSpec::TermLoan { id, .. }
                | DebtInstrumentSpec::Generic { id, .. } => {
                    let instrument = integration::build_any_instrument_from_spec(debt_spec)?;
                    (id.clone(), instrument)
                }
            };
            instruments.insert(id, instrument);
        }

        Ok(Some(instruments))
    }

    /// Evaluate a period with dynamic capital structure support.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn evaluate_period_dynamic(
        &mut self,
        model: &FinancialModelSpec,
        period: &Period,
        is_actual: bool,
        eval_order: &[String],
        node_to_column: &IndexMap<String, usize>,
        historical: &IndexMap<PeriodId, IndexMap<String, f64>>,
        market_ctx: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
        instruments: &Instruments,
        cs_state: &mut crate::capital_structure::CapitalStructureState,
        cs_affected_nodes: &HashSet<String>,
    ) -> Result<(
        IndexMap<String, f64>,
        Vec<EvalWarning>,
        crate::capital_structure::CapitalStructureCashflows,
    )> {
        let period_id = period.id;

        let contractual_flows =
            compute_contractual_flows(instruments, cs_state, period, market_ctx, as_of)?;

        let mut cs_cashflows = build_cs_cashflows_from_contractual(&contractual_flows, period_id);
        recompute_cs_totals(&mut cs_cashflows, period_id);

        let mut context =
            EvaluationContext::new(period_id, node_to_column.clone(), historical.clone());
        context.capital_structure_cashflows = Some(cs_cashflows.clone());

        self.evaluate_nodes_in_order(
            model,
            &period_id,
            is_actual,
            eval_order,
            &mut context,
            None,
            None,
        )?;

        if let Some(cs_spec) = &model.capital_structure {
            if let Some(waterfall_spec) = &cs_spec.waterfall {
                let updated_flows = crate::capital_structure::waterfall::execute_waterfall(
                    &period_id,
                    &context,
                    waterfall_spec,
                    cs_state,
                    &contractual_flows,
                )?;

                merge_updated_flows(&mut cs_cashflows, &updated_flows, period_id);
                recompute_cs_totals(&mut cs_cashflows, period_id);
                context.capital_structure_cashflows = Some(cs_cashflows);
            }
        }

        if context.capital_structure_cashflows.is_some() && !cs_affected_nodes.is_empty() {
            self.evaluate_nodes_in_order(
                model,
                &period_id,
                is_actual,
                eval_order,
                &mut context,
                None,
                Some(cs_affected_nodes),
            )?;
        }

        let period_cs_cashflows = context
            .capital_structure_cashflows
            .take()
            .unwrap_or_default();
        let (values, warnings) = context.into_results();
        Ok((values, warnings, period_cs_cashflows))
    }
}

pub(crate) fn dependent_closure(
    graph: &DependencyGraph,
    seeds: &HashSet<String>,
) -> HashSet<String> {
    let mut visited: HashSet<String> = seeds.iter().cloned().collect();
    let mut stack: Vec<String> = seeds.iter().cloned().collect();

    while let Some(node) = stack.pop() {
        if let Some(dependents) = graph.dependents.get(&node) {
            for dependent in dependents {
                if visited.insert(dependent.clone()) {
                    stack.push(dependent.clone());
                }
            }
        }
    }

    visited
}

pub(crate) fn resolve_opening_balance(
    instrument: &(dyn finstack_valuations::cashflow::CashflowProvider + Send + Sync),
    market_ctx: &finstack_core::market_data::context::MarketContext,
    as_of: Date,
    period_start: Date,
) -> Result<Money> {
    let schedule = instrument.build_full_schedule(market_ctx, as_of)?;
    let outstanding_path = schedule.outstanding_by_date()?;

    let abs_money = |m: &Money| -> Money {
        if m.amount() < 0.0 {
            Money::new(-m.amount(), m.currency())
        } else {
            *m
        }
    };

    if let Some((_, m)) = outstanding_path
        .iter()
        .filter(|(d, _)| *d <= period_start)
        .next_back()
    {
        return Ok(abs_money(m));
    }

    if let Some((_, m)) = outstanding_path.first() {
        return Ok(abs_money(m));
    }

    let currency = schedule
        .flows
        .first()
        .map(|cf| cf.amount.currency())
        .unwrap_or(finstack_core::currency::Currency::USD);
    Ok(Money::new(0.0, currency))
}

fn compute_contractual_flows(
    instruments: &Instruments,
    cs_state: &mut crate::capital_structure::CapitalStructureState,
    period: &Period,
    market_ctx: &finstack_core::market_data::context::MarketContext,
    as_of: Date,
) -> Result<IndexMap<String, crate::capital_structure::CashflowBreakdown>> {
    use crate::capital_structure::integration;

    let mut flows = IndexMap::new();
    for (instrument_id, instrument) in instruments {
        let opening_balance =
            if let Some(balance) = cs_state.opening_balances.get(instrument_id).copied() {
                balance
            } else {
                let schedule = instrument.build_full_schedule(market_ctx, as_of)?;
                Money::new(0.0, schedule.notional.initial.currency())
            };

        let (breakdown, closing_balance) = integration::calculate_period_flows(
            instrument.as_ref(),
            period,
            opening_balance,
            market_ctx,
            as_of,
        )?;

        flows.insert(instrument_id.to_string(), breakdown.clone());
        cs_state.set_closing_balance(instrument_id.to_string(), closing_balance);
    }
    Ok(flows)
}

fn build_cs_cashflows_from_contractual(
    contractual_flows: &IndexMap<String, crate::capital_structure::CashflowBreakdown>,
    period_id: PeriodId,
) -> crate::capital_structure::CapitalStructureCashflows {
    let mut cs = crate::capital_structure::CapitalStructureCashflows::new();
    for (inst_id, breakdown) in contractual_flows {
        let mut period_map = IndexMap::new();
        period_map.insert(period_id, breakdown.clone());
        cs.by_instrument.insert(inst_id.clone(), period_map);
    }
    cs
}

fn recompute_cs_totals(
    cashflows: &mut crate::capital_structure::CapitalStructureCashflows,
    period_id: PeriodId,
) {
    let mut total: Option<crate::capital_structure::CashflowBreakdown> = None;

    for breakdown in cashflows
        .by_instrument
        .values()
        .filter_map(|pm| pm.get(&period_id))
    {
        if let Some(t) = &mut total {
            t.interest_expense_cash += breakdown.interest_expense_cash;
            t.interest_expense_pik += breakdown.interest_expense_pik;
            t.principal_payment += breakdown.principal_payment;
            t.fees += breakdown.fees;
            t.debt_balance += breakdown.debt_balance;
            t.accrued_interest += breakdown.accrued_interest;
        } else {
            total = Some(breakdown.clone());
        }
    }

    if let Some(t) = total {
        cashflows.reporting_currency = Some(t.interest_expense_cash.currency());
        cashflows.totals.insert(period_id, t);
    }
}

fn merge_updated_flows(
    cs_cashflows: &mut crate::capital_structure::CapitalStructureCashflows,
    updated_flows: &IndexMap<String, crate::capital_structure::CashflowBreakdown>,
    period_id: PeriodId,
) {
    for (inst_id, breakdown) in updated_flows {
        cs_cashflows
            .by_instrument
            .entry(inst_id.clone())
            .or_default()
            .insert(period_id, breakdown.clone());
    }
}
