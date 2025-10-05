//! Rating factor tables for structured credit metrics.
//!
//! Provides standardized rating factor tables from major rating agencies
//! for use in WARF, diversity score, and other credit metrics calculations.

use super::enums::CreditRating;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Rating factor table for a specific rating agency methodology
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RatingFactorTable {
    /// Factors by rating
    factors: std::collections::HashMap<CreditRating, f64>,
    /// Agency name
    agency: String,
    /// Methodology description
    methodology: String,
}

impl RatingFactorTable {
    /// Create Moody's standard WARF table
    ///
    /// Based on Moody's IDEALIZED DEFAULT RATES table used in CLO analysis.
    /// These factors represent the expected cumulative default rate over the
    /// life of the asset for each rating category.
    ///
    /// Source: Moody's "Approach to Rating Collateralized Loan Obligations"
    pub fn moodys_standard() -> Self {
        let mut factors = std::collections::HashMap::new();
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

    /// Get factor for a specific rating
    pub fn get_factor(&self, rating: CreditRating) -> f64 {
        self.factors.get(&rating).copied().unwrap_or(3650.0)
    }

    /// Get agency name
    pub fn agency(&self) -> &str {
        &self.agency
    }

    /// Get methodology description
    pub fn methodology(&self) -> &str {
        &self.methodology
    }
}

/// Get Moody's WARF factor for a rating (convenience function)
///
/// This is the standard function that should be used throughout the codebase
/// for consistent WARF calculations.
pub fn moodys_warf_factor(rating: CreditRating) -> f64 {
    match rating {
        CreditRating::AAA => 1.0,
        CreditRating::AA => 10.0,
        CreditRating::A => 40.0,
        CreditRating::BBB => 260.0,
        CreditRating::BB => 1350.0,
        CreditRating::B => 2720.0,
        CreditRating::CCC => 6500.0,
        CreditRating::CC => 8070.0,
        CreditRating::C => 10000.0,
        CreditRating::D => 10000.0,
        CreditRating::NR => 3650.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
