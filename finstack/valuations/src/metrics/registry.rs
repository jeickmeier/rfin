//! Metric registry and computation engine.
//!
//! Manages metric calculators with dependency resolution, caching, and batch
//! computation. The registry handles which metrics apply to which instrument
//! types and ensures efficient computation ordering.

use super::ids::MetricId;
use super::traits::{MetricCalculator, MetricContext};
use finstack_core::F;
use hashbrown::HashMap;
use std::sync::Arc;

/// Registry for metric calculators.
///
/// Manages metric calculators with dependency resolution, caching, and batch
/// computation. Also handles which metrics apply to which instrument types.
///
/// # Key Features
///
/// - **Calculator management**: Register and retrieve metric calculators
/// - **Dependency resolution**: Automatic computation ordering based on dependencies
/// - **Instrument applicability**: Metrics can be restricted to specific instrument types
/// - **Batch computation**: Compute multiple metrics efficiently
///
/// # Example
/// ```rust
/// use finstack_valuations::metrics::registry::MetricRegistry;
/// use finstack_valuations::metrics::ids::MetricId;
/// use finstack_valuations::metrics::traits::MetricCalculator;
/// use std::sync::Arc;
///
/// struct MyCalculator;
/// impl MetricCalculator for MyCalculator {
///     fn calculate(&self, _context: &mut finstack_valuations::metrics::traits::MetricContext) -> finstack_core::Result<f64> {
///         Ok(42.0)
///     }
/// }
///
/// let mut registry = MetricRegistry::new();
/// registry.register_metric(
///     MetricId::Ytm,
///     Arc::new(MyCalculator),
///     &["Bond"]
/// );
///
/// assert!(registry.has_metric(MetricId::Ytm));
/// ```
#[derive(Clone)]
pub struct MetricRegistry {
    calculators: HashMap<MetricId, Arc<dyn MetricCalculator>>,
    applicability: HashMap<MetricId, Vec<&'static str>>,
}

impl MetricRegistry {
    /// Creates a new empty registry.
    ///
    /// See unit tests and `examples/` for usage.
    pub fn new() -> Self {
        Self {
            calculators: HashMap::new(),
            applicability: HashMap::new(),
        }
    }
}

