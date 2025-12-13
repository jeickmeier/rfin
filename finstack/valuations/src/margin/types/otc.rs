//! OTC derivative margin specification.
//!
//! Shared margin specification for CSA-governed OTC derivatives
//! including IRS, CDS, CDS Index, and TRS.

use super::csa::CsaSpec;
use super::enums::{ClearingStatus, ImMethodology, MarginTenor};
use finstack_core::currency::Currency;

/// OTC derivative margin specification (ISDA CSA compliant).
///
/// This is the standard margin specification for bilateral and cleared
/// OTC derivatives. It combines CSA terms with clearing-specific parameters.
///
/// # Usage
///
/// Attach this to any OTC derivative instrument that requires margining:
/// - Interest Rate Swaps (IRS)
/// - Credit Default Swaps (CDS)
/// - CDS Indices
/// - Total Return Swaps (TRS)
///
/// # Example
///
/// ```rust,ignore
/// use finstack_valuations::margin::{OtcMarginSpec, CsaSpec, ClearingStatus, ImMethodology, MarginTenor};
///
/// // Bilateral (uncleared) derivative
/// let bilateral_spec = OtcMarginSpec::bilateral_simm(CsaSpec::usd_regulatory());
///
/// // Cleared derivative
/// let cleared_spec = OtcMarginSpec::cleared("LCH", finstack_core::currency::Currency::USD);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OtcMarginSpec {
    /// Full CSA specification (for bilateral trades)
    ///
    /// For cleared trades, this represents the terms with the CCP.
    pub csa: CsaSpec,

    /// Clearing status: bilateral or cleared through CCP
    pub clearing_status: ClearingStatus,

    /// Initial margin calculation methodology
    ///
    /// - Bilateral: SIMM or Schedule
    /// - Cleared: ClearingHouse (CCP-specific)
    pub im_methodology: ImMethodology,

    /// Variation margin exchange frequency
    pub vm_frequency: MarginTenor,

    /// Settlement lag for margin transfers (business days)
    pub settlement_lag: u32,
}

impl OtcMarginSpec {
    /// Create a bilateral margin spec using ISDA SIMM.
    ///
    /// This is the standard configuration for large dealer-to-dealer
    /// or dealer-to-client bilateral trades.
    #[must_use]
    pub fn bilateral_simm(csa: CsaSpec) -> Self {
        Self {
            csa,
            clearing_status: ClearingStatus::Bilateral,
            im_methodology: ImMethodology::Simm,
            vm_frequency: MarginTenor::Daily,
            settlement_lag: 1,
        }
    }

    /// Create a bilateral margin spec using schedule-based IM.
    ///
    /// Used when SIMM is not implemented or for smaller counterparties.
    #[must_use]
    pub fn bilateral_schedule(csa: CsaSpec) -> Self {
        Self {
            csa,
            clearing_status: ClearingStatus::Bilateral,
            im_methodology: ImMethodology::Schedule,
            vm_frequency: MarginTenor::Daily,
            settlement_lag: 1,
        }
    }

    /// Create a margin spec for cleared derivatives.
    ///
    /// # Arguments
    ///
    /// * `ccp` - Clearing house identifier (e.g., "LCH", "CME", "ICE")
    /// * `currency` - Settlement currency
    #[must_use]
    pub fn cleared(ccp: impl Into<String>, currency: Currency) -> Self {
        use super::collateral::EligibleCollateralSchedule;
        use super::csa::MarginCallTiming;
        use super::thresholds::{ImParameters, VmParameters};
        use finstack_core::money::Money;
        use finstack_core::types::CurveId;

        let ccp_name = ccp.into();

        // CCPs have zero thresholds and daily margining
        let csa = CsaSpec {
            id: format!("{}-CCP-CSA", ccp_name),
            base_currency: currency,
            vm_params: VmParameters {
                threshold: Money::new(0.0, currency),
                mta: Money::new(0.0, currency),
                rounding: Money::new(1.0, currency), // CCPs often use 1 unit rounding
                independent_amount: Money::new(0.0, currency),
                frequency: MarginTenor::Daily,
                settlement_lag: 0, // Same-day settlement for CCPs
            },
            im_params: Some(ImParameters::cleared(currency)),
            eligible_collateral: EligibleCollateralSchedule::bcbs_standard(),
            call_timing: MarginCallTiming {
                notification_deadline_hours: 10, // Earlier deadline for CCPs
                response_deadline_hours: 1,
                dispute_resolution_days: 1,
                delivery_grace_days: 0,
            },
            collateral_curve_id: CurveId::new(format!("{}-OIS", currency)),
        };

        Self {
            csa,
            clearing_status: ClearingStatus::Cleared { ccp: ccp_name },
            im_methodology: ImMethodology::ClearingHouse,
            vm_frequency: MarginTenor::Daily,
            settlement_lag: 0,
        }
    }

