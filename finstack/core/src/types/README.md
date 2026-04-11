# Types Module

`finstack_core::types` is the home for the crate's small, reusable scalar/domain types.
It currently exposes three groups of public API:

- Phantom-typed identifiers from `id.rs`
- Rate/unit wrappers from `rates.rs`
- Credit-rating helpers from `ratings.rs`

The public surface is intentionally narrow. This module does **not** re-export
`Currency`, `Date`, `OffsetDateTime`, `PrimitiveDateTime`, `Timestamp`, or
`HashMap`. Import those from their owning modules instead.

## Public Exports

From `types/mod.rs`, the current public exports are:

- IDs: `Id`, `TypeTag`, `CalendarId`, `CurveId`, `DealId`, `IndexId`, `InstrumentId`, `PoolId`, `PriceId`, `UnderlyingId`
- Rates: `Rate`, `Bps`, `Percentage`
- Ratings: `CreditRating`, `RatingLabel`, `RatingFactorTable`, `moodys_warf_factor`

## Module Notes

### `id.rs`

Provides strongly typed string identifiers backed by `Arc<str>` and phantom
markers so curve, instrument, index, and other IDs cannot be mixed
accidentally at compile time.

### `rates.rs`

Provides lightweight wrappers for decimal rates, basis points, and percentages.
Use these for quoting, configuration, and rate/unit conversion rather than raw
`f64` where units matter.

### `ratings.rs`

Provides normalized rating enums, notch handling, display labels, parsing, and
factor-table helpers such as Moody's WARF mappings.

## Usage Examples

### Typed Identifiers

```rust
use finstack_core::types::{CurveId, InstrumentId};

let curve_id = CurveId::from("USD-OIS");
let instrument_id = InstrumentId::from("US912828XG60");

assert_eq!(curve_id.as_str(), "USD-OIS");
assert_eq!(instrument_id.as_str(), "US912828XG60");
```

### Rates and Percentages

```rust
use finstack_core::types::{Bps, Percentage, Rate};

let rate = Rate::from_percent(5.0);
let spread = Bps::new(25);
let pct = Percentage::new(12.5);

assert_eq!(rate.as_decimal(), 0.05);
assert_eq!(spread.as_decimal(), 0.0025);
assert_eq!(pct.as_decimal(), 0.125);
```

### Credit Ratings

```rust
use finstack_core::types::{CreditRating, RatingLabel, moodys_warf_factor};

let rating: CreditRating = "Baa3".parse().expect("valid rating");
assert_eq!(rating, CreditRating::BBBMinus);

let label = RatingLabel::moodys(CreditRating::BBBMinus);
assert_eq!(label.as_str(), "Baa3");

let factor = moodys_warf_factor(CreditRating::B).unwrap();
assert_eq!(factor, 2720.0);
```

## Extension Guidance

- Add new identifier aliases only when they are broadly useful across the platform.
- Keep rate helpers deterministic and unit-explicit.
- Keep rating parsing conservative and preserve stable external representations.
- Treat new names in `types/mod.rs` as long-lived public API.
