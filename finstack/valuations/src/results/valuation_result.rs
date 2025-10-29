use crate::covenants::CovenantReport;
use finstack_core::explain::ExplanationTrace;
use finstack_core::prelude::*;

use indexmap::IndexMap;

/// Complete valuation result with NPV and computed metrics.
///
/// Contains the instrument's present value along with all requested
/// risk measures and metadata about the calculation.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValuationResult {
    /// Unique identifier for the instrument.
    pub instrument_id: String,
    /// Valuation date.
    pub as_of: Date,
    /// Present value of the instrument.
    pub value: Money,
    /// Computed risk measures and metrics.
    pub measures: IndexMap<String, f64>,
    /// Metadata about the calculation (timing, precision, etc.).
    pub meta: ResultsMeta,
    /// Covenant check results (if applicable).
    pub covenants: Option<IndexMap<String, CovenantReport>>,
    /// Optional explanation trace (enabled via ExplainOpts).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub explanation: Option<ExplanationTrace>,
}

impl ValuationResult {
    /// Create a basic valuation result with just NPV.
    ///
    /// See unit tests and `examples/` for usage.
    pub fn stamped(instrument_id: &str, as_of: Date, value: Money) -> Self {
        // Default stamping uses default configuration; callers needing custom
        // policy should construct core `ResultsMeta` and use
        // `stamped_with_meta` to avoid creating a fresh config here.
        let meta =
            finstack_core::config::results_meta(&finstack_core::config::FinstackConfig::default());
        Self::stamped_with_meta(instrument_id, as_of, value, meta)
    }

    /// Create a valuation result with caller-provided metadata.
    ///
    /// Prefer this when you already have `ResultsMeta` available to avoid
    /// constructing a default `FinstackConfig` in hot paths.
    pub fn stamped_with_meta(
        instrument_id: &str,
        as_of: Date,
        value: Money,
        meta: ResultsMeta,
    ) -> Self {
        Self {
            instrument_id: instrument_id.to_string(),
            as_of,
            value,
            measures: IndexMap::new(),
            meta,
            covenants: None,
            explanation: None,
        }
    }

    /// Attach an explanation trace to this result.
    pub fn with_explanation(mut self, trace: ExplanationTrace) -> Self {
        self.explanation = Some(trace);
        self
    }

    /// Add measures to the result.
    /// See unit tests and `examples/` for usage.
    pub fn with_measures(mut self, measures: IndexMap<String, f64>) -> Self {
        self.measures = measures;
        self
    }

    /// Add covenant reports to the result.
    pub fn with_covenants(mut self, covenants: IndexMap<String, CovenantReport>) -> Self {
        self.covenants = Some(covenants);
        self
    }

    /// Add a single covenant report.
    pub fn with_covenant(mut self, key: &str, report: CovenantReport) -> Self {
        let mut covenants = self.covenants.unwrap_or_default();
        covenants.insert(key.to_string(), report);
        self.covenants = Some(covenants);
        self
    }

    // Note: FX policy stamping is handled at the core `ResultsMeta` level.

    /// Check if all covenants passed.
    pub fn all_covenants_passed(&self) -> bool {
        self.covenants
            .as_ref()
            .map(|c| c.values().all(|r| r.passed))
            .unwrap_or(true)
    }

    /// Get failed covenants.
    pub fn failed_covenants(&self) -> Vec<&str> {
        self.covenants
            .as_ref()
            .map(|c| {
                c.iter()
                    .filter(|(_, r)| !r.passed)
                    .map(|(k, _)| k.as_str())
                    .collect()
            })
            .unwrap_or_default()
    }
}
