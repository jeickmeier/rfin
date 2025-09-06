//! Risk report data structures and risk-related traits.

use finstack_core::prelude::*;
use finstack_core::F;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

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
    pub fn new(instrument_id: &str, base_currency: Currency) -> Self {
        Self {
            instrument_id: instrument_id.to_string(),
            base_currency,
            metrics: HashMap::new(),
            bucketed_risks: HashMap::new(),
            buckets: Vec::new(),
            meta: HashMap::new(),
        }
    }

    /// Add a risk metric.
    pub fn with_metric(mut self, name: &str, value: F) -> Self {
        self.metrics.insert(name.to_string(), value);
        self
    }

    /// Add bucketed risks.
    pub fn with_bucketed_risk(
        mut self,
        risk_type: &str,
        buckets: HashMap<String, F>,
    ) -> Self {
        self.bucketed_risks.insert(risk_type.to_string(), buckets);
        self
    }

    /// Add a risk bucket classification.
    pub fn with_bucket(mut self, bucket: RiskBucket) -> Self {
        self.buckets.push(bucket);
        self
    }
}

/// Trait for instruments that can generate risk reports.
pub trait RiskMeasurable: Send + Sync {
    /// Generate a risk report for the instrument.
    fn risk_report(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: Date,
        bucket_spec: Option<&[RiskBucket]>,
    ) -> finstack_core::Result<RiskReport>;

    /// Get default risk buckets for this instrument type.
    fn default_risk_buckets(&self) -> Option<Vec<RiskBucket>> {
        None
    }
}
