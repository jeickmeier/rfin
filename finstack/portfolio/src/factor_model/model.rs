use super::assignment::{assign_position_factors, FactorAssignmentReport};
use super::{ParametricDecomposer, RiskDecomposer, RiskDecomposition};
use super::whatif::WhatIfEngine;
use crate::error::{Error, Result};
use crate::Portfolio;
use finstack_core::dates::Date;
use finstack_core::factor_model::{
    BumpSizeConfig, FactorCovarianceMatrix, FactorDefinition, FactorModelConfig,
    FactorModelError, MatchingConfig, PricingMode, RiskMeasure, UnmatchedPolicy,
};
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::factor_model::decompose as flatten_dependencies;
use finstack_valuations::factor_model::sensitivity::{
    DeltaBasedEngine, FactorSensitivityEngine, FullRepricingEngine, SensitivityMatrix,
};
use finstack_valuations::instruments::common::traits::Instrument;

/// Builder for the top-level portfolio factor-model orchestrator.
pub struct FactorModelBuilder {
    config: Option<FactorModelConfig>,
    custom_matcher: Option<Box<dyn finstack_core::factor_model::FactorMatcher>>,
    custom_sensitivity_engine: Option<Box<dyn FactorSensitivityEngine>>,
    custom_decomposer: Option<Box<dyn RiskDecomposer>>,
}

impl FactorModelBuilder {
    /// Create an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: None,
            custom_matcher: None,
            custom_sensitivity_engine: None,
            custom_decomposer: None,
        }
    }

    /// Supply the declarative factor-model configuration.
    #[must_use]
    pub fn config(mut self, config: FactorModelConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Override the matcher built from `FactorModelConfig::matching`.
    #[must_use]
    pub fn with_custom_matcher(
        mut self,
        matcher: impl finstack_core::factor_model::FactorMatcher + 'static,
    ) -> Self {
        self.custom_matcher = Some(Box::new(matcher));
        self
    }

    /// Override the sensitivity engine selected from the pricing mode.
    #[must_use]
    pub fn with_custom_sensitivity_engine(
        mut self,
        sensitivity_engine: impl FactorSensitivityEngine + 'static,
    ) -> Self {
        self.custom_sensitivity_engine = Some(Box::new(sensitivity_engine));
        self
    }

    /// Override the risk decomposer used by the model.
    #[must_use]
    pub fn with_custom_decomposer(mut self, decomposer: impl RiskDecomposer + 'static) -> Self {
        self.custom_decomposer = Some(Box::new(decomposer));
        self
    }

    /// Build the configured factor model.
    pub fn build(self) -> Result<FactorModel> {
        let config = self
            .config
            .ok_or_else(|| Error::invalid_input("FactorModelConfig is required"))?;
        config.risk_measure.validate()?;
        let factor_ids: Vec<_> = config
            .factors
            .iter()
            .map(|factor| factor.id.clone())
            .collect();
        if factor_ids.as_slice() != config.covariance.factor_ids() {
            return Err(Error::invalid_input(
                "FactorModelConfig covariance axes must match factors in the same order",
            ));
        }

        let matcher = self
            .custom_matcher
            .unwrap_or_else(|| build_matcher(&config.matching));
        let bump_config = config.bump_size.clone().unwrap_or_default();
        let sensitivity_engine = self
            .custom_sensitivity_engine
            .unwrap_or_else(|| default_sensitivity_engine(config.pricing_mode, &bump_config));
        let decomposer = self
            .custom_decomposer
            .unwrap_or_else(|| Box::new(ParametricDecomposer));

        Ok(FactorModel {
            factors: config.factors,
            covariance: config.covariance,
            matcher,
            sensitivity_engine,
            decomposer,
            risk_measure: config.risk_measure,
            unmatched_policy: config.unmatched_policy.unwrap_or_default(),
        })
    }
}

impl Default for FactorModelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn build_matcher(
    config: &MatchingConfig,
) -> Box<dyn finstack_core::factor_model::FactorMatcher> {
    config.build_matcher()
}

fn default_sensitivity_engine(
    pricing_mode: PricingMode,
    bump_config: &BumpSizeConfig,
) -> Box<dyn FactorSensitivityEngine> {
    match pricing_mode {
        PricingMode::DeltaBased => Box::new(DeltaBasedEngine::new(bump_config.clone())),
        PricingMode::FullRepricing => Box::new(FullRepricingEngine::new(bump_config.clone(), 5)),
    }
}

