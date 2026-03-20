//! Margin-registry loading and override helpers.
//!
//! This module exposes the resolved configuration surface that powers schedule
//! IM, SIMM parameters, collateral schedules, and CCP fallback settings.
//! Values are loaded from embedded JSON assets and may be overlaid through
//! [`finstack_core::config::FinstackConfig`] using
//! [`crate::registry::MARGIN_REGISTRY_EXTENSION_KEY`].
//!
//! # Units And Conventions
//!
//! - Rates and haircuts are stored as decimal fractions, not basis points.
//! - Thresholds, MTAs, and independent amounts are stored as raw currency
//!   amounts before conversion into [`finstack_core::money::Money`].
//! - `mpor_days` and settlement lags are stored in calendar days.
//! - Schedule maturities are stored as year fractions.
use std::sync::OnceLock;

use finstack_core::config::FinstackConfig;
use finstack_core::money::Money;
use finstack_core::{Error, HashMap, Result};
use serde_json::Value;
use tracing::{debug, info};

use crate::calculators::im::schedule::{MaturityBucket, ScheduleAssetClass};
use crate::calculators::im::simm::SimmVersion;
use crate::types::{
    CollateralAssetClass, CollateralEligibility, EligibleCollateralSchedule, ImMethodology,
    ImParameters, MarginCallTiming, MarginTenor, MaturityConstraints, VmParameters,
};
use crate::SimmRiskClass;

mod embedded;
mod merge;
mod wire;

pub use merge::merge_json;

/// Fully resolved, ready-to-use margin registry.
///
/// This is the public, parsed view of the embedded margin registry after any
/// optional JSON overlay has been applied.
#[derive(Debug, Clone)]
pub struct MarginRegistry {
    /// Workspace-wide default VM, IM, timing, and settlement settings.
    pub defaults: MarginDefaults,
    /// Regulatory schedule IM grids keyed by registry id such as `"bcbs_iosco"`.
    pub schedule_im: HashMap<String, ScheduleImSchedule>,
    /// Default collateral haircuts keyed by collateral asset class.
    pub collateral_asset_class_defaults: HashMap<CollateralAssetClass, AssetClassDefault>,
    /// Eligible collateral schedules keyed by registry id.
    pub collateral_schedules: HashMap<String, EligibleCollateralSchedule>,
    /// CCP conservative fallback parameters keyed by CCP identifier.
    pub ccp: HashMap<String, CcpParams>,
    /// Optional default key into [`Self::ccp`].
    pub ccp_default: Option<String>,
    /// SIMM parameter sets keyed by registry id such as `"v2_6"`.
    pub simm: HashMap<String, SimmParams>,
    /// Optional default key into [`Self::simm`].
    pub simm_default: Option<String>,
}

/// Top-level default settings shared across margin methodologies.
#[derive(Debug, Clone)]
pub struct MarginDefaults {
    /// Variation-margin defaults.
    pub vm: VmDefaults,
    /// Initial-margin defaults split by methodology.
    pub im: ImDefaults,
    /// Margin call timing defaults for different agreement types.
    pub timing: TimingDefaults,
    /// Settlement defaults for cleared margin flows.
    pub cleared_settlement: ClearedSettlementDefaults,
}

/// Default variation-margin parameters stored in raw numeric form.
#[derive(Debug, Clone)]
pub struct VmDefaults {
    /// VM threshold amount in base-currency units before conversion to [`finstack_core::money::Money`].
    pub threshold: f64,
    /// Minimum transfer amount in base-currency units before conversion to [`finstack_core::money::Money`].
    pub mta: f64,
    /// Rounding amount in base-currency units before conversion to [`finstack_core::money::Money`].
    pub rounding: f64,
    /// Independent amount in base-currency units before conversion to [`finstack_core::money::Money`].
    pub independent_amount: f64,
    /// Margin-call frequency.
    pub frequency: MarginTenor,
    /// Settlement lag in calendar days.
    pub settlement_lag: u32,
}

/// Default initial-margin settings grouped by methodology.
#[derive(Debug, Clone)]
pub struct ImDefaults {
    /// Defaults for SIMM-based bilateral IM.
    pub simm: ImMethodDefaults,
    /// Defaults for schedule-based IM.
    pub schedule: ImMethodDefaults,
    /// Defaults for cleared-derivative CCP IM.
    pub cleared: ImMethodDefaults,
    /// Defaults for repo haircut IM.
    pub repo_haircut: ImMethodDefaults,
}

