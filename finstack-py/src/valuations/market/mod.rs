//! Market builder bindings (quote -> instrument).
//!
//! This module exposes a thin Python surface over `finstack_valuations::market`:
//! - Quote schemas (rates / CDS / CDS tranche)
//! - Build context (`BuildCtx`)
//! - Builder functions that produce an instrument ready for pricing

pub(crate) mod conventions;

use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::dates::{PyBusinessDayConvention, PyDayCount, PyTenor};
use crate::core::market_data::context::PyMarketContext;
use crate::valuations::common::parameters::PyOptionType;
use crate::valuations::common::PyInstrumentType;
use crate::valuations::market::conventions::PyCdsConventionKey;
use finstack_core::currency::Currency;
use finstack_core::dates::Tenor;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use finstack_valuations::instruments::OptionType;
use finstack_valuations::market::conventions::ids::{
    BondConventionId, FxConventionId, FxOptionConventionId, IndexId, InflationSwapConventionId,
    IrFutureContractId, OptionConventionId, SwaptionConventionId, XccyConventionId,
};
use finstack_valuations::market::quotes::bond::BondQuote;
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::cds_tranche::CDSTrancheQuote;
use finstack_valuations::market::quotes::fx::FxQuote;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::inflation::InflationQuote;
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote;
use finstack_valuations::market::quotes::vol::VolQuote;
use finstack_valuations::market::quotes::xccy::XccyQuote;
use finstack_valuations::market::{
    build_bond_instrument, build_cds_instrument, build_cds_tranche_instrument, build_fx_instrument,
    build_rate_instrument, build_xccy_instrument, BuildCtx, CDSTrancheBuildOverrides,
};
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
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
#[pyclass(
    module = "finstack.valuations.market",
    name = "QuoteId",
    frozen,
    from_py_object
)]
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
#[pyclass(
    module = "finstack.valuations.market",
    name = "Pillar",
    frozen,
    from_py_object
)]
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
#[pyclass(
    module = "finstack.valuations.market",
    name = "BuildCtx",
    from_py_object
)]
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
#[pyclass(module = "finstack.valuations.market", name = "BuiltInstrument")]
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
#[pyclass(
    module = "finstack.valuations.market",
    name = "RateQuote",
    frozen,
    from_py_object
)]
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
#[pyclass(
    module = "finstack.valuations.market",
    name = "CdsQuote",
    frozen,
    from_py_object
)]
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
    frozen,
    from_py_object
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
    name = "CDSTrancheBuildOverrides",
    from_py_object
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

fn parse_currency(obj: &Bound<'_, PyAny>) -> PyResult<Currency> {
    if let Ok(py_ccy) = obj.extract::<PyRef<PyCurrency>>() {
        Ok(py_ccy.inner)
    } else if let Ok(s) = obj.extract::<String>() {
        s.parse()
            .map_err(|e| PyValueError::new_err(format!("Invalid currency: {e}")))
    } else {
        Err(PyTypeError::new_err(
            "Expected Currency or string like 'USD'",
        ))
    }
}

fn parse_option_type(obj: &Bound<'_, PyAny>) -> PyResult<OptionType> {
    if let Ok(py_ot) = obj.extract::<PyRef<PyOptionType>>() {
        Ok(py_ot.inner)
    } else if let Ok(s) = obj.extract::<&str>() {
        s.parse().map_err(|e: String| PyValueError::new_err(e))
    } else {
        Err(PyTypeError::new_err(
            "Expected OptionType or string like 'call'/'put'",
        ))
    }
}

// ---------------------------------------------------------------------------
// BondQuote
// ---------------------------------------------------------------------------

/// Bond instrument quote schema.
#[pyclass(
    module = "finstack.valuations.market",
    name = "BondQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBondQuote {
    pub(crate) inner: BondQuote,
}

