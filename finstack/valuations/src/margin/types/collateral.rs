#![allow(clippy::unwrap_used)]

//! Eligible collateral specifications and haircuts.
//!
//! Defines collateral eligibility criteria and haircut schedules following
//! BCBS-IOSCO standards for non-centrally cleared derivatives and GMRA
//! conventions for repos.

use std::fmt;

use crate::margin::config::margin_registry_from_config;
use crate::margin::registry::embedded_registry;
use finstack_core::config::FinstackConfig;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Collateral asset classes per BCBS-IOSCO standards.
///
/// Asset classes determine baseline haircuts and eligibility criteria.
/// The BCBS-IOSCO framework specifies minimum haircuts by asset class.
///
/// # Reference
///
/// BCBS-IOSCO "Margin requirements for non-centrally cleared derivatives" (2020)
/// Annex A: Standardized haircut schedule
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum CollateralAssetClass {
    /// Cash in eligible currency
    #[default]
    Cash,
    GovernmentBonds,
    AgencyBonds,
    CoveredBonds,
    CorporateBonds,
    Equity,
    Gold,
    MutualFunds,
    /// Custom / user-defined asset class (from JSON)
    Custom(String),
}

impl CollateralAssetClass {
    fn normalize(raw: &str) -> String {
        raw.trim().to_ascii_lowercase().replace([' ', '-'], "_")
    }

    /// Normalized string identifier for this asset class.
    pub fn as_str(&self) -> &str {
        match self {
            CollateralAssetClass::Cash => "cash",
            CollateralAssetClass::GovernmentBonds => "government_bonds",
            CollateralAssetClass::AgencyBonds => "agency_bonds",
            CollateralAssetClass::CoveredBonds => "covered_bonds",
            CollateralAssetClass::CorporateBonds => "corporate_bonds",
            CollateralAssetClass::Equity => "equity",
            CollateralAssetClass::Gold => "gold",
            CollateralAssetClass::MutualFunds => "mutual_funds",
            CollateralAssetClass::Custom(s) => s.as_str(),
        }
    }
}

impl Serialize for CollateralAssetClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CollateralAssetClass {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(D::Error::custom)
    }
}

impl fmt::Display for CollateralAssetClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for CollateralAssetClass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let norm = Self::normalize(s);
        match norm.as_str() {
            "cash" => Ok(CollateralAssetClass::Cash),
            "government_bonds" | "governmentbonds" | "govies" | "sovereign" => {
                Ok(CollateralAssetClass::GovernmentBonds)
            }
            "agency_bonds" | "agencybonds" | "agency" => Ok(CollateralAssetClass::AgencyBonds),
            "covered_bonds" | "coveredbonds" => Ok(CollateralAssetClass::CoveredBonds),
            "corporate_bonds" | "corporatebonds" | "corporate" => {
                Ok(CollateralAssetClass::CorporateBonds)
            }
            "equity" | "equities" | "stock" => Ok(CollateralAssetClass::Equity),
            "gold" => Ok(CollateralAssetClass::Gold),
            "mutual_funds" | "mutualfunds" | "etf" | "funds" => {
                Ok(CollateralAssetClass::MutualFunds)
            }
            other => Ok(CollateralAssetClass::Custom(other.to_string())),
        }
    }
}

impl CollateralAssetClass {
    /// Get the BCBS-IOSCO standard haircut for this asset class.
    ///
    /// These are baseline haircuts; actual haircuts may vary based on
    /// rating, maturity, and bilateral agreement.
    ///
    /// # Returns
    ///
    /// Haircut as a decimal (e.g., 0.02 = 2%)
    #[must_use]
    pub fn standard_haircut(&self) -> f64 {
        let registry = embedded_registry().unwrap();
        registry
            .collateral_asset_class_defaults
            .get(self)
            .map(|d| d.standard_haircut)
            .unwrap()
    }

    /// Get the FX haircut add-on for currency mismatch.
    ///
    /// Per BCBS-IOSCO, an 8% add-on applies when collateral currency
    /// differs from the settlement currency of the derivative.
    #[must_use]
    pub fn fx_addon(&self) -> f64 {
        let registry = embedded_registry().unwrap();
        registry
            .collateral_asset_class_defaults
            .get(self)
            .map(|d| d.fx_addon)
            .unwrap()
    }
}