impl Default for MetricRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricRegistry {
    /// Registers a metric calculator with explicit ID and applicability.
    ///
    /// If a calculator with the same ID already exists, it will be replaced.
    /// The `applicable_to` parameter specifies which instrument types this metric
    /// applies to. An empty slice means it applies to all instruments.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the metric
    /// * `calculator` - Implementation of the metric calculation
    /// * `applicable_to` - Instrument types this metric applies to (empty = all)
    ///
    /// # Returns
    /// Mutable reference to self for method chaining
    ///
    /// See unit tests and `examples/` for usage.
    pub fn register_metric(
        &mut self,
        id: MetricId,
        calculator: Arc<dyn MetricCalculator>,
        applicable_to: &[&'static str],
    ) -> &mut Self {
        self.applicability
            .insert(id.clone(), applicable_to.to_vec());
        self.calculators.insert(id, calculator);
        self
    }

    /// Checks if a metric is registered.
    ///
    /// # Arguments
    /// * `id` - Metric ID to check
    ///
    /// # Returns
    /// `true` if the metric is registered, `false` otherwise
    ///
    /// See unit tests and `examples/` for usage.
    pub fn has_metric(&self, id: MetricId) -> bool {
        self.calculators.contains_key(&id)
    }

    /// Gets a list of all registered metric IDs.
    ///
    /// # Returns
    /// Vector of all registered metric IDs
    ///
    /// See unit tests and `examples/` for usage.
    pub fn available_metrics(&self) -> Vec<MetricId> {
        self.calculators.keys().cloned().collect()
    }

    /// Gets metrics applicable to a specific instrument type.
    ///
    /// Returns all metrics that either apply to all instruments (empty applicability)
    /// or specifically apply to the given instrument type.
    ///
    /// # Arguments
    /// * `instrument_type` - Type of instrument (e.g., "Bond", "IRS")
    ///
    /// # Returns
    /// Vector of metric IDs applicable to the instrument type
    ///
    /// See unit tests and `examples/` for usage.
    pub fn metrics_for_instrument(&self, instrument_type: &str) -> Vec<MetricId> {
        self.applicability
            .iter()
            .filter(|(_, applicable)| {
                applicable.is_empty() || applicable.contains(&instrument_type)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Checks if a metric is applicable to a specific instrument type.
    ///
    /// A metric is applicable if it's registered and either applies to all
    /// instruments (empty applicability list) or specifically applies to
    /// the given instrument type.
    ///
    /// # Arguments
    /// * `metric_id` - Metric ID to check
    /// * `instrument_type` - Type of instrument (e.g., "Bond", "IRS")
    ///
    /// # Returns
    /// `true` if the metric is applicable, `false` otherwise
    ///
    /// See unit tests and `examples/` for usage.
    pub fn is_applicable(&self, metric_id: &MetricId, instrument_type: &str) -> bool {
        if let Some(applicable) = self.applicability.get(metric_id) {
            applicable.is_empty() || applicable.contains(&instrument_type)
        } else {
            false
        }
    }

    /// Computes specific metrics with dependency resolution.
    ///
    /// Handles dependency resolution, ordering, caching of intermediate results,
    /// and error propagation. Metrics are computed in the correct order based
    /// on their dependencies, and results are cached in the context.
    ///
    /// # Arguments
    /// * `metric_ids` - Vector of metric IDs to compute
    /// * `context` - Metric context containing instrument and market data
    ///
    /// # Returns
    /// HashMap mapping metric IDs to computed values
    ///
    /// # Errors
    /// Returns an error if:
    /// - A requested metric is not registered
    /// - A metric has unregistered dependencies
    /// - Any metric calculation fails
    ///
    /// See unit tests and `examples/` for usage.
    pub fn compute(
        &self,
        metric_ids: &[MetricId],
        context: &mut MetricContext,
    ) -> finstack_core::Result<HashMap<MetricId, F>> {
        // Build dependency graph and compute order
        let order = self.resolve_dependencies(metric_ids)?;

        // Get instrument type tag once from the instrument key
        let instrument_type = match context.instrument.key() {
            crate::pricer::InstrumentType::Bond => "Bond",
            crate::pricer::InstrumentType::Loan => "Loan",
            crate::pricer::InstrumentType::CDS => "CDS",
            crate::pricer::InstrumentType::CDSIndex => "CDSIndex",
            crate::pricer::InstrumentType::CDSTranche => "CDSTranche",
            crate::pricer::InstrumentType::CDSOption => "CdsOption",
            crate::pricer::InstrumentType::IRS => "InterestRateSwap",
            crate::pricer::InstrumentType::CapFloor => "InterestRateOption",
            crate::pricer::InstrumentType::Swaption => "Swaption",
            crate::pricer::InstrumentType::TRS => "TRS",
            crate::pricer::InstrumentType::BasisSwap => "BasisSwap",
            crate::pricer::InstrumentType::Basket => "Basket",
            crate::pricer::InstrumentType::Convertible => "ConvertibleBond",
            crate::pricer::InstrumentType::Deposit => "Deposit",
            crate::pricer::InstrumentType::EquityOption => "EquityOption",
            crate::pricer::InstrumentType::FxOption => "FxOption",
            crate::pricer::InstrumentType::FxSpot => "FxSpot",
            crate::pricer::InstrumentType::FxSwap => "FxSwap",
            crate::pricer::InstrumentType::InflationLinkedBond => "InflationLinkedBond",
            crate::pricer::InstrumentType::InflationSwap => "InflationSwap",
            crate::pricer::InstrumentType::InterestRateFuture => "InterestRateFuture",
            crate::pricer::InstrumentType::VarianceSwap => "VarianceSwap",
            crate::pricer::InstrumentType::Equity => "Equity",
            crate::pricer::InstrumentType::Repo => "Repo",
            crate::pricer::InstrumentType::FRA => "FRA",
        };

        // Compute metrics in dependency order
        for metric_id in order {
            // Skip if already computed
            if context.computed.contains_key(&metric_id) {
                continue;
            }

            // Get calculator
            let calc = self.calculators.get(&metric_id).ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric_calculator".to_string(),
                })
            })?;

            // Check applicability. If metric was explicitly requested by caller, always compute.
            if !self.is_applicable(&metric_id, instrument_type) && !metric_ids.contains(&metric_id)
            {
                continue;
            }

            // Compute metric
            let value = calc.calculate(context)?;
            context.computed.insert(metric_id, value);
        }

        // Return only the requested metrics
        let mut results = HashMap::new();
        for id in metric_ids {
            if let Some(&value) = context.computed.get(id) {
                results.insert(id.clone(), value);
            }
        }

        Ok(results)
    }

