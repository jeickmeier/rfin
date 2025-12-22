use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::fx::{
    FxConfig, FxConversionPolicy, FxMatrix, FxProvider, FxQuery, FxRate,
};
use finstack_core::money::Money;
use std::sync::Arc;

struct StaticFx {
    rate: f64,
}

impl FxProvider for StaticFx {
    fn rate(
        &self,
        _from: Currency,
        _to: Currency,
        _on: Date,
        _policy: FxConversionPolicy,
    ) -> finstack_core::Result<FxRate> {
        Ok(self.rate)
    }
}

#[test]
fn explicit_convert_and_add() {
    let usd = Money::new(100.0, Currency::USD);
    let eur = Money::new(90.0, Currency::EUR);
    let prov = StaticFx { rate: 1.2 }; // EUR→USD 1.2 for test
    let d = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();

    // Convert EUR to USD, then add
    let eur_in_usd = eur
        .convert(Currency::USD, d, &prov, FxConversionPolicy::CashflowDate)
        .unwrap();
    let sum = (usd + eur_in_usd).unwrap();
    // Expected: 100 + 90*1.2 = 208
    assert!((sum.amount() - 208.0).abs() < 1e-9);
}

#[test]
fn cross_currency_add_fails_without_convert() {
    let usd = Money::new(10.0, Currency::USD);
    let eur = Money::new(10.0, Currency::EUR);
    assert!((usd + eur).is_err());
}

#[test]
fn closure_check_matrix() {
    // Market standard identity: cross rates must satisfy triangular consistency.
    // We force triangulation via USD pivot:
    // USD->EUR = 0.9, USD->GBP = 0.75, GBP->USD = 1/0.75
    // => GBP->EUR = GBP->USD * USD->EUR = 1.2
    struct Prov;
    impl FxProvider for Prov {
        fn rate(
            &self,
            from: Currency,
            to: Currency,
            _on: Date,
            _policy: FxConversionPolicy,
        ) -> finstack_core::Result<FxRate> {
            match (from, to) {
                (Currency::USD, Currency::EUR) => Ok(0.9),
                (Currency::USD, Currency::GBP) => Ok(0.75),
                (Currency::GBP, Currency::USD) => Ok(1.0 / 0.75),
                _ => Err(finstack_core::error::InputError::NotFound {
                    id: format!("FX:{from}->{to}"),
                }
                .into()),
            }
        }
    }
    let cfg = FxConfig {
        enable_triangulation: true,
        pivot_currency: Currency::USD,
        ..Default::default()
    };
    let m = FxMatrix::with_config(Arc::new(Prov), cfg);
    let d = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();

    // Direct quote (not triangulated)
    let usd_eur = m
        .rate(FxQuery::new(Currency::USD, Currency::EUR, d))
        .unwrap();
    assert!(!usd_eur.triangulated);
    assert!((usd_eur.rate - 0.9).abs() < 1e-15);

    // Triangulated cross
    let gbp_eur = m
        .rate(FxQuery::new(Currency::GBP, Currency::EUR, d))
        .unwrap();
    assert!(gbp_eur.triangulated);
    assert!((gbp_eur.rate - 1.2).abs() < 1e-15);

    // Triangular consistency: USD->GBP * GBP->EUR == USD->EUR
    let usd_gbp = m
        .rate(FxQuery::new(Currency::USD, Currency::GBP, d))
        .unwrap();
    assert!(!usd_gbp.triangulated);
    let lhs = usd_gbp.rate * gbp_eur.rate;
    assert!((lhs - usd_eur.rate).abs() < 1e-12);
}
