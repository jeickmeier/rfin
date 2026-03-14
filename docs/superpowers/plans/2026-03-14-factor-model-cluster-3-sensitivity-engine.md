# Cluster 3: Factor Sensitivity Engine — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `FactorSensitivityEngine` trait, `SensitivityMatrix`, `BumpSizeConfig`, and the two built-in engines (`DeltaBasedEngine` and `FullRepricingEngine`) in `finstack/valuations`.

**Architecture:** All sensitivity computation lives in `finstack/valuations/src/factor_model/sensitivity/`. The engines reuse existing `MarketContext::bump()` and `instrument.value()` for finite-difference sensitivities. `PricingMode` lives in core since it's a config enum.

**Tech Stack:** Rust, Rayon (for parallel computation across positions/factors)

**Spec Reference:** `docs/superpowers/specs/2026-03-14-statistical-risk-factor-model-design.md` — Section 2

**Depends on:** Cluster 1 (FactorId, FactorDefinition, MarketMapping), Cluster 2 (decompose bridge)

---

## Task 1: Add `PricingMode` and `BumpSizeConfig` to core

**Files:**

- Create: `finstack/core/src/factor_model/config.rs`
- Modify: `finstack/core/src/factor_model/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_mode_serde() {
        let mode = PricingMode::DeltaBased;
        let json = serde_json::to_string(&mode).unwrap();
        let back: PricingMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, back);
    }

    #[test]
    fn test_bump_size_config_defaults() {
        let config = BumpSizeConfig::default();
        assert!((config.rates_bp - 1.0).abs() < 1e-12);
        assert!((config.credit_bp - 1.0).abs() < 1e-12);
        assert!((config.equity_pct - 1.0).abs() < 1e-12);
        assert!((config.fx_pct - 1.0).abs() < 1e-12);
        assert!((config.vol_points - 1.0).abs() < 1e-12);
        assert!(config.overrides.is_empty());
    }

    #[test]
    fn test_bump_size_for_factor_override() {
        let mut config = BumpSizeConfig::default();
        config.overrides.insert(FactorId::new("USD-Rates"), 0.5);
        assert!((config.bump_size_for_factor(&FactorId::new("USD-Rates"), &FactorType::Rates) - 0.5).abs() < 1e-12);
        // Non-overridden factor falls back to type default
        assert!((config.bump_size_for_factor(&FactorId::new("EUR-Rates"), &FactorType::Rates) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_bump_size_config_serde() {
        let config = BumpSizeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let back: BumpSizeConfig = serde_json::from_str(&json).unwrap();
        assert!((config.rates_bp - back.rates_bp).abs() < 1e-12);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

- [ ] **Step 3: Implement PricingMode and BumpSizeConfig**

```rust
use super::types::{FactorId, FactorType};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PricingMode {
    DeltaBased,
    FullRepricing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BumpSizeConfig {
    #[serde(default = "default_one")]
    pub rates_bp: f64,
    #[serde(default = "default_one")]
    pub credit_bp: f64,
    #[serde(default = "default_one")]
    pub equity_pct: f64,
    #[serde(default = "default_one")]
    pub fx_pct: f64,
    #[serde(default = "default_one")]
    pub vol_points: f64,
    #[serde(default)]
    pub overrides: BTreeMap<FactorId, f64>,
}

fn default_one() -> f64 { 1.0 }

impl Default for BumpSizeConfig {
    fn default() -> Self {
        Self {
            rates_bp: 1.0,
            credit_bp: 1.0,
            equity_pct: 1.0,
            fx_pct: 1.0,
            vol_points: 1.0,
            overrides: BTreeMap::new(),
        }
    }
}

impl BumpSizeConfig {
    /// Get bump size for a specific factor, checking overrides first.
    pub fn bump_size_for_factor(&self, factor_id: &FactorId, factor_type: &FactorType) -> f64 {
        if let Some(&size) = self.overrides.get(factor_id) {
            return size;
        }
        match factor_type {
            FactorType::Rates => self.rates_bp,
            FactorType::Credit => self.credit_bp,
            FactorType::Equity => self.equity_pct,
            FactorType::FX => self.fx_pct,
            FactorType::Volatility => self.vol_points,
            FactorType::Commodity => self.equity_pct,
            FactorType::Inflation => self.rates_bp,
            FactorType::Custom(_) => self.rates_bp,
        }
    }
}
```

- [ ] **Step 4: Register in mod.rs**

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core factor_model::config --no-default-features`
Expected: 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/core/src/factor_model/config.rs finstack/core/src/factor_model/mod.rs
git commit -m "feat(factor-model): add PricingMode and BumpSizeConfig"
```

---

## Task 2: Create `SensitivityMatrix`

**Files:**

- Create: `finstack/valuations/src/factor_model/sensitivity/mod.rs`
- Create: `finstack/valuations/src/factor_model/sensitivity/matrix.rs`
- Modify: `finstack/valuations/src/factor_model/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::factor_model::FactorId;

    #[test]
    fn test_matrix_construction() {
        let m = SensitivityMatrix::zeros(
            vec!["pos-1".into(), "pos-2".into()],
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
        );
        assert_eq!(m.n_positions(), 2);
        assert_eq!(m.n_factors(), 2);
        assert!((m.delta(0, 0)).abs() < 1e-15);
    }

    #[test]
    fn test_matrix_set_and_get() {
        let mut m = SensitivityMatrix::zeros(
            vec!["pos-1".into()],
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
        );
        m.set_delta(0, 0, 100.0);
        m.set_delta(0, 1, -50.0);
        assert!((m.delta(0, 0) - 100.0).abs() < 1e-12);
        assert!((m.delta(0, 1) - (-50.0)).abs() < 1e-12);
    }

    #[test]
    fn test_position_deltas_slice() {
        let mut m = SensitivityMatrix::zeros(
            vec!["pos-1".into()],
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
        );
        m.set_delta(0, 0, 100.0);
        m.set_delta(0, 1, -50.0);
        let row = m.position_deltas(0);
        assert_eq!(row.len(), 2);
        assert!((row[0] - 100.0).abs() < 1e-12);
        assert!((row[1] - (-50.0)).abs() < 1e-12);
    }

    #[test]
    fn test_factor_deltas_column() {
        let mut m = SensitivityMatrix::zeros(
            vec!["pos-1".into(), "pos-2".into()],
            vec![FactorId::new("Rates")],
        );
        m.set_delta(0, 0, 100.0);
        m.set_delta(1, 0, 200.0);
        let col = m.factor_deltas(0);
        assert_eq!(col.len(), 2);
        assert!((col[0] - 100.0).abs() < 1e-12);
        assert!((col[1] - 200.0).abs() < 1e-12);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

- [ ] **Step 3: Implement SensitivityMatrix**

```rust
use finstack_core::factor_model::FactorId;

/// Positions × factors sensitivity matrix with flat row-major storage.
#[derive(Debug, Clone)]
pub struct SensitivityMatrix {
    pub position_ids: Vec<String>,
    pub factor_ids: Vec<FactorId>,
    data: Vec<f64>,
    n_factors: usize,
}

impl SensitivityMatrix {
    /// Create a zero-initialized matrix.
    pub fn zeros(position_ids: Vec<String>, factor_ids: Vec<FactorId>) -> Self {
        let n_pos = position_ids.len();
        let n_fac = factor_ids.len();
        Self {
            position_ids,
            factor_ids,
            data: vec![0.0; n_pos * n_fac],
            n_factors: n_fac,
        }
    }

    pub fn n_positions(&self) -> usize {
        self.position_ids.len()
    }

    pub fn n_factors(&self) -> usize {
        self.n_factors
    }

    pub fn delta(&self, position_idx: usize, factor_idx: usize) -> f64 {
        self.data[position_idx * self.n_factors + factor_idx]
    }

    pub fn set_delta(&mut self, position_idx: usize, factor_idx: usize, value: f64) {
        self.data[position_idx * self.n_factors + factor_idx] = value;
    }

    /// Row slice for a single position.
    pub fn position_deltas(&self, position_idx: usize) -> &[f64] {
        let start = position_idx * self.n_factors;
        &self.data[start..start + self.n_factors]
    }

    /// Column vector for a single factor (allocates).
    pub fn factor_deltas(&self, factor_idx: usize) -> Vec<f64> {
        (0..self.n_positions())
            .map(|i| self.delta(i, factor_idx))
            .collect()
    }

    /// Raw data slice.
    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }
}
```

- [ ] **Step 4: Register modules**

- [ ] **Step 5: Run tests**

Expected: 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/valuations/src/factor_model/
git commit -m "feat(factor-model): add SensitivityMatrix with flat row-major storage"
```

---

## Task 3: Create `FactorSensitivityEngine` trait

**Files:**

- Create: `finstack/valuations/src/factor_model/sensitivity/traits.rs`
- Modify: `finstack/valuations/src/factor_model/sensitivity/mod.rs`

- [ ] **Step 1: Define the trait**

```rust
use super::matrix::SensitivityMatrix;
use crate::instruments::common::traits::Instrument;
use finstack_core::factor_model::FactorDefinition;
use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use time::Date;

/// Engine that computes per-position, per-factor sensitivities.
pub trait FactorSensitivityEngine: Send + Sync {
    fn compute_sensitivities(
        &self,
        positions: &[(String, &dyn Instrument, f64)],
        factors: &[FactorDefinition],
        market: &MarketContext,
        as_of: Date,
    ) -> Result<SensitivityMatrix>;
}
```

- [ ] **Step 2: Register in mod.rs**

- [ ] **Step 3: Build to verify compilation**

Run: `cargo build -p finstack-valuations`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/factor_model/sensitivity/
git commit -m "feat(factor-model): add FactorSensitivityEngine trait"
```

---

## Task 4: Implement `DeltaBasedEngine`

**Files:**

- Create: `finstack/valuations/src/factor_model/sensitivity/delta_engine.rs`
- Modify: `finstack/valuations/src/factor_model/sensitivity/mod.rs`

**Context:** This engine uses `MarketContext::bump()` (at `finstack/core/src/market_data/context/ops_bump.rs:33`) and `instrument.value(market, as_of)` to compute finite-difference sensitivities. The `MarketMapping` enum tells it what to bump. `BumpSpec` is at `finstack/core/src/market_data/bumps.rs:96-108`.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::factor_model::{
        BumpSizeConfig, FactorDefinition, FactorId, FactorType, MarketMapping,
    };
    use finstack_core::market_data::bumps::BumpUnits;
    use finstack_core::types::id::CurveId;
    // Will need test fixtures: a simple Bond instrument + MarketContext with a discount curve

    #[test]
    fn test_mapping_to_bump_spec_curve_parallel() {
        let mapping = MarketMapping::CurveParallel {
            curve_ids: vec![CurveId::new("USD-OIS")],
            units: BumpUnits::RateBp,
        };
        let bump_size = 1.0;
        let specs = mapping_to_bump_specs(&mapping, bump_size);
        assert_eq!(specs.len(), 1);
        // Verify bump spec has correct value and units
    }
}
```

Note: Full integration testing of the engine requires constructing real instruments and market data. Write a helper function `mapping_to_bump_specs()` first, test it in isolation, then test the full engine with a simple Bond + DiscountCurve fixture from the existing test infrastructure.

- [ ] **Step 2: Run test to verify it fails**

- [ ] **Step 3: Implement the mapping-to-BumpSpec conversion**

```rust
use finstack_core::factor_model::{BumpSizeConfig, FactorDefinition, MarketMapping};
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits};

