#![warn(missing_docs)]

//! Progress reporting infrastructure for long-running computations.
//!
//! Provides a lightweight callback mechanism for reporting progress during
//! expensive operations like calibration, portfolio valuation, or scenario
//! analysis. Designed to work seamlessly with Python's `tqdm` and WASM's
//! async event loop.
//!
//! # Design Principles
//!
//! 1. **Opt-in**: Progress reporting is disabled by default
//! 2. **Batched updates**: Reports only every N steps to avoid callback overhead
//! 3. **Thread-safe**: Can be shared across threads (Arc<dyn Fn + Send + Sync>)
//! 4. **Zero overhead when disabled**: No-op when no callback is provided
//!
//! # Example
//!
//! ```
//! use finstack_core::progress::ProgressReporter;
//! use std::sync::Arc;
//!
//! // Create a progress reporter with a callback
//! let reporter = ProgressReporter::new(
//!     Some(Arc::new(|current, total, msg| {
//!         println!("{}: {}/{}", msg, current, total);
//!     })),
//!     10, // Report every 10 steps
//! );
//!
//! // Use in a loop
//! for i in 0..100 {
//!     reporter.report(i, 100, "Processing");
//! }
//! reporter.report(100, 100, "Complete");
//! ```

use std::sync::{Arc, Mutex};

/// Progress callback function signature.
///
/// Arguments:
/// - `current`: Current step (0-based)
/// - `total`: Total number of steps
/// - `message`: Status message
pub type ProgressFn = Arc<dyn Fn(usize, usize, &str) + Send + Sync>;

/// Progress reporter with batched updates.
///
/// Reports progress at regular intervals to avoid excessive callback overhead.
/// Thread-safe and can be cloned to share across multiple workers.
#[derive(Clone)]
pub struct ProgressReporter {
    callback: Option<ProgressFn>,
    batch_size: usize,
    last_reported: Arc<Mutex<usize>>,
}

impl ProgressReporter {
    /// Create a new progress reporter.
    ///
    /// # Arguments
    ///
    /// * `callback` - Optional callback function
    /// * `batch_size` - Minimum step increment between reports (0 = report every step)
    pub fn new(callback: Option<ProgressFn>, batch_size: usize) -> Self {
        Self {
            callback,
            batch_size,
            last_reported: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a disabled reporter (no-op, zero overhead).
    pub fn disabled() -> Self {
        Self::new(None, 0)
    }

    /// Create an enabled reporter with default batch size (10 steps).
    pub fn with_callback(callback: ProgressFn) -> Self {
        Self::new(Some(callback), 10)
    }

    /// Report progress if the batch threshold is reached or we're at the end.
    ///
    /// # Arguments
    ///
    /// * `current` - Current step (0-based)
    /// * `total` - Total number of steps
    /// * `message` - Status message
    pub fn report(&self, current: usize, total: usize, message: &str) {
        if let Some(ref cb) = self.callback {
            let mut last = self.last_reported.lock().unwrap();
            
            // Report if:
            // 1. We've advanced by batch_size steps, OR
            // 2. We're at the end (current == total), OR
            // 3. This is the first report (*last == 0 && current > 0)
            let should_report = current.saturating_sub(*last) >= self.batch_size
                || current == total
                || (*last == 0 && current > 0);
            
            if should_report {
                cb(current, total, message);
                *last = current;
            }
        }
    }

    /// Report progress unconditionally (ignores batch size).
    ///
    /// Useful for important milestones or final status updates.
    pub fn report_force(&self, current: usize, total: usize, message: &str) {
        if let Some(ref cb) = self.callback {
            cb(current, total, message);
            *self.last_reported.lock().unwrap() = current;
        }
    }

    /// Check if progress reporting is enabled.
    pub fn is_enabled(&self) -> bool {
        self.callback.is_some()
    }

    /// Get the batch size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }
}

impl Default for ProgressReporter {
    /// Default is disabled (no callback).
    fn default() -> Self {
        Self::disabled()
    }
}

impl std::fmt::Debug for ProgressReporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressReporter")
            .field("enabled", &self.callback.is_some())
            .field("batch_size", &self.batch_size)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_disabled_reporter_is_no_op() {
        let reporter = ProgressReporter::disabled();
        assert!(!reporter.is_enabled());
        
        // Should not panic
        reporter.report(0, 100, "Test");
        reporter.report(100, 100, "Done");
    }

    #[test]
    fn test_reporter_batching() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        
        let reporter = ProgressReporter::new(
            Some(Arc::new(move |_current, _total, _msg| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
            })),
            10,
        );
        
        // Report 100 times with batch size 10
        for i in 0..100 {
            reporter.report(i, 100, "Progress");
        }
        reporter.report(100, 100, "Complete");
        
        // Should be called ~10 times (0, 10, 20, ..., 90, 100)
        let calls = call_count.load(Ordering::SeqCst);
        assert!((10..=12).contains(&calls), "Expected ~11 calls, got {}", calls);
    }

    #[test]
    fn test_reporter_force_ignores_batch() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        
        let reporter = ProgressReporter::new(
            Some(Arc::new(move |_current, _total, _msg| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
            })),
            100, // Large batch size
        );
        
        // Force report multiple times
        reporter.report_force(1, 100, "Milestone 1");
        reporter.report_force(2, 100, "Milestone 2");
        reporter.report_force(3, 100, "Milestone 3");
        
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_reporter_always_reports_completion() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        
        let reporter = ProgressReporter::new(
            Some(Arc::new(move |current, total, _msg| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                // Verify last call is at completion
                if current == total {
                    assert_eq!(current, 100);
                }
            })),
            50, // Only report every 50 steps
        );
        
        for i in 0..100 {
            reporter.report(i, 100, "Progress");
        }
        reporter.report(100, 100, "Complete");
        
        let calls = call_count.load(Ordering::SeqCst);
        // Should at least report: 0 (first), 50, 100 (completion)
        assert!(calls >= 2, "Expected at least 2 calls (start + end), got {}", calls);
    }
}

