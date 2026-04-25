use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::pe_fund::waterfall::{AllocationLedger, EquityWaterfallEngine};
use crate::instruments::equity::pe_fund::PrivateMarketsFund;
use crate::pricer::{
    expect_inst, InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

pub(crate) fn run_waterfall(fund: &PrivateMarketsFund) -> finstack_core::Result<AllocationLedger> {
    for event in &fund.events {
        if event.amount.currency() != fund.currency {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: fund.currency,
                actual: event.amount.currency(),
            });
        }
    }
    let engine = EquityWaterfallEngine::new(&fund.waterfall_spec);
    engine.run(&fund.events)
}

pub(crate) fn lp_cashflows(
    fund: &PrivateMarketsFund,
) -> finstack_core::Result<Vec<(finstack_core::dates::Date, Money)>> {
    let ledger = run_waterfall(fund)?;
    Ok(ledger.lp_cashflows())
}

pub(crate) fn compute_pv(
    fund: &PrivateMarketsFund,
    curves: &MarketContext,
) -> finstack_core::Result<Money> {
    if let Some(ref discount_curve_id) = fund.discount_curve_id {
        use crate::instruments::common_impl::discountable::Discountable;
        let flows = lp_cashflows(fund)?;
        let disc = curves.get_discount(discount_curve_id.as_str())?;
        flows.npv(
            disc.as_ref(),
            disc.base_date(),
            Some(fund.waterfall_spec.irr_basis),
        )
    } else {
        let ledger = run_waterfall(fund)?;
        let residual_value = ledger
            .rows
            .last()
            .map(|r| r.lp_unreturned)
            .unwrap_or_else(|| Money::new(0.0, fund.currency));
        Ok(residual_value)
    }
}

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

    #[tracing::instrument(
        name = "pe_fund.discounting.price_dyn",
        level = "debug",
        skip(self, instrument, market),
        fields(inst_id = %instrument.id(), as_of = %_as_of),
        err,
    )]
    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let fund =
            expect_inst::<PrivateMarketsFund>(instrument, InstrumentType::PrivateMarketsFund)?;

        let err_ctx = || PricingErrorContext::from_instrument(fund).model(ModelKey::Discounting);

        let as_of = if let Some(ref discount_curve_id) = fund.discount_curve_id {
            let disc = market
                .get_discount(discount_curve_id.as_str())
                .map_err(|e| {
                    PricingError::model_failure_with_context(
                        e.to_string(),
                        err_ctx().curve_id(discount_curve_id.as_str()),
                    )
                })?;
            disc.base_date()
        } else {
            fund.events
                .iter()
                .map(|evt| evt.date)
                .max()
                .ok_or_else(|| {
                    PricingError::model_failure_with_context(
                        "Private markets fund requires at least one event to derive valuation date"
                            .to_string(),
                        err_ctx(),
                    )
                })?
        };

        let pv = compute_pv(fund, market)
            .map_err(|e| PricingError::model_failure_with_context(e.to_string(), err_ctx()))?;

        Ok(ValuationResult::stamped(fund.id(), as_of, pv))
    }
}
