# Example: Reusing Code Examples

**I just demonstrated this in the Portfolio Overview page!**

## What I Did

In `book/src/portfolio/overview.md`, I added:

```markdown
{{#include ../../../finstack/examples/portfolio/portfolio_example.rs}}
```

This **automatically includes** your existing example from `finstack/examples/portfolio/portfolio_example.rs`.

## Result

✅ **Code written ONCE** in `examples/`  
✅ **Appears in mdBook** automatically  
✅ **Runnable** with `cargo run --example portfolio_example`  
✅ **Testable** with your normal test suite  
✅ **No duplication!**

## How to Use This Pattern

### 1. Write Your Example Once

```rust
// finstack/examples/bond_pricing.rs
use finstack::prelude::*;

fn main() -> Result<()> {
    // Your example code here
    Ok(())
}
```

### 2. Include in mdBook

```markdown
# Bond Pricing Guide

Here's a complete example:

{{#include ../../finstack/examples/bond_pricing.rs}}

Run it yourself:
\`\`\`bash
cargo run --example bond_pricing
\`\`\`
```

### 3. Reference in Rustdoc

```rust
/// # Examples
///
/// For a complete example, see `examples/bond_pricing.rs` or run:
/// ```bash
/// cargo run --example bond_pricing
/// ```
pub struct BondPricer { ... }
```

## Advanced: Include Specific Sections

You can include only parts of a file using anchors:

### In Your Example File:

```rust
// examples/bond_pricing.rs

use finstack::prelude::*;

fn main() -> Result<()> {
    // ANCHOR: setup
    let bond = BondSpec {
        id: InstrumentId::new("BOND001"),
        // ...
    };
    // ANCHOR_END: setup
    
    // ANCHOR: pricing
    let pricer = BondPricer::new(bond);
    let result = pricer.price(&ctx)?;
    // ANCHOR_END: pricing
    
    Ok(())
}
```

### In Your mdBook Page:

```markdown
## Setting Up the Bond

{{#include ../../examples/bond_pricing.rs:setup}}

## Pricing

{{#include ../../examples/bond_pricing.rs:pricing}}
```

This includes ONLY the sections between `ANCHOR` and `ANCHOR_END`.

## Check It Out!

Run this to see it in action:

```bash
make book-serve
```

Then navigate to **Portfolio → Overview** to see your `portfolio_example.rs` included!

## Summary

You now have **three types of documentation** from **one source of truth**:

1. **Runnable examples** in `examples/` directory
2. **mdBook tutorials** that include those examples
3. **Rustdoc API docs** that reference those examples

**No duplication needed!** 🎉





