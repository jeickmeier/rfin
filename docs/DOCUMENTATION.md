# Finstack Documentation Guide

Finstack has **two complementary documentation systems**, each serving a different purpose.

---

## 📚 API Reference (rustdoc)

**Auto-generated from your Rust code docstrings**

### What It Is
- Technical reference for every public type, function, module, and trait
- Generated from `///` and `//!` comments in your Rust source code
- Includes all type signatures, trait implementations, and examples

### How to Use It

```bash
# Generate and open API documentation
make doc
```

This opens `target/doc/finstack/index.html` in your browser.

### What It Contains
- All public APIs across all crates (core, statements, valuations, etc.)
- Type signatures and method documentation
- Trait implementations
- Links between related types
- Runnable code examples from docstrings

### When to Use It
✅ "What methods does `Amount` have?"  
✅ "What parameters does `BondPricer::price()` take?"  
✅ "What traits does `MarketContext` implement?"  
✅ "How do I call this specific function?"  

❌ "How do I get started with Finstack?"  
❌ "What's the best way to price a portfolio?"  
❌ "How does currency safety work conceptually?"  

### Example Content (in your Rust code)

```rust
/// Represents a currency-safe monetary amount.
///
/// `Amount` combines a [`Decimal`] value with an ISO 4217 [`Currency`] code,
/// ensuring that arithmetic operations only occur between amounts of the
/// same currency.
///
/// # Examples
///
/// ```
/// use finstack::prelude::*;
///
/// let usd = Amount::from_str("100.50 USD")?;
/// let more_usd = Amount::from_str("50.00 USD")?;
/// let total = usd + more_usd;  // OK: both USD
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Amount {
    value: Decimal,
    currency: Currency,
}
```

---

## 📖 User Guide (mdBook)

**Hand-written tutorials, guides, and explanations**

### What It Is
- Narrative documentation teaching concepts and patterns
- Step-by-step tutorials for common tasks
- Conceptual explanations of how things work together
- Best practices and design patterns

### How to Use It

```bash
# Build and serve with live reload
make book-serve

# Just build (output in book/book/)
make book-build

# Watch for changes
make book-watch

# Clean build artifacts
make book-clean
```

### What It Should Contain

The placeholder pages are ready for you to fill with:

1. **Getting Started** ✅ (Already complete)
   - Installation instructions
   - Quick start tutorial
   - Core concepts overview

2. **Tutorials** (To be written)
   - "How to price a bond portfolio"
   - "Building a financial statement model"
   - "Running stress test scenarios"
   - "Calibrating a yield curve"

3. **Conceptual Guides** (To be written)
   - "Understanding currency safety"
   - "How determinism works"
   - "Market data organization"
   - "The pricing framework"

4. **Best Practices** (To be written)
   - "Organizing market data"
   - "Error handling patterns"
   - "Testing strategies"
   - "Performance optimization"

5. **Examples** (To be written)
   - Real-world scenarios with full code
   - Explained step-by-step
   - Including Python and WASM versions

### When to Use It
✅ "How do I get started with Finstack?"  
✅ "What's the best way to organize a pricing workflow?"  
✅ "How does currency safety work and why does it matter?"  
✅ "Step-by-step: pricing a portfolio with scenarios"  

❌ "What's the exact signature of `Amount::from_str`?"  
❌ "What fields does `BondSpec` have?"  
❌ "List all methods on `MarketContext`"  

### Where to Write Content

All content goes in **`book/src/`** as markdown files:

```
book/src/
├── introduction.md          ✅ Complete
├── getting-started/         ✅ Complete
│   ├── installation.md
│   ├── quick-start.md
│   └── core-concepts.md
├── core/                    📝 Placeholders (fill these in)
│   ├── currency-money.md
│   ├── market-data.md
│   └── ...
├── valuations/              📝 Placeholders (fill these in)
│   ├── bonds.md
│   ├── equity-options.md
│   └── ...
└── ...
```

### Example Content (in a markdown file)

````markdown
# Pricing a Bond

This guide shows you how to price a fixed-rate bond using Finstack.

## Prerequisites

- Installed Finstack (see [Installation](../getting-started/installation.md))
- Basic understanding of bonds

## Step 1: Create the Bond Specification

First, define your bond's characteristics:

```rust
use finstack::prelude::*;
use finstack::valuations::instruments::bond::*;

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
```

## Step 2: Set Up Market Data

[Continue with more steps...]
````

---

## Summary: Which System to Use?

| I want to... | Use |
|--------------|-----|
| Teach someone how to use Finstack | **mdBook** |
| Explain why currency safety matters | **mdBook** |
| Show a step-by-step bond pricing tutorial | **mdBook** |
| Document API parameters and return types | **rustdoc** |
| List all methods on a type | **rustdoc** |
| Show trait implementations | **rustdoc** |
| Provide a reference for developers | **rustdoc** |
| Write a conceptual guide | **mdBook** |

---

## Getting Started with mdBook

1. **Read the writing guide**
   ```bash
   cat book/WRITING_GUIDE.md
   ```

2. **Start the dev server**
   ```bash
   make book-serve
   ```

3. **Pick a page to write** (suggested order)
   - `book/src/valuations/bonds.md` - Most common use case
   - `book/src/core/market-data.md` - Fundamental concept
   - `book/src/scenarios/market-scenarios.md` - Popular feature

4. **Write narrative content**
   - Explain concepts
   - Show examples
   - Link to rustdoc for API details

5. **Test your changes**
   - Book auto-reloads in browser
   - Code examples should be runnable
   - Links should work

---

## Both Are Important!

Good documentation needs both:

- **rustdoc** ensures every API is documented at the code level
- **mdBook** provides the narrative to tie it all together

Think of mdBook as "The Finstack Book" (like "The Rust Book") and rustdoc as the standard library reference.

---

## Resources

- **mdBook Guide**: https://rust-lang.github.io/mdBook/
- **rustdoc Guide**: https://doc.rust-lang.org/rustdoc/
- **Writing Guide**: `book/WRITING_GUIDE.md`
- **Deployment Guide**: `book/DEPLOYMENT.md`

---

**Ready to write docs? Run `make book-serve` and start editing `book/src/` files!**

