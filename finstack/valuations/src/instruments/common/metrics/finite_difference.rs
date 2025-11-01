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
/// * `bump_bp` - Bump size in basis points (e.g., 0.0001 for 1bp)
///
/// # Returns
/// New MarketContext with bumped curve
pub fn bump_discount_curve_parallel(
    context: &finstack_core::market_data::MarketContext,
    curve_id: &str,
    bump_bp: f64,
) -> finstack_core::Result<finstack_core::market_data::MarketContext> {
    use finstack_core::market_data::bumps::BumpSpec;
    use finstack_core::types::CurveId;
    use hashbrown::HashMap;

    let mut bumps = HashMap::new();
    bumps.insert(CurveId::from(curve_id), BumpSpec::parallel_bp(bump_bp));
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
    use finstack_core::market_data::surfaces::vol_surface::VolSurface;

    // Get the original surface
    let vol_surface = context.surface_ref(vol_id)?;

    // Extract surface state
    let state = vol_surface.to_state();

    // Scale all volatilities by (1 + bump_pct)
    let scale_factor = 1.0 + bump_pct;
    let bumped_vols: Vec<f64> = state
        .vols_row_major
        .iter()
        .map(|v| v * scale_factor)
        .collect();

    // Rebuild the surface with bumped volatilities
    let bumped_surface =
        VolSurface::from_grid(vol_id, &state.expiries, &state.strikes, &bumped_vols)?;

    // Clone the context and insert the bumped surface
    let bumped_context = context.clone().insert_surface(bumped_surface);

    Ok(bumped_context)
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
/// Bump size in basis points (e.g., 1.0 for 1bp = 0.0001)
pub fn adaptive_rate_bump(override_bp: Option<f64>) -> f64 {
    override_bp.unwrap_or(bump_sizes::INTEREST_RATE_BP * 10_000.0) // Convert to bp
}

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
