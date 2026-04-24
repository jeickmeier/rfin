//! Pricer registrations for credit instruments.
//!
//! Covers: CDS, CDSIndex, CDSTranche, CDSOption, StructuredCredit.
//!
//! # Model keys
//!
//! Credit products register only their *real* model key (`HazardRate` for
//! CDS / CDSIndex / CDSTranche, `Black76` for CDSOption).

use super::{
    expect_inst, InstrumentType, ModelKey, Pricer, PricerKey, PricerRegistry, PricingError,
    PricingErrorContext, PricingResult,
};
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::structured_credit::{
    StochasticPricingResult, StructuredCredit,
};
use crate::metrics::MetricId;
use crate::results::{ValuationDetails, ValuationResult};
use finstack_core::market_data::context::MarketContext;
use indexmap::IndexMap;

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

    registry.register(
        InstrumentType::StructuredCredit,
        ModelKey::StructuredCreditStochastic,
        StructuredCreditStochasticPricer,
    );
}

struct StructuredCreditStochasticPricer;

impl Pricer for StructuredCreditStochasticPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(
            InstrumentType::StructuredCredit,
            ModelKey::StructuredCreditStochastic,
        )
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> PricingResult<ValuationResult> {
        let structured_credit =
            expect_inst::<StructuredCredit>(instrument, InstrumentType::StructuredCredit)?;
        let stochastic = structured_credit
            .price_stochastic(market, as_of)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        let measures = stochastic_summary_measures(&stochastic);
        Ok(
            ValuationResult::stamped(structured_credit.id(), as_of, stochastic.npv)
                .with_measures(measures)
                .with_details(ValuationDetails::StructuredCreditStochastic(stochastic)),
        )
    }
}

fn stochastic_summary_measures(result: &StochasticPricingResult) -> IndexMap<MetricId, f64> {
    let mut measures = IndexMap::new();
    measures.insert(
        MetricId::custom("expected_loss"),
        result.expected_loss.amount(),
    );
    measures.insert(
        MetricId::custom("unexpected_loss"),
        result.unexpected_loss.amount(),
    );
    measures.insert(
        MetricId::custom("expected_shortfall"),
        result.expected_shortfall.amount(),
    );
    measures.insert(MetricId::custom("pv_std_error"), result.pv_std_error);

    for tranche in &result.tranche_results {
        measures.insert(
            MetricId::custom(format!("tranche_npv::{}", tranche.tranche_id)),
            tranche.npv.amount(),
        );
        measures.insert(
            MetricId::custom(format!("tranche_expected_loss::{}", tranche.tranche_id)),
            tranche.expected_loss.amount(),
        );
        measures.insert(
            MetricId::custom(format!("tranche_wal::{}", tranche.tranche_id)),
            tranche.average_life,
        );
    }

    measures
}
