//! Utility functions for WASM bindings.

/// Set panic hook for better error messages in browser console
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function to get better error messages in the browser.
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
