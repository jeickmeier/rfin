//! Liability management exercise (LME) analysis.
//!
//! Models open market repurchases, tender offers, amend-and-extend
//! transactions, and dropdown/asset-stripping scenarios. Computes
//! discount capture, leverage impact, and remaining-holder impairment.
//!
//! # Overview
//!
//! LMEs are the primary restructuring tools used before (and often
//! instead of) formal bankruptcy. This module quantifies the economics
//! for both the issuer (debt reduction, leverage improvement) and the
//! remaining holders (collateral dilution, recovery impairment).

use finstack_core::dates::Date;
use finstack_core::money::Money;
use serde::{Deserialize, Serialize};

use super::error::RestructuringError;

/// Type of liability management exercise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LmeType {
    /// Open market repurchase at discount.
    OpenMarketRepurchase {
        /// Purchase price as fraction of par (e.g., 0.60 = 60 cents).
        purchase_price: f64,
        /// Maximum notional to repurchase.
        max_notional: Money,
    },
    /// Tender offer with early and late pricing tiers.
    TenderOffer {
        /// Price for early tenders (fraction of par).
        early_price: f64,
        /// Price for late tenders (fraction of par).
        late_price: f64,
        /// Early tender deadline.
        early_deadline: Date,
        /// Final expiration.
        expiration: Date,
        /// Target amount.
        target_amount: Option<Money>,
    },
    /// Amend-and-extend: maturity pushed out in exchange for fee or higher coupon.
    AmendAndExtend {
        /// New maturity date.
        new_maturity: Date,
        /// Coupon adjustment (positive = increase).
        coupon_delta_bps: f64,
        /// Extension fee as fraction of par.
        extension_fee_pct: f64,
        /// Required consent threshold.
        required_consent: f64,
    },
    /// Dropdown transaction: transfer of assets from restricted to
    /// unrestricted subsidiary, reducing collateral available to
    /// existing creditors. Models the value leakage.
    Dropdown {
        /// Description of assets being transferred.
        asset_description: String,
        /// Estimated value of transferred assets.
        transferred_value: Money,
        /// New debt issued at unrestricted subsidiary against these assets.
        new_subsidiary_debt: Option<Money>,
    },
}

/// Full LME specification combining the transaction type with
/// the instrument it targets and the analysis parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LmeSpec {
    /// Unique identifier.
    pub id: String,
    /// Type of LME transaction.
    pub lme_type: LmeType,
    /// Instrument ID being targeted.
    pub target_instrument_id: String,
    /// Outstanding amount of the target instrument.
    pub outstanding_amount: Money,
    /// Current market price (fraction of par).
    pub current_market_price: f64,
}

/// Analysis result for an LME transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LmeAnalysis {
    /// Debt reduction achieved (par value retired).
    pub par_retired: Money,
    /// Cash cost to execute.
    pub cash_cost: Money,
    /// Discount capture (par retired - cash cost).
    pub discount_capture: Money,
    /// Discount capture as percentage of par retired.
    pub discount_capture_pct: f64,
    /// Pro forma leverage impact (if applicable).
    pub leverage_impact: Option<LeverageImpact>,
    /// Impact on remaining holders (for dropdown/uptier transactions).
    pub remaining_holder_impact: Option<RemainingHolderImpact>,
}

/// Leverage change from an LME.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeverageImpact {
    /// Pre-transaction total debt.
    pub pre_total_debt: Money,
    /// Post-transaction total debt.
    pub post_total_debt: Money,
    /// Pre-transaction leverage ratio (debt / EBITDA).
    pub pre_leverage: f64,
    /// Post-transaction leverage ratio.
    pub post_leverage: f64,
    /// Leverage reduction in turns.
    pub leverage_reduction: f64,
}

/// Impact on non-participating creditors from an LME.
///
/// In dropdown transactions, asset stripping, or uptier exchanges,
/// remaining holders may see their recovery impaired.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemainingHolderImpact {
    /// Description of the impact.
    pub description: String,
    /// Estimated collateral value reduction.
    pub collateral_reduction: Money,
    /// Estimated recovery rate change (negative = impairment, in basis points).
    pub recovery_impact_bps: f64,
    /// Whether this triggers any covenant provisions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub covenant_implications: Vec<String>,
}

