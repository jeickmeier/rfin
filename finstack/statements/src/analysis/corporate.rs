//! Corporate valuation using DCF methodology.
//!
//! This module provides integration between financial statement models and
//! DCF (Discounted Cash Flow) valuation, allowing direct valuation of companies
//! from forecast models.

use crate::error::Result;
use crate::evaluator::{Evaluator, StatementResult};
use crate::types::FinancialModelSpec;
use finstack_core::currency::Currency;
use finstack_core::explain::{ExplanationTrace, TraceEntry};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::dcf_equity::{
    DiscountedCashFlow, EquityBridge, TerminalValueSpec, ValuationDiscounts,
};
use finstack_valuations::instruments::{Attributes, Instrument};
use serde_json::json;

/// Corporate valuation result containing DCF outputs.
#[derive(Debug, Clone)]
pub struct CorporateValuationResult {
    /// Equity value (EV - Net Debt, after discounts)
    pub equity_value: Money,
    /// Enterprise value (PV of all cash flows + terminal value)
    pub enterprise_value: Money,
    /// Net debt (or effective bridge amount) used in calculation
    pub net_debt: Money,
    /// Terminal value (present value)
    pub terminal_value_pv: Money,
    /// Equity value per diluted share (if shares_outstanding was provided)
    pub equity_value_per_share: Option<f64>,
    /// Diluted share count (if shares_outstanding was provided)
    pub diluted_shares: Option<f64>,
    /// The underlying DCF instrument (for further analysis)
    pub dcf_instrument: Option<DiscountedCashFlow>,
}

/// Optional configuration for DCF valuation beyond the core WACC/terminal parameters.
///
/// All fields default to `None`/`false`, preserving backward compatibility with
/// the original `evaluate_dcf()` signature.
#[derive(Debug, Clone, Default)]
pub struct DcfOptions {
    /// Enable mid-year discounting convention (default: false).
    pub mid_year_convention: bool,
    /// Structured equity bridge (replaces flat net_debt when `Some`).
    pub equity_bridge: Option<EquityBridge>,
    /// Basic shares outstanding for per-share value.
    pub shares_outstanding: Option<f64>,
    /// Private company valuation discounts (DLOM, DLOC).
    pub valuation_discounts: Option<ValuationDiscounts>,
}

#[derive(Clone, Copy)]
pub(crate) struct DcfEvalContext<'a> {
    pub(crate) net_debt_override: Option<f64>,
    pub(crate) options: &'a DcfOptions,
    pub(crate) market: Option<&'a MarketContext>,
}

/// Evaluate a financial model using DCF methodology.
///
/// # Arguments
///
/// * `model` - The financial statement model with forecast periods
/// * `wacc` - Weighted average cost of capital (decimal, e.g., 0.10 for 10%)
/// * `terminal_value` - Terminal value specification (Gordon Growth or Exit Multiple)
/// * `ufcf_node` - Node ID containing unlevered free cash flow values (default: "ufcf")
/// * `net_debt_override` - Optional fixed net debt value; if None, derived from model
///
/// # Returns
///
/// `CorporateValuationResult` containing equity value, enterprise value, and breakdown
///
/// # Example
///
/// ```rust,no_run
/// use finstack_statements::analysis::corporate::evaluate_dcf;
/// use finstack_statements::types::FinancialModelSpec;
/// use finstack_valuations::instruments::equity::dcf_equity::TerminalValueSpec;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let model: FinancialModelSpec = unimplemented!("build or load a FinancialModelSpec with model.meta[\"currency\"] set");
/// let result = evaluate_dcf(
///     &model,
///     0.10,  // 10% WACC
///     TerminalValueSpec::GordonGrowth { growth_rate: 0.02 },
///     "ufcf",
///     None,  // Calculate net debt from model
/// )?;
///
/// println!("Equity Value: {}", result.equity_value);
/// # Ok(())
/// # }
/// ```
pub fn evaluate_dcf(
    model: &FinancialModelSpec,
    wacc: f64,
    terminal_value: TerminalValueSpec,
    ufcf_node: &str,
    net_debt_override: Option<f64>,
) -> Result<CorporateValuationResult> {
    let (result, _trace) = evaluate_dcf_impl(
        model,
        wacc,
        terminal_value,
        ufcf_node,
        DcfEvalContext {
            net_debt_override,
            options: &DcfOptions::default(),
            market: None,
        },
    )?;
    Ok(result)
}

