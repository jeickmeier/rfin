## Types Module (core)

The `types` module in `finstack-core` centralizes **small, strongly‑typed building blocks** that are reused across the rest of the workspace. It focuses on:

- **Phantom‑typed identifiers** (`CurveId`, `InstrumentId`, …) to prevent ID mixups
- **Rate and percentage wrappers** (`Rate`, `Bps`, `Percentage`) with clear conversions
- **Credit ratings and factor tables** (`CreditRating`, WARF factors for CLO analysis)
- **Convenient re‑exports** of core primitives (`Currency`, `Date`, `OffsetDateTime`, `Timestamp`)

These types are deliberately lightweight and deterministic. They capture domain meaning at the type level while preserving stable serde shapes and predictable performance across Rust, Python, and WASM bindings.

---

## Module Structure

- **`mod.rs`**
  - Public entrypoint for the `types` module.
  - Re‑exports:
    - Phantom‑typed identifiers: `Id`, `CurveId`, `InstrumentId`, `IndexId`, `PriceId`, `UnderlyingId`, `TypeTag`
    - Rate wrappers: `Rate`, `Bps`, `Percentage`
    - Credit rating utilities: `CreditRating`, `RatingFactorTable`, `moodys_warf_factor`
    - Core primitives from other modules:
      - `Currency` (from `currency`)
      - `Date`, `OffsetDateTime`, `PrimitiveDateTime` (from `dates`)
    - Type aliases:
      - `Timestamp = OffsetDateTime`
- **`id.rs`**
  - Phantom‑typed identifiers:
    - `trait TypeTag` – marker trait for phantom tags.
    - `struct Id<T: TypeTag>` – string ID wrapped in `Arc<str>` plus phantom `T`.
    - Marker types: `CurveTag`, `InstrumentTag`, `IndexTag`, `PriceTag`, `UnderlyingTag`.
    - Type aliases:
      - `CurveId = Id<CurveTag>`
      - `InstrumentId = Id<InstrumentTag>`
      - `IndexId = Id<IndexTag>`
      - `PriceId = Id<PriceTag>`
      - `UnderlyingId = Id<UnderlyingTag>`
  - Design goals:
    - **Compile‑time safety**: you cannot mix `CurveId` and `InstrumentId` accidentally.
    - **Zero‑cost abstraction**: phantom tag has no runtime representation.
    - **Cheap cloning**: `Arc<str>` makes IDs inexpensive to share across threads and collections.
- **`rates.rs`**
  - Rate and percentage wrappers around primitive numeric types:
    - `Rate(f64)` – internal decimal rate (`0.05` = 5%).
    - `Bps(i32)` – basis points for precise spread quotes (`1 bp = 0.01%`).
    - `Percentage(f64)` – human‑readable percentages (`5.0` = 5%).
  - Features:
    - Safe, explicit conversions:
      - `Rate::from_decimal`, `from_percent`, `from_bps`
      - `Rate::as_decimal`, `as_percent`, `as_bps`
      - Cross‑`From` impls: `Rate ↔ Bps ↔ Percentage`
    - Arithmetic:
      - `Add`, `Sub`, `Mul<f64>`, `Div<f64>`, `Neg` on `Rate` / `Percentage`
      - `Add`, `Sub`, `Mul<i32>`, `Div<i32>`, `Neg` on `Bps`
    - Convenience constants and predicates:
      - `Rate::ZERO`, `Bps::ZERO`, `Percentage::ZERO`
      - `is_zero`, `is_positive`, `is_negative`, `abs`
  - Intended use:
    - **Quoting and configuration** (spreads, hurdle rates, fee rates), not storage of money values.
    - Actual monetary math should flow through `Money` / `Amount` in the `currency` and `money` modules.
