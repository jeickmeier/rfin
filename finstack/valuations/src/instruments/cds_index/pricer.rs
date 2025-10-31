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
//! `crate::instruments::cds::pricer::CDSPricer`.

use crate::instruments::cds::pricer::{CDSPricer, CDSPricerConfig};
use crate::instruments::cds::{CreditDefaultSwap, PayReceive};
use crate::instruments::cds_index::{CDSIndex, IndexPricing};
use crate::instruments::common::traits::Instrument;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::{Error, Result};

/// Par spread denominator method for indices in constituents mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParSpreadMethod {
    RiskyAnnuity,
    FullPremiumAoD,
}

/// Configuration for CDS Index pricing. Wraps the underlying CDS config and adds
/// index-specific policy controls.
#[derive(Clone, Debug)]
pub struct CDSIndexPricerConfig {
    /// Underlying CDS pricer config to ensure parity on legs/AoD/schedules.
    pub cds_config: CDSPricerConfig,
    /// How to compute the par spread denominator in constituents aggregation.
    pub par_spread_method: ParSpreadMethod,
    /// Tolerance for weight sum validation.
    pub weight_sum_tol: f64,
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

impl Default for CDSIndexPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl CDSIndexPricer {
    /// Create a new CDS Index pricer
    pub fn new() -> Self {
        Self {
            config: CDSIndexPricerConfig::default(),
        }
    }

    /// Create a pricer with custom configuration
    pub fn with_config(config: CDSIndexPricerConfig) -> Self {
        Self { config }
    }

    /// Compute instrument NPV from the perspective of `PayReceive`
    pub fn npv(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let (pv_protection, pv_premium) = self.pv_legs(index, curves, as_of)?;
        let mut pv = match index.side {
            PayReceive::PayFixed => pv_protection.checked_sub(pv_premium)?,
            PayReceive::ReceiveFixed => pv_premium.checked_sub(pv_protection)?,
        };
        if let Some(upfront) = index.pricing_overrides.upfront_payment {
            pv = (pv + upfront)?;
        }
        Ok(pv)
    }

    /// Present value of the protection leg (aggregated by pricing mode)
    pub fn pv_protection_leg(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        let (pv_protection, _) = self.pv_legs(index, curves, as_of)?;
        Ok(pv_protection)
    }

    /// Present value of the premium leg (aggregated by pricing mode)
    pub fn pv_premium_leg(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        let (_, pv_premium) = self.pv_legs(index, curves, as_of)?;
        Ok(pv_premium)
    }

    /// Par spread in basis points that sets NPV to zero.
    pub fn par_spread(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<f64> {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            IndexPricing::SingleCurve => {
                let cds = index.to_synthetic_cds();
                let disc = curves.get_discount_ref(&cds.premium.disc_id)?;
                let surv = curves.get_hazard_ref(&cds.protection.credit_id)?;
                pricer.par_spread(&cds, disc, surv, as_of)
            }
            IndexPricing::Constituents => {
                // Sum protection PV and risky annuity weighted by notionals
                let mut prot_sum = Money::new(0.0, index.notional.currency());
                let mut denom_sum = 0.0; // sum_i (denom_i * notional_i)
                for cds in self.constituent_cdss(index)? {
                    let disc = curves.get_discount_ref(&cds.premium.disc_id)?;
                    let surv = curves.get_hazard_ref(&cds.protection.credit_id)?;
                    prot_sum = (prot_sum + pricer.pv_protection_leg(&cds, disc, surv, as_of)?)?;
                    let denom_per_unit = match self.config.par_spread_method {
                        ParSpreadMethod::RiskyAnnuity => {
                            pricer.risky_annuity(&cds, disc, surv, as_of)?
                        }
                        ParSpreadMethod::FullPremiumAoD => {
                            pricer.premium_leg_pv_per_bp(&cds, disc, surv, as_of)?
                        }
                    };
                    denom_sum += denom_per_unit * cds.notional.amount();
                }
                if denom_sum.abs() < 1e-12 {
                    return Err(Error::Internal);
                }
                Ok(prot_sum.amount() / denom_sum * 10000.0)
            }
        }
    }

