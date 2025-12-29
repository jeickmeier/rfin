//! Cashflow primitives and classification enums.
//!
//! Defines core types for representing individual cashflows.
//! These primitives are used throughout the valuations crate for
//! building instrument-specific payment schedules.
//!
//! # Types
//!
//! - [`CashFlow`]: Single dated payment with classification
//! - [`CFKind`]: Cashflow type enumeration (fixed, floating, principal, etc.)

use crate::dates::Date;
use crate::error::InputError;
use crate::money::Money;

/// Enumeration of cash-flow kinds for classification and ordering.
///
/// Used to distinguish between different types of cashflows for
/// proper sequencing, risk calculation, and accounting treatment.
/// The enum itself is **view agnostic**: individual instruments are
/// responsible for mapping these kinds into a holder or issuer view
/// (e.g. whether amortization appears as a positive or negative amount
/// in their simplified cashflow schedules).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CFKind {
    /// Fixed-rate coupon cash-flow.
    Fixed,
    /// Floating-rate reset (index fixing).
    FloatReset,

    /// Up-front fee or cost.
    Fee,
    /// Generic fee cash-flow.
    CommitmentFee,
    /// Usage fee on drawn balance.
    UsageFee,
    /// Facility fee on total commitment.
    FacilityFee,

    /// Principal exchange / notional flow.
    Notional,
    /// Payment-in-kind interest capitalization (adds to principal).
    PIK,
    /// Amortization principal repayment (reduces principal).
    Amortization,
    /// Prepayment of principal (early return of principal in structured credit).
    PrePayment,
    /// Revolving Draw
    RevolvingDraw,
    /// Revolving Repayment
    RevolvingRepayment,

    /// Defaulted notional (principal that has defaulted).
    DefaultedNotional,
    /// Recovery cashflow (amount recovered from defaulted principal).
    Recovery,

    /// Irregular stub period.
    Stub,

    // -------------------------------------------------------------------------
    // Margin and Collateral Cashflows (ISDA CSA / GMRA / BCBS-IOSCO)
    // -------------------------------------------------------------------------
    /// Initial margin posting (collateral transfer out to counterparty).
    ///
    /// Represents the posting of initial margin collateral under CSA or clearing
    /// house rules. The amount is typically calculated using SIMM, schedule-based,
    /// or haircut methodologies.
    InitialMarginPost,
    /// Initial margin return (collateral returned from counterparty).
    ///
    /// Represents the return of previously posted initial margin when exposure
    /// decreases or trade is terminated.
    InitialMarginReturn,
    /// Variation margin received (positive VM payment to us).
    ///
    /// Daily or periodic mark-to-market payment received when exposure moves
    /// in our favor. Governed by CSA threshold, MTA, and rounding rules.
    VariationMarginReceive,
    /// Variation margin paid (negative VM payment from us).
    ///
    /// Daily or periodic mark-to-market payment made when exposure moves
    /// against us. Governed by CSA threshold, MTA, and rounding rules.
    VariationMarginPay,
    /// Interest accrued on posted margin collateral.
    ///
    /// Represents interest paid or received on cash collateral posted as margin.
    /// Rate typically defined in CSA (e.g., Fed Funds, EONIA, SONIA).
    MarginInterest,
    /// Collateral substitution inflow (replacement collateral received).
    ///
    /// When a counterparty substitutes one form of eligible collateral for another,
    /// this represents the incoming replacement asset.
    CollateralSubstitutionIn,
    /// Collateral substitution outflow (original collateral returned).
    ///
    /// When a counterparty substitutes collateral, this represents the return
    /// of the original collateral being replaced.
    CollateralSubstitutionOut,
}

