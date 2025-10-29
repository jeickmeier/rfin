//! WASM bindings for progress reporting.
//!
//! **Note**: WASM progress callbacks work differently from Python due to single-threaded nature.
//! This module provides the infrastructure for when progress is integrated into WASM functions.

use finstack_core::progress::ProgressReporter;

/// Create a disabled progress reporter for WASM (placeholder).
///
/// **Note**: WASM progress callbacks require special handling due to single-threaded
/// execution model. When integrated, progress will be reported via:
/// - Console logging
/// - Custom events dispatched to window
/// - Direct callback invocation (blocking)
///
/// For now, returns disabled reporter until full async integration is implemented.
///
/// # Future API (JavaScript)
///
/// ```javascript
/// const callback = (current, total, message) => {
///     console.log(`${message}: ${current}/${total}`);
///     updateProgressBar(current / total);
/// };
///
/// // When implemented:
/// const result = await calibrateCurve(quotes, market, opts, callback);
/// ```
#[allow(dead_code)]
pub fn create_wasm_progress_reporter() -> ProgressReporter {
    // WASM is single-threaded, so we can't use Arc<dyn Fn + Send + Sync>
    // For now, return disabled. Full integration requires:
    // 1. Custom WASM-specific callback mechanism
    // 2. Event dispatching or promise-based async
    // 3. Non-blocking progress updates
    ProgressReporter::disabled()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creates_disabled_reporter() {
        let reporter = create_wasm_progress_reporter();
        assert!(!reporter.is_enabled());
    }
}
