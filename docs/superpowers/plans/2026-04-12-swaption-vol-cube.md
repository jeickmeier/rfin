# Swaption Volatility Cube Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a 3D `VolCube` type that stores SABR parameters on an (expiry, tenor) grid with on-demand smile evaluation, plus a `VolProvider` trait for polymorphic vol lookup in swaption pricing.

**Architecture:** `VolCube` lives in `finstack/core/market_data/surfaces/` alongside `VolSurface`. It stores `SabrParams` (extended with shift support) on a rectangular grid. A `VolProvider` trait unifies vol lookup so the swaption pricer can accept either a VolSurface or VolCube. Calibration produces a VolCube directly. Python and WASM bindings expose the full API.

**Tech Stack:** Rust (finstack-core, finstack-valuations), PyO3 (finstack-py), wasm-bindgen (finstack-wasm)

**Spec:** `docs/superpowers/specs/2026-04-12-swaption-vol-cube-design.md`

---

## File Map

### New files
| File | Purpose |
|------|---------|
| `finstack/core/src/market_data/surfaces/vol_cube.rs` | VolCube struct, builder, SABR param interpolation, smile evaluation, materialization |
| `finstack/core/tests/market_data/surfaces/vol_cube_tests.rs` | Unit tests for VolCube |

### Modified files
| File | Change |
|------|--------|
| `finstack/core/src/math/volatility/sabr.rs` | Add `shift: Option<f64>` to `SabrParams`, update Hagan formulas |
| `finstack/core/src/market_data/surfaces/mod.rs` | Export `VolCube`, `VolCubeBuilder` |
| `finstack/core/src/market_data/traits.rs` | Add `VolProvider` trait |
| `finstack/core/src/market_data/context/mod.rs` | Add `vol_cubes` field to `MarketContext` |
| `finstack/core/src/market_data/context/insert.rs` | `insert_vol_cube`, `insert_vol_cube_mut` |
| `finstack/core/src/market_data/context/getters.rs` | `get_vol_cube`, `get_vol_provider` |
| `finstack/core/src/market_data/context/state_serde.rs` | `vol_cubes` in `MarketContextState`, serde impls |
| `finstack/core/src/market_data/context/stats.rs` | `vol_cube_count` in stats, update `is_empty`, `total_objects`, `contains`, `Display` |
| `finstack/core/src/market_data/context/ops_roll.rs` | Clone vol_cubes in `roll_forward` |
| `finstack/valuations/src/calibration/targets/swaption.rs` | Output `VolCube` from calibration |
| `finstack/valuations/src/instruments/rates/swaption/pricer.rs` | Use `VolProvider` for vol lookup |
| `finstack-py/src/bindings/core/market_data/curves.rs` | `PyVolCube` class |
| `finstack-py/finstack/core/market_data.pyi` | `VolCube` type stubs |
| `finstack-wasm/src/api/core/market_data.rs` | WASM `VolCube` bindings |

---

## Task 1: Add shift support to SabrParams

**Files:**
- Modify: `finstack/core/src/math/volatility/sabr.rs:76-86` (struct), `:126-160` (constructor), `:175-238` (lognormal), `:251-320` (normal)

- [ ] **Step 1: Write failing test for shifted SabrParams construction**

In `finstack/core/src/math/volatility/sabr.rs`, add to the existing `#[cfg(test)]` module at the bottom of the file:

```rust
#[test]
fn test_sabr_params_with_shift() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    assert!(p.shift.is_none());

    let shifted = p.with_shift(0.03);
    assert_eq!(shifted.shift, Some(0.03));
}

#[test]
fn test_shifted_sabr_implied_vol_lognormal() {
    // Shifted SABR: evaluate with F+shift, K+shift
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4)
        .unwrap()
        .with_shift(0.03);
    let f = -0.005; // negative forward
    let k = 0.01;
    let t = 1.0;
    let vol = p.implied_vol_lognormal(f, k, t);
    assert!(vol.is_finite() && vol > 0.0, "shifted SABR should handle negative rates");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p finstack-core --lib -E 'test(sabr_params_with_shift)' --no-capture`
Expected: FAIL — `shift` field and `with_shift` method don't exist

- [ ] **Step 3: Add shift field and with_shift method to SabrParams**

In `finstack/core/src/math/volatility/sabr.rs`, modify the `SabrParams` struct (line 77):

```rust
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SabrParams {
    /// Alpha (α): initial volatility level.
    pub alpha: f64,
    /// Beta (β): CEV exponent, in [0, 1].
    pub beta: f64,
    /// Rho (ρ): correlation between forward and vol Brownian motions, in (-1, 1).
    pub rho: f64,
    /// Nu (ν): vol-of-vol, must be > 0.
    pub nu: f64,
    /// Optional shift for negative rate support (shifted SABR).
    /// When present, the Hagan formula evaluates at F+shift, K+shift.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shift: Option<f64>,
}
```

Add after the `new()` constructor (around line 160):

```rust
/// Return a copy with the given shift applied.
#[must_use]
pub fn with_shift(mut self, shift: f64) -> Self {
    self.shift = Some(shift);
    self
}
```

Update the `new()` constructor (line 126) to initialize `shift: None`:

```rust
Ok(Self {
    alpha,
    beta,
    rho,
    nu,
    shift: None,
})
```

- [ ] **Step 4: Update implied_vol_lognormal to handle shift**

In `implied_vol_lognormal` (line 175), add shift handling at the top of the method, after extracting params:

```rust
pub fn implied_vol_lognormal(&self, f: f64, k: f64, t: f64) -> f64 {
    // Apply shift for negative rate support
    let (f, k) = if let Some(s) = self.shift {
        (f + s, k + s)
    } else {
        (f, k)
    };

    let alpha = self.alpha;
    let beta = self.beta;
    let rho = self.rho;
    let nu = self.nu;
    // ... rest of existing implementation unchanged
```

- [ ] **Step 5: Update implied_vol_normal to handle shift**

In `implied_vol_normal` (line 251), add the same shift handling:

```rust
pub fn implied_vol_normal(&self, f: f64, k: f64, t: f64) -> f64 {
    // Apply shift for negative rate support
    let (f, k) = if let Some(s) = self.shift {
        (f + s, k + s)
    } else {
        (f, k)
    };

    let alpha = self.alpha;
    let beta = self.beta;
    let rho = self.rho;
    let nu = self.nu;
    // ... rest of existing implementation unchanged
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo nextest run -p finstack-core --lib -E 'test(sabr)' --no-capture`
Expected: All SABR tests pass including new shift tests

- [ ] **Step 7: Run full core test suite to check for regressions**

Run: `cargo nextest run -p finstack-core --features mc,test-utils --lib --test '*' --no-fail-fast`
Expected: All tests pass (shift defaults to None, so no behavior change for existing code)

- [ ] **Step 8: Commit**

```bash
git add finstack/core/src/math/volatility/sabr.rs
git commit -m "feat(sabr): add shift support to SabrParams for negative rate environments"
```

---

## Task 2: Create VolCube struct with builder and from_grid

**Files:**
- Create: `finstack/core/src/market_data/surfaces/vol_cube.rs`
- Modify: `finstack/core/src/market_data/surfaces/mod.rs:126`

- [ ] **Step 1: Write failing test for VolCube construction**

