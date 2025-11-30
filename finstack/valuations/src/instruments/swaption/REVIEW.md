# Code Review: Swaption Implementation

## Summary
The swaption implementation in `finstack/valuations/src/instruments/swaption` is generally well-structured, readable, and follows many idiomatic Rust patterns. The separation of concerns between data types (`types.rs`), pricing logic (`pricer.rs`, `pricing/`), and metrics (`metrics/`) is clean. The use of builder patterns and clear factory methods (`new_payer`, `new_receiver`) enhances API ergonomics.

However, there are significant performance concerns in the Bermudan swaption tree valuator due to excessive allocations in the inner loop. Additionally, there are some opportunities to improve type safety and reduce code duplication.

## Critical Issues
*   **Performance Bottleneck in Tree Valuator**: In `pricing/tree_valuator.rs`, the `exercise_value` method allocates new `Vec<f64>`s for `remaining_payment_times` and `remaining_accruals` at **every single node** of the tree during backward induction.
    *   **Location**: `pricing/tree_valuator.rs:156-169`
    *   **Impact**: For a tree with 50 steps, there are ~1,275 nodes. For 100 steps, ~5,000 nodes. Allocating vectors at each node will cause significant GC pressure (if this were a GC language) or allocator thrashing in Rust, severely impacting performance.
    *   **Fix**: Pass slices or indices to `forward_swap_rate` and `annuity` instead of filtering and collecting new vectors. Alternatively, pre-calculate the index of the first remaining payment for each tree step.

## Refactoring Suggestions

### 1. Optimize `exercise_value` in `tree_valuator.rs`
Avoid allocations in the hot path.

```rust
// In pricing/tree_valuator.rs

impl<'a> BermudanSwaptionTreeValuator<'a> {
    // Pre-calculate first payment index for each step in `new` or similar
    // ...

    fn exercise_value(&self, step: usize, node_idx: usize) -> f64 {
        let t = self.tree.time_at_step(step);

        // OPTIMIZATION: Find start index without allocating
        // Assuming payment_times is sorted (which it should be)
        let start_idx = self.payment_times.partition_point(|&pt| pt <= t);
        
        if start_idx >= self.payment_times.len() {
            return 0.0;
        }

        let remaining_payment_times = &self.payment_times[start_idx..];
        let remaining_accruals = &self.accrual_fractions[start_idx..];

        // ... pass slices to calculation methods ...
    }
}
```

### 2. Reduce Cloning in `Swaption::to_european`
The `to_european` method clones many fields. While not critical for a single call, it's unnecessary overhead if called frequently (e.g., during calibration).

```rust
// In types.rs

pub fn to_european(&self) -> Result<Swaption> {
    // ...
    Ok(Swaption {
        // ...
        // Use cheap clones where possible or consider if ownership transfer is needed
        // For strings/IDs, clone is inevitable unless we use Cow or shared references
        discount_curve_id: self.discount_curve_id.clone(), 
        // ...
    })
}
```
*Note: This is less critical than the tree valuator issue.*

### 3. Unified Pricing Logic
`Swaption::price_black` and `Swaption::price_normal` share very similar structure (get time, forward, annuity, then apply formula).

```rust
// In types.rs

fn price_model<F>(&self, disc: &dyn Discounting, vol: f64, as_of: Date, model_fn: F) -> Result<Money>
where F: Fn(f64, f64, f64, f64, f64) -> f64 // forward, strike, vol, t, annuity -> value
{
    let t = self.year_fraction(as_of, self.expiry, self.day_count)?;
    if t <= 0.0 { return Ok(Money::zero(self.notional.currency())); }
    
    let fwd = self.forward_swap_rate(disc, as_of)?;
    let annuity = self.annuity(disc, as_of, fwd)?;
    
    let val = model_fn(fwd, self.strike_rate, vol, t, annuity);
    Ok(Money::new(val * self.notional.amount(), self.notional.currency()))
}
```

## API Polish

### 1. `BermudanSchedule` Construction
The `new` method takes `Vec<Date>` and sorts it. It might be better to validate that it's sorted or return a `Result` if invalid, rather than silently sorting (though sorting is safe).
More importantly, `co_terminal` logic is complex. Consider adding a builder for `BermudanSchedule` if options grow (lockout, notice days, etc. are already there).

### 2. `SwaptionParams` Usage
`SwaptionParams` is a good DTO, but `Swaption::new_payer` takes individual IDs (`discount_curve_id`, etc.) alongside it.
Consider bundling the IDs into a `MarketDataParams` or similar if they are always passed together, or include them in `SwaptionParams` if they are intrinsic to the definition (though usually they are model/market mapping, so keeping them separate is defensible).

### 3. `VolatilityModel` Display
The `Display` impl uses lowercase "black", "normal". Ensure this matches serialization requirements or user expectations.

## Safety & Panics
*   `Swaption::example()` and `BermudanSwaption::example()` use `expect`. This is acceptable for example constructors but ensure these never fail in practice (they use hardcoded valid dates, so it's fine).
*   No `unsafe` blocks found.

## Idiomatic Rust
*   **Iterators**: `Swaption::swap_annuity` uses a manual loop `for &d in &dates[1..]`. This could be a `windows(2)` iterator or `zip` to be more idiomatic.
    ```rust
    // types.rs:557
    for window in dates.windows(2) {
        let prev = window[0];
        let d = window[1];
        // ...
    }
    ```
*   **Error Handling**: Good use of `Result` and custom `Error` types.

## Conclusion
The code is solid but the tree valuator needs immediate optimization to be production-ready for larger trees or batch processing. The rest of the codebase is high quality.
