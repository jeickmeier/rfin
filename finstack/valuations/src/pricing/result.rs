//! Result type for pricing.
//! 
use finstack_core::prelude::*;
use finstack_core::F;
use hashbrown::HashMap;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Covenant check result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantReport {
    /// Type of covenant being checked
    pub covenant_type: String,
    
    /// Whether the covenant passed
    pub passed: bool,
    
    /// Actual value of the metric
    pub actual_value: Option<F>,
    
    /// Required threshold
    pub threshold: Option<F>,
    
    /// Details or explanation
    pub details: Option<String>,
}

impl CovenantReport {
    /// Create a passing covenant report.
    pub fn passed(covenant_type: impl Into<String>) -> Self {
        Self {
            covenant_type: covenant_type.into(),
            passed: true,
            actual_value: None,
            threshold: None,
            details: None,
        }
    }
    
    /// Create a failing covenant report.
    pub fn failed(covenant_type: impl Into<String>) -> Self {
        Self {
            covenant_type: covenant_type.into(),
            passed: false,
            actual_value: None,
            threshold: None,
            details: None,
        }
    }
    
    /// Add actual value to the report.
    pub fn with_actual(mut self, value: F) -> Self {
        self.actual_value = Some(value);
        self
    }
    
    /// Add threshold to the report.
    pub fn with_threshold(mut self, threshold: F) -> Self {
        self.threshold = Some(threshold);
        self
    }
    
    /// Add details to the report.
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// FX policy metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxPolicyMeta {
    /// Policy name/identifier
    pub policy_name: String,
    
    /// Source of FX rates (e.g., "market", "fixed", "historical")
    pub source: String,
    
    /// Effective date for FX rates
    pub effective_date: Date,
    
    /// Applied conversion pairs and rates
    pub conversions: HashMap<(Currency, Currency), F>,
}

impl FxPolicyMeta {
    /// Create new FX policy metadata.
    pub fn new(policy_name: impl Into<String>, source: impl Into<String>, effective_date: Date) -> Self {
        Self {
            policy_name: policy_name.into(),
            source: source.into(),
            effective_date,
            conversions: HashMap::new(),
        }
    }
    
    /// Add a conversion rate.
    pub fn with_conversion(mut self, from: Currency, to: Currency, rate: F) -> Self {
        self.conversions.insert((from, to), rate);
        self
    }
}

/// Extended metadata for valuation results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtendedResultsMeta {
    /// Core metadata from finstack_core
    pub core: ResultsMeta,
    
    /// FX policies applied during valuation
    pub fx_policies: IndexMap<String, FxPolicyMeta>,
    
    /// Additional custom metadata
    pub custom: HashMap<String, String>,
}

impl ExtendedResultsMeta {
    /// Create from core metadata.
    pub fn from_core(core: ResultsMeta) -> Self {
        Self {
            core,
            fx_policies: IndexMap::new(),
            custom: HashMap::new(),
        }
    }
    
    /// Add an FX policy.
    pub fn with_fx_policy(mut self, key: impl Into<String>, policy: FxPolicyMeta) -> Self {
        self.fx_policies.insert(key.into(), policy);
        self
    }
    
    /// Add custom metadata.
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }
}

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
    pub measures: HashMap<String, F>,
    /// Metadata about the calculation (timing, precision, etc.).
    pub meta: ExtendedResultsMeta,
    /// Covenant check results (if applicable).
    pub covenants: Option<IndexMap<String, CovenantReport>>,
}

impl ValuationResult {
    /// Create a basic valuation result with just NPV.
    /// 
    /// See unit tests and `examples/` for usage.
    pub fn stamped<S: Into<String>>(instrument_id: S, as_of: Date, value: Money) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            as_of,
            value,
            measures: HashMap::new(),
            meta: ExtendedResultsMeta::from_core(finstack_core::config::results_meta()),
            covenants: None,
        }
    }
    
    /// Add measures to the result.
    /// See unit tests and `examples/` for usage.
    pub fn with_measures(mut self, measures: HashMap<String, F>) -> Self {
        self.measures = measures;
        self
    }
    
    /// Add covenant reports to the result.
    pub fn with_covenants(mut self, covenants: IndexMap<String, CovenantReport>) -> Self {
        self.covenants = Some(covenants);
        self
    }
    
    /// Add a single covenant report.
    pub fn with_covenant(mut self, key: impl Into<String>, report: CovenantReport) -> Self {
        let mut covenants = self.covenants.unwrap_or_default();
        covenants.insert(key.into(), report);
        self.covenants = Some(covenants);
        self
    }
    
    /// Add an FX policy to the metadata.
    pub fn with_fx_policy(mut self, key: impl Into<String>, policy: FxPolicyMeta) -> Self {
        self.meta.fx_policies.insert(key.into(), policy);
        self
    }
    
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


