//! Exchange offer analysis and hold-vs-tender economics.
//!
//! Models distressed exchange offers including par-for-par,
//! discount, uptier, and downtier transactions. Provides
//! hold-vs-tender NPV comparison to guide creditor decisions.
//!
//! # Overview
//!
//! An exchange offer replaces existing debt instruments with new
//! instruments under different terms. This module computes the
//! economics of both scenarios (hold the old instrument vs tender
//! for the new one) and identifies the breakeven recovery rate at
//! which the holder is indifferent between the two actions.
//!
//! # Model
//!
//! The holder is faced with two outcomes under the hold scenario:
//!
//! 1. The issuer does **not** default before the hold instrument's
//!    maturity and the holder receives scheduled coupons plus
//!    principal (discounted at the hold-scenario rate).
//! 2. The issuer defaults, and the holder recovers
//!    `hold_recovery_rate * par`.
//!
//! Letting `p` be the hold default probability, `NPV_no_default` the
//! scheduled cash-flow PV, `R` the recovery, and `par` the par amount:
//!
//! ```text
//! hold_npv = (1 - p) * NPV_no_default + p * par * R
//! ```
//!
//! The tender leg receives the new instrument's PV (under its own
//! discount rate) plus consent fee and equity sweetener.
//!
//! The breakeven recovery is the `R*` that makes `hold_npv` equal to
//! the tender total.
//!
//! # References
//!
//! - Moyer, *Distressed Debt Analysis* (2004), Ch. 15-16

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::money::Money;
use serde::{Deserialize, Serialize};

use super::error::RestructuringError;
use super::types::ClaimSeniority;

/// Type of exchange offer in a restructuring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExchangeType {
    /// Par-for-par: old instrument exchanged at par for new instrument at par.
    /// Economics differ through coupon, maturity, or covenant changes.
    ParForPar,
    /// Discount exchange: new instrument issued at a discount to old par.
    /// May include equity sweetener to incentivize participation.
    Discount {
        /// Exchange ratio (e.g., 0.70 means $70 new per $100 old).
        exchange_ratio: f64,
    },
    /// Uptier exchange: existing claim moves to higher priority (e.g.,
    /// unsecured bonds exchanged into new secured notes). Controversial;
    /// requires analysis of non-participating creditor impact.
    Uptier {
        /// New seniority for exchanged claims.
        new_seniority: ClaimSeniority,
        /// Exchange ratio.
        exchange_ratio: f64,
    },
    /// Downtier: claim moves to lower priority (rare; typically forced
    /// via amendment). Models the impact on the downtiered holder.
    Downtier {
        /// New seniority for exchanged claims.
        new_seniority: ClaimSeniority,
        /// Exchange ratio.
        exchange_ratio: f64,
    },
}

/// Specification for an exchange offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeOffer {
    /// Unique identifier.
    pub id: String,
    /// Type of exchange.
    pub exchange_type: ExchangeType,
    /// Old instrument being exchanged.
    pub old_instrument: ExchangeInstrument,
    /// New instrument being received.
    pub new_instrument: ExchangeInstrument,
    /// Optional equity sweetener (shares or warrants per $1,000 face).
    pub equity_sweetener: Option<EquitySweetener>,
    /// Consent fee paid to early tenders (in basis points of par).
    pub consent_fee_bps: Option<f64>,
    /// Early tender deadline (holders tendering before this get better terms).
    pub early_tender_deadline: Option<Date>,
    /// Final expiration date.
    pub expiration_date: Date,
    /// Required consent threshold as fraction (e.g., 0.50 for majority,
    /// 0.6667 for supermajority, 0.90 for exit consent).
    pub required_consent_threshold: f64,
    /// Minimum participation threshold (offer void if not met).
    pub minimum_participation: Option<f64>,
}

