use finstack_core::dates::Date;

/// Piecewise-constant threshold schedule for covenants.
///
/// Entries should be sorted by date ascending; the effective threshold for a
/// test date is the last entry with date <= test_date. If no entry applies,
/// `threshold_for_date` returns `None`.
#[derive(Clone, Debug)]
pub struct ThresholdSchedule(pub Vec<(Date, f64)>);

impl ThresholdSchedule {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Resolve threshold for a given test date from a piecewise-constant schedule.
pub fn threshold_for_date(schedule: &ThresholdSchedule, test_date: Date) -> Option<f64> {
    if schedule.0.is_empty() {
        return None;
    }
    // Assumes schedule is reasonably small; linear scan keeps ordering simple
    // and deterministic without additional allocations.
    let mut last: Option<f64> = None;
    for (d, v) in &schedule.0 {
        if *d <= test_date {
            last = Some(*v);
        } else {
            break;
        }
    }
    last
}


