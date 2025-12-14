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
    /// Volatility bump: 1% (0.01)
    pub const VOLATILITY: f64 = 0.01;
    /// Interest rate bump: 1bp, expressed in *basis points* (1.0 = 1bp).
    ///
    /// This matches `finstack_core::market_data::bumps::BumpSpec::parallel_bp(1.0)`.
    pub const INTEREST_RATE_BP: f64 = 1.0;
    /// Interest rate bump: 1bp, expressed in decimal (0.0001 = 1bp).
    pub const INTEREST_RATE_DECIMAL: f64 = 0.0001;
    /// Credit spread bump: 1bp, expressed in *basis points* (1.0 = 1bp).
    pub const CREDIT_SPREAD_BP: f64 = 1.0;
    /// Credit spread bump: 1bp, expressed in decimal (0.0001 = 1bp).
    pub const CREDIT_SPREAD_DECIMAL: f64 = 0.0001;
    /// Correlation bump: 1% (0.01)
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
/// ```rust
/// use finstack_valuations::metrics::bump_scalar_price;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::market_data::scalars::MarketScalar;
/// use finstack_core::dates::create_date;
/// use time::Month;
///
/// # fn example() -> finstack_core::Result<()> {
/// let as_of = create_date(2024, Month::January, 1)?;
/// let mut context = MarketContext::new();
///
/// // Add a spot price
/// context.insert_price_mut("AAPL", MarketScalar::Unitless(150.0));
///
/// // Bump the price up by 1%
/// let bumped = bump_scalar_price(&context, "AAPL", 0.01)?;
/// let new_price = bumped.price("AAPL")?;
/// // new_price should be 151.5 (150 * 1.01)
/// # Ok(())
/// # }
/// ```
pub fn bump_scalar_price(
    context: &finstack_core::market_data::context::MarketContext,
    price_id: &str,
    bump_pct: f64,
) -> finstack_core::Result<finstack_core::market_data::context::MarketContext> {
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
/// ```rust
/// use finstack_valuations::metrics::bump_discount_curve_parallel;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::market_data::term_structures::DiscountCurve;
/// use finstack_core::types::CurveId;
/// use finstack_core::dates::create_date;
/// use finstack_core::dates::DayCount;
/// use time::Month;
///
/// # fn example() -> finstack_core::Result<()> {
/// let as_of = create_date(2024, Month::January, 1)?;
/// let curve_id = CurveId::from("USD-OIS");
///
/// // Create a discount curve
/// let curve = DiscountCurve::builder(curve_id.clone())
///     .base_date(as_of)
///     .day_count(DayCount::Act365F)
///     .knots(vec![(0.0, 1.0), (1.0, 0.96), (5.0, 0.85)])
///     .build()?;
///
/// let context = MarketContext::new().insert_discount(curve);
///
/// // Bump the curve by 1bp (1.0 in bp units)
/// let bumped = bump_discount_curve_parallel(&context, &curve_id, 1.0)?;
/// # Ok(())
/// # }
/// ```
pub fn bump_discount_curve_parallel(
    context: &finstack_core::market_data::context::MarketContext,
    curve_id: &finstack_core::types::CurveId,
    bump_bp: f64,
) -> finstack_core::Result<finstack_core::market_data::context::MarketContext> {
    use finstack_core::market_data::bumps::BumpSpec;
    use hashbrown::HashMap;

    let mut bumps = HashMap::new();
    bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(bump_bp));
    context.bump(bumps)
}

/// Helper to scale a volatility surface by a constant multiplicative factor.
///
/// Creates a new MarketContext with a scaled volatility surface. Used for parallel
/// volatility bumps in vega calculations via finite differences.
///
/// # Arguments
///
/// * `context` - Original market context containing the volatility surface
/// * `vol_surface_id` - ID of the volatility surface to scale
/// * `scale` - Multiplicative scale factor (e.g., 1.01 for 1% increase)
///
/// # Returns
///
/// New MarketContext with the specified volatility surface scaled by the given factor.
/// All other market data remains unchanged.
///
/// # Errors
///
/// Returns an error if:
/// - The surface ID is not found in the market context
/// - The surface data is invalid or corrupted
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::metrics::scale_surface;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::create_date;
/// use time::Month;
///
/// # fn example() -> finstack_core::Result<()> {
/// let as_of = create_date(2024, Month::January, 1)?;
/// let context = MarketContext::new();
/// // Assume context has a volatility surface "AAPL-VOL"
///
/// // Scale the surface up by 1% (for vega calculation)
/// let bumped = scale_surface(&context, "AAPL-VOL", 1.01)?;
/// # Ok(())
/// # }
/// ```
pub fn scale_surface(
    context: &finstack_core::market_data::context::MarketContext,
    vol_surface_id: &str,
    scale: f64,
) -> finstack_core::Result<finstack_core::market_data::context::MarketContext> {
    let vol_surface = context.surface_ref(vol_surface_id)?;
    let bumped_surface = vol_surface.scaled(scale);
    Ok(context.clone().insert_surface(bumped_surface))
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

#[cfg(test)]
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
        .expect_err("should fail with non-positive h or k");
        match err_k {
            finstack_core::error::Error::Input(
                finstack_core::error::InputError::NonPositiveValue,
            ) => {}
            e => panic!("unexpected error: {e:?}"),
        }
    }
}
