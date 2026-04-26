//! Core traits for the metrics framework.
//!
//! Defines the fundamental interfaces for implementing and using financial
//! metrics. The `MetricCalculator` trait enables custom metric implementations,
//! while `MetricContext` provides the execution environment with caching.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::structured_credit::TrancheCashflows;
use crate::metrics::risk::MarketHistory;
use crate::metrics::MetricId;
use crate::pricer::{ModelKey, PricerRegistry};
use finstack_core::cashflow::CashFlow;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::CurveId;

use finstack_core::config::FinstackConfig;
use std::sync::Arc;

/// Core trait for metric calculators.
///
/// Each calculator computes a single metric value based on the provided context.
/// Calculators can declare dependencies on other metrics for efficient computation
/// ordering and caching. Implement this trait to create custom financial metrics.
///
/// See unit tests and `examples/` for usage.
pub trait MetricCalculator: Send + Sync {
    /// Computes the metric value based on the provided context.
    ///
    /// This method should implement the core calculation logic for the metric.
    /// It can access cached results from `context.computed` for dependencies.
    ///
    /// # Arguments
    /// * `context` - Metric context containing instrument, market data, and cached results
    ///
    /// # Returns
    /// The computed metric value as a `Result<f64>`
    ///
    /// # Errors
    /// Returns an error if the metric cannot be computed due to missing data
    /// or invalid instrument configuration.
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64>;

    /// Lists metric IDs this calculator depends on.
    ///
    /// Dependencies will be computed first and made available via
    /// `context.computed`. The registry uses this to determine computation order.
    ///
    /// # Returns
    /// Slice of metric IDs that must be computed before this metric
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Resolver function type for dynamic bucket metric keys.
///
/// Allows callers to customize how per-bucket metrics are keyed.
/// Given a base metric ID (e.g., `MetricId::BucketedDv01`), a bucket label
/// (e.g., "1y"), and the instrument, return the final `MetricId` to store.
pub(crate) type BucketKeyResolverFn =
    dyn Fn(&MetricId, &str, &dyn Instrument) -> MetricId + Send + Sync;

/// Generic 2D structured metric container.
///
/// Rows and columns are labeled; values are a rectangular matrix of size
/// `rows.len() x cols.len()`.
#[derive(Debug, Clone)]
pub struct Structured2D {
    /// Row labels (e.g., expiries, tenors)
    pub rows: Vec<String>,
    /// Column labels (e.g., strikes, bumps)
    pub cols: Vec<String>,
    /// Matrix values; `values[r][c]` corresponds to `rows[r]`, `cols[c]`
    pub values: Vec<Vec<f64>>,
}

impl Structured2D {
    /// Validates that `values` is a rectangular matrix matching label sizes.
    pub fn validate_shape(&self) -> bool {
        if self.rows.is_empty() || self.cols.is_empty() {
            return false;
        }
        if self.values.len() != self.rows.len() {
            return false;
        }
        let expected_cols = self.cols.len();
        self.values.iter().all(|row| row.len() == expected_cols)
    }
}

/// Generic 3D structured metric container.
///
/// Axes A, B, C are labeled; values form a 3D tensor with sizes
/// `a.len() x b.len() x c.len()`.
#[derive(Debug, Clone)]
pub struct Structured3D {
    /// Axis A labels (e.g., expiries)
    pub a: Vec<String>,
    /// Axis B labels (e.g., tenors)
    pub b: Vec<String>,
    /// Axis C labels (e.g., strikes or vol buckets)
    pub c: Vec<String>,
    /// 3D tensor values; `values[i][j][k]` corresponds to `a[i]`, `b[j]`, `c[k]`
    pub values: Vec<Vec<Vec<f64>>>,
}

impl Structured3D {
    /// Validates that `values` matches axis sizes and is rectangular for each sub-dimension.
    pub fn validate_shape(&self) -> bool {
        if self.a.is_empty() || self.b.is_empty() || self.c.is_empty() {
            return false;
        }
        if self.values.len() != self.a.len() {
            return false;
        }
        let expected_b = self.b.len();
        let expected_c = self.c.len();
        for plane in &self.values {
            if plane.len() != expected_b {
                return false;
            }
            if !plane.iter().all(|row| row.len() == expected_c) {
                return false;
            }
        }
        true
    }
}

/// Context containing all data needed for metric calculations.
///
/// Provides access to the instrument, market data, base valuation,
/// and any previously computed metrics. Supports caching of intermediate
/// results like cashflows and discount factors to improve performance.
///
/// # Key Features
///
/// - **Instrument data**: Access to the instrument being valued
/// - **Market curves**: Discount and forward curves for calculations
/// - **Cached results**: Previously computed metrics for dependency resolution
/// - **Cashflow caching**: Optional caching of instrument cashflows
/// - **Metadata**: Discount curve ID and day count convention
pub struct MetricContext {
    /// The instrument being valued.
    pub instrument: Arc<dyn Instrument>,

