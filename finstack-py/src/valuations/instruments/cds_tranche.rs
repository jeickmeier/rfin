use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::{
    frequency_from_payments_per_year, to_optional_string, PyInstrumentType,
};
use finstack_core::dates::{BusinessDayConvention, DayCount};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::cds_tranche::{CdsTranche, TrancheSide};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_tranche_side(label: Option<&str>) -> PyResult<TrancheSide> {
    match label {
        None => Ok(TrancheSide::BuyProtection),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// CDS tranche wrapper exposing a simplified constructor.
///
/// Examples:
///     >>> tranche = CdsTranche.create(
///     ...     "itraxx_tranche",
///     ...     "iTraxx Europe",
///     ...     38,
///     ...     3.0,
///     ...     7.0,
///     ...     Money("EUR", 10_000_000),
///     ...     date(2029, 3, 20),
///     ...     500.0,
///     ...     "eur_discount",
///     ...     "itraxx_credit"
///     ... )
///     >>> tranche.attach_pct
///     3.0
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CdsTranche",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCdsTranche {
    pub(crate) inner: CdsTranche,
}

impl PyCdsTranche {
    pub(crate) fn new(inner: CdsTranche) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCdsTranche {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            index_name,
            series,
            attach_pct,
            detach_pct,
            notional,
            maturity,
            running_coupon_bp,
            discount_curve,
            credit_index_curve,
            *,
            side="buy_protection",
            payments_per_year=4,
            day_count=None,
            business_day_convention=None,
            calendar=None,
            effective_date=None
        ),
        text_signature = "(cls, instrument_id, index_name, series, attach_pct, detach_pct, notional, maturity, running_coupon_bp, discount_curve, credit_index_curve, /, *, side='buy_protection', payments_per_year=4, day_count='act_360', business_day_convention='following', calendar=None, effective_date=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a CDS tranche referencing a credit index.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     index_name: Name of the underlying index.
    ///     series: Index series number.
    ///     attach_pct: Attachment point in percent.
    ///     detach_pct: Detachment point in percent.
    ///     notional: Tranche notional as :class:`finstack.core.money.Money`.
    ///     maturity: Maturity date of the tranche.
    ///     running_coupon_bp: Running spread in basis points.
    ///     discount_curve: Discount curve identifier.
    ///     credit_index_curve: Credit curve identifier for the index.
    ///     side: Optional pay/receive label for protection.
    ///     payments_per_year: Optional payment frequency per year.
    ///     day_count: Optional day-count convention.
    ///     business_day_convention: Optional business-day convention.
    ///     calendar: Optional calendar identifier.
    ///     effective_date: Optional effective date override.
    ///
    /// Returns:
    ///     CdsTranche: Configured tranche instrument.
    ///
    /// Raises:
    ///     ValueError: If attachment/detachment points or inputs are invalid.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        index_name: &str,
        series: u16,
        attach_pct: f64,
        detach_pct: f64,
        notional: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        running_coupon_bp: f64,
        discount_curve: Bound<'_, PyAny>,
        credit_index_curve: Bound<'_, PyAny>,
        side: Option<&str>,
        payments_per_year: Option<u32>,
        day_count: Option<Bound<'_, PyAny>>,
        business_day_convention: Option<Bound<'_, PyAny>>,
        calendar: Option<&str>,
        effective_date: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        if attach_pct < 0.0 || detach_pct <= attach_pct {
            return Err(PyValueError::new_err(
                "detach_pct must be greater than attach_pct and both non-negative",
            ));
        }

        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let notional_money = extract_money(&notional).context("notional")?;
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        let disc_curve = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let credit_curve = CurveId::new(
            credit_index_curve
                .extract::<&str>()
                .context("credit_index_curve")?,
        );

        let side_value = parse_tranche_side(side).context("side")?;
        let freq =
            frequency_from_payments_per_year(payments_per_year).context("payments_per_year")?;
        let dc = if let Some(obj) = day_count {
            let DayCountArg(value) = obj.extract()?;
            value
        } else {
            DayCount::Act360
        };
        let bdc = if let Some(obj) = business_day_convention {
            let BusinessDayConventionArg(value) = obj.extract()?;
            value
        } else {
            BusinessDayConvention::Following
        };
        let eff = if let Some(date_obj) = effective_date {
            Some(py_to_date(&date_obj)?)
        } else {
            None
        };

        let mut builder = CdsTranche::builder();
        builder = builder.id(id);
        builder = builder.index_name(index_name.to_string());
        builder = builder.series(series);
        builder = builder.attach_pct(attach_pct);
        builder = builder.detach_pct(detach_pct);
        builder = builder.notional(notional_money);
        builder = builder.maturity(maturity_date);
        builder = builder.running_coupon_bp(running_coupon_bp);
        builder = builder.payment_frequency(freq);
        builder = builder.day_count(dc);
        builder = builder.business_day_convention(bdc);
        builder = builder.calendar_id_opt(to_optional_string(calendar));
        builder = builder.discount_curve_id(disc_curve.into());
        builder = builder.credit_index_id(credit_curve.into());
        builder = builder.side(side_value);
        builder = builder.effective_date_opt(eff);
        builder = builder.attributes(Default::default());
        builder = builder.standard_imm_dates(true);
        builder = builder.accumulated_loss(0.0);

        let tranche = builder.build().map_err(core_to_py)?;
        Ok(Self::new(tranche))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier for the tranche.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Tranche notional amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Attachment point percentage.
    ///
    /// Returns:
    ///     float: Attachment level in percent.
    #[getter]
    fn attach_pct(&self) -> f64 {
        self.inner.attach_pct
    }

    /// Detachment point percentage.
    ///
    /// Returns:
    ///     float: Detachment level in percent.
    #[getter]
    fn detach_pct(&self) -> f64 {
        self.inner.detach_pct
    }

    /// Running coupon in basis points.
    ///
    /// Returns:
    ///     float: Running spread paid on outstanding tranche balance.
    #[getter]
    fn running_coupon_bp(&self) -> f64 {
        self.inner.running_coupon_bp
    }

    /// Maturity date of the tranche.
    ///
    /// Returns:
    ///     datetime.date: Maturity converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.maturity)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve used for valuation.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Credit index curve identifier.
    ///
    /// Returns:
    ///     str: Hazard curve used for the index portfolio.
    #[getter]
    fn credit_index_curve(&self) -> String {
        self.inner.credit_index_id.as_str().to_string()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.CDS_TRANCHE``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CDSTranche)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "CdsTranche(id='{}', attach={:.2}%, detach={:.2}%)",
            self.inner.id, self.inner.attach_pct, self.inner.detach_pct
        ))
    }
}

impl fmt::Display for PyCdsTranche {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CdsTranche({}, attach={:.2}%, detach={:.2}%)",
            self.inner.index_name, self.inner.attach_pct, self.inner.detach_pct
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCdsTranche>()?;
    Ok(vec!["CdsTranche"])
}
