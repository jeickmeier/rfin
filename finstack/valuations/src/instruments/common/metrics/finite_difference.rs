//! Generic finite difference utilities for risk metric calculations.
//!
//! Provides standard bump sizes and helper functions for bump-and-reprice
//! sensitivity calculations. Each metric calculator implements its own
//! bumping and revaluation logic based on the instrument type.
//!
//! # Bump Size Standards
//!
//! Following market conventions:
//! - Spot/Underlying: 1% (0.01)
//! - Volatility: 1% (0.01)
//! - Interest rates: 1bp (0.0001)
//! - Credit spreads: 1bp (0.0001)
//! - Correlations: 1% (0.01)

/// Standard bump sizes for finite difference calculations.
pub mod bump_sizes {
    /// Spot/underlying price bump: 1% (0.01)
    pub const SPOT: f64 = 0.01;
    /// Volatility bump: 1% (0.01)
    pub const VOLATILITY: f64 = 0.01;
    /// Interest rate bump: 1bp (0.0001)
    pub const INTEREST_RATE_BP: f64 = 0.0001;
    /// Credit spread bump: 1bp (0.0001)
    pub const CREDIT_SPREAD_BP: f64 = 0.0001;
    /// Correlation bump: 1% (0.01)
    pub const CORRELATION: f64 = 0.01;
}

/// Convenience alias: bump a discount curve in parallel basis points.
///
/// Wrapper around `bump_discount_curve_parallel` to standardize naming.
pub fn bump_discount(
    context: &finstack_core::market_data::MarketContext,
    curve_id: &finstack_core::types::CurveId,
    bump_decimal: f64,
) -> finstack_core::Result<finstack_core::market_data::MarketContext> {
    // 1bp == 0.0001 (decimal)
    bump_discount_curve_parallel(context, curve_id, bump_decimal)
}

/// Helper to bump a scalar price in MarketContext.
///
/// Creates a new MarketContext with the bumped price, leaving other data unchanged.
///
/// # Arguments
/// * `context` - Original market context
/// * `price_id` - ID of the price scalar to bump
/// * `bump_pct` - Relative bump size (e.g., 0.01 for 1%)
///
/// # Returns
/// New MarketContext with bumped price
pub fn bump_scalar_price(
    context: &finstack_core::market_data::MarketContext,
    price_id: &str,
    bump_pct: f64,
) -> finstack_core::Result<finstack_core::market_data::MarketContext> {
    use finstack_core::types::CurveId;

    let mut bumped = context.clone();
    let current = bumped.price(price_id)?;

    let bumped_value = match current {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v * (1.0 + bump_pct))
        }
        finstack_core::market_data::scalars::MarketScalar::Price(m) => {
            finstack_core::market_data::scalars::MarketScalar::Price(
                finstack_core::money::Money::new(m.amount() * (1.0 + bump_pct), m.currency()),
            )
        }
    };

    bumped.prices.insert(CurveId::from(price_id), bumped_value);
    Ok(bumped)
}

/// Helper to bump a discount curve with parallel shift.
///
/// Creates a new MarketContext with a bumped discount curve using a parallel shift.
///
/// # Arguments
/// * `context` - Original market context
/// * `curve_id` - ID of the discount curve to bump
/// * `bump_decimal` - Bump size in decimal (e.g., 0.0001 for 1bp)
///
/// # Returns
/// New MarketContext with bumped curve
pub fn bump_discount_curve_parallel(
    context: &finstack_core::market_data::MarketContext,
    curve_id: &finstack_core::types::CurveId,
    bump_decimal: f64,
) -> finstack_core::Result<finstack_core::market_data::MarketContext> {
    use finstack_core::market_data::bumps::BumpSpec;
    use hashbrown::HashMap;

    let mut bumps = HashMap::new();
    bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(bump_decimal));
    context.bump(bumps)
}

/// Helper to bump a forward curve with a parallel shift (in basis points).
///
/// Creates a new `MarketContext` with the specified forward curve bumped and
/// replaced under the same ID.
pub fn bump_forward(
    context: &finstack_core::market_data::MarketContext,
    curve_id: &finstack_core::types::CurveId,
    bump_decimal: f64,
) -> finstack_core::Result<finstack_core::market_data::MarketContext> {
    use finstack_core::market_data::bumps::BumpSpec;
    use hashbrown::HashMap;

    let mut bumps = HashMap::new();
    bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(bump_decimal));
    context.bump(bumps)
}

