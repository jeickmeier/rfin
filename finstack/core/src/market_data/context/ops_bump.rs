use crate::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, Bumpable, MarketBump};
use crate::types::CurveId;
use crate::Result;
use std::sync::Arc;

use super::curve_storage::CurveStorage;
use super::MarketContext;

use crate::market_data::term_structures::{
    BaseCorrelationCurve, DiscountCurve, ForwardCurve, InflationCurve, VolatilityIndexCurve,
};

impl MarketContext {
    /// Apply a heterogeneous list of market bumps (curves, surfaces, prices, FX).
    ///
    /// This is the **single canonical** entry point for market bumping. It supports:
    /// - Curve/surface/scalar/series bumps addressed by [`CurveId`] (via [`MarketBump::Curve`])
    /// - FX percentage shocks (via [`MarketBump::FxPct`])
    /// - Volatility surface bucket bumps (via [`MarketBump::VolBucketPct`])
    /// - Base correlation bucket bumps (via [`MarketBump::BaseCorrBucketPts`])
    ///
    /// # Errors
    ///
    /// Returns an error if any bumped entry is missing, the bump type is unsupported,
    /// or reconstruction fails.
    pub fn bump<I>(&self, bumps: I) -> Result<Self>
    where
        I: IntoIterator<Item = MarketBump>,
    {
        use crate::collections::HashMap;
        use crate::error::InputError;

        let mut ctx = self.clone();
        let mut curve_bumps: HashMap<CurveId, BumpSpec> = HashMap::default();

        for bump in bumps {
            match bump {
                MarketBump::Curve { id, spec } => {
                    curve_bumps.insert(id, spec);
                }
                MarketBump::FxPct {
                    base,
                    quote,
                    pct,
                    as_of,
                } => {
                    let fx = ctx.fx.as_ref().ok_or_else(|| InputError::NotFound {
                        id: "FX matrix".to_string(),
                    })?;
                    let bumped = fx.with_bumped_rate(base, quote, pct / 100.0, as_of)?;
                    ctx.fx = Some(Arc::new(bumped));
                }
                MarketBump::VolBucketPct {
                    surface_id,
                    expiries,
                    strikes,
                    pct,
                } => {
                    // Parallel fallback if no filters provided
                    if expiries.is_none() && strikes.is_none() {
                        curve_bumps.insert(
                            surface_id,
                            BumpSpec {
                                mode: BumpMode::Additive,
                                units: BumpUnits::Percent,
                                value: pct,
                                bump_type: BumpType::Parallel,
                            },
                        );
                        continue;
                    }

                    let surface =
                        ctx.surface(surface_id.as_str())
                            .map_err(|_| InputError::NotFound {
                                id: surface_id.to_string(),
                            })?;

                    let bumped = surface
                        .apply_bucket_bump(expiries.as_deref(), strikes.as_deref(), pct)
                        .ok_or(InputError::DimensionMismatch)?;

                    ctx = ctx.insert_surface(bumped);
                }
                MarketBump::BaseCorrBucketPts {
                    surface_id,
                    detachments,
                    points,
                } => {
                    let curve = ctx.get_base_correlation(surface_id.as_str()).map_err(|_| {
                        InputError::NotFound {
                            id: surface_id.to_string(),
                        }
                    })?;

                    let bumped = curve
                        .apply_bucket_bump(detachments.as_deref(), points)
                        .ok_or(InputError::DimensionMismatch)?;

                    ctx.curves
                        .insert(surface_id, CurveStorage::BaseCorrelation(Arc::new(bumped)));
                }
            }
        }

        if !curve_bumps.is_empty() {
            ctx.apply_curve_bumps(curve_bumps)?;
        }

        Ok(ctx)
    }

