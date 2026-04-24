//! Per-flow cashflow export with discount-factor / survival-probability / PV enrichment.
//!
//! Designed as the single Rust entry point behind the `instrument_cashflows_json`
//! Python and WASM bindings. Produces a structured envelope for any instrument
//! that is priceable under either the `Discounting` or `HazardRate` model. For
//! those two models, `sum(flows.pv) ≈ base_value` within rounding.
//!
//! # Not supported
//!
//! Option / tree / Monte Carlo / PDE / static-replication pricers are rejected
//! with a clear error explaining which models *are* valid for the given
//! instrument type. This guarantees reconciliation: if the exporter answers,
//! the sum is the price.
//!
//! # Columns
//!
//! Always populated (null-as-needed): `date, amount, currency, kind,
//! accrual_factor, year_fraction, rate, reset_date, discount_factor,
//! survival_probability, conditional_default_prob, inflation_index_ratio,
//! prepayment_smm, beginning_balance, ending_balance, pv`.
//!
//! Hazard-only columns are populated when `model = "hazard_rate"`.
//! Inflation / MBS columns are populated by concrete-type downcasts when the
//! instrument is `InflationLinkedBond` / `AgencyMbsPassthrough`. CMO tranche
//! pool state is left as null in this slice (TODO; the waterfall engine's
//! per-tranche balance hook is not yet exposed cleanly).

use finstack_core::cashflow::CFKind;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCountContext};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::types::CurveId;
use finstack_core::{Error, Result};
use serde::Serialize;

use crate::instruments::fixed_income::inflation_linked_bond::InflationLinkedBond;
use crate::instruments::fixed_income::mbs_passthrough::{
    generate_cashflows as mbs_generate_cashflows, AgencyMbsPassthrough,
};
use crate::instruments::json_loader::InstrumentEnvelope;
use crate::pricer::{parse_as_of_date, shared_standard_registry, ModelKey, PricerKey};

// ---------------------------------------------------------------------------
// Envelope schema
// ---------------------------------------------------------------------------

/// Top-level JSON envelope returned by [`instrument_cashflows_json`].
#[derive(Debug, Clone, Serialize)]
pub struct InstrumentCashflowEnvelope {
    /// Instrument identifier.
    pub instrument_id: String,
    /// Reporting currency (first flow's currency; errors if schedule mixes currencies).
    pub currency: Currency,
    /// Model key used (`"discounting"` or `"hazard_rate"`).
    pub model: String,
    /// Valuation date.
    pub as_of: Date,
    /// Discount curve ID used.
    pub discount_curve_id: CurveId,
    /// Hazard curve ID used (omitted for `discounting` model).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hazard_curve_id: Option<CurveId>,
    /// Recovery rate from the hazard curve (omitted for `discounting` model).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_rate: Option<f64>,
    /// Per-row enriched cashflows.
    pub flows: Vec<CashflowRow>,
    /// Sum of `flows[i].pv`. Matches `base_value` for supported products.
    pub total_pv: f64,
    /// Always `true` for the two supported models; reserved for future guards.
    pub reconciles_with_base_value: bool,
}