/// Helper to bump a volatility surface by a percentage.
///
/// Creates a new surface with all volatilities scaled by (1 + bump_pct).
/// This is useful for computing parallel vega (sensitivity to overall vol level).
/// For point-wise bumps (bucketed vega), use `VolSurface::bump_point()` directly.
///
/// # Arguments
/// * `context` - Original market context  
/// * `vol_id` - ID of the volatility surface
/// * `bump_pct` - Relative bump size (e.g., 0.01 for 1% increase)
///
/// # Returns
/// New MarketContext with bumped volatility surface (all vols scaled)
///
/// # Errors
/// Returns error if the volatility surface is not found in the context
///
/// # Examples
/// ```rust,ignore
/// use finstack_valuations::instruments::common::metrics::finite_difference::bump_vol_surface_parallel;
///
/// // Bump all volatilities by 1%
/// let bumped_context = bump_vol_surface_parallel(&context, "EQ-VOL", 0.01)?;
/// ```
pub fn bump_vol_surface_parallel(
    context: &finstack_core::market_data::MarketContext,
    vol_id: &str,
    bump_pct: f64,
) -> finstack_core::Result<finstack_core::market_data::MarketContext> {
    scale_surface(context, vol_id, 1.0 + bump_pct)
}

/// Helper to scale a volatility surface by a constant multiplicative factor.
///
/// This is the core utility used by `bump_vol_surface_parallel` and can also be
/// applied directly when the caller already computed the desired scale.
pub fn scale_surface(
    context: &finstack_core::market_data::MarketContext,
    vol_id: &str,
    scale: f64,
) -> finstack_core::Result<finstack_core::market_data::MarketContext> {
    let vol_surface = context.surface_ref(vol_id)?;
    let bumped_surface = vol_surface.scaled(scale);
    Ok(context.clone().insert_surface(bumped_surface))
}

// -----------------------------------------------------------------------------
// Adaptive Bump Size Calculations
// -----------------------------------------------------------------------------

/// Calculate adaptive spot bump size based on volatility and time to expiry.
///
/// Adaptive bumps scale based on:
/// - Base bump size (1% of spot)
/// - Volatility-adjusted component: 0.1% * spot * σ * √T
/// - Minimum: base bump (1%)
/// - Maximum: 5% of spot
///
/// This improves numerical stability for high-vol or long-dated options
/// where standard 1% bumps may be too small relative to price uncertainty.
///
/// # Arguments
/// * `spot` - Current spot price
/// * `atm_vol` - At-the-money volatility (annualized)
/// * `time_to_expiry` - Time to expiry in years
/// * `override_pct` - Optional override from PricingOverrides (takes precedence)
///
/// # Returns
/// Adaptive bump size as percentage (e.g., 0.01 for 1%)
pub fn adaptive_spot_bump(
    _spot: f64,
    atm_vol: f64,
    time_to_expiry: f64,
    override_pct: Option<f64>,
) -> f64 {
    if let Some(pct) = override_pct {
        return pct;
    }

    let base_bump = bump_sizes::SPOT; // 1%
    let vol_scaled = 0.001 * atm_vol * time_to_expiry.sqrt(); // 0.1% * σ * √T

    // Use the larger of base bump and vol-scaled bump, but cap at 5%
    base_bump.max(vol_scaled).min(0.05)
}

// -----------------------------------------------------------------------------
// Central finite-difference helpers
// -----------------------------------------------------------------------------

/// Compute a first derivative using central differences given evaluators for
/// up/down scenarios and an absolute bump size `h`.
///
/// Returns `(f(x+h) - f(x-h)) / (2h)`.
pub fn central_diff_1d<EFutUp, EFutDown, E>(
    eval_up: EFutUp,
    eval_down: EFutDown,
    h_abs: f64,
) -> finstack_core::Result<f64>
where
    EFutUp: FnOnce() -> finstack_core::Result<E>,
    EFutDown: FnOnce() -> finstack_core::Result<E>,
    E: Into<f64>,
{
    // Guard against invalid bump size
    if !h_abs.is_finite() || h_abs <= 0.0 {
        return Err(finstack_core::error::InputError::NonPositiveValue.into());
    }
    let up = eval_up()?.into();
    let down = eval_down()?.into();
    Ok((up - down) / (2.0 * h_abs))
}

