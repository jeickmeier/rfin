use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::ndf::{Ndf, NdfFixingSource, NdfQuoteConvention};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Non-Deliverable Forward (NDF) instrument.
///
/// Represents a cash-settled forward contract on a restricted currency pair.
/// The position is long base currency (restricted) and short settlement currency.
///
/// Pricing Modes
/// -------------
///
/// **Pre-Fixing** (fixing_rate = None)
///     Forward rate is estimated via covered interest rate parity or fallback:
///     PV = notional × (F_market - contract_rate) × DF_settlement(T)
///
/// **Post-Fixing** (fixing_rate = Some)
///     Uses the observed fixing rate:
///     PV = notional × (fixing_rate - contract_rate) × DF_settlement(T)
///
/// Examples
/// --------
/// Create a 3-month USD/CNY NDF::
///
///     from finstack import Money, Date
///     from finstack.valuations.instruments import Ndf
///
///     ndf = (
///         Ndf.builder("USDCNY-NDF-3M")
///         .base_currency("CNY")
///         .settlement_currency("USD")
///         .fixing_date(Date(2025, 3, 13))
///         .maturity_date(Date(2025, 3, 15))
///         .notional(Money.from_code(10_000_000, "CNY"))
///         .contract_rate(7.25)
///         .settlement_curve("USD-OIS")
///         .quote_convention("base_per_settlement")
///         .fixing_source_enum("CNHFIX")
///         .build()
///     )
///
/// See Also
/// --------
/// FxForward : Deliverable FX forward
/// FxSwap : FX swap
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "Ndf",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyNdf {
    pub(crate) inner: Arc<Ndf>,
}

impl PyNdf {
    pub(crate) fn new(inner: Ndf) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "NdfBuilder",
    unsendable
)]
pub struct PyNdfBuilder {
    instrument_id: InstrumentId,
    base_currency: Option<Currency>,
    settlement_currency: Option<Currency>,
    fixing_date: Option<time::Date>,
    maturity_date: Option<time::Date>,
    notional: Option<Money>,
    contract_rate: Option<f64>,
    domestic_discount_curve_id: Option<CurveId>,
    foreign_discount_curve_id: Option<CurveId>,
    fixing_rate: Option<f64>,
    fixing_source_enum: Option<NdfFixingSource>,
    quote_convention: Option<NdfQuoteConvention>,
    spot_rate_override: Option<f64>,
    base_calendar_id: Option<String>,
    quote_calendar_id: Option<String>,
}

impl PyNdfBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            base_currency: None,
            settlement_currency: None,
            fixing_date: None,
            maturity_date: None,
            notional: None,
            contract_rate: None,
            domestic_discount_curve_id: None,
            foreign_discount_curve_id: None,
            fixing_rate: None,
            fixing_source_enum: None,
            quote_convention: None,
            spot_rate_override: None,
            base_calendar_id: None,
            quote_calendar_id: None,
        }
    }

    fn validate_and_build(&self) -> PyResult<Ndf> {
        use crate::errors::core_to_py;

        let base_currency = self
            .base_currency
            .ok_or_else(|| PyValueError::new_err("base_currency is required"))?;

        let settlement_currency = self
            .settlement_currency
            .ok_or_else(|| PyValueError::new_err("settlement_currency is required"))?;

        let fixing_date = self
            .fixing_date
            .ok_or_else(|| PyValueError::new_err("fixing_date is required"))?;

        let maturity_date = self
            .maturity_date
            .ok_or_else(|| PyValueError::new_err("maturity_date is required"))?;

        if maturity_date <= fixing_date {
            return Err(PyValueError::new_err(
                "maturity_date must be after fixing_date",
            ));
        }

        let notional = self
            .notional
            .ok_or_else(|| PyValueError::new_err("notional is required"))?;

        if notional.currency() != base_currency {
            return Err(PyValueError::new_err(format!(
                "notional currency ({}) must match base_currency ({})",
                notional.currency(),
                base_currency
            )));
        }

        let contract_rate = self
            .contract_rate
            .ok_or_else(|| PyValueError::new_err("contract_rate is required"))?;

        if contract_rate <= 0.0 {
            return Err(PyValueError::new_err("contract_rate must be positive"));
        }

        let settlement_curve_id = self
            .domestic_discount_curve_id
            .clone()
            .ok_or_else(|| PyValueError::new_err("settlement_curve_id is required"))?;

        let quote_convention = self.quote_convention.ok_or_else(|| {
            PyValueError::new_err("quote_convention is required (e.g. 'base_per_settlement')")
        })?;

        Ndf::builder()
            .id(self.instrument_id.clone())
            .base_currency(base_currency)
            .settlement_currency(settlement_currency)
            .fixing_date(fixing_date)
            .maturity(maturity_date)
            .notional(notional)
            .contract_rate(contract_rate)
            .domestic_discount_curve_id(settlement_curve_id)
            .quote_convention(quote_convention)
            .foreign_discount_curve_id_opt(self.foreign_discount_curve_id.clone())
            .fixing_rate_opt(self.fixing_rate)
            .fixing_source_enum_opt(self.fixing_source_enum)
            .spot_rate_override_opt(self.spot_rate_override)
            .base_calendar_id_opt(self.base_calendar_id.clone())
            .quote_calendar_id_opt(self.quote_calendar_id.clone())
            .attributes(Attributes::new())
            .build()
            .map_err(core_to_py)
    }
}

