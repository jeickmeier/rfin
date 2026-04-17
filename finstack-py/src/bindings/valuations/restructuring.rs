//! Python bindings for the credit-events / restructuring toolkit.
//!
//! Exposes a lean, function-based API over
//! [`finstack_valuations::restructuring`] using simple Python primitives
//! (dicts, strings, floats) instead of requiring callers to construct
//! the full Rust domain types.
//!
//! Functions exposed:
//!
//! * [`execute_recovery_waterfall`] - distribute value across an ordered
//!   claim stack using the Absolute Priority Rule.
//! * [`analyze_exchange_offer`] - compare hold-vs-tender economics for a
//!   distressed exchange offer.
//! * [`analyze_lme`] - compute discount capture, notional reduction, and
//!   leverage/remaining-holder impact for an LME transaction.
//!
//! Enum arguments are passed as lowercase snake-case strings; see each
//! function's docstring for the accepted values.

use crate::errors::display_to_py;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::restructuring::{
    execute_recovery_waterfall as rust_execute_waterfall, AllocationMode, Claim, ClaimSeniority,
    CollateralAllocation, RecoveryWaterfall,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a currency code string (e.g. ``"USD"``).
fn parse_currency(code: &str) -> PyResult<Currency> {
    Currency::from_str(code)
        .map_err(|e| PyValueError::new_err(format!("invalid currency '{code}': {e}")))
}

/// Parse a `ClaimSeniority` from a lowercase snake-case string.
fn parse_seniority(s: &str) -> PyResult<ClaimSeniority> {
    match s.to_ascii_lowercase().as_str() {
        "dip" | "dip_financing" => Ok(ClaimSeniority::DipFinancing),
        "admin" | "administrative" => Ok(ClaimSeniority::Administrative),
        "priority" => Ok(ClaimSeniority::Priority),
        "first_lien" | "first_lien_secured" => Ok(ClaimSeniority::FirstLienSecured),
        "second_lien" | "second_lien_secured" => Ok(ClaimSeniority::SecondLienSecured),
        "third_lien" | "junior_secured" => Ok(ClaimSeniority::JuniorSecured),
        "senior_unsecured" | "unsecured" => Ok(ClaimSeniority::SeniorUnsecured),
        "senior_subordinated" => Ok(ClaimSeniority::SeniorSubordinated),
        "subordinated" | "sub" => Ok(ClaimSeniority::Subordinated),
        "mezzanine" | "mezz" => Ok(ClaimSeniority::Mezzanine),
        "preferred_equity" | "preferred" => Ok(ClaimSeniority::PreferredEquity),
        "equity" | "common_equity" | "common" => Ok(ClaimSeniority::CommonEquity),
        other => Err(PyValueError::new_err(format!(
            "unknown seniority '{other}' (expected one of: first_lien, second_lien, \
             junior_secured, senior_unsecured, senior_subordinated, subordinated, \
             mezzanine, preferred_equity, equity, dip_financing, administrative, priority)"
        ))),
    }
}

/// Convert a `ClaimSeniority` to its canonical lowercase string form.
fn seniority_to_str(s: ClaimSeniority) -> &'static str {
    match s {
        ClaimSeniority::DipFinancing => "dip_financing",
        ClaimSeniority::Administrative => "administrative",
        ClaimSeniority::Priority => "priority",
        ClaimSeniority::FirstLienSecured => "first_lien",
        ClaimSeniority::SecondLienSecured => "second_lien",
        ClaimSeniority::JuniorSecured => "junior_secured",
        ClaimSeniority::SeniorUnsecured => "senior_unsecured",
        ClaimSeniority::SeniorSubordinated => "senior_subordinated",
        ClaimSeniority::Subordinated => "subordinated",
        ClaimSeniority::Mezzanine => "mezzanine",
        ClaimSeniority::PreferredEquity => "preferred_equity",
        ClaimSeniority::CommonEquity => "equity",
    }
}

/// Parse an intra-class allocation mode string.
fn parse_allocation_mode(s: &str) -> PyResult<AllocationMode> {
    match s.to_ascii_lowercase().as_str() {
        "pro_rata" | "prorata" | "pro-rata" => Ok(AllocationMode::ProRata),
        "strict" | "strict_priority" => Ok(AllocationMode::StrictPriority),
        other => Err(PyValueError::new_err(format!(
            "unknown allocation_mode '{other}' (expected 'pro_rata' or 'strict_priority')"
        ))),
    }
}