Create `finstack/core/tests/market_data/surfaces/vol_cube_tests.rs`:

```rust
use finstack_core::market_data::surfaces::VolCube;
use finstack_core::math::volatility::sabr::SabrParams;

#[test]
fn test_vol_cube_builder_basic() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::builder("USD-SWAPTION")
        .expiries(&[1.0, 5.0])
        .tenors(&[2.0, 10.0])
        .node(p, 0.03)
        .node(p, 0.035)
        .node(p, 0.04)
        .node(p, 0.045)
        .build()
        .unwrap();

    assert_eq!(cube.id().as_str(), "USD-SWAPTION");
    assert_eq!(cube.expiries(), &[1.0, 5.0]);
    assert_eq!(cube.tenors(), &[2.0, 10.0]);
    assert_eq!(cube.grid_shape(), (2, 2));
}

#[test]
fn test_vol_cube_from_grid() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let params = vec![p; 4];
    let forwards = vec![0.03, 0.035, 0.04, 0.045];
    let cube = VolCube::from_grid(
        "TEST",
        &[1.0, 5.0],
        &[2.0, 10.0],
        &params,
        &forwards,
    )
    .unwrap();
    assert_eq!(cube.grid_shape(), (2, 2));
}

#[test]
fn test_vol_cube_validation_rejects_bad_input() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    // Wrong number of params
    let result = VolCube::from_grid("BAD", &[1.0, 5.0], &[2.0, 10.0], &[p; 3], &[0.03; 3]);
    assert!(result.is_err());

    // Unsorted expiries
    let result = VolCube::from_grid("BAD", &[5.0, 1.0], &[2.0, 10.0], &[p; 4], &[0.03; 4]);
    assert!(result.is_err());
}
```

Add `mod vol_cube_tests;` to `finstack/core/tests/market_data/surfaces/mod.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p finstack-core --test '*' -E 'test(vol_cube)' --no-capture`
Expected: FAIL — `VolCube` doesn't exist

- [ ] **Step 3: Create vol_cube.rs with struct, builder, and from_grid**

Create `finstack/core/src/market_data/surfaces/vol_cube.rs`:

```rust
//! Swaption volatility cube with SABR parametric interpolation.
//!
//! Stores calibrated SABR parameters on a rectangular (expiry, tenor) grid.
//! Implied volatility at any (expiry, tenor, strike) is computed by bilinear
//! interpolation of SABR parameters followed by Hagan formula evaluation.

use crate::{
    error::InputError,
    math::interp::utils::locate_segment,
    math::volatility::sabr::SabrParams,
    types::CurveId,
    Error,
};

use super::vol_surface::{VolInterpolationMode, VolSurface, VolSurfaceAxis};

/// Swaption volatility cube: SABR parameters on an (expiry, tenor) grid.
///
/// The cube stores calibrated [`SabrParams`] at each grid node along with
/// the corresponding forward swap rate. Volatility at any (expiry, tenor,
/// strike) triple is obtained by bilinear interpolation of SABR parameters
/// across the (expiry, tenor) grid, then evaluating the Hagan formula at
/// the target strike.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawVolCube", into = "RawVolCube")]
pub struct VolCube {
    id: CurveId,
    /// Option expiry axis (year fractions), sorted ascending.
    expiries: Box<[f64]>,
    /// Underlying swap tenor axis (year fractions), sorted ascending.
    tenors: Box<[f64]>,
    /// SABR parameters, row-major: `params[exp_idx * n_tenors + tenor_idx]`.
    params: Vec<SabrParams>,
    /// Forward swap rates at each grid node, same layout as `params`.
    forwards: Vec<f64>,
    /// Interpolation mode for materialized slices.
    interpolation_mode: VolInterpolationMode,
}

/// Raw serializable representation of a VolCube.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVolCube {
    pub id: String,
    pub expiries: Vec<f64>,
    pub tenors: Vec<f64>,
    pub params: Vec<SabrParams>,
    pub forwards: Vec<f64>,
    #[serde(default)]
    pub interpolation_mode: VolInterpolationMode,
}

impl From<VolCube> for RawVolCube {
    fn from(cube: VolCube) -> Self {
        RawVolCube {
            id: cube.id.to_string(),
            expiries: cube.expiries.to_vec(),
            tenors: cube.tenors.to_vec(),
            params: cube.params,
            forwards: cube.forwards,
            interpolation_mode: cube.interpolation_mode,
        }
    }
}

impl TryFrom<RawVolCube> for VolCube {
    type Error = crate::Error;
    fn try_from(raw: RawVolCube) -> crate::Result<Self> {
        VolCube::from_grid(&raw.id, &raw.expiries, &raw.tenors, &raw.params, &raw.forwards)
            .map(|c| c.with_interpolation_mode(raw.interpolation_mode))
    }
}

impl VolCube {
    // ── Construction ────────────────────────────────────────────────────

    /// Start building a new volatility cube.
    pub fn builder(id: impl Into<CurveId>) -> VolCubeBuilder {
        VolCubeBuilder {
            id: id.into(),
            expiries: Vec::new(),
            tenors: Vec::new(),
            params: Vec::new(),
            forwards: Vec::new(),
            interpolation_mode: VolInterpolationMode::Vol,
        }
    }

    /// Construct from pre-built arrays (row-major order).
    pub fn from_grid(
        id: impl AsRef<str>,
        expiries: &[f64],
        tenors: &[f64],
        params: &[SabrParams],
        forwards: &[f64],
    ) -> crate::Result<Self> {
        let expected = expiries.len() * tenors.len();
        if params.len() != expected {
            return Err(Error::Validation(format!(
                "VolCube params length {} != expiries({}) * tenors({})",
                params.len(),
                expiries.len(),
                tenors.len()
            )));
        }
        if forwards.len() != expected {
            return Err(Error::Validation(format!(
                "VolCube forwards length {} != expiries({}) * tenors({})",
                forwards.len(),
                expiries.len(),
                tenors.len()
            )));
        }
        if expiries.is_empty() || tenors.is_empty() {
            return Err(Error::Input(InputError::TooFewPoints));
        }
        // Check sorted ascending
        for w in expiries.windows(2) {
            if w[1] <= w[0] {
                return Err(Error::Validation(format!(
                    "VolCube expiries must be strictly increasing; found {} >= {}",
                    w[0], w[1]
                )));
            }
        }
        for w in tenors.windows(2) {
            if w[1] <= w[0] {
                return Err(Error::Validation(format!(
                    "VolCube tenors must be strictly increasing; found {} >= {}",
                    w[0], w[1]
                )));
            }
        }
        for f in forwards {
            if !f.is_finite() {
                return Err(Error::Validation(
                    "VolCube forwards must be finite".to_string(),
                ));
            }
        }
        Ok(Self {
            id: CurveId::from(id.as_ref()),
            expiries: expiries.into(),
            tenors: tenors.into(),
            params: params.to_vec(),
            forwards: forwards.to_vec(),
            interpolation_mode: VolInterpolationMode::Vol,
        })
    }

    /// Set the interpolation mode for materialized slices.
    #[must_use]
    pub fn with_interpolation_mode(mut self, mode: VolInterpolationMode) -> Self {
        self.interpolation_mode = mode;
        self
    }

    // ── Accessors ───────────────────────────────────────────────────────

    /// Cube identifier.
    pub fn id(&self) -> &CurveId {
        &self.id
    }

    /// Option expiry axis (year fractions).
    pub fn expiries(&self) -> &[f64] {
        &self.expiries
    }

    /// Underlying swap tenor axis (year fractions).
    pub fn tenors(&self) -> &[f64] {
        &self.tenors
    }

    /// Grid dimensions as (n_expiries, n_tenors).
    pub fn grid_shape(&self) -> (usize, usize) {
        (self.expiries.len(), self.tenors.len())
    }

    /// SABR parameters at grid node (expiry_idx, tenor_idx).
    pub fn params_at(&self, expiry_idx: usize, tenor_idx: usize) -> &SabrParams {
        &self.params[expiry_idx * self.tenors.len() + tenor_idx]
    }

    /// Forward rate at grid node (expiry_idx, tenor_idx).
    pub fn forward_at(&self, expiry_idx: usize, tenor_idx: usize) -> f64 {
        self.forwards[expiry_idx * self.tenors.len() + tenor_idx]
    }
}

/// Builder for [`VolCube`].
pub struct VolCubeBuilder {
    id: CurveId,
    expiries: Vec<f64>,
    tenors: Vec<f64>,
    params: Vec<SabrParams>,
    forwards: Vec<f64>,
    interpolation_mode: VolInterpolationMode,
}

impl VolCubeBuilder {
    /// Set the expiry axis.
    pub fn expiries(mut self, expiries: &[f64]) -> Self {
        self.expiries = expiries.to_vec();
        self
    }

    /// Set the tenor axis.
    pub fn tenors(mut self, tenors: &[f64]) -> Self {
        self.tenors = tenors.to_vec();
        self
    }

    /// Add a grid node (SABR params + forward rate) in row-major order.
    pub fn node(mut self, params: SabrParams, forward: f64) -> Self {
        self.params.push(params);
        self.forwards.push(forward);
        self
    }

    /// Set interpolation mode.
    pub fn interpolation_mode(mut self, mode: VolInterpolationMode) -> Self {
        self.interpolation_mode = mode;
        self
    }

    /// Build the VolCube, validating all inputs.
    pub fn build(self) -> crate::Result<VolCube> {
        VolCube::from_grid(&self.id.to_string(), &self.expiries, &self.tenors, &self.params, &self.forwards)
            .map(|c| c.with_interpolation_mode(self.interpolation_mode))
    }
}
```

