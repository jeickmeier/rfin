//! Variation margin and initial margin parameter specifications.
//!
//! Defines the threshold, MTA (Minimum Transfer Amount), and other parameters
//! that govern margin call mechanics.

use super::enums::{ImMethodology, MarginTenor};
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::config::margin_registry_from_config;
use crate::registry::embedded_registry;
use finstack_core::config::FinstackConfig;

/// Variation margin parameters.
///
/// These parameters govern the daily (or periodic) exchange of variation margin
/// under a CSA agreement. VM is exchanged to eliminate mark-to-market exposure.
///
/// # ISDA CSA Standard Terms
///
/// The 2016 VM CSA introduced standardized terms for variation margin:
/// - Zero threshold for in-scope entities
/// - Daily exchange with T+1 settlement
/// - Cash or highly liquid securities as collateral
///
/// # Example
///
/// ```rust,no_run
/// use finstack_margin::{MarginTenor, VmParameters};
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
///
/// let vm_params = VmParameters {
///     threshold: Money::new(10_000_000.0, Currency::USD),
///     mta: Money::new(500_000.0, Currency::USD),
///     rounding: Money::new(10_000.0, Currency::USD),
///     independent_amount: Money::new(0.0, Currency::USD),
///     frequency: MarginTenor::Daily,
///     settlement_lag: 1,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VmParameters {
    /// Threshold amount below which no margin is exchanged.
    ///
    /// Under BCBS-IOSCO rules for covered entities, VM threshold must be zero.
    /// Legacy bilateral CSAs may have non-zero thresholds.
    pub threshold: Money,

    /// Minimum Transfer Amount (MTA).
    ///
    /// Margin calls below MTA are not made. BCBS-IOSCO permits combined
    /// IM+VM MTA up to €500,000 equivalent.
    pub mta: Money,

    /// Rounding increment for margin amounts.
    ///
    /// Margin calls are typically rounded to the nearest multiple of this amount.
    pub rounding: Money,

    /// Independent Amount (IA) / Additional Margin.
    ///
    /// Fixed collateral amount required regardless of exposure.
    /// Often used for credit enhancement or as a buffer.
    pub independent_amount: Money,

    /// Margin call frequency.
    ///
    /// Under BCBS-IOSCO, daily margin exchange is required.
    pub frequency: MarginTenor,

    /// Settlement lag in business days (T+n).
    ///
    /// Standard is T+1 for VM under 2016 VM CSA.
    pub settlement_lag: u32,
}

impl VmParameters {
    /// Create VM parameters with zero threshold (regulatory standard).
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded margin registry cannot be loaded.
    pub fn regulatory_standard(currency: Currency) -> Result<Self> {
        let registry = embedded_registry()?;
        Ok(Self::from_defaults(currency, &registry.defaults.vm))
    }

    /// Create VM parameters with a threshold (bilateral thresholds).
    #[must_use]
    pub fn with_threshold(threshold: Money, mta: Money) -> Self {
        let currency = threshold.currency();
        Self {
            threshold,
            mta,
            rounding: Money::new(10_000.0, currency),
            independent_amount: Money::new(0.0, currency),
            frequency: MarginTenor::Daily,
            settlement_lag: 1,
        }
    }

    /// Calculate the credit support amount (margin to be delivered/returned).
    ///
    /// # ISDA CSA Formula
    ///
    /// ```text
    /// Credit Support Amount = max(0, Exposure - Threshold + IA) - Current_Collateral
    /// Delivery Amount = CSA if CSA ≥ MTA, else 0
    /// Return Amount = -CSA if CSA ≤ -MTA, else 0
    /// ```
    ///
    /// # Arguments
    ///
    /// * `exposure` - Current mark-to-market exposure (positive = we are owed money)
    /// * `current_collateral` - Value of currently posted collateral
    ///
    /// # Returns
    ///
    /// The net margin amount to be delivered (positive) or returned (negative).
    /// Returns zero if the amount is below MTA.
    pub fn calculate_margin_call(
        &self,
        exposure: Money,
        current_collateral: Money,
    ) -> Result<Money> {
        // Prevent silent currency mixing in release builds.
        if exposure.currency() != self.threshold.currency() {
            return Err(finstack_core::Error::Validation(format!(
                "VM exposure currency mismatch: expected {}, got {}",
                self.threshold.currency(),
                exposure.currency()
            )));
        }
        if current_collateral.currency() != self.threshold.currency() {
            return Err(finstack_core::Error::Validation(format!(
                "VM collateral currency mismatch: expected {}, got {}",
                self.threshold.currency(),
                current_collateral.currency()
            )));
        }

        let currency = exposure.currency();

        // Credit Support Amount = max(0, Exposure - Threshold + IA) - Current_Collateral
        // Use f64 arithmetic to avoid Result handling
        let exp = exposure.amount();
        let threshold = self.threshold.amount();
        let ia = self.independent_amount.amount();
        let posted = current_collateral.amount();

        let required = (exp - threshold + ia).max(0.0);
        let credit_support_amount = required - posted;

        // Apply MTA
        let mta_amount = self.mta.amount();
        if credit_support_amount.abs() < mta_amount {
            return Ok(Money::new(0.0, currency));
        }

        // Apply rounding
        Ok(self.round_to_nearest(Money::new(credit_support_amount, currency)))
    }

