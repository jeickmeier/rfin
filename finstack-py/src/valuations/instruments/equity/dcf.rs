//! Rust source: `finstack/valuations/src/instruments/equity/dcf_equity/`
//! Abbreviated to `dcf` for Python ergonomics.

use crate::core::money::PyMoney;
use crate::statements::error::stmt_to_py;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::analysis::corporate::{evaluate_dcf_with_options, DcfOptions};
use finstack_valuations::instruments::equity::dcf_equity::{TerminalValueSpec, ValuationDiscounts};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::{Bound, PyResult};

/// Evaluate a corporate DCF using a statements FinancialModelSpec.
///
/// Parameters
/// ----------
/// model : finstack.statements.types.FinancialModelSpec
///     Financial statement model specification with projected UFCF node.
/// wacc : float, optional
///     Weighted average cost of capital (decimal, e.g., 0.10 for 10%).
/// terminal_growth : float, optional
///     Perpetual growth rate for Gordon Growth terminal value (decimal).
///     Ignored when ``terminal_type`` is ``"exit_multiple"`` or ``"h_model"``.
/// ufcf_node : str, optional
///     Node ID containing unlevered free cash flow values (default: "ufcf").
/// net_debt_override : float, optional
///     Optional net debt override. If not provided, derived from the model.
/// mid_year_convention : bool, optional
///     Enable mid-year discounting (default: False).
/// terminal_type : str, optional
///     Terminal value method: "gordon_growth" (default), "exit_multiple", or "h_model".
/// terminal_metric : float, optional
///     Terminal metric value for exit multiple (e.g., EBITDA). Required when
///     ``terminal_type="exit_multiple"``.
/// terminal_multiple : float, optional
///     Exit multiple (e.g., 10.0 for 10x). Required when ``terminal_type="exit_multiple"``.
/// high_growth_rate : float, optional
///     H-model initial high growth rate. Required when ``terminal_type="h_model"``.
/// stable_growth_rate : float, optional
///     H-model stable growth rate. Required when ``terminal_type="h_model"``.
/// half_life_years : float, optional
///     H-model half-life of growth fade. Required when ``terminal_type="h_model"``.
/// shares_outstanding : float, optional
///     Basic shares outstanding for per-share value calculation.
/// dlom : float, optional
///     Discount for Lack of Marketability (0.0-1.0).
/// dloc : float, optional
///     Discount for Lack of Control (0.0-1.0).
///
/// Returns
/// -------
/// dict
///     Dictionary with ``equity_value``, ``enterprise_value``, ``net_debt``,
///     ``terminal_value_pv`` as Money objects, and optionally
///     ``equity_value_per_share`` and ``diluted_shares`` as floats.
#[pyfunction]
#[pyo3(
    name = "evaluate_dcf",
    text_signature = "(model, wacc=0.10, terminal_growth=0.02, ufcf_node='ufcf', net_debt_override=None, *, mid_year_convention=False, terminal_type='gordon_growth', terminal_metric=None, terminal_multiple=None, high_growth_rate=None, stable_growth_rate=None, half_life_years=None, shares_outstanding=None, dlom=None, dloc=None)"
)]
#[allow(clippy::too_many_arguments)]
fn evaluate_dcf_py(
    _py: Python<'_>,
    model: &PyFinancialModelSpec,
    wacc: f64,
    terminal_growth: f64,
    ufcf_node: Option<String>,
    net_debt_override: Option<f64>,
    mid_year_convention: Option<bool>,
    terminal_type: Option<String>,
    terminal_metric: Option<f64>,
    terminal_multiple: Option<f64>,
    high_growth_rate: Option<f64>,
    stable_growth_rate: Option<f64>,
    half_life_years: Option<f64>,
    shares_outstanding: Option<f64>,
    dlom: Option<f64>,
    dloc: Option<f64>,
) -> PyResult<Py<PyAny>> {
    let ufcf_node = ufcf_node.unwrap_or_else(|| "ufcf".to_string());

    // Build terminal value spec from parameters
    let terminal = match terminal_type.as_deref().unwrap_or("gordon_growth") {
        "gordon_growth" => TerminalValueSpec::GordonGrowth {
            growth_rate: terminal_growth,
        },
        "exit_multiple" => {
            let metric = terminal_metric.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(
                    "terminal_metric is required when terminal_type='exit_multiple'",
                )
            })?;
            let multiple = terminal_multiple.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(
                    "terminal_multiple is required when terminal_type='exit_multiple'",
                )
            })?;
            TerminalValueSpec::ExitMultiple {
                terminal_metric: metric,
                multiple,
            }
        }
        "h_model" => {
            let hgr = high_growth_rate.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(
                    "high_growth_rate is required when terminal_type='h_model'",
                )
            })?;
            let sgr = stable_growth_rate.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(
                    "stable_growth_rate is required when terminal_type='h_model'",
                )
            })?;
            let hl = half_life_years.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(
                    "half_life_years is required when terminal_type='h_model'",
                )
            })?;
            TerminalValueSpec::HModel {
                high_growth_rate: hgr,
                stable_growth_rate: sgr,
                half_life_years: hl,
            }
        }
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown terminal_type '{}'. Expected 'gordon_growth', 'exit_multiple', or 'h_model'.",
                other
            )));
        }
    };

    // Build options
    let valuation_discounts = if dlom.is_some() || dloc.is_some() {
        Some(ValuationDiscounts {
            dlom,
            dloc,
            other_discount: None,
        })
    } else {
        None
    };

    let options = DcfOptions {
        mid_year_convention: mid_year_convention.unwrap_or(false),
        equity_bridge: None, // Not exposed via simple kwargs; use JSON model for complex bridges
        shares_outstanding,
        valuation_discounts,
    };

    let result = evaluate_dcf_with_options(
        &model.inner,
        wacc,
        terminal,
        &ufcf_node,
        net_debt_override,
        &options,
    )
    .map_err(stmt_to_py)?;

    let dict = PyDict::new(_py);
    dict.set_item("equity_value", PyMoney::new(result.equity_value))?;
    dict.set_item("enterprise_value", PyMoney::new(result.enterprise_value))?;
    dict.set_item("net_debt", PyMoney::new(result.net_debt))?;
    dict.set_item("terminal_value_pv", PyMoney::new(result.terminal_value_pv))?;

    if let Some(eps) = result.equity_value_per_share {
        dict.set_item("equity_value_per_share", eps)?;
    }
    if let Some(ds) = result.diluted_shares {
        dict.set_item("diluted_shares", ds)?;
    }

    Ok(dict.into())
}

/// Register the DCF valuation helpers under `finstack.valuations.instruments`.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(evaluate_dcf_py, module)?)?;
    module.setattr(
        "__doc__",
        "Corporate DCF valuation helpers built on top of statements and valuations.",
    )?;
    Ok(vec!["evaluate_dcf"])
}