/// Convert a MarketMapping + bump size into concrete BumpSpecs for the MarketContext.
pub(crate) fn mapping_to_bump_specs(mapping: &MarketMapping, bump_size: f64) -> Vec<BumpSpec> {
    match mapping {
        MarketMapping::CurveParallel { units, .. } => {
            vec![BumpSpec {
                mode: BumpMode::Additive,
                units: *units,
                value: bump_size,
                bump_type: BumpType::Parallel,
            }]
        }
        MarketMapping::CurveBucketed { tenor_weights, .. } => {
            // For bucketed, apply weighted bumps at each tenor
            // This produces one BumpSpec per tenor bucket
            tenor_weights
                .iter()
                .enumerate()
                .map(|(idx, (_tenor, weight))| BumpSpec {
                    mode: BumpMode::Additive,
                    units: BumpUnits::RateBp,
                    value: bump_size * weight,
                    bump_type: BumpType::TriangularKeyRate {
                        bucket_idx: idx,
                        neighbors: 1,
                    },
                })
                .collect()
        }
        MarketMapping::EquitySpot { .. } => {
            vec![BumpSpec {
                mode: BumpMode::Multiplicative,
                units: BumpUnits::Percent,
                value: bump_size,
                bump_type: BumpType::Parallel,
            }]
        }
        MarketMapping::FxRate { .. } => {
            vec![BumpSpec {
                mode: BumpMode::Multiplicative,
                units: BumpUnits::Percent,
                value: bump_size,
                bump_type: BumpType::Parallel,
            }]
        }
        MarketMapping::VolShift { units, .. } => {
            vec![BumpSpec {
                mode: BumpMode::Additive,
                units: *units,
                value: bump_size,
                bump_type: BumpType::Parallel,
            }]
        }
        MarketMapping::Custom(f) => f(bump_size),
    }
}
```

- [ ] **Step 4: Implement DeltaBasedEngine**

```rust
use super::matrix::SensitivityMatrix;
use super::traits::FactorSensitivityEngine;
use crate::instruments::common::traits::Instrument;
use finstack_core::factor_model::{BumpSizeConfig, FactorDefinition};
use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use time::Date;