/// Compute a mixed partial derivative using central differences for two
/// variables with absolute bump sizes `h` (first variable) and `k` (second).
///
/// Returns `[f(+h,+k) - f(+h,-k) - f(-h,+k) + f(-h,-k)] / (4 h k)`.
pub fn central_mixed<EEpp, EEpm, EEmp, Eemm, E1, E2, E3, E4>(
    eval_pp: EEpp,
    eval_pm: EEpm,
    eval_mp: EEmp,
    eval_mm: Eemm,
    h_abs: f64,
    k_abs: f64,
) -> finstack_core::Result<f64>
where
    EEpp: FnOnce() -> finstack_core::Result<E1>,
    EEpm: FnOnce() -> finstack_core::Result<E2>,
    EEmp: FnOnce() -> finstack_core::Result<E3>,
    Eemm: FnOnce() -> finstack_core::Result<E4>,
    E1: Into<f64>,
    E2: Into<f64>,
    E3: Into<f64>,
    E4: Into<f64>,
{
    // Guard against invalid bump sizes
    if !h_abs.is_finite() || h_abs <= 0.0 {
        return Err(finstack_core::error::InputError::NonPositiveValue.into());
    }
    if !k_abs.is_finite() || k_abs <= 0.0 {
        return Err(finstack_core::error::InputError::NonPositiveValue.into());
    }
    let v_pp = eval_pp()?.into();
    let v_pm = eval_pm()?.into();
    let v_mp = eval_mp()?.into();
    let v_mm = eval_mm()?.into();
    Ok((v_pp - v_pm - v_mp + v_mm) / (4.0 * h_abs * k_abs))
}

/// Calculate adaptive volatility bump size based on current volatility level.
///
/// Adaptive bumps scale with volatility to maintain relative accuracy:
/// - Low vol (< 5%): use 1% absolute bump
/// - Medium vol (5-20%): use 5% relative bump
/// - High vol (> 20%): use 10% relative bump, capped at 5% absolute
///
/// # Arguments
/// * `current_vol` - Current volatility level (annualized)
/// * `override_pct` - Optional override from PricingOverrides (takes precedence)
///
/// # Returns
/// Adaptive bump size as absolute volatility (e.g., 0.01 for 1% vol)
pub fn adaptive_vol_bump(current_vol: f64, override_pct: Option<f64>) -> f64 {
    if let Some(pct) = override_pct {
        return pct;
    }

    if current_vol < 0.05 {
        // Low volatility: use fixed 1% absolute bump
        bump_sizes::VOLATILITY.max(0.001)
    } else if current_vol < 0.20 {
        // Medium volatility: use 5% relative bump
        (current_vol * 0.05).clamp(0.001, 0.05)
    } else {
        // High volatility: use 10% relative bump, capped at 5% absolute
        (current_vol * 0.10).clamp(0.001, 0.05)
    }
}

/// Calculate adaptive rate bump size.
///
/// For rates, we generally use fixed basis point bumps as they're more
/// stable. However, this function allows for potential future extensions.
///
/// # Arguments
/// * `override_bp` - Optional override from PricingOverrides (takes precedence)
///
/// # Returns
/// Bump size in decimal (e.g., 0.0001 for 1bp)
pub fn adaptive_rate_bump(override_decimal: Option<f64>) -> f64 {
    override_decimal.unwrap_or(bump_sizes::INTEREST_RATE_BP)
}

// tests moved to end of file to satisfy clippy::items_after_test_module

/// Get bump sizes from PricingOverrides if adaptive bumps are enabled.
///
/// Returns (spot_bump_pct, vol_bump_pct, rate_bump_bp) as Options.
/// If adaptive is disabled or override is None, returns None for that component.
pub fn get_bump_overrides(
    overrides: &crate::instruments::PricingOverrides,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    if !overrides.adaptive_bumps {
        return (None, None, None);
    }

    (
        overrides.spot_bump_pct,
        overrides.vol_bump_pct,
        overrides.rate_bump_bp,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_rate_bump_default_is_one_bp_decimal() {
        let v = adaptive_rate_bump(None);
        assert!((v - 0.0001).abs() < 1e-12);
    }

    #[test]
    fn central_diff_1d_rejects_nonpositive_or_invalid_h() {
        let err = central_diff_1d(|| Ok(1.0f64), || Ok(1.0f64), 0.0).unwrap_err();
        match err {
            finstack_core::error::Error::Input(
                finstack_core::error::InputError::NonPositiveValue,
            ) => {}
            e => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn central_mixed_rejects_nonpositive_or_invalid_hk() {
        let err_h = central_mixed(
            || Ok(0.0f64),
            || Ok(0.0f64),
            || Ok(0.0f64),
            || Ok(0.0f64),
            0.0,
            1.0,
        )
        .unwrap_err();
        match err_h {
            finstack_core::error::Error::Input(
                finstack_core::error::InputError::NonPositiveValue,
            ) => {}
            e => panic!("unexpected error: {e:?}"),
        }
        let err_k = central_mixed(
            || Ok(0.0f64),
            || Ok(0.0f64),
            || Ok(0.0f64),
            || Ok(0.0f64),
            1.0,
            0.0,
        )
        .unwrap_err();
        match err_k {
            finstack_core::error::Error::Input(
                finstack_core::error::InputError::NonPositiveValue,
            ) => {}
            e => panic!("unexpected error: {e:?}"),
        }
    }
}