/// Maturity constraints for eligible collateral.
///
/// Some CSAs restrict collateral based on remaining maturity to limit
/// duration risk in the collateral portfolio.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MaturityConstraints {
    /// Minimum remaining years to maturity (if any)
    pub min_remaining_years: Option<f64>,
    /// Maximum remaining years to maturity (if any)
    pub max_remaining_years: Option<f64>,
}

impl MaturityConstraints {
    /// Create constraints with maximum maturity only.
    #[must_use]
    pub fn max_maturity(years: f64) -> Self {
        Self {
            min_remaining_years: None,
            max_remaining_years: Some(years),
        }
    }

    /// Check if a given remaining maturity satisfies the constraints.
    #[must_use]
    pub fn is_satisfied(&self, remaining_years: f64) -> bool {
        if let Some(min) = self.min_remaining_years {
            if remaining_years < min {
                return false;
            }
        }
        if let Some(max) = self.max_remaining_years {
            if remaining_years > max {
                return false;
            }
        }
        true
    }
}

/// Single collateral eligibility entry.
///
/// Defines eligibility criteria and haircut for a specific type of collateral.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CollateralEligibility {
    /// Asset class
    pub asset_class: CollateralAssetClass,

    /// Minimum credit rating requirement (e.g., "A-", "BBB")
    ///
    /// If None, no rating constraint applies.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub min_rating: Option<String>,

    /// Remaining maturity constraints
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub maturity_constraints: Option<MaturityConstraints>,

    /// Haircut as decimal (e.g., 0.02 = 2%)
    pub haircut: f64,

    /// Additional FX haircut for currency mismatch (decimal)
    ///
    /// Applied when collateral currency differs from settlement currency.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fx_haircut_addon: f64,

    /// Concentration limit as fraction of total collateral (optional)
    ///
    /// E.g., 0.30 means max 30% of collateral can be this type.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub concentration_limit: Option<f64>,
}

impl CollateralEligibility {
    /// Create a cash eligibility entry.
    #[must_use]
    pub fn cash() -> Self {
        Self {
            asset_class: CollateralAssetClass::Cash,
            min_rating: None,
            maturity_constraints: None,
            haircut: 0.0,
            fx_haircut_addon: CollateralAssetClass::Cash.fx_addon(),
            concentration_limit: None,
        }
    }

    /// Create a government bonds eligibility entry with standard BCBS haircuts.
    #[must_use]
    pub fn government_bonds(haircut: f64) -> Self {
        Self {
            asset_class: CollateralAssetClass::GovernmentBonds,
            min_rating: Some("A-".to_string()),
            maturity_constraints: None,
            haircut,
            fx_haircut_addon: CollateralAssetClass::GovernmentBonds.fx_addon(),
            concentration_limit: None,
        }
    }

    /// Create a corporate bonds eligibility entry.
    #[must_use]
    pub fn corporate_bonds(haircut: f64, min_rating: &str) -> Self {
        Self {
            asset_class: CollateralAssetClass::CorporateBonds,
            min_rating: Some(min_rating.to_string()),
            maturity_constraints: None,
            haircut,
            fx_haircut_addon: CollateralAssetClass::CorporateBonds.fx_addon(),
            concentration_limit: Some(0.30), // 30% concentration limit typical
        }
    }

    /// Calculate total haircut including FX add-on if applicable.
    #[must_use]
    pub fn total_haircut(&self, currency_mismatch: bool) -> f64 {
        if currency_mismatch {
            self.haircut + self.fx_haircut_addon
        } else {
            self.haircut
        }
    }
}

/// Eligible collateral schedule with haircuts.
///
/// Defines the complete set of collateral types accepted under a CSA
/// or margin agreement, along with associated haircuts and constraints.
///
/// # Example
///
/// ```rust,no_run
/// use finstack_valuations::margin::{CollateralEligibility, EligibleCollateralSchedule};
///
/// // Start from a standard schedule (BCBS-IOSCO compliant)
/// let schedule = EligibleCollateralSchedule::bcbs_standard();
/// # let _ = schedule;
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EligibleCollateralSchedule {
    /// List of eligible collateral types with haircuts
    pub eligible: Vec<CollateralEligibility>,

    /// Default haircut for unlisted collateral (if accepted)
    ///
    /// If None, only explicitly listed collateral types are accepted.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub default_haircut: Option<f64>,

    /// Whether rehypothecation of posted collateral is permitted
    ///
    /// For IM under BCBS-IOSCO rules, rehypothecation is prohibited.
    /// For VM, rehypothecation may be permitted by bilateral agreement.
    #[cfg_attr(feature = "serde", serde(default))]
    pub rehypothecation_allowed: bool,
}

