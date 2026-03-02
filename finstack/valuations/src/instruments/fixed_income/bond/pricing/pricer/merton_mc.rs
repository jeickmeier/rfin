//! Merton Monte Carlo structural credit pricer for PIK bonds.
//!
//! Prices a bond using the Merton structural credit MC engine. The
//! [`MertonMcConfig`] must be set on the bond's `pricing_overrides.model_config`.
//!
//! After MC simulation, the pricer computes **cash-equivalent** Z-spread
//! and YTM: the Z-spread / YTM of a standard cash-pay bond that would
//! produce the same MC price. This makes spread/yield metrics comparable
//! across cash, PIK, and toggle structures.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::bond::types::Bond;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use indexmap::IndexMap;

pub struct SimpleBondMertonMcPricer;

impl SimpleBondMertonMcPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleBondMertonMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleBondMertonMcPricer {
    /// Build a cash-equivalent bond: same terms but `CouponType::Cash`,
    /// with `quoted_clean_price` set to the MC price.
    fn cash_equivalent_bond(bond: &Bond, mc_clean_pct: f64) -> Bond {
        use crate::cashflow::builder::specs::CouponType;

        let mut ceq = bond.clone();
        match &mut ceq.cashflow_spec {
            crate::instruments::fixed_income::bond::CashflowSpec::Fixed(ref mut spec) => {
                spec.coupon_type = CouponType::Cash;
            }
            crate::instruments::fixed_income::bond::CashflowSpec::Amortizing {
                ref mut base,
                ..
            } => {
                if let crate::instruments::fixed_income::bond::CashflowSpec::Fixed(ref mut spec) =
                    base.as_mut()
                {
                    spec.coupon_type = CouponType::Cash;
                }
            }
            _ => {}
        }
        ceq.pricing_overrides.market_quotes.quoted_clean_price = Some(mc_clean_pct);
        ceq.pricing_overrides.model_config.merton_mc_config = None;
        ceq
    }

    /// Run the existing metric calculators on the cash-equivalent bond.
    fn compute_ceq_metrics(
        ceq_bond: Bond,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
        base_value: finstack_core::money::Money,
    ) -> IndexMap<crate::metrics::MetricId, f64> {
        use crate::instruments::fixed_income::bond::metrics::price_yield_spread::{
            YtmCalculator, ZSpreadCalculator,
        };
        use crate::metrics::{MetricCalculator, MetricContext, MetricId};

        let instrument: std::sync::Arc<dyn Instrument> = std::sync::Arc::new(ceq_bond);
        let curves = std::sync::Arc::new(market.clone());
        let config = MetricContext::default_config();

        let mut ctx = MetricContext::new(instrument, curves, as_of, base_value, config);
        let mut results = IndexMap::new();

        if let Ok(z) = ZSpreadCalculator::default().calculate(&mut ctx) {
            results.insert(MetricId::ZSpread, z);
        }
        if let Ok(y) = YtmCalculator.calculate(&mut ctx) {
            results.insert(MetricId::Ytm, y);
        }

        results
    }
}

impl Pricer for SimpleBondMertonMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Bond, ModelKey::MertonMc)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        use finstack_core::money::Money;

        let bond = instrument
            .as_any()
            .downcast_ref::<Bond>()
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::Bond, instrument.key()))?;

        let ctx = PricingErrorContext::new()
            .instrument_id(bond.id())
            .instrument_type(InstrumentType::Bond)
            .model(ModelKey::MertonMc)
            .curve_id(bond.discount_curve_id.as_str());

        let mc_override = bond
            .pricing_overrides
            .model_config
            .merton_mc_config
            .as_ref()
            .ok_or_else(|| {
                PricingError::invalid_input_with_context(
                    "MertonMc pricer requires merton_mc_config on pricing_overrides",
                    ctx.clone(),
                )
            })?;

        let disc = market
            .get_discount(bond.discount_curve_id.as_str())
            .map_err(|e| PricingError::model_failure_with_context(e.to_string(), ctx.clone()))?;

        let mat_years = (bond.maturity - as_of).whole_days() as f64 / 365.25;
        let discount_rate = if mat_years > 0.0 {
            let df = disc.df(mat_years);
            if df > 0.0 {
                -df.ln() / mat_years
            } else {
                0.0
            }
        } else {
            0.0
        };

        let mc_result = bond
            .price_merton_mc(&mc_override.0, discount_rate, as_of)
            .map_err(|e| PricingError::model_failure_with_context(e.to_string(), ctx))?;

        let mc_clean_pct = mc_result.clean_price_pct;
        let pv_amount = mc_clean_pct / 100.0 * bond.notional.amount();
        let pv = Money::new(pv_amount, bond.notional.currency());

        let mut measures = IndexMap::new();
        measures.insert(
            crate::metrics::MetricId::custom("expected_loss"),
            mc_result.expected_loss,
        );
        measures.insert(
            crate::metrics::MetricId::custom("default_rate"),
            mc_result.path_statistics.default_rate,
        );
        measures.insert(
            crate::metrics::MetricId::custom("avg_terminal_notional"),
            mc_result.path_statistics.avg_terminal_notional,
        );
        measures.insert(
            crate::metrics::MetricId::custom("pik_fraction"),
            mc_result.average_pik_fraction,
        );
        measures.insert(
            crate::metrics::MetricId::custom("mc_standard_error"),
            mc_result.standard_error,
        );
        measures.insert(
            crate::metrics::MetricId::custom("unexpected_loss"),
            mc_result.unexpected_loss,
        );
        measures.insert(
            crate::metrics::MetricId::custom("expected_shortfall_95"),
            mc_result.expected_shortfall_95,
        );

        // Cash-equivalent spread/yield metrics using the actual ZSpreadCalculator
        // and YtmCalculator on a cash-equivalent bond.
        let ceq = Self::cash_equivalent_bond(bond, mc_clean_pct);
        let ceq_metrics = Self::compute_ceq_metrics(ceq, market, as_of, pv);
        measures.extend(ceq_metrics);

        let result = ValuationResult::stamped(bond.id(), as_of, pv);
        Ok(result.with_measures(measures))
    }
}