/// Evaluate a financial model using DCF methodology with additional options.
///
/// Extends [`evaluate_dcf`] with support for mid-year convention, equity bridge,
/// shares outstanding, and valuation discounts.
pub fn evaluate_dcf_with_options(
    model: &FinancialModelSpec,
    wacc: f64,
    terminal_value: TerminalValueSpec,
    ufcf_node: &str,
    net_debt_override: Option<f64>,
    options: &DcfOptions,
) -> Result<CorporateValuationResult> {
    let (result, _trace) = evaluate_dcf_impl(
        model,
        wacc,
        terminal_value,
        ufcf_node,
        DcfEvalContext {
            net_debt_override,
            options,
            market: None,
        },
    )?;
    Ok(result)
}

/// Evaluate a financial model using DCF methodology and return an explanation trace.
///
/// This function returns both the corporate valuation result and a structured
/// explanation trace capturing UFCF derivation and EV sensitivity to WACC /
/// terminal multiple.
pub fn evaluate_dcf_with_trace(
    model: &FinancialModelSpec,
    wacc: f64,
    terminal_value: TerminalValueSpec,
    ufcf_node: &str,
    net_debt_override: Option<f64>,
) -> Result<(CorporateValuationResult, ExplanationTrace)> {
    evaluate_dcf_impl(
        model,
        wacc,
        terminal_value,
        ufcf_node,
        DcfEvalContext {
            net_debt_override,
            options: &DcfOptions::default(),
            market: None,
        },
    )
}

/// Evaluate a financial model using DCF methodology with optional market context.
///
/// This extends [`evaluate_dcf_with_options`] by accepting a [`MarketContext`]
/// for curve-based discounting when instruments reference discount curves.
pub fn evaluate_dcf_with_market(
    model: &FinancialModelSpec,
    wacc: f64,
    terminal_value: TerminalValueSpec,
    ufcf_node: &str,
    net_debt_override: Option<f64>,
    options: &DcfOptions,
    market: Option<&MarketContext>,
) -> Result<CorporateValuationResult> {
    let (result, _trace) = evaluate_dcf_impl(
        model,
        wacc,
        terminal_value,
        ufcf_node,
        DcfEvalContext {
            net_debt_override,
            options,
            market,
        },
    )?;
    Ok(result)
}

/// Core implementation shared by all `evaluate_dcf*` entry points.
fn evaluate_dcf_impl(
    model: &FinancialModelSpec,
    wacc: f64,
    terminal_value: TerminalValueSpec,
    ufcf_node: &str,
    context: DcfEvalContext<'_>,
) -> Result<(CorporateValuationResult, ExplanationTrace)> {
    // Create evaluator and evaluate the model
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate_with_market_context(model, context.market, None)?;

    evaluate_dcf_from_results_impl(model, &results, wacc, terminal_value, ufcf_node, context)
}

