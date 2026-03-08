//! Pricer registry, trait, and dispatch infrastructure.
//!
//! Defines the [`Pricer`] trait, [`PricerRegistry`], and the [`expect_inst`]
//! downcast helper used by all pricer implementations.

use super::{
    InstrumentType, ModelKey, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::instruments::common_impl::traits::Instrument as Priceable;
use finstack_core::config::{results_meta_now, FinstackConfig};
use finstack_core::market_data::context::MarketContext as Market;
use finstack_core::HashMap;

/// Helper function to safely downcast a trait object to a concrete instrument type.
///
/// This performs both enum-based type checking and actual type downcasting,
/// ensuring type safety at both levels.
pub fn expect_inst<T: Priceable + 'static>(
    inst: &dyn Priceable,
    expected: InstrumentType,
) -> PricingResult<&T> {
    if inst.key() != expected {
        return Err(PricingError::type_mismatch(expected, inst.key()));
    }

    inst.as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| PricingError::type_mismatch(expected, inst.key()))
}

/// Trait for instrument pricers.
///
/// Each pricer handles a specific (instrument, model) combination and knows
/// how to price that instrument using the specified model.
pub trait Pricer: Send + Sync {
    /// Get the (instrument, model) key this pricer handles
    fn key(&self) -> PricerKey;

    /// Price an instrument using this pricer's model
    fn price_dyn(
        &self,
        instrument: &dyn Priceable,
        market: &Market,
        as_of: finstack_core::dates::Date,
    ) -> PricingResult<crate::results::ValuationResult>;
}

/// Registry mapping (instrument type, model) pairs to pricer implementations.
///
/// Provides type-safe pricing dispatch without string comparisons or runtime
/// registration errors. Pricers are registered at compile time and looked up
/// via strongly-typed keys.
#[derive(Default)]
pub struct PricerRegistry {
    pricers: HashMap<PricerKey, Box<dyn Pricer>>,
}

impl PricerRegistry {
    /// Create a new empty pricer registry.
    ///
    /// For pre-configured registries with all standard pricers, use
    /// [`create_standard_registry()`](super::create_standard_registry).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a pricer for a specific (instrument type, model) combination.
    ///
    /// If a pricer already exists for this key, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - Pricer key identifying the (instrument type, model) pair
    /// * `pricer` - Pricer implementation for this combination
    pub fn register_pricer(&mut self, key: PricerKey, pricer: Box<dyn Pricer>) {
        debug_assert!(
            !self.pricers.contains_key(&key),
            "Duplicate pricer registration for {key:?} -- this overwrites the existing pricer"
        );
        self.pricers.insert(key, pricer);
    }

    /// Convenience method to register a pricer with separate instrument type
    /// and model key arguments, boxing the pricer automatically.
    pub fn register(
        &mut self,
        inst: InstrumentType,
        model: ModelKey,
        pricer: impl Pricer + 'static,
    ) {
        self.register_pricer(PricerKey::new(inst, model), Box::new(pricer));
    }

    /// Look up a pricer for a specific (instrument type, model) combination.
    ///
    /// # Arguments
    ///
    /// * `key` - Pricer key to look up
    ///
    /// # Returns
    ///
    /// `Some(&dyn Pricer)` if registered, `None` otherwise
    pub fn get_pricer(&self, key: PricerKey) -> Option<&dyn Pricer> {
        self.pricers.get(&key).map(|p| p.as_ref())
    }

    /// Helper to look up a pricer using distinct type and model.
    ///
    /// # Arguments
    ///
    /// * `inst` - Instrument type
    /// * `model` - Model key
    ///
    /// # Returns
    ///
    /// `Some(&dyn Pricer)` if registered, `None` otherwise
    pub fn get(&self, inst: InstrumentType, model: ModelKey) -> Option<&dyn Pricer> {
        self.get_pricer(PricerKey::new(inst, model))
    }

