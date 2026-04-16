//! Restructuring-specific error types.
//!
//! Validation and domain errors for recovery waterfall execution,
//! exchange offer analysis, and LME modeling. All variants convert
//! into [`crate::Error`] via the `Core` path for seamless `?`
//! propagation.

use std::fmt;

/// Restructuring domain error.
///
/// Covers input validation failures specific to restructuring analytics:
/// negative claim amounts, out-of-range ratios, currency mismatches,
/// and missing references.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RestructuringError {
    /// A claim amount (principal, accrued, or penalties) is negative.
    NegativeClaimAmount {
        /// Claim identifier
        claim_id: String,
        /// Which field is negative
        field: String,
        /// The invalid value
        value: f64,
    },
    /// Distributable value is negative.
    NegativeDistributableValue {
        /// The invalid value
        value: f64,
    },
    /// Collateral haircut is outside [0.0, 1.0].
    InvalidHaircut {
        /// Claim identifier
        claim_id: String,
        /// The invalid haircut value
        haircut: f64,
    },
    /// Currency mismatch between distributable value and a claim.
    CurrencyMismatch {
        /// Expected currency (from distributable value)
        expected: String,
        /// Actual currency found on the claim
        actual: String,
        /// Claim identifier
        claim_id: String,
    },
    /// Plan deviation references a non-existent claim ID.
    UnknownDeviationClaim {
        /// The invalid claim ID
        claim_id: String,
    },
    /// Exchange ratio is outside valid range (0.0, 2.0].
    InvalidExchangeRatio {
        /// The invalid ratio
        ratio: f64,
    },
    /// Coupon rate is negative.
    NegativeCouponRate {
        /// The invalid rate
        rate: f64,
    },
    /// Discount rate is non-positive.
    InvalidDiscountRate {
        /// The invalid rate
        rate: f64,
    },
    /// Recovery rate is outside [0.0, 1.0].
    InvalidRecoveryRate {
        /// The invalid rate
        rate: f64,
    },
    /// Participation rate is outside [0.0, 1.0].
    InvalidParticipationRate {
        /// The invalid rate
        rate: f64,
    },
    /// Purchase/tender price is outside valid range (0.0, 1.5].
    InvalidPurchasePrice {
        /// The invalid price
        price: f64,
    },
    /// Outstanding amount is non-positive.
    NonPositiveOutstanding {
        /// The invalid amount
        amount: f64,
    },
    /// General validation failure.
    Validation {
        /// Description of the problem
        message: String,
    },
}

impl fmt::Display for RestructuringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeClaimAmount {
                claim_id,
                field,
                value,
            } => write!(
                f,
                "claim '{}': {} is negative ({})",
                claim_id, field, value
            ),
            Self::NegativeDistributableValue { value } => {
                write!(f, "distributable value is negative ({})", value)
            }
            Self::InvalidHaircut { claim_id, haircut } => write!(
                f,
                "claim '{}': collateral haircut {} outside [0.0, 1.0]",
                claim_id, haircut
            ),
            Self::CurrencyMismatch {
                expected,
                actual,
                claim_id,
            } => write!(
                f,
                "currency mismatch on claim '{}': expected {}, got {}",
                claim_id, expected, actual
            ),
            Self::UnknownDeviationClaim { claim_id } => {
                write!(
                    f,
                    "plan deviation references unknown claim '{}'",
                    claim_id
                )
            }
            Self::InvalidExchangeRatio { ratio } => {
                write!(f, "exchange ratio {} outside (0.0, 2.0]", ratio)
            }
            Self::NegativeCouponRate { rate } => {
                write!(f, "coupon rate is negative ({})", rate)
            }
            Self::InvalidDiscountRate { rate } => {
                write!(f, "discount rate is non-positive ({})", rate)
            }
            Self::InvalidRecoveryRate { rate } => {
                write!(f, "recovery rate {} outside [0.0, 1.0]", rate)
            }
            Self::InvalidParticipationRate { rate } => {
                write!(f, "participation rate {} outside [0.0, 1.0]", rate)
            }
            Self::InvalidPurchasePrice { price } => {
                write!(f, "purchase/tender price {} outside (0.0, 1.5]", price)
            }
            Self::NonPositiveOutstanding { amount } => {
                write!(f, "outstanding amount is non-positive ({})", amount)
            }
            Self::Validation { message } => write!(f, "restructuring validation: {}", message),
        }
    }
}

impl std::error::Error for RestructuringError {}

impl From<RestructuringError> for crate::Error {
    fn from(err: RestructuringError) -> Self {
        crate::Error::Core(finstack_core::Error::Validation(err.to_string()))
    }
}
