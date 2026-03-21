use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::market_context_from_value;
use crate::core::market_data::PyMarketContext;
use crate::errors::core_to_py;
use crate::valuations::common::{pricing_error_to_py, ModelKeyArg, PyPricerKey};
use crate::valuations::instruments::{
    extract_instrument, extract_instrument_or_payload, InstrumentHandle,
};
use crate::valuations::metrics::MetricIdArg;
use crate::valuations::results::PyValuationResult;
use finstack_valuations::instruments::fixed_income::bond::{
    asw_market_with_forward, asw_par_with_forward,
};
use finstack_valuations::metrics::MetricId;
use finstack_valuations::pricer::{
    shared_standard_registry, ModelKey, PricerRegistry, PricingError,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyIterator, PyList, PyModule};
use pyo3::Bound;
use rayon::prelude::*;
use std::sync::Arc;
use time::Month;

/// Registry dispatching (instrument, model) pairs to pricing engines.
///
/// Examples:
///     >>> registry = standard_registry()
///     >>> result = registry.get_price(bond, "discounting", market)
///     >>> result.present_value
#[pyclass(
    module = "finstack.valuations.pricer",
    name = "PricerRegistry",
    frozen,
    from_py_object
)]
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

    pub(crate) fn from_arc(inner: Arc<PricerRegistry>) -> Self {
        Self { inner }
    }
}

fn extract_metric_ids(metrics: Bound<'_, PyAny>) -> PyResult<Vec<MetricId>> {
    let iter = PyIterator::from_object(&metrics)
        .map_err(|_| PyValueError::new_err("metrics must be an iterable of metric identifiers"))?;
    let mut metric_ids = Vec::new();
    for item in iter {
        let MetricIdArg(id) = item?.extract()?;
        metric_ids.push(id);
    }
    Ok(metric_ids)
}

fn parse_metrics_request(
    as_of: Bound<'_, PyAny>,
    metrics: Option<Bound<'_, PyAny>>,
) -> PyResult<(finstack_core::dates::Date, Vec<MetricId>)> {
    let metrics = metrics.ok_or_else(|| {
        PyValueError::new_err(
            "metrics are required; call price_with_metrics(..., as_of, metrics=[...]) \
or use the legacy positional order price_with_metrics(..., metrics, as_of)",
        )
    })?;

    if let Ok(as_of_date) = resolve_as_of_date(&as_of) {
        return Ok((as_of_date, extract_metric_ids(metrics)?));
    }

    let metric_ids = extract_metric_ids(as_of)?;
    let as_of_date = resolve_as_of_date(&metrics).map_err(|_| {
        PyValueError::new_err(
            "expected (instrument, model, market, as_of, metrics=...) or \
(instrument, model, market, metrics, as_of)",
        )
    })?;
    Ok((as_of_date, metric_ids))
}

fn resolve_model_key(model: Option<Bound<'_, PyAny>>) -> PyResult<ModelKey> {
    match model {
        Some(model) => {
            let ModelKeyArg(model_key) = model.extract()?;
            Ok(model_key)
        }
        None => Ok(ModelKey::Discounting),
    }
}

fn resolve_as_of_date(value: &Bound<'_, PyAny>) -> PyResult<finstack_core::dates::Date> {
    if let Ok(as_of_date) = py_to_date(value) {
        return Ok(as_of_date);
    }

    if let Ok(date_str) = value.extract::<&str>() {
        let mut parts = date_str.split('-');
        let year = parts
            .next()
            .ok_or_else(|| PyValueError::new_err("as_of must be ISO date YYYY-MM-DD"))?
            .parse::<i32>()
            .map_err(|_| PyValueError::new_err("as_of must be ISO date YYYY-MM-DD"))?;
        let month = parts
            .next()
            .ok_or_else(|| PyValueError::new_err("as_of must be ISO date YYYY-MM-DD"))?
            .parse::<u8>()
            .map_err(|_| PyValueError::new_err("as_of must be ISO date YYYY-MM-DD"))?;
        let day = parts
            .next()
            .ok_or_else(|| PyValueError::new_err("as_of must be ISO date YYYY-MM-DD"))?
            .parse::<u8>()
            .map_err(|_| PyValueError::new_err("as_of must be ISO date YYYY-MM-DD"))?;
        if parts.next().is_some() {
            return Err(PyValueError::new_err("as_of must be ISO date YYYY-MM-DD"));
        }
        let month = Month::try_from(month)
            .map_err(|_| PyValueError::new_err("as_of must be ISO date YYYY-MM-DD"))?;
        return time::Date::from_calendar_date(year, month, day)
            .map_err(|_| PyValueError::new_err("as_of must be ISO date YYYY-MM-DD"));
    }

    Err(PyValueError::new_err(
        "as_of must be datetime.date, datetime.datetime, or ISO date string",
    ))
}

