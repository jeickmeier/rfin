//! JSON-Schema helpers for Finstack types.
//!
//! Schemas are generated from the crate's serde-friendly types and checked in
//! under `schemas/`. These helpers expose them as `serde_json::Value` for use
//! in validation, UI forms, and contract generation.
//!
//! # Error Handling
//!
//! All schema accessors return `Result<&'static Value>` instead of panicking,
//! allowing callers to handle schema loading failures gracefully.

use serde_json::Value;
use std::sync::OnceLock;

/// Parse embedded JSON schema at compile time, returning a Result.
/// The JSON is embedded via `include_str!` so the content is always present,
/// but parsing can still fail if the JSON is malformed.
macro_rules! try_include_schema {
    ($path:literal) => {
        serde_json::from_str::<Value>(include_str!($path))
            .map_err(|e| format!("invalid schema JSON at {}: {}", $path, e))
    };
}

macro_rules! with_instrument_schema_registry {
    ($callback:ident $(, $extra:expr )* $(,)?) => {
        $callback!(
            [$($extra),*]
            "agency_cmo" => "../schemas/instruments/1/fixed_income/agency_cmo.schema.json";
            "agency_mbs_passthrough" => "../schemas/instruments/1/fixed_income/agency_mbs_passthrough.schema.json";
            "agency_tba" => "../schemas/instruments/1/fixed_income/agency_tba.schema.json";
            "asian_option" => "../schemas/instruments/1/exotics/asian_option.schema.json";
            "autocallable" => "../schemas/instruments/1/equity/autocallable.schema.json";
            "barrier_option" => "../schemas/instruments/1/exotics/barrier_option.schema.json";
            "basis_swap" => "../schemas/instruments/1/rates/basis_swap.schema.json";
            "basket" => "../schemas/instruments/1/exotics/basket.schema.json";
            "bond" => "../schemas/instruments/1/fixed_income/bond.schema.json";
            "bond_future" => "../schemas/instruments/1/fixed_income/bond_future.schema.json";
            "cds_index" => "../schemas/instruments/1/credit_derivatives/cds_index.schema.json";
            "cds_option" => "../schemas/instruments/1/credit_derivatives/cds_option.schema.json";
            "cds_tranche" => "../schemas/instruments/1/credit_derivatives/cds_tranche.schema.json";
            "cliquet_option" => "../schemas/instruments/1/equity/cliquet_option.schema.json";
            "cms_option" => "../schemas/instruments/1/rates/cms_option.schema.json";
            "commodity_forward" => "../schemas/instruments/1/commodity/commodity_forward.schema.json";
            "commodity_option" => "../schemas/instruments/1/commodity/commodity_option.schema.json";
            "commodity_swap" => "../schemas/instruments/1/commodity/commodity_swap.schema.json";
            "convertible_bond" => "../schemas/instruments/1/fixed_income/convertible_bond.schema.json";
            "credit_default_swap" => "../schemas/instruments/1/credit_derivatives/credit_default_swap.schema.json";
            "deposit" => "../schemas/instruments/1/rates/deposit.schema.json";
            "dollar_roll" => "../schemas/instruments/1/fixed_income/dollar_roll.schema.json";
            "equity" => "../schemas/instruments/1/equity/equity.schema.json";
            "equity_index_future" => "../schemas/instruments/1/equity/equity_index_future.schema.json";
            "equity_option" => "../schemas/instruments/1/equity/equity_option.schema.json";
            "forward_rate_agreement" => "../schemas/instruments/1/rates/forward_rate_agreement.schema.json";
            "fx_barrier_option" => "../schemas/instruments/1/fx/fx_barrier_option.schema.json";
            "fx_forward" => "../schemas/instruments/1/fx/fx_forward.schema.json";
            "fx_option" => "../schemas/instruments/1/fx/fx_option.schema.json";
            "fx_spot" => "../schemas/instruments/1/fx/fx_spot.schema.json";
            "fx_swap" => "../schemas/instruments/1/fx/fx_swap.schema.json";
            "fx_variance_swap" => "../schemas/instruments/1/fx/fx_variance_swap.schema.json";
            "inflation_cap_floor" => "../schemas/instruments/1/rates/inflation_cap_floor.schema.json";
            "inflation_linked_bond" => "../schemas/instruments/1/fixed_income/inflation_linked_bond.schema.json";
            "inflation_swap" => "../schemas/instruments/1/rates/inflation_swap.schema.json";
            "interest_rate_future" => "../schemas/instruments/1/rates/interest_rate_future.schema.json";
            "interest_rate_option" => "../schemas/instruments/1/rates/interest_rate_option.schema.json";
            "interest_rate_swap" => "../schemas/instruments/1/rates/interest_rate_swap.schema.json";
            "lookback_option" => "../schemas/instruments/1/exotics/lookback_option.schema.json";
            "ndf" => "../schemas/instruments/1/fx/ndf.schema.json";
            "private_markets_fund" => "../schemas/instruments/1/equity/private_markets_fund.schema.json";
            "quanto_option" => "../schemas/instruments/1/fx/quanto_option.schema.json";
            "range_accrual" => "../schemas/instruments/1/rates/range_accrual.schema.json";
            "real_estate_asset" => "../schemas/instruments/1/equity/real_estate_asset.schema.json";
            "repo" => "../schemas/instruments/1/rates/repo.schema.json";
            "revolving_credit" => "../schemas/instruments/1/fixed_income/revolving_credit.schema.json";
            "structured_credit" => "../schemas/instruments/1/fixed_income/structured_credit.schema.json";
            "swaption" => "../schemas/instruments/1/rates/swaption.schema.json";
            "term_loan" => "../schemas/instruments/1/fixed_income/term_loan.schema.json";
            "trs_equity" => "../schemas/instruments/1/equity/trs_equity.schema.json";
            "trs_fixed_income_index" => "../schemas/instruments/1/fixed_income/trs_fixed_income_index.schema.json";
            "variance_swap" => "../schemas/instruments/1/equity/variance_swap.schema.json";
            "volatility_index_future" => "../schemas/instruments/1/equity/volatility_index_future.schema.json";
            "volatility_index_option" => "../schemas/instruments/1/equity/volatility_index_option.schema.json";
            "xccy_swap" => "../schemas/instruments/1/rates/xccy_swap.schema.json";
            "yoy_inflation_swap" => "../schemas/instruments/1/rates/yoy_inflation_swap.schema.json";
        )
    };
}

