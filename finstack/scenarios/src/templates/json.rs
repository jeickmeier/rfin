//! Serde-only JSON document types for stress templates.

use crate::{Error, Result, ScenarioSpec, TemplateMetadata};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

/// Serde-facing JSON document for a composable stress template.
///
/// Phase 1 stages the document layer before the loader/runtime integration lands,
/// so this internal type is only exercised by unit tests for now.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JsonTemplateDocument {
    /// Template metadata shared with the runtime registry layer.
    pub(crate) metadata: TemplateMetadata,
    /// Component scenarios keyed by deterministic component identifier.
    pub(crate) components: IndexMap<String, ScenarioSpec>,
    /// Composite scenario identity and ordered component references.
    pub(crate) composite: JsonCompositeTemplate,
}

/// Phase 1 stages document validation ahead of runtime usage.
#[cfg_attr(not(test), allow(dead_code))]
impl JsonTemplateDocument {
    /// Validate document-level consistency across metadata, components, and composite.
    pub(crate) fn validate(&self) -> Result<()> {
        self.composite.validate()?;

        if self.metadata.id != self.composite.id {
            return Err(Error::validation("metadata.id must match composite.id"));
        }

        if self.metadata.components != self.composite.component_ids {
            return Err(Error::validation(
                "metadata.components must match component IDs".to_string(),
            ));
        }

        let composite_component_ids: IndexSet<&str> = self
            .composite
            .component_ids
            .iter()
            .map(String::as_str)
            .collect();

        for component_id in &self.composite.component_ids {
            if !self.components.contains_key(component_id) {
                return Err(Error::validation(format!(
                    "composite references missing component '{component_id}'"
                )));
            }
        }

        for (component_key, spec) in &self.components {
            if component_key != &spec.id {
                return Err(Error::validation(format!(
                    "component key '{component_key}' must match ScenarioSpec.id '{}'",
                    spec.id
                )));
            }

            if !composite_component_ids.contains(component_key.as_str()) {
                return Err(Error::validation(format!(
                    "component '{component_key}' is not referenced by composite.component_ids"
                )));
            }

            spec.validate().map_err(|error| {
                Error::validation(format!(
                    "component '{component_key}' failed validation: {error}"
                ))
            })?;
        }

        if self.components.len() != self.composite.component_ids.len() {
            return Err(Error::validation(
                "component map must match composite.component_ids membership".to_string(),
            ));
        }

        Ok(())
    }
}

/// Composite scenario identity plus ordered component references.
///
/// Phase 1 stages the serde/document shape before runtime loading is wired up.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JsonCompositeTemplate {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    priority: i32,
    component_ids: Vec<String>,
}