    /// Round an amount to the nearest rounding increment.
    fn round_to_nearest(&self, amount: Money) -> Money {
        let rounding = self.rounding.amount();
        if rounding <= 0.0 {
            return amount;
        }
        let rounded = (amount.amount() / rounding).round() * rounding;
        Money::new(rounded, amount.currency())
    }

    /// Build from defaults resolved via a `FinstackConfig`.
    pub fn from_finstack_config(cfg: &FinstackConfig, currency: Currency) -> Result<Self> {
        let registry = margin_registry_from_config(cfg)?;
        Ok(Self::from_defaults(currency, &registry.defaults.vm))
    }

    fn from_defaults(currency: Currency, defaults: &crate::registry::VmDefaults) -> Self {
        defaults.to_vm_params(currency)
    }
}

impl Default for VmParameters {
    #[allow(clippy::expect_used)]
    fn default() -> Self {
        Self::regulatory_standard(Currency::USD)
            .expect("embedded margin registry is a compile-time asset")
    }
}

/// Initial margin parameters.
///
/// Initial margin is collateral posted to cover potential future exposure (PFE)
/// during the close-out period following a default. IM is required for
/// non-centrally cleared derivatives under BCBS-IOSCO rules.
///
/// # Margin Period of Risk (MPOR)
///
/// The MPOR determines the horizon over which PFE is calculated:
/// - Standard: 10 business days for bilateral derivatives
/// - Reduced: 5 days for certain liquid products
///
/// # Example
///
/// ```rust,no_run
/// use finstack_margin::{ImMethodology, ImParameters};
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
///
/// let im_params = ImParameters {
///     methodology: ImMethodology::Simm,
///     mpor_days: 10,
///     threshold: Money::new(50_000_000.0, Currency::USD),
///     mta: Money::new(0.0, Currency::USD), // Combined with VM MTA
///     segregated: true,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImParameters {
    /// IM calculation methodology.
    ///
    /// Options include SIMM, regulatory schedule, or CCP methodology.
    pub methodology: ImMethodology,

    /// Margin Period of Risk in business days.
    ///
    /// Standard is 10 days under BCBS-IOSCO. CCPs may use shorter periods.
    pub mpor_days: u32,

    /// IM threshold (aggregate group level).
    ///
    /// BCBS-IOSCO permits €50M aggregate threshold at group level.
    /// Many large dealers operate with zero threshold by agreement.
    pub threshold: Money,

    /// Minimum Transfer Amount for IM.
    ///
    /// Combined IM+VM MTA must not exceed €500,000 under BCBS-IOSCO.
    pub mta: Money,

    /// Whether IM must be held in a segregated account.
    ///
    /// Under BCBS-IOSCO, IM must be segregated with a third-party custodian
    /// to protect it in case of the collecting party's insolvency.
    pub segregated: bool,
}

impl ImParameters {
    /// Create IM parameters using ISDA SIMM methodology.
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded margin registry cannot be loaded.
    pub fn simm_standard(currency: Currency) -> Result<Self> {
        let registry = embedded_registry()?;
        Ok(registry
            .defaults
            .im
            .simm
            .to_im_params(ImMethodology::Simm, currency))
    }