- [ ] **Step 4: Add module declaration and re-export**

In `finstack/core/src/market_data/surfaces/mod.rs`, add after line 56 (`mod vol_surface;`):

```rust
mod vol_cube;
```

And update the re-exports at line 126:

```rust
pub use vol_cube::{VolCube, VolCubeBuilder};
pub use vol_surface::{VolInterpolationMode, VolSurface, VolSurfaceAxis, VolSurfaceBuilder};
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo nextest run -p finstack-core --features mc,test-utils --lib --test '*' -E 'test(vol_cube)' --no-capture`
Expected: All 3 vol_cube tests pass

- [ ] **Step 6: Commit**

```bash
git add finstack/core/src/market_data/surfaces/vol_cube.rs \
        finstack/core/src/market_data/surfaces/mod.rs \
        finstack/core/tests/market_data/surfaces/vol_cube_tests.rs \
        finstack/core/tests/market_data/surfaces/mod.rs
git commit -m "feat(vol-cube): add VolCube struct with builder and from_grid construction"
```

---

## Task 3: Add SABR param interpolation and vol evaluation to VolCube

**Files:**
- Modify: `finstack/core/src/market_data/surfaces/vol_cube.rs`
- Modify: `finstack/core/tests/market_data/surfaces/vol_cube_tests.rs`

- [ ] **Step 1: Write failing tests for vol evaluation**

Add to `vol_cube_tests.rs`:

```rust
#[test]
fn test_vol_cube_vol_at_grid_node() {
    // Vol at an exact grid node should match direct Hagan eval
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let fwd = 0.03;
    let cube = VolCube::from_grid("TEST", &[1.0], &[5.0], &[p], &[fwd]).unwrap();

    let strike = 0.03; // ATM
    let vol = cube.vol(1.0, 5.0, strike).unwrap();
    let expected = p.implied_vol_lognormal(fwd, strike, 1.0);
    assert!(
        (vol - expected).abs() < 1e-14,
        "grid-node vol {vol} != direct SABR {expected}"
    );
}

#[test]
fn test_vol_cube_vol_interpolated() {
    // Vol at a midpoint should be between surrounding node vols
    let p_lo = SabrParams::new(0.020, 0.5, -0.2, 0.3).unwrap();
    let p_hi = SabrParams::new(0.050, 0.5, -0.2, 0.5).unwrap();
    let cube = VolCube::from_grid(
        "TEST",
        &[1.0, 5.0],
        &[5.0, 10.0],
        &[p_lo, p_lo, p_hi, p_hi],
        &[0.03, 0.04, 0.03, 0.04],
    )
    .unwrap();

    let strike = 0.035;
    let vol_mid = cube.vol(3.0, 7.5, strike).unwrap();
    // Should be finite and positive
    assert!(vol_mid.is_finite() && vol_mid > 0.0);
}

#[test]
fn test_vol_cube_vol_clamped_extrapolation() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::from_grid("TEST", &[1.0, 5.0], &[5.0, 10.0], &[p; 4], &[0.03; 4]).unwrap();

    // Out-of-bounds should clamp, not error
    let vol = cube.vol_clamped(0.1, 2.0, 0.03);
    assert!(vol.is_finite() && vol > 0.0);
}

#[test]
fn test_vol_cube_vol_checked_out_of_bounds() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::from_grid("TEST", &[1.0, 5.0], &[5.0, 10.0], &[p; 4], &[0.03; 4]).unwrap();

    // Out-of-bounds should error
    assert!(cube.vol(0.1, 7.0, 0.03).is_err());
    assert!(cube.vol(3.0, 2.0, 0.03).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p finstack-core --test '*' -E 'test(vol_cube_vol)' --no-capture`
Expected: FAIL — `vol`, `vol_clamped` methods don't exist

- [ ] **Step 3: Implement SABR param interpolation and vol evaluation**

Add to `VolCube` impl in `vol_cube.rs`:

```rust
    // ── Interpolation ───────────────────────────────────────────────────

    /// Bilinear interpolation helper.
    #[inline]
    fn bilinear(q00: f64, q10: f64, q01: f64, q11: f64, t: f64, u: f64) -> f64 {
        (1.0 - t) * (1.0 - u) * q00
            + t * (1.0 - u) * q10
            + (1.0 - t) * u * q01
            + t * u * q11
    }

    /// Interpolate SABR params at (expiry, tenor) via bilinear blending.
    /// Returns (interpolated_params, interpolated_forward, clamped_expiry).
    fn interpolate_params_clamped(&self, expiry: f64, tenor: f64) -> (SabrParams, f64, f64) {
        let n_t = self.tenors.len();

        let exp_clamped = expiry.clamp(self.expiries[0], self.expiries[self.expiries.len() - 1]);
        let ten_clamped = tenor.clamp(self.tenors[0], self.tenors[n_t - 1]);

        // Locate segments
        let (ei, et) = if self.expiries.len() == 1 {
            (0, 0.0)
        } else {
            let i = locate_segment(&self.expiries, exp_clamped);
            let t = (exp_clamped - self.expiries[i]) / (self.expiries[i + 1] - self.expiries[i]);
            (i, t)
        };

        let (ti, tt) = if self.tenors.len() == 1 {
            (0, 0.0)
        } else {
            let i = locate_segment(&self.tenors, ten_clamped);
            let t = (ten_clamped - self.tenors[i]) / (self.tenors[i + 1] - self.tenors[i]);
            (i, t)
        };

        // Index helpers
        let idx = |ei: usize, ti: usize| ei * n_t + ti;

        let (ei1, ti1) = (
            (ei + 1).min(self.expiries.len() - 1),
            (ti + 1).min(n_t - 1),
        );

        let p00 = &self.params[idx(ei, ti)];
        let p10 = &self.params[idx(ei1, ti)];
        let p01 = &self.params[idx(ei, ti1)];
        let p11 = &self.params[idx(ei1, ti1)];

        // Bilinear on each SABR param
        let alpha = Self::bilinear(p00.alpha, p10.alpha, p01.alpha, p11.alpha, et, tt);
        let rho = Self::bilinear(p00.rho, p10.rho, p01.rho, p11.rho, et, tt);
        let nu = Self::bilinear(p00.nu, p10.nu, p01.nu, p11.nu, et, tt);
        // Beta held constant from nearest node
        let beta = p00.beta;
        // Shift: bilinear if all four corners have shift, else nearest
        let shift = match (p00.shift, p10.shift, p01.shift, p11.shift) {
            (Some(s00), Some(s10), Some(s01), Some(s11)) => {
                Some(Self::bilinear(s00, s10, s01, s11, et, tt))
            }
            _ => p00.shift.or(p10.shift).or(p01.shift).or(p11.shift),
        };

        // Post-interpolation clamp
        let alpha = alpha.max(1e-8);
        let nu = nu.max(1e-8);
        let rho = rho.clamp(-0.9999, 0.9999);
        let beta = beta.clamp(0.0, 1.0);

        let interp = SabrParams {
            alpha,
            beta,
            rho,
            nu,
            shift,
        };

        // Forward rate
        let fwd = Self::bilinear(
            self.forwards[idx(ei, ti)],
            self.forwards[idx(ei1, ti)],
            self.forwards[idx(ei, ti1)],
            self.forwards[idx(ei1, ti1)],
            et,
            tt,
        );

        (interp, fwd, exp_clamped)
    }

    // ── Vol evaluation ──────────────────────────────────────────────────

    /// Look up implied vol at (expiry, tenor, strike).
    ///
    /// Bilinear-interpolates SABR parameters across the (expiry, tenor)
    /// grid, then evaluates the Hagan lognormal formula at the given strike.
    ///
    /// Returns an error if expiry or tenor is outside the grid bounds.
    pub fn vol(&self, expiry: f64, tenor: f64, strike: f64) -> crate::Result<f64> {
        // Bounds check
        if expiry < self.expiries[0] || expiry > self.expiries[self.expiries.len() - 1] {
            return Err(Error::Validation(format!(
                "VolCube expiry {expiry} out of bounds [{}, {}]",
                self.expiries[0],
                self.expiries[self.expiries.len() - 1]
            )));
        }
        if tenor < self.tenors[0] || tenor > self.tenors[self.tenors.len() - 1] {
            return Err(Error::Validation(format!(
                "VolCube tenor {tenor} out of bounds [{}, {}]",
                self.tenors[0],
                self.tenors[self.tenors.len() - 1]
            )));
        }

        let (params, fwd, exp_clamped) = self.interpolate_params_clamped(expiry, tenor);
        Ok(params.implied_vol_lognormal(fwd, strike, exp_clamped))
    }

    /// Look up implied vol, clamping out-of-bounds coordinates to grid edges.
    pub fn vol_clamped(&self, expiry: f64, tenor: f64, strike: f64) -> f64 {
        let (params, fwd, exp_clamped) = self.interpolate_params_clamped(expiry, tenor);
        params.implied_vol_lognormal(fwd, strike, exp_clamped)
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p finstack-core --features mc,test-utils --lib --test '*' -E 'test(vol_cube)' --no-capture`
Expected: All vol_cube tests pass

- [ ] **Step 5: Commit**

```bash
git add finstack/core/src/market_data/surfaces/vol_cube.rs \
        finstack/core/tests/market_data/surfaces/vol_cube_tests.rs
git commit -m "feat(vol-cube): add SABR param interpolation and vol evaluation"
```

---

## Task 4: Add grid materialization to VolCube

**Files:**
- Modify: `finstack/core/src/market_data/surfaces/vol_cube.rs`
- Modify: `finstack/core/tests/market_data/surfaces/vol_cube_tests.rs`

- [ ] **Step 1: Write failing test for materialize_tenor_slice**

Add to `vol_cube_tests.rs`:

```rust
use finstack_core::market_data::surfaces::VolSurface;

#[test]
fn test_vol_cube_materialize_tenor_slice() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::from_grid(
        "TEST",
        &[1.0, 5.0],
        &[5.0, 10.0],
        &[p; 4],
        &[0.03, 0.035, 0.04, 0.045],
    )
    .unwrap();

    let strikes = vec![0.01, 0.02, 0.03, 0.04, 0.05];
    let surface = cube.materialize_tenor_slice(5.0, &strikes).unwrap();

    assert_eq!(surface.expiries(), &[1.0, 5.0]);
    assert_eq!(surface.strikes(), &strikes[..]);

    // Surface vol at a grid node should match cube vol
    let cube_vol = cube.vol(1.0, 5.0, 0.03).unwrap();
    let surf_vol = surface.value_checked(1.0, 0.03).unwrap();
    assert!(
        (cube_vol - surf_vol).abs() < 1e-14,
        "materialized surface vol {surf_vol} != cube vol {cube_vol}"
    );
}

#[test]
fn test_vol_cube_materialize_expiry_slice() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::from_grid(
        "TEST",
        &[1.0, 5.0],
        &[5.0, 10.0],
        &[p; 4],
        &[0.03, 0.035, 0.04, 0.045],
    )
    .unwrap();

    let strikes = vec![0.02, 0.03, 0.04];
    let surface = cube.materialize_expiry_slice(1.0, &strikes).unwrap();

    assert_eq!(surface.expiries(), &[5.0, 10.0]); // tenor axis becomes expiry
    assert_eq!(surface.strikes(), &strikes[..]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p finstack-core --test '*' -E 'test(materialize)' --no-capture`
Expected: FAIL — `materialize_tenor_slice` doesn't exist

- [ ] **Step 3: Implement materialization methods**

Add to `VolCube` impl in `vol_cube.rs`:

```rust
    // ── Materialization ─────────────────────────────────────────────────

    /// Materialize a 2D VolSurface for a specific tenor.
    ///
    /// Interpolates SABR params to the target tenor, then evaluates the
    /// smile at each (expiry, strike) on the given strike grid.
    pub fn materialize_tenor_slice(
        &self,
        tenor: f64,
        strikes: &[f64],
    ) -> crate::Result<VolSurface> {
        let mut vols = Vec::with_capacity(self.expiries.len() * strikes.len());
        for (ei, &exp) in self.expiries.iter().enumerate() {
            let (params, fwd, _) = self.interpolate_params_clamped(exp, tenor);
            for &k in strikes {
                vols.push(params.implied_vol_lognormal(fwd, k, exp));
            }
        }
        VolSurface::from_grid(
            &format!("{}-tenor-{tenor:.2}", self.id),
            &self.expiries,
            strikes,
            &vols,
        )
    }

    /// Materialize a 2D VolSurface for a specific expiry.
    ///
    /// The returned surface has tenors as its "expiry" axis and strikes
    /// as the secondary axis.
    pub fn materialize_expiry_slice(
        &self,
        expiry: f64,
        strikes: &[f64],
    ) -> crate::Result<VolSurface> {
        let mut vols = Vec::with_capacity(self.tenors.len() * strikes.len());
        for &ten in self.tenors.iter() {
            let (params, fwd, exp_clamped) = self.interpolate_params_clamped(expiry, ten);
            for &k in strikes {
                vols.push(params.implied_vol_lognormal(fwd, k, exp_clamped));
            }
        }
        VolSurface::from_grid(
            &format!("{}-expiry-{expiry:.2}", self.id),
            &self.tenors,
            strikes,
            &vols,
        )
    }

    /// Materialize the full 3D grid at given strikes.
    ///
    /// Returns vols as a flat Vec in (expiry, tenor, strike) order:
    /// `vols[exp_idx * n_tenors * n_strikes + tenor_idx * n_strikes + strike_idx]`.
    pub fn materialize_grid(&self, strikes: &[f64]) -> crate::Result<Vec<f64>> {
        let n_s = strikes.len();
        let mut vols = Vec::with_capacity(self.expiries.len() * self.tenors.len() * n_s);
        for &exp in self.expiries.iter() {
            for &ten in self.tenors.iter() {
                let (params, fwd, _) = self.interpolate_params_clamped(exp, ten);
                for &k in strikes {
                    vols.push(params.implied_vol_lognormal(fwd, k, exp));
                }
            }
        }
        Ok(vols)
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p finstack-core --features mc,test-utils --lib --test '*' -E 'test(vol_cube)' --no-capture`
Expected: All vol_cube tests pass

- [ ] **Step 5: Commit**

```bash
git add finstack/core/src/market_data/surfaces/vol_cube.rs \
        finstack/core/tests/market_data/surfaces/vol_cube_tests.rs
git commit -m "feat(vol-cube): add tenor/expiry slice and full grid materialization"
```

---

## Task 5: Add VolProvider trait

**Files:**
- Modify: `finstack/core/src/market_data/traits.rs:402`
- Modify: `finstack/core/src/market_data/surfaces/vol_cube.rs`
- Modify: `finstack/core/src/market_data/surfaces/vol_surface.rs`
- Modify: `finstack/core/tests/market_data/surfaces/vol_cube_tests.rs`

- [ ] **Step 1: Write failing test for VolProvider trait**

Add to `vol_cube_tests.rs`:

```rust
use finstack_core::market_data::traits::VolProvider;

#[test]
fn test_vol_provider_cube() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::from_grid("TEST", &[1.0, 5.0], &[5.0, 10.0], &[p; 4], &[0.03; 4]).unwrap();

    // Use via trait
    let provider: &dyn VolProvider = &cube;
    let vol = provider.vol(1.0, 5.0, 0.03).unwrap();
    assert!(vol.is_finite() && vol > 0.0);
}

#[test]
fn test_vol_provider_surface_ignores_tenor() {
    let surface = VolSurface::builder("TEST")
        .expiries(&[1.0, 2.0])
        .strikes(&[0.02, 0.03, 0.04])
        .row(&[0.20, 0.21, 0.22])
        .row(&[0.19, 0.20, 0.21])
        .build()
        .unwrap();

    let provider: &dyn VolProvider = &surface;
    // Tenor argument should be ignored
    let vol_a = provider.vol(1.5, 5.0, 0.03).unwrap();
    let vol_b = provider.vol(1.5, 999.0, 0.03).unwrap();
    assert!((vol_a - vol_b).abs() < 1e-14, "VolSurface should ignore tenor");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p finstack-core --test '*' -E 'test(vol_provider)' --no-capture`
Expected: FAIL — `VolProvider` trait doesn't exist

- [ ] **Step 3: Define VolProvider trait**

In `finstack/core/src/market_data/traits.rs`, add before the `#[cfg(test)]` block (around line 404):

```rust
/// Trait for types that supply implied volatility given market coordinates.
///
/// Swaption pricers accept `&dyn VolProvider` to decouple from storage
/// (2D surface vs 3D cube). The three-argument signature supports both:
///
/// - **VolCube**: interpolates across all three dimensions
/// - **VolSurface**: ignores `tenor`, queries `(expiry, strike)` only
pub trait VolProvider: Send + Sync {
    /// Look up implied vol at (expiry, tenor, strike).
    ///
    /// Returns an error if coordinates are out of bounds.
    fn vol(&self, expiry: f64, tenor: f64, strike: f64) -> crate::Result<f64>;

    /// Same as [`vol`](Self::vol) but clamps out-of-bounds coordinates.
    fn vol_clamped(&self, expiry: f64, tenor: f64, strike: f64) -> f64;

    /// The identifier of this vol source.
    fn vol_id(&self) -> &crate::types::CurveId;
}
```

- [ ] **Step 4: Implement VolProvider for VolCube**

In `finstack/core/src/market_data/surfaces/vol_cube.rs`, add at the bottom:

```rust
impl crate::market_data::traits::VolProvider for VolCube {
    fn vol(&self, expiry: f64, tenor: f64, strike: f64) -> crate::Result<f64> {
        self.vol(expiry, tenor, strike)
    }

    fn vol_clamped(&self, expiry: f64, tenor: f64, strike: f64) -> f64 {
        self.vol_clamped(expiry, tenor, strike)
    }

    fn vol_id(&self) -> &CurveId {
        self.id()
    }
}
```

- [ ] **Step 5: Implement VolProvider for VolSurface**

In `finstack/core/src/market_data/surfaces/vol_surface.rs`, add an impl block (at the bottom of the file, before tests):

```rust
impl crate::market_data::traits::VolProvider for VolSurface {
    fn vol(&self, expiry: f64, _tenor: f64, strike: f64) -> crate::Result<f64> {
        self.value_checked(expiry, strike)
    }

    fn vol_clamped(&self, expiry: f64, _tenor: f64, strike: f64) -> f64 {
        self.value_clamped(expiry, strike)
    }

    fn vol_id(&self) -> &crate::types::CurveId {
        self.id()
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo nextest run -p finstack-core --features mc,test-utils --lib --test '*' -E 'test(vol_provider) | test(vol_cube) | test(vol_surface)' --no-capture`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add finstack/core/src/market_data/traits.rs \
        finstack/core/src/market_data/surfaces/vol_cube.rs \
        finstack/core/src/market_data/surfaces/vol_surface.rs \
        finstack/core/tests/market_data/surfaces/vol_cube_tests.rs
