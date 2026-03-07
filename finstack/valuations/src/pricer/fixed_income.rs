//! Pricer registrations for fixed-income instruments.
//!
//! Covers: FIIndexTotalReturnSwap, Convertible, InflationLinkedBond,
//! RevolvingCredit, TermLoan, AgencyMbsPassthrough, AgencyTba, DollarRoll, AgencyCmo.

use super::*;

macro_rules! register_pricer {
    ($registry:expr, $inst:ident, $model:ident, $pricer:expr) => {
        $registry.register_pricer(
            PricerKey::new(InstrumentType::$inst, ModelKey::$model),
            Box::new($pricer),
        );
    };
}

/// Register pricers for additional fixed-income instruments (convertibles, MBS,
/// revolving credit, term loans) not included in the minimal rates set.
pub fn register_fixed_income_pricers(registry: &mut PricerRegistry) {
    // FI Index TRS
    register_pricer!(
        registry,
        FIIndexTotalReturnSwap,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap,
        >::discounting(InstrumentType::FIIndexTotalReturnSwap)
    );

    // Convertible Bond
    register_pricer!(
        registry,
        Convertible,
        Discounting,
        crate::instruments::fixed_income::convertible::pricer::ConvertibleTreePricer
    );

    // Inflation Linked Bond
    register_pricer!(
        registry,
        InflationLinkedBond,
        Discounting,
        crate::instruments::fixed_income::inflation_linked_bond::pricer::SimpleInflationLinkedBondDiscountingPricer::default()
    );

    // Revolving Credit
    register_pricer!(
        registry,
        RevolvingCredit,
        Discounting,
        crate::instruments::fixed_income::revolving_credit::pricer::RevolvingCreditPricer::new(
            ModelKey::Discounting
        )
    );
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        RevolvingCredit,
        MonteCarloGBM,
        crate::instruments::fixed_income::revolving_credit::pricer::RevolvingCreditPricer::new(
            ModelKey::MonteCarloGBM
        )
    );

    // Term Loan (including DDTL)
    register_pricer!(
        registry,
        TermLoan,
        Discounting,
        crate::instruments::fixed_income::term_loan::pricing::TermLoanDiscountingPricer
    );
    register_pricer!(
        registry,
        TermLoan,
        Tree,
        crate::instruments::fixed_income::term_loan::pricing::TermLoanTreePricer::default()
    );

    // Agency MBS Passthrough
    register_pricer!(
        registry,
        AgencyMbsPassthrough,
        Discounting,
        crate::instruments::fixed_income::mbs_passthrough::AgencyMbsDiscountingPricer
    );

    // Agency TBA
    register_pricer!(
        registry,
        AgencyTba,
        Discounting,
        crate::instruments::fixed_income::tba::AgencyTbaDiscountingPricer
    );

    // Dollar Roll
    register_pricer!(
        registry,
        DollarRoll,
        Discounting,
        crate::instruments::fixed_income::dollar_roll::DollarRollDiscountingPricer
    );

    // Agency CMO
    register_pricer!(
        registry,
        AgencyCmo,
        Discounting,
        crate::instruments::fixed_income::cmo::AgencyCmoDiscountingPricer
    );
}
