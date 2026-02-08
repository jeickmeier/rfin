//! FX Swap specific parameters.

use finstack_core::{dates::Date, money::Money};

/// FX Swap specific parameters.
///
/// Groups parameters specific to FX swaps.
#[derive(Debug, Clone)]
pub struct FxSwapParams {
    /// Near leg date
    pub near_date: Date,
    /// Far leg date
    pub far_date: Date,
    /// Base notional amount
    pub base_notional: Money,
    /// Optional near leg rate (if fixed)
    pub near_rate: Option<f64>,
    /// Optional far leg rate (if fixed)
    pub far_rate: Option<f64>,
}

impl FxSwapParams {
    /// Create new FX swap parameters
    pub fn new(near_date: Date, far_date: Date, base_notional: Money) -> Self {
        Self {
            near_date,
            far_date,
            base_notional,
            near_rate: None,
            far_rate: None,
        }
    }

    /// Set near leg rate
    pub fn with_near_rate(mut self, rate: f64) -> Self {
        self.near_rate = Some(rate);
        self
    }

    /// Set far leg rate
    pub fn with_far_rate(mut self, rate: f64) -> Self {
        self.far_rate = Some(rate);
        self
    }
}