    fn apply_curve_bumps(
        &mut self,
        bumps: crate::collections::HashMap<CurveId, BumpSpec>,
    ) -> Result<()> {
        for (curve_id, bump_spec) in bumps {
            let cid = curve_id.as_str();

            if let Some(storage) = self.curves.get(cid).cloned() {
                let bumped_storage = match storage {
                    CurveStorage::Discount(original) => {
                        let bumped = original.apply_bump(bump_spec)?;
                        let final_curve = if bumped.id() != original.id() {
                            DiscountCurve::builder(original.id().as_str())
                                .base_date(bumped.base_date())
                                .day_count(bumped.day_count())
                                .knots(
                                    bumped
                                        .knots()
                                        .iter()
                                        .copied()
                                        .zip(bumped.dfs().iter().copied()),
                                )
                                .set_interp(bumped.interp_style())
                                .extrapolation(bumped.extrapolation())
                                .allow_non_monotonic()
                                .build()?
                        } else {
                            bumped
                        };
                        CurveStorage::Discount(Arc::new(final_curve))
                    }
                    CurveStorage::Forward(original) => {
                        let bumped = original.apply_bump(bump_spec)?;
                        let final_curve = if bumped.id() != original.id() {
                            ForwardCurve::builder(original.id().as_str(), bumped.tenor())
                                .base_date(bumped.base_date())
                                .reset_lag(bumped.reset_lag())
                                .day_count(bumped.day_count())
                                .knots(
                                    bumped
                                        .knots()
                                        .iter()
                                        .copied()
                                        .zip(bumped.forwards().iter().copied()),
                                )
                                .build()?
                        } else {
                            bumped
                        };
                        CurveStorage::Forward(Arc::new(final_curve))
                    }
                    CurveStorage::Hazard(original) => {
                        let bumped = original.apply_bump(bump_spec)?;
                        let final_curve = if bumped.id() != original.id() {
                            bumped.to_builder_with_id(original.id().clone()).build()?
                        } else {
                            bumped
                        };
                        CurveStorage::Hazard(Arc::new(final_curve))
                    }
                    CurveStorage::Inflation(original) => {
                        if let BumpType::TriangularKeyRate { target_bucket, .. } =
                            bump_spec.bump_type
                        {
                            let delta = bump_spec.additive_fraction().ok_or_else(|| {
                                crate::error::InputError::UnsupportedBump {
                                    reason:
                                        "InflationCurve key-rate bump requires additive fraction"
                                            .to_string(),
                                }
                            })?;
                            let mut points: Vec<(f64, f64)> = original
                                .knots()
                                .iter()
                                .copied()
                                .zip(original.cpi_levels().iter().copied())
                                .collect();
                            if let Some((idx, _)) = points.iter().enumerate().min_by(|a, b| {
                                let da = (a.1 .0 - target_bucket).abs();
                                let db = (b.1 .0 - target_bucket).abs();
                                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                            }) {
                                points[idx].1 *= 1.0 + delta;
                            }

                            let rebuilt = InflationCurve::builder(original.id().as_str())
                                .base_cpi(original.base_cpi())
                                .knots(points)
                                .build()?;
                            CurveStorage::Inflation(Arc::new(rebuilt))
                        } else {
                            let bumped = original.apply_bump(bump_spec)?;
                            let final_curve = if bumped.id() != original.id() {
                                InflationCurve::builder(original.id().as_str())
                                    .base_cpi(bumped.base_cpi())
                                    .knots(
                                        bumped
                                            .knots()
                                            .iter()
                                            .copied()
                                            .zip(bumped.cpi_levels().iter().copied()),
                                    )
                                    .build()?
                            } else {
                                bumped
                            };
                            CurveStorage::Inflation(Arc::new(final_curve))
                        }
                    }
                    CurveStorage::BaseCorrelation(original) => {
                        let bumped = original.apply_bump(bump_spec)?;
                        let final_curve = if bumped.id() != original.id() {
                            BaseCorrelationCurve::builder(original.id().as_str())
                                .knots(
                                    bumped
                                        .detachment_points()
                                        .iter()
                                        .copied()
                                        .zip(bumped.correlations().iter().copied()),
                                )
                                .build()?
                        } else {
                            bumped
                        };
                        CurveStorage::BaseCorrelation(Arc::new(final_curve))
                    }
                    CurveStorage::VolIndex(original) => {
                        let bumped = original.apply_bump(bump_spec)?;
                        let final_curve = if bumped.id() != original.id() {
                            VolatilityIndexCurve::builder(original.id().as_str())
                                .base_date(bumped.base_date())
                                .day_count(bumped.day_count())
                                .spot_level(bumped.spot_level())
                                .knots(
                                    bumped
                                        .knots()
                                        .iter()
                                        .copied()
                                        .zip(bumped.levels().iter().copied()),
                                )
                                .build()?
                        } else {
                            bumped
                        };
                        CurveStorage::VolIndex(Arc::new(final_curve))
                    }
                };

                self.curves.insert(curve_id.clone(), bumped_storage);
                continue;
            }

            if let Some(original) = self.surfaces.get(cid).cloned() {
                let bumped = original.apply_bump(bump_spec)?;
                self.surfaces.insert(curve_id.clone(), Arc::new(bumped));
                continue;
            }
            if let Some(original) = self.prices.get(cid).cloned() {
                let bumped = original.apply_bump(bump_spec)?;
                self.prices.insert(curve_id.clone(), bumped);
                continue;
            }
            if let Some(original) = self.series.get(cid).cloned() {
                let bumped = original.apply_bump(bump_spec)?;
                self.series.insert(curve_id.clone(), bumped);
                continue;
            }

            return Err(crate::error::InputError::NotFound {
                id: cid.to_string(),
            }
            .into());
        }

        Ok(())
    }
}
