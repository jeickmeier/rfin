use serde::Deserialize;
use serde_json::Value;

// Shared envelope used by embedded registry files (similar to market conventions).
#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryFile<R> {
    pub schema: Option<String>,
    pub version: Option<u32>,
    pub entries: Vec<RegistryEntry<R>>,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryEntry<R> {
    pub ids: Vec<String>,
    pub record: R,
}

// -----------------------------------------------------------------------------//
// Schedule IM (BCBS-IOSCO grid)
// -----------------------------------------------------------------------------//

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleImFile {
    pub schema: Option<String>,
    pub version: Option<u32>,
    pub entries: Vec<RegistryEntry<ScheduleImRecord>>,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleImRecord {
    pub bucket_boundaries_years: ScheduleBucketBoundaries,
    pub default_rate: f64,
    pub default_asset_class: String,
    pub default_maturity_years: f64,
    pub mpor_days: u32,
    pub rates: Vec<ScheduleImRate>,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleBucketBoundaries {
    pub short_to_medium: f64,
    pub medium_to_long: f64,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleImRate {
    pub asset_class: String,
    pub bucket: String,
    pub rate: f64,
}

// -----------------------------------------------------------------------------//
// Collateral schedules and defaults
// -----------------------------------------------------------------------------//

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollateralSchedulesFile {
    pub schema: Option<String>,
    pub version: Option<u32>,
    pub asset_class_defaults: Vec<AssetClassDefault>,
    pub entries: Vec<RegistryEntry<CollateralScheduleRecord>>,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetClassDefault {
    pub asset_class: String,
    pub standard_haircut: f64,
    pub fx_addon: f64,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollateralScheduleRecord {
    pub eligible: Vec<CollateralEligibilityRecord>,
    pub default_haircut: Option<f64>,
    pub rehypothecation_allowed: bool,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollateralEligibilityRecord {
    pub asset_class: String,
    #[serde(default)]
    pub min_rating: Option<String>,
    #[serde(default)]
    pub maturity_constraints: Option<MaturityConstraintsRecord>,
    pub haircut: f64,
    pub fx_haircut_addon: f64,
    #[serde(default)]
    pub concentration_limit: Option<f64>,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaturityConstraintsRecord {
    #[serde(default)]
    pub min_remaining_years: Option<f64>,
    #[serde(default)]
    pub max_remaining_years: Option<f64>,
}

// -----------------------------------------------------------------------------//
// Defaults (VM/IM thresholds, timing, settlement)
// -----------------------------------------------------------------------------//

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultsFile {
    pub schema: Option<String>,
    pub version: Option<u32>,
    pub defaults: DefaultsRecord,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultsRecord {
    pub vm: VmDefaultsRecord,
    pub im: ImDefaultsRecord,
    pub timing: TimingDefaultsRecord,
    pub cleared_settlement: ClearedSettlementRecord,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VmDefaultsRecord {
    pub threshold: f64,
    pub mta: f64,
    pub rounding: f64,
    pub independent_amount: f64,
    pub frequency: String,
    pub settlement_lag: u32,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImDefaultsRecord {
    pub simm: ImMethodDefaultsRecord,
    pub schedule: ImMethodDefaultsRecord,
    pub cleared: ImMethodDefaultsRecord,
    pub repo_haircut: ImMethodDefaultsRecord,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ImMethodDefaultsRecord {
    pub mpor_days: u32,
    pub threshold: f64,
    pub mta: f64,
    pub segregated: bool,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimingDefaultsRecord {
    pub standard: MarginCallTimingRecord,
    pub regulatory_vm: MarginCallTimingRecord,
    pub ccp: MarginCallTimingRecord,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct MarginCallTimingRecord {
    pub notification_deadline_hours: u8,
    pub response_deadline_hours: u8,
    pub dispute_resolution_days: u8,
    pub delivery_grace_days: u8,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClearedSettlementRecord {
    pub rounding: f64,
    pub settlement_lag: u32,
}

// -----------------------------------------------------------------------------//
// CCP methodologies
// -----------------------------------------------------------------------------//

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CcpFile {
    pub schema: Option<String>,
    pub version: Option<u32>,
    pub entries: Vec<RegistryEntry<CcpRecord>>,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CcpRecord {
    pub mpor_days: u32,
    pub conservative_rate: f64,
    #[serde(default)]
    pub is_default: bool,
}

// -----------------------------------------------------------------------------//
// SIMM parameters
// -----------------------------------------------------------------------------//

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimmFile {
    pub schema: Option<String>,
    pub version: Option<u32>,
    pub entries: Vec<RegistryEntry<SimmRecord>>,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimmRecord {
    pub mpor_days: u32,
    pub ir_delta_weights: Value,
    pub cq_delta_weights: Value,
    pub cnq_delta_weight: f64,
    pub equity_delta_weight: f64,
    pub fx_delta_weight: f64,
    pub risk_class_correlations: Vec<RiskClassCorrelationRecord>,
    pub commodity_bucket_weights: Value,
    #[serde(default)]
    pub ir_tenor_correlations: Value,
    #[serde(default)]
    pub ir_vega_weight: Option<f64>,
    #[serde(default)]
    pub cq_vega_weight: Option<f64>,
    #[serde(default)]
    pub cnq_vega_weight: Option<f64>,
    #[serde(default)]
    pub equity_vega_weight: Option<f64>,
    #[serde(default)]
    pub fx_vega_weight: Option<f64>,
    #[serde(default)]
    pub commodity_vega_weight: Option<f64>,
    #[serde(default)]
    pub curvature_scale_factor: Option<f64>,
    #[serde(default)]
    pub concentration_thresholds: Value,
}

#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiskClassCorrelationRecord {
    pub a: String,
    pub b: String,
    pub rho: f64,
}
