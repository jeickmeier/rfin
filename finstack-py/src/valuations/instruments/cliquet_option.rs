// Reverted start_date
use crate::core::dates::utils::date_to_py;
use crate::core::money::{extract_money, PyMoney};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::cliquet_option::CliquetOption;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::Bound;

/// Cliquet option instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CliquetOption",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCliquetOption {
    pub(crate) inner: CliquetOption,
}

impl PyCliquetOption {
    pub(crate) fn new(inner: CliquetOption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCliquetOption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, ticker, reset_dates, local_cap, global_cap, notional, discount_curve, spot_id, vol_surface, *, div_yield_id=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a cliquet option.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     ticker: Equity ticker symbol.
    ///     reset_dates: List of reset dates.
    ///     local_cap: Local cap for each period.
    ///     global_cap: Global cap for total return.
    ///     notional: Contract notional as :class:`finstack.core.money.Money`.
    ///     discount_curve: Discount curve identifier.
    ///     spot_id: Spot price identifier.
    ///     vol_surface: Volatility surface identifier.
    ///     div_yield_id: Optional dividend yield identifier.
    ///
    /// Returns:
    ///     CliquetOption: Configured cliquet option instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        reset_dates: Bound<'_, PyList>,
        local_cap: f64,
        global_cap: f64,
        notional: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        spot_id: &str,
        vol_surface: Bound<'_, PyAny>,
        div_yield_id: Option<&str>,
    ) -> PyResult<Self> {
        use crate::core::dates::utils::py_to_date;
        use crate::errors::PyContext;
        use finstack_core::dates::DayCount;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let notional_money = extract_money(&notional).context("notional")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let vol_surface_id = CurveId::new(vol_surface.extract::<&str>().context("vol_surface")?);

        // Parse reset dates
        let mut reset_dates_vec = Vec::new();
        for item in reset_dates.iter() {
            reset_dates_vec.push(py_to_date(&item).context("reset_dates")?);
        }

        let mut builder = CliquetOption::builder();
        builder = builder.id(id);
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.reset_dates(reset_dates_vec);
        builder = builder.local_cap(local_cap);
        builder = builder.local_floor(0.0);
        builder = builder.global_cap(global_cap);
        builder = builder.global_floor(0.0);
        builder = builder.notional(notional_money);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.discount_curve_id(discount_curve_id);
        builder = builder.spot_id(spot_id.to_string());
        builder = builder.vol_surface_id(vol_surface_id.into());
        if let Some(div) = div_yield_id {
            builder = builder.div_yield_id(div.to_string());
        }
        let option = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to build CliquetOption: {e}"))
        })?;
        Ok(Self::new(option))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Underlying ticker symbol.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.underlying_ticker
    }

    /// Local cap.
    #[getter]
    fn local_cap(&self) -> f64 {
        self.inner.local_cap
    }

    /// Global cap.
    #[getter]
    fn global_cap(&self) -> f64 {
        self.inner.global_cap
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Reset dates.
    #[getter]
    fn reset_dates(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dates = PyList::empty(py);
        for d in &self.inner.reset_dates {
            dates.append(date_to_py(py, *d)?)?;
        }
        Ok(dates.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "CliquetOption(id='{}', ticker='{}', local_cap={}, global_cap={})",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.local_cap,
            self.inner.global_cap
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyCliquetOption>()?;
    Ok(vec!["CliquetOption"])
}
