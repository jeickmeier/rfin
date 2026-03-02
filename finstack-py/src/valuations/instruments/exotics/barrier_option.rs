use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::exotics::barrier_option::{BarrierOption, BarrierType};
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::sync::Arc;

/// Barrier type for barrier options.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BarrierType",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyBarrierType {
    pub(crate) inner: BarrierType,
}

impl PyBarrierType {
    pub(crate) const fn new(inner: BarrierType) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            BarrierType::UpAndOut => "up_and_out",
            BarrierType::UpAndIn => "up_and_in",
            BarrierType::DownAndOut => "down_and_out",
            BarrierType::DownAndIn => "down_and_in",
        }
    }
}

#[pymethods]
impl PyBarrierType {
    #[classattr]
    const UP_AND_OUT: Self = Self::new(BarrierType::UpAndOut);
    #[classattr]
    const UP_AND_IN: Self = Self::new(BarrierType::UpAndIn);
    #[classattr]
    const DOWN_AND_OUT: Self = Self::new(BarrierType::DownAndOut);
    #[classattr]
    const DOWN_AND_IN: Self = Self::new(BarrierType::DownAndIn);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a barrier type from a string label.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        use crate::core::common::labels::normalize_label;
        match normalize_label(name).as_str() {
            "up_and_out" | "upandout" => Ok(Self::new(BarrierType::UpAndOut)),
            "up_and_in" | "upandin" => Ok(Self::new(BarrierType::UpAndIn)),
            "down_and_out" | "downandout" => Ok(Self::new(BarrierType::DownAndOut)),
            "down_and_in" | "downandin" => Ok(Self::new(BarrierType::DownAndIn)),
            other => Err(PyValueError::new_err(format!(
                "Unknown barrier type: {other}"
            ))),
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("BarrierType('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

/// Barrier option instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BarrierOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBarrierOption {
    pub(crate) inner: Arc<BarrierOption>,
}

impl PyBarrierOption {
    pub(crate) fn new(inner: BarrierOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyBarrierOption {
    #[classmethod]
    #[pyo3(
        signature = (instrument_id, ticker, strike, barrier, option_type, barrier_type, expiry, notional, discount_curve, spot_id, vol_surface, *, div_yield_id=None, rebate=None, use_gobet_miri=None),
        text_signature = "(cls, instrument_id, ticker, strike, barrier, option_type, barrier_type, expiry, notional, discount_curve, spot_id, vol_surface, *, div_yield_id=None, rebate=None, use_gobet_miri=True)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a barrier option.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     ticker: Equity ticker symbol.
    ///     strike: Strike price in quote currency.
    ///     barrier: Barrier level in quote currency.
    ///     option_type: Option type (``"call"`` or ``"put"``).
    ///     barrier_type: Barrier type (``"up_and_out"``, ``"up_and_in"``, ``"down_and_out"``, ``"down_and_in"``).
    ///     expiry: Option expiry date.
    ///     notional: Contract notional as :class:`finstack.core.money.Money`.
    ///     discount_curve: Discount curve identifier.
    ///     spot_id: Spot price identifier.
    ///     vol_surface: Volatility surface identifier.
    ///     div_yield_id: Optional dividend yield identifier.
    ///     rebate: Optional rebate payment on knock-out as :class:`finstack.core.money.Money`.
    ///     use_gobet_miri: Whether to use Gobet-Miri discrete barrier correction (default: True).
    ///
    /// Returns:
    ///     BarrierOption: Configured barrier option instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        strike: f64,
        barrier: f64,
        option_type: &str,
        barrier_type: &str,
        expiry: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        spot_id: &str,
        vol_surface: Bound<'_, PyAny>,
        div_yield_id: Option<&str>,
        rebate: Option<Bound<'_, PyAny>>,
        use_gobet_miri: Option<bool>,
    ) -> PyResult<Self> {
        use crate::core::common::labels::normalize_label;
        use crate::errors::PyContext;
        use finstack_core::dates::DayCount;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let expiry_date = py_to_date(&expiry).context("expiry")?;
        let notional_money = extract_money(&notional).context("notional")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let vol_surface_id = CurveId::new(vol_surface.extract::<&str>().context("vol_surface")?);

        let opt_type = match normalize_label(option_type).as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown option type: {other}"
                )))
            }
        };

        let barrier_type_enum = match normalize_label(barrier_type).as_str() {
            "up_and_out" | "upandout" => BarrierType::UpAndOut,
            "up_and_in" | "upandin" => BarrierType::UpAndIn,
            "down_and_out" | "downandout" => BarrierType::DownAndOut,
            "down_and_in" | "downandin" => BarrierType::DownAndIn,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown barrier type: {other}"
                )))
            }
        };

        let barrier_money = finstack_core::money::Money::new(barrier, notional_money.currency());

        let rebate_money = rebate
            .map(|r| extract_money(&r).context("rebate"))
            .transpose()?;

        let mut builder = BarrierOption::builder();
        builder = builder.id(id);
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.strike(strike);
        builder = builder.barrier(barrier_money);
        if let Some(r) = rebate_money {
            builder = builder.rebate(r);
        }
        builder = builder.option_type(opt_type);
        builder = builder.barrier_type(barrier_type_enum);
        builder = builder.expiry(expiry_date);
        builder = builder.notional(notional_money);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.use_gobet_miri(use_gobet_miri.unwrap_or(true));
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.discount_curve_id(discount_curve_id);
        builder = builder.spot_id(spot_id.to_string().into());
        builder = builder.vol_surface_id(vol_surface_id);
        if let Some(div) = div_yield_id {
            builder = builder.div_yield_id(div.into());
        }
        let option = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to build BarrierOption: {e}"))
        })?;
        Ok(Self::new(option))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Underlying ticker symbol.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.underlying_ticker
    }

    /// Strike price as scalar.
    ///
    /// Returns:
    ///     float: Strike price in underlying price units.
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Barrier level as money.
    #[getter]
    fn barrier(&self) -> PyMoney {
        PyMoney::new(self.inner.barrier)
    }

    /// Rebate payment on knock-out (if any).
    ///
    /// Returns:
    ///     Money | None: Rebate amount or None.
    #[getter]
    fn rebate(&self) -> Option<PyMoney> {
        self.inner.rebate.map(PyMoney::new)
    }

    /// Option type label.
    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Barrier type label.
    #[getter]
    fn barrier_type(&self) -> &'static str {
        match self.inner.barrier_type {
            BarrierType::UpAndOut => "up_and_out",
            BarrierType::UpAndIn => "up_and_in",
            BarrierType::DownAndOut => "down_and_out",
            BarrierType::DownAndIn => "down_and_in",
        }
    }

    /// Expiry date.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    /// Notional amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Day count convention.
    ///
    /// Returns:
    ///     DayCount: Day count convention used for time fraction calculations.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Spot price identifier.
    ///
    /// Returns:
    ///     str: Spot price identifier.
    #[getter]
    fn spot_id(&self) -> &str {
        &self.inner.spot_id
    }

    /// Volatility surface identifier.
    ///
    /// Returns:
    ///     str: Volatility surface identifier used for pricing.
    #[getter]
    fn vol_surface(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    /// Dividend yield identifier (if any).
    ///
    /// Returns:
    ///     str | None: Dividend yield identifier or None.
    #[getter]
    fn div_yield_id(&self) -> Option<&str> {
        self.inner.div_yield_id.as_deref()
    }

    /// Whether Gobet-Miri discrete barrier correction is enabled.
    ///
    /// Returns:
    ///     bool: True if Gobet-Miri correction is active.
    #[getter]
    fn use_gobet_miri(&self) -> bool {
        self.inner.use_gobet_miri
    }

    fn __repr__(&self) -> String {
        format!(
            "BarrierOption(id='{}', ticker='{}', strike={}, barrier={} {}, barrier_type='{}')",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.strike,
            self.inner.barrier.amount(),
            self.inner.barrier.currency(),
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
    parent.add_class::<PyBarrierType>()?;
    parent.add_class::<PyBarrierOption>()?;
    Ok(vec!["BarrierType", "BarrierOption"])
}
