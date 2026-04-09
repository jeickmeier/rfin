//! Corporate analysis orchestrator bindings.
//!
//! Wraps [`CorporateAnalysisBuilder`], [`CorporateAnalysis`], and
//! [`CreditInstrumentAnalysis`] from `finstack_statements::analysis::orchestrator`
//! so the full corporate analysis pipeline is available from Python.

use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::statements::analysis::corporate::{PyCorporateValuationResult, PyDcfOptions};
use crate::statements::analysis::credit_context::PyCreditContextMetrics;
use crate::statements::error::stmt_to_py;
use crate::statements::evaluator::PyStatementResult;
use crate::statements::types::model::PyFinancialModelSpec;
use crate::valuations::instruments::equity::dcf::PyTerminalValueSpec;
use finstack_statements_analytics::analysis::orchestrator::{
    CorporateAnalysis, CorporateAnalysisBuilder, CreditInstrumentAnalysis,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// CreditInstrumentAnalysis
// ---------------------------------------------------------------------------

/// Credit analysis for a single instrument.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "CreditInstrumentAnalysis",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyCreditInstrumentAnalysis {
    inner: CreditInstrumentAnalysis,
}

#[pymethods]
impl PyCreditInstrumentAnalysis {
    /// Coverage and leverage metrics computed from the statement context.
    #[getter]
    fn coverage(&self) -> PyCreditContextMetrics {
        PyCreditContextMetrics::new(self.inner.coverage.clone())
    }

    fn __repr__(&self) -> String {
        format!(
            "CreditInstrumentAnalysis(dscr_min={:?})",
            self.inner.coverage.dscr_min
        )
    }
}

// ---------------------------------------------------------------------------
// CorporateAnalysis
// ---------------------------------------------------------------------------

/// Unified corporate analysis result combining statement, equity, and credit
/// perspectives.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "CorporateAnalysis",
    frozen
)]
pub struct PyCorporateAnalysis {
    inner: CorporateAnalysis,
}

#[pymethods]
impl PyCorporateAnalysis {
    /// Full statement evaluation result (all nodes, all periods).
    #[getter]
    fn statement(&self) -> PyStatementResult {
        PyStatementResult::new(self.inner.statement.clone())
    }

    /// Equity valuation result (``None`` when no DCF was configured).
    #[getter]
    fn equity(&self) -> Option<PyCorporateValuationResult> {
        self.inner
            .equity
            .as_ref()
            .map(|e| PyCorporateValuationResult::new(e.clone()))
    }

    /// Per-instrument credit analysis keyed by instrument id.
    #[getter]
    fn credit(&self) -> HashMap<String, PyCreditInstrumentAnalysis> {
        self.inner
            .credit
            .iter()
            .map(|(k, v)| (k.clone(), PyCreditInstrumentAnalysis { inner: v.clone() }))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "CorporateAnalysis(equity={}, credit_instruments={})",
            if self.inner.equity.is_some() {
                "Some"
            } else {
                "None"
            },
            self.inner.credit.len()
        )
    }
}

// ---------------------------------------------------------------------------
// CorporateAnalysisBuilder
// ---------------------------------------------------------------------------

/// Builder for the corporate analysis pipeline.
///
/// The Rust builder uses move semantics (consuming ``self``), so internally the
/// builder is stored in an ``Option`` and taken when each method is called.
/// Calling :meth:`analyze` consumes the builder; a second call raises
/// ``RuntimeError``.
///
/// Example
/// -------
/// >>> builder = CorporateAnalysisBuilder(model)
/// >>> result = builder.dcf(0.10, tv).analyze()
/// >>> result.statement
/// StatementResult(...)
#[pyclass(
    module = "finstack.statements.analysis",
    name = "CorporateAnalysisBuilder"
)]
pub struct PyCorporateAnalysisBuilder {
    inner: Option<CorporateAnalysisBuilder>,
}

