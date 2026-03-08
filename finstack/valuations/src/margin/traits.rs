//! Traits for marginable instruments.
//!
//! Defines the common interface for instruments that support margin calculations,
//! enabling uniform margin metric calculation and portfolio aggregation.

use crate::instruments::common_impl::traits::Instrument;
use crate::margin::types::OtcMarginSpec;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

// Re-export types that were previously defined here so existing
// `use crate::margin::traits::{...}` imports continue to work.
pub use crate::margin::types::{
    InstrumentMarginResult, NettingSetId, SimmCreditSector, SimmRiskClass, SimmSensitivities,
};

/// Trait for instruments that support margin calculations.
///
/// Implements this trait for instruments that can have margin requirements,
/// enabling uniform calculation of IM and VM across different instrument types.
///
/// # Implementors
///
/// - `InterestRateSwap` - OTC interest rate derivatives
/// - `CreditDefaultSwap` - OTC credit derivatives
/// - `CDSIndex` - Credit index derivatives
/// - `EquityTotalReturnSwap` - Equity TRS
/// - `FIIndexTotalReturnSwap` - Fixed income TRS
/// - `Repo` - Repurchase agreements
pub trait Marginable: Instrument {
    /// Get the margin specification for this instrument.
    ///
    /// Returns `None` if the instrument has no margin requirements configured.
    fn margin_spec(&self) -> Option<&OtcMarginSpec>;

    /// Get the repo margin specification (for repos only).
    ///
    /// Default implementation returns `None`. Override for repo instruments.
    fn repo_margin_spec(&self) -> Option<&crate::instruments::rates::repo::RepoMarginSpec> {
        None
    }

    /// Get the netting set identifier for margin aggregation.
    ///
    /// Instruments in the same netting set can offset each other.
    /// Returns `None` if the instrument is not part of a netting set.
    fn netting_set_id(&self) -> Option<NettingSetId>;

    /// Calculate SIMM sensitivities for this instrument.
    ///
    /// Returns the risk sensitivities needed for ISDA SIMM calculation.
    /// The sensitivities are used to calculate initial margin.
    ///
    /// # Arguments
    /// * `market` - Market data context
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// SIMM sensitivities or error if calculation fails
    fn simm_sensitivities(&self, market: &MarketContext, as_of: Date) -> Result<SimmSensitivities>;

    /// Get the current mark-to-market value for VM calculation.
    ///
    /// This is typically the NPV of the instrument.
    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money>;

    /// Check if margin is applicable for this instrument.
    ///
    /// Returns true if the instrument has margin requirements.
    fn has_margin(&self) -> bool {
        self.margin_spec().is_some() || self.repo_margin_spec().is_some()
    }
}
