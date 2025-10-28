<!-- cf6299a0-4e4c-4425-bd08-c7d0cc653a9f 529ae467-ba44-42ad-bb24-bdfcc926a7f0 -->
# Consolidate MC and Math Module Overlap

## Overview

The `finstack/core/src/math/` and `finstack/valuations/src/instruments/common/mc/` modules have some overlapping functionality. This plan identifies the overlaps and proposes consolidation to eliminate duplication while preserving performance-critical MC-specific implementations.

## Overlap Analysis

### 1. **Statistics Functions**

**Current State:**

- `core/math/stats.rs`: Batch operations (`mean`, `variance`, `covariance`, `correlation`) using Kahan summation + Welford's algorithm
- `mc/stats.rs`: `OnlineStats` struct with Welford's algorithm for streaming accumulation, merge capability, and confidence intervals

**Assessment:** ✅ **No consolidation needed**

- Different use cases: batch (math) vs. streaming (mc)
- `OnlineStats` is MC-specific for incremental path accumulation
- Math functions are general-purpose utilities

### 2. **Random Number Generation**

**Current State:**

- `core/math/random.rs`: `RandomNumberGenerator` trait + `SimpleRng` (LCG-based, basic)
- `mc/rng/`: `PhiloxRng` (counter-based, splittable), `SobolRng` (QMC), Box-Muller transforms

**Assessment:** ✅ **No consolidation needed**

- `SimpleRng` is intentionally basic for testing/demos
- MC RNGs are production-grade, specialized for parallel simulation
- Already using `finstack_core::math::RandomNumberGenerator` trait in MC tests

### 3. **Normal Distribution Functions**

**Current State:**

- `core/math/special_functions.rs`: 
- `norm_cdf()`, `norm_pdf()`, `erf()`, `standard_normal_inv_cdf()` (comprehensive, tail-optimized)
- `mc/rng/transforms.rs`:
- `inverse_normal_cdf()` (Beasley-Springer-Moro approximation)

**Assessment:** ⚠️ **Consolidation recommended**

- **DUPLICATION**: `inverse_normal_cdf()` in mc/transforms vs. `standard_normal_inv_cdf()` in core/math
- Core version has better tail handling and extreme value support
- MC already uses `finstack_core::math::norm_cdf` in some places (asian.rs, qe_heston.rs)

**Action:**

1. Replace `mc/rng/transforms.rs::inverse_normal_cdf()` with `finstack_core::math::standard_normal_inv_cdf()`
2. Update MC code to consistently use core/math for all normal distribution functions

### 4. **Box-Muller Transform**

**Current State:**

- Only in `mc/rng/transforms.rs`: `box_muller_transform()`, `box_muller_polar()`

**Assessment:** ✅ **Keep in MC**

- MC-specific optimization for generating normal samples
- Not needed in general math utilities
- Could potentially move to `core/math/random.rs` if other modules need it, but not urgent

### 5. **Cholesky Decomposition & Correlation**

**Current State:**

- Only in `mc/process/correlation.rs`: `cholesky_decomposition()`, `apply_correlation()`, correlation matrix utilities

**Assessment:** 🔄 **Consider moving to core/math**

- General-purpose linear algebra (not MC-specific)
- Could be useful in other contexts (portfolio optimization, factor models)
- Would fit well in a new `core/math/linalg.rs` module

**Action:**

1. Create `core/math/linalg.rs` with Cholesky decomposition and correlation utilities
2. Re-export from `mc/process/correlation.rs` for backward compatibility
3. Consider adding QR decomposition (mentioned in MC README for LSMC robustness)

### 6. **Moment Matching**

**Current State:**

- Only in `mc/rng/transforms.rs::moment_match()` - variance reduction technique

**Assessment:** ✅ **Keep in MC**

- Specific to Monte Carlo variance reduction
- Not a general-purpose utility

### 7. **Summation Functions**

**Current State:**

- `core/math/summation.rs`: `kahan_sum()`, `pairwise_sum()`, `stable_sum()`
- MC implicitly uses these via `finstack_core::math::stats` (which uses Kahan)

