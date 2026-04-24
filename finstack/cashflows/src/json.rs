//! JSON bridge for constructing and validating cashflow schedules.
//!
//! This module is intentionally small and serde-first. It gives bindings a
//! stable string-based surface while preserving the Rust builder and schedule
//! types as the canonical schema.

use crate::accrual::{accrued_interest_amount, AccrualConfig};
use crate::builder::{CashFlowSchedule, FeeSpec, FixedCouponSpec, FloatingCouponSpec, Notional};
use crate::primitives::CFKind;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::{Error, Result};

/// Specification for building a [`CashFlowSchedule`] from JSON.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CashflowScheduleBuildSpec {
    /// Principal amount and amortization behavior.
    pub notional: Notional,
    /// Contract issue date.
    #[schemars(with = "String")]
    pub issue: Date,
    /// Contract maturity date.
    #[schemars(with = "String")]
    pub maturity: Date,
    /// Fixed coupon legs to add to the schedule.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixed_coupons: Vec<FixedCouponSpec>,
    /// Floating coupon legs to add to the schedule.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub floating_coupons: Vec<FloatingCouponSpec>,
    /// Fee legs to add to the schedule.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fees: Vec<FeeSpec>,
    /// Explicit principal events to add after the base principal setup.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub principal_events: Vec<PrincipalEventSpec>,
}

/// JSON representation of an explicit principal event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrincipalEventSpec {
    /// Event date.
    #[schemars(with = "String")]
    pub date: Date,
    /// Outstanding balance delta. Positive increases outstanding, negative repays.
    pub delta: Money,
    /// Optional cash leg. When omitted, the cash leg equals `delta`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cash: Option<Money>,
    /// Cashflow classification to emit.
    pub kind: CFKind,
}

/// JSON-friendly dated flow item.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatedFlowJson {
    /// Flow date.
    #[schemars(with = "String")]
    pub date: Date,
    /// Dated amount.
    pub amount: Money,
}

impl CashflowScheduleBuildSpec {
    /// Build a canonical cashflow schedule.
    ///
    /// # Errors
    ///
    /// Returns an error when the specification is internally inconsistent or
    /// when floating coupons require market data that is unavailable.
    pub fn build(&self, market: Option<&MarketContext>) -> Result<CashFlowSchedule> {
        let mut builder = CashFlowSchedule::builder();
        let _ = builder
            .principal(self.notional.initial, self.issue, self.maturity)
            .amortization(self.notional.amort.clone());

        for spec in &self.fixed_coupons {
            let _ = builder.fixed_cf(spec.clone());
        }
        for spec in &self.floating_coupons {
            let _ = builder.floating_cf(spec.clone());
        }
        for spec in &self.fees {
            let _ = builder.fee(spec.clone());
        }
        for event in &self.principal_events {
            let _ = builder.add_principal_event(event.date, event.delta, event.cash, event.kind);
        }

        builder.build_with_curves(market)
    }
}

/// Build a schedule from JSON and return canonical schedule JSON.
///
/// # Errors
///
/// Returns an error if the input JSON cannot be parsed, the market JSON is
/// invalid, the build spec is inconsistent, or the output cannot be serialized.
pub fn build_cashflow_schedule_json(spec_json: &str, market_json: Option<&str>) -> Result<String> {
    let spec: CashflowScheduleBuildSpec = serde_json::from_str(spec_json).map_err(|err| {
        Error::Validation(format!("invalid cashflow schedule build spec JSON: {err}"))
    })?;
    let market = parse_optional_market(market_json)?;
    let schedule = spec.build(market.as_ref())?;
    serialize_json(&schedule, "cashflow schedule")
}

/// Validate a schedule JSON payload and return canonical schedule JSON.
///
/// # Errors
///
/// Returns an error if the input is not a valid [`CashFlowSchedule`] JSON value.
pub fn validate_cashflow_schedule_json(schedule_json: &str) -> Result<String> {
    let schedule = parse_schedule(schedule_json)?;
    serialize_json(&schedule, "cashflow schedule")
}

/// Extract dated amounts from a schedule JSON payload.
///
/// # Errors
///
/// Returns an error if the schedule JSON is invalid or the output cannot be
/// serialized.
pub fn dated_flows_json(schedule_json: &str) -> Result<String> {
    let schedule = parse_schedule(schedule_json)?;
    let flows: Vec<DatedFlowJson> = schedule
        .flows
        .iter()
        .map(|flow| DatedFlowJson {
            date: flow.date,
            amount: flow.amount,
        })
        .collect();
    serialize_json(&flows, "dated flows")
}

/// Compute accrued interest from a schedule JSON payload.
///
/// # Errors
///
/// Returns an error if the schedule, as-of date, or optional accrual config JSON
/// cannot be parsed.
pub fn accrued_interest_json(
    schedule_json: &str,
    as_of: &str,
    config_json: Option<&str>,
) -> Result<f64> {
    let schedule = parse_schedule(schedule_json)?;
    let as_of = parse_iso_date(as_of)?;
    let config = match config_json {
        Some(json) => serde_json::from_str::<AccrualConfig>(json)
            .map_err(|err| Error::Validation(format!("invalid accrual config JSON: {err}")))?,
        None => AccrualConfig::default(),
    };
    accrued_interest_amount(&schedule, as_of, &config)
}

fn parse_schedule(schedule_json: &str) -> Result<CashFlowSchedule> {
    serde_json::from_str(schedule_json)
        .map_err(|err| Error::Validation(format!("invalid cashflow schedule JSON: {err}")))
}

fn parse_optional_market(market_json: Option<&str>) -> Result<Option<MarketContext>> {
    market_json
        .map(|json| {
            serde_json::from_str::<MarketContext>(json)
                .map_err(|err| Error::Validation(format!("invalid market context JSON: {err}")))
        })
        .transpose()
}

fn parse_iso_date(value: &str) -> Result<Date> {
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    Date::parse(value, &format)
        .map_err(|err| Error::Validation(format!("invalid ISO date '{value}': {err}")))
}

fn serialize_json<T: serde::Serialize>(value: &T, label: &str) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|err| Error::Validation(format!("failed to serialize {label} JSON: {err}")))
}