git commit -m "feat(vol-cube): add VolProvider trait with VolCube and VolSurface implementations"
```

---

## Task 6: Integrate VolCube into MarketContext

**Files:**
- Modify: `finstack/core/src/market_data/context/mod.rs:139-172`
- Modify: `finstack/core/src/market_data/context/insert.rs`
- Modify: `finstack/core/src/market_data/context/getters.rs`
- Modify: `finstack/core/src/market_data/context/stats.rs`
- Modify: `finstack/core/src/market_data/context/ops_roll.rs`
- Modify: `finstack/core/src/market_data/context/state_serde.rs`

- [ ] **Step 1: Write failing test for MarketContext vol cube integration**

Add to `vol_cube_tests.rs`:

```rust
use finstack_core::market_data::context::MarketContext;

#[test]
fn test_market_context_vol_cube_insert_and_get() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::from_grid("USD-SWPT", &[1.0], &[5.0], &[p], &[0.03]).unwrap();

    let ctx = MarketContext::new().insert_vol_cube(cube);
    let retrieved = ctx.get_vol_cube("USD-SWPT").unwrap();
    assert_eq!(retrieved.id().as_str(), "USD-SWPT");
}

#[test]
fn test_market_context_vol_provider_finds_cube() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::from_grid("USD-SWPT", &[1.0], &[5.0], &[p], &[0.03]).unwrap();

    let ctx = MarketContext::new().insert_vol_cube(cube);
    let provider = ctx.get_vol_provider("USD-SWPT").unwrap();
    let vol = provider.vol(1.0, 5.0, 0.03).unwrap();
    assert!(vol.is_finite() && vol > 0.0);
}

#[test]
fn test_market_context_vol_provider_falls_back_to_surface() {
    let surface = VolSurface::builder("EQ-VOL")
        .expiries(&[1.0, 2.0])
        .strikes(&[90.0, 100.0])
        .row(&[0.2, 0.2])
        .row(&[0.2, 0.2])
        .build()
        .unwrap();

    let ctx = MarketContext::new().insert_surface(surface);
    let provider = ctx.get_vol_provider("EQ-VOL").unwrap();
    let vol = provider.vol_clamped(1.5, 999.0, 95.0);
    assert!(vol > 0.0);
}