/// Extract an optional `f64` from a PyDict key, defaulting to `default`.
fn dict_get_f64(dict: &Bound<'_, PyDict>, key: &str, default: f64) -> PyResult<f64> {
    match dict.get_item(key)? {
        Some(v) => v.extract::<f64>(),
        None => Ok(default),
    }
}

/// Extract a required `f64` from a PyDict key.
fn dict_get_f64_required(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<f64> {
    dict.get_item(key)?
        .ok_or_else(|| PyValueError::new_err(format!("claim dict missing required key '{key}'")))
        .and_then(|v| v.extract::<f64>())
}

/// Extract an optional `String` from a PyDict key.
fn dict_get_string(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<String>> {
    match dict.get_item(key)? {
        Some(v) => Ok(Some(v.extract::<String>()?)),
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// execute_recovery_waterfall
// ---------------------------------------------------------------------------

/// Run a recovery waterfall over an ordered claim stack.
///
/// Distributes ``total_value`` across the given claims in priority order
/// following the Absolute Priority Rule (APR). Secured claims first
/// recover from their collateral (net of haircut); any shortfall becomes
/// a deficiency claim in the unsecured pool.
///
/// Parameters
/// ----------
/// total_value : float
///     Total enterprise value / liquidation proceeds available for
///     distribution.
/// currency : str
///     ISO currency code (e.g. ``"USD"``).
/// claims : list[dict]
///     Ordered claims. Each dict supports:
///
///     * ``seniority`` (str, required) - one of ``first_lien``,
///       ``second_lien``, ``junior_secured``, ``senior_unsecured``,
///       ``senior_subordinated``, ``subordinated``, ``mezzanine``,
///       ``preferred_equity``, ``equity``, ``dip_financing``,
///       ``administrative``, ``priority``.
///     * ``principal`` (float, required) - outstanding principal.
///     * ``accrued`` (float, optional, default ``0.0``) - accrued interest.
///     * ``penalties`` (float, optional, default ``0.0``) - make-whole /
///       prepayment penalties.
///     * ``collateral_value`` (float, optional) - if present, the claim
///       is treated as secured against collateral of this net value
///       (haircut already applied; set ``haircut=0``).
///     * ``haircut`` (float, optional, default ``0.0``) - haircut to
///       apply to ``collateral_value`` (0.0 - 1.0).
///     * ``id`` (str, optional) - claim identifier (auto-generated if
///       absent).
///     * ``label`` (str, optional) - human-readable label.
/// allocation_mode : str, optional
///     Intra-class allocation for each claim: ``"pro_rata"`` (default)
///     or ``"strict_priority"``.
///
/// Returns
/// -------
/// dict
///     ``{"total_distributed": float, "residual": float,
///     "apr_satisfied": bool, "apr_violations": list[str],
///     "per_claim_recovery": [{id, seniority, principal, accrued,
///     total_claim, collateral_recovery, general_recovery,
///     total_recovery, recovery_rate, deficiency}, ...]}``.
///
/// Raises
/// ------
/// ValueError
///     If inputs are invalid (negative amounts, unknown seniority,
///     bad haircut, etc.).
#[pyfunction]
#[pyo3(signature = (total_value, currency, claims, allocation_mode="pro_rata"))]
pub fn execute_recovery_waterfall<'py>(
    py: Python<'py>,
    total_value: f64,
    currency: &str,
    claims: Vec<Bound<'py, PyDict>>,
    allocation_mode: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let ccy = parse_currency(currency)?;
    let alloc = parse_allocation_mode(allocation_mode)?;

    let mut built: Vec<Claim> = Vec::with_capacity(claims.len());
    for (idx, dict) in claims.iter().enumerate() {
        let seniority_str: String = dict
            .get_item("seniority")?
            .ok_or_else(|| {
                PyValueError::new_err(format!("claim #{idx} missing required key 'seniority'"))
            })?
            .extract()?;
        let seniority = parse_seniority(&seniority_str)?;

        let principal = dict_get_f64_required(dict, "principal")?;
        let accrued = dict_get_f64(dict, "accrued", 0.0)?;
        let penalties = dict_get_f64(dict, "penalties", 0.0)?;

        let id = dict_get_string(dict, "id")?.unwrap_or_else(|| format!("claim-{idx}"));
        let label = dict_get_string(dict, "label")?.unwrap_or_else(|| id.clone());

        let collateral = match dict.get_item("collateral_value")? {
            Some(v) => {
                let value = v.extract::<f64>()?;
                let haircut = dict_get_f64(dict, "haircut", 0.0)?;
                Some(CollateralAllocation {
                    description: format!("{label} collateral"),
                    value: Money::new(value, ccy),
                    haircut,
                    shared: false,
                    shared_with: Vec::new(),
                })
            }
            None => None,
        };

        built.push(Claim {
            id,
            label,
            seniority,
            principal: Money::new(principal, ccy),
            accrued_interest: Money::new(accrued, ccy),
            penalties: Money::new(penalties, ccy),
            instrument_id: None,
            collateral,
            intra_class_allocation: alloc,
        });
    }

    let waterfall = RecoveryWaterfall {
        distributable_value: Money::new(total_value, ccy),
        claims: built,
        strict_apr: true,
        plan_deviations: Vec::new(),
    };

    let result = rust_execute_waterfall(&waterfall).map_err(display_to_py)?;

    let out = PyDict::new(py);
    out.set_item("total_distributed", result.total_distributed.amount())?;
    out.set_item("residual", result.residual.amount())?;
    out.set_item("apr_satisfied", result.apr_satisfied)?;
    out.set_item("apr_violations", result.apr_violations.clone())?;

    let recoveries = PyList::empty(py);
    for cr in &result.claim_recoveries {
        let row = PyDict::new(py);
        row.set_item("id", &cr.claim_id)?;
        row.set_item("seniority", seniority_to_str(cr.seniority))?;
        row.set_item("total_claim", cr.total_claim.amount())?;
        row.set_item("collateral_recovery", cr.collateral_recovery.amount())?;
        row.set_item("general_recovery", cr.general_recovery.amount())?;
        row.set_item("total_recovery", cr.total_recovery.amount())?;
        row.set_item("recovery_rate", cr.recovery_rate)?;
        row.set_item("deficiency", cr.deficiency_claim.amount())?;
        recoveries.append(row)?;
    }
    out.set_item("per_claim_recovery", recoveries)?;

    Ok(out)
}

// ---------------------------------------------------------------------------
// analyze_exchange_offer
// ---------------------------------------------------------------------------

/// Compare hold-vs-tender economics for a distressed exchange offer.
///
/// This is a lightweight, value-based comparator that operates directly
/// on dollar-valued inputs (no instrument construction required). It
/// computes the incremental tender value after consent fees and equity
/// sweeteners, plus a breakeven hold recovery implied by the tender
/// total.
///
/// Parameters
/// ----------
/// old_pv : float
///     Present value of the existing (old) claim if the holder does not
///     tender. Typically ``par * market_price``.
/// new_pv : float
///     Present value of the new instrument received in the exchange
///     (excluding consent fee and equity sweetener).
/// consent_fee : float
///     Cash consent / early-tender fee paid to participating holders.
/// equity_sweetener_value : float
///     Estimated value of any attached equity, warrants, or rights.
/// exchange_type : str, optional
///     One of ``"par_for_par"``, ``"discount"``, ``"uptier"``,
///     ``"downtier"`` - retained in the output for audit purposes; does
///     not change the arithmetic.
///
/// Returns
/// -------
/// dict
///     ``{"exchange_type", "old_npv", "new_npv", "consent_fee",
///     "equity_sweetener_value", "tender_total", "delta_npv",
///     "breakeven_recovery", "tender_recommended"}``. ``tender_total``
///     is ``new_pv + consent_fee + equity_sweetener_value``;
///     ``delta_npv`` is ``tender_total - old_pv``;
///     ``breakeven_recovery`` is the hold recovery rate at which
///     ``old_pv`` would equal ``tender_total`` (clamped to [0, 1]);
///     ``tender_recommended`` is ``True`` when ``tender_total`` exceeds
///     ``old_pv`` by at least 2%.
///
/// Raises
/// ------
/// ValueError
///     If any monetary input is negative or ``exchange_type`` is unknown.
#[pyfunction]
#[pyo3(signature = (old_pv, new_pv, consent_fee=0.0, equity_sweetener_value=0.0, exchange_type="par_for_par"))]
pub fn analyze_exchange_offer<'py>(
    py: Python<'py>,
    old_pv: f64,
    new_pv: f64,
    consent_fee: f64,
    equity_sweetener_value: f64,
    exchange_type: &str,
) -> PyResult<Bound<'py, PyDict>> {
    if old_pv < 0.0 || new_pv < 0.0 || consent_fee < 0.0 || equity_sweetener_value < 0.0 {
        return Err(PyValueError::new_err(
            "old_pv, new_pv, consent_fee, equity_sweetener_value must be non-negative",
        ));
    }

    // Canonicalize and validate exchange_type.
    let normalized = exchange_type.to_ascii_lowercase();
    let canonical = match normalized.as_str() {
        "par_for_par" | "par" | "parforpar" => "par_for_par",
        "discount" => "discount",
        "uptier" => "uptier",
        "downtier" => "downtier",
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown exchange_type '{other}' (expected par_for_par, discount, uptier, downtier)"
            )));
        }
    };

    let tender_total = new_pv + consent_fee + equity_sweetener_value;
    let delta_npv = tender_total - old_pv;

    // Breakeven hold recovery: rate at which old_pv would equal tender_total
    // assuming old_pv scales linearly with recovery. If old_pv is zero we
    // have no meaningful breakeven, so report 1.0 (always prefer tender).
    let breakeven_recovery = if old_pv > 0.0 {
        (tender_total / old_pv).clamp(0.0, 1.0)
    } else {
        1.0
    };

    // 2% threshold for significance, matching the Rust exchange_offer logic.
    let tender_recommended = tender_total > old_pv * 1.02;

    let out = PyDict::new(py);
    out.set_item("exchange_type", canonical)?;
    out.set_item("old_npv", old_pv)?;
    out.set_item("new_npv", new_pv)?;
    out.set_item("consent_fee", consent_fee)?;
    out.set_item("equity_sweetener_value", equity_sweetener_value)?;
    out.set_item("tender_total", tender_total)?;
    out.set_item("delta_npv", delta_npv)?;
    out.set_item("breakeven_recovery", breakeven_recovery)?;
    out.set_item("tender_recommended", tender_recommended)?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// analyze_lme
