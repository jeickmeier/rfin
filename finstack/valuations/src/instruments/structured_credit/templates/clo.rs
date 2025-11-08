//! CLO 2.0 waterfall template.
//!
//! Implements a standard CLO 2.0 waterfall structure with:
//! - Senior fees (trustee, admin)
//! - Management fees (senior and subordinated)
//! - Interest payments (Class A → Class B → Class C → Class D)
//! - Principal payments with OC/IC test triggers
//! - Equity distribution

use super::super::components::waterfall::CoverageTrigger;
use super::super::components::{
    AllocationMode, ManagementFeeType, PaymentCalculation, PaymentRecipient, PaymentType,
    Recipient, WaterfallBuilder, WaterfallEngine, WaterfallTier,
};
use super::super::config::{
    BASIS_POINTS_DIVISOR, CLO_SENIOR_MGMT_FEE_BPS, CLO_SUBORDINATED_MGMT_FEE_BPS,
    CLO_TRUSTEE_FEE_ANNUAL,
};
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Create a standard CLO 2.0 waterfall template
///
/// # Waterfall Structure
///
/// ## Tier 1: Senior Expenses
/// - Trustee fees (fixed annual)
/// - Administrative fees
///
/// ## Tier 2: Management Fees
/// - Senior management fee (40 bps of collateral)
/// - Subordinated management fee (20 bps of collateral)
///
/// ## Tier 3: Interest Payments (Sequential)
/// - Class A (Senior) notes
/// - Class B (Mezzanine) notes
/// - Class C (Junior) notes
/// - Class D (Subordinated) notes
///
/// ## Tier 4: Principal Payments (Sequential, Divertible)
/// - Class A principal
/// - Class B principal
/// - Class C principal
/// - Class D principal
///
/// Diverted to senior tranches if OC/IC tests fail
///
/// ## Tier 5: Equity Distribution
/// - Residual cash to equity holders
///
/// # Coverage Tests
///
/// - OC Test: 125% for Class A
/// - IC Test: 120% for Class A
///
/// If tests fail, subordinated principal diverts to senior tranches
pub fn clo_2_0_template(currency: Currency) -> WaterfallEngine {
    WaterfallBuilder::new(currency)
        // Tier 1: Senior Expenses
        .add_tier(
            WaterfallTier::new("senior_expenses", 1, PaymentType::Fee)
                .allocation_mode(AllocationMode::Sequential)
                .add_recipient(Recipient::new(
                    "trustee_fee",
                    PaymentRecipient::ServiceProvider("Trustee".into()),
                    PaymentCalculation::FixedAmount {
                        amount: Money::new(CLO_TRUSTEE_FEE_ANNUAL, currency),
                    },
                ))
                .add_recipient(Recipient::new(
                    "admin_fee",
                    PaymentRecipient::ServiceProvider("Administrator".into()),
                    PaymentCalculation::FixedAmount {
                        amount: Money::new(25_000.0, currency),
                    },
                )),
        )
        // Tier 2: Management Fees
        .add_tier(
            WaterfallTier::new("management_fees", 2, PaymentType::Fee)
                .allocation_mode(AllocationMode::Sequential)
                .add_recipient(Recipient::new(
                    "senior_mgmt_fee",
                    PaymentRecipient::ManagerFee(ManagementFeeType::Senior),
                    PaymentCalculation::PercentageOfCollateral {
                        rate: CLO_SENIOR_MGMT_FEE_BPS / BASIS_POINTS_DIVISOR,
                        annualized: true,
                    },
                ))
                .add_recipient(Recipient::new(
                    "sub_mgmt_fee",
                    PaymentRecipient::ManagerFee(ManagementFeeType::Subordinated),
                    PaymentCalculation::PercentageOfCollateral {
                        rate: CLO_SUBORDINATED_MGMT_FEE_BPS / BASIS_POINTS_DIVISOR,
                        annualized: true,
                    },
                )),
        )
        // Tier 3: Interest Payments
        .add_tier(
            WaterfallTier::new("interest_payments", 3, PaymentType::Interest)
                .allocation_mode(AllocationMode::Sequential)
                .add_recipient(Recipient::tranche_interest("class_a_int", "CLASS_A"))
                .add_recipient(Recipient::tranche_interest("class_b_int", "CLASS_B"))
                .add_recipient(Recipient::tranche_interest("class_c_int", "CLASS_C"))
                .add_recipient(Recipient::tranche_interest("class_d_int", "CLASS_D")),
        )
        // Tier 4: Principal Payments (Divertible)
        .add_tier(
            WaterfallTier::new("principal_payments", 4, PaymentType::Principal)
                .allocation_mode(AllocationMode::Sequential)
                .divertible(true)
                .add_recipient(Recipient::tranche_principal(
                    "class_a_prin",
                    "CLASS_A",
                    None,
                ))
                .add_recipient(Recipient::tranche_principal(
                    "class_b_prin",
                    "CLASS_B",
                    None,
                ))
                .add_recipient(Recipient::tranche_principal(
                    "class_c_prin",
                    "CLASS_C",
                    None,
                ))
                .add_recipient(Recipient::tranche_principal(
                    "class_d_prin",
                    "CLASS_D",
                    None,
                )),
        )
        // Tier 5: Equity Distribution
        .add_tier(
            WaterfallTier::new("equity", 5, PaymentType::Residual)
                .allocation_mode(AllocationMode::Sequential)
                .add_recipient(Recipient::new(
                    "equity_distribution",
                    PaymentRecipient::Equity,
                    PaymentCalculation::ResidualCash,
                )),
        )
        // Coverage Triggers
        .add_coverage_trigger(CoverageTrigger {
            tranche_id: "CLASS_A".into(),
            oc_trigger: Some(1.25), // 125% OC
            ic_trigger: Some(1.20), // 120% IC
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clo_2_0_template_structure() {
        let waterfall = clo_2_0_template(Currency::USD);

        // Should have 5 tiers
        assert_eq!(waterfall.tiers.len(), 5);

        // Verify tier priorities
        assert_eq!(waterfall.tiers[0].id, "senior_expenses");
        assert_eq!(waterfall.tiers[1].id, "management_fees");
        assert_eq!(waterfall.tiers[2].id, "interest_payments");
        assert_eq!(waterfall.tiers[3].id, "principal_payments");
        assert_eq!(waterfall.tiers[4].id, "equity");

        // Verify principal tier is divertible
        assert!(waterfall.tiers[3].divertible);

        // Verify coverage triggers
        assert_eq!(waterfall.coverage_triggers.len(), 1);
        assert_eq!(waterfall.coverage_triggers[0].tranche_id, "CLASS_A");
        assert_eq!(waterfall.coverage_triggers[0].oc_trigger, Some(1.25));
        assert_eq!(waterfall.coverage_triggers[0].ic_trigger, Some(1.20));
    }

    #[test]
    fn test_tier_allocation_modes() {
        let waterfall = clo_2_0_template(Currency::USD);

        // All tiers should be sequential for CLO 2.0
        for tier in &waterfall.tiers {
            assert_eq!(tier.allocation_mode, AllocationMode::Sequential);
        }
    }

    #[test]
    fn test_interest_tier_recipients() {
        let waterfall = clo_2_0_template(Currency::USD);
        let interest_tier = &waterfall.tiers[2];

        // Should have 4 interest recipients (A, B, C, D)
        assert_eq!(interest_tier.recipients.len(), 4);
        assert_eq!(interest_tier.recipients[0].id, "class_a_int");
        assert_eq!(interest_tier.recipients[1].id, "class_b_int");
        assert_eq!(interest_tier.recipients[2].id, "class_c_int");
        assert_eq!(interest_tier.recipients[3].id, "class_d_int");
    }
}
