use crate::dates::{Date, DayCount, DayCountCtx};

/// Compute signed year fraction between two dates using the provided day count.
///
/// Returns a positive fraction when `date` is after `base`, negative when before,
/// and zero when equal. Any day-count errors are propagated to the caller.
pub(crate) fn signed_year_fraction(
    dc: DayCount,
    base: Date,
    date: Date,
    ctx: DayCountCtx<'_>,
) -> crate::Result<f64> {
    if date == base {
        Ok(0.0)
    } else if date > base {
        dc.year_fraction(base, date, ctx)
    } else {
        Ok(-dc.year_fraction(date, base, ctx)?)
    }
}

/// Return `true` if the iterator contains at least one positive and one negative value.
pub(crate) fn has_sign_change<I>(iter: I) -> bool
where
    I: IntoIterator<Item = f64>,
{
    let mut has_positive = false;
    let mut has_negative = false;

    for v in iter {
        if v > 0.0 {
            has_positive = true;
        } else if v < 0.0 {
            has_negative = true;
        }
        if has_positive && has_negative {
            return true;
        }
    }
    false
}