#[pymethods]
impl PyCorporateAnalysisBuilder {
    #[new]
    #[pyo3(signature = (model))]
    /// Create a new builder for the given financial model.
    fn new(model: &PyFinancialModelSpec) -> Self {
        Self {
            inner: Some(CorporateAnalysisBuilder::new(model.inner.clone())),
        }
    }

    /// Set the market context for curve-based discounting.
    fn market<'py>(mut slf: PyRefMut<'py, Self>, ctx: &PyMarketContext) -> PyRefMut<'py, Self> {
        if let Some(builder) = slf.inner.take() {
            slf.inner = Some(builder.market(ctx.inner.clone()));
        }
        slf
    }

    /// Set the as-of date for valuation.
    fn as_of<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let d = py_to_date(date)?;
        if let Some(builder) = slf.inner.take() {
            slf.inner = Some(builder.as_of(d));
        }
        Ok(slf)
    }

    /// Configure DCF equity valuation with default options.
    fn dcf<'py>(
        mut slf: PyRefMut<'py, Self>,
        wacc: f64,
        terminal_value: &PyTerminalValueSpec,
    ) -> PyRefMut<'py, Self> {
        if let Some(builder) = slf.inner.take() {
            slf.inner = Some(builder.dcf(wacc, terminal_value.inner.clone()));
        }
        slf
    }

    /// Configure DCF equity valuation with custom options.
    fn dcf_with_options<'py>(
        mut slf: PyRefMut<'py, Self>,
        wacc: f64,
        terminal_value: &PyTerminalValueSpec,
        options: &PyDcfOptions,
    ) -> PyRefMut<'py, Self> {
        if let Some(builder) = slf.inner.take() {
            slf.inner = Some(builder.dcf_with_options(
                wacc,
                terminal_value.inner.clone(),
                options.inner.clone(),
            ));
        }
        slf
    }

    /// Override the UFCF node name (default: ``"ufcf"``).
    fn dcf_node<'py>(mut slf: PyRefMut<'py, Self>, node: &str) -> PyRefMut<'py, Self> {
        if let Some(builder) = slf.inner.take() {
            slf.inner = Some(builder.dcf_node(node));
        }
        slf
    }

    /// Override net debt for the equity bridge calculation.
    fn net_debt_override(mut slf: PyRefMut<'_, Self>, net_debt: f64) -> PyRefMut<'_, Self> {
        if let Some(builder) = slf.inner.take() {
            slf.inner = Some(builder.net_debt_override(net_debt));
        }
        slf
    }

    /// Set the coverage node for credit metrics (default: ``"ebitda"``).
    fn coverage_node<'py>(mut slf: PyRefMut<'py, Self>, node: &str) -> PyRefMut<'py, Self> {
        if let Some(builder) = slf.inner.take() {
            slf.inner = Some(builder.coverage_node(node));
        }
        slf
    }

    /// Execute the analysis pipeline and return the combined result.
    ///
    /// The builder is consumed; calling this a second time raises
    /// ``RuntimeError``.
    fn analyze(&mut self, py: Python<'_>) -> PyResult<PyCorporateAnalysis> {
        let builder = self.inner.take().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "Builder already consumed by a previous call to analyze()",
            )
        })?;
        let result = py.detach(|| builder.analyze().map_err(stmt_to_py))?;
        Ok(PyCorporateAnalysis { inner: result })
    }

    fn __repr__(&self) -> String {
        if self.inner.is_some() {
            "CorporateAnalysisBuilder(active)".to_string()
        } else {
            "CorporateAnalysisBuilder(consumed)".to_string()
        }
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCreditInstrumentAnalysis>()?;
    module.add_class::<PyCorporateAnalysis>()?;
    module.add_class::<PyCorporateAnalysisBuilder>()?;
    Ok(vec![
        "CreditInstrumentAnalysis",
        "CorporateAnalysis",
        "CorporateAnalysisBuilder",
    ])
}