    /// Risky PV01 (absolute currency units) aggregated by pricing mode.
    pub fn risky_pv01(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<f64> {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            IndexPricing::SingleCurve => {
                let cds = index.to_synthetic_cds();
                let disc = curves.get_discount_ref(&cds.premium.disc_id)?;
                let surv = curves.get_hazard_ref(&cds.protection.credit_id)?;
                pricer.risky_pv01(&cds, disc, surv, as_of)
            }
            IndexPricing::Constituents => {
                let mut sum = 0.0;
                for cds in self.constituent_cdss(index)? {
                    let disc = curves.get_discount_ref(&cds.premium.disc_id)?;
                    let surv = curves.get_hazard_ref(&cds.protection.credit_id)?;
                    sum += pricer.risky_pv01(&cds, disc, surv, as_of)?;
                }
                Ok(sum)
            }
        }
    }

    /// CS01 (approximate) aggregated by pricing mode.
    pub fn cs01(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<f64> {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            IndexPricing::SingleCurve => {
                let cds = index.to_synthetic_cds();
                pricer.cs01(&cds, curves, as_of)
            }
            IndexPricing::Constituents => {
                let mut sum = 0.0;
                for cds in self.constituent_cdss(index)? {
                    sum += pricer.cs01(&cds, curves, as_of)?;
                }
                Ok(sum)
            }
        }
    }

    // ----- internals -----

    fn pv_legs(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(Money, Money)> {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            IndexPricing::SingleCurve => {
                let cds = index.to_synthetic_cds();
                let disc = curves.get_discount_ref(&cds.premium.disc_id)?;
                let surv = curves.get_hazard_ref(&cds.protection.credit_id)?;
                let pv_protection = pricer.pv_protection_leg(&cds, disc, surv, as_of)?;
                let pv_premium = pricer.pv_premium_leg(&cds, disc, surv, as_of)?;
                Ok((pv_protection, pv_premium))
            }
            IndexPricing::Constituents => {
                let ccy = index.notional.currency();
                let mut prot_sum = Money::new(0.0, ccy);
                let mut prem_sum = Money::new(0.0, ccy);
                for cds in self.constituent_cdss(index)? {
                    let disc = curves.get_discount_ref(&cds.premium.disc_id)?;
                    let surv = curves.get_hazard_ref(&cds.protection.credit_id)?;
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
        let sum_w: f64 = index.constituents.iter().map(|c| c.weight).sum();
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
        let norm = if self.config.normalize_weights && sum_w > 0.0 {
            sum_w
        } else {
            1.0
        };
        let mut out = Vec::with_capacity(index.constituents.len());
        let scale = if self.config.use_index_factor {
            index.index_factor
        } else {
            1.0
        };
        for (i, con) in index.constituents.iter().enumerate() {
            let eff_w = con.weight / norm;
            let notional = Money::new(
                index.notional.amount() * scale * eff_w,
                index.notional.currency(),
            );
            let id = format!("{}-{:03}", index.id, i + 1);
            out.push(CreditDefaultSwap::new_isda(
                id,
                notional,
                index.side,
                index.convention,
                index.premium.spread_bp,
                index.premium.start,
                index.premium.end,
                con.credit.recovery_rate,
                index.premium.disc_id.to_owned(),
                con.credit.credit_curve_id.to_owned(),
            ));
        }
        Ok(out)
    }
}

// ========================= REGISTRY PRICER =========================

/// Registry pricer for CDS Index using the engine
pub struct SimpleCdsIndexHazardPricer {
    model_key: crate::pricer::ModelKey,
}

impl SimpleCdsIndexHazardPricer {
    pub fn new() -> Self {
        Self {
            model_key: crate::pricer::ModelKey::HazardRate,
        }
    }

    pub fn with_model(model_key: crate::pricer::ModelKey) -> Self {
        Self { model_key }
    }
}

impl Default for SimpleCdsIndexHazardPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pricer::Pricer for SimpleCdsIndexHazardPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(crate::pricer::InstrumentType::CDSIndex, self.model_key)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        use crate::instruments::common::traits::Instrument;

        // Type-safe downcasting
        let cds_index = instrument
            .as_any()
            .downcast_ref::<crate::instruments::cds_index::CDSIndex>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::CDSIndex,
                    instrument.key(),
                )
            })?;

        // Get as_of date from discount curve
        let disc = market
            .get_discount_ref(&cds_index.premium.disc_id)
            .map_err(|e| crate::pricer::PricingError::model_failure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = CDSIndexPricer::new()
            .npv(cds_index, market, as_of)
            .map_err(|e| crate::pricer::PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(crate::results::ValuationResult::stamped(
            cds_index.id(),
            as_of,
            pv,
        ))
    }
}
