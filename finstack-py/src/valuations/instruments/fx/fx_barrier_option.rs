use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::exotics::barrier_option::BarrierType;
use finstack_valuations::instruments::fx::fx_barrier_option::FxBarrierOption;
use finstack_valuations::instruments::OptionType;
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::str::FromStr;
use std::sync::Arc;

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxBarrierOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxBarrierOption {
    pub(crate) inner: Arc<FxBarrierOption>,
}

impl PyFxBarrierOption {
    pub(crate) fn new(inner: FxBarrierOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxBarrierOptionBuilder"
)]
pub struct PyFxBarrierOptionBuilder {
    instrument_id: InstrumentId,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    strike: Option<f64>,
    barrier: Option<f64>,
    rebate: Option<f64>,
    option_type: OptionType,
    barrier_type: Option<BarrierType>,
    expiry: Option<time::Date>,
    notional: Option<finstack_core::money::Money>,
    domestic_discount_curve_id: Option<CurveId>,
    foreign_discount_curve_id: Option<CurveId>,
    vol_surface_id: Option<CurveId>,
    fx_spot_id: Option<String>,
    use_gobet_miri: bool,
    day_count: DayCount,
}

impl PyFxBarrierOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            base_currency: None,
            quote_currency: None,
            strike: None,
            barrier: None,
            rebate: None,
            option_type: OptionType::Call,
            barrier_type: None,
            expiry: None,
            notional: None,
            domestic_discount_curve_id: None,
            foreign_discount_curve_id: None,
            vol_surface_id: None,
            fx_spot_id: None,
            use_gobet_miri: false,
            day_count: DayCount::Act365F,
        }
    }
}

#[pymethods]
impl PyFxBarrierOptionBuilder {
    fn base_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(c) = ccy.extract().context("base_currency")?;
        slf.base_currency = Some(c);
        Ok(slf)
    }

    fn quote_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(c) = ccy.extract().context("quote_currency")?;
        slf.quote_currency = Some(c);
        Ok(slf)
    }

    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike);
        slf
    }

    fn barrier(mut slf: PyRefMut<'_, Self>, barrier: f64) -> PyRefMut<'_, Self> {
        slf.barrier = Some(barrier);
        slf
    }

    fn rebate(mut slf: PyRefMut<'_, Self>, rebate: f64) -> PyRefMut<'_, Self> {
        slf.rebate = Some(rebate);
        slf
    }

    fn option_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        option_type: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.option_type =
            OptionType::from_str(option_type).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(slf)
    }

    fn barrier_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        barrier_type: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.barrier_type = Some(
            BarrierType::from_str(barrier_type)
                .map_err(|e| PyValueError::new_err(e.to_string()))?,
        );
        Ok(slf)
    }

    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry = Some(py_to_date(&date).context("expiry")?);
        Ok(slf)
    }

    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    fn domestic_discount_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: &str,
    ) -> PyRefMut<'py, Self> {
        slf.domestic_discount_curve_id = Some(CurveId::new(curve_id));
        slf
    }

    fn foreign_discount_curve<'py>(
        mut slf: PyRefMut<'py, Self>,
        curve_id: &str,
    ) -> PyRefMut<'py, Self> {
        slf.foreign_discount_curve_id = Some(CurveId::new(curve_id));
        slf
    }

    fn vol_surface<'py>(mut slf: PyRefMut<'py, Self>, surface_id: &str) -> PyRefMut<'py, Self> {
        slf.vol_surface_id = Some(CurveId::new(surface_id));
        slf
    }

    fn fx_spot_id<'py>(mut slf: PyRefMut<'py, Self>, spot_id: &str) -> PyRefMut<'py, Self> {
        slf.fx_spot_id = Some(spot_id.to_string());
        slf
    }

    fn use_gobet_miri(mut slf: PyRefMut<'_, Self>, flag: bool) -> PyRefMut<'_, Self> {
        slf.use_gobet_miri = flag;
        slf
    }

    fn day_count<'py>(mut slf: PyRefMut<'py, Self>, dc: &PyDayCount) -> PyRefMut<'py, Self> {
        slf.day_count = dc.inner;
        slf
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyFxBarrierOption> {
        let base = slf
            .base_currency
            .ok_or_else(|| PyValueError::new_err("base_currency is required"))?;
        let quote = slf
            .quote_currency
            .ok_or_else(|| PyValueError::new_err("quote_currency is required"))?;
        let strike = slf
            .strike
            .ok_or_else(|| PyValueError::new_err("strike is required"))?;
        let barrier = slf
            .barrier
            .ok_or_else(|| PyValueError::new_err("barrier is required"))?;
        let barrier_type = slf
            .barrier_type
            .ok_or_else(|| PyValueError::new_err("barrier_type is required"))?;
        let expiry = slf
            .expiry
            .ok_or_else(|| PyValueError::new_err("expiry is required"))?;
        let notional = slf
            .notional
            .ok_or_else(|| PyValueError::new_err("notional is required"))?;
        let domestic = slf
            .domestic_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("domestic_discount_curve is required"))?;
        let foreign = slf
            .foreign_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("foreign_discount_curve is required"))?;
        let vol = slf
            .vol_surface_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("vol_surface is required"))?;

        let mut builder = FxBarrierOption::builder();
        builder = builder.id(slf.instrument_id.clone());
        builder = builder.strike(strike);
        builder = builder.barrier(barrier);
        builder = builder.option_type(slf.option_type);
        builder = builder.barrier_type(barrier_type);
        builder = builder.expiry(expiry);
        builder = builder.notional(notional);
        builder = builder.base_currency(base);
        builder = builder.quote_currency(quote);
        builder = builder.day_count(slf.day_count);
        builder = builder.use_gobet_miri(slf.use_gobet_miri);
        builder = builder.domestic_discount_curve_id(domestic);
        builder = builder.foreign_discount_curve_id(foreign);
        builder = builder.fx_spot_id_opt(slf.fx_spot_id.as_ref().map(|s| s.clone().into()));
        builder = builder.vol_surface_id(vol);
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.attributes(finstack_valuations::instruments::Attributes::new());
        if let Some(rebate) = slf.rebate {
            builder = builder.rebate_opt(Some(rebate));
        }
        let option = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to build FxBarrierOption: {e}"
            ))
        })?;
        Ok(PyFxBarrierOption::new(option))
    }

    fn __repr__(&self) -> String {
        format!("FxBarrierOptionBuilder(id='{}')", self.instrument_id)
    }
}

