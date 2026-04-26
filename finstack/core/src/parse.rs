//! Label normalization for human-entered identifiers.

/// Normalize a human-entered label into snake_case for matching.
///
/// Rules:
/// - trim whitespace
/// - case-insensitive (lowercased)
/// - convert `-`, `/`, and ` ` (space) to `_`
///
/// # Examples
/// ```
/// use finstack_core::parse::normalize_label;
/// assert_eq!(normalize_label("Act/365F"), "act_365f");
/// assert_eq!(normalize_label("act-act ISDA"), "act_act_isda");
/// assert_eq!(normalize_label("  Bond Basis  "), "bond_basis");
/// ```
pub fn normalize_label(input: &str) -> String {
    input
        .trim()
        .chars()
        .flat_map(|ch| match ch {
            '-' | '/' | ' ' => '_'.to_lowercase(),
            c => c.to_lowercase(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_basic() {
        assert_eq!(normalize_label("Act/365F"), "act_365f");
        assert_eq!(normalize_label("ACT-ACT"), "act_act");
        assert_eq!(normalize_label("act/act ISDA"), "act_act_isda");
        assert_eq!(normalize_label("  Bond Basis  "), "bond_basis");
        assert_eq!(normalize_label("30/360"), "30_360");
        assert_eq!(normalize_label("already_snake"), "already_snake");
    }

    #[test]
    fn test_normalize_empty() {
        assert_eq!(normalize_label(""), "");
        assert_eq!(normalize_label("   "), "");
    }
}
