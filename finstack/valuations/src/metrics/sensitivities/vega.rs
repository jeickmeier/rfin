//! Vega calculators for volatility sensitivity.
//!
//! Provides parallel and key-rate vega calculators for instruments with volatility surfaces.

use crate::instruments::common::traits::Instrument;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::MetricCalculator;
use crate::metrics::{MetricContext, MetricId};
use finstack_core::market_data::scalars::MarketScalar;
use std::marker::PhantomData;
use std::sync::Arc;

/// Standard expiry buckets in years for equity options.
pub fn standard_equity_expiry_buckets() -> Vec<f64> {
    vec![
        1.0 / 12.0, // 1m
        3.0 / 12.0, // 3m
        6.0 / 12.0, // 6m
        1.0,        // 1y
        2.0,        // 2y
        3.0,        // 3y
        5.0,        // 5y
    ]
}

/// Standard strike buckets (relative to spot) for equity options.
pub fn standard_strike_ratios() -> Vec<f64> {
    vec![0.5, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5]
}

/// Key-rate vega calculator: bumps individual (expiry, strike) points.
///
/// Calculates volatility sensitivity at individual points on the volatility surface
/// by bumping each (expiry, strike) combination and measuring the present value change.
/// This provides a detailed view of how the instrument's value depends on different
/// parts of the volatility surface.
///
/// # Type Parameters
///
/// * `I` - Instrument type that implements [`Instrument`] and has a volatility surface
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::instruments::EquityOption;
/// use finstack_valuations::metrics::KeyRateVega;
///
/// // Standard equity buckets
/// let calculator = KeyRateVega::<EquityOption>::standard();
///
/// // Or custom buckets
/// let expiries = vec![0.25, 0.5, 1.0, 2.0];
/// let strikes = vec![0.9, 1.0, 1.1];
/// let calculator = KeyRateVega::<EquityOption>::new(expiries, strikes);
/// ```
pub struct KeyRateVega<I> {
    expiries: Vec<f64>,
    strikes: Vec<f64>,
    _phantom: PhantomData<I>,
}

impl<I> KeyRateVega<I> {
    /// Create a key-rate vega calculator with custom buckets.
    ///
    /// # Arguments
    ///
    /// * `expiries` - Expiry times in years for the vega grid
    /// * `strikes` - Strike ratios (relative to spot) for the vega grid
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // KeyRateVega is internal - use MetricId::KeyRateVega via price_with_metrics
    /// use finstack_valuations::metrics::sensitivities::vega::KeyRateVega;
    /// use finstack_valuations::instruments::EquityOption;
    ///
    /// let expiries = vec![0.25, 0.5, 1.0, 2.0];
    /// let strikes = vec![0.9, 1.0, 1.1];
    /// let calculator = KeyRateVega::<EquityOption>::new(expiries, strikes);
    /// ```
    pub fn new(expiries: Vec<f64>, strikes: Vec<f64>) -> Self {
        Self {
            expiries,
            strikes,
            _phantom: PhantomData,
        }
    }

    /// Create a key-rate vega calculator with standard equity buckets.
    ///
    /// Uses standard expiry buckets (1m, 3m, 6m, 1y, 2y, 3y, 5y) and
    /// standard strike ratios (0.5, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // KeyRateVega is internal - use MetricId::KeyRateVega via price_with_metrics
    /// use finstack_valuations::metrics::sensitivities::vega::KeyRateVega;
    /// use finstack_valuations::instruments::EquityOption;
    ///
    /// let calculator = KeyRateVega::<EquityOption>::standard();
    /// ```
    pub fn standard() -> Self {
        Self::new(standard_equity_expiry_buckets(), standard_strike_ratios())
    }
}

impl<I> Default for KeyRateVega<I> {
    fn default() -> Self {
        Self::standard()
    }
}

impl<I> MetricCalculator for KeyRateVega<I>
where
    I: Instrument + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let defaults = sens_config::from_finstack_config_or_default(context.config())?;

        let vol_surface_id = instrument
            .vol_surface_id()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::InputError::Invalid))?;

        let base_pv = context.base_value;
        let base_ctx = context.curves.as_ref();
        let vol_surface = base_ctx.surface(vol_surface_id.as_str())?;

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let bump_pct = defaults.vol_bump_pct;

        // Use already-computed Vega when available to keep totals consistent
        let target_total = if let Some(existing) = context.computed.get(&MetricId::Vega) {
            *existing
        } else {
            let parallel_surface = vol_surface.scaled(1.0 + bump_pct);
            let parallel_ctx = base_ctx.clone().insert_surface(parallel_surface);
            let pv_parallel = inst_arc.value(&parallel_ctx, as_of)?;
            (pv_parallel.amount() - base_pv.amount()) / bump_pct
        };

        let mut raw_matrix = Vec::new();
        let mut raw_total = 0.0;
        let debug = std::env::var("DEBUG_BUCKETED_VEGA").is_ok();

        let use_ratio_strikes = self.strikes.iter().all(|k| *k <= 10.0);
        let strike_grid: Vec<f64> = if use_ratio_strikes {
            let spot = instrument
                .spot_id()
                .and_then(|id| base_ctx.price(id).ok())
                .map(|scalar| match scalar {
                    MarketScalar::Price(m) => m.amount(),
                    MarketScalar::Unitless(v) => *v,
                })
                .ok_or_else(|| finstack_core::Error::from(finstack_core::InputError::Invalid))?;

            self.strikes.iter().map(|k| k * spot).collect()
        } else {
            self.strikes.clone()
        };

        for &expiry in &self.expiries {
            let mut row = Vec::new();
            for &strike in &strike_grid {
                // Bump vol at this specific (expiry, strike) point
                let bumped_surface = vol_surface.bump_point(expiry, strike, bump_pct)?;
                let temp_ctx = base_ctx.clone().insert_surface(bumped_surface);
                let pv_bumped = inst_arc.value(&temp_ctx, as_of)?;

                // Vega = (PV_bumped - PV_base) / bump_pct
                let vega = (pv_bumped.amount() - base_pv.amount()) / bump_pct;
                row.push(vega);
                raw_total += vega;
            }
            raw_matrix.push(row);
        }

        // Normalize bucketed vegas so they partition the parallel vega
        let scale = if raw_total.abs() > f64::EPSILON {
            target_total / raw_total
        } else {
            1.0
        };
        let matrix: Vec<Vec<f64>> = raw_matrix
            .into_iter()
            .map(|row| row.into_iter().map(|v| v * scale).collect())
            .collect();

        if debug {
            let sum_scaled: f64 = matrix.iter().flatten().sum();
            tracing::debug!(
                raw_total = raw_total,
                target_total = target_total,
                scale = scale,
                sum_scaled = sum_scaled,
                "bucketed vega debug"
            );
        }

        // Store as 2D matrix
        let row_labels: Vec<String> = self
            .expiries
            .iter()
            .map(|&t| {
                if t < 1.0 {
                    format!("{:.0}m", (t * 12.0).round())
                } else {
                    format!("{:.0}y", t)
                }
            })
            .collect();
        let col_labels: Vec<String> = self.strikes.iter().map(|&k| format!("{:.2}", k)).collect();

        let _ = context.store_matrix2d(MetricId::BucketedVega, row_labels, col_labels, matrix);

        Ok(target_total)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Vega]
    }
}
