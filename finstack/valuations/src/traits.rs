//! Core traits for financial instruments.

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::Discount;
use crate::pricing::discountable::Discountable;
use crate::metrics::MetricId;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

/// Currency-preserving schedule as a list of dated `Money` amounts.
/// 
/// Used for cashflow aggregation and NPV calculations across different
/// instruments and time periods.
pub type DatedFlows = Vec<(Date, Money)>;

/// Build cashflow schedules and provide currency-safe aggregation hooks.
/// 
/// Instruments implement this to generate their cashflow schedules
/// given market curves and valuation date.
pub trait CashflowProvider: Send + Sync {
    /// Build complete dated cashflow schedule as `(date, amount)` pairs.
    /// 
    /// # Errors
    /// Returns an error if the schedule cannot be built due to invalid
    /// instrument parameters or missing market data.
    fn build_schedule(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<DatedFlows>;

    /// Convenience: present value the built schedule against a discount curve and day-count.
    /// 
    /// See unit tests and `examples/` for usage.
    #[inline]
    fn npv_with(
        &self,
        curves: &CurveSet,
        as_of: Date,
        disc: &dyn Discount,
        dc: DayCount,
    ) -> finstack_core::Result<Money> {
        let base = disc.base_date();
        let flows = self.build_schedule(curves, as_of)?;
        flows.npv(disc, base, dc)
    }
}

/// Priceable instruments produce a `ValuationResult` at `as_of` using curves.
/// 
/// The default implementation uses the metrics framework to compute
/// measures, delegating to `value()` for base NPV calculation.
pub trait Priceable: Send + Sync {
    /// Compute full valuation with all standard metrics (backward compatible).
    /// 
    /// Returns a complete `ValuationResult` with NPV and computed metrics
    /// appropriate for the instrument type.
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<super::pricing::result::ValuationResult>;
    
    /// Compute only the base present value (fast, no metrics).
    /// 
    /// Use this when you only need the NPV and don't require
    /// duration, convexity, or other risk measures.
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        // Default implementation for backward compatibility
        self.price(curves, as_of).map(|r| r.value)
    }
    
    /// Compute value with specific metrics.
    /// 
    /// See unit tests and `examples/` for usage.
    fn price_with_metrics(
        &self, 
        curves: &CurveSet, 
        as_of: Date, 
        metrics: &[MetricId]
    ) -> finstack_core::Result<super::pricing::result::ValuationResult> {
        // Default implementation: just calls price() and filters metrics
        let result = self.price(curves, as_of)?;
        let mut filtered_result = result.clone();
        
        // Convert MetricIds to strings for filtering
        let metric_strs: Vec<String> = metrics.iter().map(|m| m.as_str().to_string()).collect();
        filtered_result.measures.retain(|k, _| metric_strs.contains(k));
        Ok(filtered_result)
    }
}

/// Attributes for scenario selection and tagging.
///
/// Provides metadata for instruments that can be used for:
/// - Scenario selection and filtering
/// - Risk bucketing and aggregation
/// - Custom tagging and classification
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Attributes {
    /// User-defined tags for categorization.
    /// 
    /// Examples: ["corporate", "investment_grade", "tech_sector"]
    pub tags: HashSet<String>,
    
    /// Key-value metadata pairs.
    /// 
    /// Examples: {"issuer": "AAPL", "rating": "AA+", "sector": "Technology"}
    pub meta: HashMap<String, String>,
}

impl Attributes {
    /// Create empty attributes.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }
    
    /// Add multiple tags.
    pub fn with_tags<I, S>(mut self, tags: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for tag in tags {
            self.tags.insert(tag.into());
        }
        self
    }
    
    /// Add a metadata key-value pair.
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.meta.insert(key.into(), value.into());
        self
    }
    
    /// Check if a tag exists.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }
    
    /// Get a metadata value by key.
    pub fn get_meta(&self, key: &str) -> Option<&str> {
        self.meta.get(key).map(|s| s.as_str())
    }
    
    /// Check if attributes match a selector pattern.
    /// 
    /// Supports:
    /// - Tag matching: "tag:corporate"
    /// - Meta matching: "meta:rating=AA+"
    /// - Wildcard: "*"
    pub fn matches_selector(&self, selector: &str) -> bool {
        if selector == "*" {
            return true;
        }
        
        if let Some(tag) = selector.strip_prefix("tag:") {
            return self.has_tag(tag);
        }
        
        if let Some(meta_spec) = selector.strip_prefix("meta:") {
            if let Some((key, value)) = meta_spec.split_once('=') {
                return self.get_meta(key) == Some(value);
            }
        }
        
        false
    }
}

