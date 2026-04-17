//! Pricer registrations for credit instruments.
//!
//! Covers: CDS, CDSIndex, CDSTranche, CDSOption, StructuredCredit.
//!
//! # Model keys
//!
//! Credit products register only their *real* model key (`HazardRate` for
//! CDS / CDSIndex / CDSTranche, `Black76` for CDSOption).

use super::{InstrumentType, ModelKey, PricerRegistry};

/// Register pricers for credit instruments.
pub(crate) fn register_credit_pricers(registry: &mut PricerRegistry) {
    // CDS
    registry.register(
        InstrumentType::CDS,
        ModelKey::HazardRate,
        crate::instruments::common_impl::GenericInstrumentPricer::cds(),
    );

    // CDS Index
    registry.register(
        InstrumentType::CDSIndex,
        ModelKey::HazardRate,
        crate::instruments::credit_derivatives::cds_index::pricer::SimpleCdsIndexHazardPricer::default(),
    );

    // CDS Tranche
    registry.register(
        InstrumentType::CDSTranche,
        ModelKey::HazardRate,
        crate::instruments::credit_derivatives::cds_tranche::pricer::SimpleCDSTrancheHazardPricer::default(),
    );

    // CDS Option
    registry.register(
        InstrumentType::CDSOption,
        ModelKey::Black76,
        crate::instruments::credit_derivatives::cds_option::pricer::SimpleCDSOptionBlackPricer::default(),
    );

    // Structured Credit - unified pricer for ABS, CLO, CMBS, RMBS
    registry.register(
        InstrumentType::StructuredCredit,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::fixed_income::structured_credit::StructuredCredit,
        >::discounting(InstrumentType::StructuredCredit),
    );
}
