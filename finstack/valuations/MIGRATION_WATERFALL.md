# Waterfall Engine Migration Guide

This guide helps you migrate from the old `PaymentRule`-based waterfall to the new tier-based waterfall engine.

## Overview of Changes

The waterfall engine has been completely refactored to support:
- **Tier-based architecture** with multi-recipient tiers
- **Pro-rata and sequential** allocation modes
- **Configurable diversion rules** with circular detection
- **Validation framework** for spec correctness
- **Pre-built templates** for common deal types
- **Enhanced explainability** with tier-level tracking

## Breaking Changes

### 1. PaymentRule → WaterfallTier + Recipient

**Old API:**
```rust
PaymentRule::new(
    "fee_id",
    priority,
    PaymentRecipient::ServiceProvider("Trustee".into()),
    PaymentCalculation::FixedAmount { amount },
)
```

**New API:**
```rust
WaterfallTier::new("fees", priority, PaymentType::Fee)
    .add_recipient(Recipient::new(
        "trustee_fee",
        PaymentRecipient::ServiceProvider("Trustee".into()),
        PaymentCalculation::FixedAmount { amount },
    ))
```

### 2. WaterfallEngine Construction

**Old API:**
```rust
let engine = WaterfallEngine::new(Currency::USD)
    .add_rule(rule1)
    .add_rule(rule2);
```

**New API:**
```rust
let engine = WaterfallBuilder::new(Currency::USD)
    .add_tier(tier1)
    .add_tier(tier2)
    .build();
```

### 3. Waterfall Execution

**Old API:**
```rust
engine.apply_waterfall(
    available_cash,
    interest_collections,
    payment_date,
    tranches,
    pool_balance,
    pool,
    market,
)?
```

**New API (same method name):**
```rust
engine.execute_waterfall(
    available_cash,
    interest_collections,
    payment_date,
    tranches,
    pool_balance,
    pool,
    market,
)?
```

**Note:** `apply_waterfall` is still available as a legacy alias.

### 4. WaterfallResult Structure

**New fields added:**
- `tier_allocations: Vec<(String, Money)>` - cash allocated to each tier
- `coverage_tests: Vec<(String, f64, bool)>` - test results (name, ratio, passed)
- `diverted_cash: Money` - total cash diverted due to test failures

**Example:**
```rust
let result = engine.execute_waterfall(/* ... */)?;

// Access tier-level allocations
for (tier_id, amount) in &result.tier_allocations {
    println!("Tier {}: {}", tier_id, amount);
}

// Check coverage test results
for (test_name, ratio, passed) in &result.coverage_tests {
    println!("Test {}: ratio={:.2}, passed={}", test_name, ratio, passed);
}
```

## Migration Steps

### Step 1: Update Imports

```rust
// Old
use finstack::structured_credit::{
    PaymentRule, WaterfallEngine, ...
};

// New
use finstack::structured_credit::{
    WaterfallTier, Recipient, WaterfallBuilder, 
    AllocationMode, PaymentType, ...
};
```

### Step 2: Convert PaymentRules to Tiers

Group related payments into tiers:

```rust
// Old: Multiple individual rules
let trustee_rule = PaymentRule::new("trustee", 1, recipient1, calc1);
let admin_rule = PaymentRule::new("admin", 2, recipient2, calc2);
let class_a_int = PaymentRule::new("class_a_int", 3, recipient3, calc3);

// New: Group into tiers
let fees_tier = WaterfallTier::new("fees", 1, PaymentType::Fee)
    .add_recipient(Recipient::new("trustee", recipient1, calc1))
    .add_recipient(Recipient::new("admin", recipient2, calc2));

let interest_tier = WaterfallTier::new("interest", 2, PaymentType::Interest)
    .add_recipient(Recipient::new("class_a_int", recipient3, calc3));
```

### Step 3: Set Allocation Modes

Choose between sequential and pro-rata for each tier:

