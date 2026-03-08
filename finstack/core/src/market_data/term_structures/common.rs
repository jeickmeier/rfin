//! Internal helpers shared by one-dimensional term-structure builders.
//!
//! The module keeps curve builders small by extracting common logic for
//! interpolation setup, knot splitting, and serde state. Hazard curves do not
//! rely on the interpolation engine and therefore only reuse the serde helpers.

use crate::dates::{Date, DayCount, DayCountCtx};
use crate::math::interp::types::Interp;
use crate::math::interp::{ExtrapolationPolicy, InterpStyle, ValidationPolicy};
use crate::Result;

/// Convention defaults inferred from a forward-curve identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ForwardConventionDefaults {
    pub day_count: DayCount,
    pub reset_lag_business_days: i32,
}

/// Build an `Interp` with unified error mapping (crate::Result) for callers
/// whose builders return `crate::Result<T>` (Forward/Inflation).
///
/// Preserves the original interpolation error context for better diagnostics.
#[inline]
pub(crate) fn build_interp(
    style: InterpStyle,
    knots: Box<[f64]>,
    values: Box<[f64]>,
    extrapolation: ExtrapolationPolicy,
) -> Result<Interp> {
    style.build_enum(knots, values, extrapolation, ValidationPolicy::Strict)
}

/// Build an `Interp` allowing any values (including negative forward rates).
///
/// This is used by forward curves where negative rates are allowed
/// (e.g., EUR, CHF, JPY markets since 2014).
///
/// Preserves the original interpolation error context for better diagnostics.
#[inline]
pub(crate) fn build_interp_allow_any_values(
    style: InterpStyle,
    knots: Box<[f64]>,
    values: Box<[f64]>,
    extrapolation: ExtrapolationPolicy,
) -> Result<Interp> {
    style.build_enum(
        knots,
        values,
        extrapolation,
        ValidationPolicy::AllowNegative,
    )
}

/// Build an `Interp` mapping errors to `InputError` for discount curve builders.
#[inline]
pub(crate) fn build_interp_input_error(
    style: InterpStyle,
    knots: Box<[f64]>,
    values: Box<[f64]>,
    extrapolation: ExtrapolationPolicy,
    skip_validation: bool,
) -> crate::Result<Interp> {
    // Preserve the original interpolation error (usually an InputError).
    let validation = if skip_validation {
        // Allow domain-specific builders (e.g., discount curves) to defer value validation
        // to downstream helpers while still validating knot shape upstream.
        ValidationPolicy::AllowNegative
    } else {
        ValidationPolicy::Strict
    };
    style.build_enum(knots, values, extrapolation, validation)
}

/// Compute year fraction from a base date to a target date using the given day-count.
///
/// Returns `0.0` when `date == base` without invoking the day-count engine
/// (avoids edge-case issues for same-day lookups). This is the canonical
/// helper shared by all term-structure `*_on_date()` methods.
#[inline]
pub(crate) fn year_fraction_to(base: Date, date: Date, day_count: DayCount) -> Result<f64> {
    if date == base {
        Ok(0.0)
    } else {
        Ok(day_count.year_fraction(base, date, DayCountCtx::default())?)
    }
}

/// Convenience to split points (t, v) into separate vectors.
#[inline]
pub(crate) fn split_points(points: Vec<(f64, f64)>) -> (Vec<f64>, Vec<f64>) {
    points.into_iter().unzip()
}

#[inline]
fn normalize_curve_id(id: &str) -> String {
    id.trim().to_ascii_uppercase()
}

#[inline]
fn leading_currency_code(normalized_id: &str) -> Option<&str> {
    match normalized_id.split(['-', '_']).next() {
        Some(
            code @ ("USD" | "EUR" | "GBP" | "JPY" | "CHF" | "CAD" | "AUD" | "NZD" | "SEK" | "NOK"),
        ) => Some(code),
        _ => None,
    }
}

#[inline]
fn contains_any(normalized_id: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| normalized_id.contains(needle))
}

#[inline]
fn has_explicit_term_marker(normalized_id: &str) -> bool {
    contains_any(
        normalized_id,
        &[
            "1D", "1W", "2W", "1M", "2M", "3M", "6M", "9M", "12M", "18M", "1Y",
        ],
    )
}

#[inline]
fn inferred_currency_day_count(currency: &str) -> DayCount {
    match currency {
        "USD" | "EUR" | "CHF" | "SEK" | "NOK" => DayCount::Act360,
        "GBP" | "JPY" | "CAD" | "AUD" | "NZD" => DayCount::Act365F,
        _ => DayCount::Act365F,
    }
}

