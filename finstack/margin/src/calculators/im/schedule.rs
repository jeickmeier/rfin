//! BCBS-IOSCO regulatory schedule-based IM calculator.
//!
//! Fallback methodology using grid-based rates applied to notional amounts.
//! Simpler but typically more conservative than SIMM.
//!
//! # Error Handling
//!
//! The constructors
//! [`crate::calculators::im::schedule::RegulatorySchedule::bcbs_iosco()`] and
//! [`crate::calculators::im::schedule::ScheduleImCalculator::bcbs_standard()`]
//! return `Result` rather than panicking,
//! allowing callers to handle missing registry data gracefully.

use crate::calculators::traits::{ImCalculator, ImResult};
use crate::registry::{embedded_registry, margin_registry_from_config};
use crate::traits::Marginable;
use crate::types::ImMethodology;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::HashMap;
use finstack_core::Result;

/// Asset class for schedule-based IM calculation.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScheduleAssetClass {
    /// Interest rate derivatives
    InterestRate,
    /// Credit derivatives
    Credit,
    /// Equity derivatives
    Equity,
    /// Commodity derivatives
    Commodity,
    /// Foreign exchange derivatives
    Fx,
    /// Other derivatives
    Other,
    /// Custom user-defined asset class (from JSON)
    Custom(String),
}

impl ScheduleAssetClass {
    fn normalize(raw: &str) -> String {
        raw.trim().to_ascii_lowercase().replace([' ', '-'], "_")
    }

    /// Normalized string identifier for this asset class.
    pub fn as_str(&self) -> &str {
        match self {
            ScheduleAssetClass::InterestRate => "interest_rate",
            ScheduleAssetClass::Credit => "credit",
            ScheduleAssetClass::Equity => "equity",
            ScheduleAssetClass::Commodity => "commodity",
            ScheduleAssetClass::Fx => "fx",
            ScheduleAssetClass::Other => "other",
            ScheduleAssetClass::Custom(s) => s.as_str(),
        }
    }
}

impl serde::Serialize for ScheduleAssetClass {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ScheduleAssetClass {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for ScheduleAssetClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ScheduleAssetClass {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let norm = ScheduleAssetClass::normalize(s);
        match norm.as_str() {
            "interest_rate" | "ir" => Ok(ScheduleAssetClass::InterestRate),
            "credit" => Ok(ScheduleAssetClass::Credit),
            "equity" => Ok(ScheduleAssetClass::Equity),
            "commodity" => Ok(ScheduleAssetClass::Commodity),
            "fx" => Ok(ScheduleAssetClass::Fx),
            "other" => Ok(ScheduleAssetClass::Other),
            other => Ok(ScheduleAssetClass::Custom(other.to_string())),
        }
    }
}

/// BCBS-IOSCO regulatory schedule for IM calculation.
///
/// Stores the schedule-grid rates used by the regulatory fallback methodology
/// for uncleared derivatives. Rates are decimals, so `0.04` means 4% of the
/// regulatory notional or other proxy exposure base.
///
/// # References
///
/// - BCBS-IOSCO uncleared margin framework: `docs/REFERENCES.md#bcbs-iosco-uncleared-margin`
#[derive(Debug, Clone)]
pub struct RegulatorySchedule {
    /// IM rates by asset class and maturity bucket
    pub rates: HashMap<(ScheduleAssetClass, MaturityBucket), f64>,
    /// Short/medium bucket boundary in years.
    pub short_to_medium: f64,
    /// Medium/long bucket boundary in years.
    pub medium_to_long: f64,
    /// Default rate when no explicit bucket is available.
    pub default_rate: f64,
}

/// Maturity bucket for schedule IM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaturityBucket {
    /// Less than 2 years
    Short,
    /// 2-5 years
    Medium,
    /// Greater than 5 years
    Long,
}

/// Default schedule IM registry id for the BCBS-IOSCO grid (`schedule_im.v1.json`).
pub const BCBS_IOSCO_SCHEDULE_ID: &str = "bcbs_iosco";

impl RegulatorySchedule {
    /// BCBS-IOSCO standard schedule loaded from the embedded registry.
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded registry cannot be loaded or if the
    /// [`BCBS_IOSCO_SCHEDULE_ID`] schedule entry is missing.
    pub fn bcbs_iosco() -> Result<Self> {
        Self::from_registry_id(BCBS_IOSCO_SCHEDULE_ID)
    }