    /// Price an instrument using the registry dispatch system.
    ///
    /// Routes the instrument to the appropriate pricer based on its type
    /// and the requested pricing model.
    ///
    /// # Arguments
    ///
    /// * `instrument` - Instrument to price (as trait object)
    /// * `model` - Pricing model to use
    /// * `market` - Market data context with curves and surfaces
    /// * `as_of` - Valuation date
    /// * `cfg` - Optional FinstackConfig. When `Some`, the result will be stamped with
    ///   the exact rounding/tolerance policy from the config. When `None`, uses default config.
    ///
    /// # Returns
    ///
    /// `ValuationResult` with present value and metadata
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No pricer registered for this (instrument, model) combination
    /// - Pricing calculation fails
    /// - Required market data is missing
    #[tracing::instrument(
        skip(self, instrument, market, cfg),
        fields(instrument_id = %instrument.id(), model = %model)
    )]
    pub fn price_with_registry(
        &self,
        instrument: &dyn Priceable,
        model: ModelKey,
        market: &Market,
        as_of: finstack_core::dates::Date,
        cfg: Option<&FinstackConfig>,
    ) -> PricingResult<crate::results::ValuationResult> {
        let key = PricerKey::new(instrument.key(), model);
        let Some(pricer) = self.get_pricer(key) else {
            return Err(PricingError::UnknownPricer(key));
        };

        let mut result = pricer.price_dyn(instrument, market, as_of)?;
        let effective_cfg = cfg.map_or_else(FinstackConfig::default, |c| c.clone());
        stamp_results_meta(&effective_cfg, &mut result);
        Ok(result)
    }

    /// Price an instrument and compute standard metrics using any registered model.
    ///
    /// Chains `price_dyn` (model PV + model-specific measures) into the standard
    /// metrics pipeline (`build_with_metrics_dyn`) so that all bond metric
    /// calculators (YTM, z-spread, durations, etc.) solve against the model price.
    ///
    /// This generalizes the `Instrument::price_with_metrics` path (which only
    /// works with the discount engine) to work with any registered model --
    /// hazard-rate, tree/OAS, Monte Carlo, etc.
    ///
    /// For non-discounting models, spread/yield metrics (z-spread, YTM, ASW, etc.)
    /// are computed on the instrument's `metrics_equivalent()` — a version with
    /// normalized cashflows (e.g., PIK coupon type converted to Cash) so that
    /// spreads are on a cash-equivalent basis.  Risk metrics (duration, DV01,
    /// convexity, CS01) use the original instrument's actual cashflows.
    ///
    /// # Arguments
    ///
    /// * `instrument` - Instrument to price (as trait object)
    /// * `model` - Pricing model to use
    /// * `market` - Market data context with curves and surfaces
    /// * `as_of` - Valuation date
    /// * `metrics` - Standard metrics to compute (e.g., `MetricId::Ytm`, `MetricId::ZSpread`)
    /// * `cfg` - Optional FinstackConfig for rounding/tolerance policy
    #[tracing::instrument(
        skip(self, instrument, market, metrics, cfg),
        fields(instrument_id = %instrument.id(), model = %model, num_metrics = metrics.len())
    )]
    pub fn price_with_metrics(
        &self,
        instrument: &dyn Priceable,
        model: ModelKey,
        market: &Market,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
        cfg: Option<&FinstackConfig>,
    ) -> PricingResult<crate::results::ValuationResult> {
        use crate::metrics::MetricId;

        let base_result = self.price_with_registry(instrument, model, market, as_of, cfg)?;

        if metrics.is_empty() {
            return Ok(base_result);
        }

        let err_ctx = PricingErrorContext::from_instrument(instrument).model(model);
        let needs_split = model != ModelKey::Discounting;

        if !needs_split {
            let mut enriched = crate::instruments::common_impl::helpers::build_with_metrics_dyn(
                std::sync::Arc::from(instrument.clone_box()),
                std::sync::Arc::new(market.clone()),
                as_of,
                base_result.value,
                metrics,
                cfg.map(|c| std::sync::Arc::new(c.clone())),
                None,
            )
            .map_err(|e| {
                PricingError::model_failure_with_context(e.to_string(), err_ctx.clone())
            })?;

            for (k, v) in base_result.measures {
                enriched.measures.insert(k, v);
            }
            return Ok(enriched);
        }

        // Non-discounting model: split metrics into spread (cash-equivalent
        // cashflows) and risk (actual cashflows).
        let spread_ids: &[MetricId] = &[
            MetricId::Ytm,
            MetricId::Ytw,
            MetricId::ZSpread,
            MetricId::ISpread,
            MetricId::DiscountMargin,
            MetricId::Oas,
            MetricId::ASWPar,
            MetricId::ASWMarket,
            MetricId::CleanPrice,
            MetricId::DirtyPrice,
            MetricId::Accrued,
            MetricId::EmbeddedOptionValue,
        ];

        let mut spread_metrics = Vec::new();
        let mut risk_metrics = Vec::new();
        for m in metrics {
            if spread_ids.contains(m) {
                spread_metrics.push(m.clone());
            } else {
                risk_metrics.push(m.clone());
            }
        }

        let cfg_arc = cfg.map(|c| std::sync::Arc::new(c.clone()));
        let market_arc = std::sync::Arc::new(market.clone());

        // Spread metrics: cash-equivalent cashflows via metrics_equivalent()
        let mut result = if !spread_metrics.is_empty() {
            crate::instruments::common_impl::helpers::build_with_metrics_dyn(
                std::sync::Arc::from(instrument.metrics_equivalent()),
                market_arc.clone(),
                as_of,
                base_result.value,
                &spread_metrics,
                cfg_arc.clone(),
                None,
            )
            .map_err(|e| PricingError::model_failure_with_context(e.to_string(), err_ctx.clone()))?
        } else {
            crate::results::ValuationResult::stamped(instrument.id(), as_of, base_result.value)
        };

        // Risk metrics: actual instrument cashflows
        if !risk_metrics.is_empty() {
            let risk_result = crate::instruments::common_impl::helpers::build_with_metrics_dyn(
                std::sync::Arc::from(instrument.clone_box()),
                market_arc,
                as_of,
                base_result.value,
                &risk_metrics,
                cfg_arc,
                None,
            )
            .map_err(|e| {
                PricingError::model_failure_with_context(e.to_string(), err_ctx.clone())
            })?;
            for (k, v) in risk_result.measures {
                result.measures.insert(k, v);
            }
        }

        // Model-specific measures from price_dyn take priority
        for (k, v) in base_result.measures {
            result.measures.insert(k, v);
        }

        Ok(result)
    }

    /// Price a batch of instruments using the registry dispatch system.
    ///
    /// The output order matches the input order. When the `parallel` feature is
    /// enabled, pricing is performed in parallel while preserving ordering.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Slice of instruments to price (as trait objects)
    /// * `model` - Pricing model to use
    /// * `market` - Market data context with curves and surfaces
    /// * `as_of` - Valuation date
    /// * `cfg` - Optional FinstackConfig. When `Some`, results will be stamped with
    ///   the exact rounding/tolerance policy from the config. When `None`, uses default config.
    pub fn price_batch(
        &self,
        instruments: &[&dyn Priceable],
        model: ModelKey,
        market: &Market,
        as_of: finstack_core::dates::Date,
        cfg: Option<&FinstackConfig>,
    ) -> Vec<PricingResult<crate::results::ValuationResult>> {
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            instruments
                .par_iter()
                .map(|&instrument| self.price_with_registry(instrument, model, market, as_of, cfg))
                .collect()
        }
        #[cfg(not(feature = "parallel"))]
        {
            instruments
                .iter()
                .map(|&instrument| self.price_with_registry(instrument, model, market, as_of, cfg))
                .collect()
        }
    }
}

