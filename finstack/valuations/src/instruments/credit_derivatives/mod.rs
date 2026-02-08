//! Credit derivatives: CDS and related instruments.
//!
//! This module provides credit derivative instruments following ISDA standard
//! conventions. All instruments are priced using hazard rate curves derived
//! from market CDS spreads via the ISDA CDS Standard Model.
//!
//! # Features
//!
//! - **Single-Name CDS**: Credit protection on individual reference entities
//! - **CDS Indices**: Portfolio credit exposure (CDX.NA.IG, CDX.NA.HY, iTraxx)
//! - **CDS Tranches**: Mezzanine credit exposure via synthetic CDOs
//! - **CDS Options**: Volatility exposure on CDS spreads (payer/receiver swaptions)
//!
//! # Pricing Framework
//!
//! Credit derivatives are priced using:
//! - **Hazard rate curves**: Bootstrapped from CDS spread quotes
//! - **Recovery rates**: Standard 40% for senior, 20% for subordinated
//! - **Accrual-on-default**: ISDA standard midpoint approximation
//!
//! # ISDA Conventions
//!
//! Post-Big Bang (2009) standardization:
//! - Standard coupons: 100bp or 500bp running spread
//! - IMM maturities: 20th of Mar/Jun/Sep/Dec
//! - Day count: ACT/360 for premium leg
//! - Settlement: Cash (auction) or physical delivery
//!
//! # Quick Example
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::credit_derivatives::CreditDefaultSwap;
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::types::InstrumentId;
//! use time::macros::date;
//!
//! let cds = CreditDefaultSwap::builder()
//!     .id(InstrumentId::new("CDS-AAPL-5Y"))
//!     .reference_entity("AAPL".to_string())
//!     .notional(Money::new(10_000_000.0, Currency::USD))
//!     .effective_date(date!(2025-03-20))
//!     .maturity(date!(2030-03-20))
//!     .running_spread_bps(100.0)
//!     .discount_curve_id("USD-OIS".into())
//!     .hazard_curve_id("AAPL-CDS".into())
//!     .build();
//! ```
//!
//! # References
//!
//! - ISDA CDS Standard Model v1.8.2 (October 2009)
//! - O'Kane, D. (2008). *Modelling Single-name and Multi-name Credit Derivatives*
//!
//! # See Also
//!
//! - [`CreditDefaultSwap`] for single-name CDS
//! - [`CDSIndex`] for credit indices
//! - [`CDSTranche`] for synthetic CDO tranches
//! - [`crate::calibration::targets::hazard`] for hazard curve calibration

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
pub use cds_option::CDSOption;
pub use cds_tranche::CDSTranche;
