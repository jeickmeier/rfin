use crate::core::common::args::{BusinessDayConventionArg, CurrencyArg, DayCountArg};
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::schedule::PyFrequency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::dates::PyTenor;
use finstack_core::prelude::DateExt;
use finstack_core::types::IndexId;
use finstack_valuations::calibration::quotes::{
    CreditQuote, FutureSpecs, InflationQuote, InstrumentConventions, MarketQuote, RatesQuote,
    VolQuote,
};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, PyRef};
use std::str::FromStr;

fn extract_tenor_opt(obj: Option<Bound<'_, PyAny>>) -> PyResult<Option<finstack_core::dates::Tenor>> {
    let Some(obj) = obj else {
        return Ok(None);
    };
    if let Ok(tenor) = obj.extract::<PyRef<'_, PyTenor>>() {
        return Ok(Some(tenor.inner));
    }
    if let Ok(text) = obj.extract::<&str>() {
        return finstack_core::dates::Tenor::parse(text).map(Some).map_err(crate::errors::core_to_py);
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "Expected Tenor or tenor string like '3M'",
    ))
}

fn extract_index_opt(obj: Option<Bound<'_, PyAny>>) -> PyResult<Option<IndexId>> {
    let Some(obj) = obj else {
        return Ok(None);
    };
    if let Ok(text) = obj.extract::<&str>() {
        // `IndexId` parsing is infallible (string newtype); keep this explicit to avoid
        // surfacing hidden defaults while also not forcing error handling in Python.
        return Ok(Some(IndexId::from_str(text).unwrap()));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "Expected index id string like 'USD-SOFR-3M'",
    ))
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "InstrumentConventions",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyInstrumentConventions {
    pub(crate) inner: InstrumentConventions,
}