/// Instrument description within an exchange offer context.
///
/// Simplified representation capturing the economics relevant to
/// hold-vs-tender analysis. Not a full `Instrument` spec -- the
/// exchange offer module only needs coupon, maturity, and priority
/// to compute comparative economics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeInstrument {
    /// Reference to full instrument if it exists in the portfolio.
    pub instrument_id: Option<String>,
    /// Face / par amount per unit.
    pub par_amount: Money,
    /// Annual coupon rate.
    pub coupon_rate: f64,
    /// Whether coupon is cash-pay, PIK, or split.
    pub coupon_type: CouponPaymentType,
    /// Maturity date.
    pub maturity: Date,
    /// Seniority / priority.
    pub seniority: ClaimSeniority,
    /// Current market price (as fraction of par, e.g., 0.65 = 65 cents on dollar).
    pub market_price: Option<f64>,
}

/// Coupon payment mode in restructured instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CouponPaymentType {
    /// Full cash pay.
    CashPay,
    /// Payment in kind (capitalizing interest).
    Pik,
    /// Split between cash and PIK.
    SplitPay {
        /// Fraction paid in cash (percentage 0-100; remainder is PIK).
        cash_fraction: u8,
    },
}

/// Equity sweetener attached to an exchange offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquitySweetener {
    /// Type of equity component.
    pub equity_type: EquityComponentType,
    /// Number of shares or warrants per $1,000 face exchanged.
    pub units_per_1000: f64,
    /// Estimated value per unit (for hold-vs-tender comparison).
    pub estimated_value_per_unit: Money,
}

/// Type of equity component in a restructuring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquityComponentType {
    /// Common shares of reorganized entity.
    CommonShares,
    /// Warrants with strike and expiry.
    Warrants,
    /// Convertible instrument.
    Convertible,
    /// Rights offering participation.
    Rights,
}

/// Economics comparison for a single holder deciding hold vs tender.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldVsTenderAnalysis {
    /// Analysis for the hold scenario.
    pub hold: ScenarioEconomics,
    /// Analysis for the tender scenario.
    pub tender: ScenarioEconomics,
    /// Breakeven recovery rate: hold recovery at which hold NPV equals
    /// tender total.
    ///
    /// `Some(r)` when `hold_default_probability > 0` so a real breakeven
    /// exists; clamped to `[0.0, 1.0]`. `None` when no default is
    /// considered (`hold_default_probability == 0`) since in that case
    /// the hold NPV does not depend on recovery and no breakeven is
    /// defined.
    pub breakeven_recovery: Option<f64>,
    /// Recommendation based on provided assumptions.
    pub recommendation: TenderRecommendation,
}

/// Economics of one scenario (hold or tender) in the exchange analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioEconomics {
    /// Par amount of claim.
    pub par_amount: Money,
    /// Estimated recovery or new instrument value.
    pub recovery_value: Money,
    /// Present value of future cash flows (coupon stream + principal).
    pub npv: Money,
    /// Yield to maturity or yield to worst.
    pub yield_metric: f64,
    /// Duration / weighted average life.
    pub wal_years: f64,
}

/// Tender recommendation output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenderRecommendation {
    /// Tender: new instrument economics dominate.
    Tender,
    /// Hold: existing claim economics dominate.
    Hold,
    /// Indifferent: economics are approximately equivalent.
    Indifferent,
}

/// Track consent accumulation against threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentTracker {
    /// Total outstanding amount eligible to vote.
    pub total_outstanding: Money,
    /// Amount that has consented.
    pub consented_amount: Money,
    /// Required threshold (fraction).
    pub required_threshold: f64,
}

impl ConsentTracker {
    /// Current participation as fraction.
    pub fn participation_rate(&self) -> f64 {
        let total = self.total_outstanding.amount();
        if total <= 0.0 {
            return 0.0;
        }
        self.consented_amount.amount() / total
    }

    /// Whether the required consent threshold has been met.
    pub fn threshold_met(&self) -> bool {
        self.participation_rate() >= self.required_threshold
    }

    /// Remaining amount needed to reach threshold.
    pub fn remaining_needed(&self) -> Money {
        let target = self.total_outstanding.amount() * self.required_threshold;
        let remaining = (target - self.consented_amount.amount()).max(0.0);
        Money::new(remaining, self.total_outstanding.currency())
    }
}

