//! NDF pricer implementation.
//!
//! Provides the discounting pricer for NDF instruments with support for
//! both pre-fixing and post-fixing valuation modes.

use crate::instruments::common::traits::Instrument as Priceable;
use crate::instruments::ndf::Ndf;
use crate::pricer::{
    expect_inst, InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
    PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;

/// Discounting pricer for NDFs supporting pre-fixing and post-fixing modes.
///
/// # Pre-Fixing Mode
///
/// When `fixing_rate` is None and valuation date is before the fixing date,
/// the forward rate is estimated via CIRP (if foreign curve available) or
/// simplified fallback (for truly restricted currencies).
///
/// # Post-Fixing Mode
///
/// When `fixing_rate` is Some, the observed rate is used directly:
/// ```text
/// Settlement = notional × (1/contract_rate - 1/fixing_rate)
/// PV = Settlement × DF_settlement(T)
/// ```
pub struct NdfDiscountingPricer;

impl Pricer for NdfDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Ndf, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Priceable,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let ndf = expect_inst::<Ndf>(instrument, InstrumentType::Ndf)?;

        // Validate maturity
        if ndf.maturity_date <= as_of {
            return Err(PricingError::invalid_input_ctx(
                format!(
                    "NDF maturity {} must be after valuation date {}",
                    ndf.maturity_date, as_of
                ),
                PricingErrorContext::default(),
            ));
        }

        // Validate fixing date <= maturity
        if ndf.fixing_date > ndf.maturity_date {
            return Err(PricingError::invalid_input_ctx(
                format!(
                    "NDF fixing date {} must be on or before maturity date {}",
                    ndf.fixing_date, ndf.maturity_date
                ),
                PricingErrorContext::default(),
            ));
        }

        // Delegate to instrument's value method
        let pv = ndf.value(market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(
                format!("NDF pricing failed: {}", e),
                PricingErrorContext::default(),
            )
        })?;

        Ok(ValuationResult::stamped(ndf.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
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
        // Create flat discount curve using builder
        let usd_curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (0.5, 0.9753), (1.0, 0.9512)])
            .build()
            .expect("should build");

        // Create FX provider with CNY/USD = 7.25
        let fx_provider = Arc::new(SimpleFxProvider::new());
        fx_provider.set_quote(Currency::CNY, Currency::USD, 7.25);
        let fx_matrix = FxMatrix::new(fx_provider);

        MarketContext::new()
            .insert_discount(usd_curve)
            .insert_fx(fx_matrix)
    }

    #[test]
    fn test_ndf_pricing_at_market() {
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid date");
        let market = create_test_market(as_of);

        // Create NDF at market rate
        let ndf = Ndf::builder()
            .id(InstrumentId::new("TEST"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2024, Month::April, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2024, Month::April, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25) // At market spot
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("valid");

        let pricer = NdfDiscountingPricer;
        let result = pricer
            .price_dyn(&ndf, &market, as_of)
            .expect("should price");

        // At-market NDF should have PV ≈ 0 (small due to discounting)
        assert!(
            result.value.amount().abs() < 1000.0,
            "At-market NDF PV should be near zero, got {}",
            result.value.amount()
        );
        assert_eq!(result.value.currency(), Currency::USD);
    }

    #[test]
    fn test_ndf_pricing_with_fixing_rate() {
        let as_of = Date::from_calendar_date(2024, Month::April, 14).expect("valid date");
        let market = create_test_market(as_of);

        // Create NDF that has been fixed
        let ndf = Ndf::builder()
            .id(InstrumentId::new("TEST"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2024, Month::April, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2024, Month::April, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .fixing_rate_opt(Some(7.30)) // Fixed above contract rate
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("valid");

        let pricer = NdfDiscountingPricer;
        let result = pricer
            .price_dyn(&ndf, &market, as_of)
            .expect("should price");

        // Fixing rate > contract rate means positive PV (we're receiving more than expected)
        // Settlement = 10M × (1/7.25 - 1/7.30) ≈ 10M × 0.000943 ≈ $9,430
        assert!(
            result.value.amount() > 0.0,
            "NDF with favorable fixing should have positive PV"
        );
        assert_eq!(result.value.currency(), Currency::USD);
    }

    #[test]
    fn test_ndf_pricing_unfavorable_fixing() {
        let as_of = Date::from_calendar_date(2024, Month::April, 14).expect("valid date");
        let market = create_test_market(as_of);

        // Create NDF fixed below contract rate
        let ndf = Ndf::builder()
            .id(InstrumentId::new("TEST"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2024, Month::April, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2024, Month::April, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .fixing_rate_opt(Some(7.20)) // Fixed below contract rate
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("valid");

        let pricer = NdfDiscountingPricer;
        let result = pricer
            .price_dyn(&ndf, &market, as_of)
            .expect("should price");

        // Fixing rate < contract rate means negative PV
        assert!(
            result.value.amount() < 0.0,
            "NDF with unfavorable fixing should have negative PV"
        );
    }

    #[test]
    fn test_ndf_expired() {
        let as_of = Date::from_calendar_date(2024, Month::April, 20).expect("valid date");
        let market = create_test_market(as_of);

        // NDF that has already matured
        let ndf = Ndf::builder()
            .id(InstrumentId::new("TEST"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2024, Month::April, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2024, Month::April, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("valid");

        let pricer = NdfDiscountingPricer;
        let result = pricer.price_dyn(&ndf, &market, as_of);

        // Should return error for expired NDF
        assert!(result.is_err());
    }
}