/// Raw default parameters for a single IM methodology.
#[derive(Debug, Clone)]
pub struct ImMethodDefaults {
    /// Margin period of risk in calendar days.
    pub mpor_days: u32,
    /// Threshold amount in base-currency units before conversion to [`finstack_core::money::Money`].
    pub threshold: f64,
    /// Minimum transfer amount in base-currency units before conversion to [`finstack_core::money::Money`].
    pub mta: f64,
    /// Whether the methodology assumes segregated collateral.
    pub segregated: bool,
}

/// Default timing conventions for margin calls and responses.
#[derive(Debug, Clone)]
pub struct TimingDefaults {
    /// Standard bilateral timing defaults.
    pub standard: MarginCallTiming,
    /// Regulatory VM timing defaults.
    pub regulatory_vm: MarginCallTiming,
    /// Cleared or CCP timing defaults.
    pub ccp: MarginCallTiming,
}

/// Default settlement handling for cleared collateral flows.
#[derive(Debug, Clone)]
pub struct ClearedSettlementDefaults {
    /// Settlement rounding amount in base-currency units before conversion to [`finstack_core::money::Money`].
    pub rounding: f64,
    /// Settlement lag in calendar days.
    pub settlement_lag: u32,
}

/// Parsed regulatory schedule IM entry from the registry.
#[derive(Debug, Clone)]
pub struct ScheduleImSchedule {
    /// Year-fraction boundaries separating short, medium, and long buckets.
    pub boundaries: ScheduleBucketBoundaries,
    /// Fallback decimal rate used when a bucket-specific entry is absent.
    pub default_rate: f64,
    /// Default asset class used by the schedule calculator's trait-based fallback path.
    pub default_asset_class: ScheduleAssetClass,
    /// Default maturity expressed as a year fraction.
    pub default_maturity_years: f64,
    /// Margin period of risk in calendar days.
    pub mpor_days: u32,
    /// Decimal rates keyed by `(asset_class, maturity_bucket)`.
    pub rates: HashMap<(ScheduleAssetClass, MaturityBucket), f64>,
}

/// Boundaries, in years, for the schedule IM maturity buckets.
#[derive(Debug, Clone)]
pub struct ScheduleBucketBoundaries {
    /// Cutoff between short and medium maturity buckets, in years.
    pub short_to_medium: f64,
    /// Cutoff between medium and long maturity buckets, in years.
    pub medium_to_long: f64,
}

/// Default collateral haircut settings for an asset class.
#[derive(Debug, Clone)]
pub struct AssetClassDefault {
    /// Standard collateral haircut as a decimal fraction.
    pub standard_haircut: f64,
    /// Additional FX haircut as a decimal fraction when collateral currency differs.
    pub fx_addon: f64,
}

/// Conservative fallback CCP parameters.
#[derive(Debug, Clone)]
pub struct CcpParams {
    /// Margin period of risk in calendar days.
    pub mpor_days: u32,
    /// Conservative fallback margin rate as a decimal fraction.
    pub conservative_rate: f64,
}

/// Registry-backed SIMM parameter set.
#[derive(Debug, Clone)]
pub struct SimmParams {
    /// SIMM version identifier for this parameter set.
    pub version: SimmVersion,
    /// Margin period of risk in calendar days.
    pub mpor_days: u32,
    /// Interest-rate delta risk weights keyed by tenor label.
    pub ir_delta_weights: HashMap<String, f64>,
    /// Credit-qualifying delta risk weights keyed by sector or bucket label.
    pub cq_delta_weights: HashMap<String, f64>,
    /// Credit-non-qualifying delta risk weight.
    pub cnq_delta_weight: f64,
    /// Equity delta risk weight.
    pub equity_delta_weight: f64,
    /// FX delta risk weight.
    pub fx_delta_weight: f64,
    /// Cross-risk-class correlations keyed by risk-class pair.
    pub risk_class_correlations: HashMap<(SimmRiskClass, SimmRiskClass), f64>,
    /// Commodity delta risk weights keyed by bucket label.
    pub commodity_bucket_weights: HashMap<String, f64>,
    /// Interest-rate tenor correlations keyed by ordered tenor pair.
    pub ir_tenor_correlations: HashMap<(String, String), f64>,
    /// Interest-rate vega risk weight.
    pub ir_vega_weight: f64,
    /// Credit-qualifying vega risk weight.
    pub cq_vega_weight: f64,
    /// Credit-non-qualifying vega risk weight.
    pub cnq_vega_weight: f64,
    /// Equity vega risk weight.
    pub equity_vega_weight: f64,
    /// FX vega risk weight.
    pub fx_vega_weight: f64,
    /// Commodity vega risk weight.
    pub commodity_vega_weight: f64,
    /// Curvature scale factor applied before aggregation.
    pub curvature_scale_factor: f64,
    /// Concentration thresholds keyed by SIMM risk class.
    pub concentration_thresholds: HashMap<SimmRiskClass, f64>,
}

