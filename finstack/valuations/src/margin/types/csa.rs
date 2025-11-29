//! Credit Support Annex (CSA) specification.
//!
//! Defines the CSA agreement terms that govern collateral exchange for
//! OTC derivatives under ISDA documentation.

use super::collateral::EligibleCollateralSchedule;
use super::thresholds::{ImParameters, VmParameters};
use finstack_core::currency::Currency;
use finstack_core::types::CurveId;

/// Margin call timing parameters.
///
/// Specifies the operational timing for margin calls including
/// notification and dispute resolution windows.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarginCallTiming {
    /// Notification deadline (hours after valuation, e.g., 13:00 local time)
    pub notification_deadline_hours: u8,

    /// Response deadline (hours after notification)
    pub response_deadline_hours: u8,

    /// Dispute resolution window (business days)
    pub dispute_resolution_days: u8,

    /// Grace period for collateral delivery (business days)
    pub delivery_grace_days: u8,
}

impl Default for MarginCallTiming {
    fn default() -> Self {
        Self {
            notification_deadline_hours: 13, // 1:00 PM local time
            response_deadline_hours: 2,      // 2 hours to respond
            dispute_resolution_days: 2,      // 2 days to resolve disputes
            delivery_grace_days: 1,          // 1 day grace for delivery
        }
    }
}

impl MarginCallTiming {
    /// Standard timing for regulatory VM CSA.
    #[must_use]
    pub fn regulatory_standard() -> Self {
        Self {
            notification_deadline_hours: 13,
            response_deadline_hours: 2,
            dispute_resolution_days: 1,
            delivery_grace_days: 0, // Same-day for regulatory VM
        }
    }
}

/// Credit Support Annex specification (ISDA standard).
///
/// The CSA governs the exchange of collateral between counterparties
/// for OTC derivatives. This specification captures all key commercial
/// terms needed for margin calculation and management.
///
/// # ISDA Documentation
///
/// This type represents terms from:
/// - ISDA 2016 Credit Support Annex for Variation Margin (VM CSA)
/// - ISDA 2018 Credit Support Annex for Initial Margin (IM CSA)
///
/// # Example
///
/// ```rust,ignore
/// use finstack_valuations::margin::{
///     CsaSpec, VmParameters, ImParameters, EligibleCollateralSchedule,
///     MarginCallTiming, ImMethodology, MarginFrequency,
/// };
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
///
/// let csa = CsaSpec {
///     id: "USD-CSA-2024".to_string(),
///     base_currency: Currency::USD,
///     vm_params: VmParameters::regulatory_standard(Currency::USD),
///     im_params: Some(ImParameters::simm_standard(Currency::USD)),
///     eligible_collateral: EligibleCollateralSchedule::bcbs_standard(),
///     call_timing: MarginCallTiming::regulatory_standard(),
///     collateral_curve_id: "USD-OIS".into(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CsaSpec {
    /// CSA identifier (e.g., "USD-CSA-STANDARD", "COUNTERPARTY-XYZ-CSA")
    pub id: String,

    /// Base currency for margin calculations.
    ///
    /// All exposures and collateral values are converted to this currency
    /// for netting and comparison with thresholds.
    pub base_currency: Currency,

    /// Variation margin parameters.
    ///
    /// Governs daily mark-to-market collateral exchange.
    pub vm_params: VmParameters,

    /// Initial margin parameters (optional).
    ///
    /// If None, no IM is exchanged (either not in scope for regulations
    /// or trade is cleared).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub im_params: Option<ImParameters>,

    /// Eligible collateral schedule.
    ///
    /// Defines what collateral types are acceptable and associated haircuts.
    pub eligible_collateral: EligibleCollateralSchedule,

    /// Margin call timing parameters.
    pub call_timing: MarginCallTiming,

    /// Discount curve ID for collateral valuation.
    ///
    /// Cash collateral is typically discounted at OIS/RFR rates.
    /// This curve should match the CSA's collateral interest rate.
    pub collateral_curve_id: CurveId,
}