/// Analyze a liability management exercise.
///
/// # Arguments
///
/// * `spec` - LME specification
/// * `ebitda` - Current EBITDA for leverage calculations (optional)
/// * `participation_rate` - Estimated fraction of holders participating
///
/// # Errors
///
/// Returns error if purchase/tender price is outside (0.0, 1.5],
/// if outstanding amount is non-positive, or if participation rate
/// is outside [0.0, 1.0].
pub fn analyze_lme(
    spec: &LmeSpec,
    ebitda: Option<Money>,
    participation_rate: f64,
) -> crate::Result<LmeAnalysis> {
    validate_lme(spec, participation_rate)?;

    let outstanding = spec.outstanding_amount.amount();
    let participating = outstanding * participation_rate;
    let currency = spec.outstanding_amount.currency();

    let (par_retired, cash_cost, remaining_impact) = match &spec.lme_type {
        LmeType::OpenMarketRepurchase {
            purchase_price,
            max_notional,
        } => {
            let notional = participating.min(max_notional.amount());
            let cost = notional * purchase_price;
            (notional, cost, None)
        }
        LmeType::TenderOffer {
            early_price,
            late_price,
            target_amount,
            ..
        } => {
            let target = target_amount
                .as_ref()
                .map_or(participating, |t| t.amount().min(participating));
            // Assume 60% early / 40% late split as simplification.
            let early_frac = 0.6;
            let cost = target * (early_frac * early_price + (1.0 - early_frac) * late_price);
            (target, cost, None)
        }
        LmeType::AmendAndExtend {
            extension_fee_pct, ..
        } => {
            // No par retired; maturity extended. Cost is the extension fee.
            let fee = participating * extension_fee_pct;
            (0.0, fee, None)
        }
        LmeType::Dropdown {
            transferred_value, ..
        } => {
            let impact = RemainingHolderImpact {
                description: "Collateral reduction from dropdown transaction".to_string(),
                collateral_reduction: *transferred_value,
                recovery_impact_bps: if outstanding > 0.0 {
                    transferred_value.amount() / outstanding * 10_000.0
                } else {
                    0.0
                },
                covenant_implications: vec![],
            };
            (0.0, 0.0, Some(impact))
        }
    };

    let discount_capture = par_retired - cash_cost;
    let discount_capture_pct = if par_retired > 0.0 {
        discount_capture / par_retired
    } else {
        0.0
    };

    let leverage_impact = ebitda.and_then(|ebitda_val| {
        let ebitda_amt = ebitda_val.amount();
        if ebitda_amt <= 0.0 {
            return None;
        }
        let pre_leverage = outstanding / ebitda_amt;
        let post_debt = outstanding - par_retired;
        let post_leverage = post_debt / ebitda_amt;
        Some(LeverageImpact {
            pre_total_debt: spec.outstanding_amount,
            post_total_debt: Money::new(post_debt, currency),
            pre_leverage,
            post_leverage,
            leverage_reduction: pre_leverage - post_leverage,
        })
    });

    Ok(LmeAnalysis {
        par_retired: Money::new(par_retired, currency),
        cash_cost: Money::new(cash_cost, currency),
        discount_capture: Money::new(discount_capture, currency),
        discount_capture_pct,
        leverage_impact,
        remaining_holder_impact: remaining_impact,
    })
}

// ─── Validation ──────────────────────────────────────────────────────

