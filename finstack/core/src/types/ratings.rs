//! Credit rating types and rating factor tables.
//!
//! This module provides fundamental credit rating types used throughout the
//! financial system, including:
//!
//! - [`CreditRating`]: An agency-agnostic credit rating scale
//! - [`NotchedRating`]: A `CreditRating` plus notch metadata (`+`, flat, `-`)
//! - [`RatingFactorTable`]: Rating-to-factor mappings (e.g., Moody's WARF)
//! - [`RatingLabel`]: Stable, display-ready labels sourced from `NotchedRating`
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
//! use finstack_core::types::{CreditRating, moodys_warf_factor};
//!
//! // Check investment grade status
//! assert!(CreditRating::BBB.is_investment_grade());
//! assert!(!CreditRating::BB.is_investment_grade());
//!
//! // Get WARF factor for a rating
//! let factor = moodys_warf_factor(CreditRating::B).unwrap();
//! assert_eq!(factor, 2720.0);
//! ```

use crate::collections::HashMap;
use std::sync::OnceLock;

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
/// use finstack_core::types::CreditRating;
///
/// assert!(CreditRating::A.is_investment_grade());
/// assert!(!CreditRating::BB.is_investment_grade());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
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
    /// Numeric ordinal for ordering. Lower values indicate higher credit quality.
    /// NR (Not Rated) is placed between C and D in the ordering.
    fn ordinal(&self) -> u8 {
        match self {
            CreditRating::AAA => 0,
            CreditRating::AA => 1,
            CreditRating::A => 2,
            CreditRating::BBB => 3,
            CreditRating::BB => 4,
            CreditRating::B => 5,
            CreditRating::CCC => 6,
            CreditRating::CC => 7,
            CreditRating::C => 8,
            CreditRating::NR => 9,
            CreditRating::D => 10,
        }
    }
}

impl PartialOrd for CreditRating {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CreditRating {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ordinal().cmp(&other.ordinal())
    }
}

impl CreditRating {
    /// Check if rating is investment grade (BBB and above).
    ///
    /// Investment grade ratings indicate lower credit risk and are often
    /// required for institutional investors with regulatory constraints.
    ///
    /// # Example
    /// ```rust
    /// use finstack_core::types::CreditRating;
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
    /// use finstack_core::types::CreditRating;
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
    /// use finstack_core::types::CreditRating;
    ///
    /// assert!(CreditRating::D.is_default());
    /// assert!(!CreditRating::CCC.is_default());
    /// ```
    pub fn is_default(&self) -> bool {
        matches!(self, Self::D)
    }

    /// Returns `true` if the rating supports notch adjustments.
    pub fn supports_notches(&self) -> bool {
        matches!(
            self,
            Self::AA | Self::A | Self::BBB | Self::BB | Self::B | Self::CCC
        )
    }

    /// Create a [`NotchedRating`] from this base rating.
    pub fn with_notch(self, notch: RatingNotch) -> NotchedRating {
        let notch = if self.supports_notches() {
            notch
        } else {
            RatingNotch::Flat
        };
        NotchedRating::new(self, notch)
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

// ============================================================================
// RATING NOTCHES
// ============================================================================

/// Notch adjustment for a [`CreditRating`].
///
/// Most agencies subdivide each letter rating into three notches. We normalize
/// those subdivisions into `Plus`, `Flat`, and `Minus`, which map to:
///
/// - `Plus`: Best notch (e.g., `AA+`, `Aa1`, `B1`)
/// - `Flat`: Middle notch (e.g., `AA`, `Aa2`, `B2`)
/// - `Minus`: Lowest notch within the grade (e.g., `AA-`, `Aa3`, `B3`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RatingNotch {
    /// Best notch (AA+, Aa1, etc.)
    Plus,
    /// Mid-notch (AA, Aa2, etc.)
    Flat,
    /// Weakest notch within the letter grade (AA-, Aa3, etc.)
    Minus,
}

impl RatingNotch {
    /// Symbol representing the notch.
    pub fn symbol(self) -> &'static str {
        match self {
            RatingNotch::Plus => "+",
            RatingNotch::Flat => "",
            RatingNotch::Minus => "-",
        }
    }
}

