//! Python bindings for CommodityOption instrument.

use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_option::CommodityOption;
use finstack_valuations::instruments::{
    Attributes, ExerciseStyle, OptionType, PricingOverrides, SettlementType,
};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyType};
use pyo3::{Bound, Py};
use std::fmt;
use std::sync::Arc;

/// Commodity option (call or put on commodity forward/spot).
///
/// Represents an option to buy (call) or sell (put) a commodity at a specified
/// strike price on or before expiry. Supports European and American exercise.
///
/// Pricing uses Black-76 for European exercise and binomial tree for American.
///
/// Examples:
///     >>> option = CommodityOption.create(
///     ...     "WTI-CALL-75-2025M06",
///     ...     commodity_type="Energy",
///     ...     ticker="CL",
///     ...     strike=75.0,
///     ...     option_type="call",
///     ...     exercise_style="european",
///     ...     expiry=Date(2025, 6, 15),
///     ...     quantity=1000.0,
///     ...     unit="BBL",
///     ...     currency="USD",
///     ...     forward_curve_id="WTI-FORWARD",
///     ...     discount_curve_id="USD-OIS",
///     ...     vol_surface_id="WTI-VOL"
///     ... )
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommodityOption",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCommodityOption {
    pub(crate) inner: Arc<CommodityOption>,
}

