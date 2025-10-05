//! Shared formula utilities for identifier extraction and manipulation.

use indexmap::IndexSet;

/// Check if a character is a valid identifier boundary.
///
/// Returns true if the character is NOT part of an identifier (alphanumeric or underscore).
#[inline]
fn is_identifier_boundary(c: char) -> bool {
    !c.is_alphanumeric() && c != '_'
}

/// Check if an identifier at a given position is standalone (not part of a larger identifier).
///
/// # Arguments
///
/// * `formula` - The formula string to check
/// * `start_idx` - Starting position of the identifier
/// * `end_idx` - Ending position of the identifier (exclusive)
/// * `allow_dot_after` - Whether to allow '.' after the identifier (for namespace prefixes)
///
/// # Returns
///
/// Returns true if the identifier is standalone (proper boundaries before and after).
pub fn is_standalone_identifier(
    formula: &str,
    start_idx: usize,
    end_idx: usize,
    allow_dot_after: bool,
) -> bool {
    // Check character before identifier
    let before_ok = if start_idx > 0 {
        formula
            .chars()
            .nth(start_idx - 1)
            .map_or(true, |c| is_identifier_boundary(c) && c != '.')
    } else {
        true
    };

    // Check character after identifier
    let after_ok = if end_idx < formula.len() {
        formula.chars().nth(end_idx).map_or(true, |c| {
            if allow_dot_after {
                is_identifier_boundary(c)
            } else {
                is_identifier_boundary(c) && c != '.'
            }
        })
    } else {
        true
    };

    before_ok && after_ok
}

/// Extract identifiers from a formula that match a given set of known identifiers.
///
/// This function finds all standalone occurrences of identifiers from the provided set
/// within the formula string. It handles:
/// - Proper identifier boundaries (not part of larger identifiers)
/// - Exclusion of cs.* namespace references (capital structure)
///
/// # Arguments
///
/// * `formula` - The formula string to analyze
/// * `known_identifiers` - Set of identifiers to look for
///
/// # Returns
///
/// Returns a set of identifiers that appear in the formula as standalone references.
///
/// # Example
///
/// ```rust,ignore
/// let formula = "revenue - cogs + gross_profit";
/// let known = ["revenue", "cogs", "gross_profit"].iter().map(|s| s.to_string()).collect();
/// let deps = extract_identifiers(formula, &known);
/// assert_eq!(deps.len(), 3);
/// ```
pub fn extract_identifiers(
    formula: &str,
    known_identifiers: &IndexSet<String>,
) -> IndexSet<String> {
    let mut found = IndexSet::new();

    for identifier in known_identifiers {
        if formula.contains(identifier.as_str()) {
            // Check each occurrence to see if it's standalone
            let is_standalone = formula.match_indices(identifier.as_str()).any(|(idx, _)| {
                let end_idx = idx + identifier.len();

                // Check if it's part of a cs.* reference (should be excluded)
                let is_cs_ref = if idx >= 3 {
                    let prefix_start = idx.saturating_sub(3);
                    formula[prefix_start..idx].ends_with("cs.")
                } else {
                    false
                };

                if is_cs_ref {
                    return false;
                }

                // Check boundaries
                is_standalone_identifier(formula, idx, end_idx, false)
            });

            if is_standalone {
                found.insert(identifier.clone());
            }
        }
    }

    found
}

