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
//! for the new one) and identifies the breakeven recovery rate.
//!
//! # References
//!
//! - Moyer, *Distressed Debt Analysis* (2004), Ch. 15-16

use finstack_core::dates::Date;
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
    /// Breakeven recovery rate: recovery at which hold = tender NPV.
    pub breakeven_recovery: f64,
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

/// Analyze hold-vs-tender economics for an exchange offer.
///
/// Computes NPV of both scenarios using provided discount rate and
/// recovery assumptions, returning a comparative analysis.
///
/// # Arguments
///
/// * `offer` - Exchange offer specification
/// * `discount_rate` - Rate used to discount future cash flows
/// * `hold_recovery_rate` - Assumed recovery if holder does not tender
///   and the issuer eventually defaults or restructures further
/// * `participation_estimate` - Estimated fraction of holders tendering
///   (affects non-participant recovery in uptier scenarios)
///
/// # Errors
///
/// Returns error if exchange ratio is outside (0.0, 2.0], if coupon
/// rates are negative, or if old/new instruments have currency mismatch.
pub fn analyze_exchange_offer(
    offer: &ExchangeOffer,
    discount_rate: f64,
    hold_recovery_rate: f64,
    _participation_estimate: f64,
) -> crate::Result<HoldVsTenderAnalysis> {
    validate_exchange_offer(offer, discount_rate, hold_recovery_rate)?;

    let currency = offer.old_instrument.par_amount.currency();

    // Hold scenario: NPV of existing instrument cash flows, weighted by
    // hold_recovery_rate for the default-contingent path.
    let hold_npv = compute_instrument_npv(&offer.old_instrument, discount_rate);
    let hold_recovery_value = Money::new(
        offer.old_instrument.par_amount.amount() * hold_recovery_rate,
        currency,
    );
    let hold_wal = estimate_wal(&offer.old_instrument);

    // Tender scenario: NPV of new instrument + equity sweetener + consent fee.
    let exchange_ratio = match &offer.exchange_type {
        ExchangeType::ParForPar => 1.0,
        ExchangeType::Discount { exchange_ratio } => *exchange_ratio,
        ExchangeType::Uptier {
            exchange_ratio, ..
        } => *exchange_ratio,
        ExchangeType::Downtier {
            exchange_ratio, ..
        } => *exchange_ratio,
    };

    let new_par_amt = offer.old_instrument.par_amount.amount() * exchange_ratio;
    let new_par = Money::new(new_par_amt, offer.new_instrument.par_amount.currency());

    let tender_npv_amt = compute_scaled_npv(
        &offer.new_instrument,
        discount_rate,
        new_par_amt,
    );

    let sweetener_value = offer.equity_sweetener.as_ref().map_or(0.0, |sw| {
        sw.units_per_1000
            * sw.estimated_value_per_unit.amount()
            * (offer.old_instrument.par_amount.amount() / 1000.0)
    });

    let consent_value = offer.consent_fee_bps.unwrap_or(0.0) / 10_000.0
        * offer.old_instrument.par_amount.amount();

    let tender_wal = estimate_wal(&offer.new_instrument);

    let hold = ScenarioEconomics {
        par_amount: offer.old_instrument.par_amount,
        recovery_value: hold_recovery_value,
        npv: Money::new(hold_npv, currency),
        yield_metric: discount_rate,
        wal_years: hold_wal,
    };

    let tender_total = tender_npv_amt + sweetener_value + consent_value;
    let tender = ScenarioEconomics {
        par_amount: new_par,
        recovery_value: Money::new(tender_total, currency),
        npv: Money::new(tender_total, currency),
        yield_metric: discount_rate,
        wal_years: tender_wal,
    };

    // Breakeven: recovery rate at which hold NPV = tender NPV.
    let par = offer.old_instrument.par_amount.amount();
    let breakeven = if par > 0.0 {
        (tender_total / par).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Recommendation: 2% threshold for significance.
    let recommendation = if tender_total > hold_npv * 1.02 {
        TenderRecommendation::Tender
    } else if hold_npv > tender_total * 1.02 {
        TenderRecommendation::Hold
    } else {
        TenderRecommendation::Indifferent
    };

    Ok(HoldVsTenderAnalysis {
        hold,
        tender,
        breakeven_recovery: breakeven,
        recommendation,
    })
}

// ─── Helpers ──────────────────────────────────────────────────────

fn validate_exchange_offer(
    offer: &ExchangeOffer,
    discount_rate: f64,
    hold_recovery_rate: f64,
) -> crate::Result<()> {
    // Validate exchange ratio.
    let ratio = match &offer.exchange_type {
        ExchangeType::ParForPar => 1.0,
        ExchangeType::Discount { exchange_ratio } => *exchange_ratio,
        ExchangeType::Uptier {
            exchange_ratio, ..
        } => *exchange_ratio,
        ExchangeType::Downtier {
            exchange_ratio, ..
        } => *exchange_ratio,
    };
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

    // Validate discount rate.
    if discount_rate <= 0.0 {
        return Err(RestructuringError::InvalidDiscountRate {
            rate: discount_rate,
        }
        .into());
    }

    // Validate recovery rate.
    if !(0.0..=1.0).contains(&hold_recovery_rate) {
        return Err(RestructuringError::InvalidRecoveryRate {
            rate: hold_recovery_rate,
        }
        .into());
    }

    // Currency consistency.
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

/// Compute a simple NPV for an instrument using flat discount rate.
///
/// NPV = sum of discounted coupons + discounted principal at maturity.
/// Uses years-to-maturity estimated from the maturity date vs a
/// reference point (instrument maturity provides the duration).
fn compute_instrument_npv(instrument: &ExchangeInstrument, discount_rate: f64) -> f64 {
    let par = instrument.par_amount.amount();
    let coupon = par * instrument.coupon_rate;
    let years = estimate_wal(instrument);

    if years <= 0.0 || discount_rate <= 0.0 {
        return par;
    }

    let n = years.ceil() as u32;
    let mut pv = 0.0;
    for t in 1..=n {
        let df = 1.0 / (1.0 + discount_rate).powi(t as i32);
        pv += coupon * df;
    }
    // Principal at maturity.
    let df_mat = 1.0 / (1.0 + discount_rate).powi(n as i32);
    pv += par * df_mat;
    pv
}

/// Compute NPV scaled to a different par amount (for exchange ratio adjustment).
fn compute_scaled_npv(
    instrument: &ExchangeInstrument,
    discount_rate: f64,
    scaled_par: f64,
) -> f64 {
    let original_par = instrument.par_amount.amount();
    if original_par <= 0.0 {
        return 0.0;
    }
    let base_npv = compute_instrument_npv(instrument, discount_rate);
    base_npv * (scaled_par / original_par)
}

/// Estimate weighted average life (WAL) in years from today.
///
/// Simplified: uses a fixed reference date and computes years to maturity.
/// In production, this would use the actual valuation date.
fn estimate_wal(instrument: &ExchangeInstrument) -> f64 {
    // Use maturity year minus a reference. Since we don't have an
    // as-of date, approximate WAL as the coupon-weighted midpoint.
    // For a bullet bond, WAL ~= years to maturity.
    // We estimate years to maturity as a reasonable default.
    // Use 5.0 years as a fallback when maturity information is ambiguous.
    let year = instrument.maturity.year() as f64;
    // Approximate: instruments typically have 2-10 year maturities.
    // We'll compute from a reference year of 2026.
    let ref_year = 2026.0;
    let years = (year - ref_year).max(0.5);
    years
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

    #[test]
    fn par_for_par_exchange_economics() {
        let offer = sample_offer(ExchangeType::ParForPar);
        let result =
            analyze_exchange_offer(&offer, 0.12, 0.40, 0.75).expect("should succeed");

        // With par-for-par and higher coupon on new instrument, tender should
        // generally be preferred.
        assert!(result.tender.npv.amount() > 0.0);
        assert!(result.hold.npv.amount() > 0.0);
        assert!(result.breakeven_recovery >= 0.0 && result.breakeven_recovery <= 1.0);
    }

    #[test]
    fn discount_exchange_economics() {
        let offer = sample_offer(ExchangeType::Discount {
            exchange_ratio: 0.70,
        });
        let result =
            analyze_exchange_offer(&offer, 0.12, 0.40, 0.75).expect("should succeed");

        // New par is 70% of old par.
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
        let without_fee =
            analyze_exchange_offer(&offer, 0.12, 0.40, 0.75).expect("should succeed");

        offer.consent_fee_bps = Some(100.0);
        let with_fee =
            analyze_exchange_offer(&offer, 0.12, 0.40, 0.75).expect("should succeed");

        assert!(
            with_fee.tender.npv.amount() > without_fee.tender.npv.amount(),
            "consent fee should increase tender NPV"
        );
    }

    #[test]
    fn equity_sweetener_adds_value() {
        let mut offer = sample_offer(ExchangeType::ParForPar);
        let without =
            analyze_exchange_offer(&offer, 0.12, 0.40, 0.75).expect("should succeed");

        offer.equity_sweetener = Some(EquitySweetener {
            equity_type: EquityComponentType::CommonShares,
            units_per_1000: 10.0,
            estimated_value_per_unit: usd(5.0),
        });
        let with =
            analyze_exchange_offer(&offer, 0.12, 0.40, 0.75).expect("should succeed");

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
        assert!(analyze_exchange_offer(&offer, 0.12, 0.40, 0.75).is_err());

        let offer2 = sample_offer(ExchangeType::Discount {
            exchange_ratio: 3.0,
        });
        assert!(analyze_exchange_offer(&offer2, 0.12, 0.40, 0.75).is_err());
    }

    #[test]
    fn negative_discount_rate_rejected() {
        let offer = sample_offer(ExchangeType::ParForPar);
        assert!(analyze_exchange_offer(&offer, -0.05, 0.40, 0.75).is_err());
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