    /// Market curves for discounting and forwarding.
    pub curves: Arc<finstack_core::market_data::context::MarketContext>,

    /// Optional market history for historical scenario revaluation (e.g., Historical VaR).
    ///
    /// This is intentionally **not** stored inside `finstack_core::MarketContext` to keep
    /// the core market container strongly typed and fully serializable.
    market_history: Option<Arc<MarketHistory>>,

    /// Pricing model to reuse for bump-and-reprice metrics.
    pricing_model: Option<ModelKey>,

    /// Pricer registry to reuse for bump-and-reprice metrics.
    pricer_registry: Option<Arc<PricerRegistry>>,

    /// Valuation date.
    pub as_of: Date,

    /// Base present value of the instrument.
    pub base_value: Money,

    /// Previously computed metrics (by ID).
    pub computed: finstack_core::HashMap<MetricId, f64>,

    /// Previously computed 1D bucketed metrics (by ID).
    ///
    /// Example: `MetricId::BucketedDv01` -> [("1m", v1), ("3m", v2), ...]
    pub computed_series: finstack_core::HashMap<MetricId, Vec<(String, f64)>>,

    /// Previously computed 2D structured metrics (by ID).
    ///
    /// Example: vega surface with rows=expiries, cols=strikes
    pub computed_matrix: finstack_core::HashMap<MetricId, Structured2D>,

    /// Previously computed 3D structured metrics (by ID).
    ///
    /// Example: 3D bucketed vegas (e.g., expiry x tenor x strike)
    pub computed_tensor3: finstack_core::HashMap<MetricId, Structured3D>,

    /// Cached cashflows for the instrument.
    pub cashflows: Option<Vec<(Date, Money)>>,

    /// Cached detailed cashflows with CFKind metadata.
    pub tagged_cashflows: Option<Vec<CashFlow>>,

    /// Tranche-level detailed cashflow results (for structured credit)
    pub detailed_tranche_cashflows: Option<TrancheCashflows>,

    /// Cached discount curve ID.
    pub discount_curve_id: Option<CurveId>,

    /// Cached day count convention.
    pub day_count: Option<DayCount>,

    /// Original notional amount for price calculations.
    ///
    /// For structured credit: typically pool original balance or tranche original balance.
    /// For bonds: face amount. For other instruments: principal amount.
    /// Used by price calculators to avoid instrument downcasts.
    pub notional: Option<Money>,

    /// Optional resolver to customize per-bucket metric keys.
    ///
    /// When set, bucketed metrics (e.g., DV01 by tenor) will use this resolver
    /// to produce `MetricId`s instead of default static keys.
    bucket_key_resolver: Option<Arc<BucketKeyResolverFn>>,
    /// Optional instrument-owned pricing inputs needed by specific metrics.
    instrument_overrides: Option<crate::instruments::InstrumentPricingOverrides>,

    /// Optional metric-only overrides to control risk calculations (e.g., bumps, theta horizon).
    metric_overrides: Option<crate::instruments::MetricPricingOverrides>,

    /// Optional scenario-only adjustments applied at the valuation boundary.
    scenario_overrides: Option<crate::instruments::ScenarioPricingOverrides>,

