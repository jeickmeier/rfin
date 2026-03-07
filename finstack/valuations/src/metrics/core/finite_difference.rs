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
//! - Interest rates: 1bp (**expressed as 1.0**) when using `BumpSpec::parallel_bp`
//! - Credit spreads: 1bp (**expressed as 1.0**) when using `BumpSpec::parallel_bp`
//! - Correlations: 1% (0.01)

/// Standard bump sizes for finite difference calculations.
pub mod bump_sizes {
    /// Spot/underlying price bump: 1% (0.01)
    pub const SPOT: f64 = 0.01;
    /// Volatility bump: **absolute** 1 vol point (0.01).
    ///
    /// This represents an **absolute** change in implied volatility, e.g. 20% → 21%.
    /// (Not a 1% relative scaling of the surface.)
    pub const VOLATILITY: f64 = 0.01;
    /// Correlation bump: 1% (0.01)
    ///
    /// Used by correlation sensitivity calculators (e.g., quanto options).
    #[allow(dead_code)]
    pub const CORRELATION: f64 = 0.01;
}

/// Helper to bump a scalar price in MarketContext.
///
/// Creates a new MarketContext with the bumped price, leaving other data unchanged.
/// Used for computing delta and other price sensitivities via finite differences.
///
/// # Arguments
///
/// * `context` - Original market context containing the price to bump
/// * `price_id` - ID of the price scalar to bump (e.g., equity spot price)
/// * `bump_pct` - Relative bump size as decimal (e.g., 0.01 for 1%)
///
/// # Returns
///
/// New MarketContext with the specified price bumped by the given percentage.
/// All other market data remains unchanged.
///
/// # Errors
///
/// Returns an error if:
/// - The price ID is not found in the market context
/// - The price data is invalid or corrupted
///
/// # Examples
///
/// ```rust,ignore
/// // This function is internal - use Delta metric calculators for public API
/// use finstack_valuations::metrics::core::finite_difference::bump_scalar_price;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::market_data::scalars::MarketScalar;
///
/// let context = MarketContext::new()
///     .insert_price("AAPL", MarketScalar::Unitless(150.0));
///
/// // Bump the price up by 1%
/// let bumped = bump_scalar_price(&context, "AAPL", 0.01)?;
/// ```
pub fn bump_scalar_price(
    context: &finstack_core::market_data::context::MarketContext,
    price_id: &str,
    bump_pct: f64,
) -> finstack_core::Result<finstack_core::market_data::context::MarketContext> {
    let mut bumped = context.clone();
    let current = bumped.get_price(price_id)?;

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

    bumped = bumped.insert_price(price_id, bumped_value);
    Ok(bumped)
}

/// Helper to bump a discount curve with parallel shift.
///
/// Creates a new MarketContext with a bumped discount curve using a parallel shift.
/// Used for computing DV01 (interest rate sensitivity) via finite differences.
///
/// # Arguments
///
/// * `context` - Original market context containing the discount curve
/// * `curve_id` - ID of the discount curve to bump (e.g., "USD-OIS")
/// * `bump_bp` - Bump size in basis points (e.g., 1.0 for 1bp)
///
/// # Returns
///
/// New MarketContext with the specified discount curve shifted in parallel by
/// the given amount. All other market data remains unchanged.
///
/// # Errors
///
/// Returns an error if:
/// - The curve ID is not found in the market context
/// - The bump operation fails due to invalid curve data
///
/// # Examples
///
/// ```rust,ignore
/// // This function is internal - use DV01 metric calculators for public API
/// use finstack_valuations::metrics::core::finite_difference::bump_discount_curve_parallel;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::types::CurveId;
///
/// let context = MarketContext::new();
/// let curve_id = CurveId::from("USD-OIS");
///
/// // Bump the curve by 1bp (1.0 in bp units)
/// let bumped = bump_discount_curve_parallel(&context, &curve_id, 1.0)?;
/// ```
pub fn bump_discount_curve_parallel(
    context: &finstack_core::market_data::context::MarketContext,
    curve_id: &finstack_core::types::CurveId,
    bump_bp: f64,
) -> finstack_core::Result<finstack_core::market_data::context::MarketContext> {
    use finstack_core::market_data::bumps::{BumpSpec, MarketBump};

    context.bump([MarketBump::Curve {
        id: curve_id.clone(),
        spec: BumpSpec::parallel_bp(bump_bp),
    }])
}

