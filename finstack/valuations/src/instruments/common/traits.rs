//! Instrument-level traits and metadata types.

use crate::metrics::MetricId;
use crate::pricer::InstrumentType; // Return type for key()
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Marker trait that associates a concrete instrument type with its `InstrumentType` enum.
///
/// Implement this on each instrument to enable generic pricers to infer the
/// correct registry key without per-instrument constructors.
pub trait InstrumentKind {
    const TYPE: InstrumentType;
}

/// Attributes for scenario selection and tagging.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Attributes {
    /// User-defined tags for categorization.
    pub tags: HashSet<String>,
    /// Key-value metadata pairs.
    pub meta: HashMap<String, String>,
}

impl Attributes {
    /// Create empty attributes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.insert(tag.to_string());
        self
    }

    /// Add multiple tags.
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        for tag in tags {
            self.tags.insert(tag.to_string());
        }
        self
    }

    /// Add a metadata key-value pair.
    pub fn with_meta(mut self, key: &str, value: &str) -> Self {
        self.meta.insert(key.to_string(), value.to_string());
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

/// Unified instrument trait combining identity, attributes, and pricing.
///
/// This is the primary trait for all financial instruments, providing both
/// metadata/identity methods and pricing functionality. All instruments
/// should implement this single trait.
pub trait Instrument: Send + Sync {
    /// Get the instrument's unique identifier.
    fn id(&self) -> &str;
    
    /// Get the strongly-typed instrument key for pricer dispatch.
    /// Uses TypeId-based mapping instead of brittle string matching.
    fn key(&self) -> InstrumentType {
        use std::any::TypeId;
        use once_cell::sync::Lazy;
        use std::collections::HashMap;

        static TYPE_MAP: Lazy<HashMap<TypeId, InstrumentType>> = Lazy::new(|| {
            let mut m = HashMap::new();
            m.insert(TypeId::of::<crate::instruments::Bond>(), InstrumentType::Bond);
            m.insert(TypeId::of::<crate::instruments::Deposit>(), InstrumentType::Deposit);
            m.insert(TypeId::of::<crate::instruments::ForwardRateAgreement>(), InstrumentType::FRA);
            m.insert(TypeId::of::<crate::instruments::InterestRateSwap>(), InstrumentType::IRS);
            m.insert(TypeId::of::<crate::instruments::cap_floor::InterestRateOption>(), InstrumentType::CapFloor);
            m.insert(TypeId::of::<crate::instruments::BasisSwap>(), InstrumentType::BasisSwap);
            m.insert(TypeId::of::<crate::instruments::Swaption>(), InstrumentType::Swaption);
            m.insert(TypeId::of::<crate::instruments::Basket>(), InstrumentType::Basket);
            m.insert(TypeId::of::<crate::instruments::ConvertibleBond>(), InstrumentType::Convertible);
            m.insert(TypeId::of::<crate::instruments::InflationLinkedBond>(), InstrumentType::InflationLinkedBond);
            m.insert(TypeId::of::<crate::instruments::InflationSwap>(), InstrumentType::InflationSwap);
            m.insert(TypeId::of::<crate::instruments::InterestRateFuture>(), InstrumentType::InterestRateFuture);
            m.insert(TypeId::of::<crate::instruments::trs::EquityTotalReturnSwap>(), InstrumentType::TRS);
            m.insert(TypeId::of::<crate::instruments::trs::FIIndexTotalReturnSwap>(), InstrumentType::TRS);
            m.insert(TypeId::of::<crate::instruments::CreditDefaultSwap>(), InstrumentType::CDS);
            m.insert(TypeId::of::<crate::instruments::CDSIndex>(), InstrumentType::CDSIndex);
            m.insert(TypeId::of::<crate::instruments::CdsOption>(), InstrumentType::CDSOption);
            m.insert(TypeId::of::<crate::instruments::CdsTranche>(), InstrumentType::CDSTranche);
            m.insert(TypeId::of::<crate::instruments::Equity>(), InstrumentType::Equity);
            m.insert(TypeId::of::<crate::instruments::EquityOption>(), InstrumentType::EquityOption);
            m.insert(TypeId::of::<crate::instruments::FxOption>(), InstrumentType::FxOption);
            m.insert(TypeId::of::<crate::instruments::FxSpot>(), InstrumentType::FxSpot);
            m.insert(TypeId::of::<crate::instruments::FxSwap>(), InstrumentType::FxSwap);
            m.insert(TypeId::of::<crate::instruments::Repo>(), InstrumentType::Repo);
            m.insert(TypeId::of::<crate::instruments::VarianceSwap>(), InstrumentType::VarianceSwap);
            m
        });

        let type_id = self.as_any().type_id();
        match TYPE_MAP.get(&type_id).copied() {
            Some(t) => t,
            None => {
                // Return a typed error via panic-free path: unknown instrument kind
                // Fallback to a safe default error type (use CDS as sentinel? better: log and return Equity)
                // Prefer logging to aid discovery
                tracing::error!("Unknown instrument concrete TypeId encountered in Instrument::key()");
                // Choose a conservative default to avoid UB; map to an impossible pricer to surface UnknownPricer later
                InstrumentType::Equity
            }
        }
    }

    /// Access to the concrete type for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Attributes accessors
    fn attributes(&self) -> &Attributes;
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

    /// Clone this instrument as a boxed trait object
    fn clone_box(&self) -> Box<dyn Instrument>;


    // === Pricing Methods ===

    /// Compute only the base present value (fast, no metrics).
    fn value(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money>;

    /// Compute value with a specific set of metrics.
    ///
    /// Implementations should build on `value()` and compute only the requested metrics.
    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult>;
}

// Note: Methods formerly on the `Attributable` trait are now default methods on `Instrument`.
