//! Parameters for TRS instruments.

use finstack_core::{
    types::id::IndexId,
    types::Currency, F,
};

/// Parameters for fixed income index underlying (for TRS and similar instruments)
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexUnderlyingParams {
    /// Index identifier (e.g., "CDX.IG", "HY.BOND.INDEX")
    pub index_id: IndexId,
    /// Base currency of the index
    pub base_currency: Currency,
    /// Optional yield curve/scalar identifier for carry calculation
    pub yield_id: Option<String>,
    /// Optional duration identifier for risk calculations
    pub duration_id: Option<String>,
    /// Optional convexity identifier for risk calculations
    pub convexity_id: Option<String>,
    /// Contract size (index units per contract, defaults to 1.0)
    pub contract_size: F,
}

impl IndexUnderlyingParams {
    /// Create index underlying parameters
    pub fn new(index_id: impl Into<String>, base_currency: Currency) -> Self {
        Self {
            index_id: IndexId::new(index_id),
            base_currency,
            yield_id: None,
            duration_id: None,
            convexity_id: None,
            contract_size: 1.0,
        }
    }

    /// Set yield identifier
    pub fn with_yield(mut self, yield_id: impl Into<String>) -> Self {
        self.yield_id = Some(yield_id.into());
        self
    }

    /// Set duration identifier
    pub fn with_duration(mut self, duration_id: impl Into<String>) -> Self {
        self.duration_id = Some(duration_id.into());
        self
    }

    /// Set convexity identifier
    pub fn with_convexity(mut self, convexity_id: impl Into<String>) -> Self {
        self.convexity_id = Some(convexity_id.into());
        self
    }

    /// Set contract size
    pub fn with_contract_size(mut self, size: F) -> Self {
        self.contract_size = size;
        self
    }
}
