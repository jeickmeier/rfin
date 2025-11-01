# Core Concepts

Understanding these core concepts will help you work effectively with Finstack.

## Currency Safety

Finstack enforces currency safety at compile time through Rust's type system:

### The `Amount` Type

```rust
pub struct Amount {
    value: Decimal,
    currency: Currency,
}
```

**Key Properties:**
- Arithmetic operations only work within the same currency
- Cross-currency math requires explicit FX conversion
- All conversions are tracked in result metadata

### Example: Safe Currency Operations

```rust
// ✅ Same currency - works
let usd1 = Amount::from_str("100.00 USD")?;
let usd2 = Amount::from_str("50.00 USD")?;
let total = usd1 + usd2;  // OK: both USD

// ❌ Different currencies - compile error
let eur = Amount::from_str("100.00 EUR")?;
let bad = usd1 + eur;  // ERROR: cannot add USD and EUR
```

### FX Conversion

To convert currencies, use an `FxProvider`:

```rust
let fx = FxMatrix::new(vec![
    FxRate::new("USDEUR", Date::today(), Rate::from_decimal(0.85)),
]);

let usd = Amount::from_str("100.00 USD")?;
let eur = fx.convert(usd, Currency::EUR, Date::today())?;
```

## Determinism

Finstack guarantees deterministic results through:

### Decimal Arithmetic

All financial calculations use `rust_decimal::Decimal` (never `f64`):

```rust
// ✅ Deterministic
let a = Decimal::from_str("0.1")?;
let b = Decimal::from_str("0.2")?;
assert_eq!(a + b, Decimal::from_str("0.3")?);

// ❌ Not deterministic (floating point)
let x = 0.1f64;
let y = 0.2f64;
assert_ne!(x + y, 0.3f64);  // Floating point error!
```

### Stable Ordering

Operations that depend on ordering (e.g., iteration over HashMap keys) use deterministic, sorted data structures.

### Parallel ≡ Serial

When using parallel features, results are identical to serial execution:

```rust
let serial_result = compute_serial();
let parallel_result = compute_parallel();
assert_eq!(serial_result, parallel_result);  // Always true
```

## Rounding & Scale

Finstack uses a global rounding policy:

```rust
use finstack::config::RoundingContext;

let ctx = RoundingContext::new()
    .with_mode(RoundingMode::HalfEven)
    .with_scale(2);

// Apply globally
RoundingContext::set_global(ctx);
```

All results include the active rounding policy in metadata.

## Time Representation

### Dates

Finstack uses `chrono::NaiveDate` for calendar dates:

```rust
let date = Date::from_ymd(2024, 1, 15);
```

### Periods

Financial periods have start and end dates:

```rust
let period = Period::new(
    Date::from_ymd(2024, 1, 1),
    Date::from_ymd(2024, 12, 31),
);
```

### Calendars

Holiday calendars define business days:

```rust
let cal = Calendar::new("US", vec![
    Date::from_ymd(2024, 7, 4),  // Independence Day
    Date::from_ymd(2024, 12, 25), // Christmas
])?;

let is_business_day = cal.is_business_day(date);
```

### Day Count Conventions

Standard conventions for calculating time fractions:

```rust
use finstack::dates::DayCount;

let start = Date::from_ymd(2024, 1, 1);
let end = Date::from_ymd(2024, 6, 30);

let act360 = DayCount::Actual360.day_fraction(start, end);
let thirty360 = DayCount::Thirty360.day_fraction(start, end);
```

## Market Data

Market data is organized in a `MarketContext`:

```rust
let mut ctx = MarketContext::new(valuation_date);

// Add discount curves
ctx.add_discount_curve(discount_curve);

// Add FX rates
ctx.add_fx_provider(fx_matrix);

// Add volatility surfaces
ctx.add_vol_surface(vol_surface);
```

## Instruments & Pricers

Instruments are defined by specifications, priced by dedicated pricers:

```rust
// 1. Define instrument
let bond_spec = BondSpec { /* ... */ };

// 2. Create pricer
let pricer = BondPricer::new(bond_spec);

// 3. Price against market
let result = pricer.price(&market_ctx)?;

// 4. Extract metrics
let pv = result.present_value();
let dv01 = result.metrics().get("dv01")?;
```

## Error Handling

Finstack uses structured errors via the `Error` enum:

```rust
pub enum Error {
    InvalidInput(String),
    MissingData(String),
    ComputationError(String),
    CurrencyMismatch { expected: Currency, actual: Currency },
    // ... more variants
}
```

All functions that can fail return `Result<T, Error>`:

```rust
fn price_instrument(spec: &InstrumentSpec) -> Result<ValuationResult> {
    // ... computation
}
```

## Result Metadata

All computation results include metadata:

```rust
pub struct ValuationResult {
    present_value: Amount,
    metrics: MetricsRegistry,
    metadata: ResultMetadata,  // ← Includes rounding, FX policies, etc.
}
```

This ensures full traceability of:
- Rounding policies applied
- FX conversions performed
- Parallel vs serial execution
- Data sources used

## Next Steps

- Learn about [Currency & Money](../core/currency-money.md) in detail
- Understand [Market Data](../core/market-data.md) structures
- Explore [Dates & Time](../core/dates-time.md) handling
