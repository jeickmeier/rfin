use super::delta_engine::mapping_to_market_bumps;
use super::matrix::SensitivityMatrix;
use super::traits::FactorSensitivityEngine;
use crate::instruments::Instrument;
use finstack_core::dates::Date;
use finstack_core::factor_model::{BumpSizeConfig, FactorDefinition, FactorId};
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// P&L profile for one factor across a scenario grid.
#[derive(Debug, Clone, PartialEq)]
pub struct FactorPnlProfile {
    /// Identifier of the shocked factor.
    pub factor_id: FactorId,
    /// Scenario shift coordinates in bump-size units.
    pub shifts: Vec<f64>,
    /// Per-shift P&L vectors indexed as `[shift_idx][position_idx]`.
    pub position_pnls: Vec<Vec<f64>>,
}

/// Symmetric grid of scenario shifts used by the full repricing engine.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioGrid {
    shifts: Vec<f64>,
}

impl ScenarioGrid {
    /// Minimum number of grid points required for central-difference delta
    /// extraction (need at least -1, 0, +1).
    pub const MIN_POINTS: usize = 3;

    /// Create a grid centered on zero, e.g. `5 -> [-2, -1, 0, 1, 2]`.
    ///
    /// # Panics
    ///
    /// Panics when `n_points < 3` because the repricing engine requires
    /// shifts at -1 and +1 for central-difference delta extraction.
    #[must_use]
    pub fn new(n_points: usize) -> Self {
        assert!(
            n_points >= Self::MIN_POINTS,
            "ScenarioGrid requires at least {} points for central-difference delta extraction, got {n_points}",
            Self::MIN_POINTS,
        );
        let half = (n_points / 2) as f64;
        let shifts = (0..n_points).map(|idx| idx as f64 - half).collect();
        Self { shifts }
    }

    /// Return the ordered shift coordinates.
    #[must_use]
    pub fn shifts(&self) -> &[f64] {
        &self.shifts
    }
}

/// Scenario-grid sensitivity engine that reprices across multiple factor shocks.
#[derive(Debug, Clone)]
pub struct FullRepricingEngine {
    bump_config: BumpSizeConfig,
    scenario_grid: ScenarioGrid,
}

impl FullRepricingEngine {
    /// Create a repricing engine using `n_scenario_points` around the base market.
    #[must_use]
    pub fn new(bump_config: BumpSizeConfig, n_scenario_points: usize) -> Self {
        Self {
            bump_config,
            scenario_grid: ScenarioGrid::new(n_scenario_points),
        }
    }

