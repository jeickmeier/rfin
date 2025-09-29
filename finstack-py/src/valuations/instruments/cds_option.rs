// use crate::core::error::core_to_py; // not used in this module currently
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_valuations::instruments::cds_option::parameters::CdsOptionParams;
use finstack_valuations::instruments::cds_option::CdsOption;
use finstack_valuations::instruments::common::parameters::{CreditParams, OptionType};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn leak_str(value: &str) -> &'static str {
    Box::leak(value.to_string().into_boxed_str())
}

fn parse_option_type(label: Option<&str>) -> PyResult<OptionType> {
    match label {
        None => Ok(OptionType::Call),
        Some(s) => s
            .parse()
            .map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Option on CDS spread with simplified constructor.
///
/// Examples:
///     >>> opt = CdsOption.create(
///     ...     "opt_xyz",
///     ...     Money("USD", 5_000_000),
///     ...     150.0,
///     ...     date(2024, 6, 20),
///     ...     date(2029, 6, 20),
///     ...     "usd_discount",
///     ...     "xyz_credit",
///     ...     "cds_vol_surface"
///     ... )
///     >>> opt.strike_spread_bp
///     150.0
#[pyclass(module = "finstack.valuations.instruments", name = "CdsOption", frozen)]
#[derive(Clone, Debug)]
pub struct PyCdsOption {
    pub(crate) inner: CdsOption,
}

impl PyCdsOption {
    pub(crate) fn new(inner: CdsOption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCdsOption {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            strike_spread_bp,
            expiry,
            cds_maturity,
            discount_curve,
            credit_curve,
            vol_surface,
            *,
            option_type="call",
            recovery_rate=0.4,
            underlying_is_index=false,
            index_factor=None,
            forward_adjust_bp=0.0
        ),
        text_signature = "(cls, instrument_id, notional, strike_spread_bp, expiry, cds_maturity, discount_curve, credit_curve, vol_surface, /, *, option_type='call', recovery_rate=0.4, underlying_is_index=False, index_factor=None, forward_adjust_bp=0.0)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a CDS option referencing a standard CDS contract.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     strike_spread_bp: Option strike spread in basis points.
    ///     expiry: Option expiry date.
    ///     cds_maturity: Maturity date of the underlying CDS.
    ///     discount_curve: Discount curve identifier.
    ///     credit_curve: Credit curve identifier for the reference entity or index.
    ///     vol_surface: Volatility surface identifier for pricing.
    ///     option_type: Optional label indicating call/put (buy/sell protection).
    ///     recovery_rate: Optional recovery rate assumption.
    ///     underlying_is_index: Optional flag to treat the underlying as an index.
    ///     index_factor: Optional outstanding factor when pricing index options.
    ///     forward_adjust_bp: Optional forward spread adjustment in basis points.
    ///
    /// Returns:
    ///     CdsOption: Configured CDS option instrument.
    ///
    /// Raises:
    ///     ValueError: If labels cannot be parsed or recovery rate lies outside [0, 1].
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        strike_spread_bp: f64,
        expiry: Bound<'_, PyAny>,
        cds_maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        credit_curve: Bound<'_, PyAny>,
        vol_surface: &str,
        option_type: Option<&str>,
        recovery_rate: Option<f64>,
        underlying_is_index: Option<bool>,
        index_factor: Option<f64>,
        forward_adjust_bp: Option<f64>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let notional_money = extract_money(&notional)?;
        let expiry_date = py_to_date(&expiry)?;
        let cds_maturity_date = py_to_date(&cds_maturity)?;
        let discount = extract_curve_id(&discount_curve)?;
        let credit = extract_curve_id(&credit_curve)?;
        let option_type_value = parse_option_type(option_type)?;
        let recovery = recovery_rate.unwrap_or(0.40);
        if !(0.0..=1.0).contains(&recovery) {
            return Err(PyValueError::new_err(
                "recovery_rate must be between 0 and 1",
            ));
        }

        let mut option_params = CdsOptionParams::new(
            strike_spread_bp,
            expiry_date,
            cds_maturity_date,
            notional_money,
            option_type_value,
        );
        if underlying_is_index.unwrap_or(false) {
            let factor = index_factor.unwrap_or(1.0);
            option_params = option_params.as_index(factor);
        }
        if let Some(adj) = forward_adjust_bp {
            option_params = option_params.with_forward_spread_adjust_bp(adj);
        }

        let credit_params = CreditParams::new("CDS_OPTION", recovery, credit.clone());
        let option = CdsOption::new(
            id,
            &option_params,
            &credit_params,
            leak_str(discount.as_str()),
            leak_str(vol_surface),
        );
        Ok(Self::new(option))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier for the CDS option.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Notional amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Strike spread in basis points.
    ///
    /// Returns:
    ///     float: Strike spread for the option.
    #[getter]
    fn strike_spread_bp(&self) -> f64 {
        self.inner.strike_spread_bp
    }

    /// Option expiry date.
    ///
    /// Returns:
    ///     datetime.date: Expiry converted to Python.
    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.expiry)
    }

    /// Maturity date of the underlying CDS.
    ///
    /// Returns:
    ///     datetime.date: Underlying maturity converted to Python.
    #[getter]
    fn cds_maturity(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.cds_maturity)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve used for valuation.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    /// Credit curve identifier.
    ///
    /// Returns:
    ///     str: Hazard curve for the reference entity or index.
    #[getter]
    fn credit_curve(&self) -> String {
        self.inner.credit_id.as_str().to_string()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.CDS_OPTION``.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CDSOption)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "CdsOption(id='{}', strike_bp={:.1}, type='{}')",
            self.inner.id,
            self.inner.strike_spread_bp,
            match self.inner.option_type {
                OptionType::Call => "call",
                OptionType::Put => "put",
            }
        ))
    }
}

impl fmt::Display for PyCdsOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CdsOption({}, strike_bp={:.1})",
            self.inner.id, self.inner.strike_spread_bp
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCdsOption>()?;
    Ok(vec!["CdsOption"])
}