impl CsaSpec {
    /// Create a standard regulatory CSA for USD derivatives.
    ///
    /// This represents post-2016 regulatory compliant terms with:
    /// - Zero VM threshold
    /// - Daily margin exchange
    /// - SIMM for IM
    /// - Cash and government bonds as eligible collateral
    #[must_use]
    pub fn usd_regulatory() -> Self {
        Self {
            id: "USD-REGULATORY-CSA".to_string(),
            base_currency: Currency::USD,
            vm_params: VmParameters::regulatory_standard(Currency::USD),
            im_params: Some(ImParameters::simm_standard(Currency::USD)),
            eligible_collateral: EligibleCollateralSchedule::bcbs_standard(),
            call_timing: MarginCallTiming::regulatory_standard(),
            collateral_curve_id: CurveId::new("USD-OIS"),
        }
    }

    /// Create a standard regulatory CSA for EUR derivatives.
    #[must_use]
    pub fn eur_regulatory() -> Self {
        Self {
            id: "EUR-REGULATORY-CSA".to_string(),
            base_currency: Currency::EUR,
            vm_params: VmParameters::regulatory_standard(Currency::EUR),
            im_params: Some(ImParameters::simm_standard(Currency::EUR)),
            eligible_collateral: EligibleCollateralSchedule::bcbs_standard(),
            call_timing: MarginCallTiming::regulatory_standard(),
            collateral_curve_id: CurveId::new("EUR-ESTR"),
        }
    }

    /// Create a legacy bilateral CSA with non-zero thresholds.
    ///
    /// This represents pre-regulatory bilateral agreements that may have
    /// thresholds based on credit rating or relationship.
    #[must_use]
    pub fn bilateral_legacy(
        id: impl Into<String>,
        currency: Currency,
        vm_threshold: f64,
        vm_mta: f64,
    ) -> Self {
        use finstack_core::money::Money;

        Self {
            id: id.into(),
            base_currency: currency,
            vm_params: VmParameters::with_threshold(
                Money::new(vm_threshold, currency),
                Money::new(vm_mta, currency),
            ),
            im_params: None, // Legacy bilateral typically no IM
            eligible_collateral: EligibleCollateralSchedule::default(),
            call_timing: MarginCallTiming::default(),
            collateral_curve_id: CurveId::new(format!("{}-OIS", currency)),
        }
    }

    /// Check if this CSA requires initial margin.
    #[must_use]
    pub fn requires_im(&self) -> bool {
        self.im_params.is_some()
    }

    /// Get the VM threshold amount.
    #[must_use]
    pub fn vm_threshold(&self) -> &finstack_core::money::Money {
        &self.vm_params.threshold
    }

    /// Get the IM threshold amount (if IM is required).
    #[must_use]
    pub fn im_threshold(&self) -> Option<&finstack_core::money::Money> {
        self.im_params.as_ref().map(|p| &p.threshold)
    }
}

impl Default for CsaSpec {
    fn default() -> Self {
        Self::usd_regulatory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::money::Money;

    #[test]
    fn usd_regulatory_csa() {
        let csa = CsaSpec::usd_regulatory();
        assert_eq!(csa.base_currency, Currency::USD);
        assert_eq!(csa.vm_params.threshold, Money::new(0.0, Currency::USD));
        assert!(csa.requires_im());
    }

    #[test]
    fn eur_regulatory_csa() {
        let csa = CsaSpec::eur_regulatory();
        assert_eq!(csa.base_currency, Currency::EUR);
        assert_eq!(csa.collateral_curve_id.as_str(), "EUR-ESTR");
    }

    #[test]
    fn bilateral_legacy_csa() {
        let csa = CsaSpec::bilateral_legacy("TEST-CSA", Currency::USD, 10_000_000.0, 500_000.0);
        assert_eq!(csa.vm_params.threshold.amount(), 10_000_000.0);
        assert!(!csa.requires_im());
    }

    #[test]
    fn margin_call_timing_defaults() {
        let timing = MarginCallTiming::default();
        assert_eq!(timing.notification_deadline_hours, 13);
        assert_eq!(timing.dispute_resolution_days, 2);
    }
}