/// Inputs for `analyze_exchange_offer`.
///
/// Keeping the full input set in a struct avoids positional-argument
/// confusion and lets the hold and tender legs carry their own discount
/// rates. The breakeven-recovery computation is only well-defined when
/// a non-zero hold default probability is supplied.
#[derive(Debug, Clone, Copy)]
pub struct ExchangeOfferAnalysisInputs {
    /// Valuation date used to compute years-to-maturity for both legs.
    pub as_of: Date,
    /// Discount rate applied to hold-scenario cash flows.
    ///
    /// Typically the holder's required yield on the existing claim at
    /// the current risk level (i.e. the pre-exchange yield).
    pub hold_discount_rate: f64,
    /// Discount rate applied to tender-scenario cash flows.
    ///
    /// Typically tighter than `hold_discount_rate` for an uptier
    /// exchange (new instrument has better priority) and wider for a
    /// downtier exchange. Equal to `hold_discount_rate` collapses to a
    /// single-rate analysis.
    pub tender_discount_rate: f64,
    /// Assumed recovery if the holder does not tender and the issuer
    /// subsequently defaults, as fraction of par (0.0 - 1.0).
    pub hold_recovery_rate: f64,
    /// Probability that the issuer defaults before the hold
    /// instrument's maturity (0.0 - 1.0). `0.0` disables the default
    /// branch; the breakeven recovery is then undefined and returned as
    /// `None`.
    pub hold_default_probability: f64,
}

/// Analyze hold-vs-tender economics for an exchange offer.
///
/// # Model
///
/// The hold scenario is modeled as a weighted combination of the
/// no-default path (scheduled coupons and principal discounted at
/// `hold_discount_rate`) and the default path (recovery times par):
///
/// ```text
/// hold_npv = (1 - p) * NPV_no_default + p * par * R
/// ```
///
/// The tender scenario discounts the new instrument's cash flows at
/// `tender_discount_rate` and adds any consent fee and equity
/// sweetener value. Breakeven recovery `R*` is the value that makes
/// `hold_npv == tender_total`, if it exists.
///
/// # Errors
///
/// Returns `RestructuringError` for:
/// - Exchange ratio outside `(0.0, 2.0]`.
/// - Negative coupon rates.
/// - Non-positive discount rates.
/// - Recovery or default probability outside `[0.0, 1.0]`.
/// - Currency mismatch between old and new par amounts.
/// - PIK / split-pay coupons, which the current flat-rate NPV engine
///   does not model.
pub fn analyze_exchange_offer(
    offer: &ExchangeOffer,
    inputs: &ExchangeOfferAnalysisInputs,
) -> crate::Result<HoldVsTenderAnalysis> {
    validate_exchange_offer(offer, inputs)?;

    let as_of = inputs.as_of;
    let currency = offer.old_instrument.par_amount.currency();
    let par = offer.old_instrument.par_amount.amount();

    // Hold scenario: combine no-default NPV with recovery under default.
    let hold_npv_no_default =
        compute_instrument_npv(&offer.old_instrument, as_of, inputs.hold_discount_rate)?;
    let p_default = inputs.hold_default_probability;
    let recovery_path = par * inputs.hold_recovery_rate;
    let hold_npv_amt = (1.0 - p_default) * hold_npv_no_default + p_default * recovery_path;

    let hold_recovery_value = Money::new(recovery_path, currency);
    let hold_wal = estimate_wal(&offer.old_instrument, as_of)?;

    // Tender scenario: NPV of new instrument + equity sweetener + consent fee.
    let exchange_ratio = exchange_ratio_of(&offer.exchange_type);
    let new_par_amt = par * exchange_ratio;
    let new_par = Money::new(new_par_amt, offer.new_instrument.par_amount.currency());

    let tender_npv_amt = compute_scaled_npv(
        &offer.new_instrument,
        as_of,
        inputs.tender_discount_rate,
        new_par_amt,
    )?;

    let sweetener_value = offer.equity_sweetener.as_ref().map_or(0.0, |sw| {
        // Sweetener is sized per $1,000 of **old** par exchanged.
        sw.units_per_1000 * sw.estimated_value_per_unit.amount() * (par / 1000.0)
    });

    // Consent fee is paid on old par, not new.
    let consent_value = offer.consent_fee_bps.unwrap_or(0.0) / 10_000.0 * par;

    let tender_wal = estimate_wal(&offer.new_instrument, as_of)?;

    let hold = ScenarioEconomics {
        par_amount: offer.old_instrument.par_amount,
        recovery_value: hold_recovery_value,
        npv: Money::new(hold_npv_amt, currency),
        yield_metric: inputs.hold_discount_rate,
        wal_years: hold_wal,
    };

    let tender_total = tender_npv_amt + sweetener_value + consent_value;
    let tender = ScenarioEconomics {
        par_amount: new_par,
        recovery_value: Money::new(tender_total, currency),
        npv: Money::new(tender_total, currency),
        yield_metric: inputs.tender_discount_rate,
        wal_years: tender_wal,
    };

    // Breakeven recovery: R* solving (1 - p) * NPV_nd + p * par * R* = tender_total.
    // Only defined when there is a default branch (p > 0) and positive par.
    let breakeven_recovery = if p_default > 0.0 && par > 0.0 {
        let numer = tender_total - (1.0 - p_default) * hold_npv_no_default;
        let denom = p_default * par;
        Some((numer / denom).clamp(0.0, 1.0))
    } else {
        None
    };

    // Recommendation: 2% threshold for significance, using the hold NPV
    // that already reflects the default-weighted recovery branch.
    let recommendation = if tender_total > hold_npv_amt * 1.02 {
        TenderRecommendation::Tender
    } else if hold_npv_amt > tender_total * 1.02 {
        TenderRecommendation::Hold
    } else {
        TenderRecommendation::Indifferent
    };

    Ok(HoldVsTenderAnalysis {
        hold,
        tender,
        breakeven_recovery,
        recommendation,
    })
}

