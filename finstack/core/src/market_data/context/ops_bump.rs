use crate::collections::HashMap;
use std::sync::Arc;

use crate::currency::Currency;
use crate::dates::Date;
use crate::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, Bumpable, MarketBump};
use crate::types::CurveId;
use crate::Result;

use super::curve_storage::CurveStorage;
use super::MarketContext;

use crate::market_data::term_structures::{
    base_correlation::BaseCorrelationCurve, discount_curve::DiscountCurve,
    forward_curve::ForwardCurve, inflation::InflationCurve,
    vol_index_curve::VolatilityIndexCurve,
};

impl MarketContext {
    /// Bump FX spot rate for a currency pair and return a new context.
    ///
    /// Creates a new MarketContext with an FX matrix that has the specified
    /// currency pair rate bumped by the given percentage. All other market data
    /// is cloned unchanged.
    ///
    /// # Parameters
    /// - `from`: Base currency
    /// - `to`: Quote currency
    /// - `bump_pct`: Relative bump size (e.g., 0.01 for 1% increase)
    /// - `on`: Date for rate lookup (typically as_of date from valuation context)
    ///
    /// # Returns
    /// New MarketContext with bumped FX rate
    ///
    /// # Errors
    /// Returns error if FX matrix is missing or rate lookup fails
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::money::fx::{FxMatrix, FxProvider, FxConversionPolicy};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # struct StaticFx;
    /// # impl FxProvider for StaticFx {
    /// #     fn rate(&self, _from: Currency, _to: Currency, _on: Date, _policy: FxConversionPolicy)
    /// #         -> finstack_core::Result<f64> { Ok(1.1) }
    /// # }
    /// # let fx = FxMatrix::new(Arc::new(StaticFx));
    /// # let ctx = MarketContext::new().insert_fx(fx);
    /// # let date = Date::from_calendar_date(2024, Month::January, 1).expect("Valid date");
    /// let bumped_ctx = ctx.bump_fx_spot(Currency::EUR, Currency::USD, 0.01, date)?;
    /// // EUR/USD rate is now 1.1 * 1.01 = 1.111
    /// # Ok(())
    /// # }
    /// ```
    pub fn bump_fx_spot(
        &self,
        from: Currency,
        to: Currency,
        bump_pct: f64,
        on: Date,
    ) -> Result<Self> {
        let fx_matrix = self
            .fx
            .as_ref()
            .ok_or_else(|| crate::error::InputError::NotFound {
                id: "FX matrix".to_string(),
            })?;

        // Create new FX matrix with bumped rate
        let new_fx_matrix = fx_matrix.with_bumped_rate(from, to, bump_pct, on)?;

        // Create new context with bumped FX
        let mut new_context = self.clone();
        new_context.fx = Some(Arc::new(new_fx_matrix));

        Ok(new_context)
    }

