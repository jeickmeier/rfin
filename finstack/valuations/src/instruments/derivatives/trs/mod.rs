//! Total Return Swap instruments for equity and fixed income indices.
//!
//! Provides implementations for TRS on equity indices and fixed income indices,
//! including builders, pricing engines, and risk metrics.

mod equity;
mod fixed_income_index;
pub mod metrics;
mod types;
pub mod pricing;

// Re-export main types
pub use equity::EquityTotalReturnSwap;
pub use fixed_income_index::FIIndexTotalReturnSwap;
pub use types::{IndexUnderlyingParams, FinancingLegSpec, TotalReturnLegSpec, TrsScheduleSpec, TrsSide};
pub use pricing::engine::TrsEngine;

/// Shared TRS helpers
pub(crate) mod helpers {
    use finstack_core::money::Money;
    use finstack_core::types::Currency;
    use finstack_core::Result;

    /// Validate TRS notional currency against optional base currency
    pub fn validate_trs_currencies(notional: Money, base: Option<Currency>) -> Result<()> {
        // Ensure same-currency amounts
        crate::instruments::utils::validate_currency_consistency(&[notional])?;
        if let Some(base_ccy) = base {
            if base_ccy != notional.currency() {
                return Err(finstack_core::Error::CurrencyMismatch { expected: base_ccy, actual: notional.currency() });
            }
        }
        Ok(())
    }
}
