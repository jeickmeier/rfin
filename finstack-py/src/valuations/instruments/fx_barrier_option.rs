use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::barrier_option::types::BarrierType;
use finstack_valuations::instruments::fx_barrier_option::FxBarrierOption;
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;

/// FX barrier option instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxBarrierOption",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyFxBarrierOption {
    pub(crate) inner: FxBarrierOption,
}

impl PyFxBarrierOption {
    pub(crate) fn new(inner: FxBarrierOption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFxBarrierOption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, strike, barrier, option_type, barrier_type, expiry, notional, domestic_currency, foreign_currency, discount_curve, foreign_discount_curve, fx_spot_id, fx_vol_surface, *, use_gobet_miri=False)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create an FX barrier option.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     strike: Strike price in quote currency.
    ///     barrier: Barrier level in quote currency.
    ///     option_type: Option type (``"call"`` or ``"put"``).
    ///     barrier_type: Barrier type (``"up_and_out"``, ``"up_and_in"``, ``"down_and_out"``, ``"down_and_in"``).
    ///     expiry: Option expiry date.
    ///     notional: Contract notional amount.
    ///     domestic_currency: Currency for settlement.
    ///     foreign_currency: Currency of the underlying.
    ///     correlation: Correlation between FX and rates.
    ///     discount_curve: Discount curve identifier.
    ///     fx_spot_id: FX spot rate identifier.
    ///     fx_vol_surface: FX volatility surface identifier.
    ///     use_gobet_miri: Whether to use Gobet-Miri approximation.
    ///
    /// Returns:
    ///     FxBarrierOption: Configured FX barrier option instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        strike: f64,
        barrier: f64,
        option_type: &str,
        barrier_type: &str,
        expiry: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        domestic_currency: Bound<'_, PyAny>,
        foreign_currency: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        foreign_discount_curve: Bound<'_, PyAny>,
        fx_spot_id: &str,
        fx_vol_surface: Bound<'_, PyAny>,
        use_gobet_miri: Option<bool>,
    ) -> PyResult<Self> {
        use crate::core::common::args::CurrencyArg;
        use crate::core::common::labels::normalize_label;
        use finstack_core::dates::DayCount;
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let expiry_date = py_to_date(&expiry).context("expiry")?;
        let domestic_discount_curve_id = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let foreign_discount_curve_id = CurveId::new(foreign_discount_curve.extract::<&str>().context("foreign_discount_curve")?);
        let fx_vol_id = CurveId::new(fx_vol_surface.extract::<&str>().context("fx_vol_surface")?);

        let CurrencyArg(dom_currency) = domestic_currency.extract().context("domestic_currency")?;
        let CurrencyArg(for_currency) = foreign_currency.extract().context("foreign_currency")?;

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

        let notional_money = extract_money(&notional)?;
        let strike_money = finstack_core::money::Money::new(strike, for_currency);
        let barrier_money = finstack_core::money::Money::new(barrier, for_currency);

        let mut builder = FxBarrierOption::builder();
        builder = builder.id(id);
        builder = builder.strike(strike_money);
        builder = builder.barrier(barrier_money);
        builder = builder.option_type(opt_type);
        builder = builder.barrier_type(barrier_type_enum);
        builder = builder.expiry(expiry_date);
        builder = builder.notional(notional_money);
        builder = builder.domestic_currency(dom_currency);
        builder = builder.foreign_currency(for_currency);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.use_gobet_miri(use_gobet_miri.unwrap_or(false));
        builder = builder.domestic_discount_curve_id(domestic_discount_curve_id);
        builder = builder.foreign_discount_curve_id(foreign_discount_curve_id);
        builder = builder.fx_spot_id(fx_spot_id.to_string());
        builder = builder.fx_vol_id(fx_vol_id.into());
        let option = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to build FxBarrierOption: {e}"
            ))
        })?;
        Ok(Self::new(option))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Strike price as money.
    #[getter]
    fn strike(&self) -> PyMoney {
        PyMoney::new(self.inner.strike)
    }

    /// Barrier level as money.
    #[getter]
    fn barrier(&self) -> PyMoney {
        PyMoney::new(self.inner.barrier)
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
    fn expiry(&self, py: Python<'_>) -> PyResult<PyObject> {
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

    fn __repr__(&self) -> String {
        format!(
            "FxBarrierOption(id='{}', strike={}, barrier={}, barrier_type='{}')",
            self.inner.id.as_str(),
            self.inner.strike.amount(),
            self.inner.barrier.amount(),
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
    parent.add_class::<PyFxBarrierOption>()?;
    Ok(vec!["FxBarrierOption"])
}
