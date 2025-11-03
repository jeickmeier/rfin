//! CMBS standard waterfall template.
//!
//! Implements a standard CMBS waterfall structure with:
//! - Servicing fees (master servicer, special servicer)
//! - Interest payments (sequential by class)
//! - Principal payments (lockout then sequential)
//! - Realized losses allocated from bottom up

use super::super::components::{
    AllocationMode, PaymentCalculation, PaymentRecipient, PaymentType, Recipient,
    WaterfallBuilder, WaterfallEngine, WaterfallTier,
};
use super::super::config::{BASIS_POINTS_DIVISOR, CMBS_MASTER_SERVICER_FEE_BPS, CMBS_SPECIAL_SERVICER_FEE_BPS, CMBS_TRUSTEE_FEE_ANNUAL};
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Create a standard CMBS waterfall template
///
/// # Waterfall Structure
///
/// ## Tier 1: Servicing Fees
/// - Master servicer fee (25 bps of collateral)
/// - Special servicer fee (25 bps of collateral)
/// - Trustee fee (fixed annual)
///
/// ## Tier 2: Interest Payments (Sequential)
/// - Class A (Super Senior) notes
/// - Class B (Senior) notes
/// - Class C (Mezzanine) notes
/// - Class D (Junior) notes
/// - Class E (Subordinated) notes
///
/// ## Tier 3: Principal Payments (Sequential)
/// - Class A principal
/// - Class B principal
/// - Class C principal
/// - Class D principal
/// - Class E principal
///
/// ## Tier 4: Equity Distribution
/// - Residual cash to equity/certificate holders
///
/// # Notes
///
/// - CMBS typically has lockout periods where principal prepayments are restricted
/// - Realized losses are allocated from bottom up (Class E → D → C → B → A)
/// - No OC/IC tests like CLO; principal follows strict sequential priority
pub fn cmbs_standard_template(currency: Currency) -> WaterfallEngine {
    WaterfallBuilder::new(currency)
        // Tier 1: Servicing Fees
        .add_tier(
            WaterfallTier::new("servicing_fees", 1, PaymentType::Fee)
                .allocation_mode(AllocationMode::Sequential)
                .add_recipient(Recipient::new(
                    "master_servicer",
                    PaymentRecipient::ServiceProvider("MasterServicer".into()),
                    PaymentCalculation::PercentageOfCollateral {
                        rate: CMBS_MASTER_SERVICER_FEE_BPS / BASIS_POINTS_DIVISOR,
                        annualized: true,
                    },
                ))
                .add_recipient(Recipient::new(
                    "special_servicer",
                    PaymentRecipient::ServiceProvider("SpecialServicer".into()),
                    PaymentCalculation::PercentageOfCollateral {
                        rate: CMBS_SPECIAL_SERVICER_FEE_BPS / BASIS_POINTS_DIVISOR,
                        annualized: true,
                    },
                ))
                .add_recipient(Recipient::new(
                    "trustee_fee",
                    PaymentRecipient::ServiceProvider("Trustee".into()),
                    PaymentCalculation::FixedAmount {
                        amount: Money::new(CMBS_TRUSTEE_FEE_ANNUAL, currency),
                    },
                )),
        )
        // Tier 2: Interest Payments
        .add_tier(
            WaterfallTier::new("interest_payments", 2, PaymentType::Interest)
                .allocation_mode(AllocationMode::Sequential)
                .add_recipient(Recipient::tranche_interest("class_a_int", "CLASS_A"))
                .add_recipient(Recipient::tranche_interest("class_b_int", "CLASS_B"))
                .add_recipient(Recipient::tranche_interest("class_c_int", "CLASS_C"))
                .add_recipient(Recipient::tranche_interest("class_d_int", "CLASS_D"))
                .add_recipient(Recipient::tranche_interest("class_e_int", "CLASS_E")),
        )
        // Tier 3: Principal Payments (Sequential, no diversion)
        .add_tier(
            WaterfallTier::new("principal_payments", 3, PaymentType::Principal)
                .allocation_mode(AllocationMode::Sequential)
                .divertible(false) // CMBS doesn't divert principal
                .add_recipient(Recipient::tranche_principal("class_a_prin", "CLASS_A", None))
                .add_recipient(Recipient::tranche_principal("class_b_prin", "CLASS_B", None))
                .add_recipient(Recipient::tranche_principal("class_c_prin", "CLASS_C", None))
                .add_recipient(Recipient::tranche_principal("class_d_prin", "CLASS_D", None))
                .add_recipient(Recipient::tranche_principal("class_e_prin", "CLASS_E", None)),
        )
        // Tier 4: Equity Distribution
        .add_tier(
            WaterfallTier::new("equity", 4, PaymentType::Residual)
                .allocation_mode(AllocationMode::Sequential)
                .add_recipient(Recipient::new(
                    "certificate_holders",
                    PaymentRecipient::Equity,
                    PaymentCalculation::ResidualCash,
                )),
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmbs_template_structure() {
        let waterfall = cmbs_standard_template(Currency::USD);

        // Should have 4 tiers
        assert_eq!(waterfall.tiers.len(), 4);

        // Verify tier priorities
        assert_eq!(waterfall.tiers[0].id, "servicing_fees");
        assert_eq!(waterfall.tiers[1].id, "interest_payments");
        assert_eq!(waterfall.tiers[2].id, "principal_payments");
        assert_eq!(waterfall.tiers[3].id, "equity");

        // Verify principal tier is NOT divertible (CMBS doesn't have OC/IC tests)
        assert!(!waterfall.tiers[2].divertible);

        // No coverage triggers for CMBS
        assert_eq!(waterfall.coverage_triggers.len(), 0);
    }

    #[test]
    fn test_servicing_fees_tier() {
        let waterfall = cmbs_standard_template(Currency::USD);
        let servicing_tier = &waterfall.tiers[0];

        // Should have 3 recipients (master, special, trustee)
        assert_eq!(servicing_tier.recipients.len(), 3);
        assert_eq!(servicing_tier.recipients[0].id, "master_servicer");
        assert_eq!(servicing_tier.recipients[1].id, "special_servicer");
        assert_eq!(servicing_tier.recipients[2].id, "trustee_fee");
    }

    #[test]
    fn test_five_class_structure() {
        let waterfall = cmbs_standard_template(Currency::USD);
        let interest_tier = &waterfall.tiers[1];

        // CMBS typically has 5 classes (A-E)
        assert_eq!(interest_tier.recipients.len(), 5);
        assert_eq!(interest_tier.recipients[0].id, "class_a_int");
        assert_eq!(interest_tier.recipients[4].id, "class_e_int");
    }
}