pub struct DeltaBasedEngine {
    bump_config: BumpSizeConfig,
}

impl DeltaBasedEngine {
    pub fn new(bump_config: BumpSizeConfig) -> Self {
        Self { bump_config }
    }
}

impl FactorSensitivityEngine for DeltaBasedEngine {
    fn compute_sensitivities(
        &self,
        positions: &[(String, &dyn Instrument, f64)],
        factors: &[FactorDefinition],
        market: &MarketContext,
        as_of: Date,
    ) -> Result<SensitivityMatrix> {
        let position_ids: Vec<String> = positions.iter().map(|(id, _, _)| id.clone()).collect();
        let factor_ids: Vec<_> = factors.iter().map(|f| f.id.clone()).collect();
        let mut matrix = SensitivityMatrix::zeros(position_ids, factor_ids);

        for (fi, factor) in factors.iter().enumerate() {
            let bump_size = self.bump_config.bump_size_for_factor(&factor.id, &factor.factor_type);
            let bump_specs = mapping_to_bump_specs(&factor.market_mapping, bump_size);

            // Build bumped market context (up)
            let market_up = market.bump(bump_specs.iter().cloned())?;

            // Build bumped market context (down) for central differencing
            let down_specs = mapping_to_bump_specs(&factor.market_mapping, -bump_size);
            let market_down = market.bump(down_specs.iter().cloned())?;

            for (pi, (_, instrument, weight)) in positions.iter().enumerate() {
                let pv_up = instrument.value(&market_up, as_of)?;
                let pv_down = instrument.value(&market_down, as_of)?;

                // Central difference: delta = (PV_up - PV_down) / (2 * bump_size) * weight
                let delta = (pv_up.amount() - pv_down.amount()) / (2.0 * bump_size) * weight;
                matrix.set_delta(pi, fi, delta);
            }
        }

        Ok(matrix)
    }
}
```

Note: The actual implementation may need adjustments based on exactly how `MarketContext::bump()` accepts `BumpSpec`s (it may need curve IDs associated with the specs). Consult `ops_bump.rs` for the exact API. The `instrument.value()` returns `Money` — use `.amount()` to get `f64`. Currency conversion should be done at the portfolio level, not here.

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-valuations factor_model::sensitivity --no-default-features`
Expected: Tests PASS

