//! Equity index future pricer engine.
//!
//! Provides deterministic PV for `EquityIndexFuture` instruments using
//! mark-to-market or cost-of-carry fair value pricing.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::equity_index_future::EquityIndexFuture;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

pub(crate) fn compute_pv(
    future: &EquityIndexFuture,
    market: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Money> {
    Ok(Money::new(
        compute_pv_raw(future, market, as_of)?,
        future.notional.currency(),
    ))
}

pub(crate) fn compute_pv_raw(
    future: &EquityIndexFuture,
    market: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<f64> {
    if future.expiry < as_of {
        return Ok(0.0);
    }
    if let Some(quoted) = future.quoted_price {
        return price_quoted(future, quoted);
    }
    price_fair_value(future, market, as_of)
}

/// Resolve the entry price for an open position, erroring if absent.
///
/// `EquityIndexFuture::entry_price` is `Option<f64>` so that an unfilled
/// order can be represented in the data model. Once you ask the pricer for
/// PV, however, the entry price is mandatory: PV is mark-to-market minus
/// entry, so a missing entry would silently default to zero and book the
/// full quoted price as P&L.
fn require_entry_price(future: &EquityIndexFuture) -> finstack_core::Result<f64> {
    future.entry_price.ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "EquityIndexFuture '{}' has no entry_price; PV requires it. \
             Set `entry_price` to the trade fill, or remove the position from valuation.",
            future.id.as_str()
        ))
    })
}

fn entry_contracts(future: &EquityIndexFuture, entry_price: f64) -> f64 {
    future.num_contracts(entry_price.max(1e-12))
}

pub(crate) fn price_quoted(
    future: &EquityIndexFuture,
    quoted_price: f64,
) -> finstack_core::Result<f64> {
    let entry = require_entry_price(future)?;
    let price_diff = quoted_price - entry;
    let contracts = entry_contracts(future, entry);
    let pv = price_diff * future.contract_specs.multiplier * contracts * future.position_sign();
    Ok(pv)
}

