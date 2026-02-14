//! Market builder bindings (quote -> instrument).
//!
//! This module exposes a thin Python surface over `finstack_valuations::market`:
//! - Quote schemas (rates / CDS / CDS tranche)
//! - Build context (`BuildCtx`)
//! - Builder functions that produce an instrument ready for pricing

use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::dates::{PyBusinessDayConvention, PyDayCount, PyTenor};
use crate::valuations::common::PyInstrumentType;
use crate::valuations::conventions::PyCdsConventionKey;
use finstack_core::dates::Tenor;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::market::conventions::ids::{IndexId, IrFutureContractId};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::cds_tranche::CDSTrancheQuote;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::rates::RateQuote;
use finstack_valuations::market::{build_cds_instrument, build_cds_tranche_instrument};
use finstack_valuations::market::{build_rate_instrument, BuildCtx, CDSTrancheBuildOverrides};
use pyo3::exceptions::{PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

fn parse_quote_id(obj: &Bound<'_, PyAny>) -> PyResult<QuoteId> {
    if let Ok(py_id) = obj.extract::<PyRef<PyQuoteId>>() {
        Ok(py_id.inner.clone())
    } else if let Ok(s) = obj.extract::<&str>() {
        Ok(QuoteId::new(s))
    } else {
        Err(PyTypeError::new_err("Expected QuoteId or string"))
    }
}

fn parse_pillar(obj: &Bound<'_, PyAny>) -> PyResult<Pillar> {
    if let Ok(py_pillar) = obj.extract::<PyRef<PyPillar>>() {
        return Ok(py_pillar.inner.clone());
    }
    if let Ok(tenor) = obj.extract::<PyRef<'_, PyTenor>>() {
        return Ok(Pillar::Tenor(tenor.inner));
    }
    if let Ok(text) = obj.extract::<&str>() {
        if let Ok(tenor) = Tenor::parse(text) {
            return Ok(Pillar::Tenor(tenor));
        }
    }
    py_to_date(obj).map(Pillar::Date)
}

fn parse_index_id(obj: &Bound<'_, PyAny>) -> PyResult<IndexId> {
    if let Ok(text) = obj.extract::<&str>() {
        return Ok(IndexId::new(text));
    }
    Err(PyTypeError::new_err(
        "Expected index id string like 'USD-SOFR-3M'",
    ))
}

/// Stable identifier for a market quote.
#[pyclass(module = "finstack.valuations.market", name = "QuoteId", frozen)]
#[derive(Clone, Debug)]
pub struct PyQuoteId {
    pub(crate) inner: QuoteId,
}