```rust
// Sequential (default): pay recipients in order
let tier = WaterfallTier::new("tier1", 1, PaymentType::Fee)
    .allocation_mode(AllocationMode::Sequential)
    .add_recipient(recipient1)
    .add_recipient(recipient2);

// Pro-rata: distribute proportionally
let tier = WaterfallTier::new("tier2", 2, PaymentType::Interest)
    .allocation_mode(AllocationMode::ProRata)
    .add_recipient(recipient1.with_weight(0.60))  // 60%
    .add_recipient(recipient2.with_weight(0.40)); // 40%
```

### Step 4: Use Templates (Recommended)

Instead of building from scratch, use pre-built templates:

```rust
// Old: Manual construction
let engine = WaterfallEngine::standard_sequential(currency, &tranches, fees);

// New: Use template
use finstack::structured_credit::templates::clo_2_0_template;
let engine = clo_2_0_template(Currency::USD);

// Or get by name
use finstack::structured_credit::templates::get_template;
let engine = get_template("clo_2.0", Currency::USD).unwrap();
```

Available templates:
- `clo_2_0_template` - Standard CLO 2.0 with OC/IC tests
- `cmbs_standard_template` - CMBS with sequential pay
- `cre_operating_company_template` - CRE operating distributions with promote

### Step 5: Add Validation

Validate your waterfall spec before execution:

```rust
use finstack::structured_credit::{WaterfallValidator, is_valid_waterfall_spec};

// Option 1: Check validity
if !is_valid_waterfall_spec(&tiers, &diversion_rules, &test_ids) {
    println!("Invalid waterfall spec!");
}

// Option 2: Get specific errors
use finstack::structured_credit::get_validation_errors;
let errors = get_validation_errors(&tiers, &diversion_rules, &test_ids);
for error in errors {
    println!("Validation error: {}", error);
}
```

## Complete Example

### Before (Old API)

```rust
use finstack::structured_credit::{
    PaymentRule, PaymentRecipient, PaymentCalculation,
    WaterfallEngine, ManagementFeeType,
};

let trustee_rule = PaymentRule::new(
    "trustee",
    1,
    PaymentRecipient::ServiceProvider("Trustee".into()),
    PaymentCalculation::FixedAmount {
        amount: Money::new(50_000.0, Currency::USD),
    },
);

let mgmt_rule = PaymentRule::new(
    "mgmt_fee",
    2,
    PaymentRecipient::ManagerFee(ManagementFeeType::Senior),
    PaymentCalculation::PercentageOfCollateral {
        rate: 0.004,
        annualized: true,
    },
);

let engine = WaterfallEngine::new(Currency::USD)
    .add_rule(trustee_rule)
    .add_rule(mgmt_rule);

let result = engine.apply_waterfall(/* ... */)?;
```

### After (New API)

```rust
use finstack::structured_credit::{
    WaterfallBuilder, WaterfallTier, Recipient,
    PaymentRecipient, PaymentCalculation, PaymentType,
    AllocationMode, ManagementFeeType,
};

let fees_tier = WaterfallTier::new("fees", 1, PaymentType::Fee)
    .allocation_mode(AllocationMode::Sequential)
    .add_recipient(Recipient::new(
        "trustee",
        PaymentRecipient::ServiceProvider("Trustee".into()),
        PaymentCalculation::FixedAmount {
            amount: Money::new(50_000.0, Currency::USD),
        },
    ))
    .add_recipient(Recipient::new(
        "mgmt_fee",
        PaymentRecipient::ManagerFee(ManagementFeeType::Senior),
        PaymentCalculation::PercentageOfCollateral {
            rate: 0.004,
            annualized: true,
        },
    ));

let engine = WaterfallBuilder::new(Currency::USD)
    .add_tier(fees_tier)
    .build();

let result = engine.execute_waterfall(/* ... */)?;

// Access new fields
println!("Tier allocations:");
for (tier_id, amount) in &result.tier_allocations {
    println!("  {}: {}", tier_id, amount);
}
```