- [ ] **Step 6: Write an integration test with a real Bond**

Create `finstack/valuations/tests/factor_model/delta_engine_test.rs` using the existing Bond test fixtures (look at `finstack/valuations/tests/` for examples of building a Bond + MarketContext). Verify that the computed delta for a bond under a rates factor matches the bond's DV01.

- [ ] **Step 7: Run integration test**

Run: `cargo test -p finstack-valuations --test factor_model`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add finstack/valuations/src/factor_model/sensitivity/ finstack/valuations/tests/
git commit -m "feat(factor-model): add DeltaBasedEngine with finite-difference sensitivities"
```

---

## Task 5: Implement `FullRepricingEngine`

**Files:**

- Create: `finstack/valuations/src/factor_model/sensitivity/repricing_engine.rs`
- Modify: `finstack/valuations/src/factor_model/sensitivity/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_grid_construction() {
        let grid = ScenarioGrid::new(5); // -2σ, -1σ, 0, +1σ, +2σ
        assert_eq!(grid.shifts().len(), 5);
        assert!((grid.shifts()[2]).abs() < 1e-12); // center is 0
    }
}
```

- [ ] **Step 2: Implement FullRepricingEngine**

```rust
use super::delta_engine::mapping_to_bump_specs;
use super::matrix::SensitivityMatrix;
use super::traits::FactorSensitivityEngine;
use crate::instruments::common::traits::Instrument;
use finstack_core::factor_model::{BumpSizeConfig, FactorDefinition, FactorId};
use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use time::Date;

/// P&L profile for one factor across multiple shift sizes.
#[derive(Debug, Clone)]
pub struct FactorPnlProfile {
    pub factor_id: FactorId,
    pub shifts: Vec<f64>,                    // e.g., [-2.0, -1.0, 0.0, 1.0, 2.0] in σ units
    pub position_pnls: Vec<Vec<f64>>,        // [shift_idx][position_idx]
}