    /// Load a named schedule IM grid from the embedded registry (no hard-coded rates).
    ///
    /// # Errors
    ///
    /// Returns an error if the registry cannot be loaded or `schedule_id` is absent.
    pub fn from_registry_id(schedule_id: &str) -> Result<Self> {
        let registry = embedded_registry()?;
        let schedule = registry.schedule_im.get(schedule_id).ok_or_else(|| {
            finstack_core::InputError::NotFound {
                id: format!("schedule_im '{schedule_id}'"),
            }
        })?;
        Ok(Self::from_registry(schedule.clone()))
    }

    /// Build from a registry entry.
    #[must_use]
    pub fn from_registry(entry: crate::registry::ScheduleImSchedule) -> Self {
        Self {
            rates: entry.rates,
            short_to_medium: entry.boundaries.short_to_medium,
            medium_to_long: entry.boundaries.medium_to_long,
            default_rate: entry.default_rate,
        }
    }

    /// Get the IM rate for an asset class and maturity.
    #[must_use]
    pub fn rate(&self, asset_class: ScheduleAssetClass, maturity_years: f64) -> f64 {
        let bucket = if maturity_years < self.short_to_medium {
            MaturityBucket::Short
        } else if maturity_years < self.medium_to_long {
            MaturityBucket::Medium
        } else {
            MaturityBucket::Long
        };

        *self
            .rates
            .get(&(asset_class, bucket))
            .unwrap_or(&self.default_rate)
    }
}

/// Schedule-based IM calculator.
///
/// Implements the BCBS-IOSCO schedule fallback for uncleared derivatives.
/// The schedule itself is a percentage grid keyed by asset class and maturity
/// bucket, with rates stored in decimal form.
///
/// There are two distinct entry points:
/// - [`Self::calculate_for_notional`] applies the schedule to an explicit
///   notional amount supplied by the caller.
/// - [`ImCalculator::calculate`] uses `instrument.mtm_for_vm(...).abs()` as a
///   conservative placeholder exposure base because [`crate::traits::Marginable`] does not yet
///   expose a regulatory notional measure. That trait-based path therefore does
///   **not** implement full regulatory schedule margin.
///
/// # Formula
///
/// ```text
/// Explicit notional helper:
/// IM = |Notional| × Schedule_Rate(asset_class, maturity)
///
/// Trait-based fallback:
/// IM_proxy = |Current_MtM| × Schedule_Rate(default_asset_class, default_maturity)
/// ```
///
/// # Conventions
///
/// - `Schedule_Rate` is a decimal fraction, not basis points.
/// - Maturity is supplied as a year fraction.
/// - The embedded BCBS-IOSCO grid uses the schedule boundaries carried in the
///   registry entry rather than hard-coded bucket cutoffs.
///
/// # Example
///
/// ```rust,no_run
/// use finstack_margin::{ImCalculator, Marginable, ScheduleImCalculator};
/// use finstack_core::dates::Date;
/// use finstack_core::market_data::context::MarketContext;
/// use time::macros::date;
///
/// # fn main() -> finstack_core::Result<()> {
/// let calc = ScheduleImCalculator::bcbs_standard()?;
/// # let swap: &dyn Marginable = todo!("provide a marginable instrument");
/// # let context = MarketContext::new();
/// # let as_of: Date = date!(2025-01-01);
/// let im = calc.calculate(swap, &context, as_of)?;
/// # let _ = im;
/// # Ok(())
/// # }
/// ```
///
/// # References
///
/// - BCBS-IOSCO uncleared margin framework: `docs/REFERENCES.md#bcbs-iosco-uncleared-margin`
#[derive(Debug, Clone)]
pub struct ScheduleImCalculator {
    /// Regulatory schedule
    pub schedule: RegulatorySchedule,
    /// Default asset class to assume
    pub default_asset_class: ScheduleAssetClass,
    /// Default maturity in years
    pub default_maturity_years: f64,
    /// Margin period of risk (days)
    pub mpor_days: u32,
}

