use crate::core::dates::utils::py_to_date;
use crate::core::market_data::PyMarketContext;
use crate::errors::core_to_py;
use crate::valuations::common::{pricing_error_to_py, ModelKeyArg, PyPricerKey};
use crate::valuations::instruments::{extract_instrument, InstrumentHandle};
use crate::valuations::metrics::MetricIdArg;
use crate::valuations::results::PyValuationResult;
use finstack_valuations::instruments::bond::metrics::price_yield_spread::asw::{
    asw_market_with_forward, asw_par_with_forward,
};
use finstack_valuations::instruments::{build_with_metrics_dyn, instrument_to_arc};
use finstack_valuations::metrics::MetricId;
use finstack_valuations::pricer::{create_standard_registry, PricerRegistry};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use rayon::prelude::*;
use std::sync::Arc;

/// Default pricing date used when no explicit `as_of` parameter is provided.
///
/// This is a placeholder value that ensures deterministic pricing when the caller
/// doesn't specify a valuation date. In production use, callers should provide
/// an explicit `as_of` date that matches their market data.
fn default_pricing_date() -> PyResult<finstack_core::dates::Date> {
    finstack_core::dates::create_date(2024, time::Month::January, 1)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Registry dispatching (instrument, model) pairs to pricing engines.
///
/// Examples:
///     >>> registry = create_standard_registry()
///     >>> result = registry.price(bond, "discounting", market)
///     >>> result.present_value
#[pyclass(module = "finstack.valuations.pricer", name = "PricerRegistry", frozen)]
#[derive(Clone, Default)]
pub struct PyPricerRegistry {
    pub(crate) inner: Arc<PricerRegistry>,
}

impl PyPricerRegistry {
    pub(crate) fn new(inner: PricerRegistry) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyPricerRegistry {
    #[new]
    #[pyo3(text_signature = "()")]
    /// Create an empty registry instance.
    ///
    /// Returns:
    ///     PricerRegistry: Registry without any registered engines.
    ///
    /// Examples:
    ///     >>> empty = PricerRegistry()
    ///     >>> list(empty.__dict__.keys())
    ///     []
    fn ctor() -> Self {
        Self::new(PricerRegistry::new())
    }

    #[pyo3(signature = (instrument, model, market, as_of=None), text_signature = "(self, instrument, model, market, as_of=None)")]
    /// Price an instrument given a model key and market data.
    ///
    /// Args:
    ///     instrument: Instrument instance created from ``finstack.valuations.instruments``.
    ///     model: Pricing model key or its snake-case label.
    ///     market: Market context supplying curves, spreads, and FX data.
    ///     as_of: Optional valuation date (datetime.date). If not provided, uses 2024-01-01 as default.
    ///
    /// Returns:
    ///     ValuationResult: Envelope containing PV, measures, and metadata.
    ///
    /// Raises:
    ///     ValueError: If the instrument or model cannot be interpreted.
    ///     RuntimeError: If pricing fails in the underlying engine.
    ///
    /// Examples:
    ///     >>> registry = create_standard_registry()
    ///     >>> result = registry.price(bond, "discounting", market)
    ///     >>> result.present_value.amount
    ///     123.45
    ///     >>> # With explicit as_of date
    ///     >>> from datetime import date
    ///     >>> result = registry.price(bond, "discounting", market, as_of=date(2024, 6, 15))
    fn price(
        &self,
        py: Python<'_>,
        instrument: Bound<'_, PyAny>,
        model: Bound<'_, PyAny>,
        market: &PyMarketContext,
        as_of: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyValuationResult> {
        let InstrumentHandle {
            instrument: inst, ..
        } = extract_instrument(&instrument)?;
        let ModelKeyArg(model_key) = model.extract()?;

        let as_of_date = match as_of {
            Some(date) => py_to_date(&date)?,
            None => default_pricing_date()?,
        };

        // Release GIL for compute-heavy Rust pricing operation
        py.detach(|| {
            self.inner
                .price_with_registry(inst.as_ref(), model_key, &market.inner, as_of_date)
                .map(PyValuationResult::new)
                .map_err(pricing_error_to_py)
        })
    }

    #[pyo3(signature = (instruments, model, market, as_of=None), text_signature = "(self, instruments, model, market, as_of=None)")]
    /// Price a batch of instruments in parallel.
    ///
    /// Args:
    ///     instruments: List of instruments to price.
    ///     model: Pricing model key or name.
    ///     market: Market context.
    ///     as_of: Optional valuation date.
    ///
    /// Returns:
    ///     list[ValuationResult]: List of results in the same order as instruments.
    fn price_batch(
        &self,
        py: Python<'_>,
        instruments: Vec<Bound<'_, PyAny>>,
        model: Bound<'_, PyAny>,
        market: &PyMarketContext,
        as_of: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Vec<PyValuationResult>> {
        let ModelKeyArg(model_key) = model.extract()?;

        let as_of_date = match as_of {
            Some(date) => py_to_date(&date)?,
            None => default_pricing_date()?,
        };

        // Extract all instruments to Rust types (InstrumentHandle)
        // This must be done while holding GIL
        let mut handles = Vec::with_capacity(instruments.len());
        for inst in instruments {
            let handle = extract_instrument(&inst)?;
            handles.push(handle);
        }

        // Release GIL and process in parallel
        let results: Result<Vec<_>, _> = py.detach(|| {
            handles
                .par_iter()
                .map(|handle| {
                    self.inner.price_with_registry(
                        handle.instrument.as_ref(),
                        model_key,
                        &market.inner,
                        as_of_date,
                    )
                })
                .collect()
        });

        match results {
            Ok(vec) => Ok(vec.into_iter().map(PyValuationResult::new).collect()),
            Err(e) => Err(pricing_error_to_py(e)),
        }
    }

    #[pyo3(signature = (instrument, model, market, metrics, as_of=None), text_signature = "(self, instrument, model, market, metrics, as_of=None)")]
    /// Price an instrument and compute the requested metrics.
    ///
    /// Args:
    ///     instrument: Instrument instance created from the bindings.
    ///     model: Pricing model key or name.
    ///     market: Market context with the necessary curve data.
    ///     metrics: Iterable of metric identifiers or names to evaluate.
    ///     as_of: Optional valuation date (datetime.date). If not provided, uses 2024-01-01 as default.
    ///
    /// Returns:
    ///     ValuationResult: Pricing result enriched with computed metrics.
    ///
    /// Raises:
    ///     ValueError: If any metric identifier is invalid.
    ///     RuntimeError: If pricing or metric calculation fails.
    ///
    /// Examples:
    ///     >>> registry = create_standard_registry()
    ///     >>> result = registry.price_with_metrics(bond, "discounting", market, ["dv01"])
    ///     >>> result.metrics["dv01"].value
    ///     -415.2
    fn price_with_metrics(
        &self,
        py: Python<'_>,
        instrument: Bound<'_, PyAny>,
        model: Bound<'_, PyAny>,
        market: &PyMarketContext,
        metrics: Vec<Bound<'_, PyAny>>,
        as_of: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyValuationResult> {
        let InstrumentHandle {
            instrument: inst, ..
        } = extract_instrument(&instrument)?;
        let ModelKeyArg(model_key) = model.extract()?;

        // Parse metrics
        let mut metric_ids: Vec<MetricId> = Vec::with_capacity(metrics.len());
        for m in metrics {
            let MetricIdArg(id) = m.extract()?;
            metric_ids.push(id);
        }

        let as_of_date = match as_of {
            Some(date) => py_to_date(&date)?,
            None => default_pricing_date()?,
        };

        // Release GIL for compute-heavy pricing and metric calculation
        py.detach(|| {
            // Base price via registry to derive as_of deterministically per pricer
            let base = self
                .inner
                .price_with_registry(inst.as_ref(), model_key, &market.inner, as_of_date)
                .map_err(pricing_error_to_py)?;

            // Compute requested metrics using helper
            let with_metrics = build_with_metrics_dyn(
                instrument_to_arc(inst.as_ref()),
                Arc::new(market.inner.clone()),
                base.as_of,
                base.value,
                &metric_ids,
                None,
            )
            .map_err(core_to_py)?;

            Ok(PyValuationResult::new(with_metrics))
        })
    }

    #[pyo3(
        signature = (bond, market, forward_curve, float_margin_bp, dirty_price_ccy=None),
        text_signature = "(self, bond, market, forward_curve, float_margin_bp, dirty_price_ccy=None)"
    )]
    /// Compute par and market asset swap spreads using a forward curve.
    ///
    /// Args:
    ///     bond: Bond instrument previously constructed in the bindings.
    ///     market: Market context providing discount curves.
    ///     forward_curve: Identifier for the forward curve.
    ///     float_margin_bp: Floating margin in basis points.
    ///     dirty_price_ccy: Dirty market price expressed in currency.
    ///         This is **required** for the market ASW leg. To interpret the
    ///         market spread relative to par, explicitly pass the par notional
    ///         amount (e.g., ``bond.notional.amount``). If omitted, this binding
    ///         raises a ValueError instead of silently assuming a par price.
    ///
    /// Returns:
    ///     tuple[float, float]: Par and market asset swap spreads in basis points.
    ///
    /// Raises:
    ///     TypeError: If ``bond`` is not a bond instrument.
    ///     ValueError: If ``dirty_price_ccy`` is omitted.
    ///     RuntimeError: If the underlying calculation fails.
    ///
    /// Examples:
    ///     >>> registry = create_standard_registry()
    ///     >>> registry.asw_forward(bond, market, "usd_libor_3m", 25.0, bond.notional.amount)
    ///     (23.4, 27.1)
    fn asw_forward(
        &self,
        bond: Bound<'_, PyAny>,
        market: &PyMarketContext,
        forward_curve: &str,
        float_margin_bp: f64,
        dirty_price_ccy: Option<f64>,
    ) -> PyResult<(f64, f64)> {
        let dirty_price_ccy = dirty_price_ccy.ok_or_else(|| {
            PyValueError::new_err(
                "dirty_price_ccy is required to compute the market ASW leg; \
pass an explicit dirty price (e.g. 1.0125 * bond.notional.amount)",
            )
        })?;
        let InstrumentHandle {
            instrument: inst, ..
        } = extract_instrument(&bond)?;
        let bond_ref = inst
            .as_ref()
            .as_any()
            .downcast_ref::<finstack_valuations::instruments::bond::Bond>()
            .ok_or_else(|| {
                pricing_error_to_py(finstack_valuations::pricer::PricingError::TypeMismatch {
                    expected: finstack_valuations::pricer::InstrumentType::Bond,
                    got: inst.as_ref().key(),
                })
            })?;

        let disc = market
            .inner
            .get_discount_ref(bond_ref.discount_curve_id.as_str())
            .map_err(core_to_py)?;
        let as_of = disc.base_date();

        let par = asw_par_with_forward(
            bond_ref,
            &market.inner,
            as_of,
            forward_curve,
            float_margin_bp,
        )
        .map_err(core_to_py)?;
        let mkt = asw_market_with_forward(
            bond_ref,
            &market.inner,
            as_of,
            forward_curve,
            float_margin_bp,
            Some(dirty_price_ccy),
        )
        .map_err(core_to_py)?;
        Ok((par, mkt))
    }

    #[pyo3(text_signature = "(self, instrument, model)")]
    /// Convenience accessor returning the internal dispatch key.
    ///
    /// Args:
    ///     instrument: Instrument instance or instrument label.
    ///     model: Model key or snake-case label.
    ///
    /// Returns:
    ///     PricerKey: Key used internally to resolve engines.
    ///
    /// Raises:
    ///     ValueError: If the arguments cannot be converted.
    ///
    /// Examples:
    ///     >>> registry = create_standard_registry()
    ///     >>> registry.key(bond, "discounting").instrument
    ///     InstrumentType.BOND
    fn key(&self, instrument: Bound<'_, PyAny>, model: Bound<'_, PyAny>) -> PyResult<PyPricerKey> {
        let handle = extract_instrument(&instrument)?;
        let ModelKeyArg(model_key) = model.extract()?;
        Ok(PyPricerKey {
            inner: finstack_valuations::pricer::PricerKey::new(handle.instrument_type, model_key),
        })
    }

    #[pyo3(text_signature = "(self)")]
    /// Clone the registry for use across threads.
    ///
    /// Returns:
    ///     PricerRegistry: A shallow clone sharing the same (immutable) registry.
    ///
    /// Examples:
    ///     >>> registry = create_standard_registry()
    ///     >>> cloned = registry.clone()
    ///     >>> isinstance(cloned, PricerRegistry)
    ///     True
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Create a registry populated with all standard finstack pricers.
///
/// Returns:
///     PricerRegistry: Registry with all built-in pricers loaded.
///
/// Examples:
///     >>> registry = create_standard_registry()
///     >>> registry.price(bond, "discounting", market)
///     <ValuationResult ...>
#[pyfunction(name = "create_standard_registry")]
fn create_standard_registry_py() -> PyResult<PyPricerRegistry> {
    Ok(PyPricerRegistry::new(create_standard_registry()))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "pricer")?;
    module.setattr(
        "__doc__",
        "Pricer registry bridging instruments and pricing models to valuation engines.",
    )?;
    module.add_class::<PyPricerRegistry>()?;
    module.add_function(pyo3::wrap_pyfunction!(
        create_standard_registry_py,
        &module
    )?)?;
    let exports = ["PricerRegistry", "create_standard_registry"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