macro_rules! build_schema_cache {
    (
        []
        $($name:literal => $path:literal;)*
    ) => {{
        let mut cache = std::collections::BTreeMap::new();
        $(
            cache.insert($name, try_include_schema!($path));
        )*
        cache
    }};
}

/// Get JSON-Schema for Bond configuration.
///
/// Sourced from the generated instrument schemas under `schemas/instruments/1/`.
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed.
#[allow(dead_code)] // Public API, used in tests
pub fn bond_schema() -> finstack_core::Result<&'static Value> {
    static SCHEMA: OnceLock<Result<Value, String>> = OnceLock::new();
    SCHEMA
        .get_or_init(|| {
            try_include_schema!("../schemas/instruments/1/fixed_income/bond.schema.json")
        })
        .as_ref()
        .map_err(|e| finstack_core::Error::Validation(e.clone()))
}

/// Get the JSON Schema for the instrument envelope.
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed.
pub fn instrument_envelope_schema() -> finstack_core::Result<&'static Value> {
    static SCHEMA: OnceLock<Result<Value, String>> = OnceLock::new();
    SCHEMA
        .get_or_init(|| try_include_schema!("../schemas/instruments/1/instrument.schema.json"))
        .as_ref()
        .map_err(|e| finstack_core::Error::Validation(e.clone()))
}

fn instrument_schema_cache(
) -> &'static std::collections::BTreeMap<&'static str, Result<Value, String>> {
    static CACHE: OnceLock<std::collections::BTreeMap<&'static str, Result<Value, String>>> =
        OnceLock::new();
    CACHE.get_or_init(|| with_instrument_schema_registry!(build_schema_cache))
}

fn fallback_instrument_schema(instrument_type: &str) -> Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": format!("{instrument_type} (generic)"),
        "description": format!(
            "Fallback schema for instrument type '{instrument_type}'. Dedicated schema is not yet available; 'spec' remains untyped."
        ),
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "schema": {
                "const": "finstack.instrument/1",
                "type": "string"
            },
            "instrument": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "type": {
                        "const": instrument_type,
                        "type": "string"
                    },
                    "spec": {
                        "type": "object",
                        "description": "Dedicated schema unavailable; accepts any object payload."
                    }
                },
                "required": ["type", "spec"]
            }
        },
        "required": ["schema", "instrument"]
    })
}

