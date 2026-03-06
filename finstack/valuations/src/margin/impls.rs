//! Marginable trait implementations for financial instruments.
//!
//! This module provides implementations of the [`Marginable`] trait for
//! instruments that support margin calculations.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::instruments::credit_derivatives::cds_index::CDSIndex;
use crate::instruments::equity::equity_trs::EquityTotalReturnSwap;
use crate::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap;
use crate::instruments::rates::irs::InterestRateSwap;
use crate::instruments::rates::repo::Repo;
use crate::instruments::TrsSide;
use crate::margin::constants::{
    self, DAYS_PER_YEAR, DEFAULT_BOND_INDEX_DURATION, DURATION_APPROXIMATION_FACTOR,
    INVESTMENT_GRADE_SPREAD_THRESHOLD_BP, ONE_BP, STANDARD_CDS_MATURITY_YEARS,
};
use crate::margin::traits::{Marginable, NettingSetId, SimmSensitivities};
use crate::margin::types::{ClearingStatus, OtcMarginSpec};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use rust_decimal::prelude::ToPrimitive;

// ============================================================================
// Helper Functions
// ============================================================================

/// Assign a years-to-maturity value to the appropriate SIMM credit tenor bucket.
#[must_use]
fn assign_credit_tenor_bucket(years_to_maturity: f64) -> &'static str {
    use constants::tenor_buckets::*;
    match years_to_maturity {
        y if y <= BUCKET_1Y => "1Y",
        y if y <= BUCKET_2Y => "2Y",
        y if y <= BUCKET_3Y => "3Y",
        y if y <= BUCKET_5Y => "5Y",
        y if y <= BUCKET_10Y => "10Y",
        _ => "15Y",
    }
}

/// Assign a years-to-maturity value to the appropriate SIMM IR tenor bucket.
#[must_use]
fn assign_ir_tenor_bucket(years_to_maturity: f64) -> &'static str {
    use constants::tenor_buckets::*;
    match years_to_maturity {
        y if y <= BUCKET_6M => "6M",
        y if y <= BUCKET_1Y => "1Y",
        y if y <= BUCKET_2Y => "2Y",
        y if y <= BUCKET_3Y => "3Y",
        y if y <= BUCKET_5Y => "5Y",
        y if y <= BUCKET_10Y => "10Y",
        y if y <= BUCKET_15Y => "15Y",
        y if y <= BUCKET_20Y => "20Y",
        _ => "30Y",
    }
}

/// Extract reference entity from a credit curve ID.
///
/// Expects format like "ISSUER-CURVE" and extracts "ISSUER".
fn extract_reference_entity(credit_curve_id: &str) -> Result<&str> {
    credit_curve_id
        .split('-')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Invalid credit curve id format: '{}'. Expected format: 'ISSUER-CURVE'",
                credit_curve_id
            ))
        })
}

/// Determine if a credit entity is qualifying (investment grade) for SIMM bucketing.
///
/// Uses a combination of heuristics based on:
/// 1. Well-known index names (CDX.NA.IG, iTraxx Main = qualifying)
/// 2. Spread level as fallback (< 200bp threshold)
///
/// In production, this should be replaced with a lookup against a ratings database
/// or ISDA SIMM bucket mapping table.
fn is_credit_qualifying(name: &str, spread_bp: f64) -> bool {
    let upper = name.to_ascii_uppercase();
    if upper.contains("CDX.NA.IG") || (upper.contains("ITRAXX") && !upper.contains("XOVER")) {
        return true;
    }
    if upper.contains("CDX.NA.HY") || upper.contains("XOVER") || upper.contains("CDX.EM") {
        return false;
    }
    spread_bp < INVESTMENT_GRADE_SPREAD_THRESHOLD_BP
}

/// Derive a netting set ID from an OTC margin specification.
///
/// Maps clearing status to the appropriate netting set identifier.
#[must_use]
fn netting_set_id_from_spec(spec: &OtcMarginSpec) -> NettingSetId {
    match &spec.clearing_status {
        ClearingStatus::Cleared { ccp } => NettingSetId::cleared(ccp),
        ClearingStatus::Bilateral => NettingSetId::bilateral(&spec.csa.id, &spec.csa.id),
    }
}

// ============================================================================
// InterestRateSwap Implementation
// ============================================================================

impl Marginable for InterestRateSwap {
    fn margin_spec(&self) -> Option<&OtcMarginSpec> {
        self.margin_spec.as_ref()
    }

    fn netting_set_id(&self) -> Option<NettingSetId> {
        self.margin_spec.as_ref().map(netting_set_id_from_spec)
    }

