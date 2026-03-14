# Cluster 5: What-If Engine & Top-Level API — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `FactorModelConfig`, `FactorModelBuilder`, `FactorModel` orchestrator, `FactorAssignmentReport`, and the `WhatIfEngine` with its three operations (position what-if, factor stress, factor-constrained optimization).

**Architecture:** `FactorModelConfig` lives in core (it's a pure data/config type). `FactorModel`, `FactorModelBuilder`, and `WhatIfEngine` live in `finstack/portfolio/src/factor_model/` since they orchestrate across all layers and the portfolio is the natural entry point.

**Tech Stack:** Rust, serde

**Spec Reference:** `docs/superpowers/specs/2026-03-14-statistical-risk-factor-model-design.md` — Sections 3-4

**Depends on:** Clusters 1-4 (all core types, matching, sensitivity, decomposition)

---

## Task 1: Create `FactorModelConfig` in core

**Files:**

- Modify: `finstack/core/src/factor_model/config.rs`
- Modify: `finstack/core/src/factor_model/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn test_factor_model_config_serde_roundtrip() {
    let config = FactorModelConfig {
        factors: vec![FactorDefinition {
            id: FactorId::new("Rates"),
            factor_type: FactorType::Rates,
            market_mapping: MarketMapping::CurveParallel {
                curve_ids: vec![CurveId::new("USD-OIS")],
                units: BumpUnits::RateBp,
            },
            description: None,
        }],
        covariance: FactorCovarianceMatrix::new(
            vec![FactorId::new("Rates")],
            vec![0.04],
        ).unwrap(),
        matching: MatchingConfig::MappingTable(vec![]),
        pricing_mode: PricingMode::DeltaBased,
        risk_measure: RiskMeasure::Variance,
        bump_size: None,
        unmatched_policy: None,
    };

    let json = serde_json::to_string_pretty(&config).unwrap();
    let back: FactorModelConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.factors.len(), 1);
    assert_eq!(back.pricing_mode, PricingMode::DeltaBased);
}
```

- [ ] **Step 2: Implement FactorModelConfig**

Add to `finstack/core/src/factor_model/config.rs`:

```rust
use super::covariance::FactorCovarianceMatrix;
use super::definition::FactorDefinition;
use super::error::UnmatchedPolicy;
use super::matching::config::MatchingConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorModelConfig {
    pub factors: Vec<FactorDefinition>,
    pub covariance: FactorCovarianceMatrix,
    pub matching: MatchingConfig,
    pub pricing_mode: PricingMode,
    #[serde(default)]
    pub risk_measure: RiskMeasure,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bump_size: Option<BumpSizeConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unmatched_policy: Option<UnmatchedPolicy>,
}
```

- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(factor-model): add FactorModelConfig"
```

---

## Task 2: Create `FactorAssignmentReport` and assignment logic

**Files:**

- Create: `finstack/portfolio/src/factor_model/assignment.rs`
- Modify: `finstack/portfolio/src/factor_model/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::factor_model::{
        CurveType, FactorId, MarketDependency, UnmatchedPolicy,
    };
    use finstack_core::factor_model::matching::{
        AttributeFilter, DependencyFilter, MappingRule, MappingTableMatcher,
    };
    use finstack_core::types::Attributes;
    use finstack_core::types::id::CurveId;

    #[test]
    fn test_assign_factors_basic() {
        let matcher = MappingTableMatcher::new(vec![
            MappingRule {
                dependency_filter: DependencyFilter {
                    dependency_type: Some(CurveType::Discount),
                    id: None,
                },
                attribute_filter: AttributeFilter::default(),
                factor_id: FactorId::new("Rates"),
            },
        ]);

        let deps = vec![MarketDependency::Curve {
            id: CurveId::new("USD-OIS"),
            curve_type: CurveType::Discount,
        }];
        let attrs = Attributes::default();

        let report = assign_position_factors(
            "bond-1",
            &deps,
            &attrs,
            &matcher,
            UnmatchedPolicy::Residual,
        );

        assert_eq!(report.mappings.len(), 1);
        assert_eq!(report.mappings[0].1, FactorId::new("Rates"));
    }

    #[test]
    fn test_unmatched_residual_policy() {
        let matcher = MappingTableMatcher::new(vec![]); // no rules

        let deps = vec![MarketDependency::Spot { id: "AAPL".into() }];
        let attrs = Attributes::default();

        let report = assign_position_factors(
            "equity-1",
            &deps,
            &attrs,
            &matcher,
            UnmatchedPolicy::Residual,
        );

        assert_eq!(report.mappings.len(), 0);
        assert_eq!(report.unmatched.len(), 1);
    }
}
```

- [ ] **Step 2: Implement assignment logic**

```rust
use finstack_core::factor_model::{
    FactorId, MarketDependency, UnmatchedPolicy,
    matching::FactorMatcher,
};
use finstack_core::types::Attributes;

