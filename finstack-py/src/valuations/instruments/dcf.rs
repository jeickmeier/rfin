use crate::core::money::PyMoney;
use crate::statements::error::stmt_to_py;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::analysis::corporate::evaluate_dcf;
use finstack_valuations::instruments::dcf::TerminalValueSpec;
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
/// ufcf_node : str, optional
///     Node ID containing unlevered free cash flow values (default: "ufcf").
/// net_debt_override : float, optional
///     Optional net debt override. If not provided, derived from the model.
///
/// Returns
/// -------
/// dict
///     Dictionary with `equity_value`, `enterprise_value`, `net_debt`,
///     and `terminal_value_pv` as Money objects.
#[pyfunction]
#[pyo3(
    name = "evaluate_dcf",
    text_signature = "(model, wacc=0.10, terminal_growth=0.02, ufcf_node='ufcf', net_debt_override=None)"
)]
fn evaluate_dcf_py(
    _py: Python<'_>,
    model: &PyFinancialModelSpec,
    wacc: f64,
    terminal_growth: f64,
    ufcf_node: Option<String>,
    net_debt_override: Option<f64>,
) -> PyResult<PyObject> {
    let ufcf_node = ufcf_node.unwrap_or_else(|| "ufcf".to_string());
    let terminal = TerminalValueSpec::GordonGrowth {
        growth_rate: terminal_growth,
    };

    let result = evaluate_dcf(&model.inner, wacc, terminal, &ufcf_node, net_debt_override)
        .map_err(stmt_to_py)?;

    let dict = PyDict::new(_py);
    dict.set_item("equity_value", PyMoney::new(result.equity_value))?;
    dict.set_item("enterprise_value", PyMoney::new(result.enterprise_value))?;
    dict.set_item("net_debt", PyMoney::new(result.net_debt))?;
    dict.set_item("terminal_value_pv", PyMoney::new(result.terminal_value_pv))?;

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