#[test]
fn test_market_context_stats_includes_vol_cubes() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::from_grid("TEST", &[1.0], &[5.0], &[p], &[0.03]).unwrap();
    let ctx = MarketContext::new().insert_vol_cube(cube);
    assert_eq!(ctx.stats().vol_cube_count, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p finstack-core --test '*' -E 'test(market_context_vol_cube) | test(market_context_vol_provider) | test(market_context_stats_includes_vol)' --no-capture`
Expected: FAIL — methods don't exist

- [ ] **Step 3: Add vol_cubes field to MarketContext**

In `finstack/core/src/market_data/context/mod.rs`, add the import at the top (around line 128):

```rust
surfaces::{FxDeltaVolSurface, VolSurface, VolCube},
```

Add the field to `MarketContext` struct (after `surfaces` at line 147):

```rust
/// Swaption volatility cubes (SABR parametric)
vol_cubes: HashMap<CurveId, Arc<VolCube>>,
```

Update the `Debug` impl (around line 185) to include:

```rust
.field("vol_cubes", &self.vol_cubes.len())
```

Update `all_ids()` (around line 281) to include:

```rust
present.extend(self.vol_cubes.keys().cloned());
```

- [ ] **Step 4: Add insert methods**

In `finstack/core/src/market_data/context/insert.rs`, add the import for `VolCube` and add after `insert_surface` (line 87):

```rust
    /// Insert a swaption volatility cube.
    pub fn insert_vol_cube(mut self, cube: impl Into<Arc<VolCube>>) -> Self {
        let arc_cube = cube.into();
        let id = arc_cube.id().to_owned();
        self.vol_cubes.insert(id, arc_cube);
        self
    }
```

And add a mutable variant after `insert_surface_mut` (around line 390):

```rust
    /// Insert a swaption volatility cube (mutable variant for FFI).
    pub fn insert_vol_cube_mut(&mut self, cube: impl Into<Arc<VolCube>>) -> &mut Self {
        let arc_cube = cube.into();
        let id = arc_cube.id().to_owned();
        self.vol_cubes.insert(id, arc_cube);
        self
    }
```

- [ ] **Step 5: Add getter methods**

In `finstack/core/src/market_data/context/getters.rs`, add the import for `VolCube` and `VolProvider`, then add after `get_surface` (line 250):

```rust
    /// Retrieve a swaption volatility cube by identifier.
    pub fn get_vol_cube(&self, id: impl AsRef<str>) -> Result<Arc<VolCube>> {
        self.get_cloned(&self.vol_cubes, id.as_ref())
    }

    /// Retrieve a vol provider by identifier.
    ///
    /// Searches vol cubes first, then vol surfaces. Returns a trait object
    /// that can be used for polymorphic vol lookup.
    pub fn get_vol_provider(
        &self,
        id: impl AsRef<str>,
    ) -> Result<Arc<dyn crate::market_data::traits::VolProvider>> {
        let id_str = id.as_ref();
        // Try cube first
        if let Some(cube) = self.vol_cubes.get(id_str) {
            return Ok(Arc::clone(cube) as Arc<dyn crate::market_data::traits::VolProvider>);
        }
        // Fall back to surface
        if let Some(surface) = self.surfaces.get(id_str) {
            return Ok(Arc::clone(surface) as Arc<dyn crate::market_data::traits::VolProvider>);
        }
        Err(Self::not_found_error(id_str))
    }
```

- [ ] **Step 6: Update stats**

In `finstack/core/src/market_data/context/stats.rs`:

Add to `stats()` method (around line 110):

```rust
vol_cube_count: self.vol_cubes.len(),
```

Add to `contains()` (around line 125):

```rust
|| self.vol_cubes.contains_key(id)
```

Add to `is_empty()` (around line 138):

```rust
&& self.vol_cubes.is_empty()
```

Add to `total_objects()` (around line 150):

```rust
+ self.vol_cubes.len()
```

Add to `ContextStats` struct (around line 252):

```rust
/// Number of swaption volatility cubes
pub vol_cube_count: usize,
```

Add to `Display` impl (around line 273):

```rust
writeln!(f, "  Vol cubes: {}", self.vol_cube_count)?;
```

- [ ] **Step 7: Update roll_forward**

In `finstack/core/src/market_data/context/ops_roll.rs`, add to the new context construction (around line 96, after `surfaces`):

```rust
vol_cubes: self.vol_cubes.clone(),
```

- [ ] **Step 8: Update state_serde**

In `finstack/core/src/market_data/context/state_serde.rs`:

Add to `MarketContextState` struct (after `surfaces` at line 179):

```rust
/// Swaption volatility cubes
#[serde(default)]
#[schemars(with = "serde_json::Value")]
pub vol_cubes: Vec<VolCube>,
```

Add to `From<&MarketContext>` impl (after surfaces extraction, around line 272):

```rust
// Convert vol cubes
let mut vol_cube_pairs: Vec<(CurveId, VolCube)> = ctx
    .vol_cubes
    .iter()
    .map(|(id, cube)| (id.clone(), (**cube).clone()))
    .collect();
vol_cube_pairs.sort_by(|a, b| a.0.cmp(&b.0));
let vol_cubes: Vec<_> = vol_cube_pairs.into_iter().map(|(_, c)| c).collect();
```

And include `vol_cubes` in the returned `MarketContextState`.

Add to `TryFrom<MarketContextState>` impl (after surfaces reconstruction, around line 402):

```rust
// Reconstruct vol cubes
for cube in state.vol_cubes {
    ctx.vol_cubes.insert(cube.id().clone(), Arc::new(cube));
}
```

- [ ] **Step 9: Run tests to verify they pass**

Run: `cargo nextest run -p finstack-core --features mc,test-utils --lib --test '*' --no-fail-fast`
Expected: All tests pass

- [ ] **Step 10: Commit**

```bash
git add finstack/core/src/market_data/context/mod.rs \
        finstack/core/src/market_data/context/insert.rs \
        finstack/core/src/market_data/context/getters.rs \
        finstack/core/src/market_data/context/stats.rs \
        finstack/core/src/market_data/context/ops_roll.rs \
        finstack/core/src/market_data/context/state_serde.rs \
        finstack/core/tests/market_data/surfaces/vol_cube_tests.rs
git commit -m "feat(vol-cube): integrate VolCube into MarketContext with insert/get/stats/serde"
```

---

## Task 7: Update swaption calibration to produce VolCube

**Files:**
- Modify: `finstack/valuations/src/calibration/targets/swaption.rs:314-396`

- [ ] **Step 1: Write failing test for VolCube calibration output**

Find the existing swaption calibration test file and add a test that checks the solve method returns a VolCube. If the existing test checks for VolSurface output, duplicate it with a VolCube expectation. The exact test depends on the existing test infrastructure — look at `finstack/valuations/tests/calibration/swaption_vol.rs` for the pattern.

For the implementation, the key change is in `SwaptionVolTarget::solve` return type: change from `(VolSurface, CalibrationReport)` to `(VolCube, CalibrationReport)`.

- [ ] **Step 2: Update solve() to collect SABR params and forwards for VolCube**

In `finstack/valuations/src/calibration/targets/swaption.rs`, the solve method currently builds a `grid: Vec<f64>` of ATM vols (lines 314-388) then constructs a `VolSurface::from_grid_with_axis`. Change to collect params and forwards instead:

Replace the grid-building loop (lines 314-388) to also collect SABR params and forwards:

```rust
let mut grid_params: Vec<finstack_core::math::volatility::sabr::SabrParams> = Vec::new();
let mut grid_forwards: Vec<f64> = Vec::new();
let mut grid = Vec::new(); // keep for ATM vol report

for &texp in &target_expiries {
    for &tten in &target_tenors {
        // ... existing interpolation logic to get `p` (SABRParameters) ...
        
        let leg_conv = Self::default_leg_conventions(params)?;
        let f = Self::calculate_forward_swap_rate_years(
            params, texp, tten, &leg_conv, context,
        )?;
        let strike = Self::atm_strike(params, f);
        let model = SABRModel::new(p.clone());
        let val = model.implied_volatility(f, strike, texp)?;
        grid.push(val);

        // Collect for VolCube
        let core_params = finstack_core::math::volatility::sabr::SabrParams {
            alpha: p.alpha,
            beta: p.beta,
            rho: p.rho,
            nu: p.nu,
            shift: p.shift,
        };
        grid_params.push(core_params);
        grid_forwards.push(f);
    }
}
```

Replace the VolSurface construction (lines 390-396) with VolCube:

```rust
let cube = finstack_core::market_data::surfaces::VolCube::from_grid(
    &params.surface_id,
    &target_expiries,
    &target_tenors,
    &grid_params,
    &grid_forwards,
)?;
```

Update the return type signature of `solve()` from `Result<(VolSurface, CalibrationReport)>` to `Result<(VolCube, CalibrationReport)>`.

- [ ] **Step 3: Update callers of solve()**

Search for all callers of `SwaptionVolTarget::solve` and update them to handle `VolCube` instead of `VolSurface`. Use `insert_vol_cube` instead of `insert_surface` when adding to MarketContext.

Run: `cargo nextest run -p finstack-valuations --features mc,test-utils --lib --test '*' --no-fail-fast`
Fix any compilation errors from the type change.

- [ ] **Step 4: Run full test suite**

Run: `cargo nextest run --workspace --exclude finstack-py --features mc,test-utils --lib --test '*' --no-fail-fast`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/src/calibration/targets/swaption.rs
# Also add any caller files that needed updates
git commit -m "feat(calibration): swaption vol calibration now produces VolCube"
```

---

## Task 8: Update swaption pricer to use VolProvider

**Files:**
- Modify: `finstack/valuations/src/instruments/rates/swaption/pricer.rs:170-270`

- [ ] **Step 1: Update SimpleSwaptionBlackPricer to use get_vol_provider**

In `finstack/valuations/src/instruments/rates/swaption/pricer.rs`, modify `price_dyn` (around line 216):

Replace:
```rust
let vol_surface = market
    .get_surface(swaption.vol_surface_id.as_str())
    .map_err(|e| { ... })?;
```

With:
```rust
let vol_provider = market
    .get_vol_provider(swaption.vol_surface_id.as_str())
    .map_err(|e| {
        PricingError::missing_market_data_with_context(
            e.to_string(),
            PricingErrorContext::default(),
        )
    })?;
```

Compute the underlying tenor from the swaption:

```rust
let underlying_tenor = year_fraction(swaption.day_count, swaption.expiry, swaption.swap_end)
    .map_err(|e| {
        PricingError::model_failure_with_context(
            e.to_string(),
            PricingErrorContext::default(),
        )
    })?;
```

Replace the vol lookup (around lines 225-262) with:

```rust
let vol = if let Some(impl_vol) =
    swaption.pricing_overrides.market_quotes.implied_volatility
{
    impl_vol
} else {
    vol_provider.vol_clamped(time_to_expiry, underlying_tenor, strike)
};
```

Note: The `require_secondary_axis` check and `LinearInVariance` extrapolation are no longer needed when using `VolProvider` — the provider handles all interpolation internally. For backward compatibility with VolSurface, `vol_clamped` already does flat extrapolation at grid edges.

- [ ] **Step 2: Run swaption tests**

Run: `cargo nextest run -p finstack-valuations --features mc,test-utils --lib --test '*' -E 'test(swaption)' --no-fail-fast`
Expected: All swaption tests pass

- [ ] **Step 3: Run full workspace tests**

Run: `cargo nextest run --workspace --exclude finstack-py --features mc,test-utils --lib --test '*' --no-fail-fast`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/instruments/rates/swaption/pricer.rs
git commit -m "feat(pricer): swaption pricer uses VolProvider for polymorphic vol lookup"
```

---

## Task 9: Python bindings for VolCube

**Files:**
- Modify: `finstack-py/src/bindings/core/market_data/curves.rs`
- Modify: `finstack-py/finstack/core/market_data.pyi`

- [ ] **Step 1: Add PyVolCube class**

In `finstack-py/src/bindings/core/market_data/curves.rs`, add after the `PyVolSurface` impl:

```rust
#[pyclass(
    name = "VolCube",
    module = "finstack.core.market_data",
    frozen,
)]
#[derive(Clone)]
pub struct PyVolCube {
    pub(crate) inner: Arc<VolCube>,
}