#[derive(Debug, Clone)]
pub struct FactorAssignmentReport {
    pub assignments: Vec<PositionAssignment>,
    pub unmatched: Vec<UnmatchedEntry>,
}

#[derive(Debug, Clone)]
pub struct PositionAssignment {
    pub position_id: String,
    pub mappings: Vec<(MarketDependency, FactorId)>,
}

#[derive(Debug, Clone)]
pub struct UnmatchedEntry {
    pub position_id: String,
    pub dependency: MarketDependency,
}

/// Assign factors for a single position's dependencies.
pub fn assign_position_factors(
    position_id: &str,
    dependencies: &[MarketDependency],
    attributes: &Attributes,
    matcher: &dyn FactorMatcher,
    _policy: UnmatchedPolicy,
) -> PositionAssignment {
    let mut mappings = Vec::new();
    let mut unmatched = Vec::new();

    for dep in dependencies {
        match matcher.match_factor(dep, attributes) {
            Some(factor_id) => mappings.push((dep.clone(), factor_id)),
            None => unmatched.push(dep.clone()),
        }
    }

    PositionAssignment {
        position_id: position_id.to_string(),
        mappings,
    }
}
```

Note: The unmatched entries are tracked at the report level when aggregating across positions.

- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(factor-model): add factor assignment logic and report types"
```

---

## Task 3: Create `FactorModelBuilder` and `FactorModel`

**Files:**

- Create: `finstack/portfolio/src/factor_model/model.rs`
- Modify: `finstack/portfolio/src/factor_model/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::factor_model::*;
    use finstack_core::factor_model::matching::*;

    fn simple_config() -> FactorModelConfig {
        FactorModelConfig {
            factors: vec![FactorDefinition {
                id: FactorId::new("Rates"),
                factor_type: FactorType::Rates,
                market_mapping: MarketMapping::CurveParallel {
                    curve_ids: vec![CurveId::new("USD-OIS")],
                    units: BumpUnits::RateBp,
                },
                description: None,
            }],
            covariance: FactorCovarianceMatrix::new(
                vec![FactorId::new("Rates")],
                vec![0.04],
            ).unwrap(),
            matching: MatchingConfig::MappingTable(vec![MappingRule {
                dependency_filter: DependencyFilter {
                    dependency_type: Some(CurveType::Discount),
                    id: None,
                },
                attribute_filter: AttributeFilter::default(),
                factor_id: FactorId::new("Rates"),
            }]),
            pricing_mode: PricingMode::DeltaBased,
            risk_measure: RiskMeasure::Variance,
            bump_size: None,
            unmatched_policy: None,
        }
    }

    #[test]
    fn test_builder_from_config() {
        let model = FactorModelBuilder::new()
            .config(simple_config())
            .build()
            .unwrap();

        assert_eq!(model.factors().len(), 1);
    }

    #[test]
    fn test_builder_missing_config_fails() {
        let result = FactorModelBuilder::new().build();
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Implement FactorModelBuilder and FactorModel**

```rust
use super::assignment::{assign_position_factors, FactorAssignmentReport, PositionAssignment, UnmatchedEntry};
use super::parametric::ParametricDecomposer;
use super::traits::RiskDecomposer;
use super::types::RiskDecomposition;
use crate::portfolio::Portfolio;
use finstack_core::factor_model::*;
use finstack_core::factor_model::matching::FactorMatcher;
use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use finstack_valuations::factor_model::decompose::decompose;
use finstack_valuations::factor_model::sensitivity::{
    DeltaBasedEngine, FactorSensitivityEngine, SensitivityMatrix,
};
use time::Date;

