//! Embedded JSON loader for built-in stress templates.

use super::json::JsonTemplateDocument;
use crate::{Error, Result};

const EMBEDDED_TEMPLATE_JSONS: [(&str, &str); 5] = [
    (
        "gfc_2008",
        include_str!("../../data/templates/gfc_2008.json"),
    ),
    (
        "covid_2020",
        include_str!("../../data/templates/covid_2020.json"),
    ),
    (
        "rate_shock_2022",
        include_str!("../../data/templates/rate_shock_2022.json"),
    ),
    (
        "svb_2023",
        include_str!("../../data/templates/svb_2023.json"),
    ),
    (
        "ltcm_1998",
        include_str!("../../data/templates/ltcm_1998.json"),
    ),
];

/// Return the embedded built-in template JSON payloads.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn embedded_template_jsons() -> &'static [(&'static str, &'static str)] {
    &EMBEDDED_TEMPLATE_JSONS
}

/// Load and validate all embedded built-in template documents.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn load_embedded_documents() -> Result<Vec<JsonTemplateDocument>> {
    embedded_template_jsons()
        .iter()
        .map(|(name, json)| parse_embedded_document(name, json))
        .collect()
}

/// Load and validate a single embedded built-in template document by name.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn load_embedded_document(name: &str) -> Result<JsonTemplateDocument> {
    let (_, json) = embedded_template_jsons()
        .iter()
        .find(|(candidate_name, _)| *candidate_name == name)
        .ok_or_else(|| Error::validation(format!("unknown embedded template '{name}'")))?;

    parse_embedded_document(name, json)
}

