//! Memory size estimation for cached valuation results.

use crate::results::ValuationResult;

/// Estimate heap size of a [`ValuationResult`] for cache memory tracking.
///
/// This is approximate: the goal is to bound total memory usage, not to
/// be byte-exact. Counts the instrument_id string, the measures map
/// entries, and a fixed overhead for struct fields.
///
/// # Arguments
///
/// * `result` - The valuation result to estimate size for.
///
/// # Returns
///
/// Estimated heap size in bytes.
pub(crate) fn estimate_result_size(result: &ValuationResult) -> usize {
    let base = std::mem::size_of::<ValuationResult>();
    let id_size = result.instrument_id.len();
    // Each measure entry: MetricId (string ~16 bytes) + f64 (8 bytes) + overhead
    let measures_size = result.measures.len() * 64;
    base + id_size + measures_size
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::create_date;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    fn estimate_size_nonzero() {
        let as_of = create_date(2025, Month::January, 15).expect("valid date");
        let pv = Money::new(1_000_000.0, Currency::USD);
        let result = ValuationResult::stamped("BOND-001", as_of, pv);
        let size = estimate_result_size(&result);
        assert!(size > 0, "estimated size should be positive");
        assert!(
            size >= std::mem::size_of::<ValuationResult>(),
            "should be at least the struct size"
        );
    }
}
