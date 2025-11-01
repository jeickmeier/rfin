# Quick Start

This guide walks you through your first Finstack program, demonstrating core concepts.

## Working with Amounts

Finstack enforces currency safety at the type level. All monetary values use the `Amount` type:

```rust
use finstack::prelude::*;

fn main() -> Result<()> {
    // Create amounts with explicit currencies
    let usd_amount = Amount::from_str("100.50 USD")?;
    let eur_amount = Amount::from_str("85.25 EUR")?;
    
    // Arithmetic works within the same currency
    let total_usd = usd_amount + Amount::from_str("50.00 USD")?;
    println!("Total USD: {}", total_usd);
    
    // Cross-currency arithmetic requires explicit FX
    // This will compile-time error:
    // let mixed = usd_amount + eur_amount;  // ❌ Won't compile!
    
    Ok(())
}
```

## Dates and Calendars

Work with financial dates, calendars, and day count conventions:

```rust
use finstack::prelude::*;

fn main() -> Result<()> {
    let start = Date::from_ymd(2024, 1, 1);
    let end = Date::from_ymd(2024, 12, 31);
    
    // Use business day calendars
    let cal = Calendar::new("US", vec![])?;
    let biz_days = cal.business_days_between(start, end);
    println!("Business days: {}", biz_days);
    
    // Calculate day count fractions
    let dcf = DayCount::Actual360.day_fraction(start, end);
    println!("Day count fraction (ACT/360): {}", dcf);
    
    Ok(())
}
```

## Pricing a Bond

Here's a complete example pricing a fixed-rate bond:

```rust
use finstack::prelude::*;
use finstack::valuations::instruments::bond::*;

fn main() -> Result<()> {
    // Create a simple fixed-rate bond
    let bond = BondSpec {
        id: InstrumentId::new("BOND001"),
        issue_date: Date::from_ymd(2020, 1, 1),
        maturity_date: Date::from_ymd(2025, 1, 1),
        coupon_rate: Rate::from_percent(5.0),
        face_value: Amount::from_str("1000.00 USD")?,
        frequency: Frequency::Semiannual,
        day_count: DayCount::Actual360,
        currency: Currency::USD,
        ..Default::default()
    };
    
    // Create market data context
    let val_date = Date::from_ymd(2024, 1, 1);
    let mut ctx = MarketContext::new(val_date);
    
    // Add a discount curve
    let curve = DiscountCurve::flat(
        CurveId::new("USD-GOVT"),
        val_date,
        Rate::from_percent(4.0),
    );
    ctx.add_discount_curve(curve);
    
    // Price the bond
    let pricer = BondPricer::new(bond);
    let result = pricer.price(&ctx)?;
    
    println!("Bond PV: {}", result.present_value());
    println!("Bond yield: {}", result.metrics().get("yield")?);
    
    Ok(())
}
```

## Working with Scenarios

Test what-if scenarios on market data:

```rust
use finstack::prelude::*;
use finstack::scenarios::*;

fn main() -> Result<()> {
    // Create base market context
    let val_date = Date::from_ymd(2024, 1, 1);
    let mut ctx = MarketContext::new(val_date);
    
    // ... populate market data ...
    
    // Define scenarios
    let scenarios = vec![
        ScenarioSpec::parse("shift USD-GOVT +100bp")?,
        ScenarioSpec::parse("shock USD spot -10%")?,
    ];
    
    // Apply and compare
    for scenario in scenarios {
        let stressed_ctx = scenario.apply(&ctx)?;
        // ... reprice instruments ...
    }
    
    Ok(())
}
```

## Next Steps

Now that you understand the basics:

- Explore [Core Concepts](./core-concepts.md)
- Dive into [Valuations](../valuations/overview.md) for pricing instruments
- Learn about [Statements](../statements/overview.md) for financial modeling
- Try [Scenarios](../scenarios/overview.md) for stress testing
- Build [Portfolios](../portfolio/overview.md) for multi-instrument analysis