static EMBEDDED_REGISTRY: OnceLock<MarginRegistry> = OnceLock::new();

/// Access the embedded registry without any overlays.
///
/// The result is cached for the lifetime of the process.
///
/// # Returns
///
/// A shared reference to the parsed embedded registry.
///
/// # Errors
///
/// Returns an error if the embedded JSON assets cannot be parsed into a valid
/// [`MarginRegistry`].
pub fn embedded_registry() -> Result<&'static MarginRegistry> {
    if EMBEDDED_REGISTRY.get().is_none() {
        let registry = build_registry(None)?;
        let _ = EMBEDDED_REGISTRY.set(registry);
    }
    EMBEDDED_REGISTRY
        .get()
        .ok_or_else(|| Error::Validation("Failed to load embedded margin registry".to_string()))
}

/// Build a registry applying an optional JSON overlay.
///
/// # Arguments
///
/// * `overlay` - Parsed JSON overlay that follows the embedded registry schema
///
/// # Returns
///
/// A fully parsed [`MarginRegistry`] containing embedded defaults plus any overlay values.
///
/// # Errors
///
/// Returns an error if the embedded assets are invalid, if the overlay is
/// malformed, or if any parsed values violate registry validation rules.
pub fn build_registry(overlay: Option<&Value>) -> Result<MarginRegistry> {
    let mut root = embedded::load_embedded_root()?;
    if let Some(ov) = overlay {
        debug!("Applying margin registry overlay");
        merge::merge_json(&mut root, ov);
    }

    let schedule_im = parse_schedule_im(root.get("schedule_im"))?;
    let (collateral_defaults, collateral_schedules) =
        parse_collateral_schedules(root.get("collateral_schedules"))?;
    let defaults = parse_defaults(root.get("defaults"))?;
    let (ccp, ccp_default) = parse_ccp(root.get("ccp"))?;
    let (simm, simm_default) = parse_simm(root.get("simm"))?;

    info!(
        schedules = schedule_im.len(),
        collateral_schedules = collateral_schedules.len(),
        ccps = ccp.len(),
        simm_versions = simm.len(),
        has_overlay = overlay.is_some(),
        "Margin registry loaded"
    );

    Ok(MarginRegistry {
        defaults,
        schedule_im,
        collateral_asset_class_defaults: collateral_defaults,
        collateral_schedules,
        ccp,
        ccp_default,
        simm,
        simm_default,
    })
}

// -----------------------------------------------------------------------------//
// Parse helpers
// -----------------------------------------------------------------------------//

fn parse_schedule_im(value: Option<&Value>) -> Result<HashMap<String, ScheduleImSchedule>> {
    let Some(val) = value else {
        return Err(Error::Validation("schedule_im section missing".to_string()));
    };
    let file: wire::ScheduleImFile = serde_json::from_value(val.clone()).map_err(to_validation)?;
    let mut map = HashMap::default();
    for entry in file.entries {
        let record = entry.record;
        validate_non_negative("default_rate", record.default_rate)?;
        let boundaries = ScheduleBucketBoundaries {
            short_to_medium: record.bucket_boundaries_years.short_to_medium,
            medium_to_long: record.bucket_boundaries_years.medium_to_long,
        };
        if boundaries.short_to_medium <= 0.0
            || boundaries.medium_to_long <= boundaries.short_to_medium
        {
            return Err(Error::Validation(
                "schedule_im bucket boundaries must be increasing and > 0".to_string(),
            ));
        }
        let default_asset_class = parse_schedule_asset_class(&record.default_asset_class)?;
        let mut rates = HashMap::default();
        for rate in record.rates {
            validate_rate("schedule_im.rate", rate.rate)?;
            let asset_class = parse_schedule_asset_class(&rate.asset_class)?;
            let bucket = parse_maturity_bucket(&rate.bucket)?;
            rates.insert((asset_class, bucket), rate.rate);
        }
        let sched = ScheduleImSchedule {
            boundaries,
            default_rate: record.default_rate,
            default_asset_class,
            default_maturity_years: record.default_maturity_years,
            mpor_days: record.mpor_days,
            rates,
        };
        for id in entry.ids {
            if map.insert(id.clone(), sched.clone()).is_some() {
                return Err(Error::Validation(format!(
                    "duplicate schedule_im id '{id}'"
                )));
            }
        }
    }
    Ok(map)
}

