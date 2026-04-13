# Swaption Volatility Cube (3D) Design

**Date:** 2026-04-12
**Status:** Approved

## Problem

The `VolSurface` is 2D (expiry x strike). Rates desks need a third dimension (underlying swap tenor) for swaption trading. The swaption calibration already produces SABR parameters per (expiry, tenor) bucket, but there is no storage/interpolation structure that can answer "give me the vol for a 5Y expiry, 10Y tenor swaption at strike K" by interpolating all three dimensions simultaneously.

**Current state:**
- `VolSurface` (core): 2D grid with bilinear interpolation. `VolSurfaceAxis` enum distinguishes Strike vs Tenor semantics, but the surface is always 2D.
- `SwaptionVolTarget::solve` (valuations): Calibrates SABR params per (expiry, tenor), then materializes a 2D ATM surface with axes (expiry, tenor) -- no strike dimension in the output.
- `SimpleSwaptionBlackPricer`: Looks up vol from a surface as (time_to_expiry, strike) -- no tenor awareness.

## Approach

**SABR-native VolCube** -- store calibrated `SabrParams` on an (expiry, tenor) grid. Evaluate the Hagan smile at any strike on demand. Optionally materialize 2D slices for MC or caching.

Chosen over a model-agnostic parametric cube because SABR is the industry standard for swaptions and is what the calibration already produces. Non-SABR models can be added later as additional concrete types implementing the same `VolProvider` trait.

## Design

### 1. SabrParams Shift Support

Add `shift: Option<f64>` to the existing `SabrParams` in `finstack/core/src/math/volatility/sabr.rs`. This aligns core's `SabrParams` with valuations' `SABRParameters` (which already has shift) and supports negative rate environments (EUR, JPY, CHF).

The Hagan formula implementations (`implied_vol_lognormal`, `implied_vol_normal`) are updated to apply `F + shift` and `K + shift` when `shift` is `Some`.

**Serde:** `shift` defaults to `None` for backward compatibility with existing serialized payloads.

### 2. VolCube Data Structure

**File:** `finstack/core/src/market_data/surfaces/vol_cube.rs`

```rust
pub struct VolCube {
    id: CurveId,
    expiries: Box<[f64]>,           // option expiry axis (year fractions), sorted ascending
    tenors: Box<[f64]>,             // underlying swap tenor axis (year fractions), sorted ascending
    params: Vec<SabrParams>,        // row-major: params[exp_idx * n_tenors + tenor_idx]
    forwards: Vec<f64>,             // forward swap rates, same layout as params
    interpolation_mode: VolInterpolationMode,  // for materialized slices
}
```

**Storage layout:** `params` and `forwards` are parallel arrays, both `n_expiries * n_tenors` long in row-major order. `forwards` are stored because the Hagan formula requires the forward rate as input.

**Construction:**

```rust
// Builder pattern (matches VolSurface convention)
let cube = VolCube::builder("USD-SWAPTION-VOL")
    .expiries(&[0.5, 1.0, 2.0, 5.0, 10.0])
    .tenors(&[1.0, 2.0, 5.0, 10.0, 30.0])
    .node(sabr_params_0, forward_0)   // (0.5Y, 1Y) node
    .node(sabr_params_1, forward_1)   // (0.5Y, 2Y) node
    // ... row-major order
    .build()?;

// Or from pre-built arrays:
let cube = VolCube::from_grid(id, &expiries, &tenors, &params, &forwards)?;
```

**Validation on construction:**
- `expiries` and `tenors` must be sorted, non-empty, and strictly increasing
- `params.len() == expiries.len() * tenors.len()`
- `forwards.len() == params.len()`
- All SABR params valid (alpha > 0, beta in [0,1], rho in (-1,1), nu > 0)
- All forwards finite

### 3. VolProvider Trait

**File:** `finstack/core/src/market_data/traits.rs` (add to existing traits module)