// ============================================================================
// NOTCHED RATING
// ============================================================================

/// A credit rating plus notch metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NotchedRating {
    base: CreditRating,
    notch: RatingNotch,
}

impl NotchedRating {
    /// Create a new notched rating.
    pub const fn new(base: CreditRating, notch: RatingNotch) -> Self {
        Self { base, notch }
    }

    /// Base letter rating without the notch.
    pub const fn base(self) -> CreditRating {
        self.base
    }

    /// Notch modifier.
    pub const fn notch(self) -> RatingNotch {
        self.notch
    }

    /// Returns a copy of this rating with the notch reset to flat.
    pub const fn without_notch(self) -> Self {
        Self::new(self.base, RatingNotch::Flat)
    }

    /// Whether the rating is investment grade.
    pub fn is_investment_grade(&self) -> bool {
        self.base.is_investment_grade()
    }

    /// Whether the rating is speculative grade.
    pub fn is_speculative_grade(&self) -> bool {
        self.base.is_speculative_grade()
    }

    /// Whether the rating indicates default.
    pub fn is_default(&self) -> bool {
        self.base.is_default()
    }

    /// Generic representation (S&P/Fitch style) used by [`core::fmt::Display`].
    fn to_generic_string(self) -> String {
        if matches!(self.base, CreditRating::NR) {
            return "NR".to_string();
        }
        if matches!(self.base, CreditRating::D) {
            return "D".to_string();
        }

        match self.notch {
            RatingNotch::Plus => format!("{}+", self.base),
            RatingNotch::Flat => format!("{}", self.base),
            RatingNotch::Minus => format!("{}-", self.base),
        }
    }

    /// Moody's-style representation (e.g., `Aa1`, `Ba2`, `B3`).
    pub fn to_moodys_string(self) -> String {
        match (self.base, self.notch) {
            (CreditRating::AAA, _) => "Aaa".to_string(),
            (CreditRating::AA, RatingNotch::Plus) => "Aa1".to_string(),
            (CreditRating::AA, RatingNotch::Flat) => "Aa2".to_string(),
            (CreditRating::AA, RatingNotch::Minus) => "Aa3".to_string(),
            (CreditRating::A, RatingNotch::Plus) => "A1".to_string(),
            (CreditRating::A, RatingNotch::Flat) => "A2".to_string(),
            (CreditRating::A, RatingNotch::Minus) => "A3".to_string(),
            (CreditRating::BBB, RatingNotch::Plus) => "Baa1".to_string(),
            (CreditRating::BBB, RatingNotch::Flat) => "Baa2".to_string(),
            (CreditRating::BBB, RatingNotch::Minus) => "Baa3".to_string(),
            (CreditRating::BB, RatingNotch::Plus) => "Ba1".to_string(),
            (CreditRating::BB, RatingNotch::Flat) => "Ba2".to_string(),
            (CreditRating::BB, RatingNotch::Minus) => "Ba3".to_string(),
            (CreditRating::B, RatingNotch::Plus) => "B1".to_string(),
            (CreditRating::B, RatingNotch::Flat) => "B2".to_string(),
            (CreditRating::B, RatingNotch::Minus) => "B3".to_string(),
            (CreditRating::CCC, RatingNotch::Plus) => "Caa1".to_string(),
            (CreditRating::CCC, RatingNotch::Flat) => "Caa2".to_string(),
            (CreditRating::CCC, RatingNotch::Minus) => "Caa3".to_string(),
            (CreditRating::CC, _) => "Ca".to_string(),
            (CreditRating::C, _) => "C".to_string(),
            (CreditRating::D, _) => "D".to_string(),
            (CreditRating::NR, _) => "NR".to_string(),
        }
    }
}