pub(crate) fn evaluate_dcf_from_results_impl(
    model: &FinancialModelSpec,
    results: &StatementResult,
    wacc: f64,
    terminal_value: TerminalValueSpec,
    ufcf_node: &str,
    context: DcfEvalContext<'_>,
) -> Result<(CorporateValuationResult, ExplanationTrace)> {
    let first_forecast_period = model.periods.iter().find(|period| !period.is_actual);
    let last_actual_period = model
        .periods
        .iter()
        .filter(|period| period.is_actual)
        .next_back();

    // Initialize explanation trace
    let mut trace = ExplanationTrace::new("corporate_dcf");

    // Extract UFCF series from results
    let mut flows = Vec::new();
    let currency = extract_currency_from_model(model)?;

    for period in &model.periods {
        if period.is_actual {
            continue;
        }
        if let Some(ufcf_value) = results.get(ufcf_node, &period.id) {
            // Use period end date for the cashflow
            let date = period.end;
            flows.push((date, ufcf_value));

            // Record UFCF contribution in the explanation trace
            trace.push(
                TraceEntry::ComputationStep {
                    name: "ufcf_period".to_string(),
                    description: "Unlevered free cash flow by period".to_string(),
                    metadata: Some(json!({
                        "period_id": format!("{:?}", period.id),
                        "ufcf": ufcf_value,
                        "date": date.to_string(),
                    })),
                },
                None,
            );
        }
    }

    if flows.is_empty() {
        return Err(crate::error::Error::Eval(format!(
            "No UFCF values found for node '{}'",
            ufcf_node
        )));
    }

    // Validate terminal value constraints.
    match &terminal_value {
        TerminalValueSpec::GordonGrowth { growth_rate } if *growth_rate >= wacc => {
            return Err(crate::error::Error::Eval(format!(
                "Gordon Growth terminal value requires growth_rate ({:.4}) < WACC ({:.4}). \
                 A growth rate >= WACC produces an infinite terminal value.",
                growth_rate, wacc
            )));
        }
        TerminalValueSpec::HModel {
            high_growth_rate,
            stable_growth_rate,
            half_life_years,
        } => {
            if *stable_growth_rate >= wacc {
                return Err(crate::error::Error::Eval(format!(
                    "H-Model terminal value requires stable_growth_rate ({:.4}) < WACC ({:.4}).",
                    stable_growth_rate, wacc
                )));
            }
            if *high_growth_rate < *stable_growth_rate {
                return Err(crate::error::Error::Eval(format!(
                    "H-Model requires high_growth_rate ({:.4}) >= stable_growth_rate ({:.4}).",
                    high_growth_rate, stable_growth_rate
                )));
            }
            if *half_life_years <= 0.0 {
                return Err(crate::error::Error::Eval(format!(
                    "H-Model requires half_life_years > 0, got {:.4}.",
                    half_life_years
                )));
            }
        }
        _ => {}
    }

    // Determine net debt
    let net_debt = if let Some(override_val) = context.net_debt_override {
        override_val
    } else {
        calculate_net_debt_from_model(model, results, last_actual_period.map(|period| period.id))?
    };

    // Determine valuation date. When historical actuals exist, anchor the DCF to the
    // first forecast boundary so explicit cashflows and bridge values share the same cut.
    let valuation_date = if let Some(forecast_period) = first_forecast_period {
        forecast_period.start
    } else {
        model
            .periods
            .first()
            .ok_or_else(|| crate::error::Error::Eval("Model has no periods".into()))?
            .start
    };

    // Create DCF instrument
    // Use a default discount curve ID - DCF uses WACC internally, but still needs a curve ID
    let discount_curve_id = CurveId::new(format!("{}-DISCOUNT", currency));
    let mut builder = DiscountedCashFlow::builder()
        .id(InstrumentId::new(format!("{}-DCF", model.id)))
        .currency(currency)
        .flows(flows)
        .wacc(wacc)
        .terminal_value(terminal_value)
        .net_debt(net_debt)
        .valuation_date(valuation_date)
        .discount_curve_id(discount_curve_id)
        .mid_year_convention(context.options.mid_year_convention)
        .attributes(Attributes::new());

    if let Some(ref bridge) = context.options.equity_bridge {
        builder = builder.equity_bridge(bridge.clone());
    }
    if let Some(shares) = context.options.shares_outstanding {
        builder = builder.shares_outstanding(shares);
    }
    if let Some(ref discounts) = context.options.valuation_discounts {
        builder = builder.valuation_discounts(discounts.clone());
    }

    let dcf = builder
        .build()
        .map_err(|e| crate::error::Error::Eval(e.to_string()))?;

    // Calculate valuation
    let default_market = MarketContext::default();
    let market_ref = context.market.unwrap_or(&default_market);
    let equity_value = dcf
        .value(market_ref, valuation_date)
        .map_err(|e| crate::error::Error::Eval(e.to_string()))?;

    // Calculate components for result
    let pv_explicit = dcf.calculate_pv_explicit_flows();
    let tv = dcf
        .calculate_terminal_value()
        .map_err(|e| crate::error::Error::Eval(e.to_string()))?;
    let pv_terminal = dcf
        .discount_terminal_value(tv)
        .map_err(|e| crate::error::Error::Eval(e.to_string()))?;
    let enterprise_value = pv_explicit + pv_terminal;

    // Record base valuation in the explanation trace
    trace.push(
        TraceEntry::ComputationStep {
            name: "dcf_base_valuation".to_string(),
            description: "Base DCF valuation (enterprise and equity value)".to_string(),
            metadata: Some(json!({
                "wacc": wacc,
                "pv_explicit_flows": pv_explicit,
                "terminal_value": tv,
                "pv_terminal_value": pv_terminal,
                "enterprise_value": enterprise_value,
                "net_debt": net_debt,
                "equity_value": equity_value.amount(),
            })),
        },
        None,
    );

    // Sensitivity of EV to WACC (+/- 100 bps).
    // Compute EV directly from PV components (not from equity + bridge) so that
    // the result is independent of valuation discounts (DLOM/DLOC).
    let ev_wacc_up = {
        let mut dcf_up = dcf.clone();
        dcf_up.wacc = wacc + 0.01;
        let pv_exp = dcf_up.calculate_pv_explicit_flows();
        let tv_up = dcf_up
            .calculate_terminal_value()
            .map_err(|e| crate::error::Error::Eval(e.to_string()))?;
        let pv_tv = dcf_up
            .discount_terminal_value(tv_up)
            .map_err(|e| crate::error::Error::Eval(e.to_string()))?;
        pv_exp + pv_tv
    };

    let ev_wacc_down = {
        let mut dcf_down = dcf.clone();
        dcf_down.wacc = (wacc - 0.01).max(0.0);
        let pv_exp = dcf_down.calculate_pv_explicit_flows();
        let tv_down = dcf_down
            .calculate_terminal_value()
            .map_err(|e| crate::error::Error::Eval(e.to_string()))?;
        let pv_tv = dcf_down
            .discount_terminal_value(tv_down)
            .map_err(|e| crate::error::Error::Eval(e.to_string()))?;
        pv_exp + pv_tv
    };

    trace.push(
        TraceEntry::ComputationStep {
            name: "wacc_sensitivity".to_string(),
            description: "Sensitivity of enterprise value to WACC (+/- 100 bps)".to_string(),
            metadata: Some(json!({
                "wacc": wacc,
                "ev_base": enterprise_value,
                "wacc_up_bp": 100.0,
                "ev_wacc_up": ev_wacc_up,
                "wacc_down_bp": 100.0,
                "ev_wacc_down": ev_wacc_down,
            })),
        },
        None,
    );

    // Sensitivity of EV to Exit Multiple (if applicable)
    if let TerminalValueSpec::ExitMultiple {
        terminal_metric,
        multiple,
    } = dcf.terminal_value
    {
        let mut dcf_up = dcf.clone();
        dcf_up.terminal_value = TerminalValueSpec::ExitMultiple {
            terminal_metric,
            multiple: multiple + 1.0,
        };
        let ev_up = {
            let pv_explicit_up = dcf_up.calculate_pv_explicit_flows();
            let tv_up = dcf_up
                .calculate_terminal_value()
                .map_err(|e| crate::error::Error::Eval(e.to_string()))?;
            let pv_tv_up = dcf_up
                .discount_terminal_value(tv_up)
                .map_err(|e| crate::error::Error::Eval(e.to_string()))?;
            pv_explicit_up + pv_tv_up
        };

        let mut dcf_down = dcf.clone();
        dcf_down.terminal_value = TerminalValueSpec::ExitMultiple {
            terminal_metric,
            multiple: (multiple - 1.0).max(0.0),
        };
        let ev_down = {
            let pv_explicit_down = dcf_down.calculate_pv_explicit_flows();
            let tv_down = dcf_down
                .calculate_terminal_value()
                .map_err(|e| crate::error::Error::Eval(e.to_string()))?;
            let pv_tv_down = dcf_down
                .discount_terminal_value(tv_down)
                .map_err(|e| crate::error::Error::Eval(e.to_string()))?;
            pv_explicit_down + pv_tv_down
        };

        trace.push(
            TraceEntry::ComputationStep {
                name: "exit_multiple_sensitivity".to_string(),
                description: "Sensitivity of enterprise value to terminal exit multiple (+/- 1.0x)"
                    .to_string(),
                metadata: Some(json!({
                    "terminal_metric": terminal_metric,
                    "multiple_base": multiple,
                    "ev_base": enterprise_value,
                    "multiple_up": multiple + 1.0,
                    "ev_multiple_up": ev_up,
                    "multiple_down": (multiple - 1.0).max(0.0),
                    "ev_multiple_down": ev_down,
                })),
            },
            None,
        );
    }

    // Compute per-share metrics if shares outstanding is set
    let equity_val = equity_value.amount();
    let equity_value_per_share = dcf.equity_value_per_share(equity_val);
    let diluted_shares = dcf.diluted_shares(equity_val);

    Ok((
        CorporateValuationResult {
            equity_value,
            enterprise_value: Money::new(enterprise_value, currency),
            net_debt: Money::new(dcf.effective_net_debt(), currency),
            terminal_value_pv: Money::new(pv_terminal, currency),
            equity_value_per_share,
            diluted_shares,
            dcf_instrument: Some(dcf),
        },
        trace,
    ))
}

