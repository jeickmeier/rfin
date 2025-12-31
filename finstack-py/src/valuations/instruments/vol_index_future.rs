//! Python bindings for VolatilityIndexFuture.

use crate::core::dates::utils::py_to_date;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::ir_future::Position;
use finstack_valuations::instruments::equity::vol_index_future::{
    VolIndexContractSpecs, VolatilityIndexFuture,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_position(label: Option<&str>) -> PyResult<Position> {
    match label {
        None => Ok(Position::Long),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Volatility index future wrapper (e.g., VIX futures).
///
/// Parameters
/// ----------
/// instrument_id : str
///     Unique identifier for the instrument.
/// notional : Money
///     Notional amount (e.g., $100,000 USD).
/// quoted_price : float
///     Quoted future price (e.g., 18.50 for VIX at 18.50).
/// expiry : date
///     Expiry date of the future.
/// discount_curve : str
///     ID of the discount curve for NPV calculations.
/// vol_index_curve : str
///     ID of the volatility index curve for forward levels.
/// position : str, optional
///     Position type: "long" (default) or "short".
/// multiplier : float, optional
///     Contract multiplier (default: 1000 for VIX).
/// tick_size : float, optional
///     Minimum price movement (default: 0.05).
/// tick_value : float, optional
///     Dollar value per tick (default: 50).
/// index_id : str, optional
///     Index identifier (default: "VIX").
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "VolatilityIndexFuture",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyVolatilityIndexFuture {
    pub(crate) inner: VolatilityIndexFuture,
}

impl PyVolatilityIndexFuture {
    pub(crate) fn new(inner: VolatilityIndexFuture) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolatilityIndexFuture {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            quoted_price,
            expiry,
            discount_curve,
            vol_index_curve,
            *,
            position=None,
            multiplier=1000.0,
            tick_size=0.05,
            tick_value=50.0,
            index_id="VIX"
        ),
        text_signature = "(cls, instrument_id, notional, quoted_price, expiry, discount_curve, vol_index_curve, /, *, position='long', multiplier=1000.0, tick_size=0.05, tick_value=50.0, index_id='VIX')"
    )]
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        quoted_price: f64,
        expiry: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        vol_index_curve: Bound<'_, PyAny>,
        position: Option<&str>,
        multiplier: Option<f64>,
        tick_size: Option<f64>,
        tick_value: Option<f64>,
        index_id: Option<&str>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let notional_money = extract_money(&notional).context("notional")?;
        let expiry_date = py_to_date(&expiry).context("expiry")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let vol_index_curve_id = CurveId::new(
            vol_index_curve
                .extract::<&str>()
                .context("vol_index_curve")?,
        );
        let position_value = parse_position(position).context("position")?;

        let specs = VolIndexContractSpecs {
            multiplier: multiplier.unwrap_or(1000.0),
            tick_size: tick_size.unwrap_or(0.05),
            tick_value: tick_value.unwrap_or(50.0),
            index_id: index_id.unwrap_or("VIX").to_string(),
        };

        let future = VolatilityIndexFuture::builder()
            .id(id)
            .notional(notional_money)
            .quoted_price(quoted_price)
            .expiry_date(expiry_date)
            .discount_curve_id(discount_curve_id)
            .vol_index_curve_id(vol_index_curve_id)
            .position(position_value)
            .contract_specs(specs)
            .attributes(Default::default())
            .build()
            .map_err(core_to_py)?;

        Ok(Self::new(future))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn quoted_price(&self) -> f64 {
        self.inner.quoted_price
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::VolatilityIndexFuture)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "VolatilityIndexFuture(id='{}', price={:.2})",
            self.inner.id, self.inner.quoted_price
        ))
    }
}

impl fmt::Display for PyVolatilityIndexFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VolatilityIndexFuture({}, price={:.2})",
            self.inner.id, self.inner.quoted_price
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyVolatilityIndexFuture>()?;
    Ok(vec!["VolatilityIndexFuture"])
}