### After (Using Template - Recommended)

```rust
use finstack::structured_credit::templates::clo_2_0_template;

let engine = clo_2_0_template(Currency::USD);
let result = engine.execute_waterfall(/* ... */)?;
```

## New Features

### 1. Pro-Rata Distribution

```rust
let tier = WaterfallTier::new("distributions", 1, PaymentType::Interest)
    .allocation_mode(AllocationMode::ProRata)
    .add_recipient(Recipient::new(/*...*/).with_weight(0.70))
    .add_recipient(Recipient::new(/*...*/).with_weight(0.30));
```

### 2. Diversion Rules

```rust
use finstack::structured_credit::{DiversionRule, DiversionCondition};

let rule = DiversionRule::on_test_failure(
    "rule1",
    "subordinated_tier",
    "senior_tier",
    "oc_test",
    1,
);

let engine = builder
    .add_tier(tier1)
    .add_tier(tier2)
    .add_diversion_rule(rule)  // Future feature
    .build();
```

### 3. Coverage Test Integration

```rust
use finstack::structured_credit::CoverageTest;

// Tests now have IDs for diversion reference
let oc_test = CoverageTest::new_oc_with_id("oc_class_a", 1.25);
let ic_test = CoverageTest::new_ic_with_id("ic_class_a", 1.20);

// Results include test IDs
let result = engine.execute_waterfall(/* ... */)?;
for (test_id, ratio, passed) in &result.coverage_tests {
    if !passed {
        println!("Test {} failed: ratio={:.2}", test_id, ratio);
    }
}
```

## Troubleshooting

### Error: "Duplicate tier ID"

**Cause:** Multiple tiers have the same ID.

**Solution:**
```rust
// Bad
.add_tier(WaterfallTier::new("tier1", 1, PaymentType::Fee))
.add_tier(WaterfallTier::new("tier1", 2, PaymentType::Interest))  // Duplicate!

// Good
.add_tier(WaterfallTier::new("fees", 1, PaymentType::Fee))
.add_tier(WaterfallTier::new("interest", 2, PaymentType::Interest))
```

### Error: "Circular diversion detected"

**Cause:** Diversion rules create a cycle (A → B → A).

**Solution:** Review diversion rule dependencies and remove cycles.

### Error: "Tier has no recipients"

**Cause:** Created a tier without adding any recipients.

**Solution:**
```rust
// Bad
let tier = WaterfallTier::new("empty", 1, PaymentType::Fee);

// Good
let tier = WaterfallTier::new("fees", 1, PaymentType::Fee)
    .add_recipient(recipient1);
```

## Performance Considerations

The new tier-based engine:
- ✅ **Same or better performance** for typical waterfalls
- ✅ **More efficient** for pro-rata distributions (single calculation vs multiple)
- ✅ **Faster validation** with upfront checks
- ✅ **Better memory locality** with tier grouping

## Getting Help

- Review the [Waterfall Engine documentation](../book/src/valuations/structured-credit-waterfall.md)
- Check the [examples](../finstack/examples/valuations/)
- See template implementations in `templates/` for patterns
- Run tests with `cargo test structured_credit::waterfall`

## Summary

Key migration points:
1. Replace `PaymentRule` with `WaterfallTier` + `Recipient`
2. Use `WaterfallBuilder` instead of direct `WaterfallEngine::new()`
3. Group related payments into tiers
4. Set allocation mode (Sequential or ProRata)
5. Consider using pre-built templates
6. Validate specs before execution
7. Access new `tier_allocations` and `coverage_tests` in results

The migration provides:
- **More flexibility** with multi-recipient tiers
- **Better organization** with tier-based structure
- **Enhanced validation** catching errors early
- **Improved explainability** with tier-level tracking
- **Pre-built templates** for faster development