/// Extract currency from the model (assumes uniform currency).
///
/// Checks model metadata for a `"currency"` key. Falls back to USD with a
/// warning log when no currency is specified, since many models are USD-based.
fn extract_currency_from_model(model: &FinancialModelSpec) -> Result<Currency> {
    if let Some(currency_meta) = model.meta.get("currency") {
        if let Some(currency_str) = currency_meta.as_str() {
            return currency_str.parse::<Currency>().map_err(|_| {
                crate::error::Error::Eval(format!("Invalid currency: {}", currency_str))
            });
        }
        return Err(crate::error::Error::Eval(
            "Model metadata key 'currency' must be a string ISO currency code".into(),
        ));
    }

    Err(crate::error::Error::Eval(format!(
        "Model '{}' is missing required metadata key 'currency'. \
         Set model.meta[\"currency\"] to an ISO currency code such as 'USD'.",
        model.id
    )))
}

/// Calculate net debt from the model.
///
/// Net Debt = Total Debt - Cash
///
/// This function attempts to find debt and cash nodes in the model results.
fn calculate_net_debt_from_model(
    model: &FinancialModelSpec,
    results: &crate::evaluator::StatementResult,
    balance_sheet_period: Option<finstack_core::dates::PeriodId>,
) -> Result<f64> {
    // Use the valuation boundary balance sheet when available; otherwise fall back
    // to the latest model period for fully forecast-only models.
    let selected_period_id = if let Some(period_id) = balance_sheet_period {
        period_id
    } else {
        model
            .periods
            .last()
            .ok_or_else(|| crate::error::Error::Eval("Model has no periods".into()))?
            .id
    };

    // Try to find total debt — warn if not found so users know the value is assumed
    let total_debt = results
        .get("total_debt", &selected_period_id)
        .or_else(|| results.get("debt", &selected_period_id));

    let cash = results
        .get("cash", &selected_period_id)
        .or_else(|| results.get("cash_and_equivalents", &selected_period_id));

    let total_debt = total_debt.ok_or_else(|| {
        crate::error::Error::Eval(format!(
            "Net debt calculation requires a 'total_debt' or 'debt' node at period {}. \
             Provide the balance-sheet node or use net_debt_override.",
            selected_period_id
        ))
    })?;
    let cash = cash.ok_or_else(|| {
        crate::error::Error::Eval(format!(
            "Net debt calculation requires a 'cash' or 'cash_and_equivalents' node at period {}. \
             Provide the balance-sheet node or use net_debt_override.",
            selected_period_id
        ))
    })?;

    Ok(total_debt - cash)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::builder::ModelBuilder;
    use crate::types::AmountOrScalar;
    use finstack_core::dates::PeriodId;

    #[test]
    fn evaluate_dcf_requires_explicit_currency_metadata() {
        let model = ModelBuilder::new("dcf-missing-currency")
            .periods("2025Q1..Q2", None)
            .expect("valid periods")
            .value(
                "ufcf",
                &[
                    (
                        PeriodId::quarter(2025, 1),
                        AmountOrScalar::scalar(100_000.0),
                    ),
                    (
                        PeriodId::quarter(2025, 2),
                        AmountOrScalar::scalar(110_000.0),
                    ),
                ],
            )
            .value(
                "total_debt",
                &[(PeriodId::quarter(2025, 2), AmountOrScalar::scalar(50_000.0))],
            )
            .value(
                "cash",
                &[(PeriodId::quarter(2025, 2), AmountOrScalar::scalar(10_000.0))],
            )
            .build()
            .expect("valid model");

        let result = evaluate_dcf(
            &model,
            0.10,
            TerminalValueSpec::GordonGrowth { growth_rate: 0.02 },
            "ufcf",
            None,
        );
        assert!(result.is_err(), "currency metadata must be explicit");
    }

    #[test]
    fn evaluate_dcf_requires_balance_sheet_inputs_without_override() {
        let model = ModelBuilder::new("dcf-missing-balance-sheet")
            .periods("2025Q1..Q2", None)
            .expect("valid periods")
            .value(
                "ufcf",
                &[
                    (
                        PeriodId::quarter(2025, 1),
                        AmountOrScalar::scalar(100_000.0),
                    ),
                    (
                        PeriodId::quarter(2025, 2),
                        AmountOrScalar::scalar(110_000.0),
                    ),
                ],
            )
            .with_meta("currency", serde_json::json!("USD"))
            .build()
            .expect("valid model");

        let result = evaluate_dcf(
            &model,
            0.10,
            TerminalValueSpec::GordonGrowth { growth_rate: 0.02 },
            "ufcf",
            None,
        );
        assert!(
            result.is_err(),
            "missing debt and cash inputs must fail without an override"
        );
    }
}