#[pymethods]
impl PyVolCube {
    #[new]
    #[pyo3(signature = (id, expiries, tenors, params_row_major, forwards_row_major, interpolation_mode="vol"))]
    fn new(
        id: &str,
        expiries: Vec<f64>,
        tenors: Vec<f64>,
        params_row_major: Vec<pyo3::Bound<'_, pyo3::types::PyDict>>,
        forwards_row_major: Vec<f64>,
        interpolation_mode: &str,
    ) -> PyResult<Self> {
        use finstack_core::math::volatility::sabr::SabrParams;

        let mut sabr_params = Vec::with_capacity(params_row_major.len());
        for dict in &params_row_major {
            let alpha: f64 = dict.get_item("alpha")?.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("Missing 'alpha' in SABR params dict")
            })?.extract()?;
            let beta: f64 = dict.get_item("beta")?.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("Missing 'beta' in SABR params dict")
            })?.extract()?;
            let rho: f64 = dict.get_item("rho")?.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("Missing 'rho' in SABR params dict")
            })?.extract()?;
            let nu: f64 = dict.get_item("nu")?.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("Missing 'nu' in SABR params dict")
            })?.extract()?;

            let mut p = SabrParams::new(alpha, beta, rho, nu).map_err(core_to_py)?;

            if let Ok(Some(shift_obj)) = dict.get_item("shift") {
                if !shift_obj.is_none() {
                    let shift: f64 = shift_obj.extract()?;
                    p = p.with_shift(shift);
                }
            }

            sabr_params.push(p);
        }

        let cube = VolCube::from_grid(id, &expiries, &tenors, &sabr_params, &forwards_row_major)
            .map_err(core_to_py)?;
        Ok(Self {
            inner: Arc::new(cube),
        })
    }

    fn vol(&self, expiry: f64, tenor: f64, strike: f64) -> PyResult<f64> {
        self.inner.vol(expiry, tenor, strike).map_err(core_to_py)
    }

    fn vol_clamped(&self, expiry: f64, tenor: f64, strike: f64) -> f64 {
        self.inner.vol_clamped(expiry, tenor, strike)
    }

    fn materialize_tenor_slice(&self, tenor: f64, strikes: Vec<f64>) -> PyResult<PyVolSurface> {
        let surface = self.inner.materialize_tenor_slice(tenor, &strikes).map_err(core_to_py)?;
        Ok(PyVolSurface {
            inner: Arc::new(surface),
        })
    }

    fn materialize_expiry_slice(&self, expiry: f64, strikes: Vec<f64>) -> PyResult<PyVolSurface> {
        let surface = self.inner.materialize_expiry_slice(expiry, &strikes).map_err(core_to_py)?;
        Ok(PyVolSurface {
            inner: Arc::new(surface),
        })
    }

    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    #[getter]
    fn expiries(&self) -> Vec<f64> {
        self.inner.expiries().to_vec()
    }

    #[getter]
    fn tenors(&self) -> Vec<f64> {
        self.inner.tenors().to_vec()
    }

    #[getter]
    fn grid_shape(&self) -> (usize, usize) {
        self.inner.grid_shape()
    }
}
```

Register `PyVolCube` in the Python module (find where `PyVolSurface` is registered and add `PyVolCube` next to it).

- [ ] **Step 2: Add type stubs**

In `finstack-py/finstack/core/market_data.pyi`, add after the `VolSurface` class:

```python
class VolCube:
    def __init__(
        self,
        id: str,
        expiries: list[float],
        tenors: list[float],
        params_row_major: list[dict[str, float]],
        forwards_row_major: list[float],
        interpolation_mode: str = "vol",
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

- [ ] **Step 3: Build and test Python bindings**

Run: `cd finstack-py && uv run maturin develop --release && uv run pytest tests/ -k "vol_cube or VolCube" -v`

If no Python tests exist yet for VolCube, verify import works:
`cd finstack-py && uv run python -c "from finstack.core.market_data import VolCube; print('OK')"`

- [ ] **Step 4: Commit**

```bash
git add finstack-py/src/bindings/core/market_data/curves.rs \
        finstack-py/finstack/core/market_data.pyi
git commit -m "feat(python): add PyVolCube bindings for swaption vol cube"
```

---

## Task 10: WASM bindings for VolCube

**Files:**
- Modify: `finstack-wasm/src/api/core/market_data.rs`

- [ ] **Step 1: Add WASM VolCube binding**

In `finstack-wasm/src/api/core/market_data.rs`, add:

```rust
use finstack_core::market_data::surfaces::VolCube as RustVolCube;
use finstack_core::math::volatility::sabr::SabrParams;

#[wasm_bindgen(js_name = VolCube)]
pub struct VolCube {
    #[wasm_bindgen(skip)]
    pub(crate) inner: Arc<RustVolCube>,
}

#[wasm_bindgen(js_class = VolCube)]
impl VolCube {
    /// Construct a VolCube from flat arrays.
    ///
    /// `params_flat` is a flattened array: [alpha0, beta0, rho0, nu0, shift0, alpha1, ...].
    /// Use NaN for shift when no shift is applied.
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        expiries: &[f64],
        tenors: &[f64],
        params_flat: &[f64],
        forwards: &[f64],
    ) -> Result<VolCube, JsValue> {
        let n_nodes = expiries.len() * tenors.len();
        if params_flat.len() != n_nodes * 5 {
            return Err(JsValue::from_str(&format!(
                "params_flat length {} != {} nodes * 5 params",
                params_flat.len(),
                n_nodes
            )));
        }

        let mut sabr_params = Vec::with_capacity(n_nodes);
        for i in 0..n_nodes {
            let base = i * 5;
            let mut p = SabrParams::new(
                params_flat[base],     // alpha
                params_flat[base + 1], // beta
                params_flat[base + 2], // rho
                params_flat[base + 3], // nu
            )
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

            let shift = params_flat[base + 4];
            if shift.is_finite() {
                p = p.with_shift(shift);
            }
            sabr_params.push(p);
        }

        let cube = RustVolCube::from_grid(id, expiries, tenors, &sabr_params, forwards)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Self {
            inner: Arc::new(cube),
        })
    }

    pub fn vol(&self, expiry: f64, tenor: f64, strike: f64) -> Result<f64, JsValue> {
        self.inner
            .vol(expiry, tenor, strike)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn vol_clamped(&self, expiry: f64, tenor: f64, strike: f64) -> f64 {
        self.inner.vol_clamped(expiry, tenor, strike)
    }
}
```

- [ ] **Step 2: Build WASM**

Run: `make wasm-build` (or the project's WASM build command)
Expected: Build succeeds

- [ ] **Step 3: Commit**

```bash
git add finstack-wasm/src/api/core/market_data.rs
git commit -m "feat(wasm): add VolCube WASM bindings"
```

---

## Task 11: Final integration test and cleanup

**Files:**
- All files from previous tasks

- [ ] **Step 1: Run full workspace test suite**

Run: `make test-rust`
Expected: All Rust tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --features mc,test-utils -- -D warnings`
Expected: No warnings

- [ ] **Step 3: Run doc tests**

Run: `make test-rust-doc`
Expected: All doc tests pass

- [ ] **Step 4: Build and test Python bindings**

Run: `make test-python`
Expected: All Python tests pass

- [ ] **Step 5: Build WASM**

Run: `make wasm-build`
Expected: Build succeeds

- [ ] **Step 6: Final commit if any fixups were needed**

```bash
git add -A
git commit -m "fix: address clippy/test issues from vol cube integration"
```