/// Portfolio-level factor-model orchestrator.
pub struct FactorModel {
    factors: Vec<FactorDefinition>,
    covariance: FactorCovarianceMatrix,
    matcher: Box<dyn finstack_core::factor_model::FactorMatcher>,
    sensitivity_engine: Box<dyn FactorSensitivityEngine>,
    decomposer: Box<dyn RiskDecomposer>,
    risk_measure: RiskMeasure,
    unmatched_policy: UnmatchedPolicy,
}

impl FactorModel {
    /// Borrow the factor definitions configured on the model.
    #[must_use]
    pub fn factors(&self) -> &[FactorDefinition] {
        &self.factors
    }

    /// Match each position dependency in `portfolio` to configured factors.
    pub fn assign_factors(&self, portfolio: &Portfolio) -> Result<FactorAssignmentReport> {
        let mut assignments = Vec::with_capacity(portfolio.positions.len());
        let mut unmatched = Vec::new();

        for position in &portfolio.positions {
            let dependencies = flatten_dependencies(&position.instrument.market_dependencies()?);
            let (assignment, position_unmatched) = assign_position_factors(
                &position.position_id,
                &dependencies,
                position.instrument.attributes(),
                self.matcher.as_ref(),
            );

            if self.unmatched_policy == UnmatchedPolicy::Strict && !position_unmatched.is_empty() {
                let first_unmatched = &position_unmatched[0];
                let message = FactorModelError::UnmatchedDependency {
                    position_id: first_unmatched.position_id.to_string(),
                    dependency: first_unmatched.dependency.clone(),
                }
                .to_string();
                return Err(Error::invalid_input(message));
            }

            if self.unmatched_policy == UnmatchedPolicy::Warn {
                for unmatched_entry in &position_unmatched {
                    tracing::warn!(
                        position_id = %unmatched_entry.position_id,
                        dependency = ?unmatched_entry.dependency,
                        "Unmatched dependency during factor assignment"
                    );
                }
            }

            assignments.push(assignment);
            unmatched.extend(position_unmatched);
        }

        Ok(FactorAssignmentReport {
            assignments,
            unmatched,
        })
    }

    /// Compute the weighted position-factor sensitivity matrix for `portfolio`.
    pub fn compute_sensitivities(
        &self,
        portfolio: &Portfolio,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<SensitivityMatrix> {
        let _assignment_report = self.assign_factors(portfolio)?;
        let positions: Vec<(String, &dyn Instrument, f64)> = portfolio
            .positions
            .iter()
            .map(|position| {
                (
                    position.position_id.to_string(),
                    position.instrument.as_ref() as &dyn Instrument,
                    position.quantity,
                )
            })
            .collect();

        Ok(self
            .sensitivity_engine
            .compute_sensitivities(&positions, &self.factors, market, as_of)?)
    }

    /// Run the full sensitivity-plus-decomposition pipeline.
    pub fn analyze(
        &self,
        portfolio: &Portfolio,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<RiskDecomposition> {
        let sensitivities = self.compute_sensitivities(portfolio, market, as_of)?;
        Ok(self
            .decomposer
            .decompose(&sensitivities, &self.covariance, &self.risk_measure)?)
    }

    /// Create a what-if engine anchored to a base decomposition and sensitivity matrix.
    #[must_use]
    pub fn what_if<'a>(
        &'a self,
        base: &'a RiskDecomposition,
        sensitivities: &'a SensitivityMatrix,
        portfolio: &'a Portfolio,
        market: &'a MarketContext,
        as_of: Date,
    ) -> WhatIfEngine<'a> {
        WhatIfEngine::new(self, base, sensitivities, portfolio, market, as_of)
    }

    pub(crate) fn covariance(&self) -> &FactorCovarianceMatrix {
        &self.covariance
    }

    pub(crate) fn decomposer(&self) -> &dyn RiskDecomposer {
        self.decomposer.as_ref()
    }

    pub(crate) fn risk_measure(&self) -> &RiskMeasure {
        &self.risk_measure
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Portfolio, Position, PositionId, PositionUnit, DUMMY_ENTITY_ID};
    use finstack_core::currency::Currency;
    use finstack_core::factor_model::matching::{DependencyFilter, MappingRule};
    use finstack_core::factor_model::{
        BumpSizeConfig, CurveType, DependencyType, FactorCovarianceMatrix, FactorDefinition,
        FactorId, FactorModelConfig, FactorType, MarketMapping, PricingMode, RiskMeasure,
        UnmatchedPolicy,
    };
    use finstack_core::market_data::bumps::BumpUnits;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::money::Money;
    use finstack_core::types::{Attributes, CurveId};
    use finstack_valuations::factor_model::sensitivity::{FactorSensitivityEngine, SensitivityMatrix};
    use finstack_valuations::instruments::common::dependencies::MarketDependencies;
    use finstack_valuations::instruments::common::traits::Instrument;
    use finstack_valuations::pricer::InstrumentType;
    use std::any::Any;
    use std::sync::Arc;
    use time::macros::date;