fn parse_collateral_schedules(
    value: Option<&Value>,
) -> Result<(
    HashMap<CollateralAssetClass, AssetClassDefault>,
    HashMap<String, EligibleCollateralSchedule>,
)> {
    let Some(val) = value else {
        return Err(Error::Validation(
            "collateral_schedules section missing".to_string(),
        ));
    };
    let file: wire::CollateralSchedulesFile =
        serde_json::from_value(val.clone()).map_err(to_validation)?;

    let mut defaults = HashMap::default();
    for def in file.asset_class_defaults {
        let asset = parse_collateral_asset_class(&def.asset_class)?;
        validate_haircut("standard_haircut", def.standard_haircut)?;
        validate_haircut("fx_addon", def.fx_addon)?;
        defaults.insert(
            asset,
            AssetClassDefault {
                standard_haircut: def.standard_haircut,
                fx_addon: def.fx_addon,
            },
        );
    }

    let mut schedules = HashMap::default();
    for entry in file.entries {
        let mut eligible = Vec::new();
        for elig in entry.record.eligible {
            validate_haircut("haircut", elig.haircut)?;
            validate_haircut("fx_haircut_addon", elig.fx_haircut_addon)?;
            let asset_class = parse_collateral_asset_class(&elig.asset_class)?;
            let constraints = elig
                .maturity_constraints
                .as_ref()
                .map(|c| MaturityConstraints {
                    min_remaining_years: c.min_remaining_years,
                    max_remaining_years: c.max_remaining_years,
                });
            eligible.push(CollateralEligibility {
                asset_class,
                min_rating: elig.min_rating.clone(),
                maturity_constraints: constraints,
                haircut: elig.haircut,
                fx_haircut_addon: elig.fx_haircut_addon,
                concentration_limit: elig.concentration_limit,
            });
        }

        let schedule = EligibleCollateralSchedule {
            eligible,
            default_haircut: entry.record.default_haircut,
            rehypothecation_allowed: entry.record.rehypothecation_allowed,
        };
        for id in entry.ids {
            if schedules.insert(id.clone(), schedule.clone()).is_some() {
                return Err(Error::Validation(format!(
                    "duplicate collateral schedule id '{id}'"
                )));
            }
        }
    }

    Ok((defaults, schedules))
}

fn parse_defaults(value: Option<&Value>) -> Result<MarginDefaults> {
    let Some(val) = value else {
        return Err(Error::Validation("defaults section missing".to_string()));
    };
    let file: wire::DefaultsFile = serde_json::from_value(val.clone()).map_err(to_validation)?;

    let vm_freq = parse_margin_tenor(&file.defaults.vm.frequency)?;
    let vm = VmDefaults {
        threshold: file.defaults.vm.threshold,
        mta: file.defaults.vm.mta,
        rounding: file.defaults.vm.rounding,
        independent_amount: file.defaults.vm.independent_amount,
        frequency: vm_freq,
        settlement_lag: file.defaults.vm.settlement_lag,
    };

    let im = ImDefaults {
        simm: to_im_method(&file.defaults.im.simm),
        schedule: to_im_method(&file.defaults.im.schedule),
        cleared: to_im_method(&file.defaults.im.cleared),
        repo_haircut: to_im_method(&file.defaults.im.repo_haircut),
    };

    let timing = TimingDefaults {
        standard: to_timing(&file.defaults.timing.standard),
        regulatory_vm: to_timing(&file.defaults.timing.regulatory_vm),
        ccp: to_timing(&file.defaults.timing.ccp),
    };

    let cleared_settlement = ClearedSettlementDefaults {
        rounding: file.defaults.cleared_settlement.rounding,
        settlement_lag: file.defaults.cleared_settlement.settlement_lag,
    };

    Ok(MarginDefaults {
        vm,
        im,
        timing,
        cleared_settlement,
    })
}