    /// Finstack configuration (tolerances + versioned extensions).
    ///
    /// This is used by metric calculators to resolve user-facing defaults
    /// (e.g., risk bump sizes) and to keep results reproducible.
    finstack_config: Arc<FinstackConfig>,
}

impl MetricContext {
    /// Returns a new [`Arc`] containing the default [`FinstackConfig`].
    #[inline]
    pub fn default_config() -> Arc<FinstackConfig> {
        Arc::new(FinstackConfig::default())
    }

    /// Creates a new metric context.
    ///
    /// # Arguments
    /// * `instrument` - The instrument to value
    /// * `curves` - Market curves for discounting and forwarding
    /// * `as_of` - Valuation date
    /// * `base_value` - Base present value of the instrument
    /// * `finstack_config` - Shared configuration controlling tolerances and feature flags
    ///
    /// See unit tests and `examples/` for usage.
    pub fn new(
        instrument: Arc<dyn Instrument>,
        curves: Arc<finstack_core::market_data::context::MarketContext>,
        as_of: Date,
        base_value: Money,
        finstack_config: Arc<FinstackConfig>,
    ) -> Self {
        Self {
            instrument,
            curves,
            market_history: None,
            pricing_model: None,
            pricer_registry: None,
            as_of,
            base_value,
            computed: finstack_core::HashMap::default(),
            computed_series: finstack_core::HashMap::default(),
            computed_matrix: finstack_core::HashMap::default(),
            computed_tensor3: finstack_core::HashMap::default(),
            cashflows: None,
            tagged_cashflows: None,
            detailed_tranche_cashflows: None,
            discount_curve_id: None,
            day_count: None,
            notional: None,
            bucket_key_resolver: None,
            instrument_overrides: None,
            metric_overrides: None,
            scenario_overrides: None,
            finstack_config,
        }
    }

    /// Access the finstack configuration associated with this context.
    #[inline]
    pub fn config(&self) -> &FinstackConfig {
        &self.finstack_config
    }

    /// Clone the shared finstack configuration.
    #[inline]
    pub fn config_arc(&self) -> Arc<FinstackConfig> {
        Arc::clone(&self.finstack_config)
    }

    /// Returns the metric-only overrides, if any.
    #[inline]
    pub(crate) fn get_metric_overrides(
        &self,
    ) -> Option<&crate::instruments::MetricPricingOverrides> {
        self.metric_overrides.as_ref()
    }

    /// Returns the instrument-owned pricing overrides, if any.
    #[inline]
    pub(crate) fn get_instrument_overrides(
        &self,
    ) -> Option<&crate::instruments::InstrumentPricingOverrides> {
        self.instrument_overrides.as_ref()
    }

    /// Returns a reference to the market history, if set.
    #[inline]
    pub(crate) fn get_market_history(&self) -> Option<&MarketHistory> {
        self.market_history.as_deref()
    }

    /// Clones the pricing dispatch pair (model + registry) for use in sub-contexts.
    #[inline]
    pub(crate) fn clone_pricer_dispatch(
        &self,
    ) -> (Option<ModelKey>, Option<Arc<PricerRegistry>>) {
        (self.pricing_model, self.pricer_registry.clone())
    }

    /// Attach market history to this context (used by Historical VaR metrics).
    pub fn with_market_history(mut self, history: Arc<MarketHistory>) -> Self {
        self.market_history = Some(history);
        self
    }

    /// Reuse a specific pricer registry/model pair for metric repricing.
    pub fn set_pricer_dispatch(
        &mut self,
        pricing_model: Option<ModelKey>,
        pricer_registry: Option<Arc<PricerRegistry>>,
    ) {
        self.pricing_model = pricing_model;
        self.pricer_registry = pricer_registry;
    }

    /// Set instrument-owned pricing inputs used by downstream calculators.
    pub fn set_instrument_overrides(
        &mut self,
        overrides: Option<crate::instruments::InstrumentPricingOverrides>,
    ) {
        self.instrument_overrides = overrides;
    }

    /// Set metric-only overrides used by downstream calculators.
    pub fn set_metric_overrides(
        &mut self,
        overrides: Option<crate::instruments::MetricPricingOverrides>,
    ) {
        self.metric_overrides = overrides;
    }

