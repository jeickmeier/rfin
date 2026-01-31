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
#![allow(dead_code)] // Public API items may be used by external bindings
//! risky PV01, and leg PVs. Heavy numerical work is delegated to
//! `crate::instruments::cds::pricer::CDSPricer`.

use crate::calibration::bumps::hazard::{bump_hazard_shift, bump_hazard_spreads};
use crate::calibration::bumps::BumpRequest;
use crate::constants::{credit, BASIS_POINTS_PER_UNIT};
use crate::instruments::cds::pricer::{CDSPricer, CDSPricerConfig};
use crate::instruments::cds::{CreditDefaultSwap, PayReceive};
use crate::instruments::cds_index::{CDSIndex, IndexPricing};
use crate::instruments::common::traits::Instrument;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::{Error, Result};

/// Par spread denominator method for indices in constituents mode.
/// Method for computing par spread of a CDS index
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParSpreadMethod {
    /// Par spread computed using risky annuity (RPV01) method
    RiskyAnnuity,
    /// Par spread with full premium and accrual-on-default
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
            pv = pv.checked_add(upfront)?;
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
                let cds = self.synthetic_cds(index);
                let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
                let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
                pricer.par_spread(&cds, disc.as_ref(), surv.as_ref(), as_of)
            }
            IndexPricing::Constituents => {
                // Sum protection PV and risky annuity weighted by notionals
                let mut prot_sum = Money::new(0.0, index.notional.currency());
                let mut denom_sum = 0.0; // sum_i (denom_i * notional_i)
                let mut used_full_premium = false;
                for cds in self.constituent_cdss(index)? {
                    let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
                    let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
                    prot_sum = prot_sum.checked_add(pricer.pv_protection_leg(
                        &cds,
                        disc.as_ref(),
                        surv.as_ref(),
                        as_of,
                    )?)?;
                    let denom_per_unit = match self.config.par_spread_method {
                        ParSpreadMethod::RiskyAnnuity => {
                            pricer.risky_annuity(&cds, disc.as_ref(), surv.as_ref(), as_of)?
                        }
                        ParSpreadMethod::FullPremiumAoD => {
                            used_full_premium = true;
                            pricer.premium_leg_pv_per_bp(
                                &cds,
                                disc.as_ref(),
                                surv.as_ref(),
                                as_of,
                            )?
                        }
                    };
                    denom_sum += denom_per_unit * cds.notional.amount();
                }
                if denom_sum.abs() < credit::PAR_SPREAD_DENOM_TOLERANCE {
                    return Err(Error::Validation(
                        "CDS Index par spread denominator near zero (risky annuity sum ≈ 0). \
                         This may indicate zero survival probability across all constituents."
                            .to_string(),
                    ));
                }
                let par = if used_full_premium {
                    // Denominator already expresses PV per 1bp, so return in bp directly.
                    prot_sum.amount() / denom_sum
                } else {
                    prot_sum.amount() / denom_sum * BASIS_POINTS_PER_UNIT
                };
                Ok(par)
            }
        }
    }

    /// Risky PV01 (absolute currency units) aggregated by pricing mode.
    pub fn risky_pv01(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<f64> {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            IndexPricing::SingleCurve => {
                let cds = self.synthetic_cds(index);
                let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
                let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
                pricer.risky_pv01(&cds, disc.as_ref(), surv.as_ref(), as_of)
            }
            IndexPricing::Constituents => {
                let mut sum = 0.0;
                for cds in self.constituent_cdss(index)? {
                    let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
                    let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
                    sum += pricer.risky_pv01(&cds, disc.as_ref(), surv.as_ref(), as_of)?;
                }
                Ok(sum)
            }
        }
    }

    /// CS01 (approximate) aggregated by pricing mode.
    pub fn cs01(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<f64> {
        match index.pricing {
            IndexPricing::SingleCurve => {
                let cds = self.synthetic_cds(index);
                self.compute_cds_cs01(&cds, curves, as_of)
            }
            IndexPricing::Constituents => {
                let mut sum = 0.0;
                for cds in self.constituent_cdss(index)? {
                    sum += self.compute_cds_cs01(&cds, curves, as_of)?;
                }
                Ok(sum)
            }
        }
    }

    fn compute_cds_cs01(
        &self,
        cds: &CreditDefaultSwap,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        let credit_id = &cds.protection.credit_curve_id;
        let discount_id = &cds.premium.discount_curve_id;

        // Base PV
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        let hazard = curves.get_hazard(credit_id)?;
        let hazard_ref = hazard.as_ref();
        let has_par_points = hazard_ref.par_spread_points().next().is_some();

        let base_pv = if has_par_points {
            match bump_hazard_spreads(
                hazard_ref,
                curves,
                &BumpRequest::Parallel(0.0),
                Some(discount_id),
            ) {
                Ok(base_recal) => {
                    let base_ctx = curves.clone().insert_hazard(base_recal);
                    let disc = base_ctx.get_discount(discount_id)?;
                    let surv = base_ctx.get_hazard(credit_id)?;
                    pricer.npv(cds, disc.as_ref(), surv.as_ref(), as_of)?
                }
                Err(_) => {
                    let disc = curves.get_discount(discount_id)?;
                    let surv = curves.get_hazard(credit_id)?;
                    pricer.npv(cds, disc.as_ref(), surv.as_ref(), as_of)?
                }
            }
        } else {
            let disc = curves.get_discount(discount_id)?;
            let surv = curves.get_hazard(credit_id)?;
            pricer.npv(cds, disc.as_ref(), surv.as_ref(), as_of)?
        };

        let bumped_hazard = if has_par_points {
            match bump_hazard_spreads(
                hazard_ref,
                curves,
                &BumpRequest::Parallel(1.0),
                Some(discount_id),
            ) {
                Ok(curve) => curve,
                Err(_) => bump_hazard_shift(hazard_ref, &BumpRequest::Parallel(1.0))?,
            }
        } else {
            bump_hazard_shift(hazard_ref, &BumpRequest::Parallel(1.0))?
        };

        let bumped_ctx = curves.clone().insert_hazard(bumped_hazard);
        let bumped_disc = bumped_ctx.get_discount(discount_id)?;
        let bumped_surv = bumped_ctx.get_hazard(credit_id)?;
        let bumped_pv = pricer
            .npv(cds, bumped_disc.as_ref(), bumped_surv.as_ref(), as_of)?
            .amount();

        Ok(bumped_pv - base_pv.amount())
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
                let cds = self.synthetic_cds(index);
                let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
                let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
                let pv_protection =
                    pricer.pv_protection_leg(&cds, disc.as_ref(), surv.as_ref(), as_of)?;
                let pv_premium =
                    pricer.pv_premium_leg(&cds, disc.as_ref(), surv.as_ref(), as_of)?;
                Ok((pv_protection, pv_premium))
            }
            IndexPricing::Constituents => {
                let ccy = index.notional.currency();
                let mut prot_sum = Money::new(0.0, ccy);
                let mut prem_sum = Money::new(0.0, ccy);
                for cds in self.constituent_cdss(index)? {
                    let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
                    let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
                    prot_sum = prot_sum.checked_add(pricer.pv_protection_leg(
                        &cds,
                        disc.as_ref(),
                        surv.as_ref(),
                        as_of,
                    )?)?;
                    prem_sum = prem_sum.checked_add(pricer.pv_premium_leg(
                        &cds,
                        disc.as_ref(),
                        surv.as_ref(),
                        as_of,
                    )?)?;
                }
                Ok((prot_sum, prem_sum))
            }
        }
    }

    fn constituent_cdss(&self, index: &CDSIndex) -> Result<Vec<CreditDefaultSwap>> {
        if index.constituents.is_empty() {
            return Err(finstack_core::InputError::TooFewPoints.into());
        }
        // Validate weights and prepare effective weights (optionally renormalized)
        let sum_w: f64 = index.constituents.iter().map(|c| c.weight).sum();
        if index.constituents.iter().any(|c| c.weight < 0.0) {
            return Err(finstack_core::InputError::Invalid.into());
        }
        if (sum_w - 1.0).abs() > self.config.weight_sum_tol {
            if self.config.normalize_weights && sum_w > 0.0 {
                // renormalize on the fly
            } else {
                return Err(finstack_core::InputError::Invalid.into());
            }
        }
        // Validate recoveries in [0,1] and suggest family-consistent values; enforce range only
        for c in &index.constituents {
            if !(0.0..=1.0).contains(&c.credit.recovery_rate) {
                return Err(finstack_core::InputError::Invalid.into());
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
                index.premium.discount_curve_id.to_owned(),
                con.credit.credit_curve_id.to_owned(),
            )?);
        }
        Ok(out)
    }

    fn synthetic_cds(&self, index: &CDSIndex) -> CreditDefaultSwap {
        let mut cds = index.to_synthetic_cds();
        if self.config.use_index_factor {
            cds.notional = Money::new(
                index.notional.amount() * index.index_factor,
                index.notional.currency(),
            );
        }
        cds
    }
}

// ========================= REGISTRY PRICER =========================

/// Registry pricer for CDS Index using the engine
pub struct SimpleCdsIndexHazardPricer {
    model_key: crate::pricer::ModelKey,
}

impl SimpleCdsIndexHazardPricer {
    /// Create a new CDS index pricer with default hazard rate model
    pub fn new() -> Self {
        Self {
            model_key: crate::pricer::ModelKey::HazardRate,
        }
    }

    /// Create a CDS index pricer with specified model key
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
        as_of: finstack_core::dates::Date,
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

        // Use the provided as_of date for valuation
        // Compute present value using the engine
        let pv = CDSIndexPricer::new()
            .npv(cds_index, market, as_of)
            .map_err(|e| {
                crate::pricer::PricingError::model_failure_ctx(
                    e.to_string(),
                    crate::pricer::PricingErrorContext::default(),
                )
            })?;

        // Return stamped result
        Ok(crate::results::ValuationResult::stamped(
            cds_index.id(),
            as_of,
            pv,
        ))
    }
}
