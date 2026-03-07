//! FX01 calculator for FX Forwards.
//!
//! Computes sensitivity to a 1bp absolute bump in the spot FX rate.
//!
//! # Formula
//!
//! For an FX forward with contract rate K:
//! ```text
//! FX01 = dPV/dS × 0.0001
//!      = notional × DF_foreign / DF_domestic × DF_domestic × 0.0001
//!      = notional × DF_foreign × 0.0001
//! ```
//!
//! For at-market forwards (where contract_rate is None), the contract rate
//! is implicitly the current market forward rate. When bumping spot, we must
//! keep the contract rate fixed at the *original* market forward, not the
//! bumped market forward.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::fx_forward::FxForward;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;

/// FX01 calculator for FX Forwards.
///
/// Computes the change in present value for a 1 basis point (0.01%) absolute
/// increase in the spot FX rate. This is equivalent to bumping spot by 0.0001
/// in rate terms.
///
/// # At-Market Forwards
///
/// When `contract_rate` is `None`, the forward is valued at-market. For
/// sensitivity calculation, the contract rate is treated as the *current*
/// market forward rate (before bump), ensuring the sensitivity correctly
/// reflects FX exposure.
pub struct Fx01Calculator;

impl MetricCalculator for Fx01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fwd: &FxForward = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let base_pv = fwd.value(&curves, as_of)?;

        let domestic_disc = curves.get_discount(fwd.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount(fwd.foreign_discount_curve_id.as_str())?;

        let df_domestic = domestic_disc.df_between_dates(as_of, fwd.maturity)?;
        let df_foreign = foreign_disc.df_between_dates(as_of, fwd.maturity)?;

        let spot = if let Some(rate) = fwd.spot_rate_override {
            rate
        } else if let Some(fx) = curves.fx() {
            (**fx)
                .rate(FxQuery::new(fwd.base_currency, fwd.quote_currency, as_of))?
                .rate
        } else {
            return Err(finstack_core::Error::from(
                finstack_core::InputError::NotFound {
                    id: "fx_matrix".to_string(),
                },
            ));
        };

        // Compute original market forward rate (before bump)
        let original_market_forward = spot * df_foreign / df_domestic;

        // For at-market forwards, contract rate is the original market forward.
        // This is crucial: we must NOT use the bumped market forward as the
        // contract rate, otherwise Fx01 would incorrectly be zero for at-market forwards.
        let contract_fwd = fwd.contract_rate.unwrap_or(original_market_forward);

        // Apply 1bp absolute bump to spot
        let bump = 0.0001;
        let bumped_spot = spot + bump;

        // Compute bumped market forward rate
        let bumped_market_forward = bumped_spot * df_foreign / df_domestic;

        // PV with bumped spot (contract rate remains fixed)
        let n_base = fwd.notional.amount();
        let bumped_pv = n_base * (bumped_market_forward - contract_fwd) * df_domestic;

        Ok(bumped_pv - base_pv.amount())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::instruments::Attributes;
    use crate::metrics::MetricContext;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::fx::{FxMatrix, SimpleFxProvider};
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, InstrumentId};
    use std::sync::Arc;
    use time::Month;

    fn create_test_market(as_of: Date) -> MarketContext {
        // USD at 5%: DF(0.5) ≈ 0.9753
        let usd_curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (0.5, 0.9753), (1.0, 0.9512)])
            .build()
            .expect("should build");

        // EUR at 3%: DF(0.5) ≈ 0.9851
        let eur_curve = DiscountCurve::builder("EUR-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (0.5, 0.9851), (1.0, 0.9704)])
            .build()
            .expect("should build");

        let fx_provider = Arc::new(SimpleFxProvider::new());
        fx_provider.set_quote(Currency::EUR, Currency::USD, 1.10);
        let fx_matrix = FxMatrix::new(fx_provider);

        MarketContext::new()
            .insert(usd_curve)
            .insert(eur_curve)
            .insert_fx(fx_matrix)
    }

    #[test]
    fn test_fx01_at_market_forward_is_nonzero() {
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid date");
        let maturity = Date::from_calendar_date(2024, Month::July, 15).expect("valid date");
        let market = create_test_market(as_of);
        let notional = 1_000_000.0;

        // At-market forward (no contract rate)
        let forward = FxForward::builder()
            .id(InstrumentId::new("EURUSD-ATM"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .maturity(maturity)
            .notional(Money::new(notional, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        let base_value = forward
            .value(&market, as_of)
            .expect("base value calculation");
        let instrument: Arc<dyn Instrument> = Arc::new(forward.clone());
        let mut context = MetricContext::new(
            instrument,
            Arc::new(market.clone()),
            as_of,
            base_value,
            MetricContext::default_config(),
        );

        let calculator = Fx01Calculator;
        let fx01 = calculator
            .calculate(&mut context)
            .expect("should calculate");

        // FX01 should be approximately notional × DF_foreign × 0.0001
        // DF_foreign at 6 months ≈ 0.9851, so expected ≈ 1_000_000 × 0.9851 × 0.0001 ≈ 98.51
        let eur_curve = market.get_discount("EUR-OIS").unwrap();
        let df_foreign = eur_curve.df_between_dates(as_of, maturity).unwrap();
        let expected_fx01 = notional * df_foreign * 0.0001;

        assert!(
            fx01 > 0.0,
            "FX01 for at-market forward must be positive (long base), got {}",
            fx01
        );
        assert!(
            (fx01 - expected_fx01).abs() < 1.0,
            "FX01 should be approximately {}, got {}",
            expected_fx01,
            fx01
        );
    }

    #[test]
    fn test_fx01_with_contract_rate() {
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid date");
        let maturity = Date::from_calendar_date(2024, Month::July, 15).expect("valid date");
        let market = create_test_market(as_of);
        let notional = 1_000_000.0;

        // Forward with explicit contract rate
        let forward = FxForward::builder()
            .id(InstrumentId::new("EURUSD-CONTRACT"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .maturity(maturity)
            .notional(Money::new(notional, Currency::EUR))
            .contract_rate_opt(Some(1.05))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        let base_value = forward
            .value(&market, as_of)
            .expect("base value calculation");
        let instrument: Arc<dyn Instrument> = Arc::new(forward.clone());
        let mut context = MetricContext::new(
            instrument,
            Arc::new(market.clone()),
            as_of,
            base_value,
            MetricContext::default_config(),
        );

        let calculator = Fx01Calculator;
        let fx01 = calculator
            .calculate(&mut context)
            .expect("should calculate");

        // FX01 should be the same regardless of contract rate (it's the spot sensitivity)
        let eur_curve = market.get_discount("EUR-OIS").unwrap();
        let df_foreign = eur_curve.df_between_dates(as_of, maturity).unwrap();
        let expected_fx01 = notional * df_foreign * 0.0001;

        assert!(
            (fx01 - expected_fx01).abs() < 1.0,
            "FX01 should be approximately {}, got {}",
            expected_fx01,
            fx01
        );
    }
}