impl ScheduleImCalculator {
    /// Create calculator with the embedded BCBS-IOSCO standard schedule.
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded registry cannot be loaded or if the
    /// [`BCBS_IOSCO_SCHEDULE_ID`] schedule entry is missing.
    pub fn bcbs_standard() -> Result<Self> {
        Self::from_registry_id(BCBS_IOSCO_SCHEDULE_ID)
    }

    /// Create calculator from a schedule grid id in the embedded or merged registry.
    ///
    /// # Arguments
    ///
    /// * `schedule_id` - Registry identifier such as [`BCBS_IOSCO_SCHEDULE_ID`]
    ///
    /// # Errors
    ///
    /// Returns an error if the registry cannot be loaded or `schedule_id` is not found.
    pub fn from_registry_id(schedule_id: &str) -> Result<Self> {
        let registry = embedded_registry()?;
        let entry = registry.schedule_im.get(schedule_id).ok_or_else(|| {
            finstack_core::InputError::NotFound {
                id: format!("schedule_im '{schedule_id}'"),
            }
        })?;
        Ok(Self::from_registry(entry))
    }

    /// Create calculator from a resolved registry entry.
    ///
    /// # Arguments
    ///
    /// * `entry` - Fully parsed schedule grid with decimal rates and bucket boundaries
    ///
    /// # Returns
    ///
    /// A calculator using the registry defaults for asset class, maturity, and MPOR.
    #[must_use]
    pub fn from_registry(entry: &crate::registry::ScheduleImSchedule) -> Self {
        Self {
            schedule: RegulatorySchedule::from_registry(entry.clone()),
            default_asset_class: entry.default_asset_class.clone(),
            default_maturity_years: entry.default_maturity_years,
            mpor_days: entry.mpor_days,
        }
    }

    /// Create calculator resolved from a provided `FinstackConfig`.
    ///
    /// Loads the schedule entry identified by [`BCBS_IOSCO_SCHEDULE_ID`] after
    /// applying any margin-registry overlay in the config.
    ///
    /// # Errors
    ///
    /// Returns an error if the registry cannot be loaded from the config or if the
    /// "bcbs_iosco" schedule entry is missing.
    pub fn from_finstack_config(cfg: &finstack_core::config::FinstackConfig) -> Result<Self> {
        let registry = margin_registry_from_config(cfg)?;
        let entry = registry
            .schedule_im
            .get(BCBS_IOSCO_SCHEDULE_ID)
            .ok_or_else(|| finstack_core::InputError::NotFound {
                id: format!("schedule_im '{}'", BCBS_IOSCO_SCHEDULE_ID),
            })?;
        Ok(Self::from_registry(entry))
    }

    /// Set the default asset class used by [`ImCalculator::calculate`].
    ///
    /// # Arguments
    ///
    /// * `asset_class` - Asset class used when the trait-based fallback path
    ///   cannot infer a more specific regulatory schedule bucket
    ///
    /// # Returns
    ///
    /// The updated calculator.
    #[must_use]
    pub fn with_asset_class(mut self, asset_class: ScheduleAssetClass) -> Self {
        self.default_asset_class = asset_class;
        self
    }

    /// Set the default maturity used by [`ImCalculator::calculate`].
    ///
    /// # Arguments
    ///
    /// * `years` - Maturity expressed as a year fraction
    ///
    /// # Returns
    ///
    /// The updated calculator.
    #[must_use]
    pub fn with_maturity(mut self, years: f64) -> Self {
        self.default_maturity_years = years;
        self
    }

    /// Calculate schedule IM from an explicit notional amount.
    ///
    /// # Arguments
    ///
    /// * `notional` - Regulatory notional or other caller-supplied exposure base
    ///   in the reporting currency
    /// * `asset_class` - Regulatory schedule asset class
    /// * `maturity_years` - Remaining maturity as a year fraction
    ///
    /// # Returns
    ///
    /// `|notional| × rate`, with the rate taken from the configured schedule grid.
    pub fn calculate_for_notional(
        &self,
        notional: Money,
        asset_class: ScheduleAssetClass,
        maturity_years: f64,
    ) -> Money {
        let rate = self.schedule.rate(asset_class, maturity_years);
        Money::new(notional.amount().abs(), notional.currency()) * rate
    }