/// Single-row enriched cashflow view.
#[derive(Debug, Clone, Serialize)]
pub struct CashflowRow {
    /// Payment date.
    pub date: Date,
    /// Signed cashflow amount in row currency.
    pub amount: f64,
    /// Row currency (matters for `XccySwap` / `FxSwap`).
    pub currency: Currency,
    /// `CFKind` discriminator (serde rename: `fixed`, `notional`, …).
    pub kind: CFKind,
    /// Accrual factor stored on the `CashFlow`.
    pub accrual_factor: f64,
    /// Year fraction from `as_of` to `date` under the discount curve's day count.
    pub year_fraction: f64,
    /// Projected / contractual rate when present (floats, real-coupon rates, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
    /// Reset date when the flow is a floating-rate fixing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_date: Option<Date>,
    /// `df(as_of, date)`.
    pub discount_factor: f64,
    /// Cumulative survival probability (hazard mode only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub survival_probability: Option<f64>,
    /// Interval default probability `SP(t_{i-1}) − SP(t_i)` (hazard mode only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional_default_prob: Option<f64>,
    /// Inflation index ratio (populated for `InflationLinkedBond`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inflation_index_ratio: Option<f64>,
    /// Single Monthly Mortality for the period (populated for agency MBS).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepayment_smm: Option<f64>,
    /// Beginning pool balance for the period (agency MBS only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beginning_balance: Option<f64>,
    /// Ending pool balance for the period (agency MBS only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_balance: Option<f64>,
    /// Per-flow present value. Sums to `total_pv`.
    pub pv: f64,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Build the enriched cashflow envelope for a tagged instrument and serialize to JSON.
///
/// # Errors
///
/// Returns `Error::Validation` if the model string is not one of
/// `{"discounting", "hazard_rate"}`, if the `(instrument_type, model)` pair is
/// not in the standard pricer registry, if required curves are missing from
/// the market, or if the schedule mixes currencies.
pub fn instrument_cashflows_json(
    instrument_json: &str,
    market: &MarketContext,
    as_of: &str,
    model: &str,
) -> Result<String> {
    let envelope = build_envelope(instrument_json, market, as_of, model)?;
    serde_json::to_string(&envelope)
        .map_err(|e| Error::Validation(format!("failed to serialize cashflow envelope: {e}")))
}

fn build_envelope(
    instrument_json: &str,
    market: &MarketContext,
    as_of: &str,
    model: &str,
) -> Result<InstrumentCashflowEnvelope> {
    // --- Parse inputs ---
    let model_key: ModelKey = model.parse().map_err(|e: String| {
        Error::Validation(format!(
            "unknown model '{model}': {e}. Supported: 'discounting', 'hazard_rate'"
        ))
    })?;
    if !matches!(model_key, ModelKey::Discounting | ModelKey::HazardRate) {
        return Err(Error::Validation(format!(
            "model '{model}' not supported for instrument_cashflows; supported: 'discounting', 'hazard_rate'"
        )));
    }

    let as_of_date = parse_as_of_date(as_of)
        .map_err(|e| Error::Validation(format!("invalid as_of '{as_of}': {e}")))?;

    let instrument = InstrumentEnvelope::from_str(instrument_json)?;
    let instrument_type = instrument.key();
    let instrument_id = instrument.id().to_string();

    // --- Pricer registry gate: ensure the (type, model) pair is supported ---
    let registry = shared_standard_registry();
    let pricer_key = PricerKey::new(instrument_type, model_key);
    if registry.get_pricer(pricer_key).is_none() {
        return Err(Error::Validation(format!(
            "instrument type {instrument_type:?} is not priced under model '{model}' in instrument_cashflows; \
             this exporter supports only 'discounting' / 'hazard_rate' products where sum(pv) == base_value"
        )));
    }

    // --- Resolve curves ---
    let deps = instrument.market_dependencies()?;
    let curves = deps.curve_dependencies();
    let discount_curve_id = curves.discount_curves.first().cloned().ok_or_else(|| {
        Error::Validation(
            "instrument has no declared discount curve; cannot compute cashflow DFs".into(),
        )
    })?;
    let discount = market.get_discount(discount_curve_id.as_str())?;

    let (hazard_curve_id, hazard_arc) = if matches!(model_key, ModelKey::HazardRate) {
        let id = curves.credit_curves.first().cloned().ok_or_else(|| {
            Error::Validation(
                "instrument declares no hazard curve; hazard_rate model requires one".into(),
            )
        })?;
        let arc = market.get_hazard(id.as_str())?;
        (Some(id), Some(arc))
    } else {
        (None, None)
    };
    let recovery_rate = hazard_arc.as_ref().map(|h| h.recovery_rate());

    // --- Build schedule ---
    let schedule = instrument.cashflow_schedule(market, as_of_date)?;

    // --- Pre-compute instrument-specific side data ---
    let mbs_state: Option<std::collections::HashMap<Date, MbsState>> = instrument
        .as_any()
        .downcast_ref::<AgencyMbsPassthrough>()
        .and_then(|mbs| {
            mbs_generate_cashflows(mbs, as_of_date, None)
                .ok()
                .map(|rows| {
                    rows.into_iter()
                        .map(|r| {
                            (
                                r.payment_date,
                                MbsState {
                                    smm: r.smm,
                                    beginning_balance: r.beginning_balance,
                                    ending_balance: r.ending_balance,
                                },
                            )
                        })
                        .collect()
                })
        });
    let inflation_bond: Option<&InflationLinkedBond> =
        instrument.as_any().downcast_ref::<InflationLinkedBond>();

    // --- Iterate flows ---
    let curve_dc = discount.day_count();
    let dc_ctx = DayCountContext::default();

    let mut rows = Vec::with_capacity(schedule.flows.len());
    let mut total_pv = 0.0;
    let mut envelope_currency: Option<Currency> = None;
    let mut prev_sp = 1.0_f64;

    for flow in &schedule.flows {
        let ccy = flow.amount.currency();
        if let Some(first) = envelope_currency {
            if first != ccy {
                return Err(Error::Validation(format!(
                    "schedule mixes currencies ({first:?} and {ccy:?}); total_pv aggregation undefined. \
                     instrument_cashflows requires a single-currency schedule"
                )));
            }
        } else {
            envelope_currency = Some(ccy);
        }

        let year_fraction = signed_year_fraction(as_of_date, flow.date, curve_dc, dc_ctx)?;
        let discount_factor = discount.df(year_fraction);

        let (survival_probability, conditional_default_prob) = match hazard_arc.as_ref() {
            Some(h) => {
                let sp = h.sp(year_fraction);
                let cond_pd = (prev_sp - sp).max(0.0);
                prev_sp = sp;
                (Some(sp), Some(cond_pd))
            }
            None => (None, None),
        };

        let pv = compute_pv(
            flow.kind,
            flow.amount.amount(),
            discount_factor,
            survival_probability,
            recovery_rate,
        );
        total_pv += pv;

        let mbs_row = mbs_state.as_ref().and_then(|m| m.get(&flow.date));

        rows.push(CashflowRow {
            date: flow.date,
            amount: flow.amount.amount(),
            currency: ccy,
            kind: flow.kind,
            accrual_factor: flow.accrual_factor,
            year_fraction,
            rate: flow.rate,
            reset_date: flow.reset_date,
            discount_factor,
            survival_probability,
            conditional_default_prob,
            inflation_index_ratio: inflation_bond
                .and_then(|b| b.index_ratio_from_market(flow.date, market).ok()),
            prepayment_smm: mbs_row.map(|s| s.smm),
            beginning_balance: mbs_row.map(|s| s.beginning_balance),
            ending_balance: mbs_row.map(|s| s.ending_balance),
            pv,
        });
    }

    let currency = envelope_currency.unwrap_or(Currency::USD);

    Ok(InstrumentCashflowEnvelope {
        instrument_id,
        currency,
        model: model_key.to_string(),
        as_of: as_of_date,
        discount_curve_id,
        hazard_curve_id,
        recovery_rate,
        flows: rows,
        total_pv,
        reconciles_with_base_value: true,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct MbsState {
    smm: f64,
    beginning_balance: f64,
    ending_balance: f64,
}

/// Per-flow PV using the same `CFKind` semantics as
/// [`finstack_cashflows::aggregation::credit_adjusted_period_pv`]. Inlined to
/// avoid promoting the pub(crate) helper.
fn compute_pv(
    kind: CFKind,
    amount: f64,
    df: f64,
    sp: Option<f64>,
    recovery_rate: Option<f64>,
) -> f64 {
    // Hazard mode off → simple DF discounting.
    let Some(sp) = sp else {
        return amount * df;
    };

    // DefaultedNotional is zeroed (already defaulted, handled via Recovery).
    if kind == CFKind::DefaultedNotional {
        return 0.0;
    }

    // Recovery / AccruedOnDefault: realised post-default cashflows. No SP adjustment.
    if matches!(kind, CFKind::Recovery | CFKind::AccruedOnDefault) {
        return amount * df;
    }

    let recovery_term = match (recovery_rate, kind) {
        (Some(r), CFKind::Amortization | CFKind::Notional | CFKind::PrePayment) => r * (1.0 - sp),
        _ => 0.0,
    };
    amount * df * (sp + recovery_term)
}

/// Signed year fraction from `as_of` to `date` under `dc`. Matches the
/// convention used by `periodized_pv_credit_adjusted` so PV results
/// reconcile byte-for-byte.
fn signed_year_fraction(
    as_of: Date,
    date: Date,
    dc: finstack_core::dates::DayCount,
    dc_ctx: DayCountContext,
) -> Result<f64> {
    if date == as_of {
        Ok(0.0)
    } else if date > as_of {
        dc.year_fraction(as_of, date, dc_ctx)
    } else {
        Ok(-dc.year_fraction(date, as_of, dc_ctx)?)
    }
}

// ---------------------------------------------------------------------------
// Silence unused imports when built without dependent instruments in scope.
// ---------------------------------------------------------------------------

// Required to bring `DiscountCurve` / `HazardCurve` into scope for doc links.
#[allow(dead_code)]
fn _ensure_imports() {
    let _: Option<&DiscountCurve> = None;
    let _: Option<&HazardCurve> = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::bond::Bond;
    use crate::instruments::json_loader::{InstrumentEnvelope, InstrumentJson};
    use crate::instruments::Instrument;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
    use finstack_core::money::Money;
    use time::Month;

    fn serialize_bond(bond: &Bond) -> String {
        let envelope = InstrumentEnvelope {
            schema: InstrumentEnvelope::CURRENT_SCHEMA.to_string(),
            instrument: InstrumentJson::Bond(bond.clone()),
        };
        serde_json::to_string(&envelope).expect("serialize bond envelope")
    }

    #[test]
    fn discounting_reconciles_with_base_value_for_fixed_bond() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).expect("date");
        let maturity = Date::from_calendar_date(2030, Month::January, 15).expect("date");
        let bond = Bond::fixed(
            "BOND-DISC-RECONCILE",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("bond");

        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.80)])
            .build()
            .expect("discount curve");
        let market = MarketContext::new().insert(disc);

        let as_of_date = issue;
        let json = serialize_bond(&bond);
        let payload = instrument_cashflows_json(&json, &market, "2025-01-15", "discounting")
            .expect("cashflows envelope");
        let envelope: InstrumentCashflowEnvelope =
            serde_json::from_str(&payload).expect("parse envelope");

        let base = bond
            .base_value(&market, as_of_date)
            .expect("base value")
            .amount();
        let diff = (envelope.total_pv - base).abs();
        assert!(
            diff < 1e-4,
            "total_pv {} should reconcile with base_value {} (diff={})",
            envelope.total_pv,
            base,
            diff,
        );
        assert_eq!(envelope.model, "discounting");
        assert_eq!(envelope.currency, Currency::USD);
        assert!(envelope.reconciles_with_base_value);
        assert!(!envelope.flows.is_empty());
        for row in &envelope.flows {
            assert!(row.survival_probability.is_none());
            assert!(row.discount_factor > 0.0);
        }
    }

