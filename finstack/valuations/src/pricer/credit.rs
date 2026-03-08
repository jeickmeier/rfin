//! Pricer registrations for credit instruments.
//!
//! Covers: CDS, CDSIndex, CDSTranche, CDSOption, StructuredCredit.

use super::{InstrumentType, ModelKey, PricerRegistry};

/// Register pricers for credit instruments.
pub fn register_credit_pricers(registry: &mut PricerRegistry) {
    // CDS
    registry.register(
        InstrumentType::CDS,
        ModelKey::HazardRate,
        crate::instruments::common_impl::GenericInstrumentPricer::cds(),
    );
    registry.register(
        InstrumentType::CDS,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::CreditDefaultSwap,
        >::new(InstrumentType::CDS, ModelKey::Discounting),
    );

    // CDS Index
    registry.register(
        InstrumentType::CDSIndex,
        ModelKey::HazardRate,
        crate::instruments::credit_derivatives::cds_index::pricer::SimpleCdsIndexHazardPricer::default(),
    );
    registry.register(
        InstrumentType::CDSIndex,
        ModelKey::Discounting,
        crate::instruments::credit_derivatives::cds_index::pricer::SimpleCdsIndexHazardPricer::with_model(
            ModelKey::Discounting,
        ),
    );

    // CDS Tranche
    registry.register(
        InstrumentType::CDSTranche,
        ModelKey::HazardRate,
        crate::instruments::credit_derivatives::cds_tranche::pricer::SimpleCDSTrancheHazardPricer::default(),
    );
    registry.register(
        InstrumentType::CDSTranche,
        ModelKey::Discounting,
        crate::instruments::credit_derivatives::cds_tranche::pricer::SimpleCDSTrancheHazardPricer::with_model(
            ModelKey::Discounting,
        ),
    );

    // CDS Option
    registry.register(
        InstrumentType::CDSOption,
        ModelKey::Black76,
        crate::instruments::credit_derivatives::cds_option::pricer::SimpleCDSOptionBlackPricer::default(),
    );
    registry.register(
        InstrumentType::CDSOption,
        ModelKey::Discounting,
        crate::instruments::credit_derivatives::cds_option::pricer::SimpleCDSOptionBlackPricer::with_model(
            ModelKey::Discounting,
        ),
    );

    // Structured Credit - unified pricer for ABS, CLO, CMBS, RMBS
    registry.register(
        InstrumentType::StructuredCredit,
        ModelKey::Discounting,
        crate::instruments::fixed_income::structured_credit::StructuredCreditDiscountingPricer::default(),
    );
}
