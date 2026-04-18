//! Name normalization and alias registry (crate-internal).
//!
//! Internal helper used by `ModelBuilder::with_name_normalization` to rewrite
//! user-authored identifiers into canonical node IDs via exact alias match or
//! Jaro-Winkler fuzzy match. Not part of the public API — callers control
//! normalization through the `ModelBuilder` methods.

use indexmap::{IndexMap, IndexSet};

/// Jaro-Winkler similarity threshold for fuzzy matching (0.0 exact-only → 1.0 permissive).
const FUZZY_THRESHOLD: f64 = 0.85;

/// Registry for name aliases and normalization.
#[derive(Debug, Clone)]
pub(crate) struct AliasRegistry {
    aliases: IndexMap<String, String>,
}

impl AliasRegistry {
    /// Create a new empty alias registry.
    pub fn new() -> Self {
        Self {
            aliases: IndexMap::new(),
        }
    }

    /// Add an alias mapping. The alias key is lower-cased and stripped of
    /// non-alphanumeric characters for matching.
    pub fn add_alias(&mut self, alias: impl Into<String>, canonical: impl Into<String>) {
        let alias_str = alias.into();
        let canonical_str = canonical.into();
        let normalized_alias = normalize_string(&alias_str);
        self.aliases.insert(normalized_alias, canonical_str);
    }

    /// Add multiple aliases pointing at the same canonical name.
    pub fn add_aliases(&mut self, canonical: impl Into<String>, aliases: Vec<String>) {
        let canonical_str = canonical.into();
        for alias in aliases {
            self.add_alias(alias, canonical_str.clone());
        }
    }

    /// Normalize a name to its canonical form using exact alias matching.
    pub fn normalize(&self, input: &str) -> Option<String> {
        let normalized_input = normalize_string(input);
        self.aliases.get(&normalized_input).cloned()
    }

    /// Normalize with fuzzy matching against available nodes. Tries the exact
    /// alias table first, then falls back to Jaro-Winkler above [`FUZZY_THRESHOLD`].
    pub fn normalize_fuzzy(
        &self,
        input: &str,
        available_nodes: &IndexSet<String>,
    ) -> Option<String> {
        if let Some(canonical) = self.normalize(input) {
            return Some(canonical);
        }
        fuzzy_match(input, available_nodes, FUZZY_THRESHOLD)
    }

    /// Populate the registry with standard financial-statement aliases
    /// (revenue/cogs/ebit/…). Used by `ModelBuilder::with_name_normalization`.
    pub fn load_standard_aliases(&mut self) {
        self.add_aliases(
            "revenue",
            vec![
                "rev".to_string(),
                "sales".to_string(),
                "turnover".to_string(),
                "top_line".to_string(),
                "topline".to_string(),
            ],
        );

        self.add_aliases(
            "cogs",
            vec![
                "cost_of_sales".to_string(),
                "cost_of_goods_sold".to_string(),
                "cos".to_string(),
            ],
        );

        self.add_aliases(
            "operating_expenses",
            vec![
                "opex".to_string(),
                "operating_expense".to_string(),
                "op_exp".to_string(),
            ],
        );

        self.add_aliases(
            "sga",
            vec![
                "sg&a".to_string(),
                "selling_general_admin".to_string(),
                "selling_general_administrative".to_string(),
            ],
        );

        self.add_aliases(
            "gross_profit",
            vec!["gp".to_string(), "gross_margin_dollars".to_string()],
        );

        self.add_aliases(
            "ebitda",
            vec!["earnings_before_interest_taxes_depreciation_amortization".to_string()],
        );

        self.add_aliases(
            "ebit",
            vec![
                "operating_income".to_string(),
                "earnings_before_interest_taxes".to_string(),
            ],
        );

        self.add_aliases(
            "net_income",
            vec![
                "ni".to_string(),
                "net_profit".to_string(),
                "bottom_line".to_string(),
                "bottomline".to_string(),
                "earnings".to_string(),
            ],
        );

        self.add_aliases(
            "depreciation_amortization",
            vec![
                "d&a".to_string(),
                "da".to_string(),
                "depreciation_and_amortization".to_string(),
            ],
        );

        self.add_aliases(
            "interest_expense",
            vec!["int_exp".to_string(), "interest".to_string()],
        );

        self.add_aliases(
            "tax_expense",
            vec!["taxes".to_string(), "income_tax".to_string()],
        );

        self.add_aliases(
            "capex",
            vec![
                "capital_expenditures".to_string(),
                "capital_expenditure".to_string(),
                "cap_ex".to_string(),
            ],
        );

        self.add_aliases(
            "free_cash_flow",
            vec!["fcf".to_string(), "free_cashflow".to_string()],
        );
    }
}

