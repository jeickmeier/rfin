//! Common label normalization helpers for JavaScript-facing parsers.
//!
//! Provides consistent string normalization across WASM bindings to match
//! the Python bindings approach.

/// Normalize a human-entered label into snake_case for matching.
///
/// Rules:
/// - trim whitespace
/// - case-insensitive (lowercased)
/// - convert `-` to `_`
/// - convert `/` to `_` (for day count conventions like "Act/360")
#[inline]
pub fn normalize_label(input: &str) -> String {
    let trimmed = input.trim();

    // Fast path: already lowercase and contains no '-' or '/'
    let mut has_dash = false;
    let mut has_slash = false;
    let mut has_upper = false;

    for b in trimmed.as_bytes() {
        if *b == b'-' {
            has_dash = true;
        }
        if *b == b'/' {
            has_slash = true;
        }
        if b.is_ascii_uppercase() {
            has_upper = true;
        }
        if has_dash && has_slash && has_upper {
            break;
        }
    }

    if !has_dash && !has_slash && !has_upper {
        return trimmed.to_string();
    }

    trimmed.to_ascii_lowercase().replace(['-', '/'], "_")
}

#[cfg(test)]
mod tests {
    use super::normalize_label;

    #[test]
    fn normalizes_simple_cases() {
        assert_eq!(normalize_label(" following "), "following");
        assert_eq!(normalize_label("MODIFIED-FOLLOWING"), "modified_following");
        assert_eq!(normalize_label("flat-forward"), "flat_forward");
        assert_eq!(normalize_label("AwayFromZero"), "awayfromzero");
        assert_eq!(normalize_label("Act/360"), "act_360");
        assert_eq!(normalize_label("30/360"), "30_360");
    }
}