    fn simm_sensitivities(
        &self,
        _market: &MarketContext,
        as_of: Date,
    ) -> Result<SimmSensitivities> {
        let currency = self.notional.currency();
        let mut sens = SimmSensitivities::new(currency);

        let days_to_maturity = (self.float.end - as_of).whole_days().max(0) as f64;
        let years_to_maturity = days_to_maturity / DAYS_PER_YEAR;

        if years_to_maturity <= 0.0 {
            return Ok(sens);
        }

        let total_dv01 = self.notional.amount().abs()
            * years_to_maturity
            * DURATION_APPROXIMATION_FACTOR
            * ONE_BP;

        let sign = match self.side {
            crate::instruments::rates::irs::PayReceive::PayFixed => -1.0,
            crate::instruments::rates::irs::PayReceive::ReceiveFixed => 1.0,
        };

        // Distribute DV01 across tenor buckets weighted by proportion of maturity
        // that falls within each bucket range. This is a simplified key-rate
        // duration decomposition.
        let buckets: &[(&str, f64, f64)] = &[
            ("6M", 0.0, 0.5),
            ("1Y", 0.5, 1.0),
            ("2Y", 1.0, 2.0),
            ("3Y", 2.0, 3.0),
            ("5Y", 3.0, 5.0),
            ("10Y", 5.0, 10.0),
            ("15Y", 10.0, 15.0),
            ("20Y", 15.0, 20.0),
            ("30Y", 20.0, 50.0),
        ];

        let mut total_weight = 0.0f64;
        let mut bucket_weights: Vec<(&str, f64)> = Vec::new();
        for &(name, lo, hi) in buckets {
            if years_to_maturity <= lo {
                break;
            }
            let effective_hi = hi.min(years_to_maturity);
            let weight = effective_hi - lo;
            if weight > 0.0 {
                bucket_weights.push((name, weight));
                total_weight += weight;
            }
        }

        if total_weight > 0.0 {
            for (name, weight) in bucket_weights {
                let fraction = weight / total_weight;
                let bucket_dv01 = sign * total_dv01 * fraction;
                sens.add_ir_delta(currency, name, bucket_dv01);
            }
        }

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        // Calculate NPV using the IRS pricer
        use crate::instruments::rates::irs::pricer::compute_pv;
        compute_pv(self, market, as_of)
    }
}

// ============================================================================
// CreditDefaultSwap Implementation
// ============================================================================

impl Marginable for CreditDefaultSwap {
    fn margin_spec(&self) -> Option<&OtcMarginSpec> {
        self.margin_spec.as_ref()
    }

    fn netting_set_id(&self) -> Option<NettingSetId> {
        self.margin_spec.as_ref().map(netting_set_id_from_spec)
    }

    fn simm_sensitivities(
        &self,
        _market: &MarketContext,
        as_of: Date,
    ) -> Result<SimmSensitivities> {
        let currency = self.notional.currency();
        let mut sens = SimmSensitivities::new(currency);

        let days_to_maturity = (self.premium.end - as_of).whole_days().max(0) as f64;
        let years_to_maturity = days_to_maturity / DAYS_PER_YEAR;
        let years_to_maturity = if years_to_maturity <= 0.0 {
            STANDARD_CDS_MATURITY_YEARS
        } else {
            years_to_maturity
        };

        let risky_duration = years_to_maturity
            * (1.0 - self.protection.recovery_rate)
            * DURATION_APPROXIMATION_FACTOR;
        let cs01 = self.notional.amount().abs() * risky_duration * ONE_BP;

        let ref_entity = extract_reference_entity(self.protection.credit_curve_id.as_str())?;
        let spread_bp_f64 = self.premium.spread_bp.to_f64().unwrap_or(f64::MAX);
        let qualifying = is_credit_qualifying(ref_entity, spread_bp_f64);

        let tenor = assign_credit_tenor_bucket(years_to_maturity);

        let signed_cs01 = match self.side {
            crate::instruments::common_impl::parameters::legs::PayReceive::PayFixed => cs01,
            crate::instruments::common_impl::parameters::legs::PayReceive::ReceiveFixed => -cs01,
        };

        sens.add_credit_delta(ref_entity, qualifying, tenor, signed_cs01);

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        use crate::instruments::credit_derivatives::cds::pricer::CDSPricer;

        // Get discount and survival curves from market context
        let disc = market.get_discount(self.premium.discount_curve_id.as_str())?;
        let surv = market.get_hazard(self.protection.credit_curve_id.as_str())?;

        let pricer = CDSPricer::new();
        let pv_prot = pricer.pv_protection_leg(self, disc.as_ref(), surv.as_ref(), as_of)?;
        let pv_prem = pricer.pv_premium_leg(self, disc.as_ref(), surv.as_ref(), as_of)?;

        // NPV from protection buyer perspective (PayFixed)
        let npv = match self.side {
            crate::instruments::common_impl::parameters::legs::PayReceive::PayFixed => {
                pv_prot.checked_sub(pv_prem)?
            }
            crate::instruments::common_impl::parameters::legs::PayReceive::ReceiveFixed => {
                pv_prem.checked_sub(pv_prot)?
            }
        };

        Ok(npv)
    }
}