/// Trait for instruments with attributes.
///
/// Enables scenario selection, tagging, and metadata management
/// for financial instruments.
pub trait Attributable: Send + Sync {
    /// Get the instrument's attributes.
    fn attributes(&self) -> &Attributes;
    
    /// Get mutable access to attributes.
    fn attributes_mut(&mut self) -> &mut Attributes;
    
    /// Check if the instrument matches a selector.
    fn matches_selector(&self, selector: &str) -> bool {
        self.attributes().matches_selector(selector)
    }
    
    /// Check if the instrument has a specific tag.
    fn has_tag(&self, tag: &str) -> bool {
        self.attributes().has_tag(tag)
    }
    
    /// Get a metadata value by key.
    fn get_meta(&self, key: &str) -> Option<&str> {
        self.attributes().get_meta(key)
    }
}

/// Risk bucket specification for aggregation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskBucket {
    /// Bucket identifier (e.g., "1Y", "5Y", "10Y")
    pub id: String,
    
    /// Tenor in years (for maturity bucketing)
    pub tenor_years: Option<F>,
    
    /// Custom classification
    pub classification: Option<String>,
}

/// Risk report for an instrument.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskReport {
    /// Instrument identifier
    pub instrument_id: String,
    
    /// Base currency for risk measures
    pub base_currency: Currency,
    
    /// Key risk metrics (DV01, CS01, duration, etc.)
    pub metrics: HashMap<String, F>,
    
    /// Bucketed sensitivities (e.g., DV01 by tenor)
    pub bucketed_risks: HashMap<String, HashMap<String, F>>,
    
    /// Risk buckets this instrument belongs to
    pub buckets: Vec<RiskBucket>,
    
    /// Additional risk metadata
    pub meta: HashMap<String, String>,
}

impl RiskReport {
    /// Create a new risk report.
    pub fn new(instrument_id: impl Into<String>, base_currency: Currency) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            base_currency,
            metrics: HashMap::new(),
            bucketed_risks: HashMap::new(),
            buckets: Vec::new(),
            meta: HashMap::new(),
        }
    }
    
    /// Add a risk metric.
    pub fn with_metric(mut self, name: impl Into<String>, value: F) -> Self {
        self.metrics.insert(name.into(), value);
        self
    }
    
    /// Add bucketed risks.
    pub fn with_bucketed_risk(mut self, risk_type: impl Into<String>, buckets: HashMap<String, F>) -> Self {
        self.bucketed_risks.insert(risk_type.into(), buckets);
        self
    }
    
    /// Add a risk bucket classification.
    pub fn with_bucket(mut self, bucket: RiskBucket) -> Self {
        self.buckets.push(bucket);
        self
    }
}

/// Trait for instruments that can generate risk reports.
///
/// Provides standardized risk measurement and bucketing
/// capabilities for financial instruments.
pub trait RiskMeasurable: Send + Sync {
    /// Generate a risk report for the instrument.
    /// 
    /// # Arguments
    /// * `curves` - Market curves for pricing and risk calculation
    /// * `as_of` - Valuation date
    /// * `bucket_spec` - Optional bucket specifications for risk aggregation
    /// 
    /// # Returns
    /// A comprehensive risk report with metrics and bucketed sensitivities
    fn risk_report(
        &self,
        curves: &CurveSet,
        as_of: Date,
        bucket_spec: Option<&[RiskBucket]>,
    ) -> finstack_core::Result<RiskReport>;
    
    /// Get default risk buckets for this instrument type.
    /// 
    /// Returns None if no default bucketing is appropriate.
    fn default_risk_buckets(&self) -> Option<Vec<RiskBucket>> {
        None
    }
}