impl From<CreditRating> for NotchedRating {
    fn from(value: CreditRating) -> Self {
        Self::new(value, RatingNotch::Flat)
    }
}

impl core::fmt::Display for NotchedRating {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&(*self).to_generic_string())
    }
}

// ============================================================================
// RATING LABEL
// ============================================================================

/// Stable label for referring to ratings (curve names, exports, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RatingLabel(String);

impl RatingLabel {
    /// Create a label using the generic (S&P/Fitch-style) string.
    pub fn generic(rating: NotchedRating) -> Self {
        Self(rating.to_generic_string())
    }

    /// Create a label using Moody's naming convention.
    pub fn moodys(rating: NotchedRating) -> Self {
        Self(rating.to_moodys_string())
    }

    /// Access the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the label and return the owned string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<NotchedRating> for RatingLabel {
    fn from(value: NotchedRating) -> Self {
        Self::generic(value)
    }
}

impl core::fmt::Display for RatingLabel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl core::str::FromStr for CreditRating {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<NotchedRating>().map(|rating| rating.base())
    }
}

impl core::str::FromStr for NotchedRating {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_notched_rating(s)
    }
}

fn parse_notched_rating(value: &str) -> Result<NotchedRating, crate::Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(crate::error::InputError::InvalidRating {
            value: value.to_string(),
        }
        .into());
    }

    let normalized = trimmed.replace(' ', "");
    if normalized.is_empty() {
        return Err(crate::error::InputError::InvalidRating {
            value: value.to_string(),
        }
        .into());
    }

    let upper = normalized.to_uppercase();

    if matches!(upper.as_str(), "NR" | "NOTRATED" | "UNRATED") {
        return Ok(NotchedRating::new(CreditRating::NR, RatingNotch::Flat));
    }

    if matches!(upper.as_str(), "D" | "DEFAULT") {
        return Ok(NotchedRating::new(CreditRating::D, RatingNotch::Flat));
    }

    let mut notch = RatingNotch::Flat;
    let mut base_slice = upper.as_str();

    if base_slice.ends_with('+') {
        notch = RatingNotch::Plus;
        base_slice = &base_slice[..base_slice.len() - 1];
    } else if base_slice.ends_with('-') {
        notch = RatingNotch::Minus;
        base_slice = &base_slice[..base_slice.len() - 1];
    } else if let Some(last) = base_slice.chars().last() {
        if last.is_ascii_digit() {
            notch = match last {
                '1' => RatingNotch::Plus,
                '2' => RatingNotch::Flat,
                '3' => RatingNotch::Minus,
                _ => {
                    return Err(crate::error::InputError::InvalidRating {
                        value: value.to_string(),
                    }
                    .into())
                }
            };
            base_slice = &base_slice[..base_slice.len() - 1];
        }
    }

    if base_slice.is_empty() {
        return Err(crate::error::InputError::InvalidRating {
            value: value.to_string(),
        }
        .into());
    }

    let base = match base_slice {
        "AAA" => CreditRating::AAA,
        "AA" => CreditRating::AA,
        "A" => CreditRating::A,
        "BBB" | "BAA" => CreditRating::BBB,
        "BB" | "BA" => CreditRating::BB,
        "B" => CreditRating::B,
        "CCC" | "CAA" => CreditRating::CCC,
        "CC" | "CA" => CreditRating::CC,
        "C" => CreditRating::C,
        "D" => CreditRating::D,
        "NR" => CreditRating::NR,
        _ => {
            return Err(crate::error::InputError::InvalidRating {
                value: value.to_string(),
            }
            .into())
        }
    };

    let notch = if base.supports_notches() {
        notch
    } else {
        RatingNotch::Flat
    };

    Ok(NotchedRating::new(base, notch))
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
/// use finstack_core::types::{CreditRating, RatingFactorTable};
///
/// let table = RatingFactorTable::moodys_standard();
/// let factor = table.get_factor(CreditRating::B).unwrap();
/// assert_eq!(factor, 2720.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingFactorTable {
    /// Factors by notched rating
    factors: HashMap<NotchedRating, f64>,
    /// Agency name
    agency: String,
    /// Methodology description
    methodology: String,
    /// Default factor when a rating is missing
    default_factor: f64,
}