// ---------------------------------------------------------------------------

/// Analyze a liability management exercise (LME).
///
/// Computes debt reduction, cash cost, discount capture, and leverage
/// impact for open-market repurchases, tender offers, amend-and-extend,
/// and dropdown transactions.
///
/// Parameters
/// ----------
/// lme_type : str
///     One of:
///
///     * ``"open_market"`` / ``"open_market_repurchase"`` - buy bonds
///       back at ``repurchase_price_pct`` of par from ``opt_acceptance_pct``
///       of the outstanding stack.
///     * ``"tender_offer"`` - cash tender; ``repurchase_price_pct`` is a
///       single blended price applied to ``opt_acceptance_pct`` of
///       ``notional``.
///     * ``"amend_and_extend"`` / ``"ae"`` - no par retired;
///       ``repurchase_price_pct`` is interpreted as the extension fee
///       (fraction of par) paid to ``opt_acceptance_pct`` of holders.
///     * ``"dropdown"`` - asset transfer; ``repurchase_price_pct`` is
///       the transferred asset value as a fraction of ``notional``.
/// notional : float
///     Outstanding notional of the target instrument.
/// repurchase_price_pct : float
///     For repurchases/tenders: price as fraction of par (``0.60`` =
///     60 cents). For A&E: extension fee as fraction of par. For
///     dropdown: transferred-asset value as fraction of notional. Must
///     be in ``(0.0, 1.5]`` for price-based variants.
/// opt_acceptance_pct : float, optional
///     Fraction of holders participating (0.0 - 1.0). Defaults to 1.0.
/// ebitda : float, optional
///     If provided (and > 0), a ``leverage_impact`` block is returned
///     with pre/post leverage turns and the reduction.
///
/// Returns
/// -------
/// dict
///     ``{"lme_type", "cost", "notional_reduction", "discount_capture",
///     "discount_capture_pct", "leverage_impact",
///     "remaining_holder_impact_pct"}``. ``leverage_impact`` is ``None``
///     unless ``ebitda`` is supplied. ``remaining_holder_impact_pct`` is
///     the fraction of notional impaired for non-participants (only
///     meaningful for ``dropdown``; zero otherwise).
///
/// Raises
/// ------
/// ValueError
///     If prices are out of range, notional is non-positive, or the
///     acceptance rate is outside ``[0.0, 1.0]``.
#[pyfunction]
#[pyo3(signature = (lme_type, notional, repurchase_price_pct, opt_acceptance_pct=1.0, ebitda=None))]
pub fn analyze_lme<'py>(
    py: Python<'py>,
    lme_type: &str,
    notional: f64,
    repurchase_price_pct: f64,
    opt_acceptance_pct: f64,
    ebitda: Option<f64>,
) -> PyResult<Bound<'py, PyDict>> {
    if notional <= 0.0 {
        return Err(PyValueError::new_err(format!(
            "notional must be positive, got {notional}"
        )));
    }
    if !(0.0..=1.0).contains(&opt_acceptance_pct) {
        return Err(PyValueError::new_err(format!(
            "opt_acceptance_pct must be in [0.0, 1.0], got {opt_acceptance_pct}"
        )));
    }

    let kind = lme_type.to_ascii_lowercase();
    let participating = notional * opt_acceptance_pct;

    let (canonical, cost, par_retired, remaining_holder_pct) = match kind.as_str() {
        "open_market" | "open_market_repurchase" | "omr" => {
            if repurchase_price_pct <= 0.0 || repurchase_price_pct > 1.5 {
                return Err(PyValueError::new_err(format!(
                    "repurchase_price_pct must be in (0.0, 1.5], got {repurchase_price_pct}"
                )));
            }
            let par = participating;
            let cost = par * repurchase_price_pct;
            ("open_market_repurchase", cost, par, 0.0)
        }
        "tender_offer" | "tender" => {
            if repurchase_price_pct <= 0.0 || repurchase_price_pct > 1.5 {
                return Err(PyValueError::new_err(format!(
                    "repurchase_price_pct must be in (0.0, 1.5], got {repurchase_price_pct}"
                )));
            }
            let par = participating;
            let cost = par * repurchase_price_pct;
            ("tender_offer", cost, par, 0.0)
        }
        "amend_and_extend" | "ae" | "a&e" => {
            if !(0.0..=0.10).contains(&repurchase_price_pct) {
                return Err(PyValueError::new_err(format!(
                    "extension_fee (as fraction of par) should be in [0.0, 0.10], got {repurchase_price_pct}"
                )));
            }
            let cost = participating * repurchase_price_pct;
            ("amend_and_extend", cost, 0.0, 0.0)
        }
        "dropdown" => {
            if !(0.0..=1.0).contains(&repurchase_price_pct) {
                return Err(PyValueError::new_err(format!(
                    "dropdown transferred-asset fraction must be in [0.0, 1.0], got {repurchase_price_pct}"
                )));
            }
            // Remaining-holder impact is the full transferred-asset fraction:
            // collateral leaves the restricted group entirely, regardless of
            // the acceptance rate of any accompanying exchange.
            ("dropdown", 0.0, 0.0, repurchase_price_pct)
        }
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown lme_type '{other}' (expected open_market, tender_offer, amend_and_extend, dropdown)"
            )));
        }
    };

    let discount_capture = par_retired - cost;
    let discount_capture_pct = if par_retired > 0.0 {
        discount_capture / par_retired
    } else {
        0.0
    };

    let out = PyDict::new(py);
    out.set_item("lme_type", canonical)?;
    out.set_item("cost", cost)?;
    out.set_item("notional_reduction", par_retired)?;
    out.set_item("discount_capture", discount_capture)?;
    out.set_item("discount_capture_pct", discount_capture_pct)?;
    out.set_item("remaining_holder_impact_pct", remaining_holder_pct)?;

    match ebitda {
        Some(e) if e > 0.0 => {
            let pre_debt = notional;
            let post_debt = notional - par_retired;
            let pre_leverage = pre_debt / e;
            let post_leverage = post_debt / e;
            let lev = PyDict::new(py);
            lev.set_item("pre_total_debt", pre_debt)?;
            lev.set_item("post_total_debt", post_debt)?;
            lev.set_item("pre_leverage", pre_leverage)?;
            lev.set_item("post_leverage", post_leverage)?;
            lev.set_item("leverage_reduction", pre_leverage - post_leverage)?;
            out.set_item("leverage_impact", lev)?;
        }
        _ => {
            out.set_item("leverage_impact", py.None())?;
        }
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Register restructuring functions on the valuations submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(execute_recovery_waterfall, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(analyze_exchange_offer, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(analyze_lme, m)?)?;
    Ok(())
}