fn parse_ccp(value: Option<&Value>) -> Result<(HashMap<String, CcpParams>, Option<String>)> {
    let Some(val) = value else {
        return Err(Error::Validation("ccp section missing".to_string()));
    };
    let file: wire::CcpFile = serde_json::from_value(val.clone()).map_err(to_validation)?;
    let mut map = HashMap::default();
    let mut default: Option<String> = None;
    for entry in file.entries {
        validate_rate("ccp.conservative_rate", entry.record.conservative_rate)?;
        let record = CcpParams {
            mpor_days: entry.record.mpor_days,
            conservative_rate: entry.record.conservative_rate,
        };
        for id in entry.ids {
            if map.insert(id.clone(), record.clone()).is_some() {
                return Err(Error::Validation(format!("duplicate ccp id '{id}'")));
            }
            if entry.record.is_default {
                default.get_or_insert(id.clone());
            }
        }
    }
    Ok((map, default))
}

fn parse_simm(value: Option<&Value>) -> Result<(HashMap<String, SimmParams>, Option<String>)> {
    let Some(val) = value else {
        return Err(Error::Validation("simm section missing".to_string()));
    };
    let file: wire::SimmFile = serde_json::from_value(val.clone()).map_err(to_validation)?;
    let mut map = HashMap::default();
    let mut default: Option<String> = None;
    for entry in file.entries {
        let record = entry.record;
        let version = parse_simm_version(entry.ids.first().map(String::as_str))?;
        let ir_delta_weights = parse_number_map(&record.ir_delta_weights, "simm.ir_delta_weights")?;
        let cq_delta_weights = parse_number_map(&record.cq_delta_weights, "simm.cq_delta_weights")?;
        let commodity_bucket_weights = parse_number_map(
            &record.commodity_bucket_weights,
            "simm.commodity_bucket_weights",
        )?;

        validate_rate("simm.cnq_delta_weight", record.cnq_delta_weight)?;
        validate_rate("simm.equity_delta_weight", record.equity_delta_weight)?;
        validate_rate("simm.fx_delta_weight", record.fx_delta_weight)?;

        let mut correlations: HashMap<(SimmRiskClass, SimmRiskClass), f64> = HashMap::default();
        for cor in record.risk_class_correlations {
            let a = parse_simm_risk_class(&cor.a)?;
            let b = parse_simm_risk_class(&cor.b)?;
            if !(cor.rho >= -1.0 && cor.rho <= 1.0) {
                return Err(Error::Validation(format!(
                    "simm correlation for ({a:?},{b:?}) must be in [-1,1]"
                )));
            }
            let key = ordered_pair(a, b);
            correlations.insert(key, cor.rho);
        }

        let ir_tenor_correlations = parse_ir_tenor_correlations(&record.ir_tenor_correlations)?;
        let ir_vega_weight = record.ir_vega_weight.unwrap_or(0.21);
        let cq_vega_weight = record.cq_vega_weight.unwrap_or(0.27);
        let cnq_vega_weight = record.cnq_vega_weight.unwrap_or(0.27);
        let equity_vega_weight = record.equity_vega_weight.unwrap_or(0.26);
        let fx_vega_weight = record.fx_vega_weight.unwrap_or(0.30);
        let commodity_vega_weight = record.commodity_vega_weight.unwrap_or(0.36);
        let curvature_scale_factor = record.curvature_scale_factor.unwrap_or(1.5);
        let concentration_thresholds =
            parse_concentration_thresholds(&record.concentration_thresholds)?;

        let params = SimmParams {
            version,
            mpor_days: record.mpor_days,
            ir_delta_weights,
            cq_delta_weights,
            cnq_delta_weight: record.cnq_delta_weight,
            equity_delta_weight: record.equity_delta_weight,
            fx_delta_weight: record.fx_delta_weight,
            risk_class_correlations: correlations,
            commodity_bucket_weights,
            ir_tenor_correlations,
            ir_vega_weight,
            cq_vega_weight,
            cnq_vega_weight,
            equity_vega_weight,
            fx_vega_weight,
            commodity_vega_weight,
            curvature_scale_factor,
            concentration_thresholds,
        };

        validate_simm_params(&params)?;

        for id in entry.ids {
            if map.insert(id.clone(), params.clone()).is_some() {
                return Err(Error::Validation(format!("duplicate simm id '{id}'")));
            }
            if default.is_none() && matches!(id.as_str(), "v2_6" | "default") {
                default = Some(id.clone());
            }
        }
    }
    Ok((map, default))
}

