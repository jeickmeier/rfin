//! CRE operating company cash distribution waterfall template.
//!
//! Implements a typical operating company waterfall for commercial real estate with:
//! - Operating expenses and debt service
//! - Capital reserves
//! - Investor preferred returns
//! - Promote/incentive distributions
//! - Residual cash to equity

use super::super::components::{
    AllocationMode, ManagementFeeType, PaymentCalculation, PaymentRecipient, PaymentType,
    Recipient, WaterfallBuilder, WaterfallEngine, WaterfallTier,
};
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Create a CRE operating company waterfall template
///
/// # Waterfall Structure
///
/// ## Tier 1: Operating Expenses
/// - Property management fees (pro-rata)
/// - Operating costs
/// - Property taxes and insurance
///
/// ## Tier 2: Debt Service
/// - Senior debt interest
/// - Senior debt principal
/// - Mezzanine debt interest
/// - Mezzanine debt principal
///
/// ## Tier 3: Capital Reserves
/// - CapEx reserve funding
/// - TI/LC reserve funding
///
/// ## Tier 4: Preferred Return (Pro-Rata)
/// - LP preferred return (8% hurdle)
/// - GP preferred return (8% hurdle)
///
/// ## Tier 5: Catchup (Pro-Rata)
/// - GP catchup to 20% of profits
///
/// ## Tier 6: Residual Split (Pro-Rata)
/// - LP residual (80%)
/// - GP residual/promote (20%)
///
/// # Notes
///
/// - Pro-rata tiers distribute by ownership percentage
/// - Typical promote structure: 8% pref → catchup → 80/20 split
/// - Can be customized for different hurdle rates and splits
pub fn cre_operating_company_template(currency: Currency) -> WaterfallEngine {
    WaterfallBuilder::new(currency)
        // Tier 1: Operating Expenses (Pro-Rata)
        .add_tier(
            WaterfallTier::new("operating_expenses", 1, PaymentType::Fee)
                .allocation_mode(AllocationMode::ProRata)
                .add_recipient(
                    Recipient::new(
                        "property_mgmt",
                        PaymentRecipient::ServiceProvider("PropertyManager".into()),
                        PaymentCalculation::PercentageOfCollateral {
                            rate: 0.03, // 3% of property value
                            annualized: true,
                        },
                    )
                    .with_weight(1.0),
                )
                .add_recipient(
                    Recipient::new(
                        "operating_costs",
                        PaymentRecipient::ServiceProvider("Operating".into()),
                        PaymentCalculation::FixedAmount {
                            amount: Money::new(100_000.0, currency),
                        },
                    )
                    .with_weight(1.0),
                )
                .add_recipient(
                    Recipient::new(
                        "property_tax",
                        PaymentRecipient::ServiceProvider("Municipality".into()),
                        PaymentCalculation::FixedAmount {
                            amount: Money::new(50_000.0, currency),
                        },
                    )
                    .with_weight(1.0),
                ),
        )
        // Tier 2: Debt Service (Sequential)
        .add_tier(
            WaterfallTier::new("debt_service", 2, PaymentType::Interest)
                .allocation_mode(AllocationMode::Sequential)
                .add_recipient(Recipient::tranche_interest(
                    "senior_debt_int",
                    "SENIOR_DEBT",
                ))
                .add_recipient(Recipient::tranche_principal(
                    "senior_debt_prin",
                    "SENIOR_DEBT",
                    None,
                ))
                .add_recipient(Recipient::tranche_interest("mezz_debt_int", "MEZZ_DEBT"))
                .add_recipient(Recipient::tranche_principal(
                    "mezz_debt_prin",
                    "MEZZ_DEBT",
                    None,
                )),
        )
        // Tier 3: Capital Reserves (Pro-Rata)
        .add_tier(
            WaterfallTier::new("capital_reserves", 3, PaymentType::Fee)
                .allocation_mode(AllocationMode::ProRata)
                .add_recipient(
                    Recipient::new(
                        "capex_reserve",
                        PaymentRecipient::ServiceProvider("CapExReserve".into()),
                        PaymentCalculation::FixedAmount {
                            amount: Money::new(25_000.0, currency),
                        },
                    )
                    .with_weight(0.5),
                )
                .add_recipient(
                    Recipient::new(
                        "tilc_reserve",
                        PaymentRecipient::ServiceProvider("TI_LC_Reserve".into()),
                        PaymentCalculation::FixedAmount {
                            amount: Money::new(25_000.0, currency),
                        },
                    )
                    .with_weight(0.5),
                ),
        )
        // Tier 4: Preferred Return (Pro-Rata, 8% hurdle)
        .add_tier(
            WaterfallTier::new("preferred_return", 4, PaymentType::Interest)
                .allocation_mode(AllocationMode::ProRata)
                .add_recipient(
                    Recipient::new(
                        "lp_pref",
                        PaymentRecipient::Tranche("LP".into()),
                        PaymentCalculation::PercentageOfCollateral {
                            rate: 0.08, // 8% preferred return
                            annualized: true,
                        },
                    )
                    .with_weight(0.95), // 95% LP ownership
                )
                .add_recipient(
                    Recipient::new(
                        "gp_pref",
                        PaymentRecipient::Tranche("GP".into()),
                        PaymentCalculation::PercentageOfCollateral {
                            rate: 0.08, // 8% preferred return
                            annualized: true,
                        },
                    )
                    .with_weight(0.05), // 5% GP ownership
                ),
        )
        // Tier 5: GP Catchup (gets GP to 20% of total distributions)
        .add_tier(
            WaterfallTier::new("gp_catchup", 5, PaymentType::Interest)
                .allocation_mode(AllocationMode::Sequential)
                .add_recipient(Recipient::new(
                    "gp_catchup_dist",
                    PaymentRecipient::ManagerFee(ManagementFeeType::Incentive),
                    PaymentCalculation::ResidualCash,
                )),
        )
        // Tier 6: Residual Split (80/20 LP/GP)
        .add_tier(
            WaterfallTier::new("residual_split", 6, PaymentType::Residual)
                .allocation_mode(AllocationMode::ProRata)
                .add_recipient(
                    Recipient::new(
                        "lp_residual",
                        PaymentRecipient::Tranche("LP".into()),
                        PaymentCalculation::ResidualCash,
                    )
                    .with_weight(0.80), // 80% to LP
                )
                .add_recipient(
                    Recipient::new(
                        "gp_promote",
                        PaymentRecipient::ManagerFee(ManagementFeeType::Incentive),
                        PaymentCalculation::ResidualCash,
                    )
                    .with_weight(0.20), // 20% promote to GP
                ),
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cre_template_structure() {
        let waterfall = cre_operating_company_template(Currency::USD);

        // Should have 6 tiers
        assert_eq!(waterfall.tiers.len(), 6);

        // Verify tier priorities
        assert_eq!(waterfall.tiers[0].id, "operating_expenses");
        assert_eq!(waterfall.tiers[1].id, "debt_service");
        assert_eq!(waterfall.tiers[2].id, "capital_reserves");
        assert_eq!(waterfall.tiers[3].id, "preferred_return");
        assert_eq!(waterfall.tiers[4].id, "gp_catchup");
        assert_eq!(waterfall.tiers[5].id, "residual_split");
    }

    #[test]
    fn test_pro_rata_tiers() {
        let waterfall = cre_operating_company_template(Currency::USD);

        // Operating expenses should be pro-rata
        assert_eq!(waterfall.tiers[0].allocation_mode, AllocationMode::ProRata);

        // Preferred return should be pro-rata
        assert_eq!(waterfall.tiers[3].allocation_mode, AllocationMode::ProRata);

        // Residual split should be pro-rata
        assert_eq!(waterfall.tiers[5].allocation_mode, AllocationMode::ProRata);
    }

    #[test]
    fn test_sequential_tiers() {
        let waterfall = cre_operating_company_template(Currency::USD);

        // Debt service should be sequential
        assert_eq!(
            waterfall.tiers[1].allocation_mode,
            AllocationMode::Sequential
        );

        // GP catchup should be sequential
        assert_eq!(
            waterfall.tiers[4].allocation_mode,
            AllocationMode::Sequential
        );
    }

    #[test]
    fn test_lp_gp_weights() {
        let waterfall = cre_operating_company_template(Currency::USD);
        let pref_tier = &waterfall.tiers[3];

        // Should have LP and GP recipients
        assert_eq!(pref_tier.recipients.len(), 2);

        // LP should have 95% weight
        assert_eq!(pref_tier.recipients[0].weight, Some(0.95));

        // GP should have 5% weight
        assert_eq!(pref_tier.recipients[1].weight, Some(0.05));

        // Residual split should be 80/20
        let residual_tier = &waterfall.tiers[5];
        assert_eq!(residual_tier.recipients[0].weight, Some(0.80));
        assert_eq!(residual_tier.recipients[1].weight, Some(0.20));
    }
}