pub struct ScenarioGrid {
    shifts: Vec<f64>,
}

impl ScenarioGrid {
    pub fn new(n_points: usize) -> Self {
        let half = (n_points / 2) as f64;
        let shifts: Vec<f64> = (0..n_points)
            .map(|i| i as f64 - half)
            .collect();
        Self { shifts }
    }

    pub fn shifts(&self) -> &[f64] {
        &self.shifts
    }
}

pub struct FullRepricingEngine {
    bump_config: BumpSizeConfig,
    scenario_grid: ScenarioGrid,
}

impl FullRepricingEngine {
    pub fn new(bump_config: BumpSizeConfig, n_scenario_points: usize) -> Self {
        Self {
            bump_config,
            scenario_grid: ScenarioGrid::new(n_scenario_points),
        }
    }

    /// Compute full P&L profiles for each factor.
    pub fn compute_pnl_profiles(
        &self,
        positions: &[(String, &dyn Instrument, f64)],
        factors: &[FactorDefinition],
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Vec<FactorPnlProfile>> {
        // Base PVs
        let base_pvs: Vec<f64> = positions
            .iter()
            .map(|(_, inst, _)| inst.value(market, as_of).map(|m| m.amount()))
            .collect::<Result<Vec<_>>>()?;

        let mut profiles = Vec::with_capacity(factors.len());

        for factor in factors {
            let bump_size = self.bump_config.bump_size_for_factor(&factor.id, &factor.factor_type);
            let mut position_pnls = Vec::with_capacity(self.scenario_grid.shifts().len());

            for &shift in self.scenario_grid.shifts() {
                let specs = mapping_to_bump_specs(&factor.market_mapping, bump_size * shift);
                let bumped_market = market.bump(specs.iter().cloned())?;

                let pnls: Vec<f64> = positions
                    .iter()
                    .enumerate()
                    .map(|(pi, (_, inst, weight))| {
                        let pv = inst.value(&bumped_market, as_of).map(|m| m.amount())?;
                        Ok((pv - base_pvs[pi]) * weight)
                    })
                    .collect::<Result<Vec<_>>>()?;

                position_pnls.push(pnls);
            }

            profiles.push(FactorPnlProfile {
                factor_id: factor.id.clone(),
                shifts: self.scenario_grid.shifts().to_vec(),
                position_pnls,
            });
        }

        Ok(profiles)
    }
}

impl FactorSensitivityEngine for FullRepricingEngine {
    fn compute_sensitivities(
        &self,
        positions: &[(String, &dyn Instrument, f64)],
        factors: &[FactorDefinition],
        market: &MarketContext,
        as_of: Date,
    ) -> Result<SensitivityMatrix> {
        // For the sensitivity matrix, extract linear delta from the P&L profiles
        // using central difference at the ±1 shift points
        let profiles = self.compute_pnl_profiles(positions, factors, market, as_of)?;

        let position_ids: Vec<String> = positions.iter().map(|(id, _, _)| id.clone()).collect();
        let factor_ids: Vec<_> = factors.iter().map(|f| f.id.clone()).collect();
        let mut matrix = SensitivityMatrix::zeros(position_ids, factor_ids);

        for (fi, profile) in profiles.iter().enumerate() {
            // Find the -1 and +1 shift indices
            let down_idx = profile.shifts.iter().position(|&s| (s - (-1.0)).abs() < 1e-10);
            let up_idx = profile.shifts.iter().position(|&s| (s - 1.0).abs() < 1e-10);

            if let (Some(di), Some(ui)) = (down_idx, up_idx) {
                for pi in 0..positions.len() {
                    let delta = (profile.position_pnls[ui][pi] - profile.position_pnls[di][pi]) / 2.0;
                    matrix.set_delta(pi, fi, delta);
                }
            }
        }

        Ok(matrix)
    }
}
```

- [ ] **Step 3: Register in mod.rs**

- [ ] **Step 4: Run tests**

Run: `cargo test -p finstack-valuations factor_model::sensitivity --no-default-features`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/src/factor_model/sensitivity/
git commit -m "feat(factor-model): add FullRepricingEngine with scenario P&L profiles"
```

---

## Task 6: Run full workspace build and tests

- [ ] **Step 1: Build workspace**

Run: `cargo build --workspace`
Expected: SUCCESS

- [ ] **Step 2: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass (existing + new)

- [ ] **Step 3: Commit any fixes if needed**
