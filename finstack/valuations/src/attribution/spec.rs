//! JSON specification and execution framework for attribution.
//!
//! Provides serializable specs for defining complete attribution runs in JSON,
//! with stable schemas and deterministic round-trip serialization.

use super::{
    attribute_pnl_metrics_based, attribute_pnl_parallel, attribute_pnl_taylor_standard,
    attribute_pnl_waterfall, compute_credit_factor_attribution, AttributionMethod,
    CreditAttributionInput, CreditFactorDetailOptions, CreditFactorModelRef, ModelParamsSnapshot,
    PnlAttribution,
};
use crate::factor_model::{decompose_levels, decompose_period};
use crate::instruments::{DynInstrument, InstrumentJson};
use crate::metrics::MetricId;
use finstack_core::{
    config::{FinstackConfig, ResultsMeta},
    currency::Currency,
    dates::Date,
    market_data::context::{MarketContext, MarketContextState},
    Result,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::str::FromStr;

/// Schema version for attribution serialization.
pub const ATTRIBUTION_SCHEMA_V1: &str = "finstack.attribution/1";

/// Top-level envelope for attribution specifications.
///
/// Mirrors the calibration and instrument envelope patterns with schema versioning
/// and strict field validation for long-term JSON stability.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttributionEnvelope {
    /// Schema version identifier (currently "finstack.attribution/1")
    pub schema: String,
    /// The attribution specification
    pub attribution: AttributionSpec,
}

impl AttributionEnvelope {
    /// Create a new attribution envelope with the current schema version.
    pub fn new(attribution: AttributionSpec) -> Self {
        Self {
            schema: ATTRIBUTION_SCHEMA_V1.to_string(),
            attribution,
        }
    }

    /// Execute the attribution and return the result envelope.
    pub fn execute(&self) -> Result<AttributionResultEnvelope> {
        let result = self.attribution.execute()?;
        Ok(AttributionResultEnvelope::new(result))
    }
}

/// Attribution specification for a single P&L attribution run.
///
/// Contains all data needed to perform attribution: instrument, market snapshots,
/// dates, and methodology.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttributionSpec {
    /// Instrument to attribute (as JSON envelope)
    pub instrument: InstrumentJson,
    /// Market context at T₀
    #[schemars(with = "serde_json::Value")]
    pub market_t0: MarketContextState,
    /// Market context at T₁
    #[schemars(with = "serde_json::Value")]
    pub market_t1: MarketContextState,
    /// Valuation date at T₀
    #[schemars(with = "String")]
    pub as_of_t0: Date,
    /// Valuation date at T₁
    #[schemars(with = "String")]
    pub as_of_t1: Date,
    /// Attribution methodology
    pub method: AttributionMethod,
    /// Optional model parameters at T₀ (for attributing parameter changes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_params_t0: Option<ModelParamsSnapshot>,
    /// Optional configuration overrides (defaults to FinstackConfig::default())
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<AttributionConfig>,
    /// Optional calibrated credit factor model. When present (and the
    /// instrument has a recognizable issuer + credit-curve exposure), the
    /// returned `PnlAttribution` carries a `credit_factor_detail` field with
    /// generic / per-level / adder P&L additively decomposing
    /// `credit_curves_pnl`. PR-7 wires metrics-based and Taylor methods.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub credit_factor_model: Option<CreditFactorModelRef>,
    /// Detail/payload options for `credit_factor_detail`. Inert when
    /// `credit_factor_model` is `None`.
    #[serde(default)]
    pub credit_factor_detail_options: CreditFactorDetailOptions,
}

/// Optional configuration for attribution runs.
///
/// Allows overriding default tolerances and metrics for attribution calculations.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttributionConfig {
    /// Absolute tolerance for residual validation (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance_abs: Option<f64>,
    /// Percentage tolerance for residual validation (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance_pct: Option<f64>,
    /// Metrics to compute for metrics-based attribution (optional)
    /// If not provided, a default set will be used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<String>>,
    /// Strict validation mode (if true, errors during attribution will propagate instead of being logged)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict_validation: Option<bool>,
    /// Rounding scale override (number of decimal places)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rounding_scale: Option<u32>,
    /// Rate bump size in basis points for sensitivities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_bump_bp: Option<f64>,
}