fn extract_instrument_handles(
    instruments: Vec<Bound<'_, PyAny>>,
) -> PyResult<Vec<InstrumentHandle>> {
    let mut handles = Vec::with_capacity(instruments.len());
    for inst in instruments {
        handles.push(extract_instrument_or_payload(&inst)?);
    }
    Ok(handles)
}

fn enrich_pricing_error(
    err: PricingError,
    handle: &InstrumentHandle,
    model_key: ModelKey,
) -> PricingError {
    err.with_instrument_id(handle.instrument.id().to_string())
        .with_instrument_type(handle.instrument_type)
        .with_model(model_key)
}

fn price_instrument_handles(
    py: Python<'_>,
    registry: Arc<PricerRegistry>,
    handles: Vec<InstrumentHandle>,
    model_key: ModelKey,
    market: &PyMarketContext,
    as_of_date: finstack_core::dates::Date,
    metric_ids: &[MetricId],
) -> PyResult<Vec<PyValuationResult>> {
    let results: Result<Vec<_>, _> = py.detach(|| {
        handles
            .par_iter()
            .map(|handle| {
                let result = if metric_ids.is_empty() {
                    registry.price(
                        handle.instrument.as_ref(),
                        model_key,
                        &market.inner,
                        as_of_date,
                        None,
                    )
                } else {
                    registry.price_with_metrics(
                        handle.instrument.as_ref(),
                        model_key,
                        &market.inner,
                        as_of_date,
                        metric_ids,
                        finstack_valuations::instruments::PricingOptions::default(),
                    )
                };
                result.map_err(|err| enrich_pricing_error(err, handle, model_key))
            })
            .collect()
    });

    match results {
        Ok(vec) => Ok(vec.into_iter().map(PyValuationResult::new).collect()),
        Err(e) => Err(pricing_error_to_py(e)),
    }
}

