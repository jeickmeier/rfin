use crate::core::common::args::{DayCountArg, TenorArg};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::dates::PyDayCount;
use crate::core::money::{extract_money, PyMoney};
use crate::valuations::common::parameters::{PyCashSettlementMethod, PyVolatilityModel};
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::swaption::{
    BermudanSchedule, BermudanSwaption, BermudanType, SABRParameters,
};
use finstack_valuations::instruments::rates::swaption::{Swaption, SwaptionParams};
use finstack_valuations::instruments::rates::swaption::{SwaptionExercise, SwaptionSettlement};
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::Bound;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

fn parse_settlement(label: Option<&str>) -> PyResult<SwaptionSettlement> {
    match label {
        None => Ok(SwaptionSettlement::Physical),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

fn parse_exercise(label: Option<&str>) -> PyResult<SwaptionExercise> {
    match label {
        None => Ok(SwaptionExercise::European),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

// ============================================================================
// SwaptionSettlement wrapper
// ============================================================================

/// Settlement type for swaptions (Physical or Cash).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "SwaptionSettlement",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PySwaptionSettlement {
    pub(crate) inner: SwaptionSettlement,
}

impl PySwaptionSettlement {
    pub(crate) const fn new(inner: SwaptionSettlement) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySwaptionSettlement {
    #[classattr]
    const PHYSICAL: Self = Self::new(SwaptionSettlement::Physical);
    #[classattr]
    const CASH: Self = Self::new(SwaptionSettlement::Cash);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("SwaptionSettlement('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

impl From<PySwaptionSettlement> for SwaptionSettlement {
    fn from(value: PySwaptionSettlement) -> Self {
        value.inner
    }
}

// ============================================================================
// SwaptionExercise wrapper
// ============================================================================

/// Exercise style for swaptions (European, Bermudan, or American).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "SwaptionExercise",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PySwaptionExercise {
    pub(crate) inner: SwaptionExercise,
}

impl PySwaptionExercise {
    pub(crate) const fn new(inner: SwaptionExercise) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySwaptionExercise {
    #[classattr]
    const EUROPEAN: Self = Self::new(SwaptionExercise::European);
    #[classattr]
    const BERMUDAN: Self = Self::new(SwaptionExercise::Bermudan);
    #[classattr]
    const AMERICAN: Self = Self::new(SwaptionExercise::American);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("SwaptionExercise('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

impl From<PySwaptionExercise> for SwaptionExercise {
    fn from(value: PySwaptionExercise) -> Self {
        value.inner
    }
}

// ============================================================================
// BermudanType wrapper
// ============================================================================

#[pyclass(
    module = "finstack.valuations.instruments.rates",
    name = "BermudanType",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyBermudanType {
    pub(crate) inner: BermudanType,
}

impl PyBermudanType {
    pub(crate) const fn new(inner: BermudanType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBermudanType {
    #[classattr]
    const CO_TERMINAL: Self = Self::new(BermudanType::CoTerminal);
    #[classattr]
    const NON_CO_TERMINAL: Self = Self::new(BermudanType::NonCoTerminal);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("BermudanType('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

impl From<PyBermudanType> for BermudanType {
    fn from(value: PyBermudanType) -> Self {
        value.inner
    }
}

// ============================================================================
// SABRParameters wrapper
// ============================================================================

#[pyclass(
    module = "finstack.valuations.instruments.rates",
    name = "SABRParameters",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySABRParameters {
    pub(crate) inner: SABRParameters,
}

#[pymethods]
impl PySABRParameters {
    #[new]
    #[pyo3(signature = (alpha, beta, nu, rho))]
    fn new(alpha: f64, beta: f64, nu: f64, rho: f64) -> PyResult<Self> {
        SABRParameters::new(alpha, beta, nu, rho)
            .map(|p| Self { inner: p })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[staticmethod]
    #[pyo3(signature = (alpha, beta, nu, rho, shift))]
    fn with_shift(alpha: f64, beta: f64, nu: f64, rho: f64, shift: f64) -> PyResult<Self> {
        SABRParameters::new_with_shift(alpha, beta, nu, rho, shift)
            .map(|p| Self { inner: p })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[staticmethod]
    fn rates_standard(alpha: f64, nu: f64, rho: f64) -> PyResult<Self> {
        SABRParameters::rates_standard(alpha, nu, rho)
            .map(|p| Self { inner: p })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[staticmethod]
    fn equity_standard(alpha: f64, nu: f64, rho: f64) -> PyResult<Self> {
        SABRParameters::equity_standard(alpha, nu, rho)
            .map(|p| Self { inner: p })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[staticmethod]
    fn normal(alpha: f64, nu: f64, rho: f64) -> PyResult<Self> {
        SABRParameters::normal(alpha, nu, rho)
            .map(|p| Self { inner: p })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[staticmethod]
    fn lognormal(alpha: f64, nu: f64, rho: f64) -> PyResult<Self> {
        SABRParameters::lognormal(alpha, nu, rho)
            .map(|p| Self { inner: p })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha
    }
    #[getter]
    fn beta(&self) -> f64 {
        self.inner.beta
    }
    #[getter]
    fn nu(&self) -> f64 {
        self.inner.nu
    }
    #[getter]
    fn rho(&self) -> f64 {
        self.inner.rho
    }
    #[getter]
    fn shift(&self) -> Option<f64> {
        self.inner.shift
    }

    fn __repr__(&self) -> String {
        format!(
            "SABRParameters(alpha={}, beta={}, nu={}, rho={})",
            self.inner.alpha, self.inner.beta, self.inner.nu, self.inner.rho
        )
    }
}

// ============================================================================
// BermudanSchedule wrapper
// ============================================================================

#[pyclass(
    module = "finstack.valuations.instruments.rates",
    name = "BermudanSchedule",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBermudanSchedule {
    pub(crate) inner: BermudanSchedule,
}

#[pymethods]
impl PyBermudanSchedule {
    #[new]
    #[pyo3(signature = (exercise_dates))]
    fn new(exercise_dates: &Bound<'_, PyAny>) -> PyResult<Self> {
        let iter = exercise_dates.try_iter()?;
        let mut dates = Vec::new();
        for item in iter {
            dates.push(py_to_date(&item?)?);
        }
        Ok(Self {
            inner: BermudanSchedule::new(dates),
        })
    }

    #[staticmethod]
    #[pyo3(signature = (first_exercise, swap_end, fixed_freq))]
    fn co_terminal(
        first_exercise: Bound<'_, PyAny>,
        swap_end: Bound<'_, PyAny>,
        fixed_freq: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let first = py_to_date(&first_exercise)?;
        let end = py_to_date(&swap_end)?;
        let TenorArg(freq) = fixed_freq.extract()?;
        BermudanSchedule::co_terminal(first, end, freq)
            .map(|s| Self { inner: s })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn with_lockout(&self, lockout_end: Bound<'_, PyAny>) -> PyResult<Self> {
        let d = py_to_date(&lockout_end)?;
        Ok(Self {
            inner: self.inner.clone().with_lockout(d),
        })
    }

    fn with_notice_days(&self, days: u32) -> Self {
        Self {
            inner: self.inner.clone().with_notice_days(days),
        }
    }

    #[getter]
    fn exercise_dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for d in &self.inner.exercise_dates {
            list.append(date_to_py(py, *d)?)?;
        }
        Ok(list.into())
    }

    #[getter]
    fn effective_dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for d in self.inner.effective_dates() {
            list.append(date_to_py(py, d)?)?;
        }
        Ok(list.into())
    }

    #[getter]
    fn num_exercises(&self) -> usize {
        self.inner.num_exercises()
    }

    fn __repr__(&self) -> String {
        format!("BermudanSchedule({} exercises)", self.inner.num_exercises())
    }
}

// ============================================================================
// Swaption wrapper
// ============================================================================

/// Swaption bindings with payer/receiver constructors.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "Swaption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySwaption {
    pub(crate) inner: Arc<Swaption>,
}

impl PySwaption {
    pub(crate) fn new(inner: Swaption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PySwaption {
    #[classmethod]
    #[pyo3(signature = (instrument_id, notional, strike, expiry, swap_start, swap_end, discount_curve, forward_curve, vol_surface, exercise=None, settlement=None, *, fixed_freq=None, float_freq=None, day_count=None, vol_model=None, calendar=None, cash_settlement_method=None, sabr_params=None, implied_volatility=None, attributes=None))]
    #[allow(clippy::too_many_arguments)]
    fn payer(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        swap_start: Bound<'_, PyAny>,
        swap_end: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        vol_surface: Bound<'_, PyAny>,
        exercise: Option<&str>,
        settlement: Option<&str>,
        fixed_freq: Option<&Bound<'_, PyAny>>,
        float_freq: Option<&Bound<'_, PyAny>>,
        day_count: Option<&Bound<'_, PyAny>>,
        vol_model: Option<&Bound<'_, PyAny>>,
        calendar: Option<&str>,
        cash_settlement_method: Option<&Bound<'_, PyAny>>,
        sabr_params: Option<PySABRParameters>,
        implied_volatility: Option<f64>,
        attributes: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        construct_swaption(
            instrument_id,
            notional,
            strike,
            expiry,
            swap_start,
            swap_end,
            discount_curve,
            forward_curve,
            vol_surface,
            exercise,
            settlement,
            true,
            fixed_freq,
            float_freq,
            day_count,
            vol_model,
            calendar,
            cash_settlement_method,
            sabr_params,
            implied_volatility,
            attributes,
        )
    }

    #[classmethod]
    #[pyo3(signature = (instrument_id, notional, strike, expiry, swap_start, swap_end, discount_curve, forward_curve, vol_surface, exercise=None, settlement=None, *, fixed_freq=None, float_freq=None, day_count=None, vol_model=None, calendar=None, cash_settlement_method=None, sabr_params=None, implied_volatility=None, attributes=None))]
    #[allow(clippy::too_many_arguments)]
    fn receiver(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        swap_start: Bound<'_, PyAny>,
        swap_end: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        vol_surface: Bound<'_, PyAny>,
        exercise: Option<&str>,
        settlement: Option<&str>,
        fixed_freq: Option<&Bound<'_, PyAny>>,
        float_freq: Option<&Bound<'_, PyAny>>,
        day_count: Option<&Bound<'_, PyAny>>,
        vol_model: Option<&Bound<'_, PyAny>>,
        calendar: Option<&str>,
        cash_settlement_method: Option<&Bound<'_, PyAny>>,
        sabr_params: Option<PySABRParameters>,
        implied_volatility: Option<f64>,
        attributes: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        construct_swaption(
            instrument_id,
            notional,
            strike,
            expiry,
            swap_start,
            swap_end,
            discount_curve,
            forward_curve,
            vol_surface,
            exercise,
            settlement,
            false,
            fixed_freq,
            float_freq,
            day_count,
            vol_model,
            calendar,
            cash_settlement_method,
            sabr_params,
            implied_volatility,
            attributes,
        )
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn strike(&self) -> f64 {
        rust_decimal::prelude::ToPrimitive::to_f64(&self.inner.strike).unwrap_or_default()
    }

    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    #[getter]
    fn swap_start(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.swap_start)
    }

    #[getter]
    fn swap_end(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.swap_end)
    }

    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "payer",
            OptionType::Put => "receiver",
        }
    }

    #[getter]
    fn settlement(&self) -> String {
        self.inner.settlement.to_string()
    }

    #[getter]
    fn exercise(&self) -> String {
        self.inner.exercise_style.to_string()
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[getter]
    fn vol_surface(&self) -> &str {
        self.inner.vol_surface_id.as_str()
    }

    #[getter]
    fn fixed_freq(&self) -> String {
        self.inner.fixed_freq.to_string()
    }

    #[getter]
    fn float_freq(&self) -> String {
        self.inner.float_freq.to_string()
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    #[getter]
    fn vol_model(&self) -> PyVolatilityModel {
        PyVolatilityModel::new(self.inner.vol_model)
    }

    #[getter]
    fn calendar(&self) -> Option<String> {
        self.inner.calendar_id.as_deref().map(String::from)
    }

    #[getter]
    fn cash_settlement_method(&self) -> PyCashSettlementMethod {
        PyCashSettlementMethod::new(self.inner.cash_settlement_method)
    }

    #[getter]
    fn sabr_params(&self) -> Option<PySABRParameters> {
        self.inner
            .sabr_params
            .as_ref()
            .map(|p| PySABRParameters { inner: p.clone() })
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Swaption)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Swaption(id='{}', type='{}')",
            self.inner.id,
            self.option_type()
        ))
    }
}

impl fmt::Display for PySwaption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Swaption({}, type={})",
            self.inner.id,
            self.option_type()
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn construct_swaption(
    instrument_id: Bound<'_, PyAny>,
    notional: Bound<'_, PyAny>,
    strike: f64,
    expiry: Bound<'_, PyAny>,
    swap_start: Bound<'_, PyAny>,
    swap_end: Bound<'_, PyAny>,
    discount_curve: Bound<'_, PyAny>,
    forward_curve: Bound<'_, PyAny>,
    vol_surface: Bound<'_, PyAny>,
    exercise: Option<&str>,
    settlement: Option<&str>,
    payer: bool,
    fixed_freq: Option<&Bound<'_, PyAny>>,
    float_freq: Option<&Bound<'_, PyAny>>,
    day_count: Option<&Bound<'_, PyAny>>,
    vol_model: Option<&Bound<'_, PyAny>>,
    calendar: Option<&str>,
    cash_settlement_method: Option<&Bound<'_, PyAny>>,
    sabr_params: Option<PySABRParameters>,
    implied_volatility: Option<f64>,
    attributes: Option<HashMap<String, String>>,
) -> PyResult<PySwaption> {
    use crate::errors::PyContext;
    let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
    let amt = extract_money(&notional).context("notional")?;
    let expiry_date = py_to_date(&expiry).context("expiry")?;
    let start = py_to_date(&swap_start).context("swap_start")?;
    let end = py_to_date(&swap_end).context("swap_end")?;
    let disc = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
    let fwd = CurveId::new(forward_curve.extract::<&str>().context("forward_curve")?);
    let exercise_style = parse_exercise(exercise)?;
    let settlement_type = parse_settlement(settlement)?;

    let params = if payer {
        SwaptionParams::payer(amt, strike, expiry_date, start, end)
    } else {
        SwaptionParams::receiver(amt, strike, expiry_date, start, end)
    };

    let vol_surface_id = vol_surface.extract::<&str>().context("vol_surface")?;

    let mut swaption = if payer {
        Swaption::new_payer(id, &params, disc, fwd, vol_surface_id)
    } else {
        Swaption::new_receiver(id, &params, disc, fwd, vol_surface_id)
    };

    swaption.exercise_style = exercise_style;
    swaption.settlement = settlement_type;
    if !payer {
        swaption.option_type = OptionType::Put;
    }

    if let Some(ff) = fixed_freq {
        let TenorArg(t) = ff.extract()?;
        swaption.fixed_freq = t;
    }
    if let Some(ff) = float_freq {
        let TenorArg(t) = ff.extract()?;
        swaption.float_freq = t;
    }
    if let Some(dc) = day_count {
        let DayCountArg(d) = dc.extract()?;
        swaption.day_count = d;
    }
    if let Some(vm) = vol_model {
        if let Ok(v) = vm.extract::<PyRef<PyVolatilityModel>>() {
            swaption.vol_model = v.inner;
        } else if let Ok(s) = vm.extract::<&str>() {
            swaption.vol_model = s.parse().map_err(|e: String| PyValueError::new_err(e))?;
        }
    }
    if let Some(cal) = calendar {
        swaption = swaption.with_calendar(cal);
    }
    if let Some(csm) = cash_settlement_method {
        if let Ok(c) = csm.extract::<PyRef<PyCashSettlementMethod>>() {
            swaption.cash_settlement_method = c.inner;
        } else if let Ok(s) = csm.extract::<&str>() {
            swaption.cash_settlement_method =
                s.parse().map_err(|e: String| PyValueError::new_err(e))?;
        }
    }
    if let Some(sabr) = sabr_params {
        swaption = swaption.with_sabr(sabr.inner);
    }
    if let Some(vol) = implied_volatility {
        swaption.pricing_overrides.market_quotes.implied_volatility = Some(vol);
    }
    if let Some(attrs) = attributes {
        for (k, v) in attrs {
            swaption.attributes.meta.insert(k, v);
        }
    }

    Ok(PySwaption::new(swaption))
}

// ============================================================================
// BermudanSwaption wrapper
// ============================================================================

#[pyclass(
    module = "finstack.valuations.instruments.rates",
    name = "BermudanSwaption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBermudanSwaption {
    pub(crate) inner: Arc<BermudanSwaption>,
}

impl PyBermudanSwaption {
    pub(crate) fn new(inner: BermudanSwaption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyBermudanSwaption {
    #[classmethod]
    #[pyo3(signature = (instrument_id, notional, strike, swap_start, swap_end, schedule, discount_curve, forward_curve, vol_surface, *, fixed_freq=None, float_freq=None, day_count=None, settlement=None, bermudan_type=None, calendar=None, implied_volatility=None, attributes=None))]
    #[allow(clippy::too_many_arguments)]
    fn payer(
        _cls: &Bound<'_, PyType>,
        instrument_id: &str,
        notional: Bound<'_, PyAny>,
        strike: f64,
        swap_start: Bound<'_, PyAny>,
        swap_end: Bound<'_, PyAny>,
        schedule: PyBermudanSchedule,
        discount_curve: &str,
        forward_curve: &str,
        vol_surface: &str,
        fixed_freq: Option<&Bound<'_, PyAny>>,
        float_freq: Option<&Bound<'_, PyAny>>,
        day_count: Option<&Bound<'_, PyAny>>,
        settlement: Option<&str>,
        bermudan_type: Option<&Bound<'_, PyAny>>,
        calendar: Option<&str>,
        implied_volatility: Option<f64>,
        attributes: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        construct_bermudan(
            instrument_id,
            &notional,
            strike,
            &swap_start,
            &swap_end,
            schedule,
            discount_curve,
            forward_curve,
            vol_surface,
            true,
            fixed_freq,
            float_freq,
            day_count,
            settlement,
            bermudan_type,
            calendar,
            implied_volatility,
            attributes,
        )
    }

    #[classmethod]
    #[pyo3(signature = (instrument_id, notional, strike, swap_start, swap_end, schedule, discount_curve, forward_curve, vol_surface, *, fixed_freq=None, float_freq=None, day_count=None, settlement=None, bermudan_type=None, calendar=None, implied_volatility=None, attributes=None))]
    #[allow(clippy::too_many_arguments)]
    fn receiver(
        _cls: &Bound<'_, PyType>,
        instrument_id: &str,
        notional: Bound<'_, PyAny>,
        strike: f64,
        swap_start: Bound<'_, PyAny>,
        swap_end: Bound<'_, PyAny>,
        schedule: PyBermudanSchedule,
        discount_curve: &str,
        forward_curve: &str,
        vol_surface: &str,
        fixed_freq: Option<&Bound<'_, PyAny>>,
        float_freq: Option<&Bound<'_, PyAny>>,
        day_count: Option<&Bound<'_, PyAny>>,
        settlement: Option<&str>,
        bermudan_type: Option<&Bound<'_, PyAny>>,
        calendar: Option<&str>,
        implied_volatility: Option<f64>,
        attributes: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        construct_bermudan(
            instrument_id,
            &notional,
            strike,
            &swap_start,
            &swap_end,
            schedule,
            discount_curve,
            forward_curve,
            vol_surface,
            false,
            fixed_freq,
            float_freq,
            day_count,
            settlement,
            bermudan_type,
            calendar,
            implied_volatility,
            attributes,
        )
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn strike(&self) -> f64 {
        rust_decimal::prelude::ToPrimitive::to_f64(&self.inner.strike).unwrap_or_default()
    }

    #[getter]
    fn swap_start(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.swap_start)
    }

    #[getter]
    fn swap_end(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.swap_end)
    }

    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "payer",
            OptionType::Put => "receiver",
        }
    }

    #[getter]
    fn settlement(&self) -> String {
        self.inner.settlement.to_string()
    }

    #[getter]
    fn bermudan_type(&self) -> PyBermudanType {
        PyBermudanType::new(self.inner.bermudan_type)
    }

    #[getter]
    fn exercise_dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for d in &self.inner.bermudan_schedule.exercise_dates {
            list.append(date_to_py(py, *d)?)?;
        }
        Ok(list.into())
    }

    #[getter]
    fn first_exercise(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match self.inner.first_exercise() {
            Some(d) => date_to_py(py, d).map(Some),
            None => Ok(None),
        }
    }

    #[getter]
    fn last_exercise(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match self.inner.last_exercise() {
            Some(d) => date_to_py(py, d).map(Some),
            None => Ok(None),
        }
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[getter]
    fn vol_surface(&self) -> &str {
        self.inner.vol_surface_id.as_str()
    }

    #[getter]
    fn fixed_freq(&self) -> String {
        self.inner.fixed_freq.to_string()
    }

    #[getter]
    fn float_freq(&self) -> String {
        self.inner.float_freq.to_string()
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    #[getter]
    fn calendar(&self) -> Option<String> {
        self.inner.calendar_id.as_deref().map(String::from)
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::BermudanSwaption)
    }

    fn __repr__(&self) -> String {
        format!(
            "BermudanSwaption('{}', {} exercises)",
            self.inner.id.as_str(),
            self.inner.bermudan_schedule.num_exercises()
        )
    }
}

impl fmt::Display for PyBermudanSwaption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BermudanSwaption({}, {} exercises)",
            self.inner.id,
            self.inner.bermudan_schedule.num_exercises()
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn construct_bermudan(
    instrument_id: &str,
    notional: &Bound<'_, PyAny>,
    strike: f64,
    swap_start: &Bound<'_, PyAny>,
    swap_end: &Bound<'_, PyAny>,
    schedule: PyBermudanSchedule,
    discount_curve: &str,
    forward_curve: &str,
    vol_surface: &str,
    payer: bool,
    fixed_freq: Option<&Bound<'_, PyAny>>,
    float_freq: Option<&Bound<'_, PyAny>>,
    day_count: Option<&Bound<'_, PyAny>>,
    settlement: Option<&str>,
    bermudan_type: Option<&Bound<'_, PyAny>>,
    calendar: Option<&str>,
    implied_volatility: Option<f64>,
    attributes: Option<HashMap<String, String>>,
) -> PyResult<PyBermudanSwaption> {
    use crate::errors::PyContext;
    let money = extract_money(notional).context("notional")?;
    let start = py_to_date(swap_start).context("swap_start")?;
    let end = py_to_date(swap_end).context("swap_end")?;

    let mut berm = if payer {
        BermudanSwaption::new_payer(
            instrument_id,
            money,
            strike,
            start,
            end,
            schedule.inner,
            discount_curve,
            forward_curve,
            vol_surface,
        )
    } else {
        BermudanSwaption::new_receiver(
            instrument_id,
            money,
            strike,
            start,
            end,
            schedule.inner,
            discount_curve,
            forward_curve,
            vol_surface,
        )
    };

    if let Some(ff) = fixed_freq {
        let TenorArg(t) = ff.extract()?;
        berm = berm.with_fixed_freq(t);
    }
    if let Some(ff) = float_freq {
        let TenorArg(t) = ff.extract()?;
        berm = berm.with_float_freq(t);
    }
    if let Some(dc) = day_count {
        let DayCountArg(d) = dc.extract()?;
        berm = berm.with_day_count(d);
    }
    if let Some(s) = settlement {
        berm = berm.with_settlement(s.parse().map_err(|e: String| PyValueError::new_err(e))?);
    }
    if let Some(bt) = bermudan_type {
        if let Ok(b) = bt.extract::<PyRef<PyBermudanType>>() {
            berm = berm.with_bermudan_type(b.inner);
        } else if let Ok(s) = bt.extract::<&str>() {
            berm =
                berm.with_bermudan_type(s.parse().map_err(|e: String| PyValueError::new_err(e))?);
        }
    }
    if let Some(cal) = calendar {
        berm = berm.with_calendar(cal);
    }
    if let Some(vol) = implied_volatility {
        berm.pricing_overrides.market_quotes.implied_volatility = Some(vol);
    }
    if let Some(attrs) = attributes {
        for (k, v) in attrs {
            berm.attributes.meta.insert(k, v);
        }
    }

    Ok(PyBermudanSwaption::new(berm))
}

// ============================================================================
// Module registration
// ============================================================================

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PySwaptionSettlement>()?;
    module.add_class::<PySwaptionExercise>()?;
    module.add_class::<PySwaption>()?;
    module.add_class::<PyBermudanType>()?;
    module.add_class::<PySABRParameters>()?;
    module.add_class::<PyBermudanSchedule>()?;
    module.add_class::<PyBermudanSwaption>()?;
    Ok(vec![
        "SwaptionSettlement",
        "SwaptionExercise",
        "Swaption",
        "BermudanType",
        "SABRParameters",
        "BermudanSchedule",
        "BermudanSwaption",
    ])
}
