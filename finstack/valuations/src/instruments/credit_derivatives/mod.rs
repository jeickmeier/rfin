//! Credit derivatives: CDS and related instruments.

/// CDS module - Single-name credit default swaps.
pub mod cds;
/// CDS index module - Credit indices (CDX, iTraxx).
pub mod cds_index;
/// CDS option module - Options on CDS spreads.
pub mod cds_option;
/// CDS tranche module - Synthetic CDO tranches.
pub mod cds_tranche;

// Re-export primary types
pub use cds::CreditDefaultSwap;
pub use cds_index::CDSIndex;
pub use cds_option::CdsOption;
pub use cds_tranche::CdsTranche;