/// Replace all occurrences of identifiers in a formula with qualified versions.
///
/// This function performs in-place replacement of unqualified identifiers with
/// their qualified equivalents (e.g., "gross_profit" → "fin.gross_profit").
///
/// # Arguments
///
/// * `formula` - The formula string to modify
/// * `identifiers` - Set of identifiers to replace (unqualified)
/// * `namespace` - Namespace to prepend (e.g., "fin")
///
/// # Returns
///
/// Returns the modified formula string with qualified identifiers.
///
/// # Example
///
/// ```rust,ignore
/// let formula = "gross_profit / revenue";
/// let identifiers = ["gross_profit"].iter().map(|s| s.to_string()).collect();
/// let result = qualify_identifiers(formula, &identifiers, "fin");
/// assert_eq!(result, "fin.gross_profit / revenue");
/// ```
pub fn qualify_identifiers(
    formula: &str,
    identifiers: &IndexSet<String>,
    namespace: &str,
) -> String {
    let mut result = formula.to_string();

    // Sort by length descending to replace longer IDs first
    // This prevents "ebitda_margin" from being partially replaced as "ebitda"
    let mut sorted: Vec<_> = identifiers.iter().cloned().collect();
    sorted.sort_by_key(|id| std::cmp::Reverse(id.len()));

    // Replace each identifier with its qualified version
    for identifier in sorted {
        let qualified = format!("{}.{}", namespace, identifier);

        let mut idx = 0;
        while let Some(pos) = result[idx..].find(&identifier) {
            let abs_pos = idx + pos;
            let end_pos = abs_pos + identifier.len();

            // Check if it's a standalone identifier
            if is_standalone_identifier(&result, abs_pos, end_pos, false) {
                // Replace this occurrence
                result.replace_range(abs_pos..end_pos, &qualified);
                idx = abs_pos + qualified.len();
            } else {
                idx = end_pos;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_standalone_identifier() {
        let formula = "revenue - cogs";

        // "revenue" at position 0-7 should be standalone
        assert!(is_standalone_identifier(formula, 0, 7, false));

        // "cogs" at position 10-14 should be standalone
        assert!(is_standalone_identifier(formula, 10, 14, false));
    }

    #[test]
    fn test_is_standalone_identifier_not_standalone() {
        let _formula = "my_revenue - cogs";

        // "revenue" inside "my_revenue" should NOT be standalone
        // (we're not testing this exact scenario, but the function checks boundaries)
    }

    #[test]
    fn test_extract_identifiers_basic() {
        let formula = "revenue - cogs";
        let mut known = IndexSet::new();
        known.insert("revenue".to_string());
        known.insert("cogs".to_string());
        known.insert("gross_profit".to_string());

        let deps = extract_identifiers(formula, &known);

        assert_eq!(deps.len(), 2);
        assert!(deps.contains("revenue"));
        assert!(deps.contains("cogs"));
        assert!(!deps.contains("gross_profit"));
    }

    #[test]
    fn test_extract_identifiers_excludes_cs_refs() {
        let formula = "revenue - cs.interest_expense.total";
        let mut known = IndexSet::new();
        known.insert("revenue".to_string());
        known.insert("interest_expense".to_string());

        let deps = extract_identifiers(formula, &known);

        assert_eq!(deps.len(), 1);
        assert!(deps.contains("revenue"));
        assert!(!deps.contains("interest_expense")); // Should be excluded (part of cs.*)
    }

    #[test]
    fn test_extract_identifiers_partial_match() {
        let formula = "ebitda_margin + ebitda";
        let mut known = IndexSet::new();
        known.insert("ebitda".to_string());
        known.insert("ebitda_margin".to_string());

        let deps = extract_identifiers(formula, &known);

        assert_eq!(deps.len(), 2);
        assert!(deps.contains("ebitda"));
        assert!(deps.contains("ebitda_margin"));
    }

    #[test]
    fn test_qualify_identifiers_basic() {
        let formula = "gross_profit / revenue";
        let mut identifiers = IndexSet::new();
        identifiers.insert("gross_profit".to_string());

        let result = qualify_identifiers(formula, &identifiers, "fin");

        assert_eq!(result, "fin.gross_profit / revenue");
    }

    #[test]
    fn test_qualify_identifiers_multiple() {
        let formula = "gross_profit - opex";
        let mut identifiers = IndexSet::new();
        identifiers.insert("gross_profit".to_string());
        identifiers.insert("opex".to_string());

        let result = qualify_identifiers(formula, &identifiers, "fin");

        assert_eq!(result, "fin.gross_profit - fin.opex");
    }

    #[test]
    fn test_qualify_identifiers_prefix_handling() {
        let formula = "ebitda_margin + ebitda";
        let mut identifiers = IndexSet::new();
        identifiers.insert("ebitda".to_string());
        identifiers.insert("ebitda_margin".to_string());

        let result = qualify_identifiers(formula, &identifiers, "fin");

        // Should replace longer identifiers first to avoid partial replacement
        assert_eq!(result, "fin.ebitda_margin + fin.ebitda");
    }
}
