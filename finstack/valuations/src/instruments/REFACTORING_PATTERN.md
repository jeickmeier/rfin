# Instrument Pricing Refactoring Pattern

## Overview

This document describes the simplified pricing pattern implemented for the BasisSwap instrument and how to apply it to other instruments.

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

- **Direct Methods** (like BasisSwap): When the instrument has specific pricing logic and needs access to its fields
- **Generic Pricer**: For simple instruments with standard discounting patterns
- **Keep Engine**: Only if the logic is truly reusable across multiple unrelated instruments

## Migration Checklist

- [ ] Move static engine methods to instrument struct as instance methods
- [ ] Update `impl_instrument!` macro to use instance methods
- [ ] Simplify or replace pricer implementation
- [ ] Update metric calculators to use instance methods
- [ ] Delete engine.rs if no longer needed
- [ ] Consider flattening structure: move pricer.rs up if pricing/ only contains it
- [ ] Update module exports
- [ ] Update registry imports if structure changed
- [ ] Run tests to verify functionality
- [ ] Update documentation