/// Infer a market-standard day-count basis from a curve identifier.
///
/// The fallback remains `Act365F` for synthetic IDs that carry no market hint.
#[inline]
pub(crate) fn infer_discount_curve_day_count(id: &str) -> DayCount {
    let normalized_id = normalize_curve_id(id);

    if contains_any(
        &normalized_id,
        &["SOFR", "FEDFUNDS", "EFFR", "ESTR", "EURIBOR", "SARON"],
    ) {
        return DayCount::Act360;
    }

    if contains_any(
        &normalized_id,
        &[
            "SONIA", "TONAR", "TONA", "TIBOR", "CORRA", "CDOR", "AONIA", "BBSW", "BKBM",
        ],
    ) {
        return DayCount::Act365F;
    }

    if let Some(currency) = leading_currency_code(&normalized_id) {
        return inferred_currency_day_count(currency);
    }

    DayCount::Act365F
}

/// Infer forward-curve day-count and reset-lag defaults from an index identifier.
///
/// Reset lag is interpreted in business days using positive T-minus semantics.
#[inline]
pub(crate) fn infer_forward_curve_defaults(id: &str) -> ForwardConventionDefaults {
    let normalized_id = normalize_curve_id(id);
    let day_count = infer_discount_curve_day_count(id);

    let is_overnight = normalized_id.contains("OIS")
        || contains_any(
            &normalized_id,
            &[
                "SONIA", "TONAR", "TONA", "SARON", "ESTR", "FEDFUNDS", "EFFR", "CORRA", "AONIA",
            ],
        )
        || (normalized_id.contains("SOFR") && !has_explicit_term_marker(&normalized_id));

    let reset_lag_business_days = if is_overnight {
        0
    } else if contains_any(
        &normalized_id,
        &["SOFR", "EURIBOR", "LIBOR", "TIBOR", "BBSW", "CDOR", "BKBM"],
    ) {
        2
    } else {
        0
    };

    ForwardConventionDefaults {
        day_count,
        reset_lag_business_days,
    }
}

/// Calculate triangular weight for key-rate DV01.
///
/// Returns a weight in [0, 1] that peaks at `target` and linearly decays to 0
/// at `prev` and `next`. This function defines the weight based on the **bucket grid**,
/// ensuring that the sum of all bucket weights at any time t equals 1.0.
///
/// # Arguments
/// * `t` - The time at which to calculate the weight
/// * `prev` - Previous bucket time (0.0 for first bucket)
/// * `target` - Target bucket time (peak of the triangle)
/// * `next` - Next bucket time (f64::INFINITY for last bucket)
///
/// # Returns
/// Weight in [0, 1] representing the contribution of this bucket to the rate at time t.
#[inline]
pub(crate) fn triangular_weight(t: f64, prev: f64, target: f64, next: f64) -> f64 {
    if t <= prev {
        0.0
    } else if t <= target {
        // Rising edge: 0 at prev, 1 at target
        let denom = (target - prev).max(1e-10);
        (t - prev) / denom
    } else if next.is_infinite() {
        // Last bucket: flat weight of 1.0 beyond target
        1.0
    } else if t < next {
        // Falling edge: 1 at target, 0 at next
        let denom = (next - target).max(1e-10);
        (next - t) / denom
    } else {
        0.0
    }
}

/// Helper to shift knot times backward by `dt` and filter out expired points (t <= 0).
///
/// Used by `roll_forward` implementations in discount and forward curves.
#[inline]
pub(crate) fn roll_knots(knots: &[f64], values: &[f64], dt: f64) -> Vec<(f64, f64)> {
    knots
        .iter()
        .zip(values.iter())
        .filter_map(|(&t, &v)| {
            let new_t = t - dt;
            if new_t > 0.0 {
                Some((new_t, v))
            } else {
                None
            }
        })
        .collect()
}

/// Validate that a value is within the unit range `[0.0, 1.0]`.
///
/// Returns an error with a descriptive message if the value is out of range.
/// Used by hazard curve recovery rates, base correlation values, etc.
#[inline]
pub(crate) fn validate_unit_range(value: f64, field_name: &str) -> crate::Result<()> {
    if !(0.0..=1.0).contains(&value) {
        return Err(crate::error::InputError::Invalid.into());
    }
    let _ = field_name; // used in error context if needed in the future
    Ok(())
}

// -----------------------------------------------------------------------------
// Shared serde state fragments to DRY curve state definitions
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateId {
    /// Curve identifier
    pub id: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateKnotPoints {
    /// Time/value pairs used to construct the curve
    pub knot_points: Vec<(f64, f64)>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateInterp {
    /// Interpolation style
    pub interp_style: InterpStyle,
    /// Extrapolation policy
    pub extrapolation: ExtrapolationPolicy,
}