/// A single dated cash-flow (payment or reset).
///
/// Represents a monetary flow at a specific date with metadata
/// for proper classification and risk calculation.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CashFlow {
    /// Payment date (or payment date for principal/fee, or reset date for `CFKind::FloatReset`).
    pub date: Date,
    /// Optional index reset date (for floating coupons).
    pub reset_date: Option<Date>,
    /// Monetary amount including its currency.
    pub amount: Money,
    /// Category/kind of cash-flow.
    pub kind: CFKind,
    /// Accrual factor used for coupon amount and sensitivity.
    pub accrual_factor: f64,
    /// Effective rate used to calculate this cashflow (None if not rate-based or unknown).
    ///
    /// For interest/fees: the annual rate used in the calculation
    /// For notional/amortization/PIK: typically None
    ///
    /// This is stored at cashflow creation time when available.
    /// For instruments with intra-period events (e.g., revolving credit with draws/repays),
    /// this may represent a time-weighted average rate across sub-periods.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
}

impl CashFlow {
    /// Validate cashflow amount and fields.
    ///
    /// # Errors
    /// Returns [`crate::Error::Input`] if the `amount` is zero.
    ///
    /// # Example
    /// ```rust
    /// use finstack_core::cashflow::{CashFlow, CFKind};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use finstack_core::money::Money;
    /// use time::Month;
    ///
    /// let date = Date::from_calendar_date(2025, Month::January, 15).expect("Valid date");
    /// let amount = Money::new(100.0, Currency::USD);
    /// let cf = CashFlow {
    ///     date,
    ///     reset_date: None,
    ///     amount,
    ///     kind: CFKind::Fixed,
    ///     accrual_factor: 0.0,
    ///     rate: None,
    /// };
    /// assert!(cf.validate().is_ok());
    ///
    /// let zero_cf = CashFlow {
    ///     date,
    ///     reset_date: None,
    ///     amount: Money::new(0.0, Currency::USD),
    ///     kind: CFKind::Fixed,
    ///     accrual_factor: 0.0,
    ///     rate: None,
    /// };
    /// assert!(zero_cf.validate().is_err());
    /// ```
    ///
    /// # Validation Rules
    ///
    /// - Amount must be non-zero and finite (not NaN or Infinity)
    /// - Accrual factor must be finite (not NaN or Infinity)
    /// - Rate (if present) must be finite (not NaN or Infinity)
    /// - Reset date (if present) must not be after the payment date
    pub fn validate(&self) -> crate::Result<()> {
        // Check for zero amount
        if self.amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }

        // Check for non-finite amount (NaN or Infinity)
        if !self.amount.amount().is_finite() {
            return Err(InputError::Invalid.into());
        }

        // Check for non-finite accrual factor
        if !self.accrual_factor.is_finite() {
            return Err(InputError::Invalid.into());
        }

        // Check for non-finite rate (if present)
        if let Some(rate) = self.rate {
            if !rate.is_finite() {
                return Err(InputError::Invalid.into());
            }
        }

        // Check that reset date is not after payment date
        if let Some(reset) = self.reset_date {
            if reset > self.date {
                return Err(InputError::Invalid.into());
            }
        }

        Ok(())
    }
}

