//! Python bindings for progress reporting.
//!
//! Enables tqdm-friendly progress callbacks for long-running computations.

use finstack_core::progress::{ProgressFn, ProgressReporter};
use pyo3::prelude::*;
use std::sync::Arc;

/// Convert a Python callback to a Rust ProgressReporter.
///
/// The callback signature should be: fn(current: int, total: int, message: str) -> None
///
/// # Arguments
///
/// * `py_callback` - Optional Python function to call for progress updates
/// * `batch_size` - Report every N steps (default: 10)
///
/// # Example (Python)
///
/// ```python
/// from tqdm import tqdm
///
/// pbar = tqdm(total=100, desc="Calibrating")
/// def update_progress(current, total, msg):
///     pbar.update(current - pbar.n)
///     pbar.set_description(msg)
///
/// result = calibrate_curve(quotes, market, progress=update_progress)
/// pbar.close()
/// ```
#[allow(dead_code)] // Will be used when progress callbacks are added to calibration functions
pub fn py_to_progress_reporter(
    py_callback: Option<PyObject>,
    batch_size: Option<usize>,
) -> ProgressReporter {
    match py_callback {
        None => ProgressReporter::disabled(),
        Some(cb) => {
            let callback: ProgressFn = Arc::new(move |current, total, msg| {
                Python::with_gil(|py| {
                    // Ignore callback errors to avoid breaking the computation
                    let _ = cb.call1(py, (current, total, msg));
                });
            });
            ProgressReporter::new(Some(callback), batch_size.unwrap_or(10))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none_callback_creates_disabled_reporter() {
        let reporter = py_to_progress_reporter(None, None);
        assert!(!reporter.is_enabled());
    }

    #[test]
    fn test_custom_batch_size() {
        let reporter = py_to_progress_reporter(None, Some(25));
        assert!(!reporter.is_enabled()); // Still disabled without callback
    }
}

