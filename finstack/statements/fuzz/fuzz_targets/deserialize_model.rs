#![no_main]

use libfuzzer_sys::fuzz_target;
use finstack_statements::types::FinancialModelSpec;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, ignoring invalid UTF-8
    if let Ok(json_str) = std::str::from_utf8(data) {
        // Attempt to deserialize as FinancialModelSpec
        // We just want to ensure no panics occur
        // deny_unknown_fields should handle malformed input gracefully
        let _ = serde_json::from_str::<FinancialModelSpec>(json_str);
    }
});

