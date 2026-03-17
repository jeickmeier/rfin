use crate::estimate::Estimate;
use crate::paths::SimulatedPath;

pub(crate) fn apply_captured_path_statistics(
    estimate: Estimate,
    paths: &[SimulatedPath],
) -> Estimate {
    if paths.is_empty() {
        return estimate;
    }

    let mut values: Vec<f64> = paths.iter().map(|path| path.final_value).collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = values.len();
    let median = if len.is_multiple_of(2) {
        (values[len / 2 - 1] + values[len / 2]) / 2.0
    } else {
        values[len / 2]
    };
    let percentile_25 = values[((len as f64 * 0.25).floor() as usize).min(len - 1)];
    let percentile_75 = values[((len as f64 * 0.75).floor() as usize).min(len - 1)];
    let min = values[0];
    let max = values[len - 1];

    estimate
        .with_median(median)
        .with_percentiles(percentile_25, percentile_75)
        .with_range(min, max)
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::apply_captured_path_statistics;
    use crate::estimate::Estimate;
    use crate::paths::SimulatedPath;

    fn captured_path(path_id: usize, final_value: f64) -> SimulatedPath {
        let mut path = SimulatedPath::new(path_id);
        path.set_final_value(final_value);
        path
    }

    #[test]
    fn test_apply_captured_path_statistics_uses_expected_percentiles() {
        let estimate = Estimate::new(0.0, 0.0, (0.0, 0.0), 5);
        let paths = vec![
            captured_path(0, 0.625),
            captured_path(1, 0.125),
            captured_path(2, 0.375),
            captured_path(3, 0.5),
            captured_path(4, 0.25),
        ];

        let estimate = apply_captured_path_statistics(estimate, &paths);

        assert_eq!(estimate.median, Some(0.375));
        assert_eq!(estimate.percentile_25, Some(0.25));
        assert_eq!(estimate.percentile_75, Some(0.5));
        assert_eq!(estimate.min, Some(0.125));
        assert_eq!(estimate.max, Some(0.625));
    }
}