impl AttributionSpec {
    /// Build an attribution spec from the JSON-friendly inputs used by bindings.
    pub fn from_json_inputs(
        instrument_json: &str,
        market_t0_json: &str,
        market_t1_json: &str,
        as_of_t0: &str,
        as_of_t1: &str,
        method_json: &str,
        config_json: Option<&str>,
    ) -> Result<Self> {
        Ok(Self {
            instrument: parse_input_json("instrument", instrument_json)?,
            market_t0: parse_input_json("market_t0", market_t0_json)?,
            market_t1: parse_input_json("market_t1", market_t1_json)?,
            as_of_t0: parse_iso_date("as_of_t0", as_of_t0)?,
            as_of_t1: parse_iso_date("as_of_t1", as_of_t1)?,
            method: parse_input_json("method", method_json)?,
            model_params_t0: None,
            config: config_json
                .map(|json| parse_input_json("config", json))
                .transpose()?,
            credit_factor_model: None,
            credit_factor_detail_options: CreditFactorDetailOptions::default(),
        })
    }

    /// Execute the attribution specification.
    ///
    /// Returns a complete result with the P&L attribution and metadata.
    pub fn execute(&self) -> Result<AttributionResult> {
        // Reconstruct instrument from JSON
        let instrument = self.instrument.clone().into_boxed()?;
        let instrument_arc: std::sync::Arc<DynInstrument> = std::sync::Arc::from(instrument);

        // Reconstruct market contexts
        let market_t0 = MarketContext::try_from(self.market_t0.clone())?;
        let market_t1 = MarketContext::try_from(self.market_t1.clone())?;

        // Determine instrument currency for config (avoids hardcoding USD)
        let instrument_ccy = instrument_arc
            .value(&market_t0, self.as_of_t0)
            .ok()
            .map(|m| m.currency());

        // Build config (defaults unless overridden)
        let config = self.build_finstack_config(instrument_ccy);

        // Determine strict validation
        let strict_validation = self
            .config
            .as_ref()
            .and_then(|c| c.strict_validation)
            .unwrap_or(false);

        // Execute attribution based on method
        let mut attribution = match &self.method {
            AttributionMethod::Parallel => attribute_pnl_parallel(
                &instrument_arc,
                &market_t0,
                &market_t1,
                self.as_of_t0,
                self.as_of_t1,
                &config,
                self.model_params_t0.as_ref(),
            )?,

            AttributionMethod::Waterfall(order) => attribute_pnl_waterfall(
                &instrument_arc,
                &market_t0,
                &market_t1,
                self.as_of_t0,
                self.as_of_t1,
                &config,
                order.clone(),
                strict_validation,
                self.model_params_t0.as_ref(),
            )?,

            AttributionMethod::Taylor(ref taylor_config) => attribute_pnl_taylor_standard(
                &instrument_arc,
                &market_t0,
                &market_t1,
                self.as_of_t0,
                self.as_of_t1,
                taylor_config,
            )?,

            AttributionMethod::MetricsBased => {
                // Determine metrics to use
                let metrics = if let Some(ref cfg) = self.config {
                    if let Some(ref metric_names) = cfg.metrics {
                        let mut parsed = Vec::new();
                        let mut unknown = Vec::new();

                        for name in metric_names {
                            match MetricId::from_str(name) {
                                Ok(id) => parsed.push(id),
                                Err(_) => unknown.push(name.clone()),
                            }
                        }

                        if !unknown.is_empty() {
                            return Err(finstack_core::Error::Validation(format!(
                                "Unknown metric names: {}",
                                unknown.join(", ")
                            )));
                        }

                        parsed
                    } else {
                        default_attribution_metrics()
                    }
                } else {
                    default_attribution_metrics()
                };

                // Compute valuations with metrics
                let val_t0 = instrument_arc.price_with_metrics(
                    &market_t0,
                    self.as_of_t0,
                    &metrics,
                    crate::instruments::PricingOptions::default(),
                )?;
                let val_t1 = instrument_arc.price_with_metrics(
                    &market_t1,
                    self.as_of_t1,
                    &metrics,
                    crate::instruments::PricingOptions::default(),
                )?;

                attribute_pnl_metrics_based(
                    &instrument_arc,
                    &market_t0,
                    &market_t1,
                    &val_t0,
                    &val_t1,
                    self.as_of_t0,
                    self.as_of_t1,
                )?
            }
        };

        // Apply tolerance overrides if provided
        if let Some(ref cfg) = self.config {
            if let Some(tol_abs) = cfg.tolerance_abs {
                attribution.meta.tolerance_abs = tol_abs;
            }
            if let Some(tol_pct) = cfg.tolerance_pct {
                attribution.meta.tolerance_pct = tol_pct;
            }
        }

        // Optional: credit-factor hierarchy decomposition of credit_curves_pnl.
        // Wired for MetricsBased and Taylor (PR-7). Other methods leave the
        // field None; they will be wired in PR-8a/b. The existing
        // `credit_curves_pnl` field is unchanged numerically — this is purely
        // additive detail.
        if let Some(model_ref) = &self.credit_factor_model {
            let supported = matches!(
                self.method,
                AttributionMethod::MetricsBased | AttributionMethod::Taylor(_)
            );
            if supported {
                let mut detail_notes: Vec<String> = Vec::new();
                match self.compute_credit_factor_detail(
                    model_ref,
                    &instrument_arc,
                    &market_t0,
                    &market_t1,
                    &attribution,
                    &mut detail_notes,
                ) {
                    Ok(Some(detail)) => attribution.credit_factor_detail = Some(detail),
                    Ok(None) => {
                        if detail_notes.is_empty() {
                            attribution.meta.notes.push(
                                "credit_factor_model supplied but no resolvable issuer/CS01 \
                                 on instrument; credit_factor_detail omitted"
                                    .into(),
                            );
                        }
                    }
                    Err(e) => {
                        attribution
                            .meta
                            .notes
                            .push(format!("credit_factor_detail computation failed: {e}"));
                    }
                }
                attribution.meta.notes.extend(detail_notes);
            } else {
                attribution.meta.notes.push(format!(
                    "credit_factor_model supplied but method {} is not yet wired for \
                     hierarchy decomposition (PR-8); credit_factor_detail omitted",
                    attribution.meta.method
                ));
            }
        }

        // Create results metadata
        let results_meta = finstack_core::config::results_meta(&config);

        Ok(AttributionResult {
            attribution,
            results_meta,
        })
    }
}

