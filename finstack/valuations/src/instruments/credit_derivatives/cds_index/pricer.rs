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
//! `crate::instruments::credit_derivatives::cds::pricer::CDSPricer`.

use crate::calibration::bumps::hazard::{bump_hazard_shift, bump_hazard_spreads};
use crate::calibration::bumps::BumpRequest;
use crate::constants::{credit, BASIS_POINTS_PER_UNIT};
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds::pricer::{CDSPricer, CDSPricerConfig};
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::instruments::credit_derivatives::cds_index::{
    CDSIndex, ConstituentResult, IndexParSpreadResult, IndexPricing, IndexResult, ParSpreadMethod,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::{Error, Result};

/// Configuration for CDS Index pricing. Wraps the underlying CDS config and adds
/// index-specific policy controls.
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct ResolvedConstituent {
    cds: CreditDefaultSwap,
    credit_curve_id: CurveId,
    recovery_rate: f64,
    weight_raw: f64,
    weight_effective: f64,
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
        Ok(self.npv_detailed(index, curves, as_of)?.total)
    }

    /// Compute instrument NPV with optional per-constituent breakdown.
    pub fn npv_detailed(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<IndexResult<Money>> {
        let mut result = self.aggregate_money_detailed(
            index,
            curves,
            as_of,
            |pricer, cds, disc, surv, as_of| pricer.npv(cds, disc, surv, as_of),
        )?;
        if let Some(upfront) = index.pricing_overrides.market_quotes.upfront_payment {
            result.total = match index.side {
                crate::instruments::credit_derivatives::cds::PayReceive::PayFixed => {
                    result.total.checked_sub(upfront)?
                }
                crate::instruments::credit_derivatives::cds::PayReceive::ReceiveFixed => {
                    result.total.checked_add(upfront)?
                }
            };
        }
        Ok(result)
    }

    /// Present value of the protection leg (aggregated by pricing mode)
    pub fn pv_protection_leg(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        Ok(self.pv_protection_leg_detailed(index, curves, as_of)?.total)
    }

    /// Present value of the protection leg with optional per-constituent breakdown.
    pub fn pv_protection_leg_detailed(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<IndexResult<Money>> {
        self.aggregate_money_detailed(index, curves, as_of, |pricer, cds, disc, surv, as_of| {
            pricer.pv_protection_leg(cds, disc, surv, as_of)
        })
    }

    /// Present value of the premium leg (aggregated by pricing mode)
    pub fn pv_premium_leg(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        Ok(self.pv_premium_leg_detailed(index, curves, as_of)?.total)
    }

    /// Present value of the premium leg with optional per-constituent breakdown.
    pub fn pv_premium_leg_detailed(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<IndexResult<Money>> {
        self.aggregate_money_detailed(index, curves, as_of, |pricer, cds, disc, surv, as_of| {
            pricer.pv_premium_leg(cds, disc, surv, as_of)
        })
    }

    /// Par spread in basis points that sets NPV to zero.
    pub fn par_spread(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<f64> {
        Ok(self
            .par_spread_detailed(index, curves, as_of)?
            .total_spread_bp)
    }

    /// Par spread in basis points with optional per-constituent breakdown.
    pub fn par_spread_detailed(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<IndexParSpreadResult> {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            IndexPricing::SingleCurve => {
                let cds = self.synthetic_cds(index);
                let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
                let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
                let total_spread_bp =
                    pricer.par_spread(&cds, disc.as_ref(), surv.as_ref(), as_of)?;
                let numerator_protection_pv =
                    pricer.pv_protection_leg(&cds, disc.as_ref(), surv.as_ref(), as_of)?;
                // Use the unified par_spread_method config for both SingleCurve and
                // Constituents modes to avoid silent divergence between two independent
                // config fields (cds_config.par_spread_uses_full_premium vs par_spread_method).
                let (denom_per_unit, method) = match self.config.par_spread_method {
                    ParSpreadMethod::FullPremiumAoD => (
                        pricer.premium_leg_pv_per_bp(&cds, disc.as_ref(), surv.as_ref(), as_of)?,
                        ParSpreadMethod::FullPremiumAoD,
                    ),
                    ParSpreadMethod::RiskyAnnuity => (
                        pricer.risky_annuity(&cds, disc.as_ref(), surv.as_ref(), as_of)?,
                        ParSpreadMethod::RiskyAnnuity,
                    ),
                };
                let denominator = denom_per_unit * cds.notional.amount();
                Ok(IndexParSpreadResult {
                    total_spread_bp,
                    constituents_spread_bp: Vec::new(),
                    method,
                    numerator_protection_pv,
                    denominator,
                })
            }
            IndexPricing::Constituents => {
                let positions = self.constituent_positions(index)?;
                let mut numerator_protection_pv = Money::new(0.0, index.notional.currency());
                let mut denominator = 0.0;
                let mut constituents_spread_bp = Vec::with_capacity(positions.len());
                let mut used_full_premium = false;
                for position in positions {
                    let cds = &position.cds;
                    let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
                    let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
                    let prot_pv =
                        pricer.pv_protection_leg(cds, disc.as_ref(), surv.as_ref(), as_of)?;
                    numerator_protection_pv = numerator_protection_pv.checked_add(prot_pv)?;
                    let denom_per_unit = match self.config.par_spread_method {
                        ParSpreadMethod::RiskyAnnuity => {
                            pricer.risky_annuity(cds, disc.as_ref(), surv.as_ref(), as_of)?
                        }
                        ParSpreadMethod::FullPremiumAoD => {
                            used_full_premium = true;
                            pricer.premium_leg_pv_per_bp(
                                cds,
                                disc.as_ref(),
                                surv.as_ref(),
                                as_of,
                            )?
                        }
                    };
                    denominator += denom_per_unit * cds.notional.amount();
                    // Guard per-constituent division: if the local denominator is near zero
                    // (e.g., for a near-defaulted name with negligible survival probability),
                    // report NaN rather than propagating Inf which corrupts aggregation.
                    let local_denom = denom_per_unit * cds.notional.amount();
                    let constituent_spread_bp =
                        if local_denom.abs() < credit::PAR_SPREAD_DENOM_TOLERANCE {
                            f64::NAN
                        } else if used_full_premium {
                            prot_pv.amount() / local_denom
                        } else {
                            prot_pv.amount() / local_denom * BASIS_POINTS_PER_UNIT
                        };
                    constituents_spread_bp.push(ConstituentResult {
                        credit_curve_id: position.credit_curve_id,
                        recovery_rate: position.recovery_rate,
                        weight_raw: position.weight_raw,
                        weight_effective: position.weight_effective,
                        value: constituent_spread_bp,
                    });
                }
                if denominator.abs() < credit::PAR_SPREAD_DENOM_TOLERANCE {
                    return Err(Error::Validation(
                        "CDS Index par spread denominator near zero (risky annuity sum ≈ 0). \
                         This may indicate zero survival probability across all constituents."
                            .to_string(),
                    ));
                }
                let total_spread_bp = if used_full_premium {
                    numerator_protection_pv.amount() / denominator
                } else {
                    numerator_protection_pv.amount() / denominator * BASIS_POINTS_PER_UNIT
                };
                Ok(IndexParSpreadResult {
                    total_spread_bp,
                    constituents_spread_bp,
                    method: self.config.par_spread_method,
                    numerator_protection_pv,
                    denominator,
                })
            }
        }
    }

    /// Risky PV01 (absolute currency units) aggregated by pricing mode.
    pub fn risky_pv01(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<f64> {
        Ok(self.risky_pv01_detailed(index, curves, as_of)?.total)
    }

    /// Risky PV01 with optional per-constituent breakdown.
    pub fn risky_pv01_detailed(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<IndexResult<f64>> {
        self.aggregate_f64_detailed(index, curves, as_of, |pricer, cds, disc, surv, as_of| {
            pricer.risky_pv01(cds, disc, surv, as_of)
        })
    }

    /// CS01 (approximate) aggregated by pricing mode.
    pub fn cs01(&self, index: &CDSIndex, curves: &MarketContext, as_of: Date) -> Result<f64> {
        Ok(self.cs01_detailed(index, curves, as_of)?.total)
    }

    /// CS01 (approximate) with optional per-constituent breakdown.
    pub fn cs01_detailed(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<IndexResult<f64>> {
        self.aggregate_f64_detailed(index, curves, as_of, |_, cds, _, _, _| {
            self.compute_cds_cs01(cds, curves, as_of)
        })
    }

    fn compute_cds_cs01(
        &self,
        cds: &CreditDefaultSwap,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        let credit_id = &cds.protection.credit_curve_id;
        let discount_id = &cds.premium.discount_curve_id;
        let bump_bp = 1.0_f64;

        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        let hazard = curves.get_hazard(credit_id)?;
        let hazard_ref = hazard.as_ref();
        let has_par_points = hazard_ref.par_spread_points().next().is_some();

        let bump_hazard_for = |bp: f64| -> Result<_> {
            if has_par_points {
                match bump_hazard_spreads(
                    hazard_ref,
                    curves,
                    &BumpRequest::Parallel(bp),
                    Some(discount_id),
                ) {
                    Ok(curve) => Ok(curve),
                    Err(_) => bump_hazard_shift(hazard_ref, &BumpRequest::Parallel(bp)),
                }
            } else {
                bump_hazard_shift(hazard_ref, &BumpRequest::Parallel(bp))
            }
        };

        let bumped_up = bump_hazard_for(bump_bp)?;
        let ctx_up = curves.clone().insert(bumped_up);
        let disc_up = ctx_up.get_discount(discount_id)?;
        let surv_up = ctx_up.get_hazard(credit_id)?;
        let pv_up = pricer
            .npv(cds, disc_up.as_ref(), surv_up.as_ref(), as_of)?
            .amount();

        let bumped_down = bump_hazard_for(-bump_bp)?;
        let ctx_down = curves.clone().insert(bumped_down);
        let disc_down = ctx_down.get_discount(discount_id)?;
        let surv_down = ctx_down.get_hazard(credit_id)?;
        let pv_down = pricer
            .npv(cds, disc_down.as_ref(), surv_down.as_ref(), as_of)?
            .amount();

        Ok((pv_up - pv_down) / (2.0 * bump_bp))
    }

    // ----- internals -----

    fn aggregate_money_detailed<F>(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
        f: F,
    ) -> Result<IndexResult<Money>>
    where
        F: Fn(&CDSPricer, &CreditDefaultSwap, &DiscountCurve, &HazardCurve, Date) -> Result<Money>,
    {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            IndexPricing::SingleCurve => {
                let cds = self.synthetic_cds(index);
                let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
                let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
                let total = f(&pricer, &cds, disc.as_ref(), surv.as_ref(), as_of)?;
                Ok(IndexResult::single_curve(total))
            }
            IndexPricing::Constituents => {
                let positions = self.constituent_positions(index)?;
                let ccy = index.notional.currency();
                let mut total = Money::new(0.0, ccy);
                let mut constituents = Vec::with_capacity(positions.len());
                for position in positions {
                    let disc = curves.get_discount(&position.cds.premium.discount_curve_id)?;
                    let surv = curves.get_hazard(&position.cds.protection.credit_curve_id)?;
                    let value = f(&pricer, &position.cds, disc.as_ref(), surv.as_ref(), as_of)?;
                    total = total.checked_add(value)?;
                    constituents.push(ConstituentResult {
                        credit_curve_id: position.credit_curve_id,
                        recovery_rate: position.recovery_rate,
                        weight_raw: position.weight_raw,
                        weight_effective: position.weight_effective,
                        value,
                    });
                }
                Ok(IndexResult {
                    total,
                    constituents,
                })
            }
        }
    }

    fn aggregate_f64_detailed<F>(
        &self,
        index: &CDSIndex,
        curves: &MarketContext,
        as_of: Date,
        f: F,
    ) -> Result<IndexResult<f64>>
    where
        F: Fn(&CDSPricer, &CreditDefaultSwap, &DiscountCurve, &HazardCurve, Date) -> Result<f64>,
    {
        let pricer = CDSPricer::with_config(self.config.cds_config.clone());
        match index.pricing {
            IndexPricing::SingleCurve => {
                let cds = self.synthetic_cds(index);
                let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
                let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
                let total = f(&pricer, &cds, disc.as_ref(), surv.as_ref(), as_of)?;
                Ok(IndexResult::single_curve(total))
            }
            IndexPricing::Constituents => {
                let positions = self.constituent_positions(index)?;
                let mut total = 0.0;
                let mut constituents = Vec::with_capacity(positions.len());
                for position in positions {
                    let disc = curves.get_discount(&position.cds.premium.discount_curve_id)?;
                    let surv = curves.get_hazard(&position.cds.protection.credit_curve_id)?;
                    let value = f(&pricer, &position.cds, disc.as_ref(), surv.as_ref(), as_of)?;
                    total += value;
                    constituents.push(ConstituentResult {
                        credit_curve_id: position.credit_curve_id,
                        recovery_rate: position.recovery_rate,
                        weight_raw: position.weight_raw,
                        weight_effective: position.weight_effective,
                        value,
                    });
                }
                Ok(IndexResult {
                    total,
                    constituents,
                })
            }
        }
    }

    fn constituent_cdss(&self, index: &CDSIndex) -> Result<Vec<CreditDefaultSwap>> {
        Ok(self
            .constituent_positions(index)?
            .into_iter()
            .map(|c| c.cds)
            .collect())
    }

    fn constituent_positions(&self, index: &CDSIndex) -> Result<Vec<ResolvedConstituent>> {
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
        let active_constituents: Vec<_> =
            index.constituents.iter().filter(|c| !c.defaulted).collect();
        if active_constituents.is_empty() {
            return Ok(Vec::new());
        }
        let active_sum_w: f64 = active_constituents.iter().map(|c| c.weight).sum();
        let norm = if active_constituents.len() != index.constituents.len() && active_sum_w > 0.0 {
            active_sum_w
        } else if self.config.normalize_weights && sum_w > 0.0 {
            sum_w
        } else {
            1.0
        };
        let mut out = Vec::with_capacity(active_constituents.len());
        let scale = if self.config.use_index_factor {
            index.index_factor
        } else {
            1.0
        };
        for (i, con) in active_constituents.into_iter().enumerate() {
            let eff_w = con.weight / norm;
            let notional = Money::new(
                index.notional.amount() * scale * eff_w,
                index.notional.currency(),
            );
            let id = format!("{}-{:03}", index.id, i + 1);
            let cds = CreditDefaultSwap::new_isda(
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
            )?;
            out.push(ResolvedConstituent {
                cds,
                credit_curve_id: con.credit.credit_curve_id.to_owned(),
                recovery_rate: con.credit.recovery_rate,
                weight_raw: con.weight,
                weight_effective: eff_w,
            });
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
        use crate::instruments::common_impl::traits::Instrument;

        // Type-safe downcasting
        let cds_index = instrument
            .as_any()
            .downcast_ref::<crate::instruments::credit_derivatives::cds_index::CDSIndex>()
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
                crate::pricer::PricingError::model_failure_with_context(
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
    use crate::instruments::common_impl::parameters::CreditParams;
    use crate::instruments::credit_derivatives::cds_index::CDSIndexConstituent;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::HazardCurve;
    use test_utils::{date, flat_discount_with_tenor};

    fn sample_market(as_of: Date) -> MarketContext {
        let hazard = HazardCurve::builder("CDX.NA.IG.HAZARD")
            .base_date(as_of)
            .currency(Currency::USD)
            .recovery_rate(0.40)
            .knots([(0.0, 0.02), (5.0, 0.02)])
            .build()
            .expect("hazard curve should build");

        MarketContext::new()
            .insert(flat_discount_with_tenor("USD-OIS", as_of, 0.03, 5.0))
            .insert(hazard)
    }

    #[test]
    fn constituent_positions_skip_defaulted_names_and_renormalize_live_weights() {
        let mut index = CDSIndex::example();
        index.pricing = IndexPricing::Constituents;
        index.index_factor = 0.6;
        index.constituents = vec![
            CDSIndexConstituent {
                credit: CreditParams::corporate_standard("LIVE", "LIVE-HAZARD"),
                weight: 0.6,
                defaulted: false,
            },
            CDSIndexConstituent {
                credit: CreditParams::corporate_standard("DEFAULTED", "DEFAULTED-HAZARD"),
                weight: 0.4,
                defaulted: true,
            },
        ];

        let positions = CDSIndexPricer::new()
            .constituent_positions(&index)
            .expect("constituent positions should resolve");

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].credit_curve_id.as_str(), "LIVE-HAZARD");
        assert!((positions[0].weight_effective - 1.0).abs() < 1e-12);
        assert!(
            (positions[0].cds.notional.amount() - index.notional.amount() * index.index_factor)
                .abs()
                < 1e-8
        );
    }

    #[test]
    fn upfront_override_respects_pay_receive_sign() {
        let as_of = date(2024, 1, 1);
        let market = sample_market(as_of);
        let pricer = CDSIndexPricer::new();
        let upfront = Money::new(125_000.0, Currency::USD);

        let mut pay = CDSIndex::example();
        pay.pricing_overrides.market_quotes.upfront_payment = Some(upfront);
        let pay_base = pricer
            .npv(&CDSIndex::example(), &market, as_of)
            .expect("base pay npv");
        let pay_with_upfront = pricer
            .npv(&pay, &market, as_of)
            .expect("pay npv with upfront");

        let mut receive = CDSIndex::example();
        receive.side = crate::instruments::credit_derivatives::cds::PayReceive::ReceiveFixed;
        let mut receive_with_upfront = receive.clone();
        receive_with_upfront
            .pricing_overrides
            .market_quotes
            .upfront_payment = Some(upfront);
        let receive_base = pricer
            .npv(&receive, &market, as_of)
            .expect("base receive npv");
        let receive_total = pricer
            .npv(&receive_with_upfront, &market, as_of)
            .expect("receive npv with upfront");

        assert!((pay_with_upfront.amount() - (pay_base.amount() - upfront.amount())).abs() < 1e-8);
        assert!((receive_total.amount() - (receive_base.amount() + upfront.amount())).abs() < 1e-8);
    }
}
