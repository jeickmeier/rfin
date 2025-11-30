//! Credit rating types and rating factor tables.
//!
//! This module provides fundamental credit rating types used throughout the
//! financial system, including:
//!
//! - [`CreditRating`]: An agency-agnostic credit rating scale
//! - [`RatingFactorTable`]: Rating-to-factor mappings (e.g., Moody's WARF)
//!
//! # Rating Factors
//!
//! Rating factors are numerical values that correspond to expected default
//! probabilities for each rating category. The most commonly used is the
//! Moody's Weighted Average Rating Factor (WARF) system used in CLO analysis.
//!
//! # Examples
//!
//! ```rust
//! use finstack_core::types::ratings::{CreditRating, moodys_warf_factor};
//!
//! // Check investment grade status
//! assert!(CreditRating::BBB.is_investment_grade());
//! assert!(!CreditRating::BB.is_investment_grade());
//!
//! // Get WARF factor for a rating
//! let factor = moodys_warf_factor(CreditRating::B);
//! assert_eq!(factor, 2720.0);
//! ```

use std::collections::HashMap;
use std::sync::OnceLock;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// CREDIT RATING
// ============================================================================

/// Credit rating scale (agency-agnostic).
///
/// This enum represents a standardized credit rating scale that maps to
/// equivalent ratings from major agencies:
///
/// | `CreditRating` | S&P/Fitch | Moody's |
/// |----------------|-----------|---------|
/// | `AAA`          | AAA       | Aaa     |
/// | `AA`           | AA        | Aa      |
/// | `A`            | A         | A       |
/// | `BBB`          | BBB       | Baa     |
/// | `BB`           | BB        | Ba      |
/// | `B`            | B         | B       |
/// | `CCC`          | CCC       | Caa     |
/// | `CC`           | CC        | Ca      |
/// | `C`            | C         | C       |
/// | `D`            | D         | D       |
/// | `NR`           | NR        | NR      |
///
/// # Investment Grade
///
/// Ratings of `BBB` and above are considered "investment grade":
///
/// ```rust
/// use finstack_core::types::ratings::CreditRating;
///
/// assert!(CreditRating::A.is_investment_grade());
/// assert!(!CreditRating::BB.is_investment_grade());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CreditRating {
    /// AAA - Highest quality, minimal credit risk
    AAA,
    /// AA - Very high quality, very low credit risk
    AA,
    /// A - High quality, low credit risk
    A,
    /// BBB - Medium grade, moderate credit risk (lowest investment grade)
    BBB,
    /// BB - Speculative, substantial credit risk
    BB,
    /// B - Highly speculative, high credit risk
    B,
    /// CCC - Poor standing, very high credit risk
    CCC,
    /// CC - Highly vulnerable, near default
    CC,
    /// C - Lowest rated, typically in default
    C,
    /// D - In default
    D,
    /// NR - Not rated
    NR,
}

impl CreditRating {
    /// Check if rating is investment grade (BBB and above).
    ///
    /// Investment grade ratings indicate lower credit risk and are often
    /// required for institutional investors with regulatory constraints.
    ///
    /// # Example
    /// ```rust
    /// use finstack_core::types::ratings::CreditRating;
    ///
    /// assert!(CreditRating::BBB.is_investment_grade());
    /// assert!(!CreditRating::BB.is_investment_grade());
    /// ```
    pub fn is_investment_grade(&self) -> bool {
        matches!(self, Self::AAA | Self::AA | Self::A | Self::BBB)
    }

    /// Check if rating is speculative grade (below BBB).
    ///
    /// Speculative grade (or "junk") ratings indicate higher credit risk.
    ///
    /// # Example
    /// ```rust
    /// use finstack_core::types::ratings::CreditRating;
    ///
    /// assert!(CreditRating::BB.is_speculative_grade());
    /// assert!(!CreditRating::A.is_speculative_grade());
    /// ```
    pub fn is_speculative_grade(&self) -> bool {
        !self.is_investment_grade() && *self != Self::NR
    }

    /// Check if the rating indicates default status.
    ///
    /// # Example
    /// ```rust
    /// use finstack_core::types::ratings::CreditRating;
    ///
    /// assert!(CreditRating::D.is_default());
    /// assert!(!CreditRating::CCC.is_default());
    /// ```
    pub fn is_default(&self) -> bool {
        matches!(self, Self::D)
    }
}

impl core::fmt::Display for CreditRating {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CreditRating::AAA => write!(f, "AAA"),
            CreditRating::AA => write!(f, "AA"),
            CreditRating::A => write!(f, "A"),
            CreditRating::BBB => write!(f, "BBB"),
            CreditRating::BB => write!(f, "BB"),
            CreditRating::B => write!(f, "B"),
            CreditRating::CCC => write!(f, "CCC"),
            CreditRating::CC => write!(f, "CC"),
            CreditRating::C => write!(f, "C"),
            CreditRating::D => write!(f, "D"),
            CreditRating::NR => write!(f, "NR"),
        }
    }
}