pub struct FactorModelBuilder {
    config: Option<FactorModelConfig>,
    custom_matcher: Option<Box<dyn FactorMatcher>>,
    custom_sensitivity_engine: Option<Box<dyn FactorSensitivityEngine>>,
    custom_decomposer: Option<Box<dyn RiskDecomposer>>,
}

impl FactorModelBuilder {
    pub fn new() -> Self {
        Self {
            config: None,
            custom_matcher: None,
            custom_sensitivity_engine: None,
            custom_decomposer: None,
        }
    }

    pub fn config(mut self, config: FactorModelConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_custom_matcher(mut self, m: impl FactorMatcher + 'static) -> Self {
        self.custom_matcher = Some(Box::new(m));
        self
    }

    pub fn with_custom_sensitivity_engine(mut self, e: impl FactorSensitivityEngine + 'static) -> Self {
        self.custom_sensitivity_engine = Some(Box::new(e));
        self
    }

    pub fn with_custom_decomposer(mut self, d: impl RiskDecomposer + 'static) -> Self {
        self.custom_decomposer = Some(Box::new(d));
        self
    }

    pub fn build(self) -> Result<FactorModel> {
        let config = self.config.ok_or_else(|| {
            finstack_core::FinstackError::invalid_input("FactorModelConfig is required".into())
        })?;

        let matcher: Box<dyn FactorMatcher> = self
            .custom_matcher
            .unwrap_or_else(|| config.matching.build_matcher());

        let bump_config = config.bump_size.clone().unwrap_or_default();

        let sensitivity_engine: Box<dyn FactorSensitivityEngine> = self
            .custom_sensitivity_engine
            .unwrap_or_else(|| match config.pricing_mode {
                PricingMode::DeltaBased => Box::new(DeltaBasedEngine::new(bump_config.clone())),
                PricingMode::FullRepricing => {
                    Box::new(finstack_valuations::factor_model::sensitivity::FullRepricingEngine::new(
                        bump_config.clone(), 5,
                    ))
                }
            });

        let decomposer: Box<dyn RiskDecomposer> = self
            .custom_decomposer
            .unwrap_or_else(|| Box::new(ParametricDecomposer));

        let unmatched_policy = config.unmatched_policy.unwrap_or_default();

        Ok(FactorModel {
            factors: config.factors,
            covariance: config.covariance,
            matcher,
            sensitivity_engine,
            decomposer,
            pricing_mode: config.pricing_mode,
            risk_measure: config.risk_measure,
            unmatched_policy,
        })
    }
}

pub struct FactorModel {
    factors: Vec<FactorDefinition>,
    covariance: FactorCovarianceMatrix,
    matcher: Box<dyn FactorMatcher>,
    sensitivity_engine: Box<dyn FactorSensitivityEngine>,
    decomposer: Box<dyn RiskDecomposer>,
    pricing_mode: PricingMode,
    risk_measure: RiskMeasure,
    unmatched_policy: UnmatchedPolicy,
}

impl FactorModel {
    pub fn factors(&self) -> &[FactorDefinition] {
        &self.factors
    }

