//! CDS Index pricing engine and helpers.
//!
//! Provides deterministic valuation for CDS indices with two pricing modes:
//! 1) Single-curve mode: price the index off a single index hazard curve by
//!    delegating to a synthetic single-name `CreditDefaultSwap` constructed
//!    from the index fields.
//! 2) Constituents mode: price each underlying issuer as a CDS with its own
//!    hazard curve and weight, then aggregate the results.
//!
//! Public API mirrors the CDS pricer surface for parity: NPV, par spread,
//! risky PV01, and leg PVs. Heavy numerical work is delegated to
//! `crate::instruments::cds::pricing::engine::CDSPricer`.

use crate::instruments::cds::pricing::engine::{CDSPricer, CDSPricerConfig};
use crate::instruments::cds::{CreditDefaultSwap, PayReceive};
use crate::instruments::cds_index::CDSIndex;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::{Error, Result, F};

// (kept for clarity; validation uses config.weight_sum_tol)
#[allow(dead_code)]
const WEIGHT_SUM_TOL: F = 1e-8;

/// Par spread denominator method for indices in constituents mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParSpreadMethod { RiskyAnnuity, FullPremiumAoD }

/// Configuration for CDS Index pricing. Wraps the underlying CDS config and adds
/// index-specific policy controls.
#[derive(Clone, Debug)]
pub struct CDSIndexPricerConfig {
    /// Underlying CDS pricer config to ensure parity on legs/AoD/schedules.
    pub cds_config: CDSPricerConfig,
    /// How to compute the par spread denominator in constituents aggregation.
    pub par_spread_method: ParSpreadMethod,
    /// Tolerance for weight sum validation.
    pub weight_sum_tol: F,
    /// If true and ∑w deviates within a looser bound, renormalize for pricing.
    pub normalize_weights: bool,
    /// If true, scale index notional by `index.index_factor`.
    pub use_index_factor: bool,
}

impl Default for CDSIndexPricerConfig {
    fn default() -> Self {
        Self {
            cds_config: CDSPricerConfig::default(),
            par_spread_method: ParSpreadMethod::RiskyAnnuity,
            weight_sum_tol: 1e-8,
            normalize_weights: false,
            use_index_factor: true,
        }
    }
}

/// CDS Index pricing engine. Aggregates single-name CDS pricing according to
/// the index's configured pricing mode.
pub struct CDSIndexPricer {
    config: CDSIndexPricerConfig,
}

impl Default for CDSIndexPricer { fn default() -> Self { Self::new() } }

impl CDSIndexPricer {
    /// Create a new CDS Index pricer
    pub fn new() -> Self { Self { config: CDSIndexPricerConfig::default() } }

    /// Create a pricer with custom configuration
    pub fn with_config(config: CDSIndexPricerConfig) -> Self { Self { config } }

    /// Compute instrument NPV from the perspective of `PayReceive`
    pub fn npv(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let (pv_protection, pv_premium) = self.pv_legs(index, curves, as_of)?;
        let mut pv = match index.side {
            PayReceive::PayProtection => pv_protection.checked_sub(pv_premium)?,
            PayReceive::ReceiveProtection => pv_premium.checked_sub(pv_protection)?,
        };
        if let Some(upfront) = index.pricing_overrides.upfront_payment {
            pv = (pv + upfront)?;
        }
        Ok(pv)
    }

    /// Present value of the protection leg (aggregated by pricing mode)
    pub fn pv_protection_leg(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let (pv_protection, _) = self.pv_legs(index, curves, as_of)?;
        Ok(pv_protection)
    }

    /// Present value of the premium leg (aggregated by pricing mode)
    pub fn pv_premium_leg(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let (_, pv_premium) = self.pv_legs(index, curves, as_of)?;
        Ok(pv_premium)
    }

    /// Par spread in basis points that sets NPV to zero.
    pub fn par_spread(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<F> {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            super::super::types::IndexPricing::SingleCurve => {
                let cds = index.to_synthetic_cds();
                let disc = curves.get_ref::<DiscountCurve>(cds.premium.disc_id)?;
                let surv = curves.get_ref::<HazardCurve>(cds.protection.credit_id)?;
                pricer.par_spread(&cds, disc, surv, as_of)
            }
            super::super::types::IndexPricing::Constituents => {
                // Sum protection PV and risky annuity weighted by notionals
                let mut prot_sum = Money::new(0.0, index.notional.currency());
                let mut denom_sum = 0.0; // sum_i (denom_i * notional_i)
                for cds in self.constituent_cdss(index)? {
                    let disc = curves.get_ref::<DiscountCurve>(cds.premium.disc_id)?;
                    let surv = curves.get_ref::<HazardCurve>(cds.protection.credit_id)?;
                    prot_sum = (prot_sum + pricer.pv_protection_leg(&cds, disc, surv, as_of)?)?;
                    let denom_per_unit = match self.config.par_spread_method {
                        ParSpreadMethod::RiskyAnnuity => pricer.risky_annuity(&cds, disc, surv, as_of)?,
                        ParSpreadMethod::FullPremiumAoD => pricer.premium_leg_pv_per_bp(&cds, disc, surv, as_of)?,
                    };
                    denom_sum += denom_per_unit * cds.notional.amount();
                }
                if denom_sum.abs() < 1e-12 { return Err(Error::Internal); }
                Ok(prot_sum.amount() / denom_sum * 10000.0)
            }
        }
    }