pub(crate) fn price_fair_value(
    future: &EquityIndexFuture,
    market: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<f64> {
    let fair_value = fair_forward(future, market, as_of)?;
    let entry = require_entry_price(future)?;
    let price_diff = fair_value - entry;
    let contracts = entry_contracts(future, entry);
    let pv = price_diff * future.contract_specs.multiplier * contracts * future.position_sign();
    Ok(pv)
}

pub(crate) fn resolve_dividend_yield(
    future: &EquityIndexFuture,
    context: &MarketContext,
) -> finstack_core::Result<f64> {
    use finstack_core::market_data::scalars::MarketScalar;

    if let Some(ref div_id) = future.div_yield_id {
        let ms = context.get_price(div_id.as_str()).map_err(|e| {
            finstack_core::Error::Validation(format!(
                "Dividend yield lookup failed for '{}': {}. If dividend yield is not needed, set div_yield_id to None.",
                div_id, e
            ))
        })?;
        match ms {
            MarketScalar::Unitless(v) => Ok(*v),
            MarketScalar::Price(m) => Err(finstack_core::Error::Validation(format!(
                "Dividend yield '{}' should be a unitless scalar, got Price({})",
                div_id,
                m.currency()
            ))),
        }
    } else {
        Ok(0.0)
    }
}

pub(crate) fn fair_forward(
    future: &EquityIndexFuture,
    context: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<f64> {
    use finstack_core::dates::{DayCount, DayCountContext};
    use finstack_core::market_data::scalars::MarketScalar;

    let spot = match context.get_price(&future.spot_id)? {
        MarketScalar::Unitless(v) => *v,
        MarketScalar::Price(m) => m.amount(),
    };
    let disc = context.get_discount(&future.discount_curve_id)?;
    let t = DayCount::Act365F
        .year_fraction(as_of, future.expiry, DayCountContext::default())?
        .max(0.0);
    let r = disc.zero(t);

    if !future.discrete_dividends.is_empty() {
        let day_count = DayCount::Act365F;
        let mut future_divs = Vec::new();
        for (div_date, amount) in &future.discrete_dividends {
            if *div_date <= as_of
                || *div_date > future.expiry
                || !amount.is_finite()
                || *amount <= 0.0
            {
                continue;
            }
            let t_div = day_count
                .year_fraction(as_of, *div_date, DayCountContext::default())?
                .max(0.0);
            future_divs.push((t_div, *amount));
        }
        let spot_adj =
            crate::instruments::equity::equity_option::pricer::adjust_spot_for_discrete_dividends(
                spot,
                r,
                &future_divs,
            );
        return Ok(spot_adj * (r * t).exp());
    }

    let q = resolve_dividend_yield(future, context)?;
    Ok(spot * ((r - q) * t).exp())
}

/// Equity index future discounting pricer.
///
/// Prices equity index futures using:
/// 1. Mark-to-market (if quoted price available)
/// 2. Cost-of-carry fair value model (otherwise)
pub struct EquityIndexFutureDiscountingPricer;

impl EquityIndexFutureDiscountingPricer {
    /// Create a new equity index future discounting pricer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for EquityIndexFutureDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for EquityIndexFutureDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::EquityIndexFuture, ModelKey::Discounting)
    }

    #[tracing::instrument(
        name = "equity_index_future.discounting.price_dyn",
        level = "debug",
        skip(self, instrument, market),
        fields(inst_id = %instrument.id(), as_of = %as_of),
        err,
    )]
    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        // Type-safe downcasting
        let future = instrument
            .as_any()
            .downcast_ref::<EquityIndexFuture>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::EquityIndexFuture, instrument.key())
            })?;

        let pv = compute_pv(future, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(
                e.to_string(),
                PricingErrorContext::from_instrument(future).model(ModelKey::Discounting),
            )
        })?;

        // Return stamped result
        Ok(ValuationResult::stamped(future.id(), as_of, pv))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::rates::ir_future::Position;
    use crate::pricer::Pricer;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn create_test_market() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        // Create flat 5% discount curve
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 0.9512), (2.0, 0.9048)]) // ~5% rate
            .build()
            .expect("should succeed");

        // Create market context with spot price
        MarketContext::new()
            .insert(discount_curve)
            .insert_price("SPX-SPOT", MarketScalar::Unitless(4500.0))
    }

    fn create_test_future_with_quoted_price() -> EquityIndexFuture {
        use crate::instruments::common_impl::traits::Attributes;
        use crate::instruments::equity::equity_index_future::EquityFutureSpecs;

        EquityIndexFuture::builder()
            .id(InstrumentId::new("ES-TEST"))
            .underlying_ticker("SPX".to_string())
            .notional(Money::new(2_250_000.0, Currency::USD))
            .expiry(Date::from_calendar_date(2025, Month::June, 20).expect("valid date"))
            .last_trading_date(Date::from_calendar_date(2025, Month::June, 19).expect("valid date"))
            .entry_price_opt(Some(4500.0))
            .quoted_price_opt(Some(4550.0))
            .position(Position::Long)
            .contract_specs(EquityFutureSpecs::sp500_emini())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".into())
            .attributes(Attributes::new())
            .build()
            .expect("should build")
    }

    fn create_test_future_without_quoted_price() -> EquityIndexFuture {
        use crate::instruments::common_impl::traits::Attributes;
        use crate::instruments::equity::equity_index_future::EquityFutureSpecs;

        EquityIndexFuture::builder()
            .id(InstrumentId::new("ES-FAIR"))
            .underlying_ticker("SPX".to_string())
            .notional(Money::new(2_250_000.0, Currency::USD))
            .expiry(Date::from_calendar_date(2025, Month::June, 20).expect("valid date"))
            .last_trading_date(Date::from_calendar_date(2025, Month::June, 19).expect("valid date"))
            .entry_price_opt(Some(4500.0))
            .position(Position::Long)
            .contract_specs(EquityFutureSpecs::sp500_emini())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".into())
            .attributes(Attributes::new())
            .build()
            .expect("should build")
    }

    #[test]
    fn test_pricer_key() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let key = pricer.key();

        assert_eq!(key.instrument, InstrumentType::EquityIndexFuture);
        assert_eq!(key.model, ModelKey::Discounting);
    }

    #[test]
    fn test_quoted_price_long_profit() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let future = create_test_future_with_quoted_price();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let result = pricer.price_dyn(&future, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("should succeed");
        assert_eq!(valuation.instrument_id, "ES-TEST");

        // Long 10 contracts, entry 4500, quoted 4550
        // PV = (4550 - 4500) × 50 × 10 × 1 = 50 × 50 × 10 = 25,000
        assert!((valuation.value.amount() - 25_000.0).abs() < 0.01);
    }

    #[test]
    fn test_quoted_price_short_loss() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let mut future = create_test_future_with_quoted_price();
        future.position = Position::Short;
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let result = pricer.price_dyn(&future, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("should succeed");

        // Short 10 contracts, entry 4500, quoted 4550
        // PV = (4550 - 4500) × 50 × 10 × (-1) = -25,000
        assert!((valuation.value.amount() + 25_000.0).abs() < 0.01);
    }

    #[test]
    fn test_fair_value_pricing() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let future = create_test_future_without_quoted_price();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let result = pricer.price_dyn(&future, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("should succeed");
        assert_eq!(valuation.instrument_id, "ES-FAIR");

        // Fair value should be positive (spot at 4500, entry at 4500, with positive carry)
        // F = 4500 × exp(0.05 × 0.47) ≈ 4607 (approximately)
        // PV = (4607 - 4500) × 50 × 10 ≈ 53,500
        assert!(valuation.value.amount() > 0.0);
    }

    #[test]
    fn test_discrete_dividends_reduce_fair_value() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let future_no_divs = create_test_future_without_quoted_price();
        let mut future_with_divs = create_test_future_without_quoted_price();
        future_with_divs.discrete_dividends = vec![
            (
                Date::from_calendar_date(2025, Month::March, 15).expect("valid date"),
                20.0,
            ),
            (
                Date::from_calendar_date(2025, Month::May, 15).expect("valid date"),
                20.0,
            ),
        ];

        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let pv_no_divs = pricer
            .price_dyn(&future_no_divs, &market, as_of)
            .expect("pricing without discrete dividends")
            .value
            .amount();
        let pv_with_divs = pricer
            .price_dyn(&future_with_divs, &market, as_of)
            .expect("pricing with discrete dividends")
            .value
            .amount();

        assert!(
            pv_with_divs < pv_no_divs,
            "Discrete dividends should reduce fair forward and PV"
        );
    }

    #[test]
    fn test_expired_future_zero_value() {
        let pricer = EquityIndexFutureDiscountingPricer::new();
        let future = create_test_future_with_quoted_price();
        let market = create_test_market();
        // Valuation date after expiry
        let as_of = Date::from_calendar_date(2025, Month::July, 1).expect("valid date");

        let result = pricer.price_dyn(&future, &market, as_of);
        assert!(result.is_ok());

        let valuation = result.expect("should succeed");
        assert_eq!(valuation.value.amount(), 0.0);
    }

    #[test]
    fn test_compute_pv_matches_instrument_value() {
        let future = create_test_future_without_quoted_price();
        let market = create_test_market();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let via_pricer = compute_pv(&future, &market, as_of).expect("pricer pv");
        let via_instrument = future.value(&market, as_of).expect("instrument pv");

        assert_eq!(via_pricer, via_instrument);
    }

    /// Regression: previously, a missing `entry_price` silently defaulted to
    /// 0.0, booking the full quoted price as P&L. The pricer must now reject.
    #[test]
    fn pricer_errors_when_entry_price_missing_with_quoted_price() {
        use crate::instruments::common_impl::traits::Attributes;
        use crate::instruments::equity::equity_index_future::EquityFutureSpecs;

        let future = EquityIndexFuture::builder()
            .id(InstrumentId::new("ES-NO-ENTRY"))
            .underlying_ticker("SPX".to_string())
            .notional(Money::new(2_250_000.0, Currency::USD))
            .expiry(Date::from_calendar_date(2025, Month::June, 20).expect("valid date"))
            .last_trading_date(
                Date::from_calendar_date(2025, Month::June, 19).expect("valid date"),
            )
            .entry_price_opt(None)
            .quoted_price_opt(Some(4550.0))
            .position(Position::Long)
            .contract_specs(EquityFutureSpecs::sp500_emini())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".into())
            .attributes(Attributes::new())
            .build()
            .expect("future should build (entry_price is optional in the data model)");

        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let err = compute_pv(&future, &create_test_market(), as_of)
            .expect_err("PV with missing entry_price must fail");
        assert!(
            err.to_string().contains("no entry_price"),
            "error message should explain entry_price requirement: {}",
            err
        );
    }

    /// Same regression — fair-value path (no quoted_price) must also reject.
    #[test]
    fn pricer_errors_when_entry_price_missing_with_fair_value() {
        use crate::instruments::common_impl::traits::Attributes;
        use crate::instruments::equity::equity_index_future::EquityFutureSpecs;

        let future = EquityIndexFuture::builder()
            .id(InstrumentId::new("ES-NO-ENTRY-FAIR"))
            .underlying_ticker("SPX".to_string())
            .notional(Money::new(2_250_000.0, Currency::USD))
            .expiry(Date::from_calendar_date(2025, Month::June, 20).expect("valid date"))
            .last_trading_date(
                Date::from_calendar_date(2025, Month::June, 19).expect("valid date"),
            )
            .entry_price_opt(None)
            .quoted_price_opt(None)
            .position(Position::Long)
            .contract_specs(EquityFutureSpecs::sp500_emini())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".into())
            .attributes(Attributes::new())
            .build()
            .expect("future should build");

        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let err = compute_pv(&future, &create_test_market(), as_of)
            .expect_err("fair-value PV without entry must fail");
        assert!(err.to_string().contains("no entry_price"));
    }
}