impl PyInstrumentConventions {
    pub(crate) fn new(inner: InstrumentConventions) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInstrumentConventions {
    #[new]
    #[pyo3(
        signature = (
            settlement_days=None,
            payment_delay_days=None,
            reset_lag=None,
            calendar_id=None,
            fixing_calendar_id=None,
            payment_calendar_id=None,
            reset_frequency=None,
            payment_frequency=None,
            business_day_convention=None,
            day_count=None,
            currency=None,
            index=None,
            recovery_rate=None
        ),
        text_signature = "(settlement_days=None, payment_delay_days=None, reset_lag=None, calendar_id=None, fixing_calendar_id=None, payment_calendar_id=None, reset_frequency=None, payment_frequency=None, business_day_convention=None, day_count=None, currency=None, index=None, recovery_rate=None)"
    )]
    fn ctor(
        settlement_days: Option<i32>,
        payment_delay_days: Option<i32>,
        reset_lag: Option<i32>,
        calendar_id: Option<String>,
        fixing_calendar_id: Option<String>,
        payment_calendar_id: Option<String>,
        reset_frequency: Option<Bound<'_, PyAny>>,
        payment_frequency: Option<Bound<'_, PyAny>>,
        business_day_convention: Option<Bound<'_, PyAny>>,
        day_count: Option<Bound<'_, PyAny>>,
        currency: Option<Bound<'_, PyAny>>,
        index: Option<Bound<'_, PyAny>>,
        recovery_rate: Option<f64>,
    ) -> PyResult<Self> {
        let mut inner = InstrumentConventions::default();
        inner.settlement_days = settlement_days;
        inner.payment_delay_days = payment_delay_days;
        inner.reset_lag = reset_lag;
        inner.calendar_id = calendar_id;
        inner.fixing_calendar_id = fixing_calendar_id;
        inner.payment_calendar_id = payment_calendar_id;
        inner.reset_frequency = extract_tenor_opt(reset_frequency)?;
        inner.payment_frequency = extract_tenor_opt(payment_frequency)?;
        if let Some(bdc) = business_day_convention {
            let BusinessDayConventionArg(conv) = BusinessDayConventionArg::extract_bound(&bdc)?;
            inner.business_day_convention = Some(conv);
        }
        if let Some(dc) = day_count {
            let DayCountArg(dc) = DayCountArg::extract_bound(&dc)?;
            inner.day_count = Some(dc);
        }
        if let Some(ccy) = currency {
            let CurrencyArg(ccy) = CurrencyArg::extract_bound(&ccy)?;
            inner.currency = Some(ccy);
        }
        inner.index = extract_index_opt(index)?;
        inner.recovery_rate = recovery_rate;
        Ok(Self::new(inner))
    }

    #[getter]
    fn settlement_days(&self) -> Option<i32> {
        self.inner.settlement_days
    }
    #[getter]
    fn payment_delay_days(&self) -> Option<i32> {
        self.inner.payment_delay_days
    }
    #[getter]
    fn reset_lag(&self) -> Option<i32> {
        self.inner.reset_lag
    }
    #[getter]
    fn calendar_id(&self) -> Option<String> {
        self.inner.calendar_id.clone()
    }
    #[getter]
    fn fixing_calendar_id(&self) -> Option<String> {
        self.inner.fixing_calendar_id.clone()
    }
    #[getter]
    fn payment_calendar_id(&self) -> Option<String> {
        self.inner.payment_calendar_id.clone()
    }
    #[getter]
    fn reset_frequency(&self) -> Option<PyTenor> {
        self.inner.reset_frequency.map(PyTenor::new)
    }
    #[getter]
    fn payment_frequency(&self) -> Option<PyTenor> {
        self.inner.payment_frequency.map(PyTenor::new)
    }
    #[getter]
    fn business_day_convention(&self) -> Option<crate::core::dates::calendar::PyBusinessDayConvention> {
        self.inner
            .business_day_convention
            .map(crate::core::dates::calendar::PyBusinessDayConvention::new)
    }
    #[getter]
    fn day_count(&self) -> Option<PyDayCount> {
        self.inner.day_count.map(PyDayCount::new)
    }
    #[getter]
    fn currency(&self) -> Option<crate::core::currency::PyCurrency> {
        self.inner.currency.map(crate::core::currency::PyCurrency::new)
    }
    #[getter]
    fn index(&self) -> Option<String> {
        self.inner.index.as_ref().map(|idx| idx.to_string())
    }
    #[getter]
    fn recovery_rate(&self) -> Option<f64> {
        self.inner.recovery_rate
    }

    fn __repr__(&self) -> String {
        format!("InstrumentConventions({:?})", self.inner)
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "FutureSpecs",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyFutureSpecs {
    pub(crate) inner: FutureSpecs,
}

impl PyFutureSpecs {
    pub(crate) fn new(inner: FutureSpecs) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFutureSpecs {
    #[new]
    #[pyo3(
        text_signature = "(*, multiplier=1_000_000.0, face_value=1_000_000.0, delivery_months=3, day_count='ACT/360', convexity_adjustment=None)"
    )]
    fn ctor(
        multiplier: Option<f64>,
        face_value: Option<f64>,
        delivery_months: Option<u8>,
        day_count: Option<Bound<'_, PyAny>>,
        convexity_adjustment: Option<Option<f64>>,
    ) -> PyResult<Self> {
        let mut specs = FutureSpecs::default();
        if let Some(value) = multiplier {
            specs.multiplier = value;
        }
        if let Some(value) = face_value {
            specs.face_value = value;
        }
        if let Some(value) = delivery_months {
            specs.delivery_months = value;
        }
        if let Some(dc) = day_count {
            let DayCountArg(inner_dc) = DayCountArg::extract_bound(&dc)?;
            specs.day_count = inner_dc;
        }
        if let Some(adjust) = convexity_adjustment {
            specs.convexity_adjustment = adjust;
        }
        Ok(Self::new(specs))
    }

    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    #[getter]
    fn face_value(&self) -> f64 {
        self.inner.face_value
    }

    #[getter]
    fn delivery_months(&self) -> u8 {
        self.inner.delivery_months
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    #[getter]
    fn convexity_adjustment(&self) -> Option<f64> {
        self.inner.convexity_adjustment
    }

    fn __repr__(&self) -> String {
        format!(
            "FutureSpecs(multiplier={}, face_value={}, delivery_months={}, day_count={:?}, convexity_adjustment={:?})",
            self.inner.multiplier,
            self.inner.face_value,
            self.inner.delivery_months,
            self.inner.day_count,
            self.inner.convexity_adjustment
        )
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "RatesQuote",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyRatesQuote {
    pub(crate) inner: RatesQuote,
}

impl PyRatesQuote {
    pub(crate) fn new(inner: RatesQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRatesQuote {
    #[classmethod]
    #[pyo3(
        signature = (expiry, price, specs, *, fixing_date=None, conventions=None),
        text_signature = "(cls, expiry, price, specs, *, fixing_date=None, conventions=None)"
    )]
    fn future(
        _cls: &Bound<'_, PyType>,
        expiry: Bound<'_, PyAny>,
        price: f64,
        specs: PyRef<PyFutureSpecs>,
        fixing_date: Option<Bound<'_, PyAny>>,
        conventions: Option<PyRef<PyInstrumentConventions>>,
    ) -> PyResult<Self> {
        let expiry_date = py_to_date(&expiry)?;
        let period_start = expiry_date;
        let period_end = expiry_date.add_months(specs.inner.delivery_months as i32);
        let fixing_date = match fixing_date {
            None => None,
            Some(v) => Some(py_to_date(&v)?),
        };
        Ok(Self::new(RatesQuote::Future {
            expiry: expiry_date,
            period_start,
            period_end,
            fixing_date,
            price,
            specs: specs.inner.clone(),
            conventions: conventions.map(|c| c.inner.clone()).unwrap_or_default(),
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (maturity, rate, *, conventions),
        text_signature = "(cls, maturity, rate, *, conventions)"
    )]
    fn deposit(
        _cls: &Bound<'_, PyType>,
        maturity: Bound<'_, PyAny>,
        rate: f64,
        conventions: PyRef<PyInstrumentConventions>,
    ) -> PyResult<Self> {
        if conventions.inner.day_count.is_none() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Deposit quote requires conventions.day_count",
            ));
        }
        let maturity_date = py_to_date(&maturity)?;
        Ok(Self::new(RatesQuote::Deposit {
            maturity: maturity_date,
            rate,
            conventions: conventions.inner.clone(),
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (start, end, rate, *, conventions),
        text_signature = "(cls, start, end, rate, *, conventions)"
    )]
    fn fra(
        _cls: &Bound<'_, PyType>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        rate: f64,
        conventions: PyRef<PyInstrumentConventions>,
    ) -> PyResult<Self> {
        if conventions.inner.day_count.is_none() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "FRA quote requires conventions.day_count",
            ));
        }
        let start_date = py_to_date(&start)?;
        let end_date = py_to_date(&end)?;
        Ok(Self::new(RatesQuote::FRA {
            start: start_date,
            end: end_date,
            rate,
            conventions: conventions.inner.clone(),
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (
            maturity,
            rate,
            *,
            fixed_leg_conventions,
            float_leg_conventions,
            is_ois = false,
            conventions = None
        ),
        text_signature = "(cls, maturity, rate, *, fixed_leg_conventions, float_leg_conventions, is_ois=False, conventions=None)"
    )]
    fn swap(
        _cls: &Bound<'_, PyType>,
        maturity: Bound<'_, PyAny>,
        rate: f64,
        fixed_leg_conventions: PyRef<PyInstrumentConventions>,
        float_leg_conventions: PyRef<PyInstrumentConventions>,
        is_ois: bool,
        conventions: Option<PyRef<PyInstrumentConventions>>,
    ) -> PyResult<Self> {
        let fixed = &fixed_leg_conventions.inner;
        let float = &float_leg_conventions.inner;
        if fixed.payment_frequency.is_none() || fixed.day_count.is_none() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Swap quote requires fixed_leg_conventions.payment_frequency and fixed_leg_conventions.day_count",
            ));
        }
        if float.payment_frequency.is_none() || float.day_count.is_none() || float.index.is_none() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Swap quote requires float_leg_conventions.payment_frequency, float_leg_conventions.day_count, and float_leg_conventions.index",
            ));
        }

        let maturity_date = py_to_date(&maturity)?;
        Ok(Self::new(RatesQuote::Swap {
            maturity: maturity_date,
            rate,
            is_ois,
            conventions: conventions.map(|c| c.inner.clone()).unwrap_or_default(),
            fixed_leg_conventions: fixed_leg_conventions.inner.clone(),
            float_leg_conventions: float_leg_conventions.inner.clone(),
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (
            maturity,
            spread_bp,
            *,
            primary_leg_conventions,
            reference_leg_conventions,
            conventions
        ),
        text_signature = "(cls, maturity, spread_bp, *, primary_leg_conventions, reference_leg_conventions, conventions)"
    )]
    fn basis_swap(
        _cls: &Bound<'_, PyType>,
        maturity: Bound<'_, PyAny>,
        spread_bp: f64,
        primary_leg_conventions: PyRef<PyInstrumentConventions>,
        reference_leg_conventions: PyRef<PyInstrumentConventions>,
        conventions: PyRef<PyInstrumentConventions>,
    ) -> PyResult<Self> {
        let primary = &primary_leg_conventions.inner;
        let reference = &reference_leg_conventions.inner;

        if primary.payment_frequency.is_none() || primary.day_count.is_none() || primary.index.is_none() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "BasisSwap quote requires primary_leg_conventions.payment_frequency, primary_leg_conventions.day_count, and primary_leg_conventions.index",
            ));
        }
        if reference.payment_frequency.is_none()
            || reference.day_count.is_none()
            || reference.index.is_none()
        {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "BasisSwap quote requires reference_leg_conventions.payment_frequency, reference_leg_conventions.day_count, and reference_leg_conventions.index",
            ));
        }
        let inst = conventions.inner.clone();
        if inst.currency.is_none() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "BasisSwap quote requires conventions.currency",
            ));
        }

        let maturity_date = py_to_date(&maturity)?;
        Ok(Self::new(RatesQuote::BasisSwap {
            maturity: maturity_date,
            spread_bp,
            conventions: inst,
            primary_leg_conventions: primary_leg_conventions.inner.clone(),
            reference_leg_conventions: reference_leg_conventions.inner.clone(),
        }))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            RatesQuote::Deposit { .. } => "deposit",
            RatesQuote::FRA { .. } => "fra",
            RatesQuote::Future { .. } => "future",
            RatesQuote::Swap { .. } => "swap",
            RatesQuote::BasisSwap { .. } => "basis_swap",
        }
    }

    fn to_market_quote(&self) -> PyMarketQuote {
        PyMarketQuote::new(MarketQuote::Rates(self.inner.clone()))
    }

    #[getter]
    fn conventions(&self) -> PyInstrumentConventions {
        let conv = match &self.inner {
            RatesQuote::Deposit { conventions, .. } => conventions,
            RatesQuote::FRA { conventions, .. } => conventions,
            RatesQuote::Future { conventions, .. } => conventions,
            RatesQuote::Swap { conventions, .. } => conventions,
            RatesQuote::BasisSwap { conventions, .. } => conventions,
        };
        PyInstrumentConventions::new(conv.clone())
    }

    #[getter]
    fn is_ois(&self) -> bool {
        match &self.inner {
            RatesQuote::Swap { is_ois, .. } => *is_ois,
            _ => false,
        }
    }

    #[getter]
    fn fixed_leg_conventions(&self) -> Option<PyInstrumentConventions> {
        match &self.inner {
            RatesQuote::Swap {
                fixed_leg_conventions,
                ..
            } => Some(PyInstrumentConventions::new(fixed_leg_conventions.clone())),
            _ => None,
        }
    }

    #[getter]
    fn float_leg_conventions(&self) -> Option<PyInstrumentConventions> {
        match &self.inner {
            RatesQuote::Swap {
                float_leg_conventions,
                ..
            } => Some(PyInstrumentConventions::new(float_leg_conventions.clone())),
            _ => None,
        }
    }

    #[getter]
    fn primary_leg_conventions(&self) -> Option<PyInstrumentConventions> {
        match &self.inner {
            RatesQuote::BasisSwap {
                primary_leg_conventions,
                ..
            } => Some(PyInstrumentConventions::new(primary_leg_conventions.clone())),
            _ => None,
        }
    }

    #[getter]
    fn reference_leg_conventions(&self) -> Option<PyInstrumentConventions> {
        match &self.inner {
            RatesQuote::BasisSwap {
                reference_leg_conventions,
                ..
            } => Some(PyInstrumentConventions::new(
                reference_leg_conventions.clone(),
            )),
            _ => None,
        }
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        match &self.inner {
            RatesQuote::Deposit { maturity, rate, .. } => {
                let maturity_py = date_to_py(py, *maturity)?;
                Ok(format!(
                    "RatesQuote.deposit(maturity={}, rate={:.6})",
                    maturity_py, rate
                ))
            }
            RatesQuote::FRA {
                start, end, rate, ..
            } => {
                let start_py = date_to_py(py, *start)?;
                let end_py = date_to_py(py, *end)?;
                Ok(format!(
                    "RatesQuote.fra(start={}, end={}, rate={:.6})",
                    start_py, end_py, rate
                ))
            }
            RatesQuote::Future { expiry, price, .. } => {
                let expiry_py = date_to_py(py, *expiry)?;
                Ok(format!(
                    "RatesQuote.future(expiry={}, price={:.6})",
                    expiry_py, price
                ))
            }
            RatesQuote::Swap {
                maturity,
                rate,
                float_leg_conventions,
                ..
            } => {
                let maturity_py = date_to_py(py, *maturity)?;
                let index = float_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("");
                Ok(format!(
                    "RatesQuote.swap(maturity={}, rate={:.6}, index='{}')",
                    maturity_py, rate, index
                ))
            }
            RatesQuote::BasisSwap {
                maturity,
                primary_leg_conventions,
                reference_leg_conventions,
                spread_bp,
                ..
            } => {
                let maturity_py = date_to_py(py, *maturity)?;
                let primary_index = primary_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("");
                let reference_index = reference_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("");
                Ok(format!(
                    "RatesQuote.basis_swap(maturity={}, primary_index='{}', reference_index='{}', spread_bp={:.6})",
                    maturity_py, primary_index, reference_index, spread_bp
                ))
            }
        }
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "CreditQuote",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCreditQuote {
    pub(crate) inner: CreditQuote,
}

impl PyCreditQuote {
    pub(crate) fn new(inner: CreditQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCreditQuote {
    #[classmethod]
    #[pyo3(text_signature = "(cls, entity, maturity, spread_bp, recovery_rate, currency)")]
    fn cds(
        _cls: &Bound<'_, PyType>,
        entity: &str,
        maturity: Bound<'_, PyAny>,
        spread_bp: f64,
        recovery_rate: f64,
        currency: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        let CurrencyArg(ccy) = CurrencyArg::extract_bound(&currency)?;
        Ok(Self::new(CreditQuote::CDS {
            entity: entity.to_string(),
            maturity: maturity_date,
            spread_bp,
            recovery_rate,
            currency: ccy,
            conventions: Default::default(),
        }))
    }

    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "(cls, entity, maturity, upfront_pct, running_spread_bp, recovery_rate, currency)"
    )]
    fn cds_upfront(
        _cls: &Bound<'_, PyType>,
        entity: &str,
        maturity: Bound<'_, PyAny>,
        upfront_pct: f64,
        running_spread_bp: f64,
        recovery_rate: f64,
        currency: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        let CurrencyArg(ccy) = CurrencyArg::extract_bound(&currency)?;
        Ok(Self::new(CreditQuote::CDSUpfront {
            entity: entity.to_string(),
            maturity: maturity_date,
            upfront_pct,
            running_spread_bp,
            recovery_rate,
            currency: ccy,
            conventions: Default::default(),
        }))
    }

    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "(cls, index, attachment, detachment, maturity, upfront_pct, running_spread_bp)"
    )]
    fn cds_tranche(
        _cls: &Bound<'_, PyType>,
        index: &str,
        attachment: f64,
        detachment: f64,
        maturity: Bound<'_, PyAny>,
        upfront_pct: f64,
        running_spread_bp: f64,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        Ok(Self::new(CreditQuote::CDSTranche {
            index: index.to_string(),
            attachment,
            detachment,
            maturity: maturity_date,
            upfront_pct,
            running_spread_bp,
            conventions: Default::default(),
        }))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            CreditQuote::CDS { .. } => "cds",
            CreditQuote::CDSUpfront { .. } => "cds_upfront",
            CreditQuote::CDSTranche { .. } => "cds_tranche",
        }
    }

    fn to_market_quote(&self) -> PyMarketQuote {
        PyMarketQuote::new(MarketQuote::Credit(self.inner.clone()))
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            CreditQuote::CDS {
                entity, spread_bp, ..
            } => {
                format!(
                    "CreditQuote.cds(entity='{}', spread_bp={:.6})",
                    entity, spread_bp
                )
            }
            CreditQuote::CDSUpfront {
                entity,
                upfront_pct,
                running_spread_bp,
                ..
            } => format!(
                "CreditQuote.cds_upfront(entity='{}', upfront_pct={:.6}, running_spread_bp={:.6})",
                entity, upfront_pct, running_spread_bp
            ),
            CreditQuote::CDSTranche {
                index,
                attachment,
                detachment,
                ..
            } => format!(
                "CreditQuote.cds_tranche(index='{}', attachment={:.4}, detachment={:.4})",
                index, attachment, detachment
            ),
        }
    }
}