#[pymethods]
impl PyFxBarrierOption {
    #[classmethod]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyFxBarrierOptionBuilder {
        PyFxBarrierOptionBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxBarrierOption)
    }

    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote_currency)
    }

    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    #[getter]
    fn barrier(&self) -> f64 {
        self.inner.barrier
    }

    #[getter]
    fn rebate(&self) -> Option<f64> {
        self.inner.rebate
    }

    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    #[getter]
    fn barrier_type(&self) -> &'static str {
        match self.inner.barrier_type {
            BarrierType::UpAndOut => "up_and_out",
            BarrierType::UpAndIn => "up_and_in",
            BarrierType::DownAndOut => "down_and_out",
            BarrierType::DownAndIn => "down_and_in",
        }
    }

    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn domestic_discount_curve(&self) -> String {
        self.inner.domestic_discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn foreign_discount_curve(&self) -> String {
        self.inner.foreign_discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn vol_surface(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    #[getter]
    fn fx_spot_id(&self) -> Option<String> {
        self.inner
            .fx_spot_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    #[getter]
    fn use_gobet_miri(&self) -> bool {
        self.inner.use_gobet_miri
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Calculate present value of the FX barrier option.
    fn value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| Instrument::value(self.inner.as_ref(), &market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn __repr__(&self) -> String {
        format!(
            "FxBarrierOption(id='{}', strike={}, barrier={}, barrier_type='{}')",
            self.inner.id.as_str(),
            self.inner.strike,
            self.inner.barrier,
            match self.inner.barrier_type {
                BarrierType::UpAndOut => "up_and_out",
                BarrierType::UpAndIn => "up_and_in",
                BarrierType::DownAndOut => "down_and_out",
                BarrierType::DownAndIn => "down_and_in",
            }
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyFxBarrierOption>()?;
    parent.add_class::<PyFxBarrierOptionBuilder>()?;
    Ok(vec!["FxBarrierOption", "FxBarrierOptionBuilder"])
}