// -----------------------------------------------------------------------------//
// Public helper for overrides via FinstackConfig
// -----------------------------------------------------------------------------//

/// Extension key used by [`finstack_core::config::FinstackConfig`] for margin-registry JSON overlays.
pub const MARGIN_REGISTRY_EXTENSION_KEY: &str = "valuations.margin_registry.v1";

/// Build a margin registry from a [`finstack_core::config::FinstackConfig`] extension overlay.
///
/// # Arguments
///
/// * `cfg` - Config whose `extensions` map may contain [`crate::registry::MARGIN_REGISTRY_EXTENSION_KEY`]
///
/// # Returns
///
/// A new [`MarginRegistry`] built from embedded defaults plus any configured overlay.
///
/// # Errors
///
/// Returns an error if the embedded registry or the configured overlay fails validation.
pub fn margin_registry_from_config(cfg: &FinstackConfig) -> Result<MarginRegistry> {
    let overlay = cfg.extensions.get(MARGIN_REGISTRY_EXTENSION_KEY);
    build_registry(overlay)
}

// -----------------------------------------------------------------------------//
// Conversions and validations
// -----------------------------------------------------------------------------//

fn parse_schedule_asset_class(value: &str) -> Result<ScheduleAssetClass> {
    value
        .parse::<ScheduleAssetClass>()
        .map_err(|e| Error::Validation(format!("invalid schedule asset class '{value}': {e}")))
}

fn parse_maturity_bucket(value: &str) -> Result<MaturityBucket> {
    match value.to_ascii_lowercase().as_str() {
        "short" => Ok(MaturityBucket::Short),
        "medium" => Ok(MaturityBucket::Medium),
        "long" => Ok(MaturityBucket::Long),
        other => Err(Error::Validation(format!(
            "unknown maturity bucket '{other}'"
        ))),
    }
}

fn parse_collateral_asset_class(value: &str) -> Result<CollateralAssetClass> {
    value
        .parse::<CollateralAssetClass>()
        .map_err(|e| Error::Validation(format!("invalid collateral asset class '{value}': {e}")))
}

fn parse_margin_tenor(value: &str) -> Result<MarginTenor> {
    value
        .parse::<MarginTenor>()
        .map_err(|e| Error::Validation(format!("invalid margin frequency '{value}': {e}")))
}

fn parse_simm_version(id: Option<&str>) -> Result<SimmVersion> {
    match id.unwrap_or_default().to_ascii_lowercase().as_str() {
        "v2_5" => Ok(SimmVersion::V2_5),
        "v2_6" | "default" => Ok(SimmVersion::V2_6),
        other => Err(Error::Validation(format!(
            "unknown SIMM version id '{other}'"
        ))),
    }
}

fn parse_simm_risk_class(value: &str) -> Result<SimmRiskClass> {
    match value.to_ascii_lowercase().as_str() {
        "interest_rate" | "ir" => Ok(SimmRiskClass::InterestRate),
        "credit_qualifying" | "cq" => Ok(SimmRiskClass::CreditQualifying),
        "credit_non_qualifying" | "cnq" => Ok(SimmRiskClass::CreditNonQualifying),
        "equity" => Ok(SimmRiskClass::Equity),
        "commodity" => Ok(SimmRiskClass::Commodity),
        "fx" => Ok(SimmRiskClass::Fx),
        other => Err(Error::Validation(format!(
            "unknown SIMM risk class '{other}'"
        ))),
    }
}

fn parse_number_map(value: &Value, context: &str) -> Result<HashMap<String, f64>> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::Validation(format!("{context} must be an object")))?;
    let mut out = HashMap::default();
    for (k, v) in obj {
        let num = v.as_f64().ok_or_else(|| {
            Error::Validation(format!("{context} value for key '{k}' must be a number"))
        })?;
        out.insert(k.clone(), num);
    }
    Ok(out)
}