impl PyBondQuote {
    pub(crate) fn new(inner: BondQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBondQuote {
    #[classmethod]
    #[pyo3(
        signature = (id, currency, issue_date, maturity, coupon_rate, convention, clean_price_pct),
        text_signature = "(cls, id, currency, issue_date, maturity, coupon_rate, convention, clean_price_pct)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn fixed_rate_bullet_clean_price(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        currency: &Bound<'_, PyAny>,
        issue_date: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        coupon_rate: f64,
        convention: &str,
        clean_price_pct: f64,
    ) -> PyResult<Self> {
        Ok(Self::new(BondQuote::FixedRateBulletCleanPrice {
            id: parse_quote_id(id)?,
            currency: parse_currency(currency)?,
            issue_date: py_to_date(issue_date)?,
            maturity: py_to_date(maturity)?,
            coupon_rate,
            convention: BondConventionId::new(convention),
            clean_price_pct,
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (id, currency, issue_date, maturity, coupon_rate, convention, z_spread),
        text_signature = "(cls, id, currency, issue_date, maturity, coupon_rate, convention, z_spread)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn fixed_rate_bullet_z_spread(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        currency: &Bound<'_, PyAny>,
        issue_date: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        coupon_rate: f64,
        convention: &str,
        z_spread: f64,
    ) -> PyResult<Self> {
        Ok(Self::new(BondQuote::FixedRateBulletZSpread {
            id: parse_quote_id(id)?,
            currency: parse_currency(currency)?,
            issue_date: py_to_date(issue_date)?,
            maturity: py_to_date(maturity)?,
            coupon_rate,
            convention: BondConventionId::new(convention),
            z_spread,
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (id, currency, issue_date, maturity, coupon_rate, convention, oas),
        text_signature = "(cls, id, currency, issue_date, maturity, coupon_rate, convention, oas)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn fixed_rate_bullet_oas(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        currency: &Bound<'_, PyAny>,
        issue_date: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        coupon_rate: f64,
        convention: &str,
        oas: f64,
    ) -> PyResult<Self> {
        Ok(Self::new(BondQuote::FixedRateBulletOas {
            id: parse_quote_id(id)?,
            currency: parse_currency(currency)?,
            issue_date: py_to_date(issue_date)?,
            maturity: py_to_date(maturity)?,
            coupon_rate,
            convention: BondConventionId::new(convention),
            oas,
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (id, currency, issue_date, maturity, coupon_rate, convention, ytm),
        text_signature = "(cls, id, currency, issue_date, maturity, coupon_rate, convention, ytm)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn fixed_rate_bullet_ytm(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        currency: &Bound<'_, PyAny>,
        issue_date: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        coupon_rate: f64,
        convention: &str,
        ytm: f64,
    ) -> PyResult<Self> {
        Ok(Self::new(BondQuote::FixedRateBulletYtm {
            id: parse_quote_id(id)?,
            currency: parse_currency(currency)?,
            issue_date: py_to_date(issue_date)?,
            maturity: py_to_date(maturity)?,
            coupon_rate,
            convention: BondConventionId::new(convention),
            ytm,
        }))
    }

    #[getter]
    fn id(&self) -> PyQuoteId {
        PyQuoteId::new(self.inner.id().clone())
    }

    fn __repr__(&self) -> String {
        let ty = match &self.inner {
            BondQuote::FixedRateBulletCleanPrice { .. } => "fixed_rate_bullet_clean_price",
            BondQuote::FixedRateBulletZSpread { .. } => "fixed_rate_bullet_z_spread",
            BondQuote::FixedRateBulletOas { .. } => "fixed_rate_bullet_oas",
            BondQuote::FixedRateBulletYtm { .. } => "fixed_rate_bullet_ytm",
        };
        format!("BondQuote(type='{}', id='{}')", ty, self.inner.id())
    }
}

// ---------------------------------------------------------------------------
// InflationQuote
// ---------------------------------------------------------------------------

/// Inflation instrument quote schema.
#[pyclass(
    module = "finstack.valuations.market",
    name = "InflationQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyInflationQuote {
    pub(crate) inner: InflationQuote,
}

impl PyInflationQuote {
    pub(crate) fn new(inner: InflationQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInflationQuote {
    #[classmethod]
    #[pyo3(
        signature = (maturity, rate, index, convention),
        text_signature = "(cls, maturity, rate, index, convention)"
    )]
    fn inflation_swap(
        _cls: &Bound<'_, PyType>,
        maturity: &Bound<'_, PyAny>,
        rate: f64,
        index: &str,
        convention: &str,
    ) -> PyResult<Self> {
        Ok(Self::new(InflationQuote::InflationSwap {
            maturity: py_to_date(maturity)?,
            rate,
            index: index.to_string(),
            convention: InflationSwapConventionId::new(convention),
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (maturity, rate, index, frequency, convention),
        text_signature = "(cls, maturity, rate, index, frequency, convention)"
    )]
    fn yoy_inflation_swap(
        _cls: &Bound<'_, PyType>,
        maturity: &Bound<'_, PyAny>,
        rate: f64,
        index: &str,
        frequency: &Bound<'_, PyAny>,
        convention: &str,
    ) -> PyResult<Self> {
        let freq = if let Ok(t) = frequency.extract::<PyRef<'_, PyTenor>>() {
            t.inner
        } else if let Ok(text) = frequency.extract::<&str>() {
            Tenor::parse(text).map_err(|e| PyValueError::new_err(e.to_string()))?
        } else {
            return Err(PyTypeError::new_err(
                "frequency must be Tenor or string like '1Y'",
            ));
        };
        Ok(Self::new(InflationQuote::YoYInflationSwap {
            maturity: py_to_date(maturity)?,
            rate,
            index: index.to_string(),
            frequency: freq,
            convention: InflationSwapConventionId::new(convention),
        }))
    }

    fn __repr__(&self) -> String {
        let ty = match &self.inner {
            InflationQuote::InflationSwap { .. } => "inflation_swap",
            InflationQuote::YoYInflationSwap { .. } => "yoy_inflation_swap",
        };
        format!("InflationQuote(type='{ty}')")
    }
}

// ---------------------------------------------------------------------------
// VolQuote
// ---------------------------------------------------------------------------

/// Volatility quote for option and swaption surface calibration.
#[pyclass(
    module = "finstack.valuations.market",
    name = "VolQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVolQuote {
    pub(crate) inner: VolQuote,
}

impl PyVolQuote {
    pub(crate) fn new(inner: VolQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolQuote {
    #[classmethod]
    #[pyo3(
        signature = (underlying, expiry, strike, vol, option_type, convention),
        text_signature = "(cls, underlying, expiry, strike, vol, option_type, convention)"
    )]
    fn option_vol(
        _cls: &Bound<'_, PyType>,
        underlying: &str,
        expiry: &Bound<'_, PyAny>,
        strike: f64,
        vol: f64,
        option_type: &Bound<'_, PyAny>,
        convention: &str,
    ) -> PyResult<Self> {
        Ok(Self::new(VolQuote::OptionVol {
            underlying: finstack_core::types::UnderlyingId::new(underlying),
            expiry: py_to_date(expiry)?,
            strike,
            vol,
            option_type: parse_option_type(option_type)?,
            convention: OptionConventionId::new(convention),
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (expiry, maturity, strike, vol, quote_type, convention),
        text_signature = "(cls, expiry, maturity, strike, vol, quote_type, convention)"
    )]
    fn swaption_vol(
        _cls: &Bound<'_, PyType>,
        expiry: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        strike: f64,
        vol: f64,
        quote_type: &str,
        convention: &str,
    ) -> PyResult<Self> {
        Ok(Self::new(VolQuote::SwaptionVol {
            expiry: py_to_date(expiry)?,
            maturity: py_to_date(maturity)?,
            strike,
            vol,
            quote_type: quote_type.to_string(),
            convention: SwaptionConventionId::new(convention),
        }))
    }

    fn __repr__(&self) -> String {
        let ty = match &self.inner {
            VolQuote::OptionVol { .. } => "option_vol",
            VolQuote::SwaptionVol { .. } => "swaption_vol",
        };
        format!("VolQuote(type='{ty}')")
    }
}

// ---------------------------------------------------------------------------
// FxQuote
// ---------------------------------------------------------------------------

/// Market quote for FX instruments.
#[pyclass(
    module = "finstack.valuations.market",
    name = "FxQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxQuote {
    pub(crate) inner: FxQuote,
}

impl PyFxQuote {
    pub(crate) fn new(inner: FxQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFxQuote {
    #[classmethod]
    #[pyo3(
        signature = (id, convention, pillar, forward_rate),
        text_signature = "(cls, id, convention, pillar, forward_rate)"
    )]
    fn forward_outright(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        convention: &str,
        pillar: &Bound<'_, PyAny>,
        forward_rate: f64,
    ) -> PyResult<Self> {
        Ok(Self::new(FxQuote::ForwardOutright {
            id: parse_quote_id(id)?,
            convention: FxConventionId::new(convention),
            pillar: parse_pillar(pillar)?,
            forward_rate,
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (id, convention, far_pillar, near_rate, far_rate),
        text_signature = "(cls, id, convention, far_pillar, near_rate, far_rate)"
    )]
    fn swap_outright(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        convention: &str,
        far_pillar: &Bound<'_, PyAny>,
        near_rate: f64,
        far_rate: f64,
    ) -> PyResult<Self> {
        Ok(Self::new(FxQuote::SwapOutright {
            id: parse_quote_id(id)?,
            convention: FxConventionId::new(convention),
            far_pillar: parse_pillar(far_pillar)?,
            near_rate,
            far_rate,
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (id, convention, expiry, strike, option_type, vol_surface_id),
        text_signature = "(cls, id, convention, expiry, strike, option_type, vol_surface_id)"
    )]
    fn option_vanilla(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        convention: &str,
        expiry: &Bound<'_, PyAny>,
        strike: f64,
        option_type: &Bound<'_, PyAny>,
        vol_surface_id: &str,
    ) -> PyResult<Self> {
        Ok(Self::new(FxQuote::OptionVanilla {
            id: parse_quote_id(id)?,
            convention: FxOptionConventionId::new(convention),
            expiry: py_to_date(expiry)?,
            strike,
            option_type: parse_option_type(option_type)?,
            vol_surface_id: CurveId::new(vol_surface_id),
        }))
    }

    #[getter]
    fn id(&self) -> PyQuoteId {
        PyQuoteId::new(self.inner.id().clone())
    }

    fn __repr__(&self) -> String {
        let ty = match &self.inner {
            FxQuote::ForwardOutright { .. } => "forward_outright",
            FxQuote::SwapOutright { .. } => "swap_outright",
            FxQuote::OptionVanilla { .. } => "option_vanilla",
        };
        format!("FxQuote(type='{}', id='{}')", ty, self.inner.id())
    }
}

// ---------------------------------------------------------------------------
// XccyQuote
// ---------------------------------------------------------------------------

/// Market quote for cross-currency basis swap instruments.
#[pyclass(
    module = "finstack.valuations.market",
    name = "XccyQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyXccyQuote {
    pub(crate) inner: XccyQuote,
}

impl PyXccyQuote {
    pub(crate) fn new(inner: XccyQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyXccyQuote {
    #[classmethod]
    #[pyo3(
        signature = (id, convention, far_pillar, basis_spread_bp, *, spot_fx=None),
        text_signature = "(cls, id, convention, far_pillar, basis_spread_bp, *, spot_fx=None)"
    )]
    fn basis_swap(
        _cls: &Bound<'_, PyType>,
        id: &Bound<'_, PyAny>,
        convention: &str,
        far_pillar: &Bound<'_, PyAny>,
        basis_spread_bp: f64,
        spot_fx: Option<f64>,
    ) -> PyResult<Self> {
        Ok(Self::new(XccyQuote::BasisSwap {
            id: parse_quote_id(id)?,
            convention: XccyConventionId::new(convention),
            far_pillar: parse_pillar(far_pillar)?,
            basis_spread_bp,
            spot_fx,
        }))
    }

    #[getter]
    fn id(&self) -> PyQuoteId {
        PyQuoteId::new(self.inner.id().clone())
    }

    fn __repr__(&self) -> String {
        format!("XccyQuote(id='{}')", self.inner.id())
    }
}

// ---------------------------------------------------------------------------
// MarketQuote (unified enum)
// ---------------------------------------------------------------------------

/// Polymorphic container for all supported market quote types.
#[pyclass(
    module = "finstack.valuations.market",
    name = "MarketQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMarketQuote {
    pub(crate) inner: MarketQuote,
}

impl PyMarketQuote {
    pub(crate) fn new(inner: MarketQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarketQuote {
    #[classmethod]
    #[pyo3(text_signature = "(cls, quote)")]
    fn from_rate(_cls: &Bound<'_, PyType>, quote: &PyRateQuote) -> Self {
        Self::new(MarketQuote::Rates(quote.inner.clone()))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, quote)")]
    fn from_cds(_cls: &Bound<'_, PyType>, quote: &PyCdsQuote) -> Self {
        Self::new(MarketQuote::Cds(quote.inner.clone()))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, quote)")]
    fn from_cds_tranche(_cls: &Bound<'_, PyType>, quote: &PyCDSTrancheQuote) -> Self {
        Self::new(MarketQuote::CDSTranche(quote.inner.clone()))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, quote)")]
    fn from_bond(_cls: &Bound<'_, PyType>, quote: &PyBondQuote) -> Self {
        Self::new(MarketQuote::Bond(quote.inner.clone()))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, quote)")]
    fn from_inflation(_cls: &Bound<'_, PyType>, quote: &PyInflationQuote) -> Self {
        Self::new(MarketQuote::Inflation(quote.inner.clone()))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, quote)")]
    fn from_vol(_cls: &Bound<'_, PyType>, quote: &PyVolQuote) -> Self {
        Self::new(MarketQuote::Vol(quote.inner.clone()))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, quote)")]
    fn from_fx(_cls: &Bound<'_, PyType>, quote: &PyFxQuote) -> Self {
        Self::new(MarketQuote::Fx(quote.inner.clone()))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, quote)")]
    fn from_xccy(_cls: &Bound<'_, PyType>, quote: &PyXccyQuote) -> Self {
        Self::new(MarketQuote::Xccy(quote.inner.clone()))
    }

    fn __repr__(&self) -> String {
        let tag = match &self.inner {
            MarketQuote::Bond(_) => "bond",
            MarketQuote::Rates(_) => "rates",
            MarketQuote::Cds(_) => "cds",
            MarketQuote::CDSTranche(_) => "cds_tranche",
            MarketQuote::Fx(_) => "fx",
            MarketQuote::Inflation(_) => "inflation",
            MarketQuote::Vol(_) => "vol",
            MarketQuote::Xccy(_) => "xccy",
        };
        format!("MarketQuote(class='{tag}')")
    }
}

// ---------------------------------------------------------------------------
// Builder functions
// ---------------------------------------------------------------------------

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

#[pyfunction(
    name = "build_bond_instrument",
    text_signature = "(quote, ctx, *, market=None)"
)]
fn build_bond_instrument_py(
    quote: &PyBondQuote,
    ctx: &PyBuildCtx,
    market: Option<&PyMarketContext>,
) -> PyResult<PyBuiltInstrument> {
    let inst = build_bond_instrument(&quote.inner, &ctx.inner, market.map(|m| &m.inner))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyBuiltInstrument::new(inst))
}

#[pyfunction(name = "build_fx_instrument", text_signature = "(quote, ctx)")]
fn build_fx_instrument_py(quote: &PyFxQuote, ctx: &PyBuildCtx) -> PyResult<PyBuiltInstrument> {
    let inst = build_fx_instrument(&quote.inner, &ctx.inner)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyBuiltInstrument::new(inst))
}

#[pyfunction(name = "build_xccy_instrument", text_signature = "(quote, ctx)")]
fn build_xccy_instrument_py(quote: &PyXccyQuote, ctx: &PyBuildCtx) -> PyResult<PyBuiltInstrument> {
    let inst = build_xccy_instrument(&quote.inner, &ctx.inner)
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
    module.add_class::<PyBondQuote>()?;
    module.add_class::<PyInflationQuote>()?;
    module.add_class::<PyVolQuote>()?;
    module.add_class::<PyFxQuote>()?;
    module.add_class::<PyXccyQuote>()?;
    module.add_class::<PyMarketQuote>()?;
    module.add_function(wrap_pyfunction!(build_rate_instrument_py, &module)?)?;
    module.add_function(wrap_pyfunction!(build_cds_instrument_py, &module)?)?;
    module.add_function(wrap_pyfunction!(build_cds_tranche_instrument_py, &module)?)?;
    module.add_function(wrap_pyfunction!(build_bond_instrument_py, &module)?)?;
    module.add_function(wrap_pyfunction!(build_fx_instrument_py, &module)?)?;
    module.add_function(wrap_pyfunction!(build_xccy_instrument_py, &module)?)?;

    let exports = [
        "QuoteId",
        "Pillar",
        "BuildCtx",
        "BuiltInstrument",
        "RateQuote",
        "CdsQuote",
        "CDSTrancheQuote",
        "CDSTrancheBuildOverrides",
        "BondQuote",
        "InflationQuote",
        "VolQuote",
        "FxQuote",
        "XccyQuote",
        "MarketQuote",
        "build_rate_instrument",
        "build_cds_instrument",
        "build_cds_tranche_instrument",
        "build_bond_instrument",
        "build_fx_instrument",
        "build_xccy_instrument",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("market", &module)?;
    Ok(exports.to_vec())
}