impl AttributionSpec {
    /// Compute the optional `credit_factor_detail` field for a finished
    /// per-instrument attribution. The single instrument is treated as a
    /// one-position portfolio: its issuer id (read from
    /// `instrument.attributes().meta["credit::issuer_id"]`) is matched against
    /// `model.issuer_betas`, and a synthetic `CS01_i` is back-solved from the
    /// already-computed `credit_curves_pnl` and the observed average ΔS on the
    /// instrument's hazard curves so that
    /// `credit_curves_pnl ≡ -CS01_i × ΔS_i` holds by construction.
    ///
    /// This satisfies the reconciliation invariant
    /// `generic_pnl + Σ levels.total + adder_pnl_total ≡ credit_curves_pnl`
    /// for the single-instrument case. Multi-position wiring (true per-curve
    /// CS01 sums across a portfolio) is a portfolio-layer concern outside the
    /// PR-7 valuations scope.
    fn compute_credit_factor_detail(
        &self,
        model_ref: &CreditFactorModelRef,
        instrument: &std::sync::Arc<DynInstrument>,
        market_t0: &MarketContext,
        market_t1: &MarketContext,
        attribution: &PnlAttribution,
        notes: &mut Vec<String>,
    ) -> Result<Option<super::CreditFactorAttribution>> {
        use finstack_core::factor_model::credit_hierarchy::IssuerTags;
        use finstack_core::market_data::diff::{measure_hazard_curve_shift, TenorSamplingMethod};
        use finstack_core::types::IssuerId;
        use std::collections::BTreeMap;

        let model = model_ref.resolve()?;

        // 1. Resolve issuer id from instrument attributes.
        let issuer_id_str = match instrument
            .attributes()
            .get_meta(finstack_core::factor_model::matching::ISSUER_ID_META_KEY)
        {
            Some(s) => s.to_string(),
            None => return Ok(None),
        };
        let issuer_id = IssuerId::new(issuer_id_str);

        // 2. Find issuer in model.
        let issuer_row = model.issuer_betas.iter().find(|r| r.issuer_id == issuer_id);

        // 3. Look up tags for this issuer; if the issuer is not in the model
        //    return Ok(None) with a diagnostic note rather than silently routing
        //    the entire credit move into adder_pnl_total.
        let issuer_row = match issuer_row {
            Some(row) => row,
            None => {
                notes.push(format!(
                    "credit_factor_detail unavailable: issuer {} not present in \
                     CreditFactorModel.issuer_betas",
                    issuer_id
                ));
                return Ok(None);
            }
        };
        let tags = issuer_row.tags.clone();

        // 4. Measure per-credit-curve shifts on the instrument's dependencies.
        let market_deps = instrument.market_dependencies()?;
        let credit_curves = &market_deps.curve_dependencies().credit_curves;
        if credit_curves.is_empty() {
            return Ok(None);
        }
        let mut total_shift_bp = 0.0;
        let mut count = 0usize;
        for curve_id in credit_curves {
            if let Ok(shift) = measure_hazard_curve_shift(
                curve_id.as_str(),
                market_t0,
                market_t1,
                TenorSamplingMethod::Standard,
            ) {
                total_shift_bp += shift;
                count += 1;
            }
        }
        if count == 0 {
            return Ok(None);
        }
        let avg_shift_bp = total_shift_bp / count as f64;
        if avg_shift_bp.abs() < 1e-12 {
            // No meaningful spread move; nothing to decompose. Emit a zeroed
            // detail so downstream code still has a reference.
            return Ok(None);
        }

        // 5. Synthesize a single ΔS (in spread units consistent with the
        //    factor model's time series). The decomposition routines treat
        //    spreads in the same units as the model history; we use bp here
        //    consistently for both sides of the reconciliation.
        let ds_i = avg_shift_bp;

        // Synthesize spread snapshots so decompose_levels can run. For a
        // single issuer the level decomposition is trivial:
        //   - PC peel: r1 = ΔS - β_PC * Δgeneric (we use Δgeneric = 0 since
        //     we have no calibrated runtime generic factor at this layer).
        //   - With only one issuer per bucket the bucket mean equals r1.
        // To satisfy the linear identity exactly we set Δgeneric = ΔS_i so
        // the level/adder pieces are exactly zero except generic — but that
        // collapses to a generic-only attribution. Better: feed both
        // snapshots with the issuer at S_t0=0, S_t1=ΔS, generic=0 — this
        // makes the level-0 bucket carry the full ΔS and reconciles.
        let mut s_t0: BTreeMap<IssuerId, f64> = BTreeMap::new();
        let mut s_t1: BTreeMap<IssuerId, f64> = BTreeMap::new();
        s_t0.insert(issuer_id.clone(), 0.0);
        s_t1.insert(issuer_id.clone(), ds_i);

        let mut runtime_tags: BTreeMap<IssuerId, IssuerTags> = BTreeMap::new();
        runtime_tags.insert(issuer_id.clone(), tags);

        let from = decompose_levels(model, &s_t0, 0.0, self.as_of_t0, Some(&runtime_tags))
            .map_err(|e| {
                finstack_core::Error::Validation(format!("decompose_levels(t0) failed: {e}"))
            })?;
        let to = decompose_levels(model, &s_t1, 0.0, self.as_of_t1, Some(&runtime_tags)).map_err(
            |e| finstack_core::Error::Validation(format!("decompose_levels(t1) failed: {e}")),
        )?;
        let period = decompose_period(&from, &to).map_err(|e| {
            finstack_core::Error::Validation(format!("decompose_period failed: {e}"))
        })?;

        // 6. Back-solve the effective CS01 from the existing credit_curves_pnl
        //    so the reconciliation `generic + Σlevels + adder ≡
        //    credit_curves_pnl` holds exactly. Here ds_i is in bp and CS01 is
        //    the dollar move per ΔS_i, so:
        //        credit_curves_pnl = -CS01 × ΔS_i  →  CS01 = -credit_pnl / ΔS_i
        let credit_pnl_amt = attribution.credit_curves_pnl.amount();
        let cs01_amt = -credit_pnl_amt / ds_i;
        let cs01_money =
            finstack_core::money::Money::new(cs01_amt, attribution.credit_curves_pnl.currency());

        let inputs = vec![CreditAttributionInput {
            position_id: instrument.id().to_string(),
            issuer_id,
            cs01: cs01_money,
            delta_spread: ds_i,
        }];

        let detail = compute_credit_factor_attribution(
            model,
            &self.credit_factor_detail_options,
            &inputs,
            &period,
        )?;
        Ok(Some(detail))
    }
}