impl core::str::FromStr for CreditRating {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "AAA" | "AAA+" | "AAA-" => Ok(CreditRating::AAA),
            "AA" | "AA+" | "AA-" | "AA1" | "AA2" | "AA3" => Ok(CreditRating::AA),
            "A" | "A+" | "A-" | "A1" | "A2" | "A3" => Ok(CreditRating::A),
            "BBB" | "BBB+" | "BBB-" | "BAA1" | "BAA2" | "BAA3" => Ok(CreditRating::BBB),
            "BB" | "BB+" | "BB-" | "BA1" | "BA2" | "BA3" => Ok(CreditRating::BB),
            "B" | "B+" | "B-" | "B1" | "B2" | "B3" => Ok(CreditRating::B),
            "CCC" | "CCC+" | "CCC-" | "CAA1" | "CAA2" | "CAA3" => Ok(CreditRating::CCC),
            "CC" | "CA" => Ok(CreditRating::CC),
            "C" => Ok(CreditRating::C),
            "D" | "DEFAULT" => Ok(CreditRating::D),
            "NR" | "NOT RATED" | "UNRATED" => Ok(CreditRating::NR),
            _ => Err(crate::error::InputError::InvalidRating {
                value: s.to_string(),
            }
            .into()),
        }
    }
}

// ============================================================================
// RATING FACTOR TABLE
// ============================================================================

/// Rating factor table for a specific rating agency methodology.
///
/// Rating factors are numerical values that correspond to expected cumulative
/// default rates over the life of an asset. They are used extensively in
/// structured credit analysis, particularly for CLO WARF calculations.
///
/// # Example
///
/// ```rust
/// use finstack_core::types::ratings::{CreditRating, RatingFactorTable};
///
/// let table = RatingFactorTable::moodys_standard();
/// let factor = table.get_factor(CreditRating::B);
/// assert_eq!(factor, 2720.0);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RatingFactorTable {
    /// Factors by rating
    factors: HashMap<CreditRating, f64>,
    /// Agency name
    agency: String,
    /// Methodology description
    methodology: String,
}

impl RatingFactorTable {
    /// Create Moody's standard WARF table.
    ///
    /// Based on Moody's IDEALIZED DEFAULT RATES table used in CLO analysis.
    /// These factors represent the expected cumulative default rate over the
    /// life of the asset for each rating category.
    ///
    /// Source: Moody's "Approach to Rating Collateralized Loan Obligations"
    ///
    /// # Example
    /// ```rust
    /// use finstack_core::types::ratings::{CreditRating, RatingFactorTable};
    ///
    /// let table = RatingFactorTable::moodys_standard();
    /// assert_eq!(table.get_factor(CreditRating::AAA), 1.0);
    /// assert_eq!(table.get_factor(CreditRating::B), 2720.0);
    /// ```
    pub fn moodys_standard() -> Self {
        let mut factors = HashMap::new();
        factors.insert(CreditRating::AAA, 1.0);
        factors.insert(CreditRating::AA, 10.0);
        factors.insert(CreditRating::A, 40.0);
        factors.insert(CreditRating::BBB, 260.0);
        factors.insert(CreditRating::BB, 1350.0);
        factors.insert(CreditRating::B, 2720.0);
        factors.insert(CreditRating::CCC, 6500.0);
        factors.insert(CreditRating::CC, 8070.0);
        factors.insert(CreditRating::C, 10000.0);
        factors.insert(CreditRating::D, 10000.0);
        factors.insert(CreditRating::NR, 3650.0); // B-/CCC+ equivalent

        Self {
            factors,
            agency: "Moody's".to_string(),
            methodology: "IDEALIZED DEFAULT RATES".to_string(),
        }
    }

    /// Get factor for a specific rating.
    ///
    /// Returns the factor for the given rating. If the rating is not found
    /// in the table, returns a default value of 3650.0 (B-/CCC+ equivalent).
    pub fn get_factor(&self, rating: CreditRating) -> f64 {
        self.factors.get(&rating).copied().unwrap_or(3650.0)
    }

    /// Get agency name.
    pub fn agency(&self) -> &str {
        &self.agency
    }

