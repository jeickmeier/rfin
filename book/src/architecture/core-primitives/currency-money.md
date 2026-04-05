# Currency & Money

## Currency

The `Currency` enum provides ISO 4217 currency codes with zero-cost abstraction
(2 bytes, stack-allocated). Discriminants match official ISO numeric codes.

**Rust**

```rust,no_run
use finstack_core::currency::Currency;

let usd = Currency::from_str("USD")?;
assert_eq!(usd.numeric(), 840);
assert_eq!(usd.decimals(), 2);

// JPY has 0 decimal places
let jpy = Currency::from_str("JPY")?;
assert_eq!(jpy.decimals(), 0);
```

**Python**

```python
from finstack.core.currency import Currency

usd = Currency("USD")
assert usd.code == "USD"
assert usd.numeric == 840
assert usd.decimals == 2

# List all supported currencies
all_currencies = Currency.all()

# Round-trip via numeric code
ccy = Currency.from_numeric(840)
assert ccy.code == "USD"
```

**TypeScript (WASM)** — *coming soon*

```typescript
// const usd = new Currency("USD");
// usd.code     // "USD"
// usd.numeric  // 840
```

### Key Properties

| Property | Example (USD) | Example (JPY) |
|----------|--------------|---------------|
| `code` | `"USD"` | `"JPY"` |
| `numeric` | `840` | `392` |
| `decimals` | `2` | `0` |

## Money

`Money` is a currency-safe monetary amount. All arithmetic enforces currency
matching at runtime — you cannot accidentally add USD to EUR.

**Rust**

```rust,no_run
use finstack_core::money::Money;
use finstack_core::currency::Currency;

let usd = Currency::from_str("USD")?;

let a = Money::new(100.50, usd);
let b = Money::new(200.00, usd);
let c = a.checked_add(b)?;  // Money(300.50, USD)

// Currency mismatch is an error
let eur = Currency::from_str("EUR")?;
let d = Money::new(50.0, eur);
assert!(a.checked_add(d).is_err());

// Scaling
let doubled = a.checked_mul_f64(2.0)?;  // Money(201.00, USD)
```

**Python**

```python
from finstack.core.money import Money

a = Money(100.50, "USD")
b = Money(200.00, "USD")
c = a + b                  # Money(300.50, USD)

# Currency mismatch raises ValueError
d = Money(50.0, "EUR")
# a + d  → raises ValueError

# Formatting
a.format()                  # "USD 100.50"
a.format_with_separators()  # "USD 100.50"

# Zero money
z = Money.zero("USD")       # Money(0.00, USD)
```

### Rounding

Money uses Bankers rounding (round half to even) by default. Custom rounding
is controlled via `FinstackConfig`:

```python
from finstack.core.config import FinstackConfig, RoundingMode

cfg = FinstackConfig()
cfg.rounding.mode = RoundingMode.BANKERS  # default

# Custom rounding
m = Money.from_config(123.455, "USD", cfg)  # 123.46 (Bankers)
```

## FX Conversion

Cross-currency operations require an explicit FX provider. There is no implicit
currency conversion:

**Python**

```python
from finstack.core.money import SimpleFxProvider, FxMatrix
from finstack.core.currency import Currency
from datetime import date

provider = SimpleFxProvider()
provider.set_quote(Currency("EUR"), Currency("USD"), 1.10)

matrix = FxMatrix(provider)
rate = matrix.rate(Currency("EUR"), Currency("USD"), date(2024, 1, 15))
# rate ≈ 1.10
```

The `FxMatrix` handles reciprocals and triangulation automatically, backed by
an LRU cache for repeated lookups.

## Type-Safe Identifiers

Finstack uses phantom-typed IDs to prevent mixing unrelated identifiers:

```rust,no_run
use finstack_core::types::{InstrumentId, CurveId};

let inst_id = InstrumentId::from("BOND_001");
let curve_id = CurveId::from("USD-OIS");

// These are distinct types — cannot be confused at compile time
```

Available ID types: `InstrumentId`, `CurveId`, `IndexId`, `PriceId`,
`UnderlyingId`, `DealId`, `PoolId`, `CalendarId`.
