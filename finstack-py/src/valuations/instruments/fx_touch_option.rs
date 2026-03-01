use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_touch_option::{
    BarrierDirection, FxTouchOption, PayoutTiming, TouchType,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::sync::Arc;

/// FX touch option (American binary option) instrument.
///
/// Touch options pay a fixed amount if the spot rate touches a barrier
/// level at any time before expiry:
/// - One-touch: pays if barrier is touched
/// - No-touch: pays if barrier is NOT touched
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxTouchOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxTouchOption {
    pub(crate) inner: Arc<FxTouchOption>,
}

impl PyFxTouchOption {
    pub(crate) fn new(inner: FxTouchOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyFxTouchOption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, barrier_level, touch_type, barrier_direction, payout_amount, payout_timing, expiry, base_currency, quote_currency, domestic_discount_curve, foreign_discount_curve, vol_surface)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create an FX touch option.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     barrier_level: Barrier exchange rate level.
    ///     touch_type: Touch type (``"one_touch"`` or ``"no_touch"``).
    ///     barrier_direction: Barrier direction (``"up"`` or ``"down"``).
    ///     payout_amount: Fixed payout amount.
    ///     payout_timing: Payout timing (``"at_hit"`` or ``"at_expiry"``).
    ///     expiry: Option expiry date.
    ///     base_currency: Base (foreign) currency.
    ///     quote_currency: Quote (domestic) currency.
    ///     domestic_discount_curve: Domestic discount curve identifier.
    ///     foreign_discount_curve: Foreign discount curve identifier.
    ///     vol_surface: FX volatility surface identifier.
    ///
    /// Returns:
    ///     FxTouchOption: Configured FX touch option instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        barrier_level: f64,
        touch_type: &str,
        barrier_direction: &str,
        payout_amount: Bound<'_, PyAny>,
        payout_timing: &str,
        expiry: Bound<'_, PyAny>,
        base_currency: Bound<'_, PyAny>,
        quote_currency: Bound<'_, PyAny>,
        domestic_discount_curve: Bound<'_, PyAny>,
        foreign_discount_curve: Bound<'_, PyAny>,
        vol_surface: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use crate::core::common::args::CurrencyArg;
        use crate::core::common::labels::normalize_label;
        use crate::errors::PyContext;
        use finstack_core::dates::DayCount;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let expiry_date = py_to_date(&expiry).context("expiry")?;
        let domestic_discount_curve_id = CurveId::new(
            domestic_discount_curve
                .extract::<&str>()
                .context("domestic_discount_curve")?,
        );
        let foreign_discount_curve_id = CurveId::new(
            foreign_discount_curve
                .extract::<&str>()
                .context("foreign_discount_curve")?,
        );
        let vol_surface_id = CurveId::new(vol_surface.extract::<&str>().context("vol_surface")?);

        let CurrencyArg(base_ccy) = base_currency.extract().context("base_currency")?;
        let CurrencyArg(quote_ccy) = quote_currency.extract().context("quote_currency")?;

        let touch_type_enum = match normalize_label(touch_type).as_str() {
            "one_touch" | "onetouch" => TouchType::OneTouch,
            "no_touch" | "notouch" => TouchType::NoTouch,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown touch type: {other}"
                )))
            }
        };

        let barrier_dir_enum = match normalize_label(barrier_direction).as_str() {
            "up" => BarrierDirection::Up,
            "down" => BarrierDirection::Down,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown barrier direction: {other}"
                )))
            }
        };

        let payout_timing_enum = match normalize_label(payout_timing).as_str() {
            "at_hit" | "athit" => PayoutTiming::AtHit,
            "at_expiry" | "atexpiry" => PayoutTiming::AtExpiry,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown payout timing: {other}"
                )))
            }
        };

        let payout_money = extract_money(&payout_amount).context("payout_amount")?;

        let option = FxTouchOption::builder()
            .id(id)
            .base_currency(base_ccy)
            .quote_currency(quote_ccy)
            .barrier_level(barrier_level)
            .touch_type(touch_type_enum)
            .barrier_direction(barrier_dir_enum)
            .payout_amount(payout_money)
            .payout_timing(payout_timing_enum)
            .expiry(expiry_date)
            .day_count(DayCount::Act365F)
            .domestic_discount_curve_id(domestic_discount_curve_id)
            .foreign_discount_curve_id(foreign_discount_curve_id)
            .vol_surface_id(vol_surface_id)
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default())
            .attributes(finstack_valuations::instruments::Attributes::new())
            .build()
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Failed to build FxTouchOption: {e}"
                ))
            })?;
        Ok(Self::new(option))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument type.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxTouchOption)
    }

    /// Base currency (foreign currency).
    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    /// Quote currency (domestic currency).
    #[getter]
    fn quote_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.quote_currency)
    }

    /// Barrier level (exchange rate).
    #[getter]
    fn barrier_level(&self) -> f64 {
        self.inner.barrier_level
    }

    /// Touch type label.
    #[getter]
    fn touch_type(&self) -> &'static str {
        match self.inner.touch_type {
            TouchType::OneTouch => "one_touch",
            TouchType::NoTouch => "no_touch",
        }
    }

    /// Barrier direction label.
    #[getter]
    fn barrier_direction(&self) -> &'static str {
        match self.inner.barrier_direction {
            BarrierDirection::Up => "up",
            BarrierDirection::Down => "down",
        }
    }

    /// Fixed payout amount.
    #[getter]
    fn payout_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.payout_amount)
    }

    /// Payout timing label.
    #[getter]
    fn payout_timing(&self) -> &'static str {
        match self.inner.payout_timing {
            PayoutTiming::AtHit => "at_hit",
            PayoutTiming::AtExpiry => "at_expiry",
        }
    }

    /// Expiry date.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    fn __repr__(&self) -> String {
        format!(
            "FxTouchOption(id='{}', barrier={}, touch_type='{}', direction='{}')",
            self.inner.id.as_str(),
            self.inner.barrier_level,
            match self.inner.touch_type {
                TouchType::OneTouch => "one_touch",
                TouchType::NoTouch => "no_touch",
            },
            match self.inner.barrier_direction {
                BarrierDirection::Up => "up",
                BarrierDirection::Down => "down",
            }
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyFxTouchOption>()?;
    Ok(vec!["FxTouchOption"])
}
