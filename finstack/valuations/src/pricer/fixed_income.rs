//! Pricer registrations for fixed-income instruments.
//!
//! Covers: FIIndexTotalReturnSwap, Convertible, InflationLinkedBond,
//! RevolvingCredit, TermLoan, AgencyMbsPassthrough, AgencyTba, DollarRoll, AgencyCmo.

use super::{InstrumentType, ModelKey, PricerRegistry};

/// Register pricers for additional fixed-income instruments (convertibles, MBS,
/// revolving credit, term loans) not included in the minimal rates set.
pub(crate) fn register_fixed_income_pricers(registry: &mut PricerRegistry) {
    // FI Index TRS
    registry.register(
        InstrumentType::FIIndexTotalReturnSwap,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap,
        >::discounting(InstrumentType::FIIndexTotalReturnSwap),
    );

    // Convertible Bond
    registry.register(
        InstrumentType::Convertible,
        ModelKey::Discounting,
        crate::instruments::fixed_income::convertible::pricer::ConvertibleTreePricer,
    );

    // Inflation Linked Bond
    registry.register(
        InstrumentType::InflationLinkedBond,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::fixed_income::inflation_linked_bond::InflationLinkedBond,
        >::discounting(InstrumentType::InflationLinkedBond),
    );

    // Revolving Credit
    registry.register(
        InstrumentType::RevolvingCredit,
        ModelKey::Discounting,
        crate::instruments::fixed_income::revolving_credit::pricer::RevolvingCreditPricer::new(
            ModelKey::Discounting,
        ),
    );

    registry.register(
        InstrumentType::RevolvingCredit,
        ModelKey::MonteCarloGBM,
        crate::instruments::fixed_income::revolving_credit::pricer::RevolvingCreditPricer::new(
            ModelKey::MonteCarloGBM,
        ),
    );

    // Term Loan (including DDTL)
    registry.register(
        InstrumentType::TermLoan,
        ModelKey::Discounting,
        crate::instruments::fixed_income::term_loan::pricing::TermLoanDiscountingPricer,
    );
    registry.register(
        InstrumentType::TermLoan,
        ModelKey::Tree,
        crate::instruments::fixed_income::term_loan::pricing::TermLoanTreePricer::default(),
    );

    // Agency MBS Passthrough
    registry.register(
        InstrumentType::AgencyMbsPassthrough,
        ModelKey::Discounting,
        crate::instruments::fixed_income::mbs_passthrough::AgencyMbsDiscountingPricer,
    );

    // Agency TBA
    registry.register(
        InstrumentType::AgencyTba,
        ModelKey::Discounting,
        crate::instruments::fixed_income::tba::AgencyTbaDiscountingPricer,
    );

    // Dollar Roll
    registry.register(
        InstrumentType::DollarRoll,
        ModelKey::Discounting,
        crate::instruments::fixed_income::dollar_roll::DollarRollDiscountingPricer,
    );

    // Agency CMO
    registry.register(
        InstrumentType::AgencyCmo,
        ModelKey::Discounting,
        crate::instruments::fixed_income::cmo::AgencyCmoDiscountingPricer,
    );
}