fn parse_ir_tenor_correlations(value: &Value) -> Result<HashMap<(String, String), f64>> {
    if value.is_null() {
        return Ok(HashMap::default());
    }
    let obj = value
        .as_object()
        .ok_or_else(|| Error::Validation("ir_tenor_correlations must be an object".to_string()))?;
    let mut out = HashMap::default();
    for (k, v) in obj {
        let parts: Vec<&str> = k.splitn(2, '_').collect();
        if parts.len() != 2 {
            return Err(Error::Validation(format!(
                "invalid ir_tenor_correlations key '{k}': expected format 'tenor1_tenor2'"
            )));
        }
        let rho = v.as_f64().ok_or_else(|| {
            Error::Validation(format!(
                "ir_tenor_correlations value for '{k}' must be a number"
            ))
        })?;
        if !(-1.0..=1.0).contains(&rho) {
            return Err(Error::Validation(format!(
                "ir_tenor_correlations value for '{k}' must be in [-1,1]"
            )));
        }
        let (a, b) = ordered_tenor_pair(parts[0], parts[1]);
        out.insert((a, b), rho);
    }
    Ok(out)
}

fn ordered_tenor_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

fn parse_concentration_thresholds(value: &Value) -> Result<HashMap<SimmRiskClass, f64>> {
    if value.is_null() {
        return Ok(HashMap::default());
    }
    let obj = value.as_object().ok_or_else(|| {
        Error::Validation("concentration_thresholds must be an object".to_string())
    })?;
    let mut out = HashMap::default();
    for (k, v) in obj {
        let rc = parse_simm_risk_class(k)?;
        let threshold = v.as_f64().ok_or_else(|| {
            Error::Validation(format!(
                "concentration_thresholds value for '{k}' must be a number"
            ))
        })?;
        out.insert(rc, threshold);
    }
    Ok(out)
}

fn to_im_method(record: &wire::ImMethodDefaultsRecord) -> ImMethodDefaults {
    ImMethodDefaults {
        mpor_days: record.mpor_days,
        threshold: record.threshold,
        mta: record.mta,
        segregated: record.segregated,
    }
}

fn to_timing(record: &wire::MarginCallTimingRecord) -> MarginCallTiming {
    MarginCallTiming {
        notification_deadline_hours: record.notification_deadline_hours,
        response_deadline_hours: record.response_deadline_hours,
        dispute_resolution_days: record.dispute_resolution_days,
        delivery_grace_days: record.delivery_grace_days,
    }
}

fn ordered_pair(a: SimmRiskClass, b: SimmRiskClass) -> (SimmRiskClass, SimmRiskClass) {
    if (a as u8) <= (b as u8) {
        (a, b)
    } else {
        (b, a)
    }
}

fn validate_simm_params(p: &SimmParams) -> Result<()> {
    if p.mpor_days == 0 {
        return Err(Error::Validation(
            "simm mpor_days must be greater than zero".to_string(),
        ));
    }
    for (k, v) in &p.ir_delta_weights {
        validate_non_negative(&format!("simm.ir_delta_weights[{k}]"), *v)?;
    }
    for (k, v) in &p.cq_delta_weights {
        validate_non_negative(&format!("simm.cq_delta_weights[{k}]"), *v)?;
    }
    for (k, v) in &p.commodity_bucket_weights {
        validate_non_negative(&format!("simm.commodity_bucket_weights[{k}]"), *v)?;
    }
    validate_non_negative("simm.cnq_delta_weight", p.cnq_delta_weight)?;
    validate_non_negative("simm.equity_delta_weight", p.equity_delta_weight)?;
    validate_non_negative("simm.fx_delta_weight", p.fx_delta_weight)?;
    validate_non_negative("simm.ir_vega_weight", p.ir_vega_weight)?;
    validate_non_negative("simm.cq_vega_weight", p.cq_vega_weight)?;
    validate_non_negative("simm.cnq_vega_weight", p.cnq_vega_weight)?;
    validate_non_negative("simm.equity_vega_weight", p.equity_vega_weight)?;
    validate_non_negative("simm.fx_vega_weight", p.fx_vega_weight)?;
    validate_non_negative("simm.commodity_vega_weight", p.commodity_vega_weight)?;
    validate_non_negative("simm.curvature_scale_factor", p.curvature_scale_factor)?;
    for (rc, v) in &p.concentration_thresholds {
        validate_non_negative(&format!("simm.concentration_thresholds[{rc:?}]"), *v)?;
    }
    Ok(())
}

fn validate_rate(name: &str, v: f64) -> Result<()> {
    if !v.is_finite() || v < 0.0 {
        return Err(Error::Validation(format!(
            "invalid rate '{name}': must be finite and >= 0"
        )));
    }
    Ok(())
}

fn validate_non_negative(name: &str, v: f64) -> Result<()> {
    if !v.is_finite() || v < 0.0 {
        return Err(Error::Validation(format!(
            "invalid value '{name}': must be finite and >= 0"
        )));
    }
    Ok(())
}