/// Scale a volatility surface by a multiplicative factor.
///
/// This is useful for finite-difference style vega calculations where the bump is expressed
/// as a multiplicative shock (e.g., "vol up 1%"). For market-standard vega definitions
/// (absolute bump in vol points), prefer [`bump_surface_vol_absolute`].
#[allow(dead_code)]
pub fn scale_surface(
    context: &finstack_core::market_data::context::MarketContext,
    vol_surface_id: &str,
    scale: f64,
) -> finstack_core::Result<finstack_core::market_data::context::MarketContext> {
    if !scale.is_finite() {
        return Err(finstack_core::InputError::Invalid.into());
    }
    if (scale - 1.0).abs() < 1e-15 {
        return Ok(context.clone());
    }
    let vol_surface = context.get_surface(vol_surface_id)?;
    let bumped_surface = vol_surface.scaled(scale);
    Ok(context.clone().insert_surface(bumped_surface))
}

/// Helper to bump a volatility surface by an **absolute** volatility amount (vol points).
///
/// This applies an additive parallel bump to the surface:
/// \[
/// \sigma'(t, k) = \max(0, \sigma(t, k) + \Delta\sigma)
/// \]
///
/// where `bump_abs` is expressed in *absolute* volatility units (e.g., `0.01` for +1 vol point).
///
/// # Non-Negativity Guarantee
///
/// The underlying [`VolSurface::bump`] implementation ensures that bumped volatilities
/// are floored at zero. This means:
/// - For negative bumps (e.g., `bump_abs = -0.15` on a 10% vol surface), vols are clamped to 0.0
/// - This prevents mathematically invalid negative volatilities
/// - Vega calculations near zero vol may exhibit non-linearity due to this floor
///
/// Prefer this helper for market-standard vega/volga/vanna definitions (derivatives w.r.t. σ).
pub fn bump_surface_vol_absolute(
    context: &finstack_core::market_data::context::MarketContext,
    vol_surface_id: &str,
    bump_abs: f64,
) -> finstack_core::Result<finstack_core::market_data::context::MarketContext> {
    use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
    use finstack_core::types::CurveId;

    if !bump_abs.is_finite() {
        return Err(finstack_core::InputError::Invalid.into());
    }
    if bump_abs == 0.0 {
        return Ok(context.clone());
    }

    context.bump([MarketBump::Curve {
        id: CurveId::from(vol_surface_id),
        spec: BumpSpec {
            mode: BumpMode::Additive,
            units: BumpUnits::Fraction,
            value: bump_abs,
            bump_type: BumpType::Parallel,
        },
    }])
}

/// Compute a mixed partial derivative using central differences for two
/// variables with absolute bump sizes `h` (first variable) and `k` (second).
///
/// Returns `[f(+h,+k) - f(+h,-k) - f(-h,+k) + f(-h,-k)] / (4 h k)`.
#[cfg(any(feature = "mc", test))]
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
        return Err(finstack_core::InputError::NonPositiveValue.into());
    }
    if !k_abs.is_finite() || k_abs <= 0.0 {
        return Err(finstack_core::InputError::NonPositiveValue.into());
    }
    let v_pp = eval_pp()?.into();
    let v_pm = eval_pm()?.into();
    let v_mp = eval_mp()?.into();
    let v_mm = eval_mm()?.into();
    Ok((v_pp - v_pm - v_mp + v_mm) / (4.0 * h_abs * k_abs))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

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
        .expect_err("should fail with non-positive h or k");
        match err_h {
            finstack_core::Error::Input(finstack_core::InputError::NonPositiveValue) => {}
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
        .expect_err("should fail with non-positive h or k");
        match err_k {
            finstack_core::Error::Input(finstack_core::InputError::NonPositiveValue) => {}
            e => panic!("unexpected error: {e:?}"),
        }
    }
}