// -------------------------------------------------------------------------
// Compile-time size assertion (≤ 56 bytes)
// -------------------------------------------------------------------------
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use core::mem::size_of;
    use time::Month;

    #[test]
    fn cashflow_size_is_reasonable() {
        // With simplified structure (removed rate_base field): 56 bytes
        // - date: 4 bytes (Date is i32 internally)
        // - reset_date: Option<Date> = 8 bytes (4 + 1 discriminant + padding)
        // - amount: Money = 16 bytes (f64 + Currency enum)
        // - kind: CFKind = 2 bytes (enum)
        // - accrual_factor: f64 = 8 bytes
        // - rate: Option<f64> = 16 bytes (8 + 1 discriminant + padding)
        // Total with alignment: 56 bytes
        assert!(size_of::<CashFlow>() <= 56);
    }

    #[test]
    fn cashflow_validation_works() {
        let date = Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let amount = Money::new(100.0, Currency::USD);

        let cf = CashFlow {
            date,
            reset_date: None,
            amount,
            kind: CFKind::Fixed,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(cf.date, date);
        assert_eq!(cf.amount, amount);
        assert_eq!(cf.kind, CFKind::Fixed);
        assert!(cf.reset_date.is_none());
        assert_eq!(cf.accrual_factor, 0.0);
        assert!(cf.validate().is_ok());
    }

    #[test]
    fn cashflow_kinds_construct_correctly() {
        let date = Date::from_calendar_date(2025, Month::March, 1).expect("Valid test date");
        let amt = Money::new(1_000.0, Currency::EUR);

        let princ = CashFlow {
            date,
            reset_date: None,
            amount: amt,
            kind: CFKind::Notional,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(princ.kind, CFKind::Notional);
        assert!(princ.validate().is_ok());

        let fee = CashFlow {
            date,
            reset_date: None,
            amount: amt,
            kind: CFKind::Fee,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(fee.kind, CFKind::Fee);
        assert!(fee.validate().is_ok());

        let pik = CashFlow {
            date,
            reset_date: None,
            amount: amt,
            kind: CFKind::PIK,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(pik.kind, CFKind::PIK);
        assert!(pik.validate().is_ok());

        let amort = CashFlow {
            date,
            reset_date: None,
            amount: amt,
            kind: CFKind::Amortization,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(amort.kind, CFKind::Amortization);
        assert!(amort.validate().is_ok());

        // Zero amount returns error on validation
        let zero = Money::new(0.0, Currency::EUR);
        let zero_cf = CashFlow {
            date,
            reset_date: None,
            amount: zero,
            kind: CFKind::Fixed,
            accrual_factor: 0.0,
            rate: None,
        };
        assert!(zero_cf.validate().is_err());
    }

    #[test]
    fn margin_cashflow_kinds_construct_correctly() {
        let date = Date::from_calendar_date(2025, Month::March, 1).expect("Valid test date");
        let amt = Money::new(1_000_000.0, Currency::USD);

        // Initial margin posting
        let im_post = CashFlow {
            date,
            reset_date: None,
            amount: amt,
            kind: CFKind::InitialMarginPost,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(im_post.kind, CFKind::InitialMarginPost);
        assert!(im_post.validate().is_ok());

        // Initial margin return
        let im_return = CashFlow {
            date,
            reset_date: None,
            amount: amt,
            kind: CFKind::InitialMarginReturn,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(im_return.kind, CFKind::InitialMarginReturn);
        assert!(im_return.validate().is_ok());

        // Variation margin received
        let vm_receive = CashFlow {
            date,
            reset_date: None,
            amount: amt,
            kind: CFKind::VariationMarginReceive,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(vm_receive.kind, CFKind::VariationMarginReceive);
        assert!(vm_receive.validate().is_ok());

        // Variation margin paid
        let vm_pay = CashFlow {
            date,
            reset_date: None,
            amount: amt,
            kind: CFKind::VariationMarginPay,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(vm_pay.kind, CFKind::VariationMarginPay);
        assert!(vm_pay.validate().is_ok());

        // Margin interest
        let margin_int = CashFlow {
            date,
            reset_date: None,
            amount: Money::new(5_000.0, Currency::USD),
            kind: CFKind::MarginInterest,
            accrual_factor: 0.25,
            rate: Some(0.05),
        };
        assert_eq!(margin_int.kind, CFKind::MarginInterest);
        assert!(margin_int.validate().is_ok());

        // Collateral substitution in
        let sub_in = CashFlow {
            date,
            reset_date: None,
            amount: amt,
            kind: CFKind::CollateralSubstitutionIn,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(sub_in.kind, CFKind::CollateralSubstitutionIn);
        assert!(sub_in.validate().is_ok());

        // Collateral substitution out
        let sub_out = CashFlow {
            date,
            reset_date: None,
            amount: amt,
            kind: CFKind::CollateralSubstitutionOut,
            accrual_factor: 0.0,
            rate: None,
        };
        assert_eq!(sub_out.kind, CFKind::CollateralSubstitutionOut);
        assert!(sub_out.validate().is_ok());
    }
}