fn validate_haircut(name: &str, v: f64) -> Result<()> {
    if !v.is_finite() || !(0.0..=1.0).contains(&v) {
        return Err(Error::Validation(format!(
            "invalid haircut '{name}': must be in [0,1]"
        )));
    }
    Ok(())
}

fn to_validation(err: serde_json::Error) -> Error {
    Error::Validation(format!("Failed to parse margin registry: {err}"))
}

// -----------------------------------------------------------------------------//
// Convenience helpers for constructing Money amounts from defaults
// -----------------------------------------------------------------------------//

impl VmDefaults {
    /// Convert raw VM defaults into currency-tagged [`VmParameters`].
    ///
    /// # Arguments
    ///
    /// * `currency` - Currency used to wrap raw numeric defaults in [`finstack_core::money::Money`]
    ///
    /// # Returns
    ///
    /// Concrete VM parameters in `currency`.
    pub fn to_vm_params(&self, currency: finstack_core::currency::Currency) -> VmParameters {
        VmParameters {
            threshold: Money::new(self.threshold, currency),
            mta: Money::new(self.mta, currency),
            rounding: Money::new(self.rounding, currency),
            independent_amount: Money::new(self.independent_amount, currency),
            frequency: self.frequency,
            settlement_lag: self.settlement_lag,
        }
    }
}

impl ImMethodDefaults {
    /// Convert raw IM defaults into currency-tagged [`ImParameters`].
    ///
    /// # Arguments
    ///
    /// * `methodology` - IM methodology label for the returned parameter set
    /// * `currency` - Currency used to wrap raw numeric defaults in [`finstack_core::money::Money`]
    ///
    /// # Returns
    ///
    /// Concrete IM parameters in `currency`.
    pub fn to_im_params(
        &self,
        methodology: ImMethodology,
        currency: finstack_core::currency::Currency,
    ) -> ImParameters {
        ImParameters {
            methodology,
            mpor_days: self.mpor_days,
            threshold: Money::new(self.threshold, currency),
            mta: Money::new(self.mta, currency),
            segregated: self.segregated,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn embedded_registry_loads_successfully() {
        let registry = embedded_registry().expect("embedded registry should load");

        assert!(
            !registry.schedule_im.is_empty(),
            "schedule_im should have entries"
        );
        assert!(
            registry.schedule_im.contains_key("bcbs_iosco"),
            "bcbs_iosco schedule should be present"
        );

        assert!(
            !registry.collateral_schedules.is_empty(),
            "collateral_schedules should have entries"
        );
        assert!(
            registry.collateral_schedules.contains_key("bcbs_standard"),
            "bcbs_standard collateral schedule should be present"
        );
        assert!(
            registry.collateral_schedules.contains_key("cash_only"),
            "cash_only collateral schedule should be present"
        );

        assert!(
            !registry.collateral_asset_class_defaults.is_empty(),
            "collateral_asset_class_defaults should have entries"
        );
        assert!(
            registry
                .collateral_asset_class_defaults
                .contains_key(&CollateralAssetClass::Cash),
            "Cash default should be present"
        );

        assert!(!registry.simm.is_empty(), "simm should have entries");
        assert!(
            registry.simm_default.is_some(),
            "simm_default should be set"
        );

        assert!(!registry.ccp.is_empty(), "ccp should have entries");

        assert!(
            registry.defaults.im.simm.mpor_days > 0,
            "simm mpor_days should be positive"
        );
    }

    #[test]
    fn simm_params_have_required_weights() {
        let registry = embedded_registry().expect("embedded registry should load");
        let simm_id = registry.simm_default.as_ref().expect("simm_default set");
        let params = registry.simm.get(simm_id).expect("default simm params");

        assert!(
            params.ir_delta_weights.contains_key("5y"),
            "5y IR weight should be present"
        );
        assert!(
            params.cq_delta_weights.contains_key("corporates"),
            "corporates CQ weight should be present"
        );
        assert!(params.cnq_delta_weight > 0.0, "CNQ weight should be > 0");
        assert!(
            params.equity_delta_weight > 0.0,
            "equity weight should be > 0"
        );
        assert!(params.fx_delta_weight > 0.0, "FX weight should be > 0");
        assert!(
            !params.risk_class_correlations.is_empty(),
            "risk class correlations should be populated"
        );
    }
}