impl Default for AliasRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a string for comparison (lowercase, remove underscores/spaces).
fn normalize_string(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Fuzzy match input against candidates using Jaro-Winkler similarity.
fn fuzzy_match(input: &str, candidates: &IndexSet<String>, threshold: f64) -> Option<String> {
    let normalized_input = normalize_string(input);
    candidates
        .iter()
        .map(|c| (jaro_winkler(&normalized_input, &normalize_string(c)), c))
        .filter(|(score, _)| *score >= threshold)
        .max_by(|a, b| {
            // Jaro-Winkler scores are always in [0, 1] and never NaN.
            a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, name)| name.clone())
}

/// Jaro-Winkler similarity in [0.0, 1.0]. Gives a prefix bonus (up to 4 chars).
fn jaro_winkler(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    let jaro = jaro_similarity(s1, s2);
    let prefix_len = s1
        .chars()
        .zip(s2.chars())
        .take(4)
        .take_while(|(c1, c2)| c1 == c2)
        .count()
        .min(4);

    (jaro + (prefix_len as f64 * 0.1 * (1.0 - jaro))).min(1.0)
}

fn jaro_similarity(s1: &str, s2: &str) -> f64 {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let s1_len = s1_chars.len();
    let s2_len = s2_chars.len();

    if s1_len == 0 && s2_len == 0 {
        return 1.0;
    }
    if s1_len == 0 || s2_len == 0 {
        return 0.0;
    }

    let match_distance = if s1_len.max(s2_len) / 2 > 0 {
        s1_len.max(s2_len) / 2 - 1
    } else {
        0
    };

    let s1_matches = &mut vec![false; s1_len];
    let s2_matches = &mut vec![false; s2_len];

    let mut matches = 0;
    let mut transpositions = 0;

    for (i, &c1) in s1_chars.iter().enumerate() {
        let start = i.saturating_sub(match_distance);
        let end = (i + match_distance + 1).min(s2_len);

        for j in start..end {
            if s2_matches[j] || c1 != s2_chars[j] {
                continue;
            }
            s1_matches[i] = true;
            s2_matches[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    let mut k = 0;
    for (i, &matched) in s1_matches.iter().enumerate() {
        if !matched {
            continue;
        }
        while k < s2_matches.len() && !s2_matches[k] {
            k += 1;
        }
        if k < s2_chars.len() && s1_chars[i] != s2_chars[k] {
            transpositions += 1;
        }
        k += 1;
    }

    let matches_f = matches as f64;
    (matches_f / s1_len as f64
        + matches_f / s2_len as f64
        + (matches_f - transpositions as f64 / 2.0) / matches_f)
        / 3.0
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_add_alias() {
        let mut registry = AliasRegistry::new();
        registry.add_alias("rev", "revenue");

        assert_eq!(registry.normalize("rev"), Some("revenue".to_string()));
        assert_eq!(registry.normalize("Rev"), Some("revenue".to_string()));
        assert_eq!(registry.normalize("REV"), Some("revenue".to_string()));
    }

    #[test]
    fn test_add_aliases() {
        let mut registry = AliasRegistry::new();
        registry.add_aliases("revenue", vec!["rev".to_string(), "sales".to_string()]);

        assert_eq!(registry.normalize("rev"), Some("revenue".to_string()));
        assert_eq!(registry.normalize("sales"), Some("revenue".to_string()));
    }

    #[test]
    fn test_normalize_not_found() {
        let registry = AliasRegistry::new();
        assert_eq!(registry.normalize("nonexistent"), None);
    }

    #[test]
    fn test_normalize_fuzzy() {
        let mut registry = AliasRegistry::new();
        registry.add_alias("revenue", "revenue");

        let available: IndexSet<String> = ["revenue", "cogs", "gross_profit"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        // Exact match through alias
        assert_eq!(
            registry.normalize_fuzzy("revenue", &available),
            Some("revenue".to_string())
        );

        // Fuzzy match (typo)
        assert_eq!(
            registry.normalize_fuzzy("revenu", &available),
            Some("revenue".to_string())
        );

        assert_eq!(
            registry.normalize_fuzzy("reveneu", &available),
            Some("revenue".to_string())
        );
    }

    #[test]
    fn test_standard_aliases() {
        let mut registry = AliasRegistry::new();
        registry.load_standard_aliases();

        // Revenue
        assert_eq!(registry.normalize("rev"), Some("revenue".to_string()));
        assert_eq!(registry.normalize("sales"), Some("revenue".to_string()));
        assert_eq!(registry.normalize("turnover"), Some("revenue".to_string()));

        // COGS
        assert_eq!(registry.normalize("cos"), Some("cogs".to_string()));
        assert_eq!(
            registry.normalize("cost_of_sales"),
            Some("cogs".to_string())
        );

        // Operating expenses
        assert_eq!(
            registry.normalize("opex"),
            Some("operating_expenses".to_string())
        );

        // Net income
        assert_eq!(registry.normalize("ni"), Some("net_income".to_string()));
        assert_eq!(
            registry.normalize("bottom_line"),
            Some("net_income".to_string())
        );
    }

    #[test]
    fn test_normalize_string() {
        assert_eq!(normalize_string("Revenue"), "revenue");
        assert_eq!(normalize_string("Cost_of_Sales"), "costofsales");
        assert_eq!(normalize_string("SG&A"), "sga");
        assert_eq!(normalize_string("  spaces  "), "spaces");
    }

    #[test]
    fn test_jaro_winkler() {
        // Exact match
        assert_eq!(jaro_winkler("revenue", "revenue"), 1.0);

        // High similarity
        assert!(jaro_winkler("revenue", "revenu") > 0.95);

        // Low similarity
        assert!(jaro_winkler("revenue", "xyz") < 0.5);

        // Prefix bonus
        let score_prefix = jaro_winkler("revenue", "revenu");
        let score_no_prefix = jaro_winkler("revenue", "evenue");
        assert!(score_prefix > score_no_prefix);
    }

    #[test]
    fn test_fuzzy_match() {
        let candidates: IndexSet<String> = ["revenue", "cogs", "gross_profit"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        // Close match
        assert_eq!(
            fuzzy_match("revenu", &candidates, 0.85),
            Some("revenue".to_string())
        );

        // No match if below threshold
        assert_eq!(fuzzy_match("xyz", &candidates, 0.85), None);
    }

    #[test]
    fn test_case_insensitive() {
        let mut registry = AliasRegistry::new();
        registry.add_alias("Rev", "revenue");

        assert_eq!(registry.normalize("rev"), Some("revenue".to_string()));
        assert_eq!(registry.normalize("REV"), Some("revenue".to_string()));
        assert_eq!(registry.normalize("Rev"), Some("revenue".to_string()));
    }
}
