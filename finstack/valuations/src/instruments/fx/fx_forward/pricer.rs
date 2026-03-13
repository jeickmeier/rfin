//! FX Forward pricer implementation.
//!
//! Provides the discounting pricer for FX forward instruments using
//! covered interest rate parity (CIRP).

use crate::instruments::common_impl::traits::Instrument as Priceable;
use crate::instruments::fx::fx_forward::FxForward;
use crate::pricer::{
    expect_inst, InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
    PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;

/// Discounting pricer for FX forwards using covered interest rate parity.
///
/// # Pricing Formula
///
/// ```text
/// F_market = S × DF_foreign(T) / DF_domestic(T)
/// PV = notional × (F_market - F_contract) × DF_domestic(T)
/// ```
///
/// where:
/// - S = spot FX rate
/// - DF_foreign(T) = discount factor in base (foreign) currency
/// - DF_domestic(T) = discount factor in quote (domestic) currency
/// - F_contract = contract forward rate (or F_market if at-market)
pub struct FxForwardDiscountingPricer;

impl Pricer for FxForwardDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxForward, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Priceable,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let fwd = expect_inst::<FxForward>(instrument, InstrumentType::FxForward)?;

        // Delegate to instrument's value method.
        // Note: value() returns zero PV for expired forwards (maturity <= as_of),
        // which is the expected behavior for settled trades.
        let pv = fwd.value(market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(
                format!("FX forward pricing failed: {}", e),
                PricingErrorContext::default(),
            )
        })?;

        Ok(ValuationResult::stamped(fwd.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, deprecated)]
mod tests {
    use super::*;
    use crate::pricer::Pricer;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::fx::{FxMatrix, SimpleFxProvider};
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, InstrumentId};
    use std::sync::Arc;
    use time::Month;

    fn create_test_market(as_of: Date) -> MarketContext {
        // Create flat discount curves using builder
        let usd_curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (0.5, 0.9753), (1.0, 0.9512)])
            .build()
            .expect("should build");

        let eur_curve = DiscountCurve::builder("EUR-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (0.5, 0.9851), (1.0, 0.9704)])
            .build()
            .expect("should build");

        // Create FX provider with EUR/USD = 1.10
        let fx_provider =
            Arc::new(SimpleFxProvider::new().with_quote(Currency::EUR, Currency::USD, 1.10));
        let fx_matrix = FxMatrix::new(fx_provider);

        MarketContext::new()
            .insert(usd_curve)
            .insert(eur_curve)
            .insert_fx(fx_matrix)
    }

    #[test]
    fn test_fx_forward_pricing_at_market() {
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid date");
        let market = create_test_market(as_of);

        // Create at-market forward (no contract rate)
        let fwd = FxForward::builder()
            .id(InstrumentId::new("TEST"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .maturity(Date::from_calendar_date(2024, Month::July, 15).expect("valid date"))
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .build()
            .expect("valid");

        let pricer = FxForwardDiscountingPricer;
        let result = pricer
            .price_dyn(&fwd, &market, as_of)
            .expect("should price");

        // At-market forward should have PV ≈ 0
        assert!(
            result.value.amount().abs() < 1.0,
            "At-market forward PV should be near zero, got {}",
            result.value.amount()
        );
        assert_eq!(result.value.currency(), Currency::USD);
    }

    #[test]
    fn test_fx_forward_pricing_with_contract_rate() {
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid date");
        let market = create_test_market(as_of);

        // Create forward with favorable contract rate (below market forward)
        // Market forward should be approximately 1.10 * exp((0.03 - 0.05) * 0.5) ≈ 1.089
        let fwd = FxForward::builder()
            .id(InstrumentId::new("TEST"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .maturity(Date::from_calendar_date(2024, Month::July, 15).expect("valid date"))
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .contract_rate_opt(Some(1.05)) // Below market forward, so positive PV
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .build()
            .expect("valid");

        let pricer = FxForwardDiscountingPricer;
        let result = pricer
            .price_dyn(&fwd, &market, as_of)
            .expect("should price");

        // Contract rate below market forward means positive PV (we're buying EUR cheap)
        assert!(
            result.value.amount() > 0.0,
            "Forward with favorable rate should have positive PV"
        );
        assert_eq!(result.value.currency(), Currency::USD);
    }

    #[test]
    fn test_fx_forward_expired() {
        let as_of = Date::from_calendar_date(2024, Month::July, 20).expect("valid date");
        let market = create_test_market(as_of);

        // Forward that has already matured
        let fwd = FxForward::builder()
            .id(InstrumentId::new("TEST"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .maturity(Date::from_calendar_date(2024, Month::July, 15).expect("valid date"))
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .build()
            .expect("valid");

        let pricer = FxForwardDiscountingPricer;
        let result = pricer
            .price_dyn(&fwd, &market, as_of)
            .expect("should price expired forward");

        // Expired forward should return zero PV (settled trade)
        assert_eq!(
            result.value.amount(),
            0.0,
            "Expired forward should have zero PV"
        );
        assert_eq!(result.value.currency(), Currency::USD);
    }

    #[test]
    fn test_fx_forward_same_day_maturity() {
        let as_of = Date::from_calendar_date(2024, Month::July, 15).expect("valid date");
        let market = create_test_market(as_of);

        // Forward maturing on valuation date
        let fwd = FxForward::builder()
            .id(InstrumentId::new("TEST"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .maturity(as_of)
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .build()
            .expect("valid");

        let pricer = FxForwardDiscountingPricer;
        let result = pricer
            .price_dyn(&fwd, &market, as_of)
            .expect("should price same-day maturity");

        // Same-day maturity should return zero PV
        assert_eq!(
            result.value.amount(),
            0.0,
            "Same-day maturity forward should have zero PV"
        );
    }
}
