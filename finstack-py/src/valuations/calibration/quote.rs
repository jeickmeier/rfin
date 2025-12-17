use crate::core::common::args::{CurrencyArg, DayCountArg};
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::schedule::PyFrequency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use finstack_core::prelude::DateExt;
use finstack_valuations::calibration::v2::domain::quotes::{
    CreditQuote, FutureSpecs, InflationQuote, InstrumentConventions, MarketQuote, RatesQuote,
    VolQuote,
};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, PyRef};

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
    #[pyo3(text_signature = "(cls, maturity, rate, day_count)")]
    fn deposit(
        _cls: &Bound<'_, PyType>,
        maturity: Bound<'_, PyAny>,
        rate: f64,
        day_count: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        let DayCountArg(dc) = DayCountArg::extract_bound(&day_count)?;
        Ok(Self::new(RatesQuote::Deposit {
            maturity: maturity_date,
            rate,
            conventions: InstrumentConventions::default().with_day_count(dc),
        }))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, start, end, rate, day_count)")]
    fn fra(
        _cls: &Bound<'_, PyType>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        rate: f64,
        day_count: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let start_date = py_to_date(&start)?;
        let end_date = py_to_date(&end)?;
        let DayCountArg(dc) = DayCountArg::extract_bound(&day_count)?;
        Ok(Self::new(RatesQuote::FRA {
            start: start_date,
            end: end_date,
            rate,
            conventions: InstrumentConventions::default().with_day_count(dc),
        }))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, expiry, price, specs)")]
    fn future(
        _cls: &Bound<'_, PyType>,
        expiry: Bound<'_, PyAny>,
        price: f64,
        specs: PyRef<PyFutureSpecs>,
    ) -> PyResult<Self> {
        let expiry_date = py_to_date(&expiry)?;
        let period_start = expiry_date;
        let period_end = expiry_date.add_months(specs.inner.delivery_months as i32);
        Ok(Self::new(RatesQuote::Future {
            expiry: expiry_date,
            period_start,
            period_end,
            fixing_date: None,
            price,
            specs: specs.inner.clone(),
            conventions: Default::default(),
        }))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, maturity, rate, fixed_freq, float_freq, fixed_day_count, float_day_count, index)"
    )]
    fn swap(
        _cls: &Bound<'_, PyType>,
        maturity: Bound<'_, PyAny>,
        rate: f64,
        fixed_freq: PyRef<PyFrequency>,
        float_freq: PyRef<PyFrequency>,
        fixed_day_count: Bound<'_, PyAny>,
        float_day_count: Bound<'_, PyAny>,
        index: &str,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        let DayCountArg(fixed_dc) = DayCountArg::extract_bound(&fixed_day_count)?;
        let DayCountArg(float_dc) = DayCountArg::extract_bound(&float_day_count)?;
        Ok(Self::new(RatesQuote::Swap {
            maturity: maturity_date,
            rate,
            is_ois: false,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(fixed_freq.inner)
                .with_day_count(fixed_dc),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(float_freq.inner)
                .with_day_count(float_dc)
                .with_index(index),
        }))
    }

    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "(cls, maturity, primary_index, reference_index, spread_bp, primary_frequency, reference_frequency, primary_day_count, reference_day_count, currency)"
    )]
    fn basis_swap(
        _cls: &Bound<'_, PyType>,
        maturity: Bound<'_, PyAny>,
        primary_index: &str,
        reference_index: &str,
        spread_bp: f64,
        primary_frequency: PyRef<PyFrequency>,
        reference_frequency: PyRef<PyFrequency>,
        primary_day_count: Bound<'_, PyAny>,
        reference_day_count: Bound<'_, PyAny>,
        currency: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        let DayCountArg(primary_dc) = DayCountArg::extract_bound(&primary_day_count)?;
        let DayCountArg(reference_dc) = DayCountArg::extract_bound(&reference_day_count)?;
        let CurrencyArg(ccy) = CurrencyArg::extract_bound(&currency)?;
        Ok(Self::new(RatesQuote::BasisSwap {
            maturity: maturity_date,
            spread_bp,
            conventions: InstrumentConventions::default().with_currency(ccy),
            primary_leg_conventions: InstrumentConventions::default()
                .with_index(primary_index)
                .with_payment_frequency(primary_frequency.inner)
                .with_day_count(primary_dc),
            reference_leg_conventions: InstrumentConventions::default()
                .with_index(reference_index)
                .with_payment_frequency(reference_frequency.inner)
                .with_day_count(reference_dc),
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
    module.add_class::<PyRatesQuote>()?;
    module.add_class::<PyCreditQuote>()?;
    module.add_class::<PyVolQuote>()?;
    module.add_class::<PyInflationQuote>()?;
    module.add_class::<PyMarketQuote>()?;
    Ok(vec![
        "FutureSpecs",
        "RatesQuote",
        "CreditQuote",
        "VolQuote",
        "InflationQuote",
        "MarketQuote",
    ])
}