```rust
pub trait VolProvider: Send + Sync {
    /// Look up implied vol at (expiry, tenor, strike).
    /// For 2D surfaces: tenor is ignored.
    fn vol(&self, expiry: f64, tenor: f64, strike: f64) -> Result<f64>;

    /// Same as vol() but clamps out-of-bounds coordinates to grid edges.
    fn vol_clamped(&self, expiry: f64, tenor: f64, strike: f64) -> f64;

    /// The identifier.
    fn id(&self) -> &CurveId;
}
```

**Implementations:**
- `VolCube` -- bilinear SABR param interpolation across (expiry, tenor), then Hagan eval at strike. SABR evaluates any strike natively, so there is no strike-grid boundary issue.
- `VolSurface` -- implements `VolProvider` directly (not a separate adapter struct). The `tenor` parameter is ignored; `vol()` delegates to `value_checked(expiry, strike)` and `vol_clamped()` to `value_clamped(expiry, strike)`.

### 4. SABR Parameter Interpolation

When a query point (expiry, tenor) falls between calibrated grid nodes:

1. Locate the bounding cell in the (expiry, tenor) grid
2. Bilinear-interpolate each parameter independently:
   - `alpha`, `rho`, `nu`, `forward` -- standard bilinear
   - `beta` -- held constant (convention-driven, typically 0.5 for rates)
   - `shift` -- bilinear if present at all four corners; if mixed (some nodes shifted, some not), use the shift from the nearest node that has one, or `None` if no corners have shift
3. Post-interpolation clamp to enforce SABR bounds:
   - `alpha > 0` (floor at 1e-8)
   - `nu > 0` (floor at 1e-8)
   - `rho` clamp to (-0.9999, 0.9999)
   - `beta` clamp to [0, 1]
4. Evaluate Hagan formula at the interpolated (params, forward, strike, expiry)

**Extrapolation:** Clamp to grid boundary (flat extrapolation in both expiry and tenor dimensions). Strike extrapolation is SABR-native -- the Hagan formula extends beyond ATM naturally.

### 5. Grid Materialization

```rust
impl VolCube {
    /// Materialize a 2D VolSurface for a specific tenor.
    /// Interpolates SABR params to the target tenor, evaluates the smile
    /// at each (expiry, strike) on the given strike grid.
    pub fn materialize_tenor_slice(
        &self,
        tenor: f64,
        strikes: &[f64],
    ) -> Result<VolSurface>;

    /// Materialize a 2D VolSurface for a specific expiry.
    pub fn materialize_expiry_slice(
        &self,
        expiry: f64,
        strikes: &[f64],
    ) -> Result<VolSurface>;

    /// Materialize the full 3D grid at given strikes.
    /// Returns vols as a flat Vec in (expiry, tenor, strike) order.
    pub fn materialize_grid(
        &self,
        strikes: &[f64],
    ) -> Result<Vec<f64>>;
}
```

The tenor slice is the primary use case -- a trader picks a tenor (e.g., 10Y underlying) and wants a standard (expiry, strike) surface. The returned `VolSurface` has `VolSurfaceAxis::Strike` and works with existing vol surface tooling.

### 6. MarketContext Integration

**New field in `MarketContext`:**

```rust
vol_cubes: HashMap<CurveId, Arc<VolCube>>,
```

**New methods:**

```rust
// Owned-builder style (matches existing pattern)
pub fn insert_vol_cube(mut self, cube: impl Into<Arc<VolCube>>) -> Self;
pub fn insert_vol_cube_mut(&mut self, cube: impl Into<Arc<VolCube>>) -> &mut Self;
pub fn get_vol_cube(&self, id: impl AsRef<str>) -> Result<Arc<VolCube>>;

/// Try cube first, then surface. Returns a trait object.
pub fn get_vol_provider(&self, id: impl AsRef<str>) -> Result<Arc<dyn VolProvider>>;
```

`get_vol_provider` searches cubes first, then surfaces. This lets the swaption pricer transparently accept either.

