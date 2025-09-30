# Capital Structure Integration Guide

**Last updated:** 2025-09-30  
**Feature flag:** `capital_structure`

---

## Overview

The capital structure integration enables modeling of debt instruments (bonds, loans, swaps) and automatically calculating their impact on financial statements through period-aligned cashflow aggregation.

**Key Features:**
- Leverage existing instrument types from `finstack-valuations`
- Automatic cashflow generation and period aggregation
- DSL references to capital structure metrics
- Multi-currency support with explicit FX

---

## 1. Enabling Capital Structure

### 1.1 Cargo.toml

```toml
[dependencies]
finstack-statements = { version = "0.2", features = ["capital_structure"] }
```

### 1.2 Required Dependencies

- `finstack-valuations` — Instrument types, cashflow generation
- `MarketContext` — Discount/forward curves for valuation

---

## 2. Supported Instruments

### 2.1 Fixed-Rate Bond

```rust
.add_bond(
    "BOND-001",                                    // Instrument ID
    Money::new(10_000_000.0, Currency::USD),      // Notional
    0.05,                                          // 5% coupon (annual)
    Date::from_calendar_date(2025, Month::January, 15).unwrap(),  // Issue date
    Date::from_calendar_date(2030, Month::January, 15).unwrap(),  // Maturity
    "USD-OIS",                                     // Discount curve ID
)?
```

**Generates:**
- Periodic coupon payments (fixed)
- Principal repayment at maturity

---

### 2.2 Interest Rate Swap

```rust
.add_swap(
    "SWAP-001",
    Money::new(5_000_000.0, Currency::USD),
    0.04,  // Fixed rate (4%)
    start_date,
    maturity_date,
    "USD-OIS",    // Discount curve
    "USD-SOFR",   // Forward curve
)?
```

**Generates:**
- Fixed leg cashflows
- Floating leg cashflows (projected from forward curve)

---

### 2.3 Generic Instrument (JSON)

```rust
.add_custom_debt(
    "TL-A",
    DebtInstrumentSpec::Generic {
        id: "TL-A".into(),
        spec: json!({
            "type": "amortizing_loan",
            "notional": 25_000_000.0,
            "currency": "USD",
            "issue_date": "2025-01-15",
            "maturity_date": "2030-01-15",
            "coupon_rate": 0.06,
            "frequency": "quarterly",
            "amortization": {
                "type": "linear",
                "final_notional": 0.0
            }
        }),
    },
)?
```

---

## 3. Referencing Capital Structure in Formulas

### 3.1 DSL Syntax

Capital structure references use the `cs.*` namespace:

```
cs.<metric>.<instrument_id>
cs.<metric>.total
```

**Available Metrics:**
- `interest_expense` — Interest payments (coupons, floating resets)
- `principal_payment` — Principal repayments (amortization, maturity)
- `debt_balance` — Outstanding notional at period end

### 3.2 Examples

```rust
// Interest expense for specific bond
.compute("bond_interest", "cs.interest_expense.BOND-001")?

// Total interest expense across all debt
.compute("total_interest", "cs.interest_expense.total")?

// Principal payments for term loan
.compute("tl_amortization", "cs.principal_payment.TL-A")?

// Total debt service (interest + principal)
.compute("debt_service", "cs.interest_expense.total + cs.principal_payment.total")?

// Outstanding debt balance
.compute("debt_balance", "cs.debt_balance.total")?
```

---

## 4. Complete Example: Leveraged Buyout Model

