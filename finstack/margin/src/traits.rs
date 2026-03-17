//! Traits for marginable instruments.
//!
//! Defines the common interface for instruments that support margin calculations,
//! enabling uniform margin metric calculation and portfolio aggregation.

use crate::types::repo_margin::RepoMarginSpec;
use crate::types::OtcMarginSpec;
use crate::types::{NettingSetId, SimmSensitivities};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Trait for instruments that support margin calculations.
///
/// This is a standalone trait so `finstack-margin` has no dependency on
/// `finstack-valuations`. Concrete implementations live in valuations as a
/// bridge layer.
pub trait Marginable: Send + Sync {
    /// Get the instrument's unique identifier.
    fn id(&self) -> &str;

    /// Get the OTC margin specification for this instrument.
    fn margin_spec(&self) -> Option<&OtcMarginSpec>;

    /// Get the repo margin specification for this instrument.
    ///
    /// Default implementation returns `None`. Override for repo instruments.
    fn repo_margin_spec(&self) -> Option<&RepoMarginSpec> {
        None
    }

    /// Get the netting set identifier for margin aggregation.
    fn netting_set_id(&self) -> Option<NettingSetId>;

    /// Calculate SIMM sensitivities for this instrument.
    fn simm_sensitivities(&self, market: &MarketContext, as_of: Date) -> Result<SimmSensitivities>;

    /// Get the current mark-to-market value used for margin calculations.
    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money>;

    /// Check if margin applies to this instrument.
    fn has_margin(&self) -> bool {
        self.margin_spec().is_some() || self.repo_margin_spec().is_some()
    }
}