    /// Risky PV01 (absolute currency units) aggregated by pricing mode.
    pub fn risky_pv01(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<F> {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            super::super::types::IndexPricing::SingleCurve => {
                let cds = index.to_synthetic_cds();
                let disc = curves.get_ref::<DiscountCurve>(cds.premium.disc_id)?;
                let surv = curves.get_ref::<HazardCurve>(cds.protection.credit_id)?;
                pricer.risky_pv01(&cds, disc, surv, as_of)
            }
            super::super::types::IndexPricing::Constituents => {
                let mut sum = 0.0;
                for cds in self.constituent_cdss(index)? {
                    let disc = curves.get_ref::<DiscountCurve>(cds.premium.disc_id)?;
                    let surv = curves.get_ref::<HazardCurve>(cds.protection.credit_id)?;
                    sum += pricer.risky_pv01(&cds, disc, surv, as_of)?;
                }
                Ok(sum)
            }
        }
    }

    /// CS01 (approximate) aggregated by pricing mode.
    pub fn cs01(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<F> {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            super::super::types::IndexPricing::SingleCurve => {
                let cds = index.to_synthetic_cds();
                pricer.cs01(&cds, curves, as_of)
            }
            super::super::types::IndexPricing::Constituents => {
                let mut sum = 0.0;
                for cds in self.constituent_cdss(index)? {
                    sum += pricer.cs01(&cds, curves, as_of)?;
                }
                Ok(sum)
            }
        }
    }

    // ----- internals -----

    fn pv_legs(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<(Money, Money)> {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            super::super::types::IndexPricing::SingleCurve => {
                let cds = index.to_synthetic_cds();
                let disc = curves.get_ref::<DiscountCurve>(cds.premium.disc_id)?;
                let surv = curves.get_ref::<HazardCurve>(cds.protection.credit_id)?;
                let pv_protection = pricer.pv_protection_leg(&cds, disc, surv, as_of)?;
                let pv_premium = pricer.pv_premium_leg(&cds, disc, surv, as_of)?;
                Ok((pv_protection, pv_premium))
            }
            super::super::types::IndexPricing::Constituents => {
                let ccy = index.notional.currency();
                let mut prot_sum = Money::new(0.0, ccy);
                let mut prem_sum = Money::new(0.0, ccy);
                for cds in self.constituent_cdss(index)? {
                    let disc = curves.get_ref::<DiscountCurve>(cds.premium.disc_id)?;
                    let surv = curves.get_ref::<HazardCurve>(cds.protection.credit_id)?;
                    prot_sum = (prot_sum + pricer.pv_protection_leg(&cds, disc, surv, as_of)?)?;
                    prem_sum = (prem_sum + pricer.pv_premium_leg(&cds, disc, surv, as_of)?)?;
                }
                Ok((prot_sum, prem_sum))
            }
        }
    }

    fn constituent_cdss(&self, index: &CDSIndex) -> Result<Vec<CreditDefaultSwap>> {
        if index.constituents.is_empty() {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }
        // Validate weights and prepare effective weights (optionally renormalized)
        let sum_w: F = index.constituents.iter().map(|c| c.weight).sum();
        if index.constituents.iter().any(|c| c.weight < 0.0) {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
        if (sum_w - 1.0).abs() > self.config.weight_sum_tol {
            if self.config.normalize_weights && sum_w > 0.0 {
                // renormalize on the fly
            } else {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
        }
        // Validate recoveries in [0,1] and suggest family-consistent values; enforce range only
        for c in &index.constituents {
            if !(0.0..=1.0).contains(&c.credit.recovery_rate) {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
        }
        let norm = if self.config.normalize_weights && sum_w > 0.0 { sum_w } else { 1.0 };
        let mut out = Vec::with_capacity(index.constituents.len());
        let scale = if self.config.use_index_factor { index.index_factor } else { 1.0 };
        for (i, con) in index.constituents.iter().enumerate() {
            let eff_w = con.weight / norm;
            let notional = Money::new(index.notional.amount() * scale * eff_w, index.notional.currency());
            let id = format!("{}-{:03}", index.id, i + 1);
            out.push(CreditDefaultSwap::new_isda(
                id,
                notional,
                index.side,
                index.convention,
                index.premium.spread_bp,
                index.premium.start,
                index.premium.end,
                &con.credit.reference_entity,
                con.credit.recovery_rate,
                index.premium.disc_id,
                con.credit.credit_id,
            ));
        }
        Ok(out)
    }
}


