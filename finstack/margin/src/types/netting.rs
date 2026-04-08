//! Netting set identification and instrument margin results.

use super::simm_types::SimmSensitivities;
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Identifies a margin netting set.
///
/// Instruments in the same netting set can offset each other for margin
/// calculation purposes. The netting set is typically defined by the
/// CSA agreement or CCP membership.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct NettingSetId {
    /// Counterparty identifier
    pub counterparty_id: String,
    /// CSA identifier (for bilateral trades)
    pub csa_id: Option<String>,
    /// CCP identifier (for cleared trades)
    pub ccp_id: Option<String>,
}

impl NettingSetId {
    /// Create a bilateral netting set ID.
    #[must_use]
    pub fn bilateral(counterparty_id: impl Into<String>, csa_id: impl Into<String>) -> Self {
        Self {
            counterparty_id: counterparty_id.into(),
            csa_id: Some(csa_id.into()),
            ccp_id: None,
        }
    }

    /// Create a cleared netting set ID.
    #[must_use]
    pub fn cleared(ccp_id: impl Into<String>) -> Self {
        let ccp_string = ccp_id.into();
        Self {
            counterparty_id: ccp_string.clone(),
            csa_id: None,
            ccp_id: Some(ccp_string),
        }
    }

    /// Check if this is a cleared netting set.
    #[must_use]
    pub fn is_cleared(&self) -> bool {
        self.ccp_id.is_some()
    }
}

impl std::fmt::Display for NettingSetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ccp) = &self.ccp_id {
            write!(f, "CCP:{}", ccp)
        } else if let Some(csa) = &self.csa_id {
            write!(f, "{}:{}", self.counterparty_id, csa)
        } else {
            write!(f, "{}", self.counterparty_id)
        }
    }
}

/// Result of calculating margin for an instrument.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct InstrumentMarginResult {
    /// Instrument identifier
    pub instrument_id: String,
    /// Calculation date
    #[schemars(with = "String")]
    pub as_of: Date,
    /// Initial margin requirement
    pub initial_margin: Money,
    /// Variation margin requirement (can be negative = return)
    pub variation_margin: Money,
    /// Total margin requirement (IM + VM if positive)
    pub total_margin: Money,
    /// IM calculation methodology used
    pub im_methodology: crate::types::ImMethodology,
    /// Whether instrument is cleared or bilateral
    pub is_cleared: bool,
    /// Netting set identifier
    pub netting_set: Option<NettingSetId>,
    /// SIMM sensitivities (if SIMM was used)
    pub sensitivities: Option<SimmSensitivities>,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_netting_set_id() {
        let bilateral = NettingSetId::bilateral("COUNTERPARTY_A", "CSA_001");
        assert!(!bilateral.is_cleared());
        assert_eq!(bilateral.to_string(), "COUNTERPARTY_A:CSA_001");

        let cleared = NettingSetId::cleared("LCH");
        assert!(cleared.is_cleared());
        assert_eq!(cleared.to_string(), "CCP:LCH");
    }
}
