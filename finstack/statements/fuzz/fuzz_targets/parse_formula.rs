#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, ignoring invalid UTF-8
    if let Ok(formula) = std::str::from_utf8(data) {
        // Attempt to parse the formula
        // We just want to ensure no panics occur
        let _ = finstack_statements::dsl::parser::parse_formula(formula);
    }
});

