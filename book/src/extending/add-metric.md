# Add a Metric

This guide covers defining a new risk metric, its key convention, and
aggregation behavior.

## Step 1: Define the Metric Key

Metric keys follow the format: `metric_type::qualifier[::sub_qualifier]`

```text
my_metric                         # scalar metric
my_metric::CURVE_ID               # curve-level metric
my_metric::CURVE_ID::TENOR        # bucketed metric
```

## Step 2: Add Computation to the Pricer

Add the metric calculation in the pricer's `price()` method:

```rust,no_run
impl Pricer for MyPricer {
    fn price(
        &self,
        instrument: &dyn Instrument,
        ctx: &PricerContext,
    ) -> Result<ValuationResult> {
        let mut result = ValuationResult::new(npv);

        if ctx.metrics.contains("my_metric") {
            // Option 1: Scalar metric
            let value = compute_my_metric(instrument, ctx)?;
            result.set("my_metric", value);

            // Option 2: Bucketed metric
            for (tenor, value) in compute_bucketed(instrument, ctx)? {
                result.set(
                    &format!("my_metric::{}::{}", curve_id, tenor),
                    value,
                );
            }
        }

        Ok(result)
    }
}
```

## Step 3: Define Aggregation

Metrics aggregate differently across positions:

| Aggregation | Example | Behavior |
|-------------|---------|----------|
| Sum | DV01, CS01, PV01 | Add across positions |
| Weighted avg | YTM, duration | Notional-weighted |
| Max | Max drawdown | Take maximum |
| None | Z-spread | Position-level only |

Register the aggregation rule:

```rust,no_run
registry.set_aggregation("my_metric", AggregationRule::Sum);
```

## Step 4: Add to MetricId Constants

Add a constant for discoverability:

```rust,no_run
pub const MY_METRIC: &str = "my_metric";
```

## Naming Conventions

- Use `snake_case` for metric names
- Use `::` as separator (never `/` or `.`)
- Curve IDs are verbatim (e.g., `USD-OIS`, `ACME-HZD`)
- Tenors use standard abbreviations: `6m`, `1y`, `2y`, `5y`, `10y`, `30y`
- Instrument IDs for z-spread metrics: `cs01::BOND_A`