- **`ratings.rs`**
  - Credit rating system and WARF factor table:
    - `CreditRating` – agency‑agnostic letter scale (`AAA`..`D`, `NR`).
    - `RatingNotch` – plus/flat/minus subdivision.
    - `NotchedRating` – `(CreditRating, RatingNotch)` pair.
    - `RatingLabel` – stable label (`"BBB-"`, `"Baa3"`) for storage / exports.
    - `RatingFactorTable` – mapping from `NotchedRating` to numeric factors.
    - `moodys_warf_factor` – lazily initialized Moody’s WARF lookup.
  - Capabilities:
    - Parse ratings from strings (`"BBB-"`, `"Baa3"`, `"NR"`, `"Not Rated"`).
    - Investment/speculative grade classification.
    - Moody’s and generic display formats.
    - Moody’s WARF factors for CLO / structured credit analysis.

---

## Core Concepts and Design

### Phantom‑Typed Identifiers (`Id<T>`)

The `Id<T>` type wraps a string identifier with a **phantom type tag**:

- **Compile‑time type safety**:
  - `CurveId` and `InstrumentId` are different Rust types, even though both store strings.
  - Equality, ordering, and hashing are only defined for the _same_ tag type.
- **Runtime representation**:
  - Internally represented as `Arc<str>` plus `PhantomData<T>`.
  - Cloning an ID is O(1) (refcount bump), ideal for large maps and graphs.
- **Serde and wire format**:
  - Under the `serde` feature, `Id<T>` is `serde(transparent)` over the underlying string.
  - All IDs use **stable string representations** for long‑lived pipelines.

Use these types whenever you pass identifiers across module boundaries (curves, instruments, indices, prices, underlyings, etc.). Avoid raw `String` / `&str` for domain IDs in new code.

### Rate Wrappers (`Rate`, `Bps`, `Percentage`)

The rate wrappers exist to prevent confusion between different representations of the same idea:

- `Rate` – **decimal representation** optimized for calculations (`0.05` = 5%).
- `Bps` – **basis points** for fixed‑income spreads and small changes (e.g., “25 bps move”).
- `Percentage` – **human‑readable percentages** for configuration and display.

Each type is small (`Copy` for `Rate` / `Percentage` and `Bps`) and provides:

- Explicit constructors and accessors with clear units.
- Arithmetic operators suited to its role (e.g., adding spreads, scaling rates).
- Conversions to bridge quoting conventions in a controlled way.

These wrappers are meant to sit at the **interface between configuration / quoting and numeric engines**; they pair naturally with date‑based year fractions from `core::dates`.

### Credit Ratings and WARF Factors

The rating types provide a normalized view of agency ratings:

- `CreditRating` expresses coarse letter grades (`AAA`, `AA`, …, `D`, `NR`).
- `RatingNotch` refines a letter grade into `Plus`, `Flat`, or `Minus`.
- `NotchedRating` combines the two and supports both:
  - Generic S&P/Fitch‑style strings (`"BBB-"`).
  - Moody’s‑style strings (`"Baa3"`).
- `RatingFactorTable` stores **methodology‑specific** mappings (e.g., Moody’s WARF).

The convenience function `moodys_warf_factor(rating)` exposes a **lazy, shared WARF table** for the entire process, guaranteeing consistent factors in valuations, scenarios, and portfolio analytics.

---

## Usage Examples

### Typed Identifiers for Curves and Instruments

```rust
use finstack_core::types::{CurveId, InstrumentId, Id};

// Standard markers
let curve_id = CurveId::from("USD-OIS");
let bond_id = InstrumentId::from("US912828XG60");

assert_eq!(curve_id.as_str(), "USD-OIS");
assert_eq!(bond_id.as_str(), "US912828XG60");

// Compile-time safety: cannot mix curve and instrument IDs
// let bad = curve_id == bond_id; // does not compile

// Custom domain-specific IDs
struct Portfolio;
struct Counterparty;

type PortfolioId = Id<Portfolio>;
type CounterpartyId = Id<Counterparty>;

let p = PortfolioId::from("PORT-001");
let c = CounterpartyId::from("CPTY-JP-MORGAN");

assert_eq!(p.as_str(), "PORT-001");
assert_eq!(c.as_str(), "CPTY-JP-MORGAN");
```

