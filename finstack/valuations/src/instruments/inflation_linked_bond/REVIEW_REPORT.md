# Inflation Linked Bond - Market Standards Review

## Executive Summary

The `InflationLinkedBond` implementation provides a solid foundation with support for major market conventions (TIPS, UK Gilts, etc.), robust parameter handling, and integration with the Finstack ecosystem. However, there are **critical deviations from market standards** in the calculation of **Real Yield** and **Pricing Methodologies** that need to be addressed to ensure accurate pricing and risk metrics. The current implementation confounds Nominal and Real frameworks in a way that will lead to incorrect Yield and Breakeven calculations.

## Critical Issues

### 1. Real Yield Calculation Logic
**Severity: High**

The current `real_yield` method calculates a **Nominal Yield**, not a Real Yield.
- **Current Behavior**: It generates *Nominal* cashflows (adjusted for projected inflation) and solves for the yield that equates these flows to the price. This results in a yield close to the Nominal market yield (e.g., 4-5%).
- **Market Standard**: Real Yield is the internal rate of return of the *unadjusted* (Real) cashflows against the *Inflation-Adjusted* price (or rather, the quoted real price plus real accrued).
    - For TIPS: `Invoice Price = (Real Clean Price + Real Accrued) * Index Ratio`.
    - To find Real Yield, one solves: `(Real Clean Price + Real Accrued) = PV(Real Cashflows, y)`.
- **Fix Required**:
    - Modify `real_yield` to generate **Real Cashflows** (coupon rates and principal without inflation indexation).
    - Ensure the `target_price` is treated correctly. If the input `clean_price` is the Quoted Real Price (standard for TIPS), it should be used directly against Real Cashflows (adding Real Accrued).

### 2. Pricing & Discounting Consistency
**Severity: Medium**

The `npv` method relies on `build_schedule`, which generates **Nominal** cashflows (projected inflation applied).
- **Implication**: This requires the linked `discount_curve_id` to be a **Nominal** Discount Curve.
- **Contradiction**: The module documentation (`mod.rs`) states: *"Inflation-linked bonds are priced using real discount curves"*. It then shows a formula mixing Real Discount Factors with Index Ratios, which is mathematically equivalent to Nominal discounting but conceptually distinct.
- **Risk**: If a user follows the doc and provides a Real Discount Curve ID, the `npv` will be roughly `PV_Nominal / (1 + r_real)^t`, which is incorrect (double counting real rates, missing inflation discounting).
- **Fix Required**:
    - Explicitly document that `npv` calculates **Nominal PV** via Fisher equation projection, requiring a **Nominal Discount Curve**.
    - Alternatively, provide a `npv_real` method that uses Real Discount Curves (if supported by the core library).

### 3. Missing Accrued Interest
**Severity: Medium**

The `real_yield` solver uses `clean_price` as the full target PV (`target_price`).
- **Issue**: `Dirty Price = Clean Price + Accrued Interest`. Solving for yield using Clean Price as the total value ignores the accrued component, leading to yield errors (magnitude depends on proximity to coupon dates).
- **Fix Required**: Calculate accrued interest (Real Accrued for Real Yield, Nominal Accrued for Nominal Yield) and add to the Clean Price to get the Target Dirty Price for the solver.

### 4. Breakeven Inflation Calculation
**Severity: High (Derived)**

Because `real_yield` is currently calculating a proxy for Nominal Yield (due to Issue #1), the `breakeven_inflation` calculation:
```rust
nominal_bond_yield - real_yield
```
will result in a value near zero (Nominal - Nominal), rather than the expected inflation expectation (Nominal - Real).

## Minor Issues & Suggestions

1.  **UK Gilt Lag Default**: The `IndexationMethod::UK` defaults to an 8-month lag. Modern UK Index-Linked Gilts (issued post-2005) typically use a 3-month lag. Consider updating the default or explicitly naming them `UK_Old` / `UK_New` (or standardizing to 3 months as the modern default).
2.  **Hardcoded Interpolation Checks**: The validation in `index_ratio` enforces `Linear` for TIPS and `Step` for UK. While generally correct, some users might want to override this for proxying or hypothetical scenarios. Consider making this a warning or soft validation rather than a hard error.
3.  **Metrics Naming**: `Inflation01` and `InflationConvexity` are correctly implemented as Nominal PV sensitivities.

## Recommendations for Remediation

### Step 1: Implement `build_real_schedule`
Add a method to generate unadjusted real cashflows.

```rust
pub fn build_real_schedule(&self, as_of: Date) -> Result<DatedFlows> {
    // Similar to build_schedule but ratio is always 1.0
    // Used for Real Yield calculation
}
```

### Step 2: Fix `real_yield`
Update the method to use real flows and handle accrued interest.

```rust
pub fn real_yield(&self, real_clean_price: f64, curves: &MarketContext, as_of: Date) -> Result<f64> {
    // 1. Get Real Cashflows
    let real_flows = self.build_real_schedule(as_of)?;
    
    // 2. Calculate Real Accrued Interest (needed for Dirty Price)
    // (Use generic bond accrued calculator on real_flows)
    let real_accrued = ...; 
    
    // 3. Target Real Dirty Price
    let real_dirty_price = Money::new(
        (real_clean_price + real_accrued) / 100.0 * self.notional.amount(),
        self.notional.currency()
    );

    // 4. Solve YTM using Real Flows and Real Dirty Price
    solve_ytm(&real_flows, as_of, real_dirty_price, spec)
}
```

### Step 3: Update Documentation
Clarify in `mod.rs` and `npv` that the primary pricing method is **Nominal Projection** (Nominal Flows discounted on Nominal Curve).

## Compliance Verdict
**Current Status**: **70% Market Standard**
- **Structure/Types**: ✅ Excellent
- **Indexation Logic**: ✅ Good (Minor UK lag note)
- **Pricing (NPV)**: ⚠️ Valid only with Nominal Curves (Documentation ambiguous)
- **Yield Metrics**: ❌ Incorrect (Calculates Nominal instead of Real)
- **Risk Metrics**: ✅ Correct (Inflation01)

**Action Plan**: Prioritize fixing `real_yield` and `breakeven_inflation` to unlock correct risk analysis.