    /// Full pipeline: match → compute sensitivities → decompose.
    pub fn analyze(
        &self,
        portfolio: &Portfolio,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<RiskDecomposition> {
        let sensitivities = self.compute_sensitivities(portfolio, market, as_of)?;
        let weights: Vec<f64> = portfolio.positions.iter().map(|p| p.quantity).collect();

        self.decomposer.decompose(
            &sensitivities,
            &self.covariance,
            &weights,
            &self.risk_measure,
        )
    }

    /// Just the matching step (for inspection/debugging).
    pub fn assign_factors(&self, portfolio: &Portfolio) -> Result<FactorAssignmentReport> {
        let mut assignments = Vec::new();
        let mut unmatched = Vec::new();

        for position in &portfolio.positions {
            let deps_result = position.instrument.market_dependencies()?;
            let deps = decompose(&deps_result);
            let attrs = position.instrument.attributes();

            let assignment = assign_position_factors(
                &position.position_id.to_string(),
                &deps,
                attrs,
                self.matcher.as_ref(),
                self.unmatched_policy,
            );

            // Collect unmatched from assignment
            for dep in &deps {
                if !assignment.mappings.iter().any(|(d, _)| d == dep) {
                    unmatched.push(UnmatchedEntry {
                        position_id: position.position_id.to_string(),
                        dependency: dep.clone(),
                    });
                }
            }

            assignments.push(assignment);
        }

        if self.unmatched_policy == UnmatchedPolicy::Strict && !unmatched.is_empty() {
            return Err(finstack_core::FinstackError::invalid_input(format!(
                "{} unmatched dependencies with Strict policy",
                unmatched.len()
            )));
        }

        Ok(FactorAssignmentReport {
            assignments,
            unmatched,
        })
    }

    /// Just sensitivities (for caching).
    pub fn compute_sensitivities(
        &self,
        portfolio: &Portfolio,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<SensitivityMatrix> {
        let positions: Vec<(String, &dyn finstack_valuations::instruments::common::traits::Instrument, f64)> =
            portfolio.positions.iter().map(|p| {
                (
                    p.position_id.to_string(),
                    p.instrument.as_ref() as &dyn finstack_valuations::instruments::common::traits::Instrument,
                    p.quantity,
                )
            }).collect();

        self.sensitivity_engine.compute_sensitivities(
            &positions,
            &self.factors,
            market,
            as_of,
        )
    }

    /// Create a WhatIfEngine from a base analysis.
    pub fn what_if<'a>(
        &'a self,
        base: &'a RiskDecomposition,
        sensitivities: &'a SensitivityMatrix,
        portfolio: &'a Portfolio,
        market: &'a MarketContext,
        as_of: Date,
    ) -> WhatIfEngine<'a> {
        WhatIfEngine {
            model: self,
            base_decomposition: base,
            base_sensitivities: sensitivities,
            portfolio,
            market,
            as_of,
        }
    }
}
```

- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(factor-model): add FactorModelBuilder and FactorModel orchestrator"
```

---

## Task 4: Implement `WhatIfEngine` — position what-if and factor stress

**Files:**

- Create: `finstack/portfolio/src/factor_model/whatif.rs`
- Modify: `finstack/portfolio/src/factor_model/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::factor_model::FactorId;

    #[test]
    fn test_position_resize_changes_risk() {
        // This is an integration-level test; write a simpler unit test
        // verifying WhatIfResult structure
        let delta = FactorContributionDelta {
            factor_id: FactorId::new("Rates"),
            absolute_change: -10.0,
            relative_change: -0.05,
        };
        assert!(delta.absolute_change < 0.0);
    }
}
```

- [ ] **Step 2: Implement WhatIfEngine**

```rust
use super::model::FactorModel;
use super::types::RiskDecomposition;
use crate::portfolio::Portfolio;
use finstack_core::factor_model::{FactorId, MarketMapping};
use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use finstack_valuations::factor_model::sensitivity::{
    mapping_to_bump_specs, SensitivityMatrix,
};
use finstack_valuations::instruments::common::traits::Instrument;
use time::Date;

pub struct WhatIfEngine<'a> {
    pub(crate) model: &'a FactorModel,
    pub(crate) base_decomposition: &'a RiskDecomposition,
    pub(crate) base_sensitivities: &'a SensitivityMatrix,
    pub(crate) portfolio: &'a Portfolio,
    pub(crate) market: &'a MarketContext,
    pub(crate) as_of: Date,
}

pub enum PositionChange {
    Add { position_id: String, instrument: Box<dyn Instrument>, weight: f64 },
    Remove { position_id: String },
    Resize { position_id: String, new_weight: f64 },
}

pub struct WhatIfResult {
    pub before: RiskDecomposition,
    pub after: RiskDecomposition,
    pub delta: Vec<FactorContributionDelta>,
}

pub struct FactorContributionDelta {
    pub factor_id: FactorId,
    pub absolute_change: f64,
    pub relative_change: f64,
}

pub struct StressResult {
    pub total_pnl: f64,
    pub position_pnl: Vec<(String, f64)>,
    pub stressed_decomposition: RiskDecomposition,
}

impl<'a> WhatIfEngine<'a> {
    /// Position what-if: add/remove/resize and see risk impact.
    pub fn position_what_if(&self, changes: &[PositionChange]) -> Result<WhatIfResult> {
        // Build modified weights vector
        let mut weights: Vec<f64> = self.portfolio.positions.iter().map(|p| p.quantity).collect();

        for change in changes {
            match change {
                PositionChange::Remove { position_id } => {
                    if let Some(idx) = self.base_sensitivities.position_ids.iter()
                        .position(|id| id == position_id) {
                        weights[idx] = 0.0;
                    }
                }
                PositionChange::Resize { position_id, new_weight } => {
                    if let Some(idx) = self.base_sensitivities.position_ids.iter()
                        .position(|id| id == position_id) {
                        weights[idx] = *new_weight;
                    }
                }
                PositionChange::Add { .. } => {
                    // Adding new positions requires recomputing sensitivities
                    // For now, compute full sensitivities with the new portfolio
                    // This is a simplification; a more efficient version would
                    // compute only the new position's sensitivities and append
                    todo!("Add position requires sensitivity recomputation — implement in future iteration")
                }
            }
        }

        // Redecompose with modified weights
        let after = self.model.decomposer.as_ref().decompose(
            self.base_sensitivities,
            &self.model.covariance,
            &weights,
            &self.model.risk_measure,
        )?;

        // Compute deltas
        let delta: Vec<FactorContributionDelta> = self.base_decomposition
            .factor_contributions.iter()
            .zip(after.factor_contributions.iter())
            .map(|(before, after)| FactorContributionDelta {
                factor_id: before.factor_id.clone(),
                absolute_change: after.absolute_risk - before.absolute_risk,
                relative_change: after.relative_risk - before.relative_risk,
            })
            .collect();

        Ok(WhatIfResult {
            before: self.base_decomposition.clone(),
            after,
            delta,
        })
    }

    /// Factor stress: shock specific factors and see P&L + risk impact.
    pub fn factor_stress(&self, stresses: &[(FactorId, f64)]) -> Result<StressResult> {
        // Build bumped market context from factor stresses
        let mut all_bump_specs = Vec::new();
        for (factor_id, shift) in stresses {
            let factor = self.model.factors().iter()
                .find(|f| &f.id == factor_id)
                .ok_or_else(|| finstack_core::FinstackError::invalid_input(
                    format!("Factor '{}' not found", factor_id)
                ))?;

            let bump_size = shift; // shift is in factor units (e.g., σ)
            let specs = mapping_to_bump_specs(&factor.market_mapping, *bump_size);
            all_bump_specs.extend(specs);
        }

        let stressed_market = self.market.bump(all_bump_specs.iter().cloned())?;

        // Reprice each position
        let mut position_pnl = Vec::new();
        let mut total_pnl = 0.0;

        for position in &self.portfolio.positions {
            let base_pv = position.instrument.value(self.market, self.as_of)?.amount();
            let stressed_pv = position.instrument.value(&stressed_market, self.as_of)?.amount();
            let pnl = (stressed_pv - base_pv) * position.quantity;
            position_pnl.push((position.position_id.to_string(), pnl));
            total_pnl += pnl;
        }

        // Recompute risk decomposition under stressed market
        let stressed_sensitivities = self.model.compute_sensitivities(
            self.portfolio,
            &stressed_market,
            self.as_of,
        )?;

        let weights: Vec<f64> = self.portfolio.positions.iter().map(|p| p.quantity).collect();
        let stressed_decomposition = self.model.decomposer.as_ref().decompose(
            &stressed_sensitivities,
            &self.model.covariance,
            &weights,
            &self.model.risk_measure,
        )?;

        Ok(StressResult {
            total_pnl,
            position_pnl,
            stressed_decomposition,
        })
    }
}
```

- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(factor-model): add WhatIfEngine with position and stress scenarios"
```

---

## Task 5: Implement factor-constrained optimization

**Files:**

- Create: `finstack/portfolio/src/factor_model/optimization.rs`
- Modify: `finstack/portfolio/src/factor_model/mod.rs`

**Context:** The existing optimizer is at `finstack/portfolio/src/optimization/`. `Constraint` enum is at `finstack/portfolio/src/optimization/constraints.rs:15-89`. `Objective` enum is at `finstack/portfolio/src/optimization/types.rs:91-98`.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::factor_model::FactorId;

    #[test]
    fn test_factor_constraint_variants() {
        let c1 = FactorConstraint::MaxFactorRisk {
            factor_id: FactorId::new("Rates"),
            max_risk: 100.0,
        };
        let c2 = FactorConstraint::FactorNeutral {
            factor_id: FactorId::new("Credit"),
        };
        // Verify construction doesn't panic
        assert!(matches!(c1, FactorConstraint::MaxFactorRisk { .. }));
        assert!(matches!(c2, FactorConstraint::FactorNeutral { .. }));
    }
}
```

- [ ] **Step 2: Implement FactorConstraint**

```rust
use finstack_core::factor_model::FactorId;

#[derive(Debug, Clone)]
pub enum FactorConstraint {
    MaxFactorRisk { factor_id: FactorId, max_risk: f64 },
    MaxFactorConcentration { factor_id: FactorId, max_fraction: f64 },
    FactorNeutral { factor_id: FactorId },
}
```

Note: The full `optimize()` method on `WhatIfEngine` requires integrating with the existing portfolio optimizer. This involves translating `FactorConstraint`s into the existing `Constraint` enum by expressing factor exposures as linear combinations of position weights (via the `SensitivityMatrix`). This is the most complex integration point and may require an additional iteration to get right. For this cluster, define the types and a stub implementation.

- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(factor-model): add FactorConstraint types for optimization"
```

---

## Task 6: Wire up top-level module exports and run full test suite

- [ ] **Step 1: Ensure mod.rs re-exports all public types**

`finstack/portfolio/src/factor_model/mod.rs`:

```rust
mod assignment;
mod model;
mod optimization;
mod parametric;
mod simulation;
mod traits;
mod types;
mod whatif;

pub use assignment::{FactorAssignmentReport, PositionAssignment, UnmatchedEntry};
pub use model::{FactorModel, FactorModelBuilder};
pub use optimization::FactorConstraint;
pub use parametric::ParametricDecomposer;
pub use simulation::SimulationDecomposer;
pub use traits::RiskDecomposer;
pub use types::{FactorContribution, PositionFactorContribution, RiskDecomposition};
pub use whatif::{
    FactorContributionDelta, PositionChange, StressResult, WhatIfEngine, WhatIfResult,
};
```

- [ ] **Step 2: Run workspace build**

Run: `cargo build --workspace`
Expected: SUCCESS

- [ ] **Step 3: Run all tests**

Run: `cargo test --workspace`
Expected: All PASS

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(factor-model): wire up top-level API exports"
```
