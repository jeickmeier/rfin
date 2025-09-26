use criterion::{black_box, criterion_group, criterion_main, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider, FxRate};
use hashbrown::HashMap;
use std::sync::Arc;

struct BenchFxProvider {
    rates: HashMap<(Currency, Currency), f64>,
}

impl BenchFxProvider {
    fn new() -> Self {
        let mut rates = HashMap::new();
        rates.insert((Currency::USD, Currency::EUR), 0.9);
        rates.insert((Currency::EUR, Currency::USD), 1.111111);
        rates.insert((Currency::USD, Currency::GBP), 0.8);
        rates.insert((Currency::GBP, Currency::USD), 1.25);
        Self { rates }
    }
}

impl FxProvider for BenchFxProvider {
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        _on: Date,
        _p: FxConversionPolicy,
    ) -> finstack_core::Result<FxRate> {
        let r = *self.rates.get(&(from, to)).unwrap_or(&1.0);
        Ok(r)
    }
}

fn bench_fx_cache_hits(c: &mut Criterion) {
    use time::Month;
    let provider = BenchFxProvider::new();
    let matrix = FxMatrix::new(Arc::new(provider));
    let d = Date::from_calendar_date(2024, Month::January, 1).unwrap();
    // Warm cache
    let _ = matrix
        .rate(finstack_core::money::fx::FxQuery::new(
            Currency::USD,
            Currency::EUR,
            d,
        ))
        .unwrap();
    c.bench_function("fx_cache_hit_usd_eur", |b| {
        b.iter(|| {
            let r = matrix
                .rate(finstack_core::money::fx::FxQuery::new(
                    Currency::USD,
                    Currency::EUR,
                    d,
                ))
                .unwrap()
                .rate;
            black_box(r)
        })
    });
}

criterion_group!(benches, bench_fx_cache_hits);
criterion_main!(benches);