    /// Get methodology description.
    pub fn methodology(&self) -> &str {
        &self.methodology
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Lazily initialized Moody's WARF rating factor table
static MOODYS_WARF_TABLE: OnceLock<RatingFactorTable> = OnceLock::new();

/// Get Moody's WARF factor for a rating (convenience function).
///
/// This is the standard function that should be used throughout the codebase
/// for consistent WARF calculations. The table is lazily initialized on first
/// call and cached for subsequent calls.
///
/// # Example
/// ```rust
/// use finstack_core::types::ratings::{CreditRating, moodys_warf_factor};
///
/// assert_eq!(moodys_warf_factor(CreditRating::AAA), 1.0);
/// assert_eq!(moodys_warf_factor(CreditRating::B), 2720.0);
/// assert_eq!(moodys_warf_factor(CreditRating::CCC), 6500.0);
/// ```
pub fn moodys_warf_factor(rating: CreditRating) -> f64 {
    MOODYS_WARF_TABLE
        .get_or_init(RatingFactorTable::moodys_standard)
        .get_factor(rating)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credit_rating_investment_grade() {
        assert!(CreditRating::AAA.is_investment_grade());
        assert!(CreditRating::AA.is_investment_grade());
        assert!(CreditRating::A.is_investment_grade());
        assert!(CreditRating::BBB.is_investment_grade());
        assert!(!CreditRating::BB.is_investment_grade());
        assert!(!CreditRating::B.is_investment_grade());
        assert!(!CreditRating::CCC.is_investment_grade());
        assert!(!CreditRating::D.is_investment_grade());
        assert!(!CreditRating::NR.is_investment_grade());
    }

    #[test]
    fn test_credit_rating_speculative_grade() {
        assert!(!CreditRating::AAA.is_speculative_grade());
        assert!(!CreditRating::BBB.is_speculative_grade());
        assert!(CreditRating::BB.is_speculative_grade());
        assert!(CreditRating::B.is_speculative_grade());
        assert!(CreditRating::CCC.is_speculative_grade());
        assert!(CreditRating::D.is_speculative_grade());
        assert!(!CreditRating::NR.is_speculative_grade()); // NR is neither
    }

    #[test]
    fn test_credit_rating_default() {
        assert!(!CreditRating::CCC.is_default());
        assert!(CreditRating::D.is_default());
    }

    #[test]
    fn test_credit_rating_display() {
        assert_eq!(format!("{}", CreditRating::AAA), "AAA");
        assert_eq!(format!("{}", CreditRating::BB), "BB");
        assert_eq!(format!("{}", CreditRating::NR), "NR");
    }

    #[test]
    fn test_credit_rating_ordering() {
        // AAA is "best" rating, so it should be smallest in Ord
        assert!(CreditRating::AAA < CreditRating::AA);
        assert!(CreditRating::AA < CreditRating::A);
        assert!(CreditRating::BBB < CreditRating::BB);
        assert!(CreditRating::B < CreditRating::D);
    }

    #[test]
    fn test_credit_rating_from_str() {
        assert_eq!(
            "AAA".parse::<CreditRating>().expect("AAA should parse"),
            CreditRating::AAA
        );
        assert_eq!(
            "aaa".parse::<CreditRating>().expect("aaa should parse"),
            CreditRating::AAA
        );
        assert_eq!(
            "BB+".parse::<CreditRating>().expect("BB+ should parse"),
            CreditRating::BB
        );
        assert_eq!(
            "B1".parse::<CreditRating>().expect("B1 should parse"),
            CreditRating::B
        );
        assert_eq!(
            "NR".parse::<CreditRating>().expect("NR should parse"),
            CreditRating::NR
        );
        assert!("XYZ".parse::<CreditRating>().is_err());
    }

    #[test]
    fn test_moodys_warf_factors() {
        let table = RatingFactorTable::moodys_standard();

        // Verify key rating factors
        assert_eq!(table.get_factor(CreditRating::AAA), 1.0);
        assert_eq!(table.get_factor(CreditRating::A), 40.0);
        assert_eq!(table.get_factor(CreditRating::B), 2720.0);
        assert_eq!(table.get_factor(CreditRating::CCC), 6500.0);
    }

    #[test]
    fn test_convenience_function_matches_table() {
        let table = RatingFactorTable::moodys_standard();

        // Verify convenience function matches table
        assert_eq!(
            moodys_warf_factor(CreditRating::AAA),
            table.get_factor(CreditRating::AAA)
        );
        assert_eq!(
            moodys_warf_factor(CreditRating::A),
            table.get_factor(CreditRating::A)
        );
        assert_eq!(
            moodys_warf_factor(CreditRating::BBB),
            table.get_factor(CreditRating::BBB)
        );
        assert_eq!(
            moodys_warf_factor(CreditRating::B),
            table.get_factor(CreditRating::B)
        );
    }

    #[test]
    fn test_rating_factor_table_metadata() {
        let table = RatingFactorTable::moodys_standard();
        assert_eq!(table.agency(), "Moody's");
        assert_eq!(table.methodology(), "IDEALIZED DEFAULT RATES");
    }
}