    /// Compute the full scenario P&L profile for each factor.
    pub fn compute_pnl_profiles(
        &self,
        positions: &[(String, &dyn Instrument, f64)],
        factors: &[FactorDefinition],
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Vec<FactorPnlProfile>> {
        let base_pvs: Vec<f64> = positions
            .iter()
            .map(|(_, instrument, _)| instrument.value_raw(market, as_of))
            .collect::<Result<_>>()?;

        let mut profiles = Vec::with_capacity(factors.len());
        for factor in factors {
            let (bump_size, bump_unit) = self
                .bump_config
                .bump_size_with_unit_for_factor(&factor.id, &factor.factor_type);
            let mut position_pnls = Vec::with_capacity(self.scenario_grid.shifts().len());

            for &shift in self.scenario_grid.shifts() {
                let bumped_market = market.bump(mapping_to_market_bumps(
                    &factor.market_mapping,
                    bump_size * shift,
                    bump_unit,
                    as_of,
                )?)?;

                let pnl_row: Vec<f64> = positions
                    .iter()
                    .enumerate()
                    .map(|(position_idx, (_, instrument, weight))| {
                        let pv = instrument.value_raw(&bumped_market, as_of)?;
                        Ok((pv - base_pvs[position_idx]) * *weight)
                    })
                    .collect::<Result<_>>()?;
                position_pnls.push(pnl_row);
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
        let profiles = self.compute_pnl_profiles(positions, factors, market, as_of)?;
        let position_ids = positions.iter().map(|(id, _, _)| id.clone()).collect();
        let factor_ids = factors.iter().map(|factor| factor.id.clone()).collect();
        let mut matrix = SensitivityMatrix::zeros(position_ids, factor_ids);

        for (factor_idx, profile) in profiles.iter().enumerate() {
            let down_idx = profile
                .shifts
                .iter()
                .position(|shift| (*shift - (-1.0)).abs() < 1e-12);
            let up_idx = profile
                .shifts
                .iter()
                .position(|shift| (*shift - 1.0).abs() < 1e-12);

            if let (Some(down_idx), Some(up_idx)) = (down_idx, up_idx) {
                let bump_size = self.bump_config.bump_size_for_factor(
                    &factors[factor_idx].id,
                    &factors[factor_idx].factor_type,
                );
                for position_idx in 0..positions.len() {
                    let delta = (profile.position_pnls[up_idx][position_idx]
                        - profile.position_pnls[down_idx][position_idx])
                        / (2.0 * bump_size);
                    matrix.set_delta(position_idx, factor_idx, delta);
                }
            }
        }

        Ok(matrix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Attributes;
    use crate::pricer::InstrumentType;
    use finstack_core::currency::Currency;
    use finstack_core::factor_model::{FactorType, MarketMapping};
    use finstack_core::market_data::bumps::BumpUnits;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_core::types::CurveId;
    use std::any::Any;
    use time::macros::date;

    #[derive(Clone)]
    struct MockInstrument {
        id: String,
        attributes: Attributes,
        curve_id: CurveId,
        tenor_years: f64,
        scale: f64,
    }

    crate::impl_empty_cashflow_provider!(
        MockInstrument,
        crate::cashflow::builder::CashflowRepresentation::NoResidual
    );

    impl MockInstrument {
        fn new(id: &str, curve_id: &str, tenor_years: f64, scale: f64) -> Self {
            Self {
                id: id.to_string(),
                attributes: Attributes::new(),
                curve_id: CurveId::new(curve_id),
                tenor_years,
                scale,
            }
        }

        fn raw_value(&self, market: &MarketContext) -> Result<f64> {
            Ok(market
                .get_discount(self.curve_id.as_str())?
                .zero(self.tenor_years)
                * self.scale)
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

        fn base_value(&self, market: &MarketContext, _as_of: Date) -> Result<Money> {
            Ok(Money::new(self.raw_value(market)?, Currency::USD))
        }

        fn value_raw(&self, market: &MarketContext, _as_of: Date) -> Result<f64> {
            self.raw_value(market)
        }
    }

    fn test_market(as_of: Date) -> Result<MarketContext> {
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .interp(InterpStyle::MonotoneConvex)
            .knots([(0.0, 1.0), (1.0, 0.97), (5.0, 0.80), (10.0, 0.60)])
            .build()?;
        Ok(MarketContext::new().insert(curve))
    }

    #[test]
    fn test_scenario_grid_construction() {
        let grid = ScenarioGrid::new(5);
        assert_eq!(grid.shifts().len(), 5);
        assert!((grid.shifts()[2]).abs() < 1e-12);
    }

    #[test]
    fn test_scenario_grid_minimum_points() {
        let grid = ScenarioGrid::new(3);
        assert_eq!(grid.shifts(), &[-1.0, 0.0, 1.0]);
    }

    #[test]
    #[should_panic(expected = "ScenarioGrid requires at least 3 points")]
    fn test_scenario_grid_rejects_too_few_points() {
        let _ = ScenarioGrid::new(2);
    }

    #[test]
    fn test_full_repricing_engine_extracts_delta_from_profile() -> Result<()> {
        let as_of = date!(2025 - 01 - 01);
        let market = test_market(as_of)?;
        let instrument = MockInstrument::new("curve-inst", "USD-OIS", 5.0, 10_000.0);
        let positions = vec![("curve-pos".to_string(), &instrument as &dyn Instrument, 1.0)];
        let factors = vec![FactorDefinition {
            id: FactorId::new("rates"),
            factor_type: FactorType::Rates,
            market_mapping: MarketMapping::CurveParallel {
                curve_ids: vec![CurveId::new("USD-OIS")],
                units: BumpUnits::RateBp,
            },
            description: None,
        }];

        let engine = FullRepricingEngine::new(BumpSizeConfig::default(), 5);
        let matrix = engine.compute_sensitivities(&positions, &factors, &market, as_of)?;

        assert!((matrix.delta(0, 0) - 1.0).abs() < 1e-3);
        Ok(())
    }

    #[test]
    fn full_repricing_delta_is_normalized_by_bump_size_override() -> Result<()> {
        let as_of = date!(2025 - 01 - 01);
        let market = test_market(as_of)?;
        let instrument = MockInstrument::new("curve-inst", "USD-OIS", 5.0, 10_000.0);
        let positions = vec![("curve-pos".to_string(), &instrument as &dyn Instrument, 1.0)];
        let factor_id = FactorId::new("rates");
        let factors = vec![FactorDefinition {
            id: factor_id.clone(),
            factor_type: FactorType::Rates,
            market_mapping: MarketMapping::CurveParallel {
                curve_ids: vec![CurveId::new("USD-OIS")],
                units: BumpUnits::RateBp,
            },
            description: None,
        }];

        let mut bump_config = BumpSizeConfig::default();
        bump_config.overrides.insert(factor_id, 5.0);
        let matrix = FullRepricingEngine::new(bump_config, 5)
            .compute_sensitivities(&positions, &factors, &market, as_of)?;

        assert!(
            (matrix.delta(0, 0) - 1.0).abs() < 1e-3,
            "linear delta should be per bp, not scaled by the 5 bp override"
        );
        Ok(())
    }

    #[test]
    fn test_full_repricing_engine_pnl_profiles_include_center_scenario() -> Result<()> {
        let as_of = date!(2025 - 01 - 01);
        let market = test_market(as_of)?;
        let instrument = MockInstrument::new("curve-inst", "USD-OIS", 5.0, 10_000.0);
        let positions = vec![("curve-pos".to_string(), &instrument as &dyn Instrument, 1.0)];
        let factors = vec![FactorDefinition {
            id: FactorId::new("rates"),
            factor_type: FactorType::Rates,
            market_mapping: MarketMapping::CurveParallel {
                curve_ids: vec![CurveId::new("USD-OIS")],
                units: BumpUnits::RateBp,
            },
            description: None,
        }];

        let profiles = FullRepricingEngine::new(BumpSizeConfig::default(), 5)
            .compute_pnl_profiles(&positions, &factors, &market, as_of)?;

        assert_eq!(profiles.len(), 1);
        let profile = &profiles[0];
        assert_eq!(profile.shifts, vec![-2.0, -1.0, 0.0, 1.0, 2.0]);
        assert!((profile.position_pnls[2][0]).abs() < 1e-12);
        Ok(())
    }
}