impl PyCommodityOption {
    pub(crate) fn new(inner: CommodityOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyCommodityOption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, *, commodity_type, ticker, strike, option_type, exercise_style, expiry, quantity, unit, currency, forward_curve_id, discount_curve_id, vol_surface_id, multiplier=1.0, settlement_type='cash', day_count=None, spot_price_id=None, quoted_forward=None, implied_volatility=None, tree_steps=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            *,
            commodity_type,
            ticker,
            strike,
            option_type,
            exercise_style,
            expiry,
            quantity,
            unit,
            currency,
            forward_curve_id,
            discount_curve_id,
            vol_surface_id,
            multiplier = 1.0,
            settlement_type = "cash",
            day_count = None,
            spot_price_id = None,
            quoted_forward = None,
            implied_volatility = None,
            tree_steps = None
        )
    )]
    /// Create a commodity option.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for this instrument.
    ///     commodity_type: Commodity type (e.g., "Energy", "Metal", "Agricultural").
    ///     ticker: Ticker or symbol (e.g., "CL" for WTI, "GC" for Gold).
    ///     strike: Strike price per unit.
    ///     option_type: Option type ("call" or "put").
    ///     exercise_style: Exercise style ("european" or "american").
    ///     expiry: Option expiry date.
    ///     quantity: Contract quantity in units.
    ///     unit: Unit of measurement (e.g., "BBL", "MT", "OZ").
    ///     currency: Currency for pricing.
    ///     forward_curve_id: Forward/futures curve ID for price interpolation.
    ///     discount_curve_id: Discount curve ID.
    ///     vol_surface_id: Volatility surface ID for implied vol.
    ///     multiplier: Contract multiplier (default 1.0).
    ///     settlement_type: Settlement type ("physical" or "cash", default "cash").
    ///     day_count: Day count convention (default Act365F).
    ///     spot_price_id: Optional spot price ID (for American options).
    ///     quoted_forward: Optional quoted forward price (overrides curve lookup).
    ///     implied_volatility: Optional implied vol override (overrides surface lookup).
    ///     tree_steps: Number of steps for binomial tree (American exercise only).
    ///
    /// Returns:
    ///     CommodityOption: Configured commodity option instrument.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        commodity_type: &str,
        ticker: &str,
        strike: f64,
        option_type: &str,
        exercise_style: &str,
        expiry: Bound<'_, PyAny>,
        quantity: f64,
        unit: &str,
        currency: Bound<'_, PyAny>,
        forward_curve_id: &str,
        discount_curve_id: &str,
        vol_surface_id: &str,
        multiplier: f64,
        settlement_type: &str,
        day_count: Option<Bound<'_, PyAny>>,
        spot_price_id: Option<&str>,
        quoted_forward: Option<f64>,
        implied_volatility: Option<f64>,
        tree_steps: Option<usize>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        let expiry_date = py_to_date(&expiry).context("expiry")?;

        // Parse option type
        let opt_type = match option_type.to_lowercase().as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid option_type: '{}'. Must be 'call' or 'put'",
                    option_type
                )));
            }
        };

        // Parse exercise style
        let exercise = match exercise_style.to_lowercase().as_str() {
            "european" => ExerciseStyle::European,
            "american" => ExerciseStyle::American,
            "bermudan" => ExerciseStyle::Bermudan,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid exercise_style: '{}'. Must be 'european', 'american', or 'bermudan'",
                    exercise_style
                )));
            }
        };

        // Parse settlement type
        let settlement = match settlement_type.to_lowercase().as_str() {
            "physical" => SettlementType::Physical,
            "cash" => SettlementType::Cash,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid settlement_type: '{}'. Must be 'physical' or 'cash'",
                    settlement_type
                )));
            }
        };

        // Parse day count
        let dc = if let Some(dc_arg) = day_count {
            if let Ok(py_dc) = dc_arg.extract::<pyo3::PyRef<PyDayCount>>() {
                py_dc.inner
            } else if let Ok(name) = dc_arg.extract::<&str>() {
                match name.to_lowercase().as_str() {
                    "act_360" | "act/360" => DayCount::Act360,
                    "act_365f" | "act/365f" | "act365f" => DayCount::Act365F,
                    "act_act" | "act/act" | "actact" => DayCount::ActAct,
                    "thirty_360" | "30/360" | "30e/360" => DayCount::Thirty360,
                    other => {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "Unsupported day count '{}'",
                            other
                        )));
                    }
                }
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "day_count expects DayCount or str",
                ));
            }
        } else {
            DayCount::Act365F
        };

        // Build pricing overrides
        let mut pricing_overrides = PricingOverrides::default();
        if let Some(vol) = implied_volatility {
            pricing_overrides.implied_volatility = Some(vol);
        }
        if let Some(steps) = tree_steps {
            pricing_overrides.tree_steps = Some(steps);
        }

        let mut builder = CommodityOption::builder()
            .id(id)
            .commodity_type(commodity_type.to_string())
            .ticker(ticker.to_string())
            .strike(strike)
            .option_type(opt_type)
            .exercise_style(exercise)
            .expiry(expiry_date)
            .quantity(quantity)
            .unit(unit.to_string())
            .multiplier(multiplier)
            .settlement(settlement)
            .currency(ccy)
            .forward_curve_id(CurveId::new(forward_curve_id))
            .discount_curve_id(CurveId::new(discount_curve_id))
            .vol_surface_id(CurveId::new(vol_surface_id))
            .day_count(dc)
            .pricing_overrides(pricing_overrides)
            .attributes(Attributes::new());

        if let Some(sp_id) = spot_price_id {
            builder = builder.spot_price_id_opt(Some(sp_id.to_string()));
        }
        if let Some(qf) = quoted_forward {
            builder = builder.quoted_forward_opt(Some(qf));
        }

        let option = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{}", e)))?;

        Ok(Self::new(option))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Commodity type (e.g., "Energy", "Metal").
    #[getter]
    fn commodity_type(&self) -> &str {
        &self.inner.commodity_type
    }

    /// Ticker symbol.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.ticker
    }

    /// Strike price.
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Option type (call or put).
    #[getter]
    fn option_type(&self) -> &str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Exercise style (european or american).
    #[getter]
    fn exercise_style(&self) -> &str {
        match self.inner.exercise_style {
            ExerciseStyle::European => "european",
            ExerciseStyle::American => "american",
            ExerciseStyle::Bermudan => "bermudan",
        }
    }

    /// Expiry date.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    /// Contract quantity.
    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    /// Unit of measurement.
    #[getter]
    fn unit(&self) -> &str {
        &self.inner.unit
    }

    /// Contract multiplier.
    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    /// Settlement type (physical or cash).
    #[getter]
    fn settlement_type(&self) -> &str {
        match self.inner.settlement {
            SettlementType::Physical => "physical",
            SettlementType::Cash => "cash",
        }
    }

    /// Currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    /// Forward curve ID.
    #[getter]
    fn forward_curve_id(&self) -> &str {
        self.inner.forward_curve_id.as_str()
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    /// Volatility surface ID.
    #[getter]
    fn vol_surface_id(&self) -> &str {
        self.inner.vol_surface_id.as_str()
    }

    /// Optional spot price ID.
    #[getter]
    fn spot_price_id(&self) -> Option<&str> {
        self.inner.spot_price_id.as_deref()
    }

    /// Optional quoted forward price.
    #[getter]
    fn quoted_forward(&self) -> Option<f64> {
        self.inner.quoted_forward
    }

    /// Day count convention.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Instrument type key.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        use finstack_valuations::instruments::Instrument;
        PyInstrumentType::new(self.inner.key())
    }

    fn __repr__(&self) -> String {
        format!(
            "CommodityOption(id='{}', ticker='{}', strike={}, type='{}', exercise='{}', expiry='{}')",
            self.inner.id.as_str(),
            self.inner.ticker,
            self.inner.strike,
            self.option_type(),
            self.exercise_style(),
            self.inner.expiry
        )
    }
}

impl fmt::Display for PyCommodityOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommodityOption({}, {}, {} {} @ {})",
            self.inner.id.as_str(),
            self.inner.ticker,
            self.option_type(),
            self.exercise_style(),
            self.inner.strike
        )
    }
}

/// Export module items for registration.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyCommodityOption>()?;
    Ok(())
}