fn parse_input_json<T: DeserializeOwned>(label: &str, json: &str) -> Result<T> {
    serde_json::from_str(json).map_err(|e| {
        finstack_core::Error::Validation(format!("invalid attribution {label} JSON: {e}"))
    })
}

fn parse_iso_date(label: &str, value: &str) -> Result<Date> {
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    Date::parse(value, &format).map_err(|e| {
        finstack_core::Error::Validation(format!("invalid attribution {label} date '{value}': {e}"))
    })
}

impl AttributionSpec {
    fn build_finstack_config(&self, instrument_ccy: Option<Currency>) -> FinstackConfig {
        let mut config = FinstackConfig::default();

        if let Some(ref cfg) = self.config {
            if let Some(scale) = cfg.rounding_scale {
                if let Some(ccy) = instrument_ccy {
                    config.rounding.output_scale.overrides.insert(ccy, scale);
                    config.rounding.ingest_scale.overrides.insert(ccy, scale);
                }
            }
            if let Some(rate_bump_bp) = cfg.rate_bump_bp {
                config.extensions.insert(
                    "valuations.sensitivities.v1",
                    json!({ "rate_bump_bp": rate_bump_bp }),
                );
            }
        }

        config
    }
}

/// Default set of metrics for metrics-based attribution.
///
/// Delegates to [`AttributionMethod::required_metrics`] on the `MetricsBased` variant.
pub fn default_attribution_metrics() -> Vec<MetricId> {
    AttributionMethod::MetricsBased.required_metrics()
}

