//! Marginable trait implementations for financial instruments.
//!
//! This module provides implementations of the [`Marginable`] trait for
//! instruments that support margin calculations.

use crate::instruments::cds::CreditDefaultSwap;
use crate::instruments::cds_index::CDSIndex;
use crate::instruments::irs::InterestRateSwap;
use crate::instruments::repo::Repo;
use crate::instruments::equity_trs::EquityTotalReturnSwap;
use crate::instruments::fi_trs::FIIndexTotalReturnSwap;
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

// ============================================================================
// Helper Functions
// ============================================================================

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

        // For IRS, the main sensitivity is IR Delta (DV01)
        // We approximate by calculating DV01 and distributing across tenor buckets

        // Get time to maturity for tenor bucketing using float leg end date
        let days_to_maturity = (self.float.end - as_of).whole_days().max(0) as f64;
        let years_to_maturity = days_to_maturity / DAYS_PER_YEAR;

        // Estimate DV01 based on notional and maturity
        // DV01 ≈ Notional × Duration × ONE_BP
        // Duration ≈ years_to_maturity × DURATION_FACTOR for reasonable rates
        let estimated_duration = years_to_maturity * DURATION_APPROXIMATION_FACTOR;
        let dv01 = self.notional.amount().abs() * estimated_duration * ONE_BP;

        // Assign to appropriate SIMM tenor bucket
        let tenor = assign_ir_tenor_bucket(years_to_maturity);

        // Sign based on direction (pay fixed = short rates, receive fixed = long rates)
        let signed_dv01 = match self.side {
            crate::instruments::irs::PayReceive::PayFixed => -dv01,
            crate::instruments::irs::PayReceive::ReceiveFixed => dv01,
        };

        sens.add_ir_delta(currency, tenor, signed_dv01);

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        // Calculate NPV using the IRS pricer
        use crate::instruments::irs::pricer::npv;
        npv(self, market, as_of)
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
        _as_of: Date,
    ) -> Result<SimmSensitivities> {
        let currency = self.notional.currency();
        let mut sens = SimmSensitivities::new(currency);

        // For CDS, main sensitivity is CS01 (credit spread sensitivity)
        // CS01 ≈ Notional × Risky Duration × ONE_BP

        // Use standard 5Y maturity for CDS (most liquid tenor)
        let years_to_maturity = STANDARD_CDS_MATURITY_YEARS;

        // Risky duration approximation
        let risky_duration = years_to_maturity
            * (1.0 - self.protection.recovery_rate)
            * DURATION_APPROXIMATION_FACTOR;
        let cs01 = self.notional.amount().abs() * risky_duration * ONE_BP;

        // Extract reference entity name from credit curve id
        let ref_entity = extract_reference_entity(self.protection.credit_curve_id.as_str())?;

        // Determine if qualifying (investment grade) or non-qualifying
        // In practice, this would be looked up from ratings data
        // For now, assume qualifying if spread < threshold
        let qualifying = self.premium.spread_bp < INVESTMENT_GRADE_SPREAD_THRESHOLD_BP;

        // Assign to 5Y bucket (most liquid CDS tenor)
        let signed_cs01 = match self.side {
            crate::instruments::common::parameters::legs::PayReceive::PayFixed => cs01, // Protection buyer
            crate::instruments::common::parameters::legs::PayReceive::ReceiveFixed => -cs01, // Protection seller
        };

        sens.add_credit_delta(ref_entity, qualifying, "5Y", signed_cs01);

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        use crate::instruments::cds::pricer::CDSPricer;

        // Get discount and survival curves from market context
        let disc = market.get_discount_ref(self.premium.discount_curve_id.as_str())?;
        let surv = market.get_hazard_ref(self.protection.credit_curve_id.as_str())?;

        let pricer = CDSPricer::new();
        let pv_prot = pricer.pv_protection_leg(self, disc, surv, as_of)?;
        let pv_prem = pricer.pv_premium_leg(self, disc, surv, as_of)?;

        // NPV from protection buyer perspective (PayFixed)
        let npv = match self.side {
            crate::instruments::common::parameters::legs::PayReceive::PayFixed => {
                pv_prot.checked_sub(pv_prem)?
            }
            crate::instruments::common::parameters::legs::PayReceive::ReceiveFixed => {
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
        _as_of: Date,
    ) -> Result<SimmSensitivities> {
        let currency = self.notional.currency();
        let mut sens = SimmSensitivities::new(currency);

        // For CDS Index, similar to single-name but using index name
        // Use standard 5Y maturity for indices (most liquid tenor)
        let years_to_maturity = STANDARD_CDS_MATURITY_YEARS;
        let recovery_rate = self.protection.recovery_rate;
        let risky_duration =
            years_to_maturity * (1.0 - recovery_rate) * DURATION_APPROXIMATION_FACTOR;
        let cs01 = self.notional.amount().abs() * risky_duration * ONE_BP;

        // CDS indices are typically qualifying (investment grade indices)
        let qualifying = self.index_name.contains("IG") || !self.index_name.contains("HY");

        let signed_cs01 = match self.side {
            crate::instruments::common::parameters::legs::PayReceive::PayFixed => cs01,
            crate::instruments::common::parameters::legs::PayReceive::ReceiveFixed => -cs01,
        };

        sens.add_credit_delta(&self.index_name, qualifying, "5Y", signed_cs01);

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        self.npv(market, as_of)
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
        self.npv(market, as_of)
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
        _market: &MarketContext,
        _as_of: Date,
    ) -> Result<SimmSensitivities> {
        let currency = self.notional.currency();
        let mut sens = SimmSensitivities::new(currency);

        // For FI Index TRS, sensitivity depends on the underlying index
        // Use default duration for bond indices when actual data unavailable
        let estimated_duration = DEFAULT_BOND_INDEX_DURATION;
        let dv01 = self.notional.amount().abs() * estimated_duration * ONE_BP;

        let signed_dv01 = match self.side {
            TrsSide::ReceiveTotalReturn => -dv01, // Long bond = short rates
            TrsSide::PayTotalReturn => dv01, // Short bond = long rates
        };

        // Map duration to appropriate tenor bucket
        let tenor = assign_ir_tenor_bucket(estimated_duration);

        sens.add_ir_delta(currency, tenor, signed_dv01);

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        self.npv(market, as_of)
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

    fn repo_margin_spec(&self) -> Option<&crate::instruments::repo::RepoMarginSpec> {
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
mod tests {
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

        let swap = InterestRateSwap::create_usd_swap(
            "TEST_IRS".into(),
            Money::new(100_000_000.0, Currency::USD),
            0.035,
            start,
            end,
            crate::instruments::irs::PayReceive::PayFixed,
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

        let mut swap = InterestRateSwap::create_usd_swap(
            "TEST_IRS".into(),
            Money::new(100_000_000.0, Currency::USD),
            0.035,
            start,
            end,
            crate::instruments::irs::PayReceive::PayFixed,
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
