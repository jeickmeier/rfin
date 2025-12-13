use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::dates::utils::py_to_date;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::basis_swap::{BasisSwap, BasisSwapLeg};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_frequency(label: Option<&str>) -> PyResult<Tenor> {
    crate::valuations::common::parse_frequency_label(label)
}

fn parse_stub(label: Option<&str>) -> PyResult<finstack_core::dates::StubKind> {
    crate::valuations::common::parse_stub_kind(label)
}

/// Basis swap leg helper mirroring ``BasisSwapLeg``.
///
/// Examples:
///     >>> leg = BasisSwapLeg(
///     ...     "usd_libor_3m",
///     ...     spread=12.5,
///     ...     frequency="quarterly"
///     ... )
///     >>> leg.spread
///     12.5
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BasisSwapLeg",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyBasisSwapLeg {
    pub(crate) inner: BasisSwapLeg,
}

#[pymethods]
impl PyBasisSwapLeg {
    #[new]
    #[pyo3(
        signature = (
            forward_curve,
            *,
            frequency=None,
            day_count=None,
            business_day_convention=None,
            spread=0.0
        ),
        text_signature = "(forward_curve, /, *, frequency='quarterly', day_count='act_360', business_day_convention='modified_following', spread=0.0)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a basis swap leg specification.
    ///
    /// Args:
    ///     forward_curve: Identifier for the forward curve.
    ///     frequency: Optional payment frequency label (e.g., ``"quarterly"``).
    ///     day_count: Optional day-count convention.
    ///     business_day_convention: Optional business-day adjustment rule.
    ///     spread: Optional spread in basis points.
    ///
    /// Returns:
    ///     BasisSwapLeg: Leg specification describing accrual and spread inputs.
    ///
    /// Raises:
    ///     ValueError: If frequency or stub labels are invalid.
    ///     TypeError: If curve identifiers or conventions cannot be parsed.
    fn new(
        forward_curve: Bound<'_, PyAny>,
        frequency: Option<&str>,
        day_count: Option<Bound<'_, PyAny>>,
        business_day_convention: Option<Bound<'_, PyAny>>,
        spread: Option<f64>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let forward_id = forward_curve.extract::<&str>().context("forward_curve")?;
        let freq = parse_frequency(frequency).context("frequency")?;
        let dc = if let Some(obj) = day_count {
            let DayCountArg(value) = obj.extract().context("day_count")?;
            value
        } else {
            DayCount::Act360
        };
        let bdc = if let Some(obj) = business_day_convention {
            let BusinessDayConventionArg(value) =
                obj.extract().context("business_day_convention")?;
            value
        } else {
            BusinessDayConvention::ModifiedFollowing
        };

        Ok(Self {
            inner: BasisSwapLeg {
                forward_curve_id: forward_id.into(),
                frequency: freq,
                day_count: dc,
                bdc,
                spread: spread.unwrap_or(0.0),
                payment_lag_days: 0,
                reset_lag_days: 0,
            },
        })
    }

    /// Forward curve identifier.
    ///
    /// Returns:
    ///     str: Identifier used to retrieve the forward curve.
    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    /// Spread in basis points.
    ///
    /// Returns:
    ///     float: Leg spread applied to the floating rate.
    #[getter]
    fn spread(&self) -> f64 {
        self.inner.spread
    }
}

/// Basis swap wrapper with convenience constructor.
///
/// Examples:
///     >>> swap = BasisSwap.create(
///     ...     "basis_usd",
///     ...     Money("USD", 10_000_000),
///     ...     date(2024, 1, 2),
///     ...     date(2027, 1, 2),
///     ...     primary_leg,
///     ...     reference_leg,
///     ...     "usd_discount"
///     ... )
///     >>> swap.notional.amount
///     10000000
#[pyclass(module = "finstack.valuations.instruments", name = "BasisSwap", frozen)]
#[derive(Clone, Debug)]
pub struct PyBasisSwap {
    pub(crate) inner: BasisSwap,
}

impl PyBasisSwap {
    pub(crate) fn new(inner: BasisSwap) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBasisSwap {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            start_date,
            maturity,
            primary_leg,
            reference_leg,
            discount_curve,
            *,
            calendar=None,
            stub="none"
        ),
        text_signature = "(cls, instrument_id, notional, start_date, maturity, primary_leg, reference_leg, discount_curve, /, *, calendar=None, stub='none')"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a floating-for-floating basis swap with two legs.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Swap notional as :class:`finstack.core.money.Money`.
    ///     start_date: Effective start date of the swap.
    ///     maturity: Maturity date of the swap.
    ///     primary_leg: Primary leg specification.
    ///     reference_leg: Reference leg specification.
    ///     discount_curve: Discount curve identifier for valuation.
    ///     calendar: Optional calendar identifier for scheduling adjustments.
    ///     stub: Optional stub label (e.g., ``"short_front"``).
    ///
    /// Returns:
    ///     BasisSwap: Configured basis swap instrument ready for pricing.
    ///
    /// Raises:
    ///     ValueError: If frequency or stub settings are invalid.
    ///     RuntimeError: When the underlying builder detects inconsistent input.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        start_date: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        primary_leg: &PyBasisSwapLeg,
        reference_leg: &PyBasisSwapLeg,
        discount_curve: Bound<'_, PyAny>,
        calendar: Option<&str>,
        stub: Option<&str>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let notional_money = extract_money(&notional).context("notional")?;
        let start = py_to_date(&start_date).context("start_date")?;
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let stub_kind = parse_stub(stub).context("stub")?;

        let mut builder = BasisSwap::builder();
        builder = builder.id(id);
        builder = builder.notional(notional_money);
        builder = builder.start_date(start);
        builder = builder.maturity_date(maturity_date);
        builder = builder.primary_leg(primary_leg.inner.clone());
        builder = builder.reference_leg(reference_leg.inner.clone());
        builder = builder.discount_curve_id(discount_curve_id);
        builder = builder.stub_kind(stub_kind);
        builder = builder.calendar_id_opt(calendar.map(|s| s.to_string()));
        builder = builder.attributes(Default::default());

        let swap = builder.build().map_err(core_to_py)?;
        Ok(Self::new(swap))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the swap.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Swap notional amount.
    ///
    /// Returns:
    ///     Money: Notional expressed as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Identifier of the discount curve used for valuation.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.BasisSwap``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::BasisSwap)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("BasisSwap(id='{}')", self.inner.id))
    }
}

impl fmt::Display for PyBasisSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BasisSwap({}, notional={})",
            self.inner.id, self.inner.notional
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyBasisSwapLeg>()?;
    module.add_class::<PyBasisSwap>()?;
    Ok(vec!["BasisSwapLeg", "BasisSwap"])
}