    /// Apply one or more bumps to the market context in a single call.
    ///
    /// This consolidated API supports discount/forward/hazard/inflation/base-correlation
    /// curves, volatility surfaces, market scalars, and generic scalar time series.
    ///
    /// # Example
    /// ```rust
    /// # use finstack_core::collections::HashMap;
    /// # use finstack_core::market_data::context::{MarketContext, BumpSpec};
    /// # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// # use finstack_core::dates::Date;
    /// # use finstack_core::types::CurveId;
    /// # let curve = DiscountCurve::builder("USD-OIS")
    /// #     .base_date(Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 1.0), (5.0, 0.9)])
    /// #     .build().expect("DiscountCurve builder should succeed");
    /// # let context = MarketContext::new().insert_discount(curve);
    /// let mut bumps = HashMap::default();
    /// bumps.insert(CurveId::new("USD-OIS"), BumpSpec::parallel_bp(100.0));
    /// let bumped = context.bump(bumps).expect("Bump operation should succeed");
    /// // The bumped curve replaces the original under the same ID
    /// assert!(bumped.get_discount("USD-OIS").is_ok());
    /// ```
    pub fn bump(&self, bumps: HashMap<CurveId, BumpSpec>) -> Result<Self> {
        let mut new_context = self.clone();

        for (curve_id, bump_spec) in bumps {
            let cid = curve_id.as_str();

            if let Some(storage) = self.curves.get(cid) {
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
                                .points(
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

                new_context.curves.insert(curve_id.clone(), bumped_storage);
                continue;
            }

            if let Some(original) = self.surfaces.get(cid) {
                let bumped = original.apply_bump(bump_spec)?;
                new_context
                    .surfaces
                    .insert(curve_id.clone(), Arc::new(bumped));
                continue;
            }
            if let Some(original) = self.prices.get(cid) {
                let bumped = original.apply_bump(bump_spec)?;
                new_context.prices.insert(curve_id.clone(), bumped);
                continue;
            }
            if let Some(original) = self.series.get(cid) {
                let bumped = original.apply_bump(bump_spec)?;
                new_context.series.insert(curve_id.clone(), bumped);
                continue;
            }

            return Err(crate::error::InputError::NotFound {
                id: cid.to_string(),
            }
            .into());
        }

        Ok(new_context)
    }

    /// Apply a heterogeneous list of market bumps (curves, surfaces, prices, FX).
    ///
    /// This is a thin wrapper around [`MarketContext::bump`] for all
    /// `Curve`-addressable entries plus explicit handling for FX shocks using
    /// [`crate::money::fx::FxMatrix::with_bumped_rate`]. It is intended for scenario engines and
    /// risk utilities that want a single entry point for all market mutations.
    pub fn apply_bumps(&self, bumps: &[MarketBump]) -> Result<Self> {
        use crate::error::InputError;

        let mut ctx = self.clone();
        let mut curve_bumps: HashMap<CurveId, BumpSpec> = HashMap::default();

        for bump in bumps {
            match bump {
                MarketBump::Curve { id, spec } => {
                    curve_bumps.insert(id.clone(), *spec);
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
                    let bumped = fx.with_bumped_rate(*base, *quote, *pct / 100.0, *as_of)?;
                    ctx.fx = Some(Arc::new(bumped));
                }
                MarketBump::VolBucketPct {
                    surface_id,
                    expiries,
                    strikes,
                    pct,
                } => {
                    let surface =
                        ctx.surface_ref(surface_id.as_str())
                            .map_err(|_| InputError::NotFound {
                                id: surface_id.to_string(),
                            })?;

                    // Parallel fallback if no filters provided
                    if expiries.is_none() && strikes.is_none() {
                        let mut single = HashMap::default();
                        single.insert(
                            surface_id.clone(),
                            BumpSpec {
                                mode: BumpMode::Additive,
                                units: BumpUnits::Percent,
                                value: *pct,
                                bump_type: BumpType::Parallel,
                            },
                        );
                        ctx = ctx.bump(single)?;
                        continue;
                    }

                    let bumped = surface
                        .apply_bucket_bump(expiries.as_deref(), strikes.as_deref(), *pct)
                        .ok_or(InputError::DimensionMismatch)?;

                    ctx.insert_surface_mut(bumped);
                }
                MarketBump::BaseCorrBucketPts {
                    surface_id,
                    detachments,
                    points,
                } => {
                    let curve =
                        ctx.get_base_correlation_ref(surface_id.as_str())
                            .map_err(|_| InputError::NotFound {
                                id: surface_id.to_string(),
                            })?;

                    let bumped = curve
                        .apply_bucket_bump(detachments.as_deref(), *points)
                        .ok_or(InputError::DimensionMismatch)?;

                    ctx.curves.insert(
                        surface_id.clone(),
                        CurveStorage::BaseCorrelation(Arc::new(bumped)),
                    );
                }
            }
        }

        if !curve_bumps.is_empty() {
            ctx = ctx.bump(curve_bumps)?;
        }

        Ok(ctx)
    }
}