IDs are `Hash`, `Eq`, and `Ord`, making them ideal as keys in `HashMap` / `BTreeMap`:

```rust
use finstack_core::types::CurveId;
use hashbrown::HashMap;

let mut curves = HashMap::new();
curves.insert(CurveId::from("USD-OIS"), 0.045);
curves.insert(CurveId::from("EUR-OIS"), 0.035);

let usd_rate = curves.get(&CurveId::from("USD-OIS"));
assert_eq!(usd_rate, Some(&0.045));
```

### Rates, Basis Points, and Percentages

```rust
use finstack_core::types::{Rate, Bps, Percentage};

// Construct using whichever representation is most natural
let r_decimal = Rate::from_decimal(0.05);  // 5%
let r_percent = Rate::from_percent(5.0);   // 5%
let r_bps = Rate::from_bps(500);           // 5%

assert_eq!(r_decimal, r_percent);
assert_eq!(r_percent, r_bps);

// Convert between units
assert_eq!(r_decimal.as_percent(), 5.0);
assert_eq!(r_decimal.as_bps(), 500);

let spread = Bps::new(25); // 0.25%
assert_eq!(spread.as_percent(), 0.25);
assert_eq!(spread.as_decimal(), 0.0025);

let pct = Percentage::new(12.5); // 12.5%
assert_eq!(pct.as_decimal(), 0.125);
assert_eq!(pct.as_bps(), 1250);
```

Rates support basic arithmetic, which is useful for spreads and adjustments:

```rust
use finstack_core::types::{Rate, Bps};

let base = Rate::from_percent(3.0);
let spread = Rate::from_bps(50); // 0.5%
let total = base + spread;

assert!((total.as_percent() - 3.5).abs() < 1e-10);

// Basis points arithmetic
let x = Bps::new(20);
let y = Bps::new(5);
assert_eq!(x + y, Bps::new(25));
assert_eq!(x * 2, Bps::new(40));
```

### Credit Ratings and Moody’s WARF Factors

```rust
use finstack_core::types::ratings::{CreditRating, RatingNotch, NotchedRating, RatingLabel, moodys_warf_factor};

// Investment vs speculative grade
assert!(CreditRating::BBB.is_investment_grade());
assert!(!CreditRating::BB.is_investment_grade());

// Parse from generic and Moody's-style strings
let bb_plus: NotchedRating = "BB+".parse().unwrap();
let baa3: NotchedRating = "Baa3".parse().unwrap();

assert_eq!(bb_plus.base(), CreditRating::BB);
assert_eq!(bb_plus.notch(), RatingNotch::Plus);
assert_eq!(baa3.base(), CreditRating::BBB);
assert_eq!(baa3.notch(), RatingNotch::Minus);

// Display labels
let generic = RatingLabel::generic(bb_plus);
let moodys = RatingLabel::moodys(baa3);

assert_eq!(generic.as_str(), "BB+");
assert_eq!(moodys.as_str(), "Baa3");

// Moody's WARF factor (for CLO analysis)
let factor_b = moodys_warf_factor(CreditRating::B);
assert_eq!(factor_b, 2720.0);
```

### Re‑exported Primitives

The `types` module re‑exports several core primitives so that many modules can simply depend on `finstack_core::types`:

```rust
use finstack_core::types::{Currency, Date, Timestamp, CurveId, Rate};
use time::Month;

let as_of: Date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
let now: Timestamp = as_of.midnight().assume_utc();

let curve_id = CurveId::from("USD-OIS");
let discount_rate = Rate::from_percent(4.0);

// ... construct curves, cashflows, or statements using these primitives ...
```

---

## Adding New Features

The `types` module is **core infrastructure** for identifiers, rates, and ratings. When extending it:

- Keep public APIs **small, deterministic, and focused on type safety**.
- Preserve **serde stability** for existing public types (no renames, no semantic changes).
- Prefer **newtype wrappers and phantom types** over unstructured `String` / `f64`.

### New Identifier Types

To introduce a new domain‑specific identifier:

1. Decide whether the ID is:
   - General to the platform (belongs in `core::types::id` as a new marker + alias), or
   - Local to a crate (define a crate‑local `Id<MyTag>` alias instead).
2. For platform‑wide IDs, in `id.rs`:
   - Add a zero‑sized marker type:
     - `#[derive(Debug, Clone, Copy, Default)]`
     - `pub struct MyNewTag;`
   - Add a type alias:
     - `pub type MyNewId = Id<MyNewTag>;`
   - Add doc comments describing the domain (e.g., “book identifier”, “scenario identifier”).
3. Add tests that:
   - Construct the new ID from strings (`new`, `From<&str>`, `From<String>`).
   - Use it as a map key to ensure `Hash` / `Eq` behave as expected.

Guidelines:

- **Do not** remove or repurpose existing tag types or aliases (backwards compatibility).
- Use new IDs consistently across APIs rather than re‑using overly generic ones.

### Extending Rate Functionality

For new rate‑related helpers:

- Prefer **pure functions or methods** on `Rate` / `Bps` / `Percentage` that:
  - Are deterministic, side‑effect‑free, and panic‑free for valid inputs.
  - Validate inputs where necessary (e.g., denominators, bounds).
- Keep `rates.rs` the **single home** for scalar rate logic; more complex compounding logic should live in `dates::rate_conversions`.
- Add doc comments with examples, and tests that:
  - Cover realistic market values (small and large rates, positive and negative).
  - Round‑trip across conversions where applicable.

Avoid introducing additional floating‑point state beyond what’s strictly necessary; the rest of the stack is designed around deterministic Decimal numerics for `Money` / `Amount`.

### New Rating Factor Tables or Agencies

For additional rating methodologies:

- Implement new constructors on `RatingFactorTable`, e.g.:
  - `pub fn moodys_alternative() -> Self`
  - `pub fn sp_idealized() -> Self`
- Store factors in a `HashMap<NotchedRating, f64>` following the existing pattern.
- Provide:
  - Clear documentation including the **source methodology** and date.
  - Unit tests that validate a handful of key factors against published tables.
- If a table is broadly useful, expose a **convenience function** (like `moodys_warf_factor`) with a lazily initialized `OnceLock`.

When parsing ratings:

- Extend parsing behavior carefully and conservatively to avoid breaking existing inputs.
- Treat invalid strings as `InputError::InvalidRating` and propagate through `crate::Error`.

### Additional Re‑exports or Aliases

New aliases or re‑exports in `mod.rs` should:

- Reflect **widely used primitives** (e.g., `Timestamp`).
- Avoid creating ambiguous or overlapping names that obscure the source module.
- Remain stable once public; treat names as part of the crate’s external contract.

Before adding a new re‑export, prefer importing from the original module inside internal code. Promote it to `types` only when it clearly improves ergonomics for multiple crates.

---

## When to Use This Module vs. Other Crates

- **Use `core::types` when**:
  - Defining identifiers, rates, ratings, and other small scalar types shared across the platform.
  - You want **compile‑time guarantees** against ID and unit mixups.
  - You are building new APIs that sit on top of core primitives (`Money`, `Date`, curves, statements, portfolios).
- **Use higher‑level crates (`valuations`, `statements`, `scenarios`, `portfolio`) when**:
  - Implementing full instrument pricing, risk analytics, statement models, or scenario engines.
  - Working with cashflows, term structures, and models rather than scalar types.

Keeping this separation clean ensures the `core` crate remains **small, deterministic, and reusable**, while higher‑level crates compose these primitives into full financial models.