fn parse_embedded_document(name: &str, json: &str) -> Result<JsonTemplateDocument> {
    let document: JsonTemplateDocument = serde_json::from_str(json).map_err(|error| {
        Error::validation(format!(
            "failed to parse embedded template '{name}': {error}"
        ))
    })?;

    if document.metadata.id != name {
        return Err(Error::validation(format!(
            "embedded template lookup name '{name}' must match metadata.id '{}'",
            document.metadata.id
        )));
    }

    document.validate().map_err(|error| match error {
        Error::Validation(message) => Error::validation(format!(
            "embedded template '{name}' failed validation: {message}"
        )),
        other => other,
    })?;

    Ok(document)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use super::{
        embedded_template_jsons, load_embedded_document, load_embedded_documents,
        parse_embedded_document,
    };
    use crate::OperationSpec;
    use finstack_core::currency::Currency;

    fn composite_component_ids(document: &super::JsonTemplateDocument) -> Vec<String> {
        serde_json::to_value(&document.composite).expect("serialize composite")["component_ids"]
            .as_array()
            .expect("component_ids should be an array")
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("component_id should be a string")
                    .to_string()
            })
            .collect()
    }

    fn assert_component_order(document: &super::JsonTemplateDocument, component_ids: &[&str]) {
        assert_eq!(
            document.metadata.components,
            component_ids
                .iter()
                .map(|component_id| (*component_id).to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            composite_component_ids(document),
            document.metadata.components
        );
        let loaded_component_ids: Vec<_> = document.components.keys().cloned().collect();
        assert_eq!(loaded_component_ids, document.metadata.components);
    }

    #[test]
    fn loads_all_embedded_documents() {
        let documents = load_embedded_documents().expect("embedded documents should load");

        assert_eq!(documents.len(), embedded_template_jsons().len());
        assert_eq!(documents.len(), 5);
        assert_eq!(
            documents
                .iter()
                .map(|document| document.metadata.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "gfc_2008",
                "covid_2020",
                "rate_shock_2022",
                "svb_2023",
                "ltcm_1998",
            ]
        );
    }

    #[test]
    fn loaded_documents_validate_successfully() {
        let documents = load_embedded_documents().expect("embedded documents should load");

        for document in documents {
            document
                .validate()
                .expect("embedded document should validate");
        }
    }

    #[test]
    fn can_load_named_builtins() {
        let gfc = load_embedded_document("gfc_2008").expect("gfc document should load");
        let covid = load_embedded_document("covid_2020").expect("covid document should load");
        let rate_shock =
            load_embedded_document("rate_shock_2022").expect("rate shock document should load");
        let svb = load_embedded_document("svb_2023").expect("svb document should load");
        let ltcm = load_embedded_document("ltcm_1998").expect("ltcm document should load");

        assert_eq!(gfc.metadata.id, "gfc_2008");
        assert_eq!(covid.metadata.id, "covid_2020");
        assert_eq!(rate_shock.metadata.id, "rate_shock_2022");
        assert_eq!(svb.metadata.id, "svb_2023");
        assert_eq!(ltcm.metadata.id, "ltcm_1998");
    }

    #[test]
    fn missing_embedded_name_errors_cleanly() {
        let error = load_embedded_document("does_not_exist").expect_err("missing name should fail");

        assert!(matches!(error, crate::Error::Validation(_)));
        assert!(error
            .to_string()
            .contains("unknown embedded template 'does_not_exist'"));
    }

    #[test]
    fn rejects_embedded_document_when_lookup_name_does_not_match_metadata_id() {
        let raw_json = embedded_template_jsons()
            .iter()
            .find_map(|(name, json)| (*name == "gfc_2008").then_some(*json))
            .expect("gfc fixture should exist");
        let mut value: serde_json::Value = serde_json::from_str(raw_json).expect("parse fixture");

        value["metadata"]["id"] = serde_json::Value::String("other_template".into());

        let error = parse_embedded_document(
            "gfc_2008",
            &serde_json::to_string(&value).expect("serialize modified fixture"),
        )
        .expect_err("mismatched lookup name should fail");

        assert!(matches!(error, crate::Error::Validation(_)));
        assert!(error.to_string().contains(
            "embedded template lookup name 'gfc_2008' must match metadata.id 'other_template'"
        ));
    }

    #[test]
    fn parsed_content_matches_expected_ids_and_component_order() {
        let gfc = load_embedded_document("gfc_2008").expect("gfc document should load");
        let covid = load_embedded_document("covid_2020").expect("covid document should load");
        let rate_shock =
            load_embedded_document("rate_shock_2022").expect("rate shock document should load");
        let svb = load_embedded_document("svb_2023").expect("svb document should load");
        let ltcm = load_embedded_document("ltcm_1998").expect("ltcm document should load");

        assert_component_order(
            &gfc,
            &[
                "gfc_2008_rates",
                "gfc_2008_credit",
                "gfc_2008_equity",
                "gfc_2008_vol",
                "gfc_2008_fx",
            ],
        );
        assert_component_order(
            &covid,
            &[
                "covid_2020_rates",
                "covid_2020_credit",
                "covid_2020_equity",
                "covid_2020_vol",
                "covid_2020_fx",
            ],
        );
        assert_component_order(
            &rate_shock,
            &[
                "rate_shock_2022_rates",
                "rate_shock_2022_credit",
                "rate_shock_2022_equity",
                "rate_shock_2022_vol",
                "rate_shock_2022_fx",
            ],
        );
        assert_component_order(
            &svb,
            &[
                "svb_2023_rates",
                "svb_2023_credit",
                "svb_2023_equity",
                "svb_2023_vol",
                "svb_2023_fx",
            ],
        );
        assert_component_order(
            &ltcm,
            &[
                "ltcm_1998_rates",
                "ltcm_1998_credit",
                "ltcm_1998_equity",
                "ltcm_1998_vol",
                "ltcm_1998_fx",
            ],
        );
    }

    #[test]
    fn new_builtins_have_expected_focused_content() {
        let rate_shock =
            load_embedded_document("rate_shock_2022").expect("rate shock document should load");
        let svb = load_embedded_document("svb_2023").expect("svb document should load");
        let ltcm = load_embedded_document("ltcm_1998").expect("ltcm document should load");

        let rate_credit = rate_shock
            .components
            .get("rate_shock_2022_credit")
            .expect("rate shock credit component should exist");
        assert_eq!(rate_credit.operations.len(), 2);

        let svb_credit = svb
            .components
            .get("svb_2023_credit")
            .expect("svb credit component should exist");
        assert!(svb_credit.operations.iter().any(|operation| {
            matches!(
                operation,
                OperationSpec::InstrumentSpreadBpByAttr { attrs, bp }
                    if attrs.get("sector").map(String::as_str) == Some("regional_banks")
                        && (*bp - 150.0).abs() < f64::EPSILON
            )
        }));

        let ltcm_fx = ltcm
            .components
            .get("ltcm_1998_fx")
            .expect("ltcm fx component should exist");
        assert!(ltcm_fx.operations.iter().any(|operation| {
            matches!(
                operation,
                OperationSpec::MarketFxPct { base, quote, pct }
                    if *base == Currency::BRL
                        && *quote == Currency::USD
                        && (*pct - -30.0).abs() < f64::EPSILON
            )
        }));
        assert!(ltcm_fx.operations.iter().any(|operation| {
            matches!(
                operation,
                OperationSpec::MarketFxPct { base, quote, pct }
                    if *base == Currency::RUB
                        && *quote == Currency::USD
                        && (*pct - -50.0).abs() < f64::EPSILON
            )
        }));
    }
}
