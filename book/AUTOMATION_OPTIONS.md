# Documentation Automation Options

Ways to reduce duplication between rustdoc and mdBook.

## Option 1: Cross-Linking (Easiest - Recommended)

**Don't duplicate - just link!**

In mdBook, link to rustdoc for technical details:

```markdown
# Working with Amounts

The [`Amount`](../api/finstack/core/struct.Amount.html) type provides 
currency-safe arithmetic. See the API docs for all available methods.

## Conceptual Overview

[Your narrative explanation here - not duplicating what's in rustdoc]
```

**Pros:**
- ✅ No duplication
- ✅ Single source of truth for APIs
- ✅ mdBook focuses on teaching, rustdoc on reference

**Cons:**
- Users need to jump between docs (but that's normal!)

---

## Option 2: Include External Files (Good for Examples)

mdBook can include external markdown or code files:

```markdown
# Bond Pricing Example

{{#include ../../examples/bond_pricing.rs}}
```

**Setup:**

1. Keep examples as standalone runnable files in `examples/`
2. Include them in mdBook with `{{#include}}`
3. Also reference them in rustdoc

**Pros:**
- ✅ Examples exist once, used everywhere
- ✅ Examples are testable (`cargo run --example`)
- ✅ Examples are real, runnable code

**Cons:**
- Need to structure examples carefully
- May need to strip some boilerplate for display

**Example:**

```rust
// examples/bond_pricing.rs
use finstack::prelude::*;

fn main() -> Result<()> {
    // {{#include examples/bond_pricing.rs:setup}}
    let bond = BondSpec {
        id: InstrumentId::new("BOND001"),
        // ... more fields
    };
    // {{#include examples/bond_pricing.rs:setup}}
    
    // Price the bond
    let pricer = BondPricer::new(bond);
    let result = pricer.price(&market_ctx)?;
    
    println!("PV: {}", result.present_value());
    Ok(())
}
```

In mdBook:
```markdown
# Bond Pricing

Here's a complete example:

{{#include ../../examples/bond_pricing.rs:setup}}
```

---

## Option 3: Rustdoc JSON + Custom Tool (Advanced)

Rustdoc can export JSON that you could process:

```bash
cargo +nightly rustdoc -- -Z unstable-options --output-format json
```

Then write a script to:
1. Extract doc comments from JSON
2. Generate mdBook pages
3. Run as part of build

**Pros:**
- ✅ Fully automated
- ✅ Always in sync

**Cons:**
- ❌ Complex to set up
- ❌ Requires nightly Rust
- ❌ Generated docs often aren't great for teaching
- ❌ Loss of narrative control

**Verdict:** Usually not worth it

---

## Option 4: mdbook-rustdoc Plugin

There's an experimental plugin: https://github.com/zjp-CN/mdbook-rustdoc

```bash
cargo install mdbook-rustdoc
```

Add to `book.toml`:
```toml
[preprocessor.rustdoc]
```

Then in your markdown:
```markdown
# Amount Type

{{#rustdoc finstack::core::Amount}}
```

**Pros:**
- ✅ Auto-pulls rustdoc content
- ✅ Stays in sync

**Cons:**
- ❌ Experimental/unmaintained
- ❌ Limited formatting control
- ❌ May not work with your structure

---

## Option 5: Shared Example Library

Create a crate with examples that both docs use:

```
finstack/
├── examples/          # Runnable examples
│   ├── basic/
│   │   └── amount_usage.rs
│   └── advanced/
│       └── bond_pricing.rs
```

**In rustdoc:**
```rust
/// # Examples
///
/// See `examples/basic/amount_usage.rs` for a complete example.
```

**In mdBook:**
```markdown
{{#include ../examples/basic/amount_usage.rs}}
```

**Pros:**
- ✅ Examples in one place
- ✅ Runnable and testable
- ✅ Both docs reference same code

**Cons:**
- Need good example organization

---

## Recommended Approach

**Use a combination:**

### 1. Examples Once, Used Everywhere

```
finstack/examples/
├── 01_amounts_and_currency.rs
├── 02_bond_pricing.rs
├── 03_portfolio_valuation.rs
└── README.md
```

Each example is:
- Runnable: `cargo run --example 01_amounts_and_currency`
- Included in mdBook: `{{#include}}`
- Referenced in rustdoc: Link or brief snippet

### 2. API Details ONLY in Rustdoc

Never duplicate signatures, parameters, return types.

### 3. Concepts ONLY in mdBook

Explain "why" and "how things work together".

### 4. Cross-Link Aggressively

In mdBook:
```markdown
See [`BondPricer::price()`](link-to-rustdoc) for API details.
```

In rustdoc:
```rust
/// For a tutorial on pricing bonds, see the
/// [user guide](https://yoursite.com/book/valuations/bonds.html).
```

---

## Practical Example

### examples/bond_pricing.rs
```rust
//! # Bond Pricing Example
//!
//! This example demonstrates how to price a fixed-rate bond.

use finstack::prelude::*;
use finstack::valuations::instruments::bond::*;

fn main() -> Result<()> {
    // Create bond specification
    let bond = BondSpec {
        id: InstrumentId::new("BOND001"),
        issue_date: Date::from_ymd(2020, 1, 1),
        maturity_date: Date::from_ymd(2025, 1, 1),
        coupon_rate: Rate::from_percent(5.0),
        face_value: Amount::from_str("1000.00 USD")?,
        frequency: Frequency::Semiannual,
        day_count: DayCount::Actual360,
        currency: Currency::USD,
    };

    // Set up market data
    let val_date = Date::from_ymd(2024, 1, 1);
    let mut ctx = MarketContext::new(val_date);
    
    let curve = DiscountCurve::flat(
        CurveId::new("USD-GOVT"),
        val_date,
        Rate::from_percent(4.0),
    );
    ctx.add_discount_curve(curve);

    // Price the bond
    let pricer = BondPricer::new(bond);
    let result = pricer.price(&ctx)?;

    println!("Bond Present Value: {}", result.present_value());
    println!("Bond Yield: {}", result.metrics().get("yield")?);
    
    Ok(())
}
```

### In rustdoc (finstack/valuations/src/instruments/bond/mod.rs)
```rust
/// Pricer for fixed-rate bonds.
///
/// # Examples
///
/// For a complete example, see `examples/bond_pricing.rs`:
///
/// ```bash
/// cargo run --example bond_pricing
/// ```
///
/// Or see the [Bond Pricing Guide](https://docs.yoursite.com/book/valuations/bonds.html)
/// in the user guide.
pub struct BondPricer { ... }
```

### In mdBook (book/src/valuations/bonds.md)
```markdown
# Pricing Bonds

This guide shows how to price a fixed-rate bond using the 
[`BondPricer`](../../api/finstack/valuations/bond/struct.BondPricer.html).

## Complete Example

Here's a full working example:

{{#include ../../../examples/bond_pricing.rs}}

You can run this example yourself:

\`\`\`bash
cargo run --example bond_pricing
\`\`\`

## Step-by-Step Explanation

Let's break down what's happening...

[Your narrative explanation here]
```

---

## What This Achieves

✅ **Examples written ONCE** in `examples/`  
✅ **API details written ONCE** in rustdoc  
✅ **Concepts written ONCE** in mdBook  
✅ **Everything cross-linked** for navigation  
✅ **No duplication!**

---

## Quick Setup

1. **Move examples to standalone files:**
   ```bash
   # Create examples with descriptive names
   touch examples/01_currency_basics.rs
   touch examples/02_bond_pricing.rs
   ```

2. **Enable includes in mdBook:**
   Already enabled by default!

3. **Include in markdown:**
   ```markdown
   {{#include ../../examples/bond_pricing.rs}}
   ```

4. **Cross-link everywhere:**
   - mdBook → rustdoc: Link to API details
   - rustdoc → mdBook: Link to tutorials
   - Both → examples: Reference or include

---

## Bottom Line

**You don't write twice if you:**
1. Put examples in `examples/` and include them
2. Put API docs in rustdoc comments
3. Put concepts/tutorials in mdBook
4. Cross-link between all three

The key insight: **They're not duplicates, they're different content types that complement each other.**