impl Default for EligibleCollateralSchedule {
    fn default() -> Self {
        Self {
            eligible: vec![CollateralEligibility::cash()],
            default_haircut: None,
            rehypothecation_allowed: false,
        }
    }
}

impl EligibleCollateralSchedule {
    /// Create a schedule accepting only cash.
    #[must_use]
    pub fn cash_only() -> Self {
        let registry = embedded_registry().unwrap();
        registry
            .collateral_schedules
            .get("cash_only")
            .cloned()
            .unwrap()
    }

    /// Create a standard BCBS-IOSCO compliant schedule.
    ///
    /// Includes cash and government bonds with standard haircuts.
    #[must_use]
    pub fn bcbs_standard() -> Self {
        let registry = embedded_registry().unwrap();
        registry
            .collateral_schedules
            .get("bcbs_standard")
            .cloned()
            .unwrap()
    }

    /// Create a standard repo collateral schedule (US Treasuries).
    #[must_use]
    pub fn us_treasuries() -> Self {
        let registry = embedded_registry().unwrap();
        registry
            .collateral_schedules
            .get("us_treasuries")
            .cloned()
            .unwrap()
    }

    /// Load a named schedule from a provided config (with overrides).
    pub fn from_finstack_config(
        cfg: &FinstackConfig,
        schedule_id: &str,
    ) -> finstack_core::Result<Self> {
        let registry = margin_registry_from_config(cfg)?;
        registry
            .collateral_schedules
            .get(schedule_id)
            .cloned()
            .ok_or_else(|| {
                finstack_core::Error::Validation(format!(
                    "collateral schedule '{schedule_id}' not found"
                ))
            })
    }

    /// Find the applicable haircut for a given asset class.
    ///
    /// Returns the first matching eligibility entry's haircut, or the default
    /// haircut if no specific match is found.
    #[must_use]
    pub fn haircut_for(&self, asset_class: &CollateralAssetClass) -> Option<f64> {
        self.eligible
            .iter()
            .find(|e| &e.asset_class == asset_class)
            .map(|e| e.haircut)
            .or(self.default_haircut)
    }

    /// Check if an asset class is eligible.
    #[must_use]
    pub fn is_eligible(&self, asset_class: &CollateralAssetClass) -> bool {
        self.eligible.iter().any(|e| &e.asset_class == asset_class)
            || self.default_haircut.is_some()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn collateral_asset_class_haircuts() {
        assert_eq!(CollateralAssetClass::Cash.standard_haircut(), 0.0);
        assert_eq!(CollateralAssetClass::Equity.standard_haircut(), 0.15);
        assert_eq!(CollateralAssetClass::Gold.standard_haircut(), 0.15);
    }

    #[test]
    fn maturity_constraints_work() {
        let constraints = MaturityConstraints {
            min_remaining_years: Some(1.0),
            max_remaining_years: Some(5.0),
        };
        assert!(!constraints.is_satisfied(0.5)); // Too short
        assert!(constraints.is_satisfied(2.0)); // Within range
        assert!(!constraints.is_satisfied(6.0)); // Too long
    }

    #[test]
    fn total_haircut_includes_fx_addon() {
        let eligibility = CollateralEligibility::government_bonds(0.02);
        assert_eq!(eligibility.total_haircut(false), 0.02);
        assert_eq!(eligibility.total_haircut(true), 0.10); // 0.02 + 0.08
    }

    #[test]
    fn schedule_finds_haircut() {
        let schedule = EligibleCollateralSchedule::cash_only();
        assert_eq!(schedule.haircut_for(&CollateralAssetClass::Cash), Some(0.0));
        assert_eq!(schedule.haircut_for(&CollateralAssetClass::Equity), None);
    }

    #[test]
    fn bcbs_standard_schedule() {
        let schedule = EligibleCollateralSchedule::bcbs_standard();
        assert!(schedule.is_eligible(&CollateralAssetClass::Cash));
        assert!(schedule.is_eligible(&CollateralAssetClass::GovernmentBonds));
        assert!(!schedule.is_eligible(&CollateralAssetClass::Equity));
        assert!(!schedule.rehypothecation_allowed);
    }
}