    /// Get the decimal schedule rate for an asset class and maturity.
    ///
    /// # Arguments
    ///
    /// * `asset_class` - Regulatory schedule asset class
    /// * `maturity_years` - Remaining maturity as a year fraction
    ///
    /// # Returns
    ///
    /// A decimal rate such as `0.01` for 1%.
    #[must_use]
    pub fn rate(&self, asset_class: ScheduleAssetClass, maturity_years: f64) -> f64 {
        self.schedule.rate(asset_class, maturity_years)
    }
}

impl ImCalculator for ScheduleImCalculator {
    fn calculate(
        &self,
        instrument: &dyn Marginable,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        let mtm = instrument.mtm_for_vm(context, as_of)?;
        let notional = Money::new(mtm.amount().abs(), mtm.currency());

        let im_amount = self.calculate_for_notional(
            notional,
            self.default_asset_class.clone(),
            self.default_maturity_years,
        );

        let mut breakdown = HashMap::default();
        breakdown.insert(self.default_asset_class.to_string(), im_amount);

        Ok(ImResult::with_breakdown(
            im_amount,
            ImMethodology::Schedule,
            as_of,
            self.mpor_days,
            breakdown,
        ))
    }

    fn methodology(&self) -> ImMethodology {
        ImMethodology::Schedule
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn bcbs_schedule_rates() {
        let schedule = RegulatorySchedule::bcbs_iosco()
            .expect("bcbs_iosco schedule should load from embedded registry");

        // Interest rate
        assert_eq!(schedule.rate(ScheduleAssetClass::InterestRate, 1.0), 0.01); // 1%
        assert_eq!(schedule.rate(ScheduleAssetClass::InterestRate, 3.0), 0.02); // 2%
        assert_eq!(schedule.rate(ScheduleAssetClass::InterestRate, 10.0), 0.04); // 4%

        // Credit
        assert_eq!(schedule.rate(ScheduleAssetClass::Credit, 1.0), 0.02);
        assert_eq!(schedule.rate(ScheduleAssetClass::Credit, 10.0), 0.10);

        // Equity (constant)
        assert_eq!(schedule.rate(ScheduleAssetClass::Equity, 1.0), 0.15);
        assert_eq!(schedule.rate(ScheduleAssetClass::Equity, 10.0), 0.15);
    }

    #[test]
    fn schedule_im_calculation() {
        let calc = ScheduleImCalculator::bcbs_standard()
            .expect("bcbs_standard calculator should load from embedded registry");

        let notional = Money::new(100_000_000.0, Currency::USD);
        let im = calc.calculate_for_notional(notional, ScheduleAssetClass::InterestRate, 5.0);

        // 5y IR uses long bucket (4%) since maturity >= 5.0
        assert_eq!(im.amount(), 4_000_000.0);
    }

    #[test]
    fn credit_schedule_im() {
        let calc = ScheduleImCalculator::bcbs_standard()
            .expect("bcbs_standard calculator should load from embedded registry")
            .with_asset_class(ScheduleAssetClass::Credit)
            .with_maturity(7.0);

        let notional = Money::new(50_000_000.0, Currency::USD);
        let im = calc.calculate_for_notional(notional, ScheduleAssetClass::Credit, 7.0);

        // 7y credit uses long bucket (10%)
        assert_eq!(im.amount(), 5_000_000.0);
    }

    #[test]
    fn bcbs_constructors_return_ok() {
        // Verify the embedded registry is available and constructors succeed.
        // This catches registry configuration issues at CI time.
        assert!(
            RegulatorySchedule::bcbs_iosco().is_ok(),
            "RegulatorySchedule::bcbs_iosco() should return Ok"
        );
        assert!(
            ScheduleImCalculator::bcbs_standard().is_ok(),
            "ScheduleImCalculator::bcbs_standard() should return Ok"
        );
    }

    #[test]
    fn from_registry_id_matches_bcbs_iosco() {
        let via_named = RegulatorySchedule::from_registry_id(BCBS_IOSCO_SCHEDULE_ID)
            .expect("named schedule should load");
        let via_legacy = RegulatorySchedule::bcbs_iosco().expect("bcbs_iosco should load");
        assert_eq!(via_named.default_rate, via_legacy.default_rate);
        assert_eq!(via_named.rates.len(), via_legacy.rates.len());
    }
}
