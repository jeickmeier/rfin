# Instrument Pricing Refactoring Pattern

## Overview

This document describes the simplified pricing pattern implemented for BasisSwap and Basket instruments and how to apply it to other instruments.

## The Problem

The original pattern had redundant layers:
1. **Engine** (e.g., `BasisEngine`) - Static methods with pricing logic
2. **Pricer** (e.g., `SimpleBasisSwapDiscountingPricer`) - Thin wrapper that downcasts and delegates to engine
3. **Instrument** - Uses `impl_instrument!` macro to delegate to engine

This creates unnecessary indirection and code duplication.

## The Solution

Move pricing logic directly to the instrument struct as methods:

1. **Instrument struct** - Contains all pricing logic as instance methods
2. **Simplified Pricer** - Minimal wrapper for registry integration (can be in the root module)
3. **Instrument trait** - Implemented via `impl_instrument!` macro, calls instrument's own methods
4. **Proper curve specification** - Add `discount_curve_id` field instead of hardcoding curve names

The result is a flatter structure:
```
instruments/basis_swap/
├── mod.rs        # Module declaration and exports
├── types.rs      # Instrument struct with pricing methods
├── pricer.rs     # Simple registry pricer (no subdirectory needed)
└── metrics/      # Metric calculators
    └── ...
```

## Step-by-Step Refactoring

### 1. Move Engine Logic to Instrument

Convert static engine methods to instance methods on the instrument struct:

```rust
// Before (in engine.rs)
impl BasisEngine {
    pub fn npv(swap: &BasisSwap, curves: &MarketContext, as_of: Date) -> Result<Money> {
        // pricing logic
    }
}

// After (in types.rs)
impl BasisSwap {
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        // same pricing logic, but as instance method
    }
}
```

### 2. Update impl_instrument! Macro Usage

Update the macro to call the instrument's own method:

```rust
crate::impl_instrument!(
    BasisSwap,
    "BasisSwap",
    pv = |s, curves, as_of| {
        // Call the instrument's own method
        s.npv(curves, as_of)
    }
);
```

### 3. Simplify the Pricer

The pricer becomes a simple wrapper that:
- Downcasts the instrument
- Gets the as_of date from the appropriate curve
- Calls the instrument's value() method
- Returns a stamped result

```rust
impl Pricer for SimpleBasisSwapDiscountingPricer {
    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        let typed = instrument.as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or(/* type error */)?;
        
        // Get as_of from the instrument's discount curve
        let disc = market.get_discount_ref(typed.discount_curve_id().clone())?;
        let as_of = disc.base_date();
        
        // Use instrument's value method
        let pv = typed.value(market, as_of)?;
        
        Ok(ValuationResult::stamped(typed.id(), as_of, pv))
    }
}
```

### 4. Update Metrics

Update metric calculators to use the instrument's methods directly:

```rust
// Before
BasisEngine::annuity_for_leg(&schedule, leg.day_count, disc_id, curves)

// After  
swap.annuity_for_leg(leg, &schedule, curves)
```

### 5. Flatten Structure (if appropriate)

If the pricing directory now only contains the pricer:
- Move pricer.rs to the instrument's root directory
- Delete the pricing subdirectory
- Update mod.rs to export the pricer directly
- Update registry imports

### 6. Clean Up

- Delete the engine.rs file
- Remove empty pricing directory
- Update mod.rs to remove engine exports
- Update any external imports

## Benefits

1. **Reduced Indirection** - One less layer of abstraction
2. **Better Encapsulation** - Pricing logic is part of the instrument
3. **Clearer Code** - Direct method calls instead of static utility functions
4. **Easier Testing** - Test the instrument directly
5. **Better IDE Support** - Method autocomplete on instrument instances

## Alternative: Generic Pricer

For simple instruments, consider using the generic pricer:

```rust
// For discounting-based pricing
pub type SimpleDepositDiscountingPricer = GenericDiscountingPricer<Deposit>;

// The generic pricer handles the common pattern
```

However, this generic approach has limitations:
- Difficulty getting the correct as_of date
- Less flexibility for instrument-specific logic

## When to Use Each Pattern

- **Direct Methods** (like BasisSwap/Basket): When the instrument has specific pricing logic and needs access to its fields
- **Generic Pricer**: For simple instruments with standard discounting patterns  
- **Keep Engine**: Only if the logic is truly reusable across multiple unrelated instruments