```rust
use finstack_statements::prelude::*;
use finstack_core::{Currency, Date, Month};
use finstack_valuations::MarketContext;

fn build_lbo_model() -> Result<FinancialModel> {
    // Set up market context
    let base_date = Date::from_calendar_date(2025, Month::January, 1)?;
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.75)])
        .build()?;
    
    let market_ctx = MarketContext::new()
        .insert_discount(discount_curve);
    
    // Build model
    let model = ModelBuilder::new("LBO Model")
        .periods("2025Q1..2030Q4", Some("2025Q1..Q2"))?
        
        // Operating model
        .value("revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(50_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(52_000_000.0)),
        ])
        .forecast("revenue", ForecastSpec {
            method: ForecastMethod::GrowthPct,
            params: indexmap! { "rate".into() => json!(0.05) },
        })
        .compute("cogs", "revenue * 0.6")?
        .compute("opex", "revenue * 0.2")?
        .compute("ebitda", "revenue - cogs - opex")?
        
        // Capital structure: Senior debt + Sub debt
        .add_bond(
            "Senior-Notes",
            Money::new(100_000_000.0, Currency::USD),
            0.06,  // 6% coupon
            Date::from_calendar_date(2025, Month::January, 15)?,
            Date::from_calendar_date(2030, Month::January, 15)?,
            "USD-OIS",
        )?
        .add_bond(
            "Sub-Notes",
            Money::new(50_000_000.0, Currency::USD),
            0.09,  // 9% coupon
            Date::from_calendar_date(2025, Month::January, 15)?,
            Date::from_calendar_date(2032, Month::January, 15)?,
            "USD-OIS",
        )?
        
        // Integrate debt into P&L
        .compute("interest_expense", "cs.interest_expense.total")?
        .compute("ebt", "ebitda - interest_expense")?
        .compute("taxes", "if(ebt > 0, ebt * 0.25, 0)")?
        .compute("net_income", "ebt - taxes")?
        
        // Cashflow statement
        .compute("fcf", "ebitda - taxes - cs.principal_payment.total")?
        
        // Credit metrics
        .compute("debt_balance", "cs.debt_balance.total")?
        .compute("leverage", "debt_balance / ttm(ebitda)")?
        .compute("interest_coverage", "ttm(ebitda) / ttm(interest_expense)")?
        
        .build()?;
    
    Ok(model)
}

fn main() -> Result<()> {
    let model = build_lbo_model()?;
    
    // Evaluate with market context
    let market_ctx = /* ... create market context ... */;
    let mut evaluator = Evaluator::with_market_context(Arc::new(market_ctx));
    let results = evaluator.evaluate(&model, false)?;
    
    // Export results
    let df = results.to_polars_long()?;
    println!("{}", df);
    
    Ok(())
}
```

---

## 5. Cashflow Aggregation

### 5.1 How It Works

1. **Generate cashflows** — Each instrument generates a schedule of dated cashflows
2. **Classify cashflows** — By kind: Interest, Principal, Fee, etc.
3. **Aggregate by period** — Map cashflows to statement periods
4. **Currency handling** — Multi-currency flows use explicit FX

### 5.2 Period Alignment

Cashflows are aggregated to periods using these rules:

| Cashflow Date | Belongs to Period |
|---------------|-------------------|
| `2025-01-15` | `2025Q1` (if period contains date) |
| `2025-02-28` | `2025Q1` (Jan-Mar) |
| `2025-04-01` | `2025Q2` (Apr-Jun) |

**Logic:** A cashflow belongs to the period whose `[start, end)` interval contains the cashflow date.

---

## 6. Multi-Currency Handling

### 6.1 Single Currency (Simple)

If all instruments are in the same currency, no FX conversion needed:

```rust
.add_bond("B1", Money::new(10_000_000.0, Currency::USD), ...)?
.add_bond("B2", Money::new(5_000_000.0, Currency::USD), ...)?

// Both aggregate directly
.compute("total_interest", "cs.interest_expense.total")?  // USD
```

### 6.2 Multi-Currency (FX Required)

If instruments span multiple currencies, you must provide FX configuration:

```rust
.add_bond("US-Bond", Money::new(10_000_000.0, Currency::USD), ...)?
.add_bond("EU-Bond", Money::new(8_000_000.0, Currency::EUR), ...)?

// Specify model currency
.capital_structure_config(CapitalStructureConfig {
    model_currency: Currency::USD,
    fx_policy: FxConversionPolicy::PeriodEnd,  // Convert at period-end rates
})?
```

