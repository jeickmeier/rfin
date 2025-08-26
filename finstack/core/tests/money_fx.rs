use finstack_core::dates::Date;
use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider, FxRate};
use finstack_core::{Currency, Money};

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
        #[cfg(feature = "decimal128")]
        {
            Ok(rust_decimal::Decimal::from_f64_retain(self.rate).unwrap())
        }
        #[cfg(not(feature = "decimal128"))]
        {
            Ok(self.rate)
        }
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
    // provider: from USD->EUR 0.9, USD->GBP 0.75, GBP->EUR 1.2, so USD->GBP*GBP->EUR = 0.9 ≈ USD->EUR
    struct Prov;
    impl FxProvider for Prov {
        fn rate(
            &self,
            from: Currency,
            to: Currency,
            _on: Date,
            _policy: FxConversionPolicy,
        ) -> finstack_core::Result<FxRate> {
            #[cfg(feature = "decimal128")]
            {
                use rust_decimal::Decimal as D;
                let r = match (from, to) {
                    (Currency::USD, Currency::EUR) => D::from_f64_retain(0.9).unwrap(),
                    (Currency::USD, Currency::GBP) => D::from_f64_retain(0.75).unwrap(),
                    (Currency::GBP, Currency::EUR) => D::from_f64_retain(1.2).unwrap(),
                    _ => D::from_f64_retain(1.0).unwrap(),
                };
                Ok(r)
            }
            #[cfg(not(feature = "decimal128"))]
            {
                let r = match (from, to) {
                    (Currency::USD, Currency::EUR) => 0.9,
                    (Currency::USD, Currency::GBP) => 0.75,
                    (Currency::GBP, Currency::EUR) => 1.2,
                    _ => 1.0,
                };
                return Ok(r);
            }
        }
    }
    let m = FxMatrix::new(Prov);
    let d = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
    let (_rate, _closure_result) = m
        .rate_with_closure(
            Currency::USD,
            Currency::EUR,
            d,
            FxConversionPolicy::CashflowDate,
            Currency::GBP,
        )
        .unwrap();
}