// ─── Helpers ──────────────────────────────────────────────────────

fn exchange_ratio_of(exchange_type: &ExchangeType) -> f64 {
    match exchange_type {
        ExchangeType::ParForPar => 1.0,
        ExchangeType::Discount { exchange_ratio }
        | ExchangeType::Uptier { exchange_ratio, .. }
        | ExchangeType::Downtier { exchange_ratio, .. } => *exchange_ratio,
    }
}

fn validate_exchange_offer(
    offer: &ExchangeOffer,
    inputs: &ExchangeOfferAnalysisInputs,
) -> crate::Result<()> {
    let ratio = exchange_ratio_of(&offer.exchange_type);
    if ratio <= 0.0 || ratio > 2.0 {
        return Err(RestructuringError::InvalidExchangeRatio { ratio }.into());
    }

    // Validate coupon rates.
    if offer.old_instrument.coupon_rate < 0.0 {
        return Err(RestructuringError::NegativeCouponRate {
            rate: offer.old_instrument.coupon_rate,
        }
        .into());
    }
    if offer.new_instrument.coupon_rate < 0.0 {
        return Err(RestructuringError::NegativeCouponRate {
            rate: offer.new_instrument.coupon_rate,
        }
        .into());
    }

    // Reject coupon types this flat-rate NPV engine does not model. PIK
    // and split-pay require simulating interest capitalization and a
    // partial-cash coupon schedule, neither of which is captured here.
    reject_unsupported_coupon(offer.old_instrument.coupon_type, "old_instrument")?;
    reject_unsupported_coupon(offer.new_instrument.coupon_type, "new_instrument")?;

    if inputs.hold_discount_rate <= 0.0 {
        return Err(RestructuringError::InvalidDiscountRate {
            rate: inputs.hold_discount_rate,
        }
        .into());
    }
    if inputs.tender_discount_rate <= 0.0 {
        return Err(RestructuringError::InvalidDiscountRate {
            rate: inputs.tender_discount_rate,
        }
        .into());
    }

    if !(0.0..=1.0).contains(&inputs.hold_recovery_rate) {
        return Err(RestructuringError::InvalidRecoveryRate {
            rate: inputs.hold_recovery_rate,
        }
        .into());
    }
    if !(0.0..=1.0).contains(&inputs.hold_default_probability) {
        return Err(RestructuringError::Validation {
            message: format!(
                "hold_default_probability {} outside [0.0, 1.0]",
                inputs.hold_default_probability
            ),
        }
        .into());
    }

    if offer.old_instrument.par_amount.currency() != offer.new_instrument.par_amount.currency() {
        return Err(RestructuringError::CurrencyMismatch {
            expected: format!("{:?}", offer.old_instrument.par_amount.currency()),
            actual: format!("{:?}", offer.new_instrument.par_amount.currency()),
            claim_id: offer.id.clone(),
        }
        .into());
    }

    Ok(())
}

