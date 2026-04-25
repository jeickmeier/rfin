//! Factor-correlated default model.

use super::traits::{MacroCreditFactors, StochasticDefault};
use crate::cashflow::builder::specs::DefaultModelSpec;
use finstack_core::math::distributions::binomial_distribution;

/// Default model that shocks a deterministic default curve by a systematic factor.
#[derive(Debug, Clone)]
pub(crate) struct FactorCorrelatedDefault {
    base_spec: DefaultModelSpec,
    factor_loading: f64,
    cdr_volatility: f64,
}

impl FactorCorrelatedDefault {
    /// Create a factor-correlated default model.
    pub(crate) fn new(
        base_spec: DefaultModelSpec,
        factor_loading: f64,
        cdr_volatility: f64,
    ) -> Self {
        Self {
            base_spec,
            factor_loading: factor_loading.clamp(-1.0, 1.0),
            cdr_volatility: cdr_volatility.clamp(0.0, 1.0),
        }
    }

    fn base_mdr_at_seasoning(&self, seasoning: u32) -> f64 {
        self.base_spec.mdr(seasoning).unwrap_or(0.0).clamp(0.0, 1.0)
    }
}

impl StochasticDefault for FactorCorrelatedDefault {
    fn conditional_mdr(
        &self,
        seasoning: u32,
        factors: &[f64],
        _macro_factors: &MacroCreditFactors,
    ) -> f64 {
        let base_mdr = self.base_mdr_at_seasoning(seasoning);
        if base_mdr <= f64::EPSILON {
            return 0.0;
        }

        let z = factors.first().copied().unwrap_or(0.0);
        let shock = (self.factor_loading * z * self.cdr_volatility).exp();
        (base_mdr * shock).clamp(0.0, 0.50)
    }

    fn default_distribution(
        &self,
        n: usize,
        pds: &[f64],
        factors: &[f64],
        _correlation: f64,
    ) -> Vec<f64> {
        let base_pd = pds
            .first()
            .copied()
            .unwrap_or_else(|| self.base_mdr_at_seasoning(1));
        let z = factors.first().copied().unwrap_or(0.0);
        let shock = (self.factor_loading * z * self.cdr_volatility).exp();
        binomial_distribution(n, (base_pd * shock).clamp(0.0, 0.9999))
    }

    fn correlation(&self) -> f64 {
        self.factor_loading.abs().clamp(0.0, 0.99)
    }

    fn model_name(&self) -> &'static str {
        "Factor-Correlated Default"
    }

    fn expected_mdr(&self, seasoning: u32) -> f64 {
        self.base_mdr_at_seasoning(seasoning)
    }
}