// ============================================================================
// CDSIndex Implementation
// ============================================================================

impl Marginable for CDSIndex {
    fn margin_spec(&self) -> Option<&OtcMarginSpec> {
        self.margin_spec.as_ref()
    }

    fn netting_set_id(&self) -> Option<NettingSetId> {
        self.margin_spec.as_ref().map(netting_set_id_from_spec)
    }

    fn simm_sensitivities(
        &self,
        _market: &MarketContext,
        as_of: Date,
    ) -> Result<SimmSensitivities> {
        let currency = self.notional.currency();
        let mut sens = SimmSensitivities::new(currency);

        let days_to_maturity = (self.premium.end - as_of).whole_days().max(0) as f64;
        let years_to_maturity = days_to_maturity / DAYS_PER_YEAR;
        let years_to_maturity = if years_to_maturity <= 0.0 {
            STANDARD_CDS_MATURITY_YEARS
        } else {
            years_to_maturity
        };

        let recovery_rate = self.protection.recovery_rate;
        let risky_duration =
            years_to_maturity * (1.0 - recovery_rate) * DURATION_APPROXIMATION_FACTOR;
        let cs01 = self.notional.amount().abs() * risky_duration * ONE_BP;

        let qualifying = is_credit_qualifying(&self.index_name, 0.0);

        let tenor = assign_credit_tenor_bucket(years_to_maturity);

        let signed_cs01 = match self.side {
            crate::instruments::common_impl::parameters::legs::PayReceive::PayFixed => cs01,
            crate::instruments::common_impl::parameters::legs::PayReceive::ReceiveFixed => -cs01,
        };

        sens.add_credit_delta(&self.index_name, qualifying, tenor, signed_cs01);

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        self.value(market, as_of)
    }
}

// ============================================================================
// EquityTotalReturnSwap Implementation
// ============================================================================

impl Marginable for EquityTotalReturnSwap {
    fn margin_spec(&self) -> Option<&OtcMarginSpec> {
        self.margin_spec.as_ref()
    }

    fn netting_set_id(&self) -> Option<NettingSetId> {
        self.margin_spec.as_ref().map(netting_set_id_from_spec)
    }

    fn simm_sensitivities(
        &self,
        _market: &MarketContext,
        _as_of: Date,
    ) -> Result<SimmSensitivities> {
        let currency = self.notional.currency();
        let mut sens = SimmSensitivities::new(currency);

        // For Equity TRS, main sensitivity is equity delta
        // Delta = Notional (100% exposure to underlying)
        let delta = match self.side {
            TrsSide::ReceiveTotalReturn => self.notional.amount(),
            TrsSide::PayTotalReturn => -self.notional.amount(),
        };

        // Use the underlier as the equity identifier
        let underlier = &self.underlying.ticker;
        sens.add_equity_delta(underlier, delta);

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        use crate::instruments::common_impl::traits::Instrument;
        self.value(market, as_of)
    }
}

// ============================================================================
// FIIndexTotalReturnSwap Implementation
// ============================================================================

impl Marginable for FIIndexTotalReturnSwap {
    fn margin_spec(&self) -> Option<&OtcMarginSpec> {
        self.margin_spec.as_ref()
    }

    fn netting_set_id(&self) -> Option<NettingSetId> {
        self.margin_spec.as_ref().map(netting_set_id_from_spec)
    }

    fn simm_sensitivities(
        &self,
        market: &MarketContext,
        _as_of: Date,
    ) -> Result<SimmSensitivities> {
        let currency = self.notional.currency();
        let mut sens = SimmSensitivities::new(currency);

        // Use duration from market data when available, otherwise fall back to default.
        // This mirrors the logic in DurationDv01Calculator for consistency.
        let duration = self
            .underlying
            .duration_id
            .as_ref()
            .and_then(|id| {
                market.price(id.as_str()).ok().and_then(|s| match s {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Some(*v),
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => None,
                })
            })
            .unwrap_or(DEFAULT_BOND_INDEX_DURATION);

        let dv01 = self.notional.amount().abs() * duration * ONE_BP;

        let signed_dv01 = match self.side {
            TrsSide::ReceiveTotalReturn => -dv01, // Long bond = short rates
            TrsSide::PayTotalReturn => dv01,      // Short bond = long rates
        };

        // Map duration to appropriate tenor bucket
        let tenor = assign_ir_tenor_bucket(duration);

        sens.add_ir_delta(currency, tenor, signed_dv01);

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        self.value(market, as_of)
    }
}

// ============================================================================
// Repo Implementation
// ============================================================================

