use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use finstack_valuations::instruments::range_accrual::RangeAccrual;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::Bound;
use finstack_core::types::{CurveId, InstrumentId};

/// Range accrual instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RangeAccrual",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyRangeAccrual {
    pub(crate) inner: RangeAccrual,
}

impl PyRangeAccrual {
    pub(crate) fn new(inner: RangeAccrual) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRangeAccrual {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, ticker, observation_dates, lower_bound, upper_bound, coupon_rate, notional, discount_curve, spot_id, vol_surface, *, div_yield_id=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a range accrual instrument.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     ticker: Equity ticker symbol.
    ///     observation_dates: List of observation dates.
    ///     lower_bound: Lower bound for range.
    ///     upper_bound: Upper bound for range.
    ///     coupon_rate: Coupon rate in decimal form.
    ///     notional: Contract notional as :class:`finstack.core.money.Money`.
    ///     discount_curve: Discount curve identifier.
    ///     spot_id: Spot price identifier.
    ///     vol_surface: Volatility surface identifier.
    ///     div_yield_id: Optional dividend yield identifier.
    ///
    /// Returns:
    ///     RangeAccrual: Configured range accrual instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        observation_dates: Bound<'_, PyList>,
        lower_bound: f64,
        upper_bound: f64,
        coupon_rate: f64,
        notional: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        spot_id: &str,
        vol_surface: Bound<'_, PyAny>,
        div_yield_id: Option<&str>,
    ) -> PyResult<Self> {
        use finstack_core::dates::DayCount;

        let id = InstrumentId::new(instrument_id.extract::<&str>()?);
        let notional_money = extract_money(&notional)?;
        let discount_curve_id = CurveId::new(discount_curve.extract::<&str>()?);
        let vol_surface_id = CurveId::new(vol_surface.extract::<&str>()?);

        // Parse observation dates
        let mut obs_dates = Vec::new();
        for item in observation_dates.iter() {
            obs_dates.push(py_to_date(&item)?);
        }

        let mut builder = RangeAccrual::builder();
        builder = builder.id(id);
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.observation_dates(obs_dates);
        builder = builder.lower_bound(lower_bound);
        builder = builder.upper_bound(upper_bound);
        builder = builder.coupon_rate(coupon_rate);
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
        let range_accrual = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to build RangeAccrual: {e}"))
        })?;
        Ok(Self::new(range_accrual))
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

    /// Lower bound.
    #[getter]
    fn lower_bound(&self) -> f64 {
        self.inner.lower_bound
    }

    /// Upper bound.
    #[getter]
    fn upper_bound(&self) -> f64 {
        self.inner.upper_bound
    }

    /// Coupon rate.
    #[getter]
    fn coupon_rate(&self) -> f64 {
        self.inner.coupon_rate
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Observation dates.
    #[getter]
    fn observation_dates(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dates = PyList::empty(py);
        for d in &self.inner.observation_dates {
            dates.append(date_to_py(py, *d)?)?;
        }
        Ok(dates.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "RangeAccrual(id='{}', ticker='{}', lower_bound={}, upper_bound={})",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.lower_bound,
            self.inner.upper_bound
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyRangeAccrual>()?;
    Ok(vec!["RangeAccrual"])
}
