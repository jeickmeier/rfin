# Add a Pricer

This guide covers implementing a new pricing model and registering it in the
pricer registry.

## Step 1: Implement the Pricer Trait

```rust,no_run
use finstack_valuations::pricer::{Pricer, PricerContext};

pub struct MyPricer;

impl Pricer for MyPricer {
    fn price(
        &self,
        instrument: &dyn Instrument,
        ctx: &PricerContext,
    ) -> Result<ValuationResult> {
        let my_inst = instrument
            .downcast_ref::<MyDerivative>()
            .ok_or(Error::WrongInstrumentType)?;

        // Get market data from context
        let disc = ctx.market.get_discount(my_inst.disc_id())?;

        // Compute NPV
        let npv = /* pricing logic */;

        let mut result = ValuationResult::new(npv);

        // Add metrics if requested
        if ctx.metrics.contains("dv01") {
            let dv01 = /* bump and reprice */;
            result.set("dv01", dv01);
        }

        Ok(result)
    }
}
```

## Step 2: Register in the Standard Registry

Add registration in the appropriate registration module:

```rust,no_run
pub fn register_my_pricers(registry: &mut PricerRegistry) {
    registry.register(
        InstrumentType::MyDerivative,
        "my_model",             // model key
        Box::new(MyPricer),
    );
}
```

Then call this from `standard_registry()` in
`finstack/valuations/src/pricer/registry.rs`.

## Step 3: Test

```rust,no_run
#[test]
fn test_my_pricer() {
    let registry = standard_registry();
    let instrument = MyDerivative::builder("TEST")
        .notional(Money::new(1_000_000.0, Currency::USD))
        .disc_id("USD-OIS")
        .build()
        .unwrap();

    let result = registry.price_with_metrics(
        &instrument, "my_model", &market, as_of, &["dv01"],
    ).unwrap();

    assert!(result.npv.amount.abs() > 0.0);
}
```

## Model Key Conventions

| Key | Use |
|-----|-----|
| `"discounting"` | Standard discount curve pricing |
| `"black_scholes"` | Black-Scholes for European options |
| `"black76"` | Black-76 for rates options |
| `"bachelier"` | Normal model for caps/floors |
| `"isda_standard"` | ISDA Standard Model for CDS |
| `"monte_carlo"` | Monte Carlo simulation |
