use super::model::FactorModel;
use super::RiskDecomposition;
use crate::error::{Error, Result};
use crate::{Portfolio, Position, PositionId};
use finstack_core::dates::Date;
use finstack_core::factor_model::FactorId;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::factor_model::mapping_to_market_bumps;
use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;

/// Base/after delta for a single factor contribution.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FactorContributionDelta {
    /// Factor identifier whose contribution changed.
    pub factor_id: FactorId,
    /// Absolute change in the reported risk contribution.
    pub absolute_change: f64,
    /// Relative change in the reported risk contribution.
    pub relative_change: f64,
}

/// Result of a position what-if scenario.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WhatIfResult {
    /// Baseline decomposition used as the comparison point.
    pub before: RiskDecomposition,
    /// Decomposition after applying the requested position changes.
    pub after: RiskDecomposition,
    /// Per-factor changes between `before` and `after`.
    pub delta: Vec<FactorContributionDelta>,
}

/// Result of a factor-stress scenario.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StressResult {
    /// Total portfolio P&L under the stressed market.
    pub total_pnl: f64,
    /// Per-position P&L contributions.
    pub position_pnl: Vec<(PositionId, f64)>,
    /// Risk decomposition recomputed under the stressed market.
    pub stressed_decomposition: RiskDecomposition,
}

/// Position edits supported by `WhatIfEngine::position_what_if`.
#[derive(Debug, Clone)]
pub enum PositionChange {
    /// Add a new position. This currently requires recomputing sensitivities from scratch.
    Add {
        /// Position to add to the scenario portfolio.
        position: Box<Position>,
    },
    /// Remove an existing position by identifier.
    Remove {
        /// Position identifier to remove from the scenario.
        position_id: PositionId,
    },
    /// Resize an existing position to a new quantity.
    Resize {
        /// Position identifier to resize.
        position_id: PositionId,
        /// Replacement quantity for the position.
        new_quantity: f64,
    },
}

/// Scenario engine built from a baseline factor-model analysis.
pub struct WhatIfEngine<'a> {
    model: &'a FactorModel,
    base_decomposition: &'a RiskDecomposition,
    base_sensitivities: &'a SensitivityMatrix,
    portfolio: &'a Portfolio,
    market: &'a MarketContext,
    as_of: Date,
}

impl<'a> WhatIfEngine<'a> {
    /// Create a what-if engine from a previously computed baseline.
    #[must_use]
    pub fn new(
        model: &'a FactorModel,
        base_decomposition: &'a RiskDecomposition,
        base_sensitivities: &'a SensitivityMatrix,
        portfolio: &'a Portfolio,
        market: &'a MarketContext,
        as_of: Date,
    ) -> Self {
        Self {
            model,
            base_decomposition,
            base_sensitivities,
            portfolio,
            market,
            as_of,
        }
    }

    /// Reallocate existing sensitivity rows to simulate remove/resize scenarios.
    pub fn position_what_if(&self, changes: &[PositionChange]) -> Result<WhatIfResult> {
        let mut sensitivities = self.base_sensitivities.clone();

        for change in changes {
            match change {
                PositionChange::Add { .. } => {
                    return Err(Error::invalid_input(
                        "PositionChange::Add is not supported yet; recompute sensitivities against a cloned Portfolio".to_string(),
                    ));
                }
                PositionChange::Remove { position_id } => {
                    let Some(position_idx) = self.position_index(position_id) else {
                        return Err(Error::invalid_input(format!(
                            "Unknown position '{}'",
                            position_id
                        )));
                    };
                    for factor_idx in 0..sensitivities.n_factors() {
                        sensitivities.set_delta(position_idx, factor_idx, 0.0);
                    }
                }
                PositionChange::Resize {
                    position_id,
                    new_quantity,
                } => {
                    let Some(position_idx) = self.position_index(position_id) else {
                        return Err(Error::invalid_input(format!(
                            "Unknown position '{}'",
                            position_id
                        )));
                    };
                    let Some(position) = self.portfolio.get_position(position_id.as_str()) else {
                        return Err(Error::invalid_input(format!(
                            "Unknown position '{}'",
                            position_id
                        )));
                    };
                    if position.quantity.abs() < f64::EPSILON {
                        return Err(Error::invalid_input(format!(
                            "Position '{}' has zero quantity and cannot be resized proportionally",
                            position_id
                        )));
                    }
                    let scale = *new_quantity / position.quantity;
                    let row = sensitivities.position_deltas(position_idx).to_vec();
                    for (factor_idx, delta) in row.into_iter().enumerate() {
                        sensitivities.set_delta(position_idx, factor_idx, delta * scale);
                    }
                }
            }
        }

        let after = self.model.decomposer().decompose(
            &sensitivities,
            self.model.covariance(),
            self.model.risk_measure(),
        )?;

        Ok(WhatIfResult {
            before: self.base_decomposition.clone(),
            delta: factor_deltas(self.base_decomposition, &after),
            after,
        })
    }