    /// Create IM parameters using schedule-based methodology.
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded margin registry cannot be loaded.
    pub fn schedule_based(currency: Currency) -> Result<Self> {
        let registry = embedded_registry()?;
        Ok(registry
            .defaults
            .im
            .schedule
            .to_im_params(ImMethodology::Schedule, currency))
    }

    /// Create IM parameters for cleared trades (CCP methodology).
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded margin registry cannot be loaded.
    pub fn cleared(currency: Currency) -> Result<Self> {
        let registry = embedded_registry()?;
        Ok(registry
            .defaults
            .im
            .cleared
            .to_im_params(ImMethodology::ClearingHouse, currency))
    }

    /// Create IM parameters for repos using haircut methodology.
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded margin registry cannot be loaded.
    pub fn repo_haircut(currency: Currency) -> Result<Self> {
        let registry = embedded_registry()?;
        Ok(registry
            .defaults
            .im
            .repo_haircut
            .to_im_params(ImMethodology::Haircut, currency))
    }

    /// Create IM parameters using defaults resolved from a config.
    pub fn from_finstack_config(
        cfg: &FinstackConfig,
        methodology: ImMethodology,
        currency: Currency,
    ) -> Result<Self> {
        let registry = margin_registry_from_config(cfg)?;
        let defaults = match methodology {
            ImMethodology::Simm => &registry.defaults.im.simm,
            ImMethodology::Schedule => &registry.defaults.im.schedule,
            ImMethodology::ClearingHouse => &registry.defaults.im.cleared,
            ImMethodology::Haircut => &registry.defaults.im.repo_haircut,
            ImMethodology::InternalModel => &registry.defaults.im.simm,
        };
        Ok(defaults.to_im_params(methodology, currency))
    }
}

impl Default for ImParameters {
    #[allow(clippy::expect_used)]
    fn default() -> Self {
        Self::simm_standard(Currency::USD)
            .expect("embedded margin registry is a compile-time asset")
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn vm_params_regulatory_standard() {
        let params =
            VmParameters::regulatory_standard(Currency::USD).expect("registry should load");
        assert_eq!(params.threshold, Money::new(0.0, Currency::USD));
        assert_eq!(params.frequency, MarginTenor::Daily);
        assert_eq!(params.settlement_lag, 1);
    }

    #[test]
    fn vm_margin_call_calculation() {
        let params = VmParameters {
            threshold: Money::new(1_000_000.0, Currency::USD),
            mta: Money::new(100_000.0, Currency::USD),
            rounding: Money::new(10_000.0, Currency::USD),
            independent_amount: Money::new(0.0, Currency::USD),
            frequency: MarginTenor::Daily,
            settlement_lag: 1,
        };

        // Exposure below threshold: no margin call
        let exposure = Money::new(500_000.0, Currency::USD);
        let collateral = Money::new(0.0, Currency::USD);
        let call = params
            .calculate_margin_call(exposure, collateral)
            .expect("matching currencies should succeed");
        assert_eq!(call, Money::new(0.0, Currency::USD));

        // Exposure above threshold: margin call
        let exposure = Money::new(2_000_000.0, Currency::USD);
        let call = params
            .calculate_margin_call(exposure, collateral)
            .expect("matching currencies should succeed");
        assert_eq!(call, Money::new(1_000_000.0, Currency::USD)); // 2M - 1M threshold

        // Amount below MTA: no call
        let exposure = Money::new(1_050_000.0, Currency::USD);
        let call = params
            .calculate_margin_call(exposure, collateral)
            .expect("matching currencies should succeed");
        assert_eq!(call, Money::new(0.0, Currency::USD)); // 50K < 100K MTA
    }

    #[test]
    fn im_params_simm_standard() {
        let params = ImParameters::simm_standard(Currency::EUR).expect("registry should load");
        assert_eq!(params.methodology, ImMethodology::Simm);
        assert_eq!(params.mpor_days, 10);
        assert!(params.segregated);
    }

    #[test]
    fn im_params_cleared() {
        let params = ImParameters::cleared(Currency::USD).expect("registry should load");
        assert_eq!(params.methodology, ImMethodology::ClearingHouse);
        assert_eq!(params.mpor_days, 5);
        assert!(!params.segregated);
        assert_eq!(params.threshold, Money::new(0.0, Currency::USD));
    }
}