impl RatingFactorTable {
    /// Create Moody's standard WARF table.
    ///
    /// Based on Moody's IDEALIZED DEFAULT RATES table used in CLO analysis.
    /// The table includes every notch published by Moody's as of September
    /// 2024.
    ///
    /// Source: Moody's "Approach to Rating Collateralized Loan Obligations"
    ///
    /// # Example
    /// ```rust
    /// use finstack_core::types::{CreditRating, RatingFactorTable};
    ///
    /// let table = RatingFactorTable::moodys_standard();
    /// assert_eq!(table.get_factor(CreditRating::AAA).unwrap(), 1.0);
    /// assert_eq!(table.get_factor(CreditRating::B).unwrap(), 2720.0);
    /// ```
    pub fn moodys_standard() -> Self {
        let mut factors = HashMap::default();
        insert_factor(&mut factors, CreditRating::AAA, RatingNotch::Flat, 1.0);
        insert_factor(&mut factors, CreditRating::AA, RatingNotch::Plus, 10.0);
        insert_factor(&mut factors, CreditRating::AA, RatingNotch::Flat, 20.0);
        insert_factor(&mut factors, CreditRating::AA, RatingNotch::Minus, 40.0);
        insert_factor(&mut factors, CreditRating::A, RatingNotch::Plus, 70.0);
        insert_factor(&mut factors, CreditRating::A, RatingNotch::Flat, 120.0);
        insert_factor(&mut factors, CreditRating::A, RatingNotch::Minus, 180.0);
        insert_factor(&mut factors, CreditRating::BBB, RatingNotch::Plus, 260.0);
        insert_factor(&mut factors, CreditRating::BBB, RatingNotch::Flat, 360.0);
        insert_factor(&mut factors, CreditRating::BBB, RatingNotch::Minus, 610.0);
        insert_factor(&mut factors, CreditRating::BB, RatingNotch::Plus, 940.0);
        insert_factor(&mut factors, CreditRating::BB, RatingNotch::Flat, 1350.0);
        insert_factor(&mut factors, CreditRating::BB, RatingNotch::Minus, 1760.0);
        insert_factor(&mut factors, CreditRating::B, RatingNotch::Plus, 2220.0);
        insert_factor(&mut factors, CreditRating::B, RatingNotch::Flat, 2720.0);
        insert_factor(&mut factors, CreditRating::B, RatingNotch::Minus, 3490.0);
        insert_factor(&mut factors, CreditRating::CCC, RatingNotch::Plus, 4770.0);
        insert_factor(&mut factors, CreditRating::CCC, RatingNotch::Flat, 6500.0);
        insert_factor(&mut factors, CreditRating::CCC, RatingNotch::Minus, 8070.0);
        insert_factor(&mut factors, CreditRating::CC, RatingNotch::Flat, 9550.0);
        insert_factor(&mut factors, CreditRating::C, RatingNotch::Flat, 10000.0);
        insert_factor(&mut factors, CreditRating::D, RatingNotch::Flat, 10000.0);
        insert_factor(&mut factors, CreditRating::NR, RatingNotch::Flat, 3650.0); // B-/CCC+ equivalent

        Self {
            factors,
            agency: "Moody's".to_string(),
            methodology: "IDEALIZED DEFAULT RATES".to_string(),
            default_factor: 3650.0,
        }
    }

    /// Get factor for a specific rating (with optional notch precision).
    ///
    /// If the exact notch is missing, falls back to the flat notch for the same
    /// base rating. If no entry exists, returns an error instead of silently
    /// substituting the table default.
    pub fn get_factor<R>(&self, rating: R) -> crate::Result<f64>
    where
        R: Into<NotchedRating>,
    {
        let rating = rating.into();
        self.factors
            .get(&rating)
            .copied()
            .or_else(|| self.factors.get(&rating.without_notch()).copied())
            .ok_or_else(|| {
                crate::Error::Input(crate::error::InputError::NotFound {
                    id: format!("rating factor for {}", rating),
                })
            })
    }