    fn simple_config() -> FactorModelConfig {
        let covariance_result =
            FactorCovarianceMatrix::new(vec![FactorId::new("Rates")], vec![0.04]);
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return unreachable_config();
        };

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
            bump_size: Some(BumpSizeConfig::default()),
            unmatched_policy: Some(UnmatchedPolicy::Residual),
        }
    }

    fn unreachable_config() -> FactorModelConfig {
        FactorModelConfig {
            factors: Vec::new(),
            covariance: FactorCovarianceMatrix::new_unchecked(Vec::new(), Vec::new()),
            matching: MatchingConfig::MappingTable(Vec::new()),
            pricing_mode: PricingMode::DeltaBased,
            risk_measure: RiskMeasure::Variance,
            bump_size: None,
            unmatched_policy: None,
        }
    }

    #[test]
    fn test_builder_from_config_exposes_factors() {
        let build_result = FactorModelBuilder::new().config(simple_config()).build();
        assert!(build_result.is_ok());
        let Ok(model) = build_result else {
            return;
        };

        assert_eq!(model.factors().len(), 1);
        assert_eq!(model.factors()[0].id, FactorId::new("Rates"));
    }

    #[test]
    fn test_builder_missing_config_fails() {
        let result = FactorModelBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_rejects_covariance_axes_not_aligned_to_factors() {
        let covariance_result = FactorCovarianceMatrix::new(
            vec![FactorId::new("Credit"), FactorId::new("Rates")],
            vec![0.09, 0.01, 0.01, 0.04],
        );
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return;
        };

        let result = FactorModelBuilder::new()
            .config(FactorModelConfig {
                factors: vec![
                    FactorDefinition {
                        id: FactorId::new("Rates"),
                        factor_type: FactorType::Rates,
                        market_mapping: MarketMapping::CurveParallel {
                            curve_ids: vec![CurveId::new("USD-OIS")],
                            units: BumpUnits::RateBp,
                        },
                        description: None,
                    },
                    FactorDefinition {
                        id: FactorId::new("Credit"),
                        factor_type: FactorType::Credit,
                        market_mapping: MarketMapping::CurveParallel {
                            curve_ids: vec![CurveId::new("ACME-HAZARD")],
                            units: BumpUnits::RateBp,
                        },
                        description: None,
                    },
                ],
                covariance,
                matching: MatchingConfig::MappingTable(vec![]),
                pricing_mode: PricingMode::DeltaBased,
                risk_measure: RiskMeasure::Variance,
                bump_size: None,
                unmatched_policy: Some(UnmatchedPolicy::Residual),
            })
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_assign_factors_collects_matches_and_unmatched() {
        let build_result = FactorModelBuilder::new().config(simple_config()).build();
        assert!(build_result.is_ok());
        let Ok(model) = build_result else {
            return;
        };

        let position_result = Position::new(
            "pos-1",
            DUMMY_ENTITY_ID,
            "inst-1",
            Arc::new(MockInstrument::new("inst-1", "USD-OIS", vec!["AAPL".into()])),
            2.0,
            PositionUnit::Units,
        );
        assert!(position_result.is_ok());
        let Ok(position) = position_result else {
            return;
        };

        let mut portfolio = Portfolio::new("portfolio", Currency::USD, date!(2024 - 01 - 01));
        portfolio.positions.push(position);
        portfolio.rebuild_index();

        let report_result = model.assign_factors(&portfolio);
        assert!(report_result.is_ok());
        let Ok(report) = report_result else {
            return;
        };

        assert_eq!(report.assignments.len(), 1);
        assert_eq!(report.assignments[0].position_id, PositionId::new("pos-1"));
        assert_eq!(report.assignments[0].mappings.len(), 1);
        assert_eq!(report.assignments[0].mappings[0].1, FactorId::new("Rates"));
        assert_eq!(report.unmatched.len(), 1);
        assert_eq!(report.unmatched[0].position_id, PositionId::new("pos-1"));
    }

    #[test]
    fn test_analyze_uses_custom_sensitivity_engine_and_decomposer() {
        let covariance_result =
            FactorCovarianceMatrix::new(vec![FactorId::new("Rates")], vec![0.04]);
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return;
        };

        let expected = RiskDecomposition {
            total_risk: 2.0,
            measure: RiskMeasure::Variance,
            factor_contributions: vec![],
            residual_risk: 0.0,
            position_factor_contributions: vec![],
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
                matching: MatchingConfig::MappingTable(vec![]),
                pricing_mode: PricingMode::DeltaBased,
                risk_measure: RiskMeasure::Variance,
                bump_size: None,
                unmatched_policy: Some(UnmatchedPolicy::Residual),
            })
            .with_custom_sensitivity_engine(FixedSensitivityEngine)
            .with_custom_decomposer(FixedDecomposer(expected.clone()))
            .build();
        assert!(model_result.is_ok());
        let Ok(model) = model_result else {
            return;
        };

        let portfolio = Portfolio::new("portfolio", Currency::USD, date!(2024 - 01 - 01));
        let analysis_result = model.analyze(&portfolio, &MarketContext::new(), date!(2024 - 01 - 01));
        assert!(analysis_result.is_ok());
        let Ok(actual) = analysis_result else {
            return;
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_analyze_fails_when_strict_policy_has_unmatched_dependencies() {
        let covariance_result =
            FactorCovarianceMatrix::new(vec![FactorId::new("Rates")], vec![0.04]);
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return;
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
                matching: MatchingConfig::MappingTable(vec![]),
                pricing_mode: PricingMode::DeltaBased,
                risk_measure: RiskMeasure::Variance,
                bump_size: None,
                unmatched_policy: Some(UnmatchedPolicy::Strict),
            })
            .with_custom_sensitivity_engine(FixedSensitivityEngine)
            .with_custom_decomposer(FixedDecomposer(RiskDecomposition {
                total_risk: 0.0,
                measure: RiskMeasure::Variance,
                factor_contributions: vec![],
                residual_risk: 0.0,
                position_factor_contributions: vec![],
            }))
            .build();
        assert!(model_result.is_ok());
        let Ok(model) = model_result else {
            return;
        };

        let position_result = Position::new(
            "pos-1",
            DUMMY_ENTITY_ID,
            "inst-1",
            Arc::new(MockInstrument::new("inst-1", "USD-OIS", vec![])),
            1.0,
            PositionUnit::Units,
        );
        assert!(position_result.is_ok());
        let Ok(position) = position_result else {
            return;
        };

        let mut portfolio = Portfolio::new("portfolio", Currency::USD, date!(2024 - 01 - 01));
        portfolio.positions.push(position);
        portfolio.rebuild_index();

        let analysis_result = model.analyze(&portfolio, &MarketContext::new(), date!(2024 - 01 - 01));
        assert!(analysis_result.is_err());
    }

    #[derive(Clone)]
    struct MockInstrument {
        id: String,
        attributes: Attributes,
        discount_curve: CurveId,
        spots: Vec<String>,
    }

    impl MockInstrument {
        fn new(id: &str, discount_curve: &str, spots: Vec<String>) -> Self {
            Self {
                id: id.to_string(),
                attributes: Attributes::default(),
                discount_curve: CurveId::new(discount_curve),
                spots,
            }
        }
    }

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
            _market: &MarketContext,
            _as_of: finstack_core::dates::Date,
        ) -> finstack_core::Result<Money> {
            Ok(Money::new(100.0, Currency::USD))
        }

        fn market_dependencies(&self) -> finstack_core::Result<MarketDependencies> {
            let mut dependencies = MarketDependencies::new();
            dependencies
                .curves
                .discount_curves
                .push(self.discount_curve.clone());
            dependencies.spot_ids.extend(self.spots.iter().cloned());
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
            Ok(SensitivityMatrix::zeros(
                Vec::new(),
                factors.iter().map(|factor| factor.id.clone()).collect(),
            ))
        }
    }

    struct FixedDecomposer(RiskDecomposition);

    impl crate::factor_model::RiskDecomposer for FixedDecomposer {
        fn decompose(
            &self,
            _sensitivities: &SensitivityMatrix,
            _covariance: &FactorCovarianceMatrix,
            _measure: &RiskMeasure,
        ) -> finstack_core::Result<RiskDecomposition> {
            Ok(self.0.clone())
        }
    }
}
