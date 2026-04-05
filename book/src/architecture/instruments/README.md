# Instruments

The `finstack-valuations` crate provides a unified instrument model with 71
instrument types. Each instrument type implements a common trait interface, and
the pricer registry dispatches to the appropriate pricing model.

## Instrument Trait

Every instrument implements the `Instrument` trait:

```rust,no_run
pub trait Instrument {
    fn id(&self) -> &str;                           // Unique identifier
    fn key(&self) -> InstrumentType;                // Type classification
    fn value(&self, ...) -> Result<Money>;          // Fast NPV
    fn price_with_metrics(&self, ...) -> Result<ValuationResult>;  // NPV + metrics
    fn cashflow_schedule(&self) -> Schedule;        // Payment dates
    fn dated_cashflows(&self) -> Vec<(Date, Money)>; // Flattened cashflows
}
```

## Pricer Registry

The `standard_registry()` function returns a pre-loaded registry with 40+
pricers covering all instrument types:

```python
from finstack.valuations.pricer import standard_registry

registry = standard_registry()  # Lazy singleton, initialized once

result = registry.price_with_metrics(
    instrument,       # any Instrument
    "discounting",    # model key
    market,           # MarketContext
    as_of,            # valuation date
    metrics=["dv01", "cs01", "ytm"],
)
```

Dispatch is based on `(InstrumentType, ModelKey)` pairs. Registration modules:

| Module | Covers |
|--------|--------|
| `register_rates_pricers` | Bonds, IRS, caps, swaptions |
| `register_credit_pricers` | CDS, CDX, tranches |
| `register_equity_pricers` | Options, forwards, TRS |
| `register_fx_pricers` | FX options, swaps |
| `register_fixed_income_pricers` | Convertibles, MBS, structured credit |
| `register_inflation_pricers` | Inflation derivatives |
| `register_exotic_pricers` | Barriers, Asian, autocallables |
| `register_commodity_pricers` | Commodity derivatives |

## Builder Pattern

All instruments use a fluent builder API:

```python
instrument = Bond.builder("BOND_001") \
    .money(Money(1_000_000, "USD")) \
    .coupon_rate(0.045) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .build()
```

## Asset Classes

- [Fixed Income](fixed_income.md) — Bonds, inflation-linked, term loans, structured credit
- [Rates](rates.md) — IRS, basis swaps, xccy, caps/floors, swaptions
- [Credit](credit.md) — CDS, CDX, CDX options, CDX tranches
- [Equity](equity.md) — Equity options, variance swaps, total return swaps
- [FX](fx.md) — FX forwards, options, barriers, NDFs
- [Commodity](commodity.md) — Commodity swaps, options, Asian, swaptions
- [Exotic](exotic.md) — Autocallables, range accruals, cliquets, lookbacks