/// Phase 1 stages document validation ahead of runtime usage.
#[cfg_attr(not(test), allow(dead_code))]
impl JsonCompositeTemplate {
    /// Create a composite template from top-level identity fields and ordered references.
    pub(crate) fn new(
        id: impl Into<String>,
        name: Option<&str>,
        description: Option<&str>,
        priority: i32,
        component_ids: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.map(str::to_string),
            description: description.map(str::to_string),
            priority,
            component_ids,
        }
    }

    /// Validate the composite identity and ordered component references.
    pub(crate) fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(Error::validation("Scenario ID cannot be empty"));
        }

        if self.component_ids.is_empty() {
            return Err(Error::validation("component_ids cannot be empty"));
        }

        let mut seen = IndexSet::new();
        for component_id in &self.component_ids {
            if component_id.trim().is_empty() {
                return Err(Error::validation("component_ids cannot contain empty IDs"));
            }
            if !seen.insert(component_id) {
                return Err(Error::validation(format!(
                    "duplicate component_id '{component_id}'"
                )));
            }
        }

        Ok(())
    }

    /// Return the composite identifier.
    #[must_use]
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    /// Return the optional composite display name.
    #[must_use]
    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Return the optional composite description.
    #[must_use]
    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Return the composite priority.
    #[must_use]
    pub(crate) fn priority(&self) -> i32 {
        self.priority
    }

    /// Return the ordered component identifiers referenced by the composite.
    #[must_use]
    pub(crate) fn component_ids(&self) -> &[String] {
        &self.component_ids
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use super::{JsonCompositeTemplate, JsonTemplateDocument};
    use crate::{CurveKind, OperationSpec, ScenarioSpec, TemplateMetadata};
    use indexmap::{indexmap, IndexMap};
    use time::macros::date;

    fn component_spec(id: &str) -> ScenarioSpec {
        ScenarioSpec {
            id: id.to_string(),
            name: Some(format!("{id} component")),
            description: Some(format!("{id} description")),
            operations: vec![OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: format!("{id}_curve"),
                discount_curve_id: None,
                bp: 25.0,
            }],
            priority: 0,
            resolution_mode: Default::default(),
        }
    }

    fn metadata(components: Vec<&str>) -> TemplateMetadata {
        TemplateMetadata {
            id: "gfc_2008".into(),
            name: "Global Financial Crisis 2008".into(),
            description: "Lehman collapse scenario".into(),
            event_date: date!(2008 - 09 - 15),
            asset_classes: vec![crate::AssetClass::Rates, crate::AssetClass::Credit],
            tags: vec!["systemic".into()],
            severity: crate::Severity::Severe,
            components: components.into_iter().map(str::to_string).collect(),
        }
    }

    fn composite(component_ids: Vec<&str>) -> JsonCompositeTemplate {
        JsonCompositeTemplate::new(
            "gfc_2008",
            Some("Global Financial Crisis 2008"),
            Some("Composite scenario"),
            10,
            component_ids.into_iter().map(str::to_string).collect(),
        )
    }

    fn valid_document() -> JsonTemplateDocument {
        JsonTemplateDocument {
            metadata: metadata(vec!["rates", "credit"]),
            components: indexmap! {
                "rates".into() => component_spec("rates"),
                "credit".into() => component_spec("credit"),
            },
            composite: composite(vec!["rates", "credit"]),
        }
    }

    #[test]
    fn valid_document_passes_validation() {
        valid_document()
            .validate()
            .expect("document should validate");
    }

    #[test]
    fn rejects_mismatched_metadata_and_composite_id() {
        let mut document = valid_document();
        document.composite.id = "other_id".into();

        let error = document.validate().expect_err("validation should fail");

        assert!(matches!(error, crate::Error::Validation(_)));
        assert!(error
            .to_string()
            .contains("metadata.id must match composite.id"));
    }

    #[test]
    fn rejects_missing_referenced_component() {
        let document = JsonTemplateDocument {
            metadata: metadata(vec!["rates", "credit"]),
            components: indexmap! {
                "rates".into() => component_spec("rates"),
            },
            composite: composite(vec!["rates", "credit"]),
        };

        let error = document.validate().expect_err("validation should fail");

        assert!(matches!(error, crate::Error::Validation(_)));
        assert!(error.to_string().contains("missing component 'credit'"));
    }

    #[test]
    fn rejects_mismatched_metadata_components() {
        let document = JsonTemplateDocument {
            metadata: metadata(vec!["rates", "vol"]),
            components: indexmap! {
                "rates".into() => component_spec("rates"),
                "credit".into() => component_spec("credit"),
            },
            composite: composite(vec!["rates", "credit"]),
        };

        let error = document.validate().expect_err("validation should fail");

        assert!(matches!(error, crate::Error::Validation(_)));
        assert!(error
            .to_string()
            .contains("metadata.components must match component IDs"));
    }

    #[test]
    fn rejects_empty_composite_component_ids() {
        let document = JsonTemplateDocument {
            metadata: metadata(vec![]),
            components: IndexMap::new(),
            composite: composite(vec![]),
        };

        let error = document.validate().expect_err("validation should fail");

        assert!(matches!(error, crate::Error::Validation(_)));
        assert!(error.to_string().contains("component_ids cannot be empty"));
    }

    #[test]
    fn rejects_duplicate_component_ids() {
        let document = JsonTemplateDocument {
            metadata: metadata(vec!["rates", "rates"]),
            components: indexmap! {
                "rates".into() => component_spec("rates"),
            },
            composite: composite(vec!["rates", "rates"]),
        };

        let error = document.validate().expect_err("validation should fail");

        assert!(matches!(error, crate::Error::Validation(_)));
        assert!(error.to_string().contains("duplicate component_id 'rates'"));
    }

    #[test]
    fn validates_component_specs() {
        let document = JsonTemplateDocument {
            metadata: metadata(vec!["rates"]),
            components: indexmap! {
                "rates".into() => ScenarioSpec {
                    id: "rates".into(),
                    name: Some("Broken component".into()),
                    description: None,
                    operations: vec![OperationSpec::CurveParallelBp {
                        curve_kind: CurveKind::Discount,
                        curve_id: String::new(),
                        discount_curve_id: None,
                        bp: 25.0,
                    }],
                    priority: 0,
                    resolution_mode: Default::default(),
                },
            },
            composite: composite(vec!["rates"]),
        };

        let error = document.validate().expect_err("validation should fail");

        assert!(matches!(error, crate::Error::Validation(_)));
        assert!(error
            .to_string()
            .contains("component 'rates' failed validation"));
    }

    #[test]
    fn rejects_component_key_spec_id_mismatch() {
        let document = JsonTemplateDocument {
            metadata: metadata(vec!["rates"]),
            components: indexmap! {
                "rates".into() => component_spec("other_rates"),
            },
            composite: composite(vec!["rates"]),
        };

        let error = document.validate().expect_err("validation should fail");

        assert!(matches!(error, crate::Error::Validation(_)));
        assert!(error
            .to_string()
            .contains("component key 'rates' must match ScenarioSpec.id 'other_rates'"));
    }

    #[test]
    fn component_membership_validation_is_independent_of_map_insertion_order() {
        let document = JsonTemplateDocument {
            metadata: metadata(vec!["rates", "credit"]),
            components: indexmap! {
                "credit".into() => component_spec("credit"),
                "rates".into() => component_spec("rates"),
            },
            composite: composite(vec!["rates", "credit"]),
        };

        document
            .validate()
            .expect("validation should use composite.component_ids ordering");
    }

    #[test]
    fn serde_roundtrip_for_document_shape() {
        let document = valid_document();

        let json = serde_json::to_string(&document).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse serialized json");
        assert_eq!(value["metadata"]["id"], "gfc_2008");
        assert_eq!(value["components"]["rates"]["id"], "rates");
        assert_eq!(value["composite"]["id"], "gfc_2008");
        assert_eq!(
            value["composite"]["component_ids"],
            serde_json::json!(["rates", "credit"])
        );
        assert!(value["composite"].get("operations").is_none());

        let roundtrip: JsonTemplateDocument = serde_json::from_str(&json).expect("deserialize");
        roundtrip.validate().expect("roundtrip should validate");

        let component_ids: Vec<_> = roundtrip.components.keys().cloned().collect();
        assert_eq!(
            component_ids,
            vec!["rates".to_string(), "credit".to_string()]
        );
    }
}
