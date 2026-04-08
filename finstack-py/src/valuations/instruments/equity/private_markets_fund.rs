//! Rust source: `finstack/valuations/src/instruments/equity/pe_fund/`
//! Full name (`private_markets_fund`) used in Python instead of Rust abbreviation.

use crate::core::currency::PyCurrency;
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::cashflow::builder::PyCashFlowSchedule;
use crate::valuations::common::PyInstrumentType;
use finstack_valuations::instruments::equity::pe_fund::PrivateMarketsFund;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;
use std::fmt;
use std::sync::Arc;

/// Private markets fund instrument wrapper parsed from JSON definitions.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PrivateMarketsFund",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPrivateMarketsFund {
    pub(crate) inner: Arc<PrivateMarketsFund>,
}

impl PyPrivateMarketsFund {
    pub(crate) fn new(inner: PrivateMarketsFund) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyPrivateMarketsFund {
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::PrivateMarketsFund)
    }

    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    #[getter]
    fn discount_curve(&self) -> Option<String> {
        self.inner
            .discount_curve_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    #[pyo3(text_signature = "($self, market, as_of)")]
    fn cashflow_schedule(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyCashFlowSchedule> {
        use finstack_valuations::cashflow::CashflowProvider;

        let date = py_to_date(&as_of).context("as_of")?;
        self.inner
            .cashflow_schedule(&market.inner, date)
            .map(PyCashFlowSchedule::new)
            .map_err(core_to_py)
    }

    #[pyo3(text_signature = "(self)")]
    fn run_waterfall(&self) -> PyResult<String> {
        let ledger = self.inner.run_waterfall().map_err(core_to_py)?;
        ledger.to_json().map_err(core_to_py)
    }

    #[pyo3(text_signature = "(self)")]
    fn run_waterfall_tabular(&self) -> PyResult<(Vec<String>, Vec<Vec<String>>)> {
        let ledger = self.inner.run_waterfall().map_err(core_to_py)?;
        let (headers, rows) = ledger.to_tabular_data();
        Ok((headers.into_iter().map(|h| h.to_string()).collect(), rows))
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "PrivateMarketsFund(id='{}', events={})",
            self.inner.id,
            self.inner.events.len()
        ))
    }
}

impl fmt::Display for PyPrivateMarketsFund {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PrivateMarketsFund({}, events={})",
            self.inner.id,
            self.inner.events.len()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyPrivateMarketsFund>()?;
    module.setattr(
        "__doc__",
        "Private markets fund instrument parsed from JSON definitions (cashflow schedules, waterfall specs).",
    )?;
    Ok(vec!["PrivateMarketsFund"])
}