    /// Get factor for a specific rating, falling back to the table default when missing.
    pub fn get_factor_or_default<R>(&self, rating: R) -> f64
    where
        R: Into<NotchedRating>,
    {
        let rating = rating.into();
        self.get_factor(rating).unwrap_or(self.default_factor)
    }

    /// Get agency name.
    pub fn agency(&self) -> &str {
        &self.agency
    }

    /// Get methodology description.
    pub fn methodology(&self) -> &str {
        &self.methodology
    }

    /// Get the table default factor used for missing entries.
    pub fn default_factor(&self) -> f64 {
        self.default_factor
    }
}

fn insert_factor(
    map: &mut HashMap<NotchedRating, f64>,
    base: CreditRating,
    notch: RatingNotch,
    value: f64,
) {
    map.insert(base.with_notch(notch), value);
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
/// use finstack_core::types::{CreditRating, RatingNotch, moodys_warf_factor};
///
/// assert_eq!(moodys_warf_factor(CreditRating::AAA).unwrap(), 1.0);
/// assert_eq!(moodys_warf_factor(CreditRating::B).unwrap(), 2720.0);
/// assert_eq!(
///     moodys_warf_factor(CreditRating::BBB.with_notch(RatingNotch::Plus)).unwrap(),
///     260.0
/// );
/// ```
pub fn moodys_warf_factor<R>(rating: R) -> crate::Result<f64>
where
    R: Into<NotchedRating>,
{
    MOODYS_WARF_TABLE
        .get_or_init(RatingFactorTable::moodys_standard)
        .get_factor(rating)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::collections::HashMap;

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
    fn test_notched_rating_investment_and_speculative() {
        let bbb_minus = CreditRating::BBB.with_notch(RatingNotch::Minus);
        let bb_plus = CreditRating::BB.with_notch(RatingNotch::Plus);

        assert!(bbb_minus.is_investment_grade());
        assert!(!bbb_minus.is_speculative_grade());

        assert!(bb_plus.is_speculative_grade());
        assert!(!bb_plus.is_investment_grade());
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
    fn test_notched_rating_from_str() {
        let bb_plus = "BB+".parse::<NotchedRating>().expect("BB+");
        assert_eq!(bb_plus.base(), CreditRating::BB);
        assert_eq!(bb_plus.notch(), RatingNotch::Plus);

        let b_minus = "B-".parse::<NotchedRating>().expect("B-");
        assert_eq!(b_minus.base(), CreditRating::B);
        assert_eq!(b_minus.notch(), RatingNotch::Minus);

        let b1 = "B1".parse::<NotchedRating>().expect("B1");
        assert_eq!(b1.base(), CreditRating::B);
        assert_eq!(b1.notch(), RatingNotch::Plus);

        let ba2 = "Ba2".parse::<NotchedRating>().expect("Ba2");
        assert_eq!(ba2.base(), CreditRating::BB);
        assert_eq!(ba2.notch(), RatingNotch::Flat);

        let nr = "Not Rated"
            .parse::<NotchedRating>()
            .expect("NR should parse");
        assert_eq!(nr.base(), CreditRating::NR);
        assert_eq!(nr.notch(), RatingNotch::Flat);
    }

    #[test]
    fn test_notched_rating_ordering() {
        let aa_plus = CreditRating::AA.with_notch(RatingNotch::Plus);
        let aa_flat = CreditRating::AA.with_notch(RatingNotch::Flat);
        let aa_minus = CreditRating::AA.with_notch(RatingNotch::Minus);

        assert!(aa_plus < aa_flat);
        assert!(aa_flat < aa_minus);
        assert!(aa_minus < CreditRating::A.with_notch(RatingNotch::Plus));
    }

    #[test]
    fn test_rating_labels() {
        let rating = CreditRating::BBB.with_notch(RatingNotch::Minus);
        let generic = RatingLabel::generic(rating);
        let moodys = RatingLabel::moodys(rating);

        assert_eq!(generic.as_str(), "BBB-");
        assert_eq!(moodys.as_str(), "Baa3");
        assert_eq!(generic.to_string(), "BBB-");
        assert_eq!(moodys.into_inner(), "Baa3".to_string());
    }

    #[test]
    fn test_moodys_warf_factors() {
        let table = RatingFactorTable::moodys_standard();

        // Verify key rating factors
        assert_eq!(
            table
                .get_factor(CreditRating::AAA)
                .expect("AAA should have a Moody's WARF factor"),
            1.0
        );
        assert_eq!(
            table
                .get_factor(CreditRating::AA.with_notch(RatingNotch::Plus))
                .expect("AA+ should have a Moody's WARF factor"),
            10.0
        );
        assert_eq!(
            table
                .get_factor(CreditRating::A)
                .expect("A should have a Moody's WARF factor"),
            120.0
        );
        assert_eq!(
            table
                .get_factor(CreditRating::B.with_notch(RatingNotch::Minus))
                .expect("B- should have a Moody's WARF factor"),
            3490.0
        );
        assert_eq!(
            table
                .get_factor(CreditRating::CCC.with_notch(RatingNotch::Flat))
                .expect("CCC should have a Moody's WARF factor"),
            6500.0
        );
    }

    #[test]
    fn test_rating_factor_table_notch_fallback() {
        let table = RatingFactorTable::moodys_standard();
        let synthetic = NotchedRating::new(CreditRating::NR, RatingNotch::Plus);
        assert_eq!(
            table
                .get_factor(synthetic)
                .expect("NR+ should fall back to the NR Moody's WARF factor"),
            3650.0
        );
    }

    #[test]
    fn test_rating_factor_table_errors_when_missing() {
        let table = RatingFactorTable {
            factors: HashMap::default(),
            agency: "Test".to_string(),
            methodology: "Test".to_string(),
            default_factor: 42.0,
        };

        assert!(table.get_factor(CreditRating::AA).is_err());
        assert_eq!(table.get_factor_or_default(CreditRating::AA), 42.0);
    }

    #[test]
    fn test_convenience_function_matches_table() {
        let table = RatingFactorTable::moodys_standard();

        // Verify convenience function matches table
        assert_eq!(
            moodys_warf_factor(CreditRating::AAA)
                .expect("AAA should resolve via convenience WARF lookup"),
            table
                .get_factor(CreditRating::AAA)
                .expect("AAA should resolve via table WARF lookup")
        );
        assert_eq!(
            moodys_warf_factor(CreditRating::AA.with_notch(RatingNotch::Minus))
                .expect("AA- should resolve via convenience WARF lookup"),
            table
                .get_factor(CreditRating::AA.with_notch(RatingNotch::Minus))
                .expect("AA- should resolve via table WARF lookup")
        );
        assert_eq!(
            moodys_warf_factor(CreditRating::BBB)
                .expect("BBB should resolve via convenience WARF lookup"),
            table
                .get_factor(CreditRating::BBB)
                .expect("BBB should resolve via table WARF lookup")
        );
        assert_eq!(
            moodys_warf_factor(CreditRating::B.with_notch(RatingNotch::Plus))
                .expect("B+ should resolve via convenience WARF lookup"),
            table
                .get_factor(CreditRating::B.with_notch(RatingNotch::Plus))
                .expect("B+ should resolve via table WARF lookup")
        );
    }

    #[test]
    fn test_rating_factor_table_metadata() {
        let table = RatingFactorTable::moodys_standard();
        assert_eq!(table.agency(), "Moody's");
        assert_eq!(table.methodology(), "IDEALIZED DEFAULT RATES");
        assert_eq!(table.default_factor(), 3650.0);
    }
}