    /// Shock factors, reprice positions, and recompute the stressed decomposition.
    pub fn factor_stress(&self, stresses: &[(FactorId, f64)]) -> Result<StressResult> {
        let mut bumps = Vec::new();
        for (factor_id, shift) in stresses {
            let factor = self
                .model
                .factors()
                .iter()
                .find(|factor| factor.id == *factor_id)
                .ok_or_else(|| Error::invalid_input(format!("Unknown factor '{}'", factor_id)))?;
            bumps.extend(mapping_to_market_bumps(
                &factor.market_mapping,
                *shift,
                self.as_of,
            )?);
        }

        let stressed_market = self.market.bump(bumps)?;
        let mut position_pnl = Vec::with_capacity(self.portfolio.positions.len());
        let mut total_pnl = 0.0;

        for position in &self.portfolio.positions {
            let base_value = position.instrument.value_raw(self.market, self.as_of)?;
            let stressed_value = position
                .instrument
                .value_raw(&stressed_market, self.as_of)?;
            let pnl = (stressed_value - base_value) * position.quantity;
            position_pnl.push((position.position_id.clone(), pnl));
            total_pnl += pnl;
        }

        let stressed_decomposition =
            self.model
                .analyze(self.portfolio, &stressed_market, self.as_of)?;

        Ok(StressResult {
            total_pnl,
            position_pnl,
            stressed_decomposition,
        })
    }

    fn position_index(&self, position_id: &PositionId) -> Option<usize> {
        self.base_sensitivities
            .position_ids()
            .iter()
            .position(|current| current == position_id.as_str())
    }
}

