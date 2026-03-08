//! Volatility index future pricer implementation.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::vol_index_future::VolatilityIndexFuture;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

/// Registry-facing pricer for volatility index futures.
pub struct VolIndexFutureDiscountingPricer;

pub(crate) fn compute_pv(
    future: &VolatilityIndexFuture,
    context: &MarketContext,
) -> finstack_core::Result<Money> {
    Ok(Money::new(
        compute_pv_raw(future, context)?,
        future.notional.currency(),
    ))
}

pub(crate) fn compute_pv_raw(
    future: &VolatilityIndexFuture,
    context: &MarketContext,
) -> finstack_core::Result<f64> {
    let forward_vol = forward_vol(future, context)?;
    let sign = match future.position {
        crate::instruments::rates::ir_future::Position::Long => 1.0,
        crate::instruments::rates::ir_future::Position::Short => -1.0,
    };
    let contracts = future.num_contracts();
    let pv_per_contract = (future.quoted_price - forward_vol) * future.contract_specs.multiplier;
    Ok(sign * contracts * pv_per_contract)
}

pub(crate) fn forward_vol(
    future: &VolatilityIndexFuture,
    context: &MarketContext,
) -> finstack_core::Result<f64> {
    let vol_curve = context.get_vol_index_curve(&future.vol_index_curve_id)?;
    let t = vol_curve
        .day_count()
        .year_fraction(
            vol_curve.base_date(),
            future.settlement_date,
            finstack_core::dates::DayCountCtx::default(),
        )?
        .max(0.0);
    Ok(vol_curve.forward_level(t))
}

pub(crate) fn delta_vol(future: &VolatilityIndexFuture) -> f64 {
    let sign = match future.position {
        crate::instruments::rates::ir_future::Position::Long => 1.0,
        crate::instruments::rates::ir_future::Position::Short => -1.0,
    };
    -sign * future.num_contracts() * future.contract_specs.multiplier
}

impl Default for VolIndexFutureDiscountingPricer {
    fn default() -> Self {
        Self
    }
}

impl Pricer for VolIndexFutureDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::VolatilityIndexFuture, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let future = instrument
            .as_any()
            .downcast_ref::<VolatilityIndexFuture>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::VolatilityIndexFuture, instrument.key())
            })?;
        let pv = compute_pv(future, market).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;
        Ok(ValuationResult::stamped(future.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Attributes;
    use crate::instruments::rates::ir_future::Position;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::{DiscountCurve, VolatilityIndexCurve};
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn setup_market() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96)])
            .build()
            .expect("valid discount curve");
        let vix = VolatilityIndexCurve::builder("VIX")
            .base_date(base_date)
            .spot_level(18.0)
            .knots([(0.0, 18.0), (0.25, 20.0), (0.5, 21.0), (1.0, 22.0)])
            .build()
            .expect("valid VIX curve");
        MarketContext::new().insert(disc).insert(vix)
    }

    fn sample_future() -> VolatilityIndexFuture {
        VolatilityIndexFuture::builder()
            .id(InstrumentId::new("VIX-PRICER"))
            .notional(Money::new(20_000.0, Currency::USD))
            .expiry(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .settlement_date(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .quoted_price(20.0)
            .position(Position::Long)
            .contract_specs(
                crate::instruments::equity::vol_index_future::VolIndexContractSpecs::vix(),
            )
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .attributes(Attributes::new())
            .build()
            .expect("valid future")
    }

    #[test]
    fn compute_pv_matches_instrument_value() {
        let market = setup_market();
        let future = sample_future();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let via_pricer = compute_pv(&future, &market).expect("pricer pv");
        let via_instrument = future.value(&market, as_of).expect("instrument pv");

        assert_eq!(via_pricer, via_instrument);
    }
}