impl Marginable for Repo {
    fn margin_spec(&self) -> Option<&OtcMarginSpec> {
        // Repos don't use OtcMarginSpec - they use RepoMarginSpec
        None
    }

    fn repo_margin_spec(&self) -> Option<&crate::instruments::rates::repo::RepoMarginSpec> {
        self.margin_spec.as_ref()
    }

    fn netting_set_id(&self) -> Option<NettingSetId> {
        // Repos typically have their own netting arrangements
        // Use the repo ID as a simple netting set identifier
        Some(NettingSetId::bilateral(self.id.as_str(), "REPO_NETTING"))
    }

    fn simm_sensitivities(
        &self,
        _market: &MarketContext,
        as_of: Date,
    ) -> Result<SimmSensitivities> {
        let currency = self.cash_amount.currency();
        let mut sens = SimmSensitivities::new(currency);

        // Repos have limited rate sensitivity - mainly to the repo rate
        // Short-term IR sensitivity
        let days_to_maturity = (self.maturity - as_of).whole_days().max(1) as f64;
        let years_to_maturity = days_to_maturity / DAYS_PER_YEAR;

        // DV01 approximation for short-term lending
        let dv01 = self.cash_amount.amount() * years_to_maturity * ONE_BP;

        // Assign to shortest tenor bucket (3M for very short, otherwise 6M)
        let tenor = if years_to_maturity <= constants::tenor_buckets::BUCKET_3M {
            "3M"
        } else {
            "6M"
        };

        sens.add_ir_delta(currency, tenor, dv01);

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        self.pv(market, as_of)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    #[allow(clippy::expect_used, clippy::unwrap_used, dead_code, unused_imports)]
    mod test_utils {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/test_utils.rs"
        ));
    }

    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::June, 15).expect("valid date")
    }

    #[test]
    fn test_irs_marginable() {
        let start = test_date();
        let end = Date::from_calendar_date(2029, Month::June, 15).expect("valid date");

        let swap = test_utils::usd_irs_swap(
            "TEST_IRS",
            Money::new(100_000_000.0, Currency::USD),
            0.035,
            start,
            end,
            crate::instruments::rates::irs::PayReceive::PayFixed,
        )
        .expect("swap creation");

        // Without margin spec
        assert!(swap.margin_spec().is_none());
        assert!(!swap.has_margin());

        // Calculate sensitivities
        let market = MarketContext::new();
        let sens = swap
            .simm_sensitivities(&market, start)
            .expect("sensitivities");

        // Should have IR delta
        assert!(!sens.ir_delta.is_empty());
        assert!(sens.total_ir_delta().abs() > 0.0);
    }

    #[test]
    fn test_irs_multi_tenor_decomposition() {
        let start = test_date();
        let end = Date::from_calendar_date(2034, Month::June, 15).expect("valid date");

        let swap = test_utils::usd_irs_swap(
            "TEST_IRS_10Y",
            Money::new(100_000_000.0, Currency::USD),
            0.035,
            start,
            end,
            crate::instruments::rates::irs::PayReceive::PayFixed,
        )
        .expect("swap creation");

        let market = MarketContext::new();
        let sens = swap
            .simm_sensitivities(&market, start)
            .expect("sensitivities");

        assert!(
            sens.ir_delta.len() > 1,
            "Expected multi-tenor decomposition"
        );
        assert!(
            sens.total_ir_delta() < 0.0,
            "Pay fixed should be short rates"
        );
    }

    #[test]
    fn test_repo_marginable() {
        let repo = Repo::example();

        // Repo uses repo_margin_spec, not margin_spec
        assert!(repo.margin_spec().is_none());

        // Should have netting set
        let netting_set = repo.netting_set_id();
        assert!(netting_set.is_some());
    }

    #[test]
    fn test_netting_set_from_cleared_spec() {
        use crate::margin::types::{CsaSpec, ImMethodology, MarginTenor};

        let start = test_date();
        let end = Date::from_calendar_date(2029, Month::June, 15).expect("valid date");

        let mut swap = test_utils::usd_irs_swap(
            "TEST_IRS",
            Money::new(100_000_000.0, Currency::USD),
            0.035,
            start,
            end,
            crate::instruments::rates::irs::PayReceive::PayFixed,
        )
        .expect("swap creation");

        // Add cleared margin spec
        swap.margin_spec = Some(OtcMarginSpec {
            csa: CsaSpec::usd_regulatory(),
            clearing_status: ClearingStatus::Cleared {
                ccp: "LCH".to_string(),
            },
            im_methodology: ImMethodology::ClearingHouse,
            vm_frequency: MarginTenor::Daily,
            settlement_lag: 0,
        });

        let netting_set = swap.netting_set_id().expect("netting set");
        assert!(netting_set.is_cleared());
        assert_eq!(netting_set.ccp_id, Some("LCH".to_string()));
    }
}
