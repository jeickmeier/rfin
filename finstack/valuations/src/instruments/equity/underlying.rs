//! Shared equity underlying parameters used across equity-linked instruments.

use finstack_core::F;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Equity underlying parameters for options and equity-linked swaps.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EquityUnderlyingParams {
    /// Underlying ticker/identifier
    pub ticker: String,
    /// Spot price identifier in market data
    pub spot_id: String,
    /// Optional dividend yield identifier
    pub dividend_yield_id: Option<String>,
    /// Contract size (shares per contract)
    pub contract_size: F,
}

impl EquityUnderlyingParams {
    /// Create equity underlying parameters
    pub fn new(ticker: impl Into<String>, spot_id: impl Into<String>) -> Self {
        Self {
            ticker: ticker.into(),
            spot_id: spot_id.into(),
            dividend_yield_id: None,
            contract_size: 1.0,
        }
    }

    /// Set dividend yield identifier
    pub fn with_dividend_yield(mut self, div_yield_id: impl Into<String>) -> Self {
        self.dividend_yield_id = Some(div_yield_id.into());
        self
    }

    /// Set contract size
    pub fn with_contract_size(mut self, size: F) -> Self {
        self.contract_size = size;
        self
    }
}