    /// Create a USD bilateral spec with standard regulatory terms.
    #[must_use]
    pub fn usd_bilateral() -> Self {
        Self::bilateral_simm(CsaSpec::usd_regulatory())
    }

    /// Create a EUR bilateral spec with standard regulatory terms.
    #[must_use]
    pub fn eur_bilateral() -> Self {
        Self::bilateral_simm(CsaSpec::eur_regulatory())
    }

    /// Create a spec for LCH SwapClear cleared IRS.
    #[must_use]
    pub fn lch_swapclear(currency: Currency) -> Self {
        Self::cleared("LCH", currency)
    }

    /// Create a spec for CME cleared derivatives.
    #[must_use]
    pub fn cme_cleared(currency: Currency) -> Self {
        Self::cleared("CME", currency)
    }

    /// Create a spec for ICE Clear Credit (cleared CDS).
    #[must_use]
    pub fn ice_clear_credit() -> Self {
        Self::cleared("ICE", Currency::USD)
    }

    /// Check if this is a cleared trade.
    #[must_use]
    pub fn is_cleared(&self) -> bool {
        matches!(self.clearing_status, ClearingStatus::Cleared { .. })
    }

    /// Check if this is a bilateral trade.
    #[must_use]
    pub fn is_bilateral(&self) -> bool {
        matches!(self.clearing_status, ClearingStatus::Bilateral)
    }

    /// Get the CCP name if cleared.
    #[must_use]
    pub fn ccp(&self) -> Option<&str> {
        match &self.clearing_status {
            ClearingStatus::Cleared { ccp } => Some(ccp.as_str()),
            ClearingStatus::Bilateral => None,
        }
    }

    /// Get the base currency for margin calculations.
    #[must_use]
    pub fn base_currency(&self) -> Currency {
        self.csa.base_currency
    }
}

impl Default for OtcMarginSpec {
    fn default() -> Self {
        Self::usd_bilateral()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::money::Money;

    #[test]
    fn bilateral_simm_spec() {
        let spec = OtcMarginSpec::usd_bilateral();
        assert!(spec.is_bilateral());
        assert!(!spec.is_cleared());
        assert_eq!(spec.im_methodology, ImMethodology::Simm);
        assert_eq!(spec.vm_frequency, MarginTenor::Daily);
        assert!(spec.ccp().is_none());
    }

    #[test]
    fn cleared_spec() {
        let spec = OtcMarginSpec::lch_swapclear(Currency::USD);
        assert!(spec.is_cleared());
        assert!(!spec.is_bilateral());
        assert_eq!(spec.im_methodology, ImMethodology::ClearingHouse);
        assert_eq!(spec.ccp(), Some("LCH"));
        assert_eq!(spec.settlement_lag, 0);
    }

    #[test]
    fn ice_clear_credit_spec() {
        let spec = OtcMarginSpec::ice_clear_credit();
        assert!(spec.is_cleared());
        assert_eq!(spec.ccp(), Some("ICE"));
        assert_eq!(spec.base_currency(), Currency::USD);
    }

    #[test]
    fn csa_thresholds() {
        let spec = OtcMarginSpec::cleared("CME", Currency::EUR);
        assert_eq!(spec.csa.vm_params.threshold, Money::new(0.0, Currency::EUR));
    }
}
