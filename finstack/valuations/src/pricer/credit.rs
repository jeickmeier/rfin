//! Pricer registrations for credit instruments.
//!
//! Covers: CDS, CDSIndex, CDSTranche, CDSOption, StructuredCredit.

use super::*;

macro_rules! register_pricer {
    ($registry:expr, $inst:ident, $model:ident, $pricer:expr) => {
        $registry.register_pricer(
            PricerKey::new(InstrumentType::$inst, ModelKey::$model),
            Box::new($pricer),
        );
    };
}

/// Register pricers for credit instruments.
pub fn register_credit_pricers(registry: &mut PricerRegistry) {
    // CDS
    register_pricer!(
        registry,
        CDS,
        HazardRate,
        crate::instruments::common_impl::GenericInstrumentPricer::cds()
    );
    register_pricer!(
        registry,
        CDS,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::CreditDefaultSwap,
        >::new(InstrumentType::CDS, ModelKey::Discounting)
    );

    // CDS Index
    register_pricer!(
        registry,
        CDSIndex,
        HazardRate,
        crate::instruments::credit_derivatives::cds_index::pricer::SimpleCdsIndexHazardPricer::default()
    );
    register_pricer!(
        registry,
        CDSIndex,
        Discounting,
        crate::instruments::credit_derivatives::cds_index::pricer::SimpleCdsIndexHazardPricer::with_model(
            ModelKey::Discounting
        )
    );

    // CDS Tranche
    register_pricer!(
        registry,
        CDSTranche,
        HazardRate,
        crate::instruments::credit_derivatives::cds_tranche::pricer::SimpleCDSTrancheHazardPricer::default()
    );
    register_pricer!(
        registry,
        CDSTranche,
        Discounting,
        crate::instruments::credit_derivatives::cds_tranche::pricer::SimpleCDSTrancheHazardPricer::with_model(
            ModelKey::Discounting
        )
    );

    // CDS Option
    register_pricer!(
        registry,
        CDSOption,
        Black76,
        crate::instruments::credit_derivatives::cds_option::pricer::SimpleCDSOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        CDSOption,
        Discounting,
        crate::instruments::credit_derivatives::cds_option::pricer::SimpleCDSOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Structured Credit - unified pricer for ABS, CLO, CMBS, RMBS
    register_pricer!(
        registry,
        StructuredCredit,
        Discounting,
        crate::instruments::fixed_income::structured_credit::StructuredCreditDiscountingPricer::default()
    );
}