fn factor_deltas(
    before: &RiskDecomposition,
    after: &RiskDecomposition,
) -> Vec<FactorContributionDelta> {
    let after_by_id: std::collections::HashMap<&FactorId, &super::types::FactorContribution> =
        after
            .factor_contributions
            .iter()
            .map(|fc| (&fc.factor_id, fc))
            .collect();

    before
        .factor_contributions
        .iter()
        .map(|before_factor| {
            let (abs_after, rel_after) = after_by_id
                .get(&before_factor.factor_id)
                .map(|af| (af.absolute_risk, af.relative_risk))
                .unwrap_or((0.0, 0.0));
            FactorContributionDelta {
                factor_id: before_factor.factor_id.clone(),
                absolute_change: abs_after - before_factor.absolute_risk,
                relative_change: rel_after - before_factor.relative_risk,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_model::{FactorModel, FactorModelBuilder};
    use crate::test_utils::build_test_market_at;
    use crate::{Portfolio, Position, PositionId, PositionUnit, DUMMY_ENTITY_ID};
    use finstack_core::currency::Currency;
    use finstack_core::factor_model::matching::{DependencyFilter, MappingRule, MatchingConfig};
    use finstack_core::factor_model::{
        CurveType, DependencyType, FactorCovarianceMatrix, FactorDefinition, FactorId,
        FactorModelConfig, FactorType, MarketMapping, PricingMode, RiskMeasure, UnmatchedPolicy,
    };
    use finstack_core::market_data::bumps::BumpUnits;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::money::Money;
    use finstack_core::types::{Attributes, CurveId};
    use finstack_valuations::factor_model::sensitivity::{
        FactorSensitivityEngine, SensitivityMatrix,
    };
    use finstack_valuations::instruments::common::dependencies::MarketDependencies;
    use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
    use finstack_valuations::pricer::InstrumentType;
    use std::any::Any;
    use std::sync::Arc;
    use time::macros::date;

    #[test]
    fn test_position_resize_scales_total_risk() {
        let setup = build_test_model();
        assert!(setup.is_some());
        let Some((model, portfolio, market)) = setup else {
            return;
        };
        let base_result = model.analyze(&portfolio, &market, date!(2024 - 01 - 01));
        assert!(base_result.is_ok());
        let Ok(base) = base_result else {
            return;
        };
        let sensitivities_result =
            model.compute_sensitivities(&portfolio, &market, date!(2024 - 01 - 01));
        assert!(sensitivities_result.is_ok());
        let Ok(sensitivities) = sensitivities_result else {
            return;
        };

        let result = model
            .what_if(
                &base,
                &sensitivities,
                &portfolio,
                &market,
                date!(2024 - 01 - 01),
            )
            .position_what_if(&[PositionChange::Resize {
                position_id: PositionId::new("pos-1"),
                new_quantity: 4.0,
            }]);
        assert!(result.is_ok());
        let Ok(result) = result else {
            return;
        };

        assert!(result.after.total_risk > result.before.total_risk);
    }

    #[test]
    fn test_position_remove_zeroes_risk() {
        let setup = build_test_model();
        assert!(setup.is_some());
        let Some((model, portfolio, market)) = setup else {
            return;
        };
        let base_result = model.analyze(&portfolio, &market, date!(2024 - 01 - 01));
        assert!(base_result.is_ok());
        let Ok(base) = base_result else {
            return;
        };
        let sensitivities_result =
            model.compute_sensitivities(&portfolio, &market, date!(2024 - 01 - 01));
        assert!(sensitivities_result.is_ok());
        let Ok(sensitivities) = sensitivities_result else {
            return;
        };

        let result = model
            .what_if(
                &base,
                &sensitivities,
                &portfolio,
                &market,
                date!(2024 - 01 - 01),
            )
            .position_what_if(&[PositionChange::Remove {
                position_id: PositionId::new("pos-1"),
            }]);
        assert!(result.is_ok());
        let Ok(result) = result else {
            return;
        };

        assert!((result.after.total_risk).abs() < 1e-12);
    }

    #[test]
    fn test_position_add_is_not_supported_yet() {
        let setup = build_test_model();
        assert!(setup.is_some());
        let Some((model, portfolio, market)) = setup else {
            return;
        };
        let base_result = model.analyze(&portfolio, &market, date!(2024 - 01 - 01));
        assert!(base_result.is_ok());
        let Ok(base) = base_result else {
            return;
        };
        let sensitivities_result =
            model.compute_sensitivities(&portfolio, &market, date!(2024 - 01 - 01));
        assert!(sensitivities_result.is_ok());
        let Ok(sensitivities) = sensitivities_result else {
            return;
        };

        let added_position_result = Position::new(
            "pos-2",
            DUMMY_ENTITY_ID,
            "inst-2",
            Arc::new(MockInstrument::new("inst-2", "USD-OIS", 100.0)),
            1.0,
            PositionUnit::Units,
        );
        assert!(added_position_result.is_ok());
        let Ok(added_position) = added_position_result else {
            return;
        };

        let result = model
            .what_if(
                &base,
                &sensitivities,
                &portfolio,
                &market,
                date!(2024 - 01 - 01),
            )
            .position_what_if(&[PositionChange::Add {
                position: Box::new(added_position),
            }]);
        assert!(result.is_err());
    }

    #[test]
    fn test_factor_stress_returns_position_pnl() {
        let setup = build_test_model();
        assert!(setup.is_some());
        let Some((model, portfolio, market)) = setup else {
            return;
        };
        let base_result = model.analyze(&portfolio, &market, date!(2024 - 01 - 01));
        assert!(base_result.is_ok());
        let Ok(base) = base_result else {
            return;
        };
        let sensitivities_result =
            model.compute_sensitivities(&portfolio, &market, date!(2024 - 01 - 01));
        assert!(sensitivities_result.is_ok());
        let Ok(sensitivities) = sensitivities_result else {
            return;
        };

        let stress_result = model
            .what_if(
                &base,
                &sensitivities,
                &portfolio,
                &market,
                date!(2024 - 01 - 01),
            )
            .factor_stress(&[(FactorId::new("Rates"), 1.0)]);
        assert!(stress_result.is_ok());
        let Ok(stress_result) = stress_result else {
            return;
        };

        assert_eq!(stress_result.position_pnl.len(), 1);
        assert!(stress_result.total_pnl.is_finite());
    }

    fn build_test_model() -> Option<(FactorModel, Portfolio, MarketContext)> {
        let covariance_result =
            FactorCovarianceMatrix::new(vec![FactorId::new("Rates")], vec![0.04]);
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return None;
        };

        let model_result = FactorModelBuilder::new()
            .config(FactorModelConfig {
                factors: vec![FactorDefinition {
                    id: FactorId::new("Rates"),
                    factor_type: FactorType::Rates,
                    market_mapping: MarketMapping::CurveParallel {
                        curve_ids: vec![CurveId::new("USD-OIS")],
                        units: BumpUnits::RateBp,
                    },
                    description: None,
                }],
                covariance,
                matching: MatchingConfig::MappingTable(vec![MappingRule {
                    dependency_filter: DependencyFilter {
                        dependency_type: Some(DependencyType::Discount),
                        curve_type: Some(CurveType::Discount),
                        id: None,
                    },
                    attribute_filter: finstack_core::factor_model::AttributeFilter::default(),
                    factor_id: FactorId::new("Rates"),
                }]),
                pricing_mode: PricingMode::DeltaBased,
                risk_measure: RiskMeasure::Variance,
                bump_size: None,
                unmatched_policy: Some(UnmatchedPolicy::Residual),
            })
            .with_custom_sensitivity_engine(FixedSensitivityEngine)
            .build();
        assert!(model_result.is_ok());
        let Ok(model) = model_result else {
            return None;
        };

        let position_result = Position::new(
            "pos-1",
            DUMMY_ENTITY_ID,
            "inst-1",
            Arc::new(MockInstrument::new("inst-1", "USD-OIS", 100.0)),
            2.0,
            PositionUnit::Units,
        );
        assert!(position_result.is_ok());
        let Ok(position) = position_result else {
            return None;
        };

        let mut portfolio = Portfolio::new("portfolio", Currency::USD, date!(2024 - 01 - 01));
        portfolio.positions.push(position);
        portfolio.rebuild_index();

        Some((
            model,
            portfolio,
            build_test_market_at(date!(2024 - 01 - 01)),
        ))
    }

    #[derive(Clone)]
    struct MockInstrument {
        id: String,
        attributes: Attributes,
        discount_curve: CurveId,
        scale: f64,
    }

    impl MockInstrument {
        fn new(id: &str, discount_curve: &str, scale: f64) -> Self {
            Self {
                id: id.to_string(),
                attributes: Attributes::default(),
                discount_curve: CurveId::new(discount_curve),
                scale,
            }
        }
    }

    finstack_valuations::impl_empty_cashflow_provider!(
        MockInstrument,
        finstack_valuations::cashflow::builder::CashflowRepresentation::NoResidual
    );

    impl Instrument for MockInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn key(&self) -> InstrumentType {
            InstrumentType::Bond
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn attributes(&self) -> &Attributes {
            &self.attributes
        }

        fn attributes_mut(&mut self) -> &mut Attributes {
            &mut self.attributes
        }

        fn clone_box(&self) -> Box<dyn Instrument> {
            Box::new(self.clone())
        }

        fn value(
            &self,
            market: &MarketContext,
            _as_of: finstack_core::dates::Date,
        ) -> finstack_core::Result<Money> {
            let pv = market.get_discount(self.discount_curve.as_str())?.zero(1.0) * self.scale;
            Ok(Money::new(pv, Currency::USD))
        }

        fn market_dependencies(&self) -> finstack_core::Result<MarketDependencies> {
            let mut dependencies = MarketDependencies::new();
            dependencies
                .curves
                .discount_curves
                .push(self.discount_curve.clone());
            Ok(dependencies)
        }
    }

    struct FixedSensitivityEngine;

    impl FactorSensitivityEngine for FixedSensitivityEngine {
        fn compute_sensitivities(
            &self,
            _positions: &[(String, &dyn Instrument, f64)],
            factors: &[FactorDefinition],
            _market: &MarketContext,
            _as_of: finstack_core::dates::Date,
        ) -> finstack_core::Result<SensitivityMatrix> {
            let mut matrix = SensitivityMatrix::zeros(
                vec!["pos-1".into()],
                factors.iter().map(|factor| factor.id.clone()).collect(),
            );
            if !factors.is_empty() {
                matrix.set_delta(0, 0, 10.0);
            }
            Ok(matrix)
        }
    }
}