#[pyfunction]
#[pyo3(
    signature = (instruments, market, as_of, metrics=None, model=None, registry=None, return_dicts=false),
    text_signature = "(instruments, market, as_of, metrics=None, model='discounting', registry=None, return_dicts=False)"
)]
/// Price a list of instruments using a single service-friendly helper.
///
/// Args:
///     instruments: List of instrument instances, instrument dictionaries, or
///         instrument JSON payloads to price.
///     market: Market context object, market dictionary, or market JSON payload.
///     as_of: Valuation date (datetime.date) or ISO ``YYYY-MM-DD`` string.
///     metrics: Optional iterable of metric identifiers to compute for every instrument.
///     model: Optional pricing model key or name. Defaults to ``"discounting"``.
///     registry: Optional registry to use. Defaults to the shared standard registry.
///     return_dicts: When true, return ``list[dict]`` via ``ValuationResult.to_dict()``.
///
/// Returns:
///     list[ValuationResult] | list[dict]: Results in the same order as *instruments*.
fn price_portfolio<'py>(
    py: Python<'py>,
    instruments: Vec<Bound<'py, PyAny>>,
    market: Bound<'py, PyAny>,
    as_of: Bound<'py, PyAny>,
    metrics: Option<Bound<'py, PyAny>>,
    model: Option<Bound<'py, PyAny>>,
    registry: Option<PyRef<'py, PyPricerRegistry>>,
    return_dicts: bool,
) -> PyResult<Py<PyAny>> {
    let as_of_date = resolve_as_of_date(&as_of)?;
    let model_key = resolve_model_key(model)?;
    let metric_ids = match metrics {
        Some(metrics) => extract_metric_ids(metrics)?,
        None => Vec::new(),
    };
    let handles = extract_instrument_handles(instruments)?;
    let market = market_context_from_value(&market)?;
    let registry_inner = registry
        .map(|registry| registry.inner.clone())
        .unwrap_or_else(shared_standard_registry);
    let results = price_instrument_handles(
        py,
        registry_inner,
        handles,
        model_key,
        &market,
        as_of_date,
        &metric_ids,
    )?;

    if return_dicts {
        let dicts = results
            .iter()
            .map(|result| result.to_dict_py(py))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(PyList::new(py, dicts)?.into())
    } else {
        Ok(PyList::new(py, results)?.into())
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

    #[pyo3(signature = (instrument, model, market, as_of), text_signature = "(self, instrument, model, market, as_of)")]
    /// Price an instrument given a model key and market data.
    ///
    /// Args:
    ///     instrument: Instrument instance created from ``finstack.valuations.instruments``.
    ///     model: Pricing model key or its snake-case label.
    ///     market: Market context supplying curves, spreads, and FX data.
    ///     as_of: Valuation date (datetime.date).
    ///
    /// Returns:
    ///     ValuationResult: Envelope containing PV, measures, and metadata.
    ///
    /// Raises:
    ///     ValueError: If the instrument or model cannot be interpreted.
    ///     RuntimeError: If pricing fails in the underlying engine.
    ///
    /// Examples:
    ///     >>> from datetime import date
    ///     >>> registry = standard_registry()
    ///     >>> result = registry.price(bond, "discounting", market, date(2024, 6, 15))
    fn price(
        &self,
        py: Python<'_>,
        instrument: Bound<'_, PyAny>,
        model: Bound<'_, PyAny>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyValuationResult> {
        let handle = extract_instrument(&instrument)?;
        let ModelKeyArg(model_key) = model.extract()?;
        let as_of_date = resolve_as_of_date(&as_of)?;

        py.detach(|| {
            self.inner
                .price(
                    handle.instrument.as_ref(),
                    model_key,
                    &market.inner,
                    as_of_date,
                    None,
                )
                .map_err(|err| enrich_pricing_error(err, &handle, model_key))
                .map(PyValuationResult::new)
                .map_err(pricing_error_to_py)
        })
    }

    #[pyo3(signature = (instrument, model, market, as_of), text_signature = "(self, instrument, model, market, as_of)")]
    /// Backward-compatible alias for :meth:`price`.
    ///
    /// Older examples and tests still call ``get_price(...)``. Keep this alias so
    /// those call sites continue to work while ``price(...)`` remains the primary
    /// API name.
    fn get_price(
        &self,
        py: Python<'_>,
        instrument: Bound<'_, PyAny>,
        model: Bound<'_, PyAny>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyValuationResult> {
        self.price(py, instrument, model, market, as_of)
    }

    #[pyo3(signature = (instruments, model, market, as_of), text_signature = "(self, instruments, model, market, as_of)")]
    /// Price a batch of instruments in parallel.
    ///
    /// Args:
    ///     instruments: List of instruments to price.
    ///     model: Pricing model key or name.
    ///     market: Market context.
    ///     as_of: Valuation date (datetime.date).
    ///
    /// Returns:
    ///     list[ValuationResult]: List of results in the same order as instruments.
    fn price_batch(
        &self,
        py: Python<'_>,
        instruments: Vec<Bound<'_, PyAny>>,
        model: Bound<'_, PyAny>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<Vec<PyValuationResult>> {
        let ModelKeyArg(model_key) = model.extract()?;
        let as_of_date = resolve_as_of_date(&as_of)?;
        let handles = extract_instrument_handles(instruments)?;
        price_instrument_handles(
            py,
            self.inner.clone(),
            handles,
            model_key,
            market,
            as_of_date,
            &[],
        )
    }

    #[pyo3(signature = (instrument, model, market, as_of, metrics=None), text_signature = "(self, instrument, model, market, as_of, metrics=None)")]
    /// Price an instrument and compute the requested metrics.
    ///
    /// Args:
    ///     instrument: Instrument instance created from the bindings.
    ///     model: Pricing model key or name.
    ///     market: Market context with the necessary curve data.
    ///     as_of: Valuation date (datetime.date).
    ///     metrics: Iterable of metric identifiers or names to evaluate.
    ///
    /// Returns:
    ///     ValuationResult: Pricing result enriched with computed metrics.
    ///
    /// Raises:
    ///     ValueError: If any metric identifier is invalid.
    ///     RuntimeError: If pricing or metric calculation fails.
    ///
    /// Examples:
    ///     >>> from datetime import date
    ///     >>> registry = standard_registry()
    ///     >>> result = registry.price_with_metrics(
    ///     ...     bond,
    ///     ...     "discounting",
    ///     ...     market,
    ///     ...     date(2024, 6, 15),
    ///     ...     metrics=["dv01"],
    ///     ... )
    ///     >>> result.metrics["dv01"].value
    ///     -415.2
    fn price_with_metrics(
        &self,
        py: Python<'_>,
        instrument: Bound<'_, PyAny>,
        model: Bound<'_, PyAny>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
        metrics: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyValuationResult> {
        let handle = extract_instrument(&instrument)?;
        let ModelKeyArg(model_key) = model.extract()?;
        let (as_of_date, metric_ids) = parse_metrics_request(as_of, metrics)?;

        py.detach(|| {
            self.inner
                .price_with_metrics(
                    handle.instrument.as_ref(),
                    model_key,
                    &market.inner,
                    as_of_date,
                    &metric_ids,
                    finstack_valuations::instruments::PricingOptions::default(),
                )
                .map_err(|err| enrich_pricing_error(err, &handle, model_key))
                .map(PyValuationResult::new)
                .map_err(pricing_error_to_py)
        })
    }

    #[pyo3(signature = (instruments, model, market, as_of, metrics=None), text_signature = "(self, instruments, model, market, as_of, metrics=None)")]
    /// Price a batch of instruments in parallel and compute requested metrics.
    ///
    /// The documented call shape is ``(instruments, model, market, as_of, metrics=...)``.
    /// The legacy positional order ``(..., metrics, as_of)`` remains supported for
    /// compatibility with older callers.
    fn price_batch_with_metrics(
        &self,
        py: Python<'_>,
        instruments: Vec<Bound<'_, PyAny>>,
        model: Bound<'_, PyAny>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
        metrics: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Vec<PyValuationResult>> {
        let ModelKeyArg(model_key) = model.extract()?;
        let (as_of_date, metric_ids) = parse_metrics_request(as_of, metrics)?;
        let handles = extract_instrument_handles(instruments)?;
        price_instrument_handles(
            py,
            self.inner.clone(),
            handles,
            model_key,
            market,
            as_of_date,
            &metric_ids,
        )
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
    ///     >>> registry = standard_registry()
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
            .downcast_ref::<finstack_valuations::instruments::fixed_income::bond::Bond>()
            .ok_or_else(|| {
                pricing_error_to_py(finstack_valuations::pricer::PricingError::TypeMismatch {
                    expected: finstack_valuations::pricer::InstrumentType::Bond,
                    got: inst.as_ref().key(),
                })
            })?;

        let disc = market
            .inner
            .get_discount(bond_ref.discount_curve_id.as_str())
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
    ///     >>> registry = standard_registry()
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
    ///     >>> registry = standard_registry()
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
///     >>> registry = standard_registry()
///     >>> registry.get_price(bond, "discounting", market)
///     <ValuationResult ...>
#[pyfunction(name = "standard_registry")]
fn standard_registry_py() -> PyResult<PyPricerRegistry> {
    Ok(PyPricerRegistry::from_arc(shared_standard_registry()))
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
    module.add_function(pyo3::wrap_pyfunction!(standard_registry_py, &module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(price_portfolio, &module)?)?;
    let exports = ["PricerRegistry", "standard_registry", "price_portfolio"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