#[pyclass(module = "finstack.valuations.calibration", name = "VolQuote", frozen)]
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
    #[allow(clippy::too_many_arguments)]
    #[pyo3(text_signature = "(cls, underlying, expiry, strike, vol, option_type)")]
    fn option_vol(
        _cls: &Bound<'_, PyType>,
        underlying: &str,
        expiry: Bound<'_, PyAny>,
        strike: f64,
        vol: f64,
        option_type: &str,
    ) -> PyResult<Self> {
        let expiry_date = py_to_date(&expiry)?;
        Ok(Self::new(VolQuote::OptionVol {
            underlying: underlying.to_string().into(),
            expiry: expiry_date,
            strike,
            vol,
            option_type: option_type.to_string(),
            conventions: Default::default(),
        }))
    }

    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(text_signature = "(cls, expiry, tenor, strike, vol, quote_type)")]
    fn swaption_vol(
        _cls: &Bound<'_, PyType>,
        expiry: Bound<'_, PyAny>,
        tenor: Bound<'_, PyAny>,
        strike: f64,
        vol: f64,
        quote_type: &str,
    ) -> PyResult<Self> {
        let expiry_date = py_to_date(&expiry)?;
        let tenor_date = py_to_date(&tenor)?;
        Ok(Self::new(VolQuote::SwaptionVol {
            expiry: expiry_date,
            tenor: tenor_date,
            strike,
            vol,
            quote_type: quote_type.to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            VolQuote::OptionVol { .. } => "option",
            VolQuote::SwaptionVol { .. } => "swaption",
        }
    }

    fn to_market_quote(&self) -> PyMarketQuote {
        PyMarketQuote::new(MarketQuote::Vol(self.inner.clone()))
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            VolQuote::OptionVol {
                underlying, strike, ..
            } => format!(
                "VolQuote.option_vol(underlying='{}', strike={:.6})",
                underlying, strike
            ),
            VolQuote::SwaptionVol {
                strike, quote_type, ..
            } => format!(
                "VolQuote.swaption_vol(strike={:.6}, quote_type='{}')",
                strike, quote_type
            ),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "InflationQuote",
    frozen
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
    #[pyo3(text_signature = "(cls, maturity, rate, index)")]
    fn inflation_swap(
        _cls: &Bound<'_, PyType>,
        maturity: Bound<'_, PyAny>,
        rate: f64,
        index: &str,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        Ok(Self::new(InflationQuote::InflationSwap {
            maturity: maturity_date,
            rate,
            index: index.to_string(),
            conventions: Default::default(),
        }))
    }

    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(text_signature = "(cls, maturity, rate, index, frequency)")]
    fn yoy_inflation_swap(
        _cls: &Bound<'_, PyType>,
        maturity: Bound<'_, PyAny>,
        rate: f64,
        index: &str,
        frequency: PyRef<PyFrequency>,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        Ok(Self::new(InflationQuote::YoYInflationSwap {
            maturity: maturity_date,
            rate,
            index: index.to_string(),
            frequency: frequency.inner,
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            inflation_leg_conventions: Default::default(),
        }))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            InflationQuote::InflationSwap { .. } => "inflation_swap",
            InflationQuote::YoYInflationSwap { .. } => "yoy_inflation_swap",
        }
    }

    fn to_market_quote(&self) -> PyMarketQuote {
        PyMarketQuote::new(MarketQuote::Inflation(self.inner.clone()))
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            InflationQuote::InflationSwap { index, rate, .. } => format!(
                "InflationQuote.inflation_swap(index='{}', rate={:.6})",
                index, rate
            ),
            InflationQuote::YoYInflationSwap { index, rate, .. } => format!(
                "InflationQuote.yoy_inflation_swap(index='{}', rate={:.6})",
                index, rate
            ),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "MarketQuote",
    frozen
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
    fn from_rates(_cls: &Bound<'_, PyType>, quote: PyRef<PyRatesQuote>) -> Self {
        Self::new(MarketQuote::Rates(quote.inner.clone()))
    }

    #[classmethod]
    fn from_credit(_cls: &Bound<'_, PyType>, quote: PyRef<PyCreditQuote>) -> Self {
        Self::new(MarketQuote::Credit(quote.inner.clone()))
    }

    #[classmethod]
    fn from_vol(_cls: &Bound<'_, PyType>, quote: PyRef<PyVolQuote>) -> Self {
        Self::new(MarketQuote::Vol(quote.inner.clone()))
    }

    #[classmethod]
    fn from_inflation(_cls: &Bound<'_, PyType>, quote: PyRef<PyInflationQuote>) -> Self {
        Self::new(MarketQuote::Inflation(quote.inner.clone()))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            MarketQuote::Rates(_) => "rates",
            MarketQuote::Credit(_) => "credit",
            MarketQuote::Vol(_) => "vol",
            MarketQuote::Inflation(_) => "inflation",
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            MarketQuote::Rates(q) => match q {
                RatesQuote::Deposit { .. } => "MarketQuote.from_rates(deposit)".to_string(),
                RatesQuote::FRA { .. } => "MarketQuote.from_rates(fra)".to_string(),
                RatesQuote::Future { .. } => "MarketQuote.from_rates(future)".to_string(),
                RatesQuote::Swap { .. } => "MarketQuote.from_rates(swap)".to_string(),
                RatesQuote::BasisSwap { .. } => "MarketQuote.from_rates(basis_swap)".to_string(),
            },
            MarketQuote::Credit(q) => match q {
                CreditQuote::CDS { .. } => "MarketQuote.from_credit(cds)".to_string(),
                CreditQuote::CDSUpfront { .. } => {
                    "MarketQuote.from_credit(cds_upfront)".to_string()
                }
                CreditQuote::CDSTranche { .. } => {
                    "MarketQuote.from_credit(cds_tranche)".to_string()
                }
            },
            MarketQuote::Vol(q) => match q {
                VolQuote::OptionVol { .. } => "MarketQuote.from_vol(option)".to_string(),
                VolQuote::SwaptionVol { .. } => "MarketQuote.from_vol(swaption)".to_string(),
            },
            MarketQuote::Inflation(q) => match q {
                InflationQuote::InflationSwap { .. } => {
                    "MarketQuote.from_inflation(inflation_swap)".to_string()
                }
                InflationQuote::YoYInflationSwap { .. } => {
                    "MarketQuote.from_inflation(yoy_inflation_swap)".to_string()
                }
            },
        }
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyFutureSpecs>()?;
    module.add_class::<PyInstrumentConventions>()?;
    module.add_class::<PyRatesQuote>()?;
    module.add_class::<PyCreditQuote>()?;
    module.add_class::<PyVolQuote>()?;
    module.add_class::<PyInflationQuote>()?;
    module.add_class::<PyMarketQuote>()?;
    Ok(vec![
        "FutureSpecs",
        "InstrumentConventions",
        "RatesQuote",
        "CreditQuote",
        "VolQuote",
        "InflationQuote",
        "MarketQuote",
    ])
}