/// Stamp result metadata from a config, preserving FX policy stamps if present.
fn stamp_results_meta(cfg: &FinstackConfig, result: &mut crate::results::ValuationResult) {
    let prev_fx_policy = result.meta.fx_policy_applied.clone();
    let mut meta = results_meta_now(cfg);
    meta.fx_policy_applied = prev_fx_policy;
    result.meta = meta;
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn registry_creation_test() {
        let registry = super::super::create_standard_registry();

        let key = PricerKey::new(InstrumentType::Bond, ModelKey::Discounting);
        assert!(registry.get_pricer(key).is_some());

        assert!(registry
            .get(InstrumentType::Bond, ModelKey::Discounting)
            .is_some());
    }

    #[test]
    fn registration_covers_all_pricers() {
        let registry = super::super::create_standard_registry();

        // Bond pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Bond, ModelKey::Discounting))
                .is_some(),
            "Bond Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Bond, ModelKey::HazardRate))
                .is_some(),
            "Bond HazardRate pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Bond, ModelKey::Tree))
                .is_some(),
            "Bond OAS pricer should be registered"
        );

        // Interest Rate pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::IRS, ModelKey::Discounting))
                .is_some(),
            "IRS Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::FRA, ModelKey::Discounting))
                .is_some(),
            "FRA Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CapFloor, ModelKey::Black76))
                .is_some(),
            "CapFloor Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CapFloor,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CapFloor Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Swaption, ModelKey::Black76))
                .is_some(),
            "Swaption Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Swaption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Swaption Discounting pricer should be registered"
        );

        // Credit pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CDS, ModelKey::HazardRate))
                .is_some(),
            "CDS HazardRate pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CDS, ModelKey::Discounting))
                .is_some(),
            "CDS Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSIndex,
                    ModelKey::HazardRate
                ))
                .is_some(),
            "CDSIndex HazardRate pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSIndex,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CDSIndex Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CDSOption, ModelKey::Black76))
                .is_some(),
            "CDSOption Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSOption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CDSOption Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSTranche,
                    ModelKey::HazardRate
                ))
                .is_some(),
            "CDSTranche HazardRate pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSTranche,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CDSTranche Discounting pricer should be registered"
        );

        // FX pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FxSpot,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxSpot Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::FxOption, ModelKey::Black76))
                .is_some(),
            "FxOption Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FxOption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxOption Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FxSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxSwap Discounting pricer should be registered"
        );

        // Equity pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Equity,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Equity Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::EquityOption,
                    ModelKey::Black76
                ))
                .is_some(),
            "EquityOption Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::EquityOption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "EquityOption Discounting pricer should be registered"
        );

        // Basic pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Deposit,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Deposit Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InterestRateFuture,
                    ModelKey::Discounting
                ))
                .is_some(),
            "InterestRateFuture Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::BasisSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "BasisSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Repo, ModelKey::Discounting))
                .is_some(),
            "Repo Discounting pricer should be registered"
        );

        // Inflation pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InflationSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "InflationSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::YoYInflationSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "YoYInflationSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InflationCapFloor,
                    ModelKey::Black76
                ))
                .is_some(),
            "InflationCapFloor Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InflationCapFloor,
                    ModelKey::Normal
                ))
                .is_some(),
            "InflationCapFloor Normal pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InflationLinkedBond,
                    ModelKey::Discounting
                ))
                .is_some(),
            "InflationLinkedBond Discounting pricer should be registered"
        );

        // Complex pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::VarianceSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "VarianceSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FxVarianceSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxVarianceSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::RealEstateAsset,
                    ModelKey::Discounting
                ))
                .is_some(),
            "RealEstateAsset Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CommodityOption,
                    ModelKey::Black76
                ))
                .is_some(),
            "CommodityOption Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Basket,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Basket Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Convertible,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Convertible Discounting pricer should be registered"
        );

        // Structured credit pricer (unified for ABS, CLO, CMBS, RMBS)
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::StructuredCredit,
                    ModelKey::Discounting
                ))
                .is_some(),
            "StructuredCredit Discounting pricer should be registered"
        );

        // TRS and Private Markets
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::EquityTotalReturnSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "EquityTotalReturnSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FIIndexTotalReturnSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FIIndexTotalReturnSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::PrivateMarketsFund,
                    ModelKey::Discounting
                ))
                .is_some(),
            "PrivateMarketsFund Discounting pricer should be registered"
        );
    }
}