## Key Design Principle: Avoid Redundant PV Metrics

**✅ Good PV Metrics** (Calculate something different from `instrument.value()`):
- **BasisSwap**: Individual leg PVs (components of overall NPV)
- **FRA**: Returns `context.base_value` (reuses computed value efficiently)
- **Bond**: Various price metrics (clean price, dirty price, etc.)

**❌ Bad PV Metrics** (Just duplicate `instrument.value()`):
- Metrics that call the same calculator methods as the instrument's `value()` method
- Different parameterizations of the same calculation the instrument already does

**Rule**: Only create PV metrics for components, legs, or specific calculations that are genuinely different from the instrument's main valuation.

## Separation of Concerns Guidelines

### What Goes Where:

**types.rs** - Type definitions only:
- Struct definitions with fields
- Enums and supporting types  
- Basic constructors and builders
- Simple validation methods
- Trait implementations that delegate to calculators

**calculator.rs** - Business logic:
- All pricing calculations
- Complex algorithms
- Helper methods for calculations
- State-dependent operations
- Only if complicated or multiple models. For simple cases just put into pricer.rs.

**pricer.rs** - Integration (often not needed):
- Registry integration
- Type downcasting  
- Delegation to calculator
- Result formatting
- **Consider using `GenericDiscountingPricer<T>` instead of custom pricers**

**metrics.rs** - Specific calculations:
- Use calculator for computations
- Handle metric-specific logic
- Dependencies and caching

### Simplification Guidelines

When refactoring, consider removing features that are:
- **Operational rather than pricing-related** (ETF creation/redemption mechanics)
- **Metadata that doesn't affect valuation** (ticker symbols, names, rebalancing frequencies)
- **Complex analysis that could be separate** (tracking error, performance attribution)
- **Configuration that could be parameters** (shares outstanding, AUM)

Focus on the **core pricing essentials**: constituents, expense ratios, currencies, discount curves.

## Examples

### BasisSwap Refactoring

- Moved `BasisEngine` static methods to `BasisSwap` instance methods
- Updated all metrics to use `swap.method()` instead of `BasisEngine::method()`  
- Flattened `pricing/pricer.rs` to `pricer.rs`
- Removed engine.rs entirely

### Basket Refactoring  

- **Proper separation of concerns:**
  - Moved pricing logic from `Basket` struct to separate `BasketCalculator` in `pricer.rs`
  - `types.rs` now contains only type definitions and basic utilities
  - `pricer.rs` contains all pricing logic and business rules
  - **Eliminated custom registry pricer** - now uses `GenericDiscountingPricer<Basket>`
  - Added `basket.calculator()` method to centralize calculator creation
  - Metrics use the centralized calculator method
- Added `discount_curve_id` field to specify which discount curve to use for valuation dates
- **Simplified by removing ETF-specific features:**
  - Removed `ReplicationMethod` enum and `replication` field
  - Removed `creation_basket()` method and `CreationRedemptionBasket` struct
  - Removed `ticker`, `name`, `creation_unit_size`, `rebalance_freq`, `tracking_index` fields
  - Made `shares_outstanding` a parameter instead of stored field
  - Removed tracking error functionality
- **Eliminated duplication:**
  - Removed 63-line custom pricer (now just a type alias to generic pricer)
  - Centralized calculator instantiation to avoid repeated `new(config.clone())`
- Flattened structure (no pricing subdirectory)
- Implemented `HasDiscountCurve` trait for consistent curve access

## Migration Checklist

- [ ] Move static engine methods to instrument struct as instance methods
- [ ] Update `impl_instrument!` macro to use instance methods
- [ ] Add `discount_curve_id` field to instrument struct (don't hardcode curve names in pricer)
- [ ] Implement `HasDiscountCurve` trait for consistent curve access
- [ ] **Create separate calculator module** for complex pricing logic
- [ ] **Add `instrument.calculator()` method** to centralize calculator creation
- [ ] **Consider using `GenericDiscountingPricer<T>`** instead of custom pricer
- [ ] Update metric calculators to use centralized calculator
- [ ] Delete engine.rs if no longer needed
- [ ] Consider flattening structure: move pricer.rs up if pricing/ only contains it
- [ ] Update module exports
- [ ] Update registry imports if structure changed
- [ ] Run tests to verify functionality
- [ ] Update documentation