#[pymethods]
impl PyNdfBuilder {
    fn base_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::core::currency::extract_currency;
        slf.base_currency = Some(extract_currency(&ccy)?);
        Ok(slf)
    }

    fn settlement_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::core::currency::extract_currency;
        slf.settlement_currency = Some(extract_currency(&ccy)?);
        Ok(slf)
    }

    fn fixing_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.fixing_date = Some(py_to_date(&date)?);
        Ok(slf)
    }

    fn maturity_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity_date = Some(py_to_date(&date)?);
        Ok(slf)
    }

    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.notional = Some(extract_money(&notional)?);
        Ok(slf)
    }

    fn contract_rate<'py>(mut slf: PyRefMut<'py, Self>, rate: f64) -> PyRefMut<'py, Self> {
        slf.contract_rate = Some(rate);
        slf
    }

    fn settlement_curve<'py>(mut slf: PyRefMut<'py, Self>, curve_id: &str) -> PyRefMut<'py, Self> {
        slf.domestic_discount_curve_id = Some(CurveId::new(curve_id));
        slf
    }

    fn foreign_curve<'py>(mut slf: PyRefMut<'py, Self>, curve_id: &str) -> PyRefMut<'py, Self> {
        slf.foreign_discount_curve_id = Some(CurveId::new(curve_id));
        slf
    }

    fn fixing_rate<'py>(mut slf: PyRefMut<'py, Self>, rate: f64) -> PyRefMut<'py, Self> {
        slf.fixing_rate = Some(rate);
        slf
    }

    fn fixing_source_enum<'py>(
        mut slf: PyRefMut<'py, Self>,
        source: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let parsed = source
            .parse::<NdfFixingSource>()
            .map_err(PyValueError::new_err)?;
        slf.fixing_source_enum = Some(parsed);
        Ok(slf)
    }

    fn quote_convention<'py>(
        mut slf: PyRefMut<'py, Self>,
        convention: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let parsed = convention
            .parse::<NdfQuoteConvention>()
            .map_err(PyValueError::new_err)?;
        slf.quote_convention = Some(parsed);
        Ok(slf)
    }

    fn spot_rate_override<'py>(mut slf: PyRefMut<'py, Self>, rate: f64) -> PyRefMut<'py, Self> {
        slf.spot_rate_override = Some(rate);
        slf
    }

    fn base_calendar<'py>(mut slf: PyRefMut<'py, Self>, calendar_id: &str) -> PyRefMut<'py, Self> {
        slf.base_calendar_id = Some(calendar_id.to_string());
        slf
    }

    fn quote_calendar<'py>(mut slf: PyRefMut<'py, Self>, calendar_id: &str) -> PyRefMut<'py, Self> {
        slf.quote_calendar_id = Some(calendar_id.to_string());
        slf
    }

    fn build(slf: PyRef<'_, Self>) -> PyResult<PyNdf> {
        let inner = slf.validate_and_build()?;
        Ok(PyNdf::new(inner))
    }

    fn __repr__(&self) -> String {
        format!("NdfBuilder(id='{}')", self.instrument_id)
    }
}

#[pymethods]
impl PyNdf {
    /// Create a builder for a non-deliverable forward contract.
    ///
    /// Parameters
    /// ----------
    /// instrument_id : str
    ///     Unique instrument identifier (e.g., "USDCNY-NDF-3M")
    ///
    /// Returns
    /// -------
    /// NdfBuilder
    ///     Builder instance for fluent configuration
    #[classmethod]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyNdfBuilder {
        PyNdfBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument type.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Ndf)
    }

    /// Base currency (restricted/non-deliverable, numerator).
    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    /// Settlement currency (convertible, denominator).
    #[getter]
    fn settlement_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.settlement_currency)
    }

    /// Fixing date (rate observation date).
    #[getter]
    fn fixing_date<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.fixing_date)
    }

    /// Maturity/settlement date.
    #[getter]
    fn maturity_date<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Notional amount in base currency.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Contract forward rate (base per settlement).
    #[getter]
    fn contract_rate(&self) -> f64 {
        self.inner.contract_rate
    }

    /// Observed fixing rate (if set).
    #[getter]
    fn fixing_rate(&self) -> Option<f64> {
        self.inner.fixing_rate
    }

    /// Fixing source/benchmark (e.g., "CNHFIX", "RBI", "PTAX").
    #[getter]
    fn fixing_source_enum(&self) -> Option<String> {
        self.inner
            .fixing_source_enum
            .as_ref()
            .map(|source| source.to_string())
    }

    /// Check if NDF is in post-fixing mode (fixing rate is set).
    ///
    /// Returns
    /// -------
    /// bool
    ///     True if fixing rate has been observed, False otherwise
    fn is_fixed(&self) -> bool {
        self.inner.is_fixed()
    }

    /// Calculate present value in settlement currency.
    ///
    /// Uses mark-to-market if fixing_rate is set (post-fixing mode), otherwise
    /// estimates forward rate via covered interest rate parity (pre-fixing mode).
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data including discount curves and FX rates
    /// as_of : Date
    ///     Valuation date
    ///
    /// Returns
    /// -------
    /// Money
    ///     Present value in settlement currency
    fn value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.value(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn __repr__(&self) -> String {
        let fixed_status = if self.inner.is_fixed() {
            "post-fixing"
        } else {
            "pre-fixing"
        };
        format!(
            "Ndf(id='{}', {}/{}, contract_rate={}, maturity={}, {})",
            self.inner.id,
            self.inner.base_currency,
            self.inner.settlement_currency,
            self.inner.contract_rate,
            self.inner.maturity,
            fixed_status
        )
    }
}

impl fmt::Display for PyNdf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Ndf({}, {}/{})",
            self.inner.id, self.inner.base_currency, self.inner.settlement_currency
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyNdf>()?;
    module.add_class::<PyNdfBuilder>()?;
    Ok(vec!["Ndf", "NdfBuilder"])
}