    /// Computes all registered metrics applicable to the instrument.
    ///
    /// This is a convenience method that finds all applicable metrics
    /// for the instrument type and computes them all at once. Useful
    /// for comprehensive analysis or when you want all available metrics.
    ///
    /// # Arguments
    /// * `context` - Metric context containing instrument and market data
    ///
    /// # Returns
    /// HashMap mapping all applicable metric IDs to computed values
    ///
    /// See unit tests and `examples/` for usage.
    pub fn compute_all(
        &self,
        context: &mut MetricContext,
    ) -> finstack_core::Result<HashMap<MetricId, F>> {
        let instrument_type = match context.instrument.key() {
            crate::pricer::InstrumentType::Bond => "Bond",
            crate::pricer::InstrumentType::Loan => "Loan",
            crate::pricer::InstrumentType::CDS => "CDS",
            crate::pricer::InstrumentType::CDSIndex => "CDSIndex",
            crate::pricer::InstrumentType::CDSTranche => "CDSTranche",
            crate::pricer::InstrumentType::CDSOption => "CdsOption",
            crate::pricer::InstrumentType::IRS => "InterestRateSwap",
            crate::pricer::InstrumentType::CapFloor => "InterestRateOption",
            crate::pricer::InstrumentType::Swaption => "Swaption",
            crate::pricer::InstrumentType::TRS => "TRS",
            crate::pricer::InstrumentType::BasisSwap => "BasisSwap",
            crate::pricer::InstrumentType::Basket => "Basket",
            crate::pricer::InstrumentType::Convertible => "ConvertibleBond",
            crate::pricer::InstrumentType::Deposit => "Deposit",
            crate::pricer::InstrumentType::EquityOption => "EquityOption",
            crate::pricer::InstrumentType::FxOption => "FxOption",
            crate::pricer::InstrumentType::FxSpot => "FxSpot",
            crate::pricer::InstrumentType::FxSwap => "FxSwap",
            crate::pricer::InstrumentType::InflationLinkedBond => "InflationLinkedBond",
            crate::pricer::InstrumentType::InflationSwap => "InflationSwap",
            crate::pricer::InstrumentType::InterestRateFuture => "InterestRateFuture",
            crate::pricer::InstrumentType::VarianceSwap => "VarianceSwap",
            crate::pricer::InstrumentType::Equity => "Equity",
            crate::pricer::InstrumentType::Repo => "Repo",
            crate::pricer::InstrumentType::FRA => "FRA",
        };
        let applicable = self.metrics_for_instrument(instrument_type);
        self.compute(&applicable, context)
    }

    /// Resolves dependencies and returns computation order.
    ///
    /// Uses topological sorting to ensure dependencies are computed first.
    /// This prevents circular dependencies and ensures efficient computation.
    ///
    /// # Arguments
    /// * `metric_ids` - Vector of metric IDs to resolve dependencies for
    ///
    /// # Returns
    /// Vector of metric IDs in dependency order (dependencies first)
    ///
    /// # Errors
    /// Returns an error if circular dependencies are detected
    fn resolve_dependencies(
        &self,
        metric_ids: &[MetricId],
    ) -> finstack_core::Result<Vec<MetricId>> {
        let mut visited = hashbrown::HashSet::new();
        let mut order = Vec::new();
        let mut temp_mark = hashbrown::HashSet::new();

        for id in metric_ids {
            self.visit_metric(id.clone(), &mut visited, &mut temp_mark, &mut order)?;
        }

        Ok(order)
    }

    /// DFS visit for topological sort.
    fn visit_metric(
        &self,
        id: MetricId,
        visited: &mut hashbrown::HashSet<MetricId>,
        temp_mark: &mut hashbrown::HashSet<MetricId>,
        order: &mut Vec<MetricId>,
    ) -> finstack_core::Result<()> {
        if visited.contains(&id) {
            return Ok(());
        }

        if temp_mark.contains(&id) {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ));
        }

        temp_mark.insert(id.clone());

        // Get calculator and process dependencies
        if let Some(calc) = self.calculators.get(&id) {
            for dep_id in calc.dependencies() {
                self.visit_metric(dep_id.clone(), visited, temp_mark, order)?;
            }
        }

        temp_mark.remove(&id);
        visited.insert(id.clone());
        order.push(id);

        Ok(())
    }
}