/// Return the canonical instrument discriminators advertised by the envelope schema.
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed.
pub fn instrument_types() -> finstack_core::Result<Vec<String>> {
    let schema = instrument_envelope_schema()?;
    let Some(values) = schema
        .pointer("/properties/instrument/properties/type/enum")
        .and_then(serde_json::Value::as_array)
    else {
        return Err(finstack_core::Error::Validation(
            "instrument schema enum is missing".to_string(),
        ));
    };

    values
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                finstack_core::Error::Validation(
                    "instrument schema enum contains a non-string value".to_string(),
                )
            })
        })
        .collect()
}

/// Get the JSON Schema for a single instrument type.
///
/// Returns a dedicated schema when available. For supported instrument tags
/// without a dedicated checked-in schema file, returns a fallback tagged schema
/// with an untyped `spec` payload.
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed or the
/// requested instrument type is not supported.
pub fn instrument_schema(instrument_type: &str) -> finstack_core::Result<Value> {
    if let Some(schema) = instrument_schema_cache().get(instrument_type) {
        return schema
            .as_ref()
            .cloned()
            .map_err(|e| finstack_core::Error::Validation(e.clone()));
    }

    if instrument_types()?.iter().any(|ty| ty == instrument_type) {
        return Ok(fallback_instrument_schema(instrument_type));
    }

    Err(finstack_core::Error::Validation(format!(
        "unknown instrument type '{instrument_type}'"
    )))
}

/// Get JSON-Schema for ValuationResult.
///
/// Returns schema for valuation result envelope (PV + metrics).
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed.
#[allow(dead_code)] // Public API, used in tests
pub fn valuation_result_schema() -> finstack_core::Result<&'static Value> {
    static SCHEMA: OnceLock<Result<Value, String>> = OnceLock::new();
    SCHEMA
        .get_or_init(|| try_include_schema!("../schemas/results/1/valuation_result.schema.json"))
        .as_ref()
        .map_err(|e| finstack_core::Error::Validation(e.clone()))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_stubs() {
        // Verify stub schemas are valid JSON and have expected structure
        let bond = bond_schema().expect("bond schema should parse");
        assert_eq!(bond["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(bond["title"], "Bond");

        let envelope =
            instrument_envelope_schema().expect("instrument envelope schema should parse");
        assert_eq!(envelope["title"], "Finstack Instrument");

        let result = valuation_result_schema().expect("valuation result schema should parse");
        assert_eq!(result["title"], "ValuationResult");
    }

    #[test]
    fn test_all_schemas_parse_successfully() {
        // Ensure all embedded schemas parse without error.
        // This test catches invalid JSON at CI time rather than runtime.
        assert!(bond_schema().is_ok(), "bond_schema() should return Ok");
        assert!(
            instrument_envelope_schema().is_ok(),
            "instrument_envelope_schema() should return Ok"
        );
        assert!(
            valuation_result_schema().is_ok(),
            "valuation_result_schema() should return Ok"
        );
    }

    #[test]
    fn test_instrument_types_lists_supported_tags() {
        let types = instrument_types().expect("instrument types should parse");
        assert!(types.iter().any(|ty| ty == "bond"));
        assert!(types.iter().any(|ty| ty == "cms_swap"));
    }

    #[test]
    fn test_instrument_schema_returns_dedicated_schema_when_available() {
        let schema = instrument_schema("bond").expect("bond schema should load");
        assert_eq!(schema["title"], "Bond");
        assert_eq!(
            schema["$id"],
            "https://finstack.dev/schemas/instrument/1/bond.schema.json"
        );
    }

    #[test]
    fn test_instrument_schema_returns_fallback_for_missing_dedicated_schema() {
        let schema = instrument_schema("cms_swap").expect("cms_swap should return fallback schema");
        assert_eq!(
            schema["properties"]["instrument"]["properties"]["type"]["const"],
            "cms_swap"
        );
        assert!(schema["description"]
            .as_str()
            .expect("fallback schema should include description")
            .contains("Dedicated schema is not yet available"));
    }

    #[test]
    fn test_instrument_schema_rejects_unknown_discriminator() {
        let err = instrument_schema("not_a_supported_instrument_type").expect_err("unknown type");
        let msg = err.to_string();
        assert!(
            msg.contains("unknown instrument type"),
            "unexpected message: {msg}"
        );
    }

    #[test]
    fn test_instrument_schema_cache_covers_registered_aliases() {
        let bond = instrument_schema("bond").expect("bond");
        assert_eq!(bond["title"], "Bond");
        let swap = instrument_schema("interest_rate_swap").expect("irs");
        assert_eq!(swap["title"], "Interest Rate Swap");
    }
}