    /// Set scenario-only adjustments used by valuation helpers.
    pub fn set_scenario_overrides(
        &mut self,
        overrides: Option<crate::instruments::ScenarioPricingOverrides>,
    ) {
        self.scenario_overrides = overrides;
    }

    /// Value the instrument and apply any active scenario price shock.
    pub fn instrument_value_with_scenario(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let value = self.reprice_money(market, as_of)?;
        Ok(self
            .scenario_overrides
            .as_ref()
            .map_or(value, |overrides| overrides.apply_to_value(value)))
    }

    /// Reprice the context instrument using the active dispatch path.
    pub fn reprice_money(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        self.reprice_instrument_money(self.instrument.as_ref(), market, as_of)
    }

    /// Reprice the context instrument as a raw amount using the active dispatch path.
    pub fn reprice_raw(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        self.reprice_instrument_raw(self.instrument.as_ref(), market, as_of)
    }

    /// Reprice an arbitrary instrument using the active dispatch path.
    pub fn reprice_instrument_money(
        &self,
        instrument: &dyn Instrument,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        if let (Some(model), Some(registry)) = (self.pricing_model, self.pricer_registry.as_ref()) {
            let options = crate::instruments::PricingOptions::default().with_config(self.config());
            return Ok(registry
                .price_with_metrics(instrument, model, market, as_of, &[], options)?
                .value);
        }
        instrument.value(market, as_of)
    }

    /// Reprice an arbitrary instrument as a raw amount using the active dispatch path.
    pub fn reprice_instrument_raw(
        &self,
        instrument: &dyn Instrument,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        if let (Some(model), Some(registry)) = (self.pricing_model, self.pricer_registry.as_ref()) {
            return registry
                .price_raw(instrument, model, market, as_of)
                .map_err(Into::into);
        }
        instrument.value_raw(market, as_of)
    }

    /// Downcast the instrument to a specific concrete type.
    ///
    /// # Returns
    /// Reference to the concrete instrument type if the downcast succeeds
    ///
    /// # Errors
    /// Returns an error if the instrument is not of the expected type
    #[inline(never)] // Prevent inlining to reduce coverage metadata conflicts
    pub fn instrument_as<T: 'static>(&self) -> finstack_core::Result<&T> {
        self.instrument.as_any().downcast_ref::<T>().ok_or_else(|| {
            finstack_core::InputError::NotFound {
                id: format!(
                    "instrument downcast: expected {}, got {} (id={})",
                    std::any::type_name::<T>(),
                    self.instrument.key(),
                    self.instrument.id(),
                ),
            }
            .into()
        })
    }

    /// Store a 1D bucketed series under `base_metric_id` and optionally
    /// flatten into `computed` using a stable composite key per bucket.
    ///
    /// When a custom `bucket_key_resolver` is present, it is used to produce
    /// per-bucket keys; otherwise a default `base::bucket` composite key is used.
    pub fn store_bucketed_series<I, K>(&mut self, base_metric_id: MetricId, series: I)
    where
        I: IntoIterator<Item = (K, f64)>,
        K: Into<String>,
    {
        let collected: Vec<(String, f64)> =
            series.into_iter().map(|(k, v)| (k.into(), v)).collect();

        for (label, value) in &collected {
            let key = if let Some(resolver) = &self.bucket_key_resolver {
                resolver(&base_metric_id, label, self.instrument.as_ref())
            } else {
                Self::default_composite_key(&base_metric_id, &[label.as_str()])
            };
            self.computed.insert(key, *value);
        }

        self.computed_series.insert(base_metric_id, collected);
    }

