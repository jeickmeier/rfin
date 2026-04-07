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

/// Apply an additive parallel bump to a slice of (t, value) knots.
///
/// Each value is clamped to zero from below: `max(0, v + bump)`.
/// Returns the bumped knots as a new `Vec`.
#[inline]
pub(crate) fn bump_knots_parallel(knots: &[f64], values: &[f64], bump: f64) -> Vec<(f64, f64)> {
    knots
        .iter()
        .zip(values.iter())
        .map(|(&t, &v)| (t, (v + bump).max(0.0)))
        .collect()
}

/// Apply a multiplicative percentage bump to a slice of (t, value) knots.
///
/// Each value is scaled by `1 + pct` and clamped to zero from below.
#[inline]
pub(crate) fn bump_knots_percentage(knots: &[f64], values: &[f64], pct: f64) -> Vec<(f64, f64)> {
    let factor = 1.0 + pct;
    knots
        .iter()
        .zip(values.iter())
        .map(|(&t, &v)| (t, (v * factor).max(0.0)))
        .collect()
}

/// Apply a triangular key-rate bump to a slice of (t, value) knots.
///
/// Each knot receives a weight in `[0, 1]` based on its proximity to
/// `target_bucket`. Spot (t=0) is typically excluded by the caller.
#[inline]
pub(crate) fn bump_knots_triangular(
    knots: &[f64],
    values: &[f64],
    prev_bucket: f64,
    target_bucket: f64,
    next_bucket: f64,
    bump: f64,
) -> Vec<(f64, f64)> {
    knots
        .iter()
        .zip(values.iter())
        .map(|(&t, &v)| {
            let w = triangular_weight(t, prev_bucket, target_bucket, next_bucket);
            (t, (v + bump * w).max(0.0))
        })
        .collect()
}

/// Validate that all values in a knot slice are non-negative.
///
/// Returns a descriptive error that includes the tenor and value for quick diagnosis.
pub(crate) fn validate_non_negative_knots(
    knots: &[f64],
    values: &[f64],
    value_label: &str,
) -> crate::Result<()> {
    for (i, (&t, &v)) in knots.iter().zip(values.iter()).enumerate() {
        if v < 0.0 {
            return Err(crate::Error::Validation(format!(
                "{value_label} must be non-negative at t={t:.6}: value={v:.8} (index {i})"
            )));
        }
    }
    Ok(())
}

/// Infer the spot value from a knot set when the first knot is at t≈0.
///
/// Returns `Some(v)` when the first knot is at t=0 (within 1e-14), otherwise `None`.
#[inline]
pub(crate) fn infer_spot_from_knots(knots: &[f64], values: &[f64]) -> Option<f64> {
    knots
        .first()
        .filter(|&&t| t.abs() <= 1e-14)
        .map(|_| values[0])
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
