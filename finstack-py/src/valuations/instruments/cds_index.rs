// use crate::errors::core_to_py; // not used directly
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::valuations::common::PyInstrumentType;
use crate::valuations::instruments::cds::normalize_cds_side;
use finstack_core::types::InstrumentId;
use finstack_valuations::instruments::credit_derivatives::cds::{CDSConvention, PayReceive};
use finstack_valuations::instruments::credit_derivatives::cds_index::{
    CDSIndexConstructionParams, CDSIndexParams,
};
use finstack_valuations::instruments::credit_derivatives::cds_index::CDSIndex;
use finstack_valuations::instruments::CreditParams;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use rust_decimal::prelude::ToPrimitive;
use std::fmt;

const STANDARD_RECOVERY_SENIOR: f64 = 0.40;

/// CDS index instrument binding exposing a simplified constructor.
///
/// Examples:
///     >>> itraxx = CDSIndex.create(
///     ...     "itraxx_main",
///     ...     "iTraxx Europe",
///     ...     38,
///     ...     1,
///     ...     Money("EUR", 10_000_000),
///     ...     100.0,
///     ...     date(2024, 3, 20),
///     ...     date(2029, 3, 20),
///     ...     "eur_discount",
///     ...     "itraxx_credit"
///     ... )
///     >>> itraxx.fixed_coupon_bp
///     100.0
#[pyclass(module = "finstack.valuations.instruments", name = "CDSIndex", frozen)]
#[derive(Clone, Debug)]
pub struct PyCdsIndex {
    pub(crate) inner: CDSIndex,
}

impl PyCdsIndex {
    pub(crate) fn new(inner: CDSIndex) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCdsIndex {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            index_name,
            series,
            version,
            notional,
            fixed_coupon_bp,
            start_date,
            maturity,
            discount_curve,
            credit_curve,
            *,
            side="pay_protection",
            recovery_rate=None,
            index_factor=None
        ),
        text_signature = "(cls, instrument_id, index_name, series, version, notional, fixed_coupon_bp, start_date, maturity, discount_curve, credit_curve, /, *, side='pay_protection', recovery_rate=None, index_factor=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a CDS index instrument with standard ISDA conventions.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     index_name: Name of the CDS index family (e.g., ``"iTraxx"``).
    ///     series: Index series number.
    ///     version: Index version.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     fixed_coupon_bp: Fixed coupon in basis points.
    ///     start_date: Start date for premium payments.
    ///     maturity: Maturity date of the index swap.
    ///     discount_curve: Discount curve identifier.
    ///     credit_curve: Credit curve identifier for the portfolio.
    ///     side: Optional side label (``"pay_protection"`` or ``"receive_protection"``).
    ///     recovery_rate: Optional recovery rate across constituents.
    ///     index_factor: Optional outstanding notional factor.
    ///
    /// Returns:
    ///     CDSIndex: Configured CDS index instrument.
    ///
    /// Raises:
    ///     ValueError: If inputs cannot be parsed or recovery rate is invalid.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        index_name: &str,
        series: u16,
        version: u16,
        notional: Bound<'_, PyAny>,
        fixed_coupon_bp: f64,
        start_date: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        credit_curve: Bound<'_, PyAny>,
        side: Option<&str>,
        recovery_rate: Option<f64>,
        index_factor: Option<f64>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let notional_money = extract_money(&notional).context("notional")?;
        let start = py_to_date(&start_date).context("start_date")?;
        let end = py_to_date(&maturity).context("maturity")?;
        let disc_curve = discount_curve.extract::<&str>().context("discount_curve")?;
        let credit_curve_id = credit_curve.extract::<&str>().context("credit_curve")?;
        let side_value = normalize_cds_side(side.unwrap_or("pay_protection")).context("side")?;
        let recovery = recovery_rate.unwrap_or(STANDARD_RECOVERY_SENIOR);
        if !(0.0..=1.0).contains(&recovery) {
            return Err(PyValueError::new_err(
                "recovery_rate must be between 0 and 1",
            ));
        }

        let mut index_params = CDSIndexParams::new(index_name, series, version, fixed_coupon_bp);
        if let Some(factor) = index_factor {
            index_params = index_params.with_index_factor(factor);
        }
        let construction =
            CDSIndexConstructionParams::new(notional_money, side_value, CDSConvention::IsdaNa);
        let credit_params = CreditParams::new(index_name.to_string(), recovery, credit_curve_id);

        let index = CDSIndex::new_standard(
            id,
            &index_params,
            &construction,
            start,
            end,
            &credit_params,
            disc_curve,
            credit_curve_id,
        );
        Ok(Self::new(
            index.map_err(|e| PyValueError::new_err(e.to_string()))?,
        ))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier for the CDS index position.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Index family name.
    ///
    /// Returns:
    ///     str: Name of the underlying CDS index.
    #[getter]
    fn index_name(&self) -> &str {
        &self.inner.index_name
    }

    /// Notional principal amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Fixed coupon in basis points.
    ///
    /// Returns:
    ///     float: Coupon spread applied to premium leg.
    #[getter]
    fn fixed_coupon_bp(&self) -> f64 {
        self.inner.premium.spread_bp.to_f64().unwrap_or(0.0)
    }

    /// Pay/receive direction for protection.
    ///
    /// Returns:
    ///     str: ``"pay_protection"`` or ``"receive_protection"``.
    #[getter]
    fn side(&self) -> &'static str {
        match self.inner.side {
            PayReceive::PayFixed => "pay_protection",
            PayReceive::ReceiveFixed => "receive_protection",
        }
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve used for premium leg.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.premium.discount_curve_id.as_str().to_string()
    }

    /// Credit curve identifier.
    ///
    /// Returns:
    ///     str: Hazard curve for the index constituents.
    #[getter]
    fn credit_curve(&self) -> String {
        self.inner.protection.credit_curve_id.as_str().to_string()
    }

    /// Maturity date of the index swap.
    ///
    /// Returns:
    ///     datetime.date: Maturity date converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.premium.end)
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.CDS_INDEX``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CDSIndex)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "CDSIndex(id='{}', name='{}', series={}, version={})",
            self.inner.id, self.inner.index_name, self.inner.series, self.inner.version
        ))
    }
}

impl fmt::Display for PyCdsIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CDSIndex({}, series={}, version={})",
            self.inner.index_name, self.inner.series, self.inner.version
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCdsIndex>()?;
    Ok(vec!["CDSIndex"])
}
