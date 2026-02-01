use crate::instruments::common::traits::Instrument;
use crate::instruments::equity::pe_fund::PrivateMarketsFund;
use crate::pricer::{
    expect_inst, InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;

/// Simplified discounting pricer for private markets funds.
pub struct PrivateMarketsFundDiscountingPricer;

impl PrivateMarketsFundDiscountingPricer {
    /// Create a new private markets fund pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrivateMarketsFundDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for PrivateMarketsFundDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::PrivateMarketsFund, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let fund =
            expect_inst::<PrivateMarketsFund>(instrument, InstrumentType::PrivateMarketsFund)?;

        let as_of = if let Some(ref discount_curve_id) = fund.discount_curve_id {
            let disc = market
                .get_discount(discount_curve_id.as_str())
                .map_err(|e| {
                    PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
                })?;
            disc.base_date()
        } else {
            fund.events
                .iter()
                .map(|evt| evt.date)
                .max()
                .ok_or_else(|| {
                    PricingError::model_failure_ctx(
                        "Private markets fund requires at least one event to derive valuation date"
                            .to_string(),
                        PricingErrorContext::default(),
                    )
                })?
        };

        let pv = fund.value(market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(fund.id(), as_of, pv))
    }
}
