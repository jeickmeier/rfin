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