fn reject_unsupported_coupon(coupon: CouponPaymentType, which: &str) -> crate::Result<()> {
    match coupon {
        CouponPaymentType::CashPay => Ok(()),
        CouponPaymentType::Pik | CouponPaymentType::SplitPay { .. } => {
            Err(RestructuringError::Validation {
                message: format!(
                    "{which} has coupon type {coupon:?}: PIK and split-pay coupons \
                     are not supported by analyze_exchange_offer (flat-rate cash \
                     coupon engine). Convert to a cash-equivalent coupon rate or \
                     price with a full cashflow engine."
                ),
            }
            .into())
        }
    }
}

/// Compute a simple NPV for an instrument using a flat discount rate.
///
/// Cash flows:
/// - Annual coupons of `coupon_rate * par` at each full year `1, 2, ..., n`
///   where `n = floor(years_to_maturity)`.
/// - A stub coupon proportional to the residual year fraction plus the
///   principal at exact maturity.
///
/// Years are measured on Act/365F from `as_of` to `instrument.maturity`.
/// This keeps the NPV accurate for short or odd-dated claims without
/// requiring a full coupon schedule.
fn compute_instrument_npv(
    instrument: &ExchangeInstrument,
    as_of: Date,
    discount_rate: f64,
) -> crate::Result<f64> {
    let par = instrument.par_amount.amount();
    if par <= 0.0 {
        return Ok(0.0);
    }
    let coupon = par * instrument.coupon_rate;
    let years = years_to_maturity(instrument, as_of)?;

    if years <= 0.0 {
        // Already matured / negative time: redemption at par.
        return Ok(par);
    }

    let r = discount_rate;
    let mut pv = 0.0;

    // Full-year coupons.
    let n_full = years.floor() as u32;
    for t in 1..=n_full {
        pv += coupon / (1.0 + r).powi(t as i32);
    }

    // Residual stub: from the last full-year anniversary to maturity.
    let stub = years - n_full as f64;
    let df_maturity = 1.0 / (1.0 + r).powf(years);
    pv += coupon * stub * df_maturity;

    // Principal at maturity.
    pv += par * df_maturity;

    Ok(pv)
}

/// Compute NPV scaled to a different par amount (for exchange ratio adjustment).
fn compute_scaled_npv(
    instrument: &ExchangeInstrument,
    as_of: Date,
    discount_rate: f64,
    scaled_par: f64,
) -> crate::Result<f64> {
    let original_par = instrument.par_amount.amount();
    if original_par <= 0.0 {
        return Ok(0.0);
    }
    let base_npv = compute_instrument_npv(instrument, as_of, discount_rate)?;
    Ok(base_npv * (scaled_par / original_par))
}

/// Time-to-maturity in years on an Act/365F basis.
fn years_to_maturity(instrument: &ExchangeInstrument, as_of: Date) -> crate::Result<f64> {
    DayCount::Act365F
        .year_fraction(as_of, instrument.maturity, DayCountCtx::default())
        .map_err(Into::into)
}

