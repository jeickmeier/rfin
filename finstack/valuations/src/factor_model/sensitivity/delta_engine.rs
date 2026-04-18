use super::matrix::SensitivityMatrix;
use super::traits::FactorSensitivityEngine;
use crate::instruments::internal::InstrumentExt as Instrument;
use finstack_core::dates::Date;
use finstack_core::factor_model::{BumpSizeConfig, FactorDefinition, MarketMapping};
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, MarketBump};
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::CurveId;
use finstack_core::{Error, InputError, Result};

use rayon::prelude::*;

/// Finite-difference sensitivity engine using central bumps around the base market.
#[derive(Debug, Clone)]
pub struct DeltaBasedEngine {
    bump_config: BumpSizeConfig,
}

impl DeltaBasedEngine {
    /// Create a new delta-based engine with the provided bump configuration.
    #[must_use]
    pub fn new(bump_config: BumpSizeConfig) -> Self {
        Self { bump_config }
    }

    fn compute_factor_column(
        &self,
        positions: &[(String, &dyn Instrument, f64)],
        factor: &FactorDefinition,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Vec<f64>> {
        let bump_size = self
            .bump_config
            .bump_size_for_factor(&factor.id, &factor.factor_type);
        if bump_size.abs() < f64::EPSILON {
            return Err(InputError::Invalid.into());
        }

        let up_market = market.bump(mapping_to_market_bumps(
            &factor.market_mapping,
            bump_size,
            as_of,
        )?)?;
        let down_market = market.bump(mapping_to_market_bumps(
            &factor.market_mapping,
            -bump_size,
            as_of,
        )?)?;

        positions
            .iter()
            .map(|(_, instrument, weight)| {
                let pv_up = instrument.value_raw(&up_market, as_of)?;
                let pv_down = instrument.value_raw(&down_market, as_of)?;
                Ok((pv_up - pv_down) / (2.0 * bump_size) * *weight)
            })
            .collect()
    }
}

/// Convert a `MarketMapping` into concrete `MarketBump`s that can be applied to a market.
pub fn mapping_to_market_bumps(
    mapping: &MarketMapping,
    bump_size: f64,
    as_of: Date,
) -> Result<Vec<MarketBump>> {
    match mapping {
        MarketMapping::CurveParallel { curve_ids, units } => Ok(curve_ids
            .iter()
            .cloned()
            .map(|id| MarketBump::Curve {
                id,
                spec: BumpSpec {
                    mode: BumpMode::Additive,
                    units: *units,
                    value: bump_size,
                    bump_type: BumpType::Parallel,
                },
            })
            .collect()),
        MarketMapping::CurveBucketed {
            curve_id,
            tenor_weights,
        } => Ok(tenor_weights
            .iter()
            .enumerate()
            .map(|(idx, &(target_bucket, weight))| {
                let prev_bucket = if idx == 0 {
                    0.0
                } else {
                    tenor_weights[idx - 1].0
                };
                let next_bucket = tenor_weights
                    .get(idx + 1)
                    .map_or(f64::INFINITY, |(bucket, _)| *bucket);
                MarketBump::Curve {
                    id: curve_id.clone(),
                    spec: BumpSpec::triangular_key_rate_bp(
                        prev_bucket,
                        target_bucket,
                        next_bucket,
                        bump_size * weight,
                    ),
                }
            })
            .collect()),
        MarketMapping::EquitySpot { tickers } => Ok(tickers
            .iter()
            .map(|ticker| MarketBump::Curve {
                id: CurveId::new(ticker),
                spec: BumpSpec::multiplier(1.0 + bump_size / 100.0),
            })
            .collect()),
        MarketMapping::FxRate { pair } => Ok(vec![MarketBump::FxPct {
            base: pair.0,
            quote: pair.1,
            pct: bump_size,
            as_of,
        }]),
        MarketMapping::VolShift { surface_ids, units } => Ok(surface_ids
            .iter()
            .map(|surface_id| MarketBump::Curve {
                id: CurveId::new(surface_id),
                spec: BumpSpec {
                    mode: BumpMode::Additive,
                    units: *units,
                    value: bump_size,
                    bump_type: BumpType::Parallel,
                },
            })
            .collect()),
        MarketMapping::Custom(_) => Err(Error::Validation(
            "Factor sensitivity engines do not support MarketMapping::Custom because the custom mapping does not identify target market objects".to_string(),
        )),
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
        let position_ids = positions.iter().map(|(id, _, _)| id.clone()).collect();
        let factor_ids = factors.iter().map(|factor| factor.id.clone()).collect();
        let mut matrix = SensitivityMatrix::zeros(position_ids, factor_ids);

        let columns: Result<Vec<Vec<f64>>> = factors
            .par_iter()
            .map(|factor| self.compute_factor_column(positions, factor, market, as_of))
            .collect();

        for (factor_idx, column) in columns?.iter().enumerate() {
            for (position_idx, value) in column.iter().enumerate() {
                matrix.set_delta(position_idx, factor_idx, *value);
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
    use finstack_core::market_data::bumps::BumpUnits;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use std::any::Any;
    use time::macros::date;

    #[derive(Clone)]
    enum MockKind {
        CurveZero { curve_id: CurveId, tenor_years: f64 },
        Spot { spot_id: String },
    }

    #[derive(Clone)]
    struct MockInstrument {
        id: String,
        attributes: Attributes,
        kind: MockKind,
        scale: f64,
    }

    crate::impl_empty_cashflow_provider!(
        MockInstrument,
        crate::cashflow::builder::CashflowRepresentation::NoResidual
    );

    impl MockInstrument {
        fn curve_zero(id: &str, curve_id: &str, tenor_years: f64, scale: f64) -> Self {
            Self {
                id: id.to_string(),
                attributes: Attributes::new(),
                kind: MockKind::CurveZero {
                    curve_id: CurveId::new(curve_id),
                    tenor_years,
                },
                scale,
            }
        }

        fn spot(id: &str, spot_id: &str, scale: f64) -> Self {
            Self {
                id: id.to_string(),
                attributes: Attributes::new(),
                kind: MockKind::Spot {
                    spot_id: spot_id.to_string(),
                },
                scale,
            }
        }

        fn raw_value(&self, market: &MarketContext) -> Result<f64> {
            match &self.kind {
                MockKind::CurveZero {
                    curve_id,
                    tenor_years,
                } => Ok(market.get_discount(curve_id.as_str())?.zero(*tenor_years) * self.scale),
                MockKind::Spot { spot_id } => {
                    let price = market.get_price(spot_id)?.clone();
                    let value = match price {
                        MarketScalar::Unitless(v) => v,
                        MarketScalar::Price(money) => money.amount(),
                    };
                    Ok(value * self.scale)
                }
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

        fn value(&self, market: &MarketContext, _as_of: Date) -> Result<Money> {
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

        Ok(MarketContext::new()
            .insert(curve)
            .insert_price("SPOT", MarketScalar::Unitless(100.0)))
    }

    #[test]
    fn test_mapping_to_market_bumps_curve_parallel() -> Result<()> {
        let mapping = MarketMapping::CurveParallel {
            curve_ids: vec![CurveId::new("USD-OIS")],
            units: BumpUnits::RateBp,
        };

        let bumps = mapping_to_market_bumps(&mapping, 1.0, date!(2025 - 01 - 01))?;

        assert_eq!(bumps.len(), 1);
        assert!(matches!(bumps[0], MarketBump::Curve { .. }));
        if let MarketBump::Curve { id, spec } = &bumps[0] {
            assert_eq!(id.as_str(), "USD-OIS");
            assert_eq!(spec.value, 1.0);
            assert_eq!(spec.units, BumpUnits::RateBp);
        }
        Ok(())
    }

    #[test]
    fn test_mapping_to_market_bumps_curve_bucketed() -> Result<()> {
        let mapping = MarketMapping::CurveBucketed {
            curve_id: CurveId::new("USD-OIS"),
            tenor_weights: vec![(2.0, 0.5), (5.0, 1.0), (10.0, 0.5)],
        };

        let bumps = mapping_to_market_bumps(&mapping, 1.0, date!(2025 - 01 - 01))?;

        assert_eq!(bumps.len(), 3);
        assert!(matches!(bumps[1], MarketBump::Curve { .. }));
        if let MarketBump::Curve { spec, .. } = &bumps[1] {
            assert_eq!(spec.value, 1.0);
            assert_eq!(
                spec.bump_type,
                BumpType::TriangularKeyRate {
                    prev_bucket: 2.0,
                    target_bucket: 5.0,
                    next_bucket: 10.0,
                }
            );
        }
        Ok(())
    }

    #[test]
    fn test_delta_based_engine_curve_parallel_sensitivity() -> Result<()> {
        let as_of = date!(2025 - 01 - 01);
        let market = test_market(as_of)?;
        let instrument = MockInstrument::curve_zero("curve-inst", "USD-OIS", 5.0, 10_000.0);
        let positions = vec![("curve-pos".to_string(), &instrument as &dyn Instrument, 1.0)];
        let factors = vec![FactorDefinition {
            id: finstack_core::factor_model::FactorId::new("rates"),
            factor_type: finstack_core::factor_model::FactorType::Rates,
            market_mapping: MarketMapping::CurveParallel {
                curve_ids: vec![CurveId::new("USD-OIS")],
                units: BumpUnits::RateBp,
            },
            description: None,
        }];

        let matrix = DeltaBasedEngine::new(BumpSizeConfig::default())
            .compute_sensitivities(&positions, &factors, &market, as_of)?;

        assert_eq!(matrix.n_positions(), 1);
        assert_eq!(matrix.n_factors(), 1);
        assert!((matrix.delta(0, 0) - 1.0).abs() < 1e-3);
        Ok(())
    }

    #[test]
    fn test_delta_based_engine_equity_spot_sensitivity() -> Result<()> {
        let as_of = date!(2025 - 01 - 01);
        let market = test_market(as_of)?;
        let instrument = MockInstrument::spot("spot-inst", "SPOT", 1.0);
        let positions = vec![("spot-pos".to_string(), &instrument as &dyn Instrument, 1.0)];
        let factors = vec![FactorDefinition {
            id: finstack_core::factor_model::FactorId::new("equity"),
            factor_type: finstack_core::factor_model::FactorType::Equity,
            market_mapping: MarketMapping::EquitySpot {
                tickers: vec!["SPOT".to_string()],
            },
            description: None,
        }];

        let matrix = DeltaBasedEngine::new(BumpSizeConfig::default())
            .compute_sensitivities(&positions, &factors, &market, as_of)?;

        assert!((matrix.delta(0, 0) - 1.0).abs() < 1e-9);
        Ok(())
    }
}