**FX Policies:**
- `PeriodEnd` — Convert using period-end spot rate
- `CashflowDate` — Convert using cashflow date spot rate
- `Average` — Convert using period-average rate

---

## 7. Amortizing Schedules

### 7.1 Linear Amortization

```rust
.add_custom_debt(
    "TL-A",
    DebtInstrumentSpec::Generic {
        id: "TL-A".into(),
        spec: json!({
            "type": "amortizing_loan",
            "notional": 100_000_000.0,
            "amortization": {
                "type": "linear",
                "payment_dates": ["2025Q4", "2026Q4", "2027Q4", ...],
            }
        }),
    },
)?
```

**Result:** Equal principal payments each period.

### 7.2 Custom Schedule

```rust
"amortization": {
    "type": "custom",
    "schedule": [
        {"date": "2026-01-15", "principal": 10_000_000.0},
        {"date": "2027-01-15", "principal": 15_000_000.0},
        {"date": "2028-01-15", "principal": 20_000_000.0},
        ...
    ]
}
```

---

## 8. Advanced: Revolving Credit Facility

For more complex structures (revolver with draws/repayments), use the generic instrument type:

```json
{
  "type": "revolver",
  "facility_size": 50_000_000.0,
  "currency": "USD",
  "commitment_fee": 0.005,  // 50bps on undrawn
  "utilization_fee": 0.001,  // 10bps on drawn
  "interest_rate": 0.055,    // 5.5% on outstanding
  "draws": [
    {"date": "2025-03-15", "amount": 10_000_000.0},
    {"date": "2025-06-15", "amount": 5_000_000.0}
  ],
  "repayments": [
    {"date": "2025-12-15", "amount": 8_000_000.0}
  ]
}
```

---

## 9. Validation

The capital structure validates:

- [ ] All `discount_curve_id` references exist in `MarketContext`
- [ ] Issue date < maturity date
- [ ] Coupon rates are valid (0.0 to 1.0 for percentages)
- [ ] Notional amounts are positive
- [ ] No duplicate instrument IDs

**Validation happens:**
- At build time (`.build()`)
- Errors are clear and actionable

---

## 10. Troubleshooting

### Issue: "Discount curve not found"

```
Error: Capital structure validation failed
Cause: Discount curve 'USD-OIS' not found in MarketContext
```

**Fix:** Ensure the discount curve is added to the market context before evaluation:

```rust
let market_ctx = MarketContext::new()
    .insert_discount(discount_curve);

let evaluator = Evaluator::with_market_context(Arc::new(market_ctx));
```

---

### Issue: "Currency mismatch in aggregation"

```
Error: Cannot aggregate cashflows with different currencies without FX provider
```

**Fix:** Configure FX policy:

```rust
.capital_structure_config(CapitalStructureConfig {
    model_currency: Currency::USD,
    fx_policy: FxConversionPolicy::PeriodEnd,
})?
```

---

### Issue: "Interest expense is zero"

**Possible causes:**
1. Coupon frequency doesn't align with periods
2. Cashflows fall outside period boundaries
3. Issue date is after the evaluation period

**Debug:**
```rust
// Inspect cashflow schedule
let capital_structure = model.capital_structure.as_ref().unwrap();
let flows = capital_structure.aggregate_cashflows(&model.periods)?;
println!("Cashflows: {:#?}", flows);
```

---

## 11. Performance Considerations

- **Cashflow generation:** O(n) where n = number of payment dates
- **Period aggregation:** O(m * p) where m = cashflows, p = periods
- **Large models:** 100+ instruments, 60+ periods evaluates in < 100ms

**Optimization tips:**
- Use bullet bonds (no amortization) when possible — fewer cashflows
- Aggregate instruments at model level, not per-period
- Cache cashflow schedules if re-evaluating model

---

## References

- [API Reference](./API_REFERENCE.md) — Full API documentation
- [Examples](./examples/capital_structure.md) — More examples
- [Valuations Docs](../../03_valuations/) — Instrument details
