//! Rust source: `finstack/valuations/src/instruments/equity/dcf_equity/`
//! DCF terminal value types shared by the Rust DCF instrument API.

use finstack_valuations::instruments::equity::dcf_equity::TerminalValueSpec;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use pyo3::{Bound, PyResult};

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

/// Register DCF terminal value types under `finstack.valuations.instruments`.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyTerminalValueSpec>()?;
    module.setattr(
        "__doc__",
        "DCF terminal value types shared by the Rust-backed discounted cash flow instrument API.",
    )?;
    Ok(vec!["TerminalValueSpec"])
}
