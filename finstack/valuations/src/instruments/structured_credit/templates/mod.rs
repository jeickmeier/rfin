//! Pre-built waterfall templates for common structured credit deal types.
//!
//! This module provides production-ready waterfall configurations for:
//! - CLO 2.0 structures
//! - CMBS standard waterfalls
//! - CRE operating company distributions
//!
//! Templates can be used as-is or customized for specific deals.

pub mod clo;
pub mod cmbs;
pub mod cre;

use finstack_core::currency::Currency;

// Re-export template builders
pub use clo::clo_2_0_template;
pub use cmbs::cmbs_standard_template;
pub use cre::cre_operating_company_template;

/// Template metadata
pub struct WaterfallTemplate {
    /// Template name
    pub name: String,
    /// Description
    pub description: String,
    /// Deal type
    pub deal_type: super::components::DealType,
}

impl WaterfallTemplate {
    /// Create a new template metadata
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        deal_type: super::components::DealType,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            deal_type,
        }
    }
}

/// Get all available templates
pub fn available_templates() -> Vec<WaterfallTemplate> {
    vec![
        WaterfallTemplate::new(
            "clo_2.0",
            "Standard CLO 2.0 waterfall with OC/IC tests and diversion",
            super::components::DealType::CLO,
        ),
        WaterfallTemplate::new(
            "cmbs_standard",
            "Standard CMBS waterfall with lockout and sequential pay",
            super::components::DealType::CMBS,
        ),
        WaterfallTemplate::new(
            "cre_operating",
            "CRE operating company cash distribution waterfall",
            super::components::DealType::CMBS,
        ),
    ]
}

/// Get a template by name
pub fn get_template(name: &str, currency: Currency) -> Option<super::components::WaterfallEngine> {
    match name {
        "clo_2.0" => Some(clo_2_0_template(currency)),
        "cmbs_standard" => Some(cmbs_standard_template(currency)),
        "cre_operating" => Some(cre_operating_company_template(currency)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn test_available_templates() {
        let templates = available_templates();
        assert_eq!(templates.len(), 3);
        assert!(templates.iter().any(|t| t.name == "clo_2.0"));
        assert!(templates.iter().any(|t| t.name == "cmbs_standard"));
        assert!(templates.iter().any(|t| t.name == "cre_operating"));
    }

    #[test]
    fn test_get_template() {
        let template = get_template("clo_2.0", Currency::USD);
        assert!(template.is_some());

        let nonexistent = get_template("nonexistent", Currency::USD);
        assert!(nonexistent.is_none());
    }
}