fn validate_lme(spec: &LmeSpec, participation_rate: f64) -> crate::Result<()> {
    // Validate outstanding.
    if spec.outstanding_amount.amount() <= 0.0 {
        return Err(RestructuringError::NonPositiveOutstanding {
            amount: spec.outstanding_amount.amount(),
        }
        .into());
    }

    // Validate participation rate.
    if !(0.0..=1.0).contains(&participation_rate) {
        return Err(
            RestructuringError::InvalidParticipationRate {
                rate: participation_rate,
            }
            .into(),
        );
    }

    // Validate type-specific constraints.
    match &spec.lme_type {
        LmeType::OpenMarketRepurchase { purchase_price, .. } => {
            if *purchase_price <= 0.0 || *purchase_price > 1.5 {
                return Err(
                    RestructuringError::InvalidPurchasePrice {
                        price: *purchase_price,
                    }
                    .into(),
                );
            }
        }
        LmeType::TenderOffer {
            early_price,
            late_price,
            ..
        } => {
            if *early_price <= 0.0 || *early_price > 1.5 {
                return Err(
                    RestructuringError::InvalidPurchasePrice {
                        price: *early_price,
                    }
                    .into(),
                );
            }
            if *late_price <= 0.0 || *late_price > 1.5 {
                return Err(
                    RestructuringError::InvalidPurchasePrice {
                        price: *late_price,
                    }
                    .into(),
                );
            }
        }
        _ => {}
    }

    Ok(())
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

    #[test]
    fn open_market_repurchase_discount_capture() {
        let spec = LmeSpec {
            id: "omr-1".into(),
            lme_type: LmeType::OpenMarketRepurchase {
                purchase_price: 0.60,
                max_notional: usd(50_000_000.0),
            },
            target_instrument_id: "bond-a".into(),
            outstanding_amount: usd(100_000_000.0),
            current_market_price: 0.58,
        };

        let result = analyze_lme(&spec, Some(usd(50_000_000.0)), 0.80).expect("should succeed");

        // Participating = 80M, capped by max_notional = 50M.
        assert!(
            (result.par_retired.amount() - 50_000_000.0).abs() < 1e-2,
            "par retired should be 50M, got {}",
            result.par_retired.amount()
        );
        // Cash cost = 50M * 0.60 = 30M.
        assert!(
            (result.cash_cost.amount() - 30_000_000.0).abs() < 1e-2,
            "cash cost should be 30M, got {}",
            result.cash_cost.amount()
        );
        // Discount capture = 50M - 30M = 20M.
        assert!(
            (result.discount_capture.amount() - 20_000_000.0).abs() < 1e-2,
            "discount capture should be 20M"
        );
        // Discount capture pct = 20M / 50M = 40%.
        assert!(
            (result.discount_capture_pct - 0.40).abs() < 1e-6,
            "discount capture pct should be 40%"
        );

        // Leverage impact.
        let lev = result.leverage_impact.expect("should have leverage impact");
        assert!((lev.pre_leverage - 2.0).abs() < 1e-6, "pre leverage = 100M / 50M = 2.0x");
        assert!(
            (lev.post_leverage - 1.0).abs() < 1e-6,
            "post leverage = (100M - 50M) / 50M = 1.0x"
        );
        assert!(
            (lev.leverage_reduction - 1.0).abs() < 1e-6,
            "leverage reduction should be 1.0x"
        );
    }

    #[test]
    fn tender_offer_early_late_pricing() {
        let spec = LmeSpec {
            id: "tender-1".into(),
            lme_type: LmeType::TenderOffer {
                early_price: 0.85,
                late_price: 0.80,
                early_deadline: make_date(2026, Month::May, 15),
                expiration: make_date(2026, Month::June, 15),
                target_amount: None,
            },
            target_instrument_id: "bond-b".into(),
            outstanding_amount: usd(200_000_000.0),
            current_market_price: 0.75,
        };

        let result = analyze_lme(&spec, None, 0.60).expect("should succeed");

        // Participating = 200M * 0.60 = 120M.
        let par = result.par_retired.amount();
        assert!(
            (par - 120_000_000.0).abs() < 1e-2,
            "par retired should be 120M, got {}",
            par
        );

        // Blended price = 0.6 * 0.85 + 0.4 * 0.80 = 0.83.
        let expected_cost = 120_000_000.0 * 0.83;
        assert!(
            (result.cash_cost.amount() - expected_cost).abs() < 1e-2,
            "cash cost should be {}, got {}",
            expected_cost,
            result.cash_cost.amount()
        );

        // Discount capture should be positive (bought below par).
        assert!(
            result.discount_capture.amount() > 0.0,
            "discount capture should be positive"
        );
    }

    #[test]
    fn dropdown_collateral_impact() {
        let spec = LmeSpec {
            id: "dropdown-1".into(),
            lme_type: LmeType::Dropdown {
                asset_description: "IP portfolio".into(),
                transferred_value: usd(30_000_000.0),
                new_subsidiary_debt: Some(usd(20_000_000.0)),
            },
            target_instrument_id: "bond-c".into(),
            outstanding_amount: usd(200_000_000.0),
            current_market_price: 0.90,
        };

        let result = analyze_lme(&spec, None, 1.0).expect("should succeed");

        // No par retired in a dropdown.
        assert!(result.par_retired.amount().abs() < 1e-6);

        let impact = result
            .remaining_holder_impact
            .expect("should have remaining holder impact");
        assert!(
            (impact.collateral_reduction.amount() - 30_000_000.0).abs() < 1e-2,
            "collateral reduction should be 30M"
        );
        // Impact bps = 30M / 200M * 10000 = 1500 bps.
        assert!(
            (impact.recovery_impact_bps - 1500.0).abs() < 1.0,
            "recovery impact should be ~1500 bps, got {}",
            impact.recovery_impact_bps
        );
    }

    #[test]
    fn amend_and_extend_no_par_retired() {
        let spec = LmeSpec {
            id: "ae-1".into(),
            lme_type: LmeType::AmendAndExtend {
                new_maturity: make_date(2030, Month::January, 15),
                coupon_delta_bps: 50.0,
                extension_fee_pct: 0.005,
                required_consent: 0.50,
            },
            target_instrument_id: "bond-d".into(),
            outstanding_amount: usd(100_000_000.0),
            current_market_price: 0.95,
        };

        let result = analyze_lme(&spec, None, 0.70).expect("should succeed");
        assert!(result.par_retired.amount().abs() < 1e-6, "no par retired in A&E");
    }

    #[test]
    fn invalid_purchase_price_rejected() {
        let spec = LmeSpec {
            id: "bad".into(),
            lme_type: LmeType::OpenMarketRepurchase {
                purchase_price: 0.0,
                max_notional: usd(10_000.0),
            },
            target_instrument_id: "x".into(),
            outstanding_amount: usd(100_000.0),
            current_market_price: 0.5,
        };
        assert!(analyze_lme(&spec, None, 0.5).is_err());
    }

    #[test]
    fn invalid_participation_rate_rejected() {
        let spec = LmeSpec {
            id: "bad2".into(),
            lme_type: LmeType::OpenMarketRepurchase {
                purchase_price: 0.5,
                max_notional: usd(10_000.0),
            },
            target_instrument_id: "x".into(),
            outstanding_amount: usd(100_000.0),
            current_market_price: 0.5,
        };
        assert!(analyze_lme(&spec, None, 1.5).is_err());
    }
}