/// Estimate weighted average life (WAL) in years from `as_of`.
///
/// For a bullet bond this collapses to time-to-maturity on Act/365F.
/// Floored at `0.5` years to keep the downstream NPV calculation from
/// collapsing to zero for near-matured claims.
fn estimate_wal(instrument: &ExchangeInstrument, as_of: Date) -> crate::Result<f64> {
    Ok(years_to_maturity(instrument, as_of)?.max(0.5))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    fn usd(amount: f64) -> Money {
        Money::new(amount, Currency::USD)
    }

    fn make_date(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).expect("valid date")
    }

    fn sample_offer(exchange_type: ExchangeType) -> ExchangeOffer {
        ExchangeOffer {
            id: "test-offer".into(),
            exchange_type,
            old_instrument: ExchangeInstrument {
                instrument_id: None,
                par_amount: usd(1000.0),
                coupon_rate: 0.085,
                coupon_type: CouponPaymentType::CashPay,
                maturity: make_date(2029, Month::January, 15),
                seniority: ClaimSeniority::SeniorUnsecured,
                market_price: Some(0.65),
            },
            new_instrument: ExchangeInstrument {
                instrument_id: None,
                par_amount: usd(1000.0),
                coupon_rate: 0.10,
                coupon_type: CouponPaymentType::CashPay,
                maturity: make_date(2031, Month::January, 15),
                seniority: ClaimSeniority::FirstLienSecured,
                market_price: None,
            },
            equity_sweetener: None,
            consent_fee_bps: Some(50.0),
            early_tender_deadline: None,
            expiration_date: make_date(2026, Month::June, 30),
            required_consent_threshold: 0.6667,
            minimum_participation: Some(0.50),
        }
    }

    fn as_of() -> Date {
        make_date(2026, Month::January, 15)
    }

    fn default_inputs() -> ExchangeOfferAnalysisInputs {
        ExchangeOfferAnalysisInputs {
            as_of: as_of(),
            hold_discount_rate: 0.12,
            tender_discount_rate: 0.10,
            hold_recovery_rate: 0.40,
            hold_default_probability: 0.20,
        }
    }

    #[test]
    fn par_for_par_exchange_economics() {
        let offer = sample_offer(ExchangeType::ParForPar);
        let result = analyze_exchange_offer(&offer, &default_inputs()).expect("should succeed");

        assert!(result.tender.npv.amount() > 0.0);
        assert!(result.hold.npv.amount() > 0.0);
        // With default probability > 0, a breakeven recovery is defined.
        let br = result.breakeven_recovery.expect("breakeven defined");
        assert!((0.0..=1.0).contains(&br));
    }

    #[test]
    fn discount_exchange_economics() {
        let offer = sample_offer(ExchangeType::Discount {
            exchange_ratio: 0.70,
        });
        let result = analyze_exchange_offer(&offer, &default_inputs()).expect("should succeed");

        assert!(
            (result.tender.par_amount.amount() - 700.0).abs() < 1e-6,
            "tender par should be 700, got {}",
            result.tender.par_amount.amount()
        );
    }

    #[test]
    fn consent_fee_adds_value() {
        let mut offer = sample_offer(ExchangeType::ParForPar);
        offer.consent_fee_bps = None;
        let without = analyze_exchange_offer(&offer, &default_inputs()).expect("should succeed");

        offer.consent_fee_bps = Some(100.0);
        let with = analyze_exchange_offer(&offer, &default_inputs()).expect("should succeed");

        assert!(
            with.tender.npv.amount() > without.tender.npv.amount(),
            "consent fee should increase tender NPV"
        );
    }

    #[test]
    fn equity_sweetener_adds_value() {
        let mut offer = sample_offer(ExchangeType::ParForPar);
        let without = analyze_exchange_offer(&offer, &default_inputs()).expect("should succeed");

        offer.equity_sweetener = Some(EquitySweetener {
            equity_type: EquityComponentType::CommonShares,
            units_per_1000: 10.0,
            estimated_value_per_unit: usd(5.0),
        });
        let with = analyze_exchange_offer(&offer, &default_inputs()).expect("should succeed");

        assert!(
            with.tender.npv.amount() > without.tender.npv.amount(),
            "sweetener should increase tender NPV"
        );
    }

    #[test]
    fn invalid_exchange_ratio_rejected() {
        let offer = sample_offer(ExchangeType::Discount {
            exchange_ratio: 0.0,
        });
        assert!(analyze_exchange_offer(&offer, &default_inputs()).is_err());

        let offer2 = sample_offer(ExchangeType::Discount {
            exchange_ratio: 3.0,
        });
        assert!(analyze_exchange_offer(&offer2, &default_inputs()).is_err());
    }

    #[test]
    fn negative_discount_rate_rejected() {
        let offer = sample_offer(ExchangeType::ParForPar);
        let mut inp = default_inputs();
        inp.hold_discount_rate = -0.05;
        assert!(analyze_exchange_offer(&offer, &inp).is_err());
    }

    #[test]
    fn pik_coupon_rejected() {
        let mut offer = sample_offer(ExchangeType::ParForPar);
        offer.old_instrument.coupon_type = CouponPaymentType::Pik;
        assert!(analyze_exchange_offer(&offer, &default_inputs()).is_err());

        let mut offer2 = sample_offer(ExchangeType::ParForPar);
        offer2.new_instrument.coupon_type = CouponPaymentType::SplitPay { cash_fraction: 50 };
        assert!(analyze_exchange_offer(&offer2, &default_inputs()).is_err());
    }

    #[test]
    fn breakeven_recovery_is_none_when_no_default_risk() {
        let offer = sample_offer(ExchangeType::ParForPar);
        let mut inp = default_inputs();
        inp.hold_default_probability = 0.0;
        let result = analyze_exchange_offer(&offer, &inp).expect("should succeed");
        assert!(result.breakeven_recovery.is_none());
    }

    #[test]
    fn breakeven_recovery_satisfies_indifference() {
        let offer = sample_offer(ExchangeType::ParForPar);
        let inp = default_inputs();
        let result = analyze_exchange_offer(&offer, &inp).expect("should succeed");
        let r_star = result
            .breakeven_recovery
            .expect("breakeven recovery defined for p > 0");

        // Recompute hold NPV at r_star and confirm it matches tender total,
        // modulo the 0-1 clamp. If the clamp activated, the math need not
        // satisfy indifference exactly but both sides must still obey the
        // inequality implied by the clamp.
        let par = offer.old_instrument.par_amount.amount();
        let hold_nd =
            compute_instrument_npv(&offer.old_instrument, inp.as_of, inp.hold_discount_rate)
                .expect("hold NPV");
        let p = inp.hold_default_probability;
        let hold_at_r_star = (1.0 - p) * hold_nd + p * par * r_star;
        let tender_total = result.tender.npv.amount();
        let diff = (hold_at_r_star - tender_total).abs();

        // Either indifference holds or the clamp is active.
        let clamp_active = (r_star - 0.0).abs() < 1e-12 || (r_star - 1.0).abs() < 1e-12;
        assert!(
            diff < 1e-6 || clamp_active,
            "|hold(R*) - tender| = {diff}, R* = {r_star}",
        );
    }

    #[test]
    fn separate_discount_rates_produce_distinct_npvs() {
        let offer = sample_offer(ExchangeType::ParForPar);
        let mut narrow = default_inputs();
        narrow.hold_discount_rate = 0.15;
        narrow.tender_discount_rate = 0.08;

        let mut wide = default_inputs();
        wide.hold_discount_rate = 0.08;
        wide.tender_discount_rate = 0.15;

        let narrow_res = analyze_exchange_offer(&offer, &narrow).expect("narrow");
        let wide_res = analyze_exchange_offer(&offer, &wide).expect("wide");

        // Tender NPV should be higher when tender_discount_rate is lower.
        assert!(narrow_res.tender.npv.amount() > wide_res.tender.npv.amount());
        // Hold NPV should be higher when hold_discount_rate is lower.
        assert!(wide_res.hold.npv.amount() > narrow_res.hold.npv.amount());
    }

    #[test]
    fn consent_tracker_threshold() {
        let tracker = ConsentTracker {
            total_outstanding: usd(100_000.0),
            consented_amount: usd(66_000.0),
            required_threshold: 0.6667,
        };
        assert!(!tracker.threshold_met());
        assert!((tracker.participation_rate() - 0.66).abs() < 0.01);

        let tracker2 = ConsentTracker {
            total_outstanding: usd(100_000.0),
            consented_amount: usd(67_000.0),
            required_threshold: 0.6667,
        };
        assert!(tracker2.threshold_met());
        assert!((tracker2.remaining_needed().amount()).abs() < 1.0);
    }
}
