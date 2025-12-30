use crate::covenants::CovenantReport;
use crate::metrics::MetricId;
use finstack_core::config::{results_meta, FinstackConfig, ResultsMeta};
use finstack_core::dates::Date;
use finstack_core::explain::ExplanationTrace;
use finstack_core::money::Money;

use indexmap::IndexMap;

/// Complete valuation result envelope with NPV, risk metrics, and metadata.
///
/// This is the primary output structure returned by pricing operations.
/// It contains the instrument's present value, computed risk metrics,
/// calculation metadata, and optional covenant checks or explainability traces.
///
/// # Structure
///
/// - **Value**: Present value in the instrument's native currency
/// - **Measures**: Risk metrics as key-value pairs (e.g., "dv01" → 500.0)
/// - **Metadata**: Calculation context (rounding, numeric mode, timing)
/// - **Covenants**: Optional covenant compliance results
/// - **Explanation**: Optional computation trace for debugging
///
/// # Metadata Stamping
///
/// Results are stamped with metadata indicating:
/// - Numeric mode (Decimal vs f64)
/// - Rounding policy applied
/// - FX policy for cross-currency calculations
/// - Calculation timestamp and duration
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use finstack_valuations::results::ValuationResult;
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_core::dates::create_date;
/// use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let as_of = create_date(2025, Month::January, 15)?;
/// let pv = Money::new(1_000_000.0, Currency::USD);
///
/// let result = ValuationResult::stamped("BOND-001", as_of, pv);
///
/// assert_eq!(result.instrument_id, "BOND-001");
/// assert_eq!(result.value.amount(), 1_000_000.0);
/// assert_eq!(result.value.currency(), Currency::USD);
/// # Ok(())
/// # }
/// ```
///
/// ## With Metrics
///
/// ```rust
/// use finstack_valuations::results::ValuationResult;
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_core::dates::create_date;
/// use indexmap::IndexMap;
/// use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let as_of = create_date(2025, Month::January, 15)?;
/// let pv = Money::new(1_000_000.0, Currency::USD);
///
/// let mut measures = IndexMap::new();
/// measures.insert(MetricId::custom("ytm"), 0.0475);
/// measures.insert(MetricId::custom("modified_duration"), 4.25);
/// measures.insert(MetricId::custom("dv01"), 425.0);
///
/// let result = ValuationResult::stamped("BOND-001", as_of, pv)
///     .with_measures(measures);
///
/// assert_eq!(result.metric_str("ytm"), Some(0.0475));
/// assert_eq!(result.metric_str("dv01"), Some(425.0));
/// # Ok(())
/// # }
/// ```
///
/// ## With Covenants
///
/// ```rust
/// use finstack_valuations::results::ValuationResult;
/// use finstack_valuations::covenants::CovenantReport;
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_core::dates::create_date;
/// use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let as_of = create_date(2025, Month::January, 15)?;
/// let pv = Money::new(1_000_000.0, Currency::USD);
///
/// let covenant = CovenantReport {
///     covenant_type: "dscr".to_string(),
///     passed: true,
///     actual_value: Some(1.5),
///     threshold: Some(1.25),
///     details: Some("DSCR: 1.50x >= 1.25x".to_string()),
///     headroom: Some(0.25),
/// };
///
/// let result = ValuationResult::stamped("LOAN-001", as_of, pv)
///     .with_covenant("dscr_test", covenant);
///
/// assert!(result.all_covenants_passed());
/// assert_eq!(result.failed_covenants().len(), 0);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValuationResult {
    /// Unique identifier for the priced instrument.
    pub instrument_id: String,

    /// Valuation date (T+0) for the calculation.
    pub as_of: Date,

    /// Present value in the instrument's native currency.
    ///
    /// This is the primary pricing output and is **always available** regardless
    /// of which metrics are requested. The PV is **not** included in the `measures`
    /// map - it is provided here as a `Money` type with full currency information.
    ///
    /// For cross-currency instruments, this may be in a different currency than
    /// the base calculation currency.
    ///
    /// # Example
    /// ```rust
    /// # use finstack_valuations::results::ValuationResult;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use time::Month;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let as_of = create_date(2025, Month::January, 15)?;
    /// # let pv = Money::new(1_000_000.0, Currency::USD);
    /// # let result = ValuationResult::stamped("BOND-001", as_of, pv);
    /// // PV is always in result.value, not in measures
    /// let pv_money = result.value;  // Money type
    /// let pv_amount = result.value.amount();  // f64 value
    /// let currency = result.value.currency();  // Currency type
    /// # Ok(())
    /// # }
    /// ```
    pub value: Money,

    /// Computed risk measures and financial metrics.
    ///
    /// Contains **derived risk metrics** such as DV01, Delta, Vega, etc.
    /// The present value (PV) is **not** included here - it is available
    /// in the `value` field above.
    ///
    /// Keys are strongly-typed metric IDs (serialized as strings such as
    /// "ytm", "dv01", "delta"). Use `MetricId` helpers for consistent lookups.
    pub measures: IndexMap<MetricId, f64>,

    /// Calculation metadata and policy stamps.
    ///
    /// Contains:
    /// - Numeric mode (Decimal vs f64)
    /// - Rounding context and precision
    /// - FX policy for cross-currency calculations
    /// - Calculation timing information
    pub meta: ResultsMeta,

    /// Covenant compliance results for structured products.
    ///
    /// Present only for instruments with covenants (loans, structured credit).
    /// Each covenant is keyed by its identifier with pass/fail status and details.
    pub covenants: Option<IndexMap<String, CovenantReport>>,

    /// Optional computation explanation trace.
    ///
    /// Enabled via `ExplainOpts` in configuration. Provides step-by-step
    /// trace of calculations for debugging and auditability.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub explanation: Option<ExplanationTrace>,
}