impl PyQuoteId {
    pub(crate) fn new(inner: QuoteId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyQuoteId {
    #[new]
    #[pyo3(text_signature = "(id)")]
    fn ctor(id: String) -> Self {
        Self::new(QuoteId::new(id))
    }

    #[getter]
    fn value(&self) -> String {
        self.inner.as_str().to_string()
    }

    fn __repr__(&self) -> String {
        format!("QuoteId('{}')", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.as_str().to_string()
    }
}

/// Pillar for quote maturity (tenor or date).
#[pyclass(module = "finstack.valuations.market", name = "Pillar", frozen)]
#[derive(Clone, Debug)]
pub struct PyPillar {
    pub(crate) inner: Pillar,
}

impl PyPillar {
    pub(crate) fn new(inner: Pillar) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPillar {
    #[classmethod]
    #[pyo3(signature = (tenor,), text_signature = "(cls, tenor)")]
    fn tenor(_cls: &Bound<'_, PyType>, tenor: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(t) = tenor.extract::<PyRef<'_, PyTenor>>() {
            Ok(Self::new(Pillar::Tenor(t.inner)))
        } else if let Ok(text) = tenor.extract::<&str>() {
            let t = Tenor::parse(text)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            Ok(Self::new(Pillar::Tenor(t)))
        } else {
            Err(PyTypeError::new_err("Expected Tenor or string like '5Y'"))
        }
    }

    #[classmethod]
    #[pyo3(signature = (date,), text_signature = "(cls, date)")]
    fn date(_cls: &Bound<'_, PyType>, date: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(Pillar::Date(py_to_date(date)?)))
    }

    fn __repr__(&self) -> String {
        format!("Pillar('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Build context for quote-to-instrument construction.
#[pyclass(module = "finstack.valuations.market", name = "BuildCtx")]
#[derive(Clone, Debug)]
pub struct PyBuildCtx {
    pub(crate) inner: BuildCtx,
}

impl PyBuildCtx {
    pub(crate) fn new(inner: BuildCtx) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBuildCtx {
    #[new]
    #[pyo3(signature = (as_of, notional, *, curve_ids=None), text_signature = "(as_of, notional, *, curve_ids=None)")]
    fn ctor(
        as_of: &Bound<'_, PyAny>,
        notional: f64,
        curve_ids: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let as_of = py_to_date(as_of)?;
        let curve_ids = if let Some(obj) = curve_ids {
            obj.extract::<finstack_core::HashMap<String, String>>()?
        } else {
            finstack_core::HashMap::default()
        };
        Ok(Self::new(BuildCtx::new(as_of, notional, curve_ids)))
    }

    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.as_of())
    }

    #[getter]
    fn notional(&self) -> f64 {
        self.inner.notional()
    }

    #[pyo3(text_signature = "(self, role)")]
    fn curve_id(&self, role: &str) -> Option<String> {
        self.inner.curve_id(role).map(String::from)
    }

    fn __repr__(&self) -> String {
        format!(
            "BuildCtx(as_of={}, notional={})",
            self.inner.as_of(),
            self.inner.notional()
        )
    }
}

/// An instrument produced by a market builder.
///
/// This wraps a Rust `Box<dyn Instrument>` and is accepted anywhere an instrument is expected
/// (pricer, portfolio, etc.) via the Python bindings.
#[pyclass(
    module = "finstack.valuations.market",
    name = "BuiltInstrument",
    unsendable
)]
pub struct PyBuiltInstrument {
    pub(crate) inner: Box<dyn Instrument>,
}

impl PyBuiltInstrument {
    pub(crate) fn new(inner: Box<dyn Instrument>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBuiltInstrument {
    #[getter]
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(self.inner.key())
    }

    fn __repr__(&self) -> String {
        format!(
            "BuiltInstrument(id={}, instrument_type={})",
            self.inner.id(),
            self.inner.key()
        )
    }
}

/// Rates quote schema.
#[pyclass(module = "finstack.valuations.market", name = "RateQuote", frozen)]
#[derive(Clone, Debug)]
pub struct PyRateQuote {
    pub(crate) inner: RateQuote,
}

impl PyRateQuote {
    pub(crate) fn new(inner: RateQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRateQuote {
    #[classmethod]
    #[pyo3(signature = (id, index, pillar, rate), text_signature = "(cls, id, index, pillar, rate)")]
    fn deposit(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        index: &Bound<'_, PyAny>,
        pillar: &Bound<'_, PyAny>,
        rate: f64,
    ) -> PyResult<Self> {
        Ok(Self::new(RateQuote::Deposit {
            id: parse_quote_id(id)?,
            index: parse_index_id(index)?,
            pillar: parse_pillar(pillar)?,
            rate,
        }))
    }

    #[classmethod]
    #[pyo3(signature = (id, index, start, end, rate), text_signature = "(cls, id, index, start, end, rate)")]
    fn fra(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        index: &Bound<'_, PyAny>,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
        rate: f64,
    ) -> PyResult<Self> {
        Ok(Self::new(RateQuote::Fra {
            id: parse_quote_id(id)?,
            index: parse_index_id(index)?,
            start: parse_pillar(start)?,
            end: parse_pillar(end)?,
            rate,
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (id, expiry, price, *, contract=None, convexity_adjustment=None, vol_surface_id=None),
        text_signature = "(cls, id, expiry, price, *, contract=None, convexity_adjustment=None, vol_surface_id=None)"
    )]
    fn future(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        expiry: &Bound<'_, PyAny>,
        price: f64,
        contract: Option<&str>,
        convexity_adjustment: Option<f64>,
        vol_surface_id: Option<&str>,
    ) -> PyResult<Self> {
        Ok(Self::new(RateQuote::Futures {
            id: parse_quote_id(id)?,
            contract: IrFutureContractId::new(contract.unwrap_or("UNKNOWN")),
            expiry: py_to_date(expiry)?,
            price,
            convexity_adjustment,
            vol_surface_id: vol_surface_id.map(CurveId::new),
        }))
    }

    #[classmethod]
    #[pyo3(signature = (id, index, pillar, rate, *, spread_decimal=None), text_signature = "(cls, id, index, pillar, rate, *, spread_decimal=None)")]
    fn swap(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        index: &Bound<'_, PyAny>,
        pillar: &Bound<'_, PyAny>,
        rate: f64,
        spread_decimal: Option<f64>,
    ) -> PyResult<Self> {
        Ok(Self::new(RateQuote::Swap {
            id: parse_quote_id(id)?,
            index: parse_index_id(index)?,
            pillar: parse_pillar(pillar)?,
            rate,
            spread_decimal,
        }))
    }

    #[getter]
    fn id(&self) -> PyQuoteId {
        PyQuoteId::new(self.inner.id().clone())
    }

    fn __repr__(&self) -> String {
        let ty = match &self.inner {
            RateQuote::Deposit { .. } => "deposit",
            RateQuote::Fra { .. } => "fra",
            RateQuote::Futures { .. } => "futures",
            RateQuote::Swap { .. } => "swap",
        };
        format!("RateQuote(type='{}', id='{}')", ty, self.inner.id())
    }
}

/// CDS quote schema.
#[pyclass(module = "finstack.valuations.market", name = "CdsQuote", frozen)]
#[derive(Clone, Debug)]
pub struct PyCdsQuote {
    pub(crate) inner: CdsQuote,
}

impl PyCdsQuote {
    pub(crate) fn new(inner: CdsQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCdsQuote {
    #[classmethod]
    #[pyo3(
        signature = (id, entity, convention, pillar, spread_bp, *, recovery_rate=0.40),
        text_signature = "(cls, id, entity, convention, pillar, spread_bp, *, recovery_rate=0.40)"
    )]
    fn par_spread(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        entity: String,
        convention: &PyCdsConventionKey,
        pillar: &Bound<'_, PyAny>,
        spread_bp: f64,
        recovery_rate: f64,
    ) -> PyResult<Self> {
        Ok(Self::new(CdsQuote::CdsParSpread {
            id: parse_quote_id(id)?,
            entity,
            convention: convention.inner.clone(),
            pillar: parse_pillar(pillar)?,
            spread_bp,
            recovery_rate,
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (id, entity, convention, pillar, running_spread_bp, upfront_pct, *, recovery_rate=0.40),
        text_signature = "(cls, id, entity, convention, pillar, running_spread_bp, upfront_pct, *, recovery_rate=0.40)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn upfront(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        entity: String,
        convention: &PyCdsConventionKey,
        pillar: &Bound<'_, PyAny>,
        running_spread_bp: f64,
        upfront_pct: f64,
        recovery_rate: f64,
    ) -> PyResult<Self> {
        Ok(Self::new(CdsQuote::CdsUpfront {
            id: parse_quote_id(id)?,
            entity,
            convention: convention.inner.clone(),
            pillar: parse_pillar(pillar)?,
            running_spread_bp,
            upfront_pct,
            recovery_rate,
        }))
    }

    #[getter]
    fn id(&self) -> PyQuoteId {
        PyQuoteId::new(self.inner.id().clone())
    }

    fn __repr__(&self) -> String {
        let ty = match &self.inner {
            CdsQuote::CdsParSpread { .. } => "cds_par_spread",
            CdsQuote::CdsUpfront { .. } => "cds_upfront",
        };
        format!("CdsQuote(type='{}', id='{}')", ty, self.inner.id())
    }
}

/// CDS tranche quote schema.
#[pyclass(
    module = "finstack.valuations.market",
    name = "CdsTrancheQuote",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCDSTrancheQuote {
    pub(crate) inner: CDSTrancheQuote,
}

impl PyCDSTrancheQuote {
    pub(crate) fn new(inner: CDSTrancheQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCDSTrancheQuote {
    #[classmethod]
    #[pyo3(
        signature = (id, index, attachment, detachment, maturity, upfront_pct, running_spread_bp, convention),
        text_signature = "(cls, id, index, attachment, detachment, maturity, upfront_pct, running_spread_bp, convention)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn cds_tranche(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        index: String,
        attachment: f64,
        detachment: f64,
        maturity: &Bound<'_, PyAny>,
        upfront_pct: f64,
        running_spread_bp: f64,
        convention: &PyCdsConventionKey,
    ) -> PyResult<Self> {
        Ok(Self::new(CDSTrancheQuote::CDSTranche {
            id: parse_quote_id(id)?,
            index,
            attachment,
            detachment,
            maturity: py_to_date(maturity)?,
            upfront_pct,
            running_spread_bp,
            convention: convention.inner.clone(),
        }))
    }

    #[getter]
    fn id(&self) -> PyQuoteId {
        PyQuoteId::new(self.inner.id().clone())
    }

    fn __repr__(&self) -> String {
        format!("CDSTrancheQuote(id='{}')", self.inner.id())
    }
}

/// Overrides for CDS tranche schedule and index metadata during build.
#[pyclass(
    module = "finstack.valuations.market",
    name = "CDSTrancheBuildOverrides"
)]
#[derive(Clone, Debug)]
pub struct PyCDSTrancheBuildOverrides {
    pub(crate) inner: CDSTrancheBuildOverrides,
}

impl PyCDSTrancheBuildOverrides {
    pub(crate) fn new(inner: CDSTrancheBuildOverrides) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCDSTrancheBuildOverrides {
    #[new]
    #[pyo3(
        signature = (series, *, payment_frequency=None, day_count=None, business_day_convention=None, calendar_id=None, use_imm_dates=true),
        text_signature = "(series, *, payment_frequency=None, day_count=None, business_day_convention=None, calendar_id=None, use_imm_dates=True)"
    )]
    fn ctor(
        series: u16,
        payment_frequency: Option<&Bound<'_, PyAny>>,
        day_count: Option<PyRef<'_, PyDayCount>>,
        business_day_convention: Option<PyRef<'_, PyBusinessDayConvention>>,
        calendar_id: Option<String>,
        use_imm_dates: bool,
    ) -> PyResult<Self> {
        let mut inner = CDSTrancheBuildOverrides::new(series);
        inner.use_imm_dates = use_imm_dates;
        if let Some(freq) = payment_frequency {
            if let Ok(t) = freq.extract::<PyRef<'_, PyTenor>>() {
                inner.frequency = Some(t.inner);
            } else if let Ok(text) = freq.extract::<&str>() {
                inner.frequency = Some(
                    Tenor::parse(text)
                        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?,
                );
            } else {
                return Err(PyTypeError::new_err(
                    "payment_frequency must be Tenor or string like '3M'",
                ));
            }
        }
        if let Some(dc) = day_count {
            inner.day_count = Some(dc.inner);
        }
        if let Some(bdc) = business_day_convention {
            inner.bdc = Some(bdc.inner);
        }
        inner.calendar_id = calendar_id;
        Ok(Self::new(inner))
    }

    #[getter]
    fn series(&self) -> u16 {
        self.inner.series
    }

    #[getter]
    fn use_imm_dates(&self) -> bool {
        self.inner.use_imm_dates
    }

    fn __repr__(&self) -> String {
        format!(
            "CDSTrancheBuildOverrides(series={}, use_imm_dates={})",
            self.inner.series, self.inner.use_imm_dates
        )
    }
}

#[pyfunction(name = "build_rate_instrument", text_signature = "(quote, ctx)")]
fn build_rate_instrument_py(quote: &PyRateQuote, ctx: &PyBuildCtx) -> PyResult<PyBuiltInstrument> {
    let inst = build_rate_instrument(&quote.inner, &ctx.inner)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyBuiltInstrument::new(inst))
}

#[pyfunction(name = "build_cds_instrument", text_signature = "(quote, ctx)")]
fn build_cds_instrument_py(quote: &PyCdsQuote, ctx: &PyBuildCtx) -> PyResult<PyBuiltInstrument> {
    let inst = build_cds_instrument(&quote.inner, &ctx.inner)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyBuiltInstrument::new(inst))
}

#[pyfunction(
    name = "build_cds_tranche_instrument",
    text_signature = "(quote, ctx, overrides)"
)]
fn build_cds_tranche_instrument_py(
    quote: &PyCDSTrancheQuote,
    ctx: &PyBuildCtx,
    overrides: &PyCDSTrancheBuildOverrides,
) -> PyResult<PyBuiltInstrument> {
    let inst = build_cds_tranche_instrument(&quote.inner, &ctx.inner, &overrides.inner)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyBuiltInstrument::new(inst))
}

/// Register market builder exports.
pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "market")?;
    module.setattr(
        "__doc__",
        "Market builders (quotes + BuildCtx) to create calibration-ready instruments.",
    )?;

    module.add_class::<PyQuoteId>()?;
    module.add_class::<PyPillar>()?;
    module.add_class::<PyBuildCtx>()?;
    module.add_class::<PyBuiltInstrument>()?;
    module.add_class::<PyRateQuote>()?;
    module.add_class::<PyCdsQuote>()?;
    module.add_class::<PyCDSTrancheQuote>()?;
    module.add_class::<PyCDSTrancheBuildOverrides>()?;
    module.add_function(wrap_pyfunction!(build_rate_instrument_py, &module)?)?;
    module.add_function(wrap_pyfunction!(build_cds_instrument_py, &module)?)?;
    module.add_function(wrap_pyfunction!(build_cds_tranche_instrument_py, &module)?)?;

    let exports = [
        "QuoteId",
        "Pillar",
        "BuildCtx",
        "BuiltInstrument",
        "RateQuote",
        "CdsQuote",
        "CDSTrancheQuote",
        "CDSTrancheBuildOverrides",
        "build_rate_instrument",
        "build_cds_instrument",
        "build_cds_tranche_instrument",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("market", &module)?;
    Ok(exports.to_vec())
}
