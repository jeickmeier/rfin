//! Rust source: `finstack/valuations/src/instruments/equity/dcf_equity/`
//! Abbreviated to `dcf` for Python ergonomics.

use crate::core::money::PyMoney;
use crate::statements::error::stmt_to_py;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::analysis::corporate::{evaluate_dcf_with_options, DcfOptions};
use finstack_valuations::instruments::equity::dcf_equity::{
    EquityBridge, TerminalValueSpec, ValuationDiscounts,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use pyo3::{Bound, PyRef, PyResult};

// ============================================================================
// TERMINAL VALUE SPEC
// ============================================================================

/// Terminal value specification for DCF analysis.
///
/// Use classmethods to construct:
///
///   * ``TerminalValueSpec.gordon_growth(growth_rate=0.02)``
///   * ``TerminalValueSpec.exit_multiple(terminal_metric=100.0, multiple=10.0)``
///   * ``TerminalValueSpec.h_model(high_growth_rate=0.15, stable_growth_rate=0.03, half_life_years=5.0)``
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TerminalValueSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTerminalValueSpec {
    pub(crate) inner: TerminalValueSpec,
}

#[pymethods]
impl PyTerminalValueSpec {
    /// Gordon Growth model terminal value.
    ///
    /// Args:
    ///     growth_rate: Perpetual growth rate (decimal, e.g. 0.02 for 2%).
    #[classmethod]
    #[pyo3(text_signature = "(cls, growth_rate)")]
    fn gordon_growth(_cls: &Bound<'_, PyType>, growth_rate: f64) -> Self {
        Self {
            inner: TerminalValueSpec::GordonGrowth { growth_rate },
        }
    }

    /// Exit Multiple terminal value.
    ///
    /// Args:
    ///     terminal_metric: Terminal metric value (e.g., EBITDA).
    ///     multiple: Exit multiple (e.g., 10.0 for 10x).
    #[classmethod]
    #[pyo3(text_signature = "(cls, terminal_metric, multiple)")]
    fn exit_multiple(_cls: &Bound<'_, PyType>, terminal_metric: f64, multiple: f64) -> Self {
        Self {
            inner: TerminalValueSpec::ExitMultiple {
                terminal_metric,
                multiple,
            },
        }
    }

    /// H-Model terminal value with growth fade.
    ///
    /// Args:
    ///     high_growth_rate: Initial high growth rate (decimal).
    ///     stable_growth_rate: Long-term stable growth rate (decimal).
    ///     half_life_years: Half-life of growth fade in years.
    #[classmethod]
    #[pyo3(text_signature = "(cls, high_growth_rate, stable_growth_rate, half_life_years)")]
    fn h_model(
        _cls: &Bound<'_, PyType>,
        high_growth_rate: f64,
        stable_growth_rate: f64,
        half_life_years: f64,
    ) -> Self {
        Self {
            inner: TerminalValueSpec::HModel {
                high_growth_rate,
                stable_growth_rate,
                half_life_years,
            },
        }
    }

    /// Growth rate (only for GordonGrowth, else None).
    #[getter]
    fn growth_rate(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::GordonGrowth { growth_rate } => Some(*growth_rate),
            _ => None,
        }
    }

    /// Terminal metric (only for ExitMultiple, else None).
    #[getter]
    fn terminal_metric(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::ExitMultiple {
                terminal_metric, ..
            } => Some(*terminal_metric),
            _ => None,
        }
    }

    /// Exit multiple (only for ExitMultiple, else None).
    #[getter]
    fn multiple(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::ExitMultiple { multiple, .. } => Some(*multiple),
            _ => None,
        }
    }

    /// High growth rate (only for HModel, else None).
    #[getter]
    fn high_growth_rate(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::HModel {
                high_growth_rate, ..
            } => Some(*high_growth_rate),
            _ => None,
        }
    }

    /// Stable growth rate (only for HModel, else None).
    #[getter]
    fn stable_growth_rate(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::HModel {
                stable_growth_rate, ..
            } => Some(*stable_growth_rate),
            _ => None,
        }
    }

    /// Half-life of growth fade (only for HModel, else None).
    #[getter]
    fn half_life_years(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::HModel {
                half_life_years, ..
            } => Some(*half_life_years),
            _ => None,
        }
    }

    /// Variant name.
    #[getter]
    fn name(&self) -> &'static str {
        match &self.inner {
            TerminalValueSpec::GordonGrowth { .. } => "gordon_growth",
            TerminalValueSpec::ExitMultiple { .. } => "exit_multiple",
            TerminalValueSpec::HModel { .. } => "h_model",
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            TerminalValueSpec::GordonGrowth { growth_rate } => {
                format!("TerminalValueSpec.gordon_growth(growth_rate={growth_rate})")
            }
            TerminalValueSpec::ExitMultiple {
                terminal_metric,
                multiple,
            } => {
                format!("TerminalValueSpec.exit_multiple(terminal_metric={terminal_metric}, multiple={multiple})")
            }
            TerminalValueSpec::HModel {
                high_growth_rate,
                stable_growth_rate,
                half_life_years,
            } => {
                format!("TerminalValueSpec.h_model(high_growth_rate={high_growth_rate}, stable_growth_rate={stable_growth_rate}, half_life_years={half_life_years})")
            }
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

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
/// total_debt : float, optional
///     Total interest-bearing debt for the equity bridge.
/// cash : float, optional
///     Cash and cash equivalents for the equity bridge.
/// preferred_equity : float, optional
///     Preferred stock at liquidation preference for the equity bridge.
/// minority_interest : float, optional
///     Non-controlling (minority) interests for the equity bridge.
/// non_operating_assets : float, optional
///     Non-operating assets (excess cash, investments, etc.) for the equity bridge.
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
    text_signature = "(model, wacc=0.10, terminal_growth=0.02, ufcf_node='ufcf', net_debt_override=None, *, mid_year_convention=False, terminal_type='gordon_growth', terminal_metric=None, terminal_multiple=None, high_growth_rate=None, stable_growth_rate=None, half_life_years=None, shares_outstanding=None, dlom=None, dloc=None, total_debt=None, cash=None, preferred_equity=None, minority_interest=None, non_operating_assets=None)"
)]
#[allow(clippy::too_many_arguments)]
fn evaluate_dcf_py<'py>(
    _py: Python<'py>,
    model: &PyFinancialModelSpec,
    wacc: f64,
    terminal_growth: f64,
    ufcf_node: Option<String>,
    net_debt_override: Option<f64>,
    mid_year_convention: Option<bool>,
    terminal_type: Option<Bound<'py, PyAny>>,
    terminal_metric: Option<f64>,
    terminal_multiple: Option<f64>,
    high_growth_rate: Option<f64>,
    stable_growth_rate: Option<f64>,
    half_life_years: Option<f64>,
    shares_outstanding: Option<f64>,
    dlom: Option<f64>,
    dloc: Option<f64>,
    total_debt: Option<f64>,
    cash: Option<f64>,
    preferred_equity: Option<f64>,
    minority_interest: Option<f64>,
    non_operating_assets: Option<f64>,
) -> PyResult<Py<PyAny>> {
    let ufcf_node = ufcf_node.unwrap_or_else(|| "ufcf".to_string());

    // Build terminal value spec from parameters.
    // Accepts either a TerminalValueSpec instance or a string label.
    let terminal = if let Some(ref tt) = terminal_type {
        if let Ok(spec) = tt.extract::<PyRef<'_, PyTerminalValueSpec>>() {
            spec.inner.clone()
        } else if let Ok(s) = tt.extract::<String>() {
            match s.as_str() {
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
            }
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "terminal_type must be a string or TerminalValueSpec",
            ));
        }
    } else {
        TerminalValueSpec::GordonGrowth {
            growth_rate: terminal_growth,
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

    let equity_bridge = if total_debt.is_some()
        || cash.is_some()
        || preferred_equity.is_some()
        || minority_interest.is_some()
        || non_operating_assets.is_some()
    {
        Some(EquityBridge {
            total_debt: total_debt.unwrap_or(0.0),
            cash: cash.unwrap_or(0.0),
            preferred_equity: preferred_equity.unwrap_or(0.0),
            minority_interest: minority_interest.unwrap_or(0.0),
            non_operating_assets: non_operating_assets.unwrap_or(0.0),
            other_adjustments: Vec::new(),
        })
    } else {
        None
    };

    let options = DcfOptions {
        mid_year_convention: mid_year_convention.unwrap_or(false),
        equity_bridge,
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
    module.add_class::<PyTerminalValueSpec>()?;
    module.add_function(wrap_pyfunction!(evaluate_dcf_py, module)?)?;
    module.setattr(
        "__doc__",
        "Corporate DCF valuation helpers built on top of statements and valuations.",
    )?;
    Ok(vec!["TerminalValueSpec", "evaluate_dcf"])
}
