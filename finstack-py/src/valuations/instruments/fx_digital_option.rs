use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_digital_option::{DigitalPayoutType, FxDigitalOption};
use finstack_valuations::instruments::OptionType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::sync::Arc;

/// FX digital (binary) option instrument.
///
/// Pays a fixed cash amount if the option expires in-the-money.
///
/// Two payout types:
/// - Cash-or-nothing: pays a fixed amount in the payout currency
/// - Asset-or-nothing: pays one unit of foreign currency
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FxDigitalOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxDigitalOption {
    pub(crate) inner: Arc<FxDigitalOption>,
}

impl PyFxDigitalOption {
    pub(crate) fn new(inner: FxDigitalOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyFxDigitalOption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, strike, option_type, payout_type, payout_amount, expiry, notional, base_currency, quote_currency, domestic_discount_curve, foreign_discount_curve, vol_surface)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create an FX digital option.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     strike: Strike exchange rate (quote per base).
    ///     option_type: Option type (``"call"`` or ``"put"``).
    ///     payout_type: Payout type (``"cash_or_nothing"`` or ``"asset_or_nothing"``).
    ///     payout_amount: Fixed payout amount.
    ///     expiry: Option expiry date.
    ///     notional: Contract notional amount.
    ///     base_currency: Base (foreign) currency.
    ///     quote_currency: Quote (domestic) currency.
    ///     domestic_discount_curve: Domestic discount curve identifier.
    ///     foreign_discount_curve: Foreign discount curve identifier.
    ///     vol_surface: FX volatility surface identifier.
    ///
    /// Returns:
    ///     FxDigitalOption: Configured FX digital option instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        strike: f64,
        option_type: &str,
        payout_type: &str,
        payout_amount: Bound<'_, PyAny>,
        expiry: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
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

        let opt_type = match normalize_label(option_type).as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown option type: {other}"
                )))
            }
        };

        let payout_type_enum = match normalize_label(payout_type).as_str() {
            "cash_or_nothing" | "cashornothing" => DigitalPayoutType::CashOrNothing,
            "asset_or_nothing" | "assetornothing" => DigitalPayoutType::AssetOrNothing,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown payout type: {other}"
                )))
            }
        };

        let payout_money = extract_money(&payout_amount).context("payout_amount")?;
        let notional_money = extract_money(&notional).context("notional")?;

        let option = FxDigitalOption::builder()
            .id(id)
            .base_currency(base_ccy)
            .quote_currency(quote_ccy)
            .strike(strike)
            .option_type(opt_type)
            .payout_type(payout_type_enum)
            .payout_amount(payout_money)
            .expiry(expiry_date)
            .day_count(DayCount::Act365F)
            .notional(notional_money)
            .domestic_discount_curve_id(domestic_discount_curve_id)
            .foreign_discount_curve_id(foreign_discount_curve_id)
            .vol_surface_id(vol_surface_id)
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default())
            .attributes(finstack_valuations::instruments::Attributes::new())
            .build()
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Failed to build FxDigitalOption: {e}"
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
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FxDigitalOption)
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

    /// Strike exchange rate (quote per base).
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Option type label.
    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Payout type label.
    #[getter]
    fn payout_type(&self) -> &'static str {
        match self.inner.payout_type {
            DigitalPayoutType::CashOrNothing => "cash_or_nothing",
            DigitalPayoutType::AssetOrNothing => "asset_or_nothing",
        }
    }

    /// Fixed payout amount.
    #[getter]
    fn payout_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.payout_amount)
    }

    /// Expiry date.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    fn __repr__(&self) -> String {
        format!(
            "FxDigitalOption(id='{}', strike={}, option_type='{}', payout_type='{}')",
            self.inner.id.as_str(),
            self.inner.strike,
            match self.inner.option_type {
                OptionType::Call => "call",
                OptionType::Put => "put",
            },
            match self.inner.payout_type {
                DigitalPayoutType::CashOrNothing => "cash_or_nothing",
                DigitalPayoutType::AssetOrNothing => "asset_or_nothing",
            }
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyFxDigitalOption>()?;
    Ok(vec!["FxDigitalOption"])
}
