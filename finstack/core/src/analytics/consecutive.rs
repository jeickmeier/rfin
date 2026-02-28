/// Count maximum consecutive runs where `predicate` holds.
///
/// Scans `values` left-to-right, tracking the longest streak of elements
/// satisfying the predicate (e.g., positive returns → consecutive wins).
pub fn count_consecutive<F: Fn(f64) -> bool>(values: &[f64], predicate: F) -> usize {
    let mut max_streak = 0usize;
    let mut current = 0usize;
    for &v in values {
        if predicate(v) {
            current += 1;
            if current > max_streak {
                max_streak = current;
            }
        } else {
            current = 0;
        }
    }
    max_streak
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(count_consecutive(&[], |v| v > 0.0), 0);
    }

    #[test]
    fn all_positive() {
        let data = [0.01, 0.02, 0.03];
        assert_eq!(count_consecutive(&data, |v| v > 0.0), 3);
    }

    #[test]
    fn streak_at_end() {
        let data = [-0.01, 0.02, 0.03, 0.04];
        assert_eq!(count_consecutive(&data, |v| v > 0.0), 3);
    }

    #[test]
    fn no_match() {
        let data = [-0.01, -0.02, -0.03];
        assert_eq!(count_consecutive(&data, |v| v > 0.0), 0);
    }

    #[test]
    fn alternating() {
        let data = [0.01, -0.01, 0.01, -0.01];
        assert_eq!(count_consecutive(&data, |v| v > 0.0), 1);
    }
}