**Serialization:** `MarketContextState` gets `vol_cubes: Vec<VolCube>`, matching the pattern for surfaces and FX delta surfaces. Serde derives on `VolCube` use a `RawVolCube` intermediate (same pattern as `VolSurface`).

**Stats/Debug/Roll:** Updated to include vol_cubes in counts, `is_empty()`, `total_items()`, `Debug` output, and `roll_forward`.

### 7. Calibration Output Change

`SwaptionVolTarget::solve` currently produces a 2D `VolSurface`. Updated to produce a `VolCube`:

```rust
// Replace the surface construction at the tail of solve():
let cube = VolCube::from_grid(
    &params.surface_id,
    &target_expiries,
    &target_tenors,
    &sabr_grid_params,    // Vec<SabrParams> from calibrated BTreeMap
    &sabr_grid_forwards,  // Vec<f64> from forward rate calculations
)?;
```

The calibrated `sabr_params: BTreeMap<(u64, u64), SABRParameters>` intermediate already exists. The conversion maps each `SABRParameters` (valuations) to `SabrParams` (core) and collects forwards from the existing `calculate_forward_swap_rate_years` calls.

### 8. Swaption Pricer Integration

**Current flow:**
```
Swaption -> pricer -> market.get_surface(id) -> vol_surface.value_clamped(expiry, strike)
```

**New flow:**
```
Swaption -> pricer -> market.get_vol_provider(id) -> provider.vol_clamped(expiry, tenor, strike)
```

Changes to `SimpleSwaptionBlackPricer::price_dyn`:
1. Call `market.get_vol_provider(swaption.vol_surface_id)` instead of `market.get_surface()`
2. Compute `underlying_tenor` from the swaption's swap schedule (expiry to maturity)
3. Pass `(time_to_expiry, underlying_tenor, strike)` to the `VolProvider`
4. Extrapolation handling: `Clamp` calls `vol_clamped()`, `Error` calls `vol()` (returns error if out-of-bounds). `LinearInVariance` is VolSurface-specific (strike-grid edge extrapolation); for VolCube, SABR evaluates any strike natively so LinearInVariance is a no-op -- the pricer falls through to standard `vol()` when a cube is the provider

**Backward compatibility:** If the market context only has a `VolSurface` (no cube), `get_vol_provider()` returns the surface wrapped in the adapter. Existing tests pass unchanged.

### 9. Python Bindings

**File:** `finstack-py/src/bindings/core/market_data/curves.rs` (alongside PyVolSurface)

```python
class VolCube:
    def __init__(
        self,
        id: str,
        expiries: list[float],
        tenors: list[float],
        params_row_major: list[dict],  # [{"alpha": ..., "beta": ..., "rho": ..., "nu": ..., "shift": ...}]
        forwards_row_major: list[float],
    ) -> None: ...

    def vol(self, expiry: float, tenor: float, strike: float) -> float: ...
    def vol_clamped(self, expiry: float, tenor: float, strike: float) -> float: ...
    def materialize_tenor_slice(self, tenor: float, strikes: list[float]) -> VolSurface: ...
    def materialize_expiry_slice(self, expiry: float, strikes: list[float]) -> VolSurface: ...

    @property
    def id(self) -> str: ...
    @property
    def expiries(self) -> list[float]: ...
    @property
    def tenors(self) -> list[float]: ...
    @property
    def grid_shape(self) -> tuple[int, int]: ...
```

**PyO3 pattern:** Matches existing `PyVolSurface`. SABR params passed as list of dicts for ergonomic Python construction. Returns `PyVolSurface` from materialization methods.

### 10. WASM Bindings

**File:** `finstack-wasm/src/api/core/market_data.rs`