    /// Store a 2D structured metric (rows x cols) under `base_metric_id` and
    /// flatten each cell into `computed` using stable composite keys
    /// `base::row::col`.
    pub fn store_matrix2d<I, J, RS, CS>(
        &mut self,
        base_metric_id: MetricId,
        rows: I,
        cols: J,
        values: Vec<Vec<f64>>,
    ) -> finstack_core::Result<()>
    where
        I: IntoIterator<Item = RS>,
        J: IntoIterator<Item = CS>,
        RS: Into<String>,
        CS: Into<String>,
    {
        let rows: Vec<String> = rows.into_iter().map(Into::into).collect();
        let cols: Vec<String> = cols.into_iter().map(Into::into).collect();
        let matrix = Structured2D { rows, cols, values };
        if !matrix.validate_shape() {
            return Err(finstack_core::InputError::Invalid.into());
        }
        for (r_idx, r_label) in matrix.rows.iter().enumerate() {
            for (c_idx, c_label) in matrix.cols.iter().enumerate() {
                let key = Self::default_composite_key(
                    &base_metric_id,
                    &[r_label.as_str(), c_label.as_str()],
                );
                self.computed.insert(key, matrix.values[r_idx][c_idx]);
            }
        }
        self.computed_matrix.insert(base_metric_id, matrix);
        Ok(())
    }

    /// Store a 3D structured metric (a x b x c) under `base_metric_id` and
    /// flatten each cell into `computed` using stable composite keys
    /// `base::a::b::c`.
    pub fn store_tensor3<IA, IB, IC, SA, SB, SC>(
        &mut self,
        base_metric_id: MetricId,
        a: IA,
        b: IB,
        c: IC,
        values: Vec<Vec<Vec<f64>>>,
    ) -> finstack_core::Result<()>
    where
        IA: IntoIterator<Item = SA>,
        IB: IntoIterator<Item = SB>,
        IC: IntoIterator<Item = SC>,
        SA: Into<String>,
        SB: Into<String>,
        SC: Into<String>,
    {
        let tensor = Structured3D {
            a: a.into_iter().map(Into::into).collect(),
            b: b.into_iter().map(Into::into).collect(),
            c: c.into_iter().map(Into::into).collect(),
            values,
        };
        if !tensor.validate_shape() {
            return Err(finstack_core::InputError::Invalid.into());
        }
        for (i, a_label) in tensor.a.iter().enumerate() {
            for (j, b_label) in tensor.b.iter().enumerate() {
                for (k, c_label) in tensor.c.iter().enumerate() {
                    let key = Self::default_composite_key(
                        &base_metric_id,
                        &[a_label.as_str(), b_label.as_str(), c_label.as_str()],
                    );
                    self.computed.insert(key, tensor.values[i][j][k]);
                }
            }
        }
        self.computed_tensor3.insert(base_metric_id, tensor);
        Ok(())
    }

    /// Retrieves a previously stored 1D bucketed series.
    pub fn get_series(&self, id: &MetricId) -> Option<&[(String, f64)]> {
        self.computed_series.get(id).map(|v| v.as_slice())
    }

    /// Retrieves a previously stored 2D structured metric.
    pub fn get_matrix2d(&self, id: &MetricId) -> Option<&Structured2D> {
        self.computed_matrix.get(id)
    }

    /// Retrieves a previously stored 3D structured metric.
    pub fn get_tensor3(&self, id: &MetricId) -> Option<&Structured3D> {
        self.computed_tensor3.get(id)
    }

    /// Builds a default composite key like `base::p1[::p2[::p3]]...` using sanitized labels.
    fn default_composite_key(base: &MetricId, parts: &[&str]) -> MetricId {
        // Calculate exact capacity needed
        let base_len = base.as_str().len();
        let separator_len = 2; // "::"
        let parts_len: usize = parts.iter().map(|p| p.len() + separator_len).sum();

        let mut key = String::with_capacity(base_len + parts_len);
        key.push_str(base.as_str());

        for p in parts {
            key.push_str("::");

            let start_len = key.len();
            let mut last_was_underscore = true; // Treat start as "underscore" to trim leading separators

            for ch in p.chars() {
                if ch.is_ascii_alphanumeric() {
                    key.push(ch.to_ascii_lowercase());
                    last_was_underscore = false;
                } else if !last_was_underscore {
                    key.push('_');
                    last_was_underscore = true;
                }
            }

            // Trim trailing underscore
            if last_was_underscore && key.len() > start_len {
                key.pop();
            }
        }
        MetricId::custom(key)
    }
}
