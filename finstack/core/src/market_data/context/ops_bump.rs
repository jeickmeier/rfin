//! Scenario bump operations for [`MarketContext`](super::MarketContext).
//!
//! This submodule implements the canonical heterogeneous bump entry points used
//! for risk and scenario analysis across curves, surfaces, market scalars, and FX.

use crate::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, Bumpable, MarketBump};
use crate::types::CurveId;
use crate::Result;
use std::sync::Arc;

use super::curve_storage::CurveStorage;
use super::{ContextMutationInfo, MarketContext};

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
    /// Apply a heterogeneous list of market bumps (curves, surfaces, prices, FX).
    ///
    /// # Errors
    ///
    /// Returns an error if any bumped entry is missing, the bump type is unsupported,
    /// or reconstruction fails.
    pub fn bump<I>(&self, bumps: I) -> Result<Self>
    where
        I: IntoIterator<Item = MarketBump>,
    {
        let (ctx, _info) = self.bump_observed(bumps)?;
        Ok(ctx)
    }

    /// Like [`bump`](Self::bump), but also returns a [`ContextMutationInfo`]
    /// describing any credit indices that were invalidated.
    ///
    /// Use this in production workflows where silent credit-index invalidation
    /// is a risk.
    pub fn bump_observed<I>(&self, bumps: I) -> Result<(Self, ContextMutationInfo)>
    where
        I: IntoIterator<Item = MarketBump>,
    {
        use crate::collections::HashMap;
        use crate::error::InputError;

        let mut ctx = self.clone();
        let mut curve_bumps: HashMap<CurveId, BumpSpec> = HashMap::default();
        let mut needs_credit_rebind = false;
        #[allow(unused_variables)]
        let mut processed_bumps = 0usize;

        for bump in bumps {
            if cfg!(feature = "tracing") {
                processed_bumps += 1;
            }
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
                        ctx.get_surface(surface_id.as_str())
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
                    needs_credit_rebind = true;
                }
            }
        }

        if !curve_bumps.is_empty() {
            ctx.apply_curve_bumps(curve_bumps)?;
        }
        let mut mutation_info = ContextMutationInfo::default();
        if needs_credit_rebind {
            mutation_info.invalidated_credit_indices = ctx.rebind_all_credit_indices();
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(
            processed_bumps,
            needs_credit_rebind,
            invalidated_count = mutation_info.invalidated_credit_indices.len(),
            "applied MarketContext bumps"
        );

        Ok((ctx, mutation_info))
    }

    /// Apply curve bumps using the centralized bump-and-rebuild logic in `CurveStorage`.
    ///
    /// This method iterates over the bump specifications and applies them to curves,
    /// surfaces, prices, or series. The `CurveStorage::apply_bump_preserving_id` method
    /// handles the curve-specific bumping and ID preservation logic.
    fn apply_curve_bumps(
        &mut self,
        bumps: crate::collections::HashMap<CurveId, BumpSpec>,
    ) -> Result<()> {
        let mut needs_credit_rebind = false;
        for (curve_id, bump_spec) in bumps {
            let cid = curve_id.as_str();

            // Try curves first (most common case)
            if let Some(storage) = self.curves.get(cid).cloned() {
                let bumped_storage = storage.apply_bump_preserving_id(&curve_id, bump_spec)?;
                self.curves.insert(curve_id.clone(), bumped_storage);
                needs_credit_rebind = true;
                continue;
            }

            // Try vol surfaces
            if let Some(original) = self.surfaces.get(cid).cloned() {
                let bumped = original.apply_bump(bump_spec)?;
                self.surfaces.insert(curve_id.clone(), Arc::new(bumped));
                continue;
            }

            // Try scalar prices
            if let Some(original) = self.prices.get(cid).cloned() {
                let bumped = original.apply_bump(bump_spec)?;
                self.prices.insert(curve_id.clone(), bumped);
                continue;
            }

            // Try time series
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

        if needs_credit_rebind {
            let _invalidated = self.rebind_all_credit_indices();
        }

        Ok(())
    }
}
