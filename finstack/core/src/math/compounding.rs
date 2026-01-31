//! Compounding convention definitions for interest rate calculations.
//!
//! This module defines the [`Compounding`] enum which specifies how interest rates
//! should be quoted or converted. Different markets and instruments use different
//! compounding conventions, and this enum provides a unified way to specify the
//! desired convention.
//!
//! # Market Conventions
//!
//! | Convention | Common Usage | Formula (rate from DF) |
//! |------------|--------------|------------------------|
//! | Continuous | Internal calculations, curve construction | r = -ln(DF) / t |
//! | Annual | Bond markets (UK, Europe) | r = DF^(-1/t) - 1 |
//! | Semi-Annual | US Treasury, corporate bonds | r = 2 × (DF^(-1/(2t)) - 1) |
//! | Quarterly | Some floating rate notes | r = 4 × (DF^(-1/(4t)) - 1) |
//! | Simple | Money market (< 1Y), deposits | r = (1/DF - 1) / t |
//!
//! # Examples
//!
//! ```rust
//! use finstack_core::math::Compounding;
//! use std::num::NonZeroU32;
//!
//! // Continuous compounding (most common for quant calculations)
//! let cont = Compounding::Continuous;
//!
//! // Semi-annual compounding (US Treasury convention)
//! let semi = Compounding::SEMI_ANNUAL;
//!
//! // Custom periodic compounding
//! let monthly = Compounding::Periodic(NonZeroU32::new(12).unwrap());
//!
//! // Simple interest (money market)
//! let simple = Compounding::Simple;
//! ```
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.).
//!   Pearson. Chapter 4 (Interest Rates).
//! - Brigo, D., & Mercurio, F. (2006). *Interest Rate Models - Theory and Practice*
//!   (2nd ed.). Springer. Chapter 1 (Definitions and Notation).

use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Compounding convention for interest rates.
///
/// Used to specify how interest rates should be quoted or converted.
/// All variants produce mathematically equivalent discount factors when
/// applied consistently.
///
/// # Relationship Between Conventions
///
/// For a given discount factor DF at time t, the rates under different
/// conventions are related by:
///
/// ```text
/// DF = e^(-r_cc × t)                    [Continuous]
///    = (1 + r_ann)^(-t)                 [Annual]
///    = (1 + r_per/n)^(-n×t)             [Periodic(n)]
///    = 1 / (1 + r_simple × t)           [Simple]
/// ```
///
/// # Ordering of Rates
///
/// For positive rates and t > 0: `r_simple > r_annual > r_continuous`
/// (less frequent compounding requires a higher quoted rate for the same DF).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Compounding {
    /// Continuous compounding: r = -ln(DF) / t
    ///
    /// Most common for internal calculations, curve construction, and
    /// quantitative finance models. Provides the simplest mathematical
    /// properties (additive over time).
    Continuous,

    /// Annual compounding: r = DF^(-1/t) - 1
    ///
    /// Standard for many bond markets, particularly UK gilts and
    /// European government bonds. Also common for Bloomberg zero rate display.
    Annual,

    /// Periodic compounding with n periods per year: r = n × (DF^(-1/(n×t)) - 1)
    ///
    /// Common values:
    /// - n=2: Semi-annual (US Treasury, corporate bonds)
    /// - n=4: Quarterly (some FRNs)
    /// - n=12: Monthly (rare, some retail products)
    ///
    /// Note: Uses [`NonZeroU32`] to prevent division by zero at compile time.
    Periodic(NonZeroU32),

    /// Simple interest (no compounding): r = (1/DF - 1) / t
    ///
    /// Used for money market instruments with maturity < 1 year, including:
    /// - Interbank deposits
    /// - T-bills and commercial paper
    /// - SOFR, SONIA, €STR fixings
    ///
    /// Typically paired with ACT/360 (USD, EUR) or ACT/365F (GBP) day counts.
    Simple,
}

/// Helper const fn to create NonZeroU32 in const context (panics if n is 0).
const fn nonzero_u32(n: u32) -> NonZeroU32 {
    match NonZeroU32::new(n) {
        Some(v) => v,
        None => panic!("Value must be non-zero"),
    }
}

