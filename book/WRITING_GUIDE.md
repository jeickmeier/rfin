# Writing Documentation Guide

This guide explains how to effectively use mdBook for user-facing documentation alongside rustdoc for API references.

## Documentation Philosophy

Finstack has **two complementary documentation systems**:

### 1. API Reference (rustdoc)

**Purpose**: Technical reference for every public type, function, and module  
**Generated from**: `///` docstrings in Rust code  
**Command**: `make doc`  
**Audience**: Developers who know what they're looking for

**Example** (in your Rust code):
```rust
/// Represents a currency-safe monetary amount.
///
/// # Examples
///
/// ```
/// let amount = Amount::from_str("100.50 USD")?;
/// ```
pub struct Amount { ... }
```

### 2. User Guide (mdBook)

**Purpose**: Narrative documentation, tutorials, conceptual guides  
**Written in**: Markdown files in `book/src/`  
**Command**: `make book-serve`  
**Audience**: New users, people learning concepts, tutorial seekers

**Example** (in `book/src/core/currency-money.md`):
```markdown
# Working with Currency-Safe Amounts

In Finstack, all monetary values use the `Amount` type, which combines
a decimal value with a currency code to prevent accidental currency mixing.

## Why Currency Safety Matters

Imagine adding USD and EUR without conversion...
[narrative explanation with examples]
```

## What Goes Where?

| Content Type | mdBook | rustdoc |
|--------------|--------|---------|
| "How do I get started?" | ✅ | ❌ |
| "How does X work conceptually?" | ✅ | ❌ |
| "What are best practices for Y?" | ✅ | ❌ |
| "Step-by-step tutorial for Z" | ✅ | ❌ |
| "What parameters does function F take?" | ❌ | ✅ |
| "What methods does type T have?" | ❌ | ✅ |
| "What's the exact signature of X?" | ❌ | ✅ |
| Long-form examples with explanation | ✅ | Maybe brief |

## Writing mdBook Content

### Structure of a Good Page

```markdown
# Page Title

Brief introduction explaining what this page covers.

## Core Concept

Explanation of the main idea...

## Basic Example

\`\`\`rust
use finstack::prelude::*;

// Simple example with inline comments
let amount = Amount::from_str("100.00 USD")?;
\`\`\`

## Common Patterns

### Pattern 1: ...

Explanation and code...

### Pattern 2: ...

Explanation and code...

## Real-World Example

More complex example showing realistic usage...

## Common Pitfalls

- **Pitfall 1**: Why it happens and how to avoid it
- **Pitfall 2**: ...

## Next Steps

- Link to related topics
- Link to API reference for this module
```

### Guidelines

1. **Start with "why"** before "how"
2. **Use progressive disclosure** - simple examples first, complex later
3. **Include both Rust and Python** examples where applicable
4. **Link to API docs** for detailed reference
5. **Explain gotchas** and common mistakes
6. **Add "Next Steps"** to guide readers

### Code Examples

Always include:
- The full import path
- Comments explaining non-obvious parts
- Error handling where appropriate
- Both success and error cases

**Good example:**
```rust
use finstack::prelude::*;

fn price_bond() -> Result<()> {
    // Create the bond specification
    let bond = BondSpec {
        id: InstrumentId::new("BOND001"),
        issue_date: Date::from_ymd(2020, 1, 1),
        // ... more fields
    };
    
    // Set up market data
    let mut ctx = MarketContext::new(Date::today());
    ctx.add_discount_curve(/* ... */);
    
    // Price the bond
    let pricer = BondPricer::new(bond);
    let result = pricer.price(&ctx)?;
    
    println!("PV: {}", result.present_value());
    Ok(())
}
```

**Bad example:**
```rust
// Don't do this - too vague
let result = pricer.price(&ctx);
```

## Filling In the Pages

Start with the most important pages for users:

### Priority 1: Getting Started
- ✅ Already complete: Installation, Quick Start, Core Concepts

### Priority 2: Common Use Cases
Focus on what users will do most often:

1. **`valuations/bonds.md`** - How to price a bond (most common)
2. **`core/market-data.md`** - How to set up market data
3. **`scenarios/market-scenarios.md`** - How to run scenarios
4. **`portfolio/aggregation.md`** - How to aggregate positions

### Priority 3: Advanced Features
Fill in specialized topics:

1. Language bindings (Python/WASM)
2. Advanced valuations (exotics, structured products)
3. Performance optimization
4. Custom instruments

### Priority 4: Developer Documentation
For contributors:

1. Architecture overview
2. Contributing guide
3. Code standards
4. Release process

## Template for Module Pages

Use this template for core library modules:

```markdown
# [Module Name]

[Brief description of what this module does and when to use it]

## Quick Example

\`\`\`rust
// Minimal working example
\`\`\`

## Concepts

### Concept 1
Explanation...

### Concept 2
Explanation...

## Common Tasks

### Task 1: [Description]

\`\`\`rust
// Code example
\`\`\`

### Task 2: [Description]

\`\`\`rust
// Code example
\`\`\`

## API Reference

For detailed API documentation, see:
- [`ModuleName`](link-to-rustdoc)
- [`SpecificType`](link-to-rustdoc)

## Examples

### Example 1: [Realistic Scenario]

Full working example with explanation...

### Example 2: [Another Scenario]

Another full working example...

## Best Practices

1. **Practice 1**: Explanation
2. **Practice 2**: Explanation

## Troubleshooting

**Problem**: Common error message
**Solution**: How to fix it

**Problem**: Another issue
**Solution**: How to resolve

## Next Steps

- [Related Topic 1](link)
- [Related Topic 2](link)
```

## Tips

### Link to API Docs

When deployed, you can link directly to rustdoc:

```markdown
See the [`Amount`](../api/finstack/core/struct.Amount.html) API reference.
```

### Use Callouts

Create visual emphasis for important points:

```markdown
> **Note**: Important information that's helpful but not critical.

> **Warning**: Something that could cause problems if ignored.

> **Tip**: A helpful suggestion or best practice.
```

### Include Python Examples

For pages covering features available in Python:

````markdown
## Rust

```rust
use finstack::prelude::*;
let amount = Amount::from_str("100.00 USD")?;
```

## Python

```python
from finstack import Amount
amount = Amount.from_str("100.00 USD")
```
````

### Cross-Reference Liberally

Help readers navigate:

```markdown
For more on dates, see [Dates & Time](../core/dates-time.md).

This builds on concepts from [Currency Safety](./currency-safety.md).
```

## Maintenance

### Keeping in Sync

When you update code:
1. Update rustdoc comments first (in the code)
2. Update mdBook if the change affects tutorials or guides
3. Update examples if interfaces change

### Testing

Run these before committing documentation:

```bash
# Build the book (checks for broken references)
make book-build

# Generate rustdoc (checks for doc errors)
make doc

# Run doc tests (ensures code examples work)
make test-doc
```

## Getting Help

- **mdBook Guide**: https://rust-lang.github.io/mdBook/
- **rustdoc Guide**: https://doc.rust-lang.org/rustdoc/
- **Examples**: Look at the Rust Book source for inspiration

---

Remember: **Good documentation is iterative**. Start with the basics, gather feedback, and improve over time!





