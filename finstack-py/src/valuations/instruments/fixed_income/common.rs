use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fixed_income::cmo::CmoWaterfall;
use finstack_valuations::instruments::fixed_income::mbs_passthrough::AgencyProgram;
use finstack_valuations::instruments::fixed_income::tba::TbaTerm;
use finstack_valuations::instruments::Attributes;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

pub(crate) struct AgencyTradeContext {
    pub(crate) instrument_id: InstrumentId,
    pub(crate) agency: AgencyProgram,
    pub(crate) coupon: f64,
    pub(crate) term: TbaTerm,
    pub(crate) notional: Money,
    pub(crate) discount_curve_id: CurveId,
    pub(crate) attributes: Attributes,
}

pub(crate) struct AgencyMbsContext {
    pub(crate) instrument_id: InstrumentId,
    pub(crate) pool_id: String,
    pub(crate) agency: AgencyProgram,
    pub(crate) original_face: Money,
    pub(crate) current_face: Money,
    pub(crate) current_factor: f64,
    pub(crate) wac: f64,
    pub(crate) pass_through_rate: f64,
    pub(crate) wam: u32,
    pub(crate) issue_date: time::Date,
    pub(crate) maturity_date: time::Date,
    pub(crate) discount_curve_id: CurveId,
    pub(crate) day_count: DayCount,
    pub(crate) attributes: Attributes,
}

pub(crate) struct AgencyCmoContext {
    pub(crate) instrument_id: InstrumentId,
    pub(crate) deal_name: String,
    pub(crate) agency: AgencyProgram,
    pub(crate) issue_date: time::Date,
    pub(crate) waterfall: CmoWaterfall,
    pub(crate) reference_tranche_id: String,
    pub(crate) discount_curve_id: CurveId,
    pub(crate) attributes: Attributes,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn validated_agency_trade_context(
    builder_name: &str,
    instrument_id: &InstrumentId,
    agency: Option<AgencyProgram>,
    coupon: Option<f64>,
    term: Option<TbaTerm>,
    notional: Option<f64>,
    currency: Option<Currency>,
    discount_curve_id: Option<&str>,
) -> PyResult<AgencyTradeContext> {
    let currency = require_field(builder_name, "currency", currency)?;
    let notional_amount = require_field(builder_name, "notional", notional)?;

    Ok(AgencyTradeContext {
        instrument_id: instrument_id.clone(),
        agency: require_field(builder_name, "agency", agency)?,
        coupon: require_field(builder_name, "coupon", coupon)?,
        term: require_field(builder_name, "term", term)?,
        notional: Money::new(notional_amount, currency),
        discount_curve_id: CurveId::new(require_non_empty_field(
            builder_name,
            "discount_curve_id",
            discount_curve_id,
        )?),
        attributes: Attributes::new(),
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn validated_agency_mbs_context(
    builder_name: &str,
    instrument_id: &InstrumentId,
    pool_id: Option<&str>,
    agency: Option<AgencyProgram>,
    original_face: Option<f64>,
    current_face: Option<f64>,
    current_factor: Option<f64>,
    currency: Option<Currency>,
    wac: Option<f64>,
    pass_through_rate: Option<f64>,
    wam: Option<u32>,
    issue_date: Option<time::Date>,
    maturity_date: Option<time::Date>,
    discount_curve_id: Option<&str>,
    day_count: DayCount,
) -> PyResult<AgencyMbsContext> {
    let currency = require_field(builder_name, "currency", currency)?;
    let original_face_amount = require_field(builder_name, "original_face", original_face)?;
    let current_face_amount = require_field(builder_name, "current_face", current_face)?;

    Ok(AgencyMbsContext {
        instrument_id: instrument_id.clone(),
        pool_id: require_non_empty_field(builder_name, "pool_id", pool_id)?.to_string(),
        agency: require_field(builder_name, "agency", agency)?,
        original_face: Money::new(original_face_amount, currency),
        current_face: Money::new(current_face_amount, currency),
        current_factor: current_factor.unwrap_or(current_face_amount / original_face_amount),
        wac: require_field(builder_name, "wac", wac)?,
        pass_through_rate: require_field(builder_name, "pass_through_rate", pass_through_rate)?,
        wam: require_field(builder_name, "wam", wam)?,
        issue_date: require_field(builder_name, "issue_date", issue_date)?,
        maturity_date: require_field(builder_name, "maturity_date", maturity_date)?,
        discount_curve_id: CurveId::new(require_non_empty_field(
            builder_name,
            "discount_curve_id",
            discount_curve_id,
        )?),
        day_count,
        attributes: Attributes::new(),
    })
}

pub(crate) fn validated_agency_cmo_context(
    builder_name: &str,
    instrument_id: &InstrumentId,
    deal_name: Option<&str>,
    agency: Option<AgencyProgram>,
    issue_date: Option<time::Date>,
    waterfall: Option<CmoWaterfall>,
    reference_tranche_id: Option<&str>,
    discount_curve_id: Option<&str>,
) -> PyResult<AgencyCmoContext> {
    Ok(AgencyCmoContext {
        instrument_id: instrument_id.clone(),
        deal_name: require_non_empty_field(builder_name, "deal_name", deal_name)?.to_string(),
        agency: require_field(builder_name, "agency", agency)?,
        issue_date: require_field(builder_name, "issue_date", issue_date)?,
        waterfall: require_field(builder_name, "waterfall", waterfall)?,
        reference_tranche_id: require_non_empty_field(
            builder_name,
            "reference_tranche_id",
            reference_tranche_id,
        )?
        .to_string(),
        discount_curve_id: CurveId::new(require_non_empty_field(
            builder_name,
            "discount_curve_id",
            discount_curve_id,
        )?),
        attributes: Attributes::new(),
    })
}

pub(crate) fn require_field<T>(
    builder_name: &str,
    field_name: &str,
    value: Option<T>,
) -> PyResult<T> {
    value.ok_or_else(|| {
        PyRuntimeError::new_err(format!(
            "{builder_name} internal error: missing {field_name} after validation"
        ))
    })
}

pub(crate) fn require_non_empty_field<'a>(
    builder_name: &str,
    field_name: &str,
    value: Option<&'a str>,
) -> PyResult<&'a str> {
    match value {
        Some(value) if !value.is_empty() => Ok(value),
        _ => Err(PyRuntimeError::new_err(format!(
            "{builder_name} internal error: missing {field_name} after validation"
        ))),
    }
}