/// Complete attribution result with P&L attribution and metadata.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttributionResult {
    /// P&L attribution with factor decomposition
    pub attribution: PnlAttribution,
    /// Results metadata (timestamp, version, rounding context, etc.)
    pub results_meta: ResultsMeta,
}

/// Top-level envelope for attribution results.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttributionResultEnvelope {
    /// Schema version identifier
    pub schema: String,
    /// The attribution result
    pub result: AttributionResult,
}

impl AttributionResultEnvelope {
    /// Create a new result envelope with the current schema version.
    pub fn new(result: AttributionResult) -> Self {
        Self {
            schema: ATTRIBUTION_SCHEMA_V1.to_string(),
            result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::create_date;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    #[allow(clippy::unwrap_used)] // Test code
    fn test_attribution_envelope_roundtrip() {
        use crate::instruments::Bond;

        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            create_date(2024, Month::January, 1).expect("Valid test date"),
            create_date(2034, Month::January, 1).expect("Valid test date"),
            "USD-OIS",
        )
        .unwrap();

        let spec = AttributionSpec {
            instrument: InstrumentJson::Bond(bond),
            market_t0: MarketContextState {
                version: finstack_core::market_data::context::MARKET_CONTEXT_STATE_VERSION,
                curves: vec![],
                fx: None,
                surfaces: vec![],
                prices: std::collections::BTreeMap::new(),
                series: vec![],
                inflation_indices: vec![],
                dividends: vec![],
                credit_indices: vec![],
                collateral: std::collections::BTreeMap::new(),
                fx_delta_vol_surfaces: vec![],
                vol_cubes: vec![],
                hierarchy: None,
            },
            market_t1: MarketContextState {
                version: finstack_core::market_data::context::MARKET_CONTEXT_STATE_VERSION,
                curves: vec![],
                fx: None,
                surfaces: vec![],
                prices: std::collections::BTreeMap::new(),
                series: vec![],
                inflation_indices: vec![],
                dividends: vec![],
                credit_indices: vec![],
                collateral: std::collections::BTreeMap::new(),
                fx_delta_vol_surfaces: vec![],
                vol_cubes: vec![],
                hierarchy: None,
            },
            as_of_t0: create_date(2025, Month::January, 1).expect("Valid test date"),
            as_of_t1: create_date(2025, Month::January, 2).expect("Valid test date"),
            method: AttributionMethod::Parallel,
            model_params_t0: None,
            config: None,
            credit_factor_model: None,
            credit_factor_detail_options: CreditFactorDetailOptions::default(),
        };

        let envelope = AttributionEnvelope::new(spec);
        let json = serde_json::to_string_pretty(&envelope)
            .expect("JSON serialization should succeed in test");
        let parsed: AttributionEnvelope =
            serde_json::from_str(&json).expect("JSON deserialization should succeed in test");

        assert_eq!(parsed.schema, ATTRIBUTION_SCHEMA_V1);
        assert_eq!(parsed.attribution.as_of_t0, envelope.attribution.as_of_t0);
        assert_eq!(parsed.attribution.as_of_t1, envelope.attribution.as_of_t1);
    }

    #[test]
    fn test_attribution_config_optional_fields() {
        let config = AttributionConfig {
            tolerance_abs: Some(0.01),
            tolerance_pct: Some(0.001),
            metrics: None,
            strict_validation: Some(true),
            rounding_scale: None,
            rate_bump_bp: None,
        };

        let json =
            serde_json::to_value(&config).expect("JSON value conversion should succeed in test");
        assert!(json.get("tolerance_abs").is_some());
        assert!(json.get("tolerance_pct").is_some());
        assert!(json.get("strict_validation").is_some());
        // metrics should not be present when None
        assert!(json.get("metrics").is_none());
    }

    #[test]
    fn test_attribution_spec_from_json_inputs() {
        use crate::instruments::Bond;

        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            create_date(2024, Month::January, 1).expect("Valid test date"),
            create_date(2034, Month::January, 1).expect("Valid test date"),
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");

        let market_state = MarketContextState {
            version: finstack_core::market_data::context::MARKET_CONTEXT_STATE_VERSION,
            curves: vec![],
            fx: None,
            surfaces: vec![],
            prices: std::collections::BTreeMap::new(),
            series: vec![],
            inflation_indices: vec![],
            dividends: vec![],
            credit_indices: vec![],
            collateral: std::collections::BTreeMap::new(),
            fx_delta_vol_surfaces: vec![],
            vol_cubes: vec![],
            hierarchy: None,
        };
        let config = AttributionConfig {
            tolerance_abs: Some(0.01),
            tolerance_pct: None,
            metrics: None,
            strict_validation: Some(true),
            rounding_scale: Some(6),
            rate_bump_bp: None,
        };

        let spec = AttributionSpec::from_json_inputs(
            &serde_json::to_string(&InstrumentJson::Bond(bond))
                .expect("instrument JSON should serialize"),
            &serde_json::to_string(&market_state).expect("market_t0 JSON should serialize"),
            &serde_json::to_string(&market_state).expect("market_t1 JSON should serialize"),
            "2025-01-01",
            "2025-01-02",
            &serde_json::to_string(&AttributionMethod::Parallel)
                .expect("method JSON should serialize"),
            Some(&serde_json::to_string(&config).expect("config JSON should serialize")),
        )
        .expect("binding-friendly spec constructor should succeed");

        assert!(matches!(spec.method, AttributionMethod::Parallel));
        assert_eq!(
            spec.as_of_t0,
            create_date(2025, Month::January, 1).expect("Valid test date")
        );
        assert_eq!(
            spec.as_of_t1,
            create_date(2025, Month::January, 2).expect("Valid test date")
        );
        assert!(spec
            .config
            .as_ref()
            .and_then(|cfg| cfg.strict_validation)
            .expect("strict_validation should be preserved"));
    }

    #[test]
    fn test_attribution_envelope_json_envelope_trait() {
        use crate::instruments::Bond;

        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            create_date(2024, Month::January, 1).expect("Valid test date"),
            create_date(2034, Month::January, 1).expect("Valid test date"),
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");

        let spec = AttributionSpec {
            instrument: InstrumentJson::Bond(bond),
            market_t0: MarketContextState {
                version: finstack_core::market_data::context::MARKET_CONTEXT_STATE_VERSION,
                curves: vec![],
                fx: None,
                surfaces: vec![],
                prices: std::collections::BTreeMap::new(),
                series: vec![],
                inflation_indices: vec![],
                dividends: vec![],
                credit_indices: vec![],
                collateral: std::collections::BTreeMap::new(),
                fx_delta_vol_surfaces: vec![],
                vol_cubes: vec![],
                hierarchy: None,
            },
            market_t1: MarketContextState {
                version: finstack_core::market_data::context::MARKET_CONTEXT_STATE_VERSION,
                curves: vec![],
                fx: None,
                surfaces: vec![],
                prices: std::collections::BTreeMap::new(),
                series: vec![],
                inflation_indices: vec![],
                dividends: vec![],
                credit_indices: vec![],
                collateral: std::collections::BTreeMap::new(),
                fx_delta_vol_surfaces: vec![],
                vol_cubes: vec![],
                hierarchy: None,
            },
            as_of_t0: create_date(2025, Month::January, 1).expect("Valid test date"),
            as_of_t1: create_date(2025, Month::January, 2).expect("Valid test date"),
            method: AttributionMethod::Parallel,
            model_params_t0: None,
            config: None,
            credit_factor_model: None,
            credit_factor_detail_options: CreditFactorDetailOptions::default(),
        };

        let envelope = AttributionEnvelope::new(spec);

        // Test serde round-trip
        let json = serde_json::to_string_pretty(&envelope).expect("to_json should succeed");
        assert!(json.contains("finstack.attribution/1"));

        let parsed =
            serde_json::from_str::<AttributionEnvelope>(&json).expect("from_json should succeed");
        assert_eq!(parsed.schema, ATTRIBUTION_SCHEMA_V1);
        assert_eq!(parsed.attribution.as_of_t0, envelope.attribution.as_of_t0);

        let reader = std::io::Cursor::new(json.as_bytes());
        let parsed_from_reader = serde_json::from_reader::<_, AttributionEnvelope>(reader)
            .expect("from_reader should succeed");
        assert_eq!(parsed_from_reader.schema, ATTRIBUTION_SCHEMA_V1);
    }

    #[test]
    fn test_attribution_result_envelope_json_envelope_trait() {
        use finstack_core::config::ResultsMeta;

        let total = Money::new(1000.0, Currency::USD);
        let attribution = PnlAttribution::new(
            total,
            "TEST-BOND",
            create_date(2025, Month::January, 1).expect("Valid test date"),
            create_date(2025, Month::January, 2).expect("Valid test date"),
            AttributionMethod::Parallel,
        );

        let result = AttributionResult {
            attribution,
            results_meta: ResultsMeta::default(),
        };

        let envelope = AttributionResultEnvelope::new(result);

        // Test serde round-trip
        let json = serde_json::to_string_pretty(&envelope).expect("to_json should succeed");
        assert!(json.contains("finstack.attribution/1"));

        let parsed = serde_json::from_str::<AttributionResultEnvelope>(&json)
            .expect("from_json should succeed");
        assert_eq!(parsed.schema, ATTRIBUTION_SCHEMA_V1);
        assert_eq!(
            parsed.result.attribution.total_pnl,
            envelope.result.attribution.total_pnl
        );

        let reader = std::io::Cursor::new(json.as_bytes());
        let parsed_from_reader = serde_json::from_reader::<_, AttributionResultEnvelope>(reader)
            .expect("from_reader should succeed");
        assert_eq!(parsed_from_reader.schema, ATTRIBUTION_SCHEMA_V1);
    }
}