```typescript
class VolCube {
    constructor(
        id: string,
        expiries: Float64Array,
        tenors: Float64Array,
        params_flat: Float64Array,   // [alpha0, beta0, rho0, nu0, shift0, alpha1, ...]
        forwards: Float64Array,
    );
    vol(expiry: number, tenor: number, strike: number): number;
    vol_clamped(expiry: number, tenor: number, strike: number): number;
    materialize_tenor_slice(tenor: number, strikes: Float64Array): VolSurface;
}
```

SABR params flattened to Float64Array (5 values per node: alpha, beta, rho, nu, shift) for efficient WASM transfer. NaN sentinel for no-shift.

### 11. Testing Strategy

1. **Unit tests (vol_cube.rs):**
   - Construction validation (sorted axes, correct lengths, valid SABR params)
   - Vol query at exact grid nodes matches direct Hagan eval (zero interpolation error)
   - Bilinear param interpolation at grid midpoints
   - Boundary clamping (flat extrapolation in expiry/tenor)
   - Shift handling (shifted SABR eval, shift interpolation)

2. **Interpolation accuracy:**
   - Vol at grid midpoint vs. average of surrounding nodes -- bounded error
   - Monotonicity in alpha direction preserves vol level ordering

3. **Materialization round-trip:**
   - `materialize_tenor_slice(t, strikes)` produces a VolSurface
   - Surface queries match direct cube queries at the same (expiry, strike) for the given tenor

4. **VolProvider trait:**
   - VolCube and VolSurface both implement VolProvider
   - VolSurface adapter ignores tenor -- same results as direct surface query

5. **Pricer backward compatibility:**
   - Existing swaption pricing tests pass with VolSurface (no cube in market context)
   - Same tests pass when VolCube is present instead

6. **Calibration integration:**
   - `SwaptionVolTarget::solve` produces a VolCube
   - ATM vols from cube match legacy 2D surface ATM vols

7. **Python binding tests:**
   - Construct PyVolCube from dict params, query vol, materialize slice
   - Round-trip serialization

8. **WASM binding tests:**
   - Construct from flat arrays, query vol

## Files Changed

### New files:
- `finstack/core/src/market_data/surfaces/vol_cube.rs` -- VolCube struct, builder, interpolation, materialization
- `finstack/core/tests/market_data/surfaces/vol_cube_tests.rs` -- unit tests
- `finstack-wasm/src/api/core/vol_cube.rs` -- WASM VolCube bindings

### Modified files:
- `finstack/core/src/math/volatility/sabr.rs` -- add `shift: Option<f64>` to SabrParams, update Hagan formulas
- `finstack/core/src/market_data/surfaces/mod.rs` -- export VolCube
- `finstack/core/src/market_data/traits.rs` -- add VolProvider trait
- `finstack/core/src/market_data/context/mod.rs` -- add vol_cubes field
- `finstack/core/src/market_data/context/insert.rs` -- insert_vol_cube methods
- `finstack/core/src/market_data/context/getters.rs` -- get_vol_cube, get_vol_provider
- `finstack/core/src/market_data/context/state_serde.rs` -- MarketContextState vol_cubes field
- `finstack/core/src/market_data/context/stats.rs` -- include vol_cubes in stats
- `finstack/core/src/market_data/context/ops_roll.rs` -- clone vol_cubes in roll_forward
- `finstack/valuations/src/calibration/targets/swaption.rs` -- output VolCube instead of VolSurface
- `finstack/valuations/src/instruments/rates/swaption/pricer.rs` -- use VolProvider trait
- `finstack-py/src/bindings/core/market_data/curves.rs` -- PyVolCube class
- `finstack-py/finstack/core/market_data.pyi` -- VolCube type stubs
- `finstack-wasm/src/api/core/market_data.rs` -- WASM VolCube bindings

## Non-Goals

- Non-SABR smile models (SVI, polynomial). Can be added as separate VolProvider implementations later.
- Caching of materialized slices. Callers can cache the returned VolSurface externally.
- Arbitrage-free interpolation across the cube. Bilinear on SABR params is the pragmatic starting point; variance-preserving or arbitrage-free methods are future work.