    #[test]
    fn rejects_unsupported_model_for_equity_option_style_instrument() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).expect("date");
        let maturity = Date::from_calendar_date(2026, Month::January, 15).expect("date");
        let bond = Bond::fixed(
            "BOND-BAD-MODEL",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("bond");
        let json = serialize_bond(&bond);
        let market = MarketContext::new();

        let err = instrument_cashflows_json(&json, &market, "2025-01-15", "monte_carlo_gbm")
            .expect_err("monte_carlo_gbm should reject bond");
        let msg = err.to_string();
        assert!(
            msg.contains("monte_carlo_gbm")
                || msg.contains("not priced")
                || msg.contains("supported"),
            "error should explain unsupported model: {msg}"
        );
    }

    #[test]
    fn hazard_rate_populates_survival_columns_for_bond_with_hazard_curve() {
        use crate::instruments::fixed_income::bond::BondConvention;

        let issue = Date::from_calendar_date(2025, Month::January, 15).expect("date");
        let maturity = Date::from_calendar_date(2028, Month::January, 15).expect("date");
        let mut bond = Bond::fixed(
            "BOND-HAZARD",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("bond");
        // Wire a hazard curve into the bond's convention.
        bond.convention = BondConvention {
            hazard_curve_id: Some("ACME-HZD".into()),
            ..bond.convention
        };

        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (1.0, 0.97), (5.0, 0.85)])
            .build()
            .expect("discount curve");
        let hazard = HazardCurve::builder("ACME-HZD")
            .base_date(issue)
            .recovery_rate(0.40)
            .knots([(0.0, 0.01), (5.0, 0.02)])
            .build()
            .expect("hazard curve");
        let market = MarketContext::new().insert(disc).insert(hazard);

        let json = serialize_bond(&bond);
        let payload = instrument_cashflows_json(&json, &market, "2025-01-15", "hazard_rate")
            .expect("hazard envelope");
        let envelope: InstrumentCashflowEnvelope =
            serde_json::from_str(&payload).expect("parse envelope");

        assert_eq!(envelope.model, "hazard_rate");
        assert_eq!(
            envelope.hazard_curve_id.as_deref().map(|c| c.as_str()),
            Some("ACME-HZD")
        );
        assert_eq!(envelope.recovery_rate, Some(0.40));
        for row in &envelope.flows {
            assert!(row.survival_probability.is_some());
            assert!(row.conditional_default_prob.is_some());
            let sp = row.survival_probability.expect("sp");
            assert!((0.0..=1.0).contains(&sp));
        }
    }
}