impl ValuationResult {
    /// Create a basic valuation result with NPV and default metadata.
    ///
    /// Constructs a result with just the present value, using default
    /// configuration for metadata stamping (Decimal mode, default rounding).
    /// For custom metadata, use [`stamped_with_meta()`](Self::stamped_with_meta).
    ///
    /// # Arguments
    ///
    /// * `instrument_id` - Unique identifier for the priced instrument
    /// * `as_of` - Valuation date
    /// * `value` - Present value in the instrument's currency
    ///
    /// # Returns
    ///
    /// `ValuationResult` with NPV and default metadata (no metrics or covenants)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::results::ValuationResult;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_core::dates::create_date;
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let as_of = create_date(2025, Month::January, 15)?;
    /// let pv = Money::new(1_000_000.0, Currency::USD);
    ///
    /// let result = ValuationResult::stamped("BOND-001", as_of, pv);
    ///
    /// assert_eq!(result.instrument_id, "BOND-001");
    /// assert_eq!(result.value.amount(), 1_000_000.0);
    /// assert!(result.measures.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn stamped(instrument_id: &str, as_of: Date, value: Money) -> Self {
        // Default stamping uses default configuration; callers needing custom
        // policy should construct core `ResultsMeta` and use
        // `stamped_with_meta` to avoid creating a fresh config here.
        let meta = results_meta(&FinstackConfig::default());
        Self::stamped_with_meta(instrument_id, as_of, value, meta)
    }

    /// Create a valuation result using a provided configuration.
    ///
    /// This helper ensures the metadata stamp matches the exact `FinstackConfig`
    /// used during pricing, avoiding mismatches between execution policy and
    /// reported metadata.
    pub fn stamped_with_config(
        instrument_id: &str,
        as_of: Date,
        value: Money,
        cfg: &FinstackConfig,
    ) -> Self {
        let meta = results_meta(cfg);
        Self::stamped_with_meta(instrument_id, as_of, value, meta)
    }

    /// Create a valuation result with caller-provided metadata.
    ///
    /// Use this in hot paths when you already have `ResultsMeta` available
    /// to avoid constructing a default `FinstackConfig`. This is the
    /// performance-optimized constructor for repeated valuations.
    ///
    /// # Arguments
    ///
    /// * `instrument_id` - Unique identifier for the priced instrument
    /// * `as_of` - Valuation date
    /// * `value` - Present value in the instrument's currency
    /// * `meta` - Pre-constructed metadata with policy stamps
    ///
    /// # Returns
    ///
    /// `ValuationResult` with NPV and provided metadata
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::results::ValuationResult;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_core::dates::create_date;
    /// use finstack_core::config::{FinstackConfig, results_meta};
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let as_of = create_date(2025, Month::January, 15)?;
    /// let pv = Money::new(1_000_000.0, Currency::USD);
    ///
    /// // Pre-construct metadata once for batch pricing
    /// let config = FinstackConfig::default();
    /// let meta = results_meta(&config);
    ///
    /// let result = ValuationResult::stamped_with_meta("BOND-001", as_of, pv, meta);
    /// assert_eq!(result.instrument_id, "BOND-001");
    /// # Ok(())
    /// # }
    /// ```
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

    /// Attach an explanation trace for debugging and auditability.
    ///
    /// Explanation traces provide step-by-step computation logs showing
    /// intermediate calculations and data flow. Enable via `ExplainOpts`
    /// in configuration.
    ///
    /// # Arguments
    ///
    /// * `trace` - Explanation trace from the computation
    ///
    /// # Returns
    ///
    /// `Self` for method chaining
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::results::ValuationResult;
    /// use finstack_core::explain::ExplanationTrace;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let as_of = create_date(2025, Month::January, 15)?;
    /// # let pv = Money::new(1_000_000.0, Currency::USD);
    ///
    /// let trace = ExplanationTrace::new("bond_pricing");
    /// // Add trace entries using TraceEntry variants (see explain module for available types)
    ///
    /// let result = ValuationResult::stamped("BOND-001", as_of, pv)
    ///     .with_explanation(trace);
    ///
    /// assert!(result.explanation.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_explanation(mut self, trace: ExplanationTrace) -> Self {
        self.explanation = Some(trace);
        self
    }

    /// Attach computed risk metrics to the result.
    ///
    /// Replaces any existing measures with the provided map. Metrics
    /// are keyed by their string identifier (e.g., "ytm", "dv01").
    ///
    /// # Arguments
    ///
    /// * `measures` - Map of metric name to computed value
    ///
    /// # Returns
    ///
    /// `Self` for method chaining
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::results::ValuationResult;
    /// use indexmap::IndexMap;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let as_of = create_date(2025, Month::January, 15)?;
    /// # let pv = Money::new(1_000_000.0, Currency::USD);
    /// let mut measures = IndexMap::new();
    /// measures.insert(MetricId::custom("ytm"), 0.0475);
    /// measures.insert(MetricId::custom("modified_duration"), 4.25);
    ///
    /// let result = ValuationResult::stamped("BOND-001", as_of, pv)
    ///     .with_measures(measures);
    ///
    /// assert_eq!(result.measures.len(), 2);
    /// assert_eq!(
    ///     result.metric(MetricId::custom("ytm")),
    ///     Some(0.0475)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_measures(mut self, measures: IndexMap<MetricId, f64>) -> Self {
        self.measures = measures;
        self
    }

    /// Get a metric by `MetricId`.
    pub fn metric(&self, id: MetricId) -> Option<f64> {
        self.measures.get(&id).copied()
    }

    /// Get a metric by string identifier (strict parse with custom fallback).
    pub fn metric_str(&self, id: &str) -> Option<f64> {
        let metric_id =
            MetricId::parse_strict(id).unwrap_or_else(|_| MetricId::custom(id.to_owned()));
        self.metric(metric_id)
    }

    /// Attach multiple covenant reports to the result.
    ///
    /// Replaces any existing covenant reports with the provided map.
    /// Used for structured products with multiple compliance tests.
    ///
    /// # Arguments
    ///
    /// * `covenants` - Map of covenant identifier to compliance report
    ///
    /// # Returns
    ///
    /// `Self` for method chaining
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::results::ValuationResult;
    /// use finstack_valuations::covenants::CovenantReport;
    /// use indexmap::IndexMap;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let as_of = create_date(2025, Month::January, 15)?;
    /// # let pv = Money::new(1_000_000.0, Currency::USD);
    /// let mut covenants = IndexMap::new();
    /// covenants.insert("dscr".to_string(), CovenantReport {
    ///     covenant_type: "dscr".to_string(),
    ///     passed: true,
    ///     actual_value: Some(1.5),
    ///     threshold: Some(1.25),
    ///     details: Some("DSCR test passed".to_string()),
    ///     headroom: Some(0.25),
    /// });
    ///
    /// let result = ValuationResult::stamped("LOAN-001", as_of, pv)
    ///     .with_covenants(covenants);
    ///
    /// assert!(result.all_covenants_passed());
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_covenants(mut self, covenants: IndexMap<String, CovenantReport>) -> Self {
        self.covenants = Some(covenants);
        self
    }

    /// Add a single covenant report to the result.
    ///
    /// Preserves existing covenant reports and adds a new one.
    /// Convenient for incrementally building covenant results.
    ///
    /// # Arguments
    ///
    /// * `key` - Covenant identifier (e.g., "dscr_test", "ltv_check")
    /// * `report` - Covenant compliance report
    ///
    /// # Returns
    ///
    /// `Self` for method chaining
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::results::ValuationResult;
    /// use finstack_valuations::covenants::CovenantReport;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let as_of = create_date(2025, Month::January, 15)?;
    /// # let pv = Money::new(1_000_000.0, Currency::USD);
    /// let result = ValuationResult::stamped("LOAN-001", as_of, pv)
    ///     .with_covenant("dscr", CovenantReport {
    ///         covenant_type: "dscr".to_string(),
    ///         passed: true,
    ///         actual_value: Some(1.5),
    ///         threshold: Some(1.25),
    ///         details: None,
    ///         headroom: Some(0.25),
    ///     })
    ///     .with_covenant("ltv", CovenantReport {
    ///         covenant_type: "ltv".to_string(),
    ///         passed: true,
    ///         actual_value: Some(0.70),
    ///         threshold: Some(0.80),
    ///         details: None,
    ///         headroom: Some(0.10),
    ///     });
    ///
    /// assert_eq!(result.covenants.as_ref().expect("should succeed").len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_covenant(mut self, key: &str, report: CovenantReport) -> Self {
        let mut covenants = self.covenants.unwrap_or_default();
        covenants.insert(key.to_string(), report);
        self.covenants = Some(covenants);
        self
    }

    // Note: FX policy stamping is handled at the core `ResultsMeta` level.

    /// Check if all covenants passed their compliance tests.
    ///
    /// Returns `true` if there are no covenants or if all covenants passed.
    /// Use [`failed_covenants()`](Self::failed_covenants) to get specific failures.
    ///
    /// # Returns
    ///
    /// `true` if all covenants passed or no covenants present, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::results::ValuationResult;
    /// use finstack_valuations::covenants::CovenantReport;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let as_of = create_date(2025, Month::January, 15)?;
    /// # let pv = Money::new(1_000_000.0, Currency::USD);
    /// let result = ValuationResult::stamped("LOAN-001", as_of, pv)
    ///     .with_covenant("dscr", CovenantReport {
    ///         covenant_type: "dscr".to_string(),
    ///         passed: true,
    ///         actual_value: Some(1.5),
    ///         threshold: Some(1.25),
    ///         details: None,
    ///         headroom: Some(0.25),
    ///     });
    ///
    /// assert!(result.all_covenants_passed());
    /// # Ok(())
    /// # }
    /// ```
    pub fn all_covenants_passed(&self) -> bool {
        self.covenants
            .as_ref()
            .map(|c| c.values().all(|r| r.passed))
            .unwrap_or(true)
    }

    /// Get list of failed covenant identifiers.
    ///
    /// Returns identifiers of covenants that did not pass their compliance
    /// tests. Empty vector if all covenants passed or no covenants present.
    ///
    /// # Returns
    ///
    /// Vector of covenant identifiers that failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::results::ValuationResult;
    /// use finstack_valuations::covenants::CovenantReport;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let as_of = create_date(2025, Month::January, 15)?;
    /// # let pv = Money::new(1_000_000.0, Currency::USD);
    /// let result = ValuationResult::stamped("LOAN-001", as_of, pv)
    ///     .with_covenant("dscr", CovenantReport {
    ///         covenant_type: "dscr".to_string(),
    ///         passed: false,
    ///         actual_value: Some(1.1),
    ///         threshold: Some(1.25),
    ///         details: Some("DSCR below threshold".to_string()),
    ///         headroom: Some(-0.15),
    ///     });
    ///
    /// let failed = result.failed_covenants();
    /// assert_eq!(failed.len(), 1);
    /// assert_eq!(failed[0], "dscr");
    /// # Ok(())
    /// # }
    /// ```
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