impl Compounding {
    /// Semi-annual compounding (n=2).
    ///
    /// Standard for US Treasury bonds and most US corporate bonds.
    pub const SEMI_ANNUAL: Self = Compounding::Periodic(nonzero_u32(2));

    /// Quarterly compounding (n=4).
    ///
    /// Used for some floating rate notes and quarterly-paying bonds.
    pub const QUARTERLY: Self = Compounding::Periodic(nonzero_u32(4));

    /// Monthly compounding (n=12).
    ///
    /// Relatively rare in wholesale markets; more common in retail products.
    pub const MONTHLY: Self = Compounding::Periodic(nonzero_u32(12));

    /// Returns the number of compounding periods per year, if applicable.
    ///
    /// - `Periodic(n)` returns `Some(n)`
    /// - `Annual` returns `Some(1)` (equivalent to `Periodic(1)`)
    /// - `Continuous` and `Simple` return `None`
    #[must_use]
    pub fn periods_per_year(&self) -> Option<u32> {
        match self {
            Compounding::Annual => Some(1),
            Compounding::Periodic(n) => Some(n.get()),
            Compounding::Continuous | Compounding::Simple => None,
        }
    }

    /// Returns true if this is a periodic compounding convention (including Annual).
    #[must_use]
    pub fn is_periodic(&self) -> bool {
        matches!(self, Compounding::Annual | Compounding::Periodic(_))
    }
}

impl Default for Compounding {
    /// Default to continuous compounding (most common for quant finance).
    fn default() -> Self {
        Compounding::Continuous
    }
}

impl std::fmt::Display for Compounding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Compounding::Continuous => write!(f, "Continuous"),
            Compounding::Annual => write!(f, "Annual"),
            Compounding::Periodic(n) => write!(f, "Periodic({})", n),
            Compounding::Simple => write!(f, "Simple"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_compounding_constants() {
        assert_eq!(
            Compounding::SEMI_ANNUAL,
            Compounding::Periodic(NonZeroU32::new(2).expect("2 is non-zero"))
        );
        assert_eq!(
            Compounding::QUARTERLY,
            Compounding::Periodic(NonZeroU32::new(4).expect("4 is non-zero"))
        );
        assert_eq!(
            Compounding::MONTHLY,
            Compounding::Periodic(NonZeroU32::new(12).expect("12 is non-zero"))
        );
    }

    #[test]
    fn test_periods_per_year() {
        assert_eq!(Compounding::Continuous.periods_per_year(), None);
        assert_eq!(Compounding::Annual.periods_per_year(), Some(1));
        assert_eq!(Compounding::SEMI_ANNUAL.periods_per_year(), Some(2));
        assert_eq!(Compounding::QUARTERLY.periods_per_year(), Some(4));
        assert_eq!(Compounding::MONTHLY.periods_per_year(), Some(12));
        assert_eq!(Compounding::Simple.periods_per_year(), None);
    }

    #[test]
    fn test_is_periodic() {
        assert!(!Compounding::Continuous.is_periodic());
        assert!(Compounding::Annual.is_periodic());
        assert!(Compounding::SEMI_ANNUAL.is_periodic());
        assert!(!Compounding::Simple.is_periodic());
    }

    #[test]
    fn test_default() {
        assert_eq!(Compounding::default(), Compounding::Continuous);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Compounding::Continuous), "Continuous");
        assert_eq!(format!("{}", Compounding::Annual), "Annual");
        assert_eq!(format!("{}", Compounding::SEMI_ANNUAL), "Periodic(2)");
        assert_eq!(format!("{}", Compounding::Simple), "Simple");
    }

    #[test]
    fn test_serde_roundtrip() {
        let variants = [
            Compounding::Continuous,
            Compounding::Annual,
            Compounding::SEMI_ANNUAL,
            Compounding::Simple,
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).expect("serialize");
            let deserialized: Compounding = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(variant, deserialized);
        }
    }
}