**Assessment:** ✅ **Already consolidated**

- MC correctly depends on core/math for summation
- No duplication found

## Consolidation Plan

### Phase 1: Normal Distribution Functions (High Priority)

**Goal:** Eliminate duplicate inverse normal CDF implementations

**Changes:**

1. Update `mc/rng/transforms.rs`:

- Remove local `inverse_normal_cdf()` function
- Add `use finstack_core::math::standard_normal_inv_cdf;`
- Add type alias or re-export for backward compatibility if needed

2. Update MC documentation to reference core/math for normal functions

3. Add tests to verify parity between old and new implementations

### Phase 2: Cholesky & Correlation (Medium Priority)

**Goal:** Make linear algebra utilities available across codebase

**Changes:**

1. Create `core/math/linalg.rs`:

- Move `cholesky_decomposition()` from mc/process/correlation.rs
- Move `apply_correlation()`, `build_correlation_matrix()`, `validate_correlation_matrix()`
- Keep all existing tests

2. Update `mc/process/correlation.rs`:

- Re-export from `finstack_core::math::linalg`
- Maintain backward compatibility for MC code

3. Update `core/math/mod.rs` to expose linalg module

### Phase 3: Documentation & Guidelines (Low Priority)

**Goal:** Prevent future duplication

**Changes:**

1. Document in `.cursor/rules/rust/code-standards.mdc`:

- Use `finstack_core::math::*` for general-purpose math functions
- Keep MC-specific optimizations in `mc/` (Philox RNG, variance reduction, etc.)
- Before adding math utilities to MC, check if they belong in core/math

2. Update `mc/README.md` to clarify dependency on core/math

## What Stays in MC Module

The following are genuinely MC-pricing-specific and should remain in `mc/`:

1. **Simulation Infrastructure**:

- `RandomStream` trait (MC-specific streaming interface)
- `StochasticProcess` trait and implementations (GBM, Heston, OU)
- `Discretization` trait and schemes (Euler, Milstein, QE)
- `TimeGrid` for path simulation

2. **Pricing Components**:

- `Payoff` trait and implementations (European, Asian, Barrier, Lookback)
- `McEngine` and pricing engines (European, PathDependent, LSMC)
- Pricer configurations

3. **MC-Specific Techniques**:

- Variance reduction strategies (antithetic, control variates, importance sampling)
- Greeks computation (pathwise, LRM, finite difference)
- Barrier corrections (Brownian bridge, Gobet-Miri)
- LSMC basis functions and regression

4. **Results & State**:

- `MoneyEstimate`, `Estimate` (MC-specific result types)
- `PathState` trait

## Testing Strategy

For each consolidation:

1. Run existing MC integration tests (`mc_v01_integration.rs`, `mc_v02_integration.rs`, `mc_v03_integration.rs`)
2. Run MC benchmarks to ensure no performance regression
3. Add property tests for numerical equivalence where applicable
4. Run `make lint` and `make test` across workspace

## Benefits

1. **Eliminate Duplication**: Single source of truth for normal distribution functions
2. **Better Tail Handling**: MC inherits core/math's enhanced extreme value support
3. **Broader Reusability**: Cholesky decomposition available for portfolio/scenarios
4. **Maintainability**: One place to improve numerical stability
5. **Consistency**: All crates use same math implementations

### To-dos

- [ ] Identify all uses of inverse_normal_cdf in MC module to ensure safe replacement
- [ ] Replace mc/rng/transforms.rs::inverse_normal_cdf with finstack_core::math::standard_normal_inv_cdf
- [ ] Add tests verifying numerical parity between old and new inverse normal implementations
- [ ] Create core/math/linalg.rs with Cholesky decomposition and correlation utilities
- [ ] Move Cholesky functions from mc/process/correlation.rs to core/math/linalg.rs
- [ ] Update mc/process/correlation.rs to re-export from core/math/linalg
- [ ] Update code standards and MC README with consolidation guidelines
- [ ] Run full test suite including MC integration tests and benchmarks