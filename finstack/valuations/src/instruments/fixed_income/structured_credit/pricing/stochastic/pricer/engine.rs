//! Stochastic structured-credit scenario waterfall pricing engine.

use super::config::{PricingMode, StochasticPricerConfig};
use super::result::{StochasticPricingResult, TranchePricingResult};
use crate::correlation::{FactorSpec, RecoverySpec};
use crate::instruments::fixed_income::structured_credit::pricing::simulation_engine::{
    run_simulation_with_source, PeriodPoolShock, StochasticPathFlowSource,
};
use crate::instruments::fixed_income::structured_credit::pricing::{
    StochasticDefaultSpec, StochasticPrepaySpec,
};
use crate::instruments::fixed_income::structured_credit::types::{StructuredCredit, Tranche};
use finstack_core::dates::{Date, DayCount, DayCountContext};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::random::{Pcg64Rng, RandomNumberGenerator};
use finstack_core::money::Money;
use finstack_core::Result;
use std::cmp::Ordering;

/// Stochastic pricing engine for structured credit.
///
/// Each scenario path feeds period SMM/MDR/recovery assumptions into the same
/// waterfall simulation used by deterministic tranche valuation. PV is computed
/// from actual dated tranche payments, not from terminal expected loss shortcuts.
pub(crate) struct StochasticPricer {
    config: StochasticPricerConfig,
}

impl StochasticPricer {
    /// Create a new stochastic pricer.
    pub(crate) fn new(config: StochasticPricerConfig) -> Self {
        Self { config }
    }

    /// Price the full deal and all tranches through scenario-level waterfalls.
    pub(crate) fn price(
        &self,
        instrument: &StructuredCredit,
        context: &MarketContext,
    ) -> Result<StochasticPricingResult> {
        match &self.config.pricing_mode {
            PricingMode::Tree => self.price_tree(instrument, context),
            PricingMode::MonteCarlo {
                num_paths,
                antithetic,
            } => self.price_monte_carlo(instrument, context, *num_paths, *antithetic),
            PricingMode::Hybrid { .. } => Err(finstack_core::Error::Validation(
                "Structured-credit stochastic Hybrid mode is disabled until conditional suffix \
                 path aggregation is implemented"
                    .to_string(),
            )),
        }
    }

    fn price_tree(
        &self,
        instrument: &StructuredCredit,
        context: &MarketContext,
    ) -> Result<StochasticPricingResult> {
        let terminal_paths = self
            .config
            .tree_config
            .branching
            .estimate_terminal_nodes(self.config.tree_config.num_periods);
        if terminal_paths > self.config.max_tree_paths {
            return Err(finstack_core::Error::Validation(format!(
                "structured_credit_stochastic tree requires {terminal_paths} terminal paths, \
                 above max_tree_paths={}",
                self.config.max_tree_paths
            )));
        }

        let branch_count = self
            .config
            .tree_config
            .branching
            .branches_at_node(0.0)
            .max(1);
        let path_count = terminal_paths.max(1);
        let mut collector = ScenarioCollector::new(instrument, path_count)?;
        for path_index in 0..path_count {
            let shocks = self.tree_path_shocks(instrument, path_index, branch_count)?;
            self.record_path(instrument, context, shocks, &mut collector)?;
        }
        Ok(collector.finalize(self, "Tree"))
    }

    fn price_monte_carlo(
        &self,
        instrument: &StructuredCredit,
        context: &MarketContext,
        num_paths: usize,
        antithetic: bool,
    ) -> Result<StochasticPricingResult> {
        if num_paths == 0 {
            return Err(finstack_core::Error::Validation(
                "Monte Carlo pricing requires at least one simulation path".to_string(),
            ));
        }

        let mut rng = Pcg64Rng::new(self.config.seed);
        let mut collector = ScenarioCollector::new(instrument, num_paths)?;
        let mut paired_factors: Option<Vec<f64>> = None;

        for path_index in 0..num_paths {
            let factors = if antithetic && path_index % 2 == 1 {
                paired_factors
                    .take()
                    .unwrap_or_else(|| self.random_factors(instrument, &mut rng))
                    .into_iter()
                    .map(|z| -z)
                    .collect()
            } else {
                let factors = self.random_factors(instrument, &mut rng);
                if antithetic {
                    paired_factors = Some(factors.clone());
                }
                factors
            };
            let shocks = self.path_shocks_from_factors(instrument, &factors)?;
            self.record_path(instrument, context, shocks, &mut collector)?;
        }

        let mode = if antithetic {
            format!("MonteCarlo({}, antithetic)", num_paths)
        } else {
            format!("MonteCarlo({num_paths})")
        };
        Ok(collector.finalize(self, &mode))
    }

    fn record_path(
        &self,
        instrument: &StructuredCredit,
        context: &MarketContext,
        shocks: Vec<PeriodPoolShock>,
        collector: &mut ScenarioCollector,
    ) -> Result<()> {
        let mut source = StochasticPathFlowSource::new(shocks);
        let path_results = run_simulation_with_source(
            instrument,
            context,
            self.config.valuation_date,
            &mut source,
        )?;

        let mut deal_pv = 0.0;
        let mut deal_loss = 0.0;
        for (idx, tranche) in instrument.tranches.tranches.iter().enumerate() {
            let tranche_result = path_results.get(tranche.id.as_str()).ok_or_else(|| {
                finstack_core::Error::Validation(format!(
                    "stochastic waterfall omitted tranche result '{}'",
                    tranche.id
                ))
            })?;
            let metrics = PathTrancheMetrics::from_cashflows(
                tranche_result,
                self.config.valuation_date,
                &self.config.discount_curve,
            )?;
            deal_pv += metrics.pv;
            deal_loss += metrics.loss;
            collector.record_tranche(idx, metrics);
        }
        collector.record_deal(deal_pv, deal_loss);
        Ok(())
    }

    fn random_factors(&self, instrument: &StructuredCredit, rng: &mut Pcg64Rng) -> Vec<f64> {
        let month_count = self.month_count(instrument);
        if !self.has_stochastic_rates() {
            return vec![0.0; month_count];
        }
        (0..month_count).map(|_| rng.normal(0.0, 1.0)).collect()
    }

    fn tree_path_shocks(
        &self,
        instrument: &StructuredCredit,
        mut path_index: usize,
        branch_count: usize,
    ) -> Result<Vec<PeriodPoolShock>> {
        let month_count = self.month_count(instrument);
        let branch_count = branch_count.max(1);
        let mut factors = Vec::with_capacity(month_count);

        for _ in 0..month_count {
            let branch = path_index % branch_count;
            path_index /= branch_count;
            let z = if self.has_stochastic_rates() {
                let p = (branch as f64 + 0.5) / branch_count as f64;
                finstack_core::math::standard_normal_inv_cdf(p)
            } else {
                0.0
            };
            factors.push(z);
        }

        self.path_shocks_from_factors(instrument, &factors)
    }

    fn path_shocks_from_factors(
        &self,
        instrument: &StructuredCredit,
        factors: &[f64],
    ) -> Result<Vec<PeriodPoolShock>> {
        let months_per_period = instrument.frequency.months().ok_or_else(|| {
            finstack_core::Error::Validation(
                "Structured credit stochastic pricing requires month-based payment frequencies"
                    .to_string(),
            )
        })? as usize;
        let months_per_period = months_per_period.max(1);
        let payment_periods = self.payment_period_count(instrument);
        let mut shocks = Vec::with_capacity(payment_periods);

        for period in 0..payment_periods {
            let start = period * months_per_period;
            let end = (start + months_per_period).min(factors.len());
            let month_slice = if start < end {
                &factors[start..end]
            } else {
                &[][..]
            };
            shocks.push(self.aggregate_monthly_shocks(month_slice));
        }

        Ok(shocks)
    }

    fn aggregate_monthly_shocks(&self, factors: &[f64]) -> PeriodPoolShock {
        if factors.is_empty() {
            return self.monthly_shock(0.0);
        }

        let mut prepay_survival = 1.0;
        let mut default_survival = 1.0;
        let mut recovery_sum = 0.0;
        for factor in factors {
            let shock = self.monthly_shock(*factor);
            prepay_survival *= 1.0 - shock.smm;
            default_survival *= 1.0 - shock.mdr;
            recovery_sum += shock.recovery_rate;
        }

        let months = factors.len() as f64;
        PeriodPoolShock {
            smm: 1.0 - prepay_survival.powf(1.0 / months),
            mdr: 1.0 - default_survival.powf(1.0 / months),
            recovery_rate: recovery_sum / months,
        }
    }

    fn monthly_shock(&self, z: f64) -> PeriodPoolShock {
        let factor = if self.has_stochastic_rates() { z } else { 0.0 };
        let prepay_factor = self.prepay_factor_scale() * factor;
        let default_factor = self.default_factor_scale() * factor;
        let base_smm = self.config.tree_config.prepay_spec.base_smm();
        let base_mdr = self.config.tree_config.default_spec.base_mdr();

        PeriodPoolShock {
            smm: (base_smm * prepay_factor.exp()).clamp(0.0, 0.50),
            mdr: (base_mdr * default_factor.exp()).clamp(0.0, 0.50),
            recovery_rate: self.recovery_rate(factor),
        }
    }

    fn prepay_factor_scale(&self) -> f64 {
        let model_scale = match &self.config.tree_config.prepay_spec {
            StochasticPrepaySpec::Deterministic(_) => 0.0,
            StochasticPrepaySpec::FactorCorrelated {
                factor_loading,
                cpr_volatility,
                ..
            }
            | StochasticPrepaySpec::RichardRoll {
                factor_loading,
                cpr_volatility,
                ..
            } => factor_loading * cpr_volatility,
            StochasticPrepaySpec::RegimeSwitching { .. } => {
                self.config.correlation_prepay_loading()
            }
        };
        model_scale * self.factor_model_prepay_scale()
    }

    fn default_factor_scale(&self) -> f64 {
        let model_scale = match &self.config.tree_config.default_spec {
            StochasticDefaultSpec::Deterministic(_) => 0.0,
            StochasticDefaultSpec::Copula { correlation, .. } => correlation.sqrt(),
            StochasticDefaultSpec::IntensityProcess {
                factor_sensitivity,
                volatility,
                ..
            }
            | StochasticDefaultSpec::HazardCurveBased {
                factor_sensitivity,
                volatility,
                ..
            } => factor_sensitivity * volatility,
            StochasticDefaultSpec::FactorCorrelated {
                factor_loading,
                cdr_volatility,
                ..
            } => factor_loading * cdr_volatility,
        };
        model_scale * self.factor_model_credit_scale()
    }

    fn factor_model_prepay_scale(&self) -> f64 {
        match &self.config.tree_config.factor_spec {
            FactorSpec::SingleFactor { volatility, .. } => *volatility,
            FactorSpec::TwoFactor { prepay_vol, .. } => *prepay_vol,
            FactorSpec::MultiFactor { volatilities, .. } => {
                volatilities.first().copied().unwrap_or(1.0)
            }
        }
    }

    fn factor_model_credit_scale(&self) -> f64 {
        match &self.config.tree_config.factor_spec {
            FactorSpec::SingleFactor { volatility, .. } => *volatility,
            FactorSpec::TwoFactor { credit_vol, .. } => *credit_vol,
            FactorSpec::MultiFactor { volatilities, .. } => volatilities
                .get(1)
                .or_else(|| volatilities.first())
                .copied()
                .unwrap_or(1.0),
        }
    }

    fn recovery_rate(&self, factor: f64) -> f64 {
        match &self.config.tree_config.recovery_spec {
            RecoverySpec::Constant { rate } => *rate,
            RecoverySpec::MarketCorrelated {
                mean_recovery,
                recovery_volatility,
                factor_correlation,
            } => {
                (mean_recovery + factor_correlation * recovery_volatility * factor).clamp(0.0, 1.0)
            }
        }
    }

    fn has_stochastic_rates(&self) -> bool {
        self.config.tree_config.prepay_spec.is_stochastic()
            || self.config.tree_config.default_spec.is_stochastic()
            || matches!(
                self.config.tree_config.recovery_spec,
                RecoverySpec::MarketCorrelated { .. }
            )
    }

    fn month_count(&self, instrument: &StructuredCredit) -> usize {
        let periods = self.payment_period_count(instrument);
        let months_per_period = instrument.frequency.months().unwrap_or(1).max(1) as usize;
        periods.saturating_mul(months_per_period).max(1)
    }

    fn payment_period_count(&self, instrument: &StructuredCredit) -> usize {
        let months_per_period = instrument.frequency.months().unwrap_or(1).max(1) as usize;
        let base_periods = self
            .config
            .tree_config
            .num_periods
            .saturating_add(months_per_period - 1)
            / months_per_period;
        base_periods.saturating_add(2).max(1)
    }
}

trait CorrelationLoading {
    fn correlation_prepay_loading(&self) -> f64;
}

impl CorrelationLoading for StochasticPricerConfig {
    fn correlation_prepay_loading(&self) -> f64 {
        self.tree_config.correlation.prepay_factor_loading()
    }
}

#[derive(Clone, Copy, Default)]
struct PathTrancheMetrics {
    pv: f64,
    loss: f64,
    wal: f64,
    duration: f64,
}

impl PathTrancheMetrics {
    fn from_cashflows(
        cashflows: &crate::instruments::fixed_income::structured_credit::TrancheCashflows,
        as_of: Date,
        discount_curve: &finstack_core::market_data::term_structures::DiscountCurve,
    ) -> Result<Self> {
        let mut pv = 0.0;
        let mut positive_pv = 0.0;
        let mut weighted_duration = 0.0;
        for (date, amount) in &cashflows.cashflows {
            if *date <= as_of {
                continue;
            }
            let df = discount_curve.df_between_dates(as_of, *date)?;
            let flow_pv = amount.amount() * df;
            pv += flow_pv;
            if flow_pv > 0.0 {
                let t =
                    DayCount::Act365F.year_fraction(as_of, *date, DayCountContext::default())?;
                positive_pv += flow_pv;
                weighted_duration += flow_pv * t;
            }
        }

        let mut principal = 0.0;
        let mut weighted_principal_time = 0.0;
        for (date, amount) in &cashflows.principal_flows {
            if *date <= as_of || amount.amount() <= 0.0 {
                continue;
            }
            let t = DayCount::Act365F.year_fraction(as_of, *date, DayCountContext::default())?;
            principal += amount.amount();
            weighted_principal_time += amount.amount() * t;
        }

        Ok(Self {
            pv,
            loss: cashflows.total_writedown.amount(),
            wal: if principal > f64::EPSILON {
                weighted_principal_time / principal
            } else {
                0.0
            },
            duration: if positive_pv > f64::EPSILON {
                weighted_duration / positive_pv
            } else {
                0.0
            },
        })
    }
}

struct TrancheScenarioStats {
    tranche_id: String,
    seniority: String,
    attachment: f64,
    detachment: f64,
    pv_sum: f64,
    pv_sq_sum: f64,
    loss_sum: f64,
    loss_sq_sum: f64,
    losses: Vec<f64>,
    wal_sum: f64,
    duration_sum: f64,
}

impl TrancheScenarioStats {
    fn new(tranche: &Tranche, num_paths: usize) -> Self {
        Self {
            tranche_id: tranche.id.to_string(),
            seniority: format!("{:?}", tranche.seniority),
            attachment: tranche.attachment_point / 100.0,
            detachment: tranche.detachment_point / 100.0,
            pv_sum: 0.0,
            pv_sq_sum: 0.0,
            loss_sum: 0.0,
            loss_sq_sum: 0.0,
            losses: Vec::with_capacity(num_paths),
            wal_sum: 0.0,
            duration_sum: 0.0,
        }
    }

    fn record(&mut self, metrics: PathTrancheMetrics) {
        self.pv_sum += metrics.pv;
        self.pv_sq_sum += metrics.pv * metrics.pv;
        self.loss_sum += metrics.loss;
        self.loss_sq_sum += metrics.loss * metrics.loss;
        self.losses.push(metrics.loss);
        self.wal_sum += metrics.wal;
        self.duration_sum += metrics.duration;
    }

    fn finalize(
        mut self,
        currency: finstack_core::currency::Currency,
        num_paths: usize,
        es_confidence: f64,
    ) -> TranchePricingResult {
        let paths = num_paths.max(1) as f64;
        let mean_pv = self.pv_sum / paths;
        let mean_loss = self.loss_sum / paths;
        let loss_variance = (self.loss_sq_sum / paths) - mean_loss * mean_loss;
        let es = expected_shortfall(&mut self.losses, es_confidence);

        TranchePricingResult::new(
            self.tranche_id,
            self.seniority,
            Money::new(mean_pv, currency),
        )
        .with_subordination(self.attachment, self.detachment)
        .with_risk_metrics(
            Money::new(mean_loss, currency),
            Money::new(loss_variance.max(0.0).sqrt(), currency),
            Money::new(es, currency),
        )
        .with_average_life(self.wal_sum / paths)
        .with_credit_duration(self.duration_sum / paths)
    }
}

struct ScenarioCollector {
    currency: finstack_core::currency::Currency,
    num_paths: usize,
    deal_pv_sum: f64,
    deal_pv_sq_sum: f64,
    deal_loss_sum: f64,
    deal_loss_sq_sum: f64,
    deal_losses: Vec<f64>,
    tranche_stats: Vec<TrancheScenarioStats>,
}

impl ScenarioCollector {
    fn new(instrument: &StructuredCredit, num_paths: usize) -> Result<Self> {
        if num_paths == 0 {
            return Err(finstack_core::Error::Validation(
                "stochastic scenario collector requires at least one path".to_string(),
            ));
        }
        Ok(Self {
            currency: instrument.pool.base_currency(),
            num_paths,
            deal_pv_sum: 0.0,
            deal_pv_sq_sum: 0.0,
            deal_loss_sum: 0.0,
            deal_loss_sq_sum: 0.0,
            deal_losses: Vec::with_capacity(num_paths),
            tranche_stats: instrument
                .tranches
                .tranches
                .iter()
                .map(|tranche| TrancheScenarioStats::new(tranche, num_paths))
                .collect(),
        })
    }

    fn record_tranche(&mut self, idx: usize, metrics: PathTrancheMetrics) {
        if let Some(stats) = self.tranche_stats.get_mut(idx) {
            stats.record(metrics);
        }
    }

    fn record_deal(&mut self, pv: f64, loss: f64) {
        self.deal_pv_sum += pv;
        self.deal_pv_sq_sum += pv * pv;
        self.deal_loss_sum += loss;
        self.deal_loss_sq_sum += loss * loss;
        self.deal_losses.push(loss);
    }

    fn finalize(
        mut self,
        pricer: &StochasticPricer,
        pricing_mode: &str,
    ) -> StochasticPricingResult {
        let paths = self.num_paths as f64;
        let mean_pv = self.deal_pv_sum / paths;
        let mean_loss = self.deal_loss_sum / paths;
        let pv_variance = (self.deal_pv_sq_sum / paths) - mean_pv * mean_pv;
        let loss_variance = (self.deal_loss_sq_sum / paths) - mean_loss * mean_loss;
        let std_error = pv_variance.max(0.0).sqrt() / paths.sqrt();
        let es = expected_shortfall(&mut self.deal_losses, pricer.config.es_confidence);

        let mut result = StochasticPricingResult::new(
            Money::new(mean_pv, self.currency),
            Money::new(mean_loss, self.currency),
            self.num_paths,
        )
        .with_unexpected_loss(Money::new(loss_variance.max(0.0).sqrt(), self.currency))
        .with_expected_shortfall(Money::new(es, self.currency), pricer.config.es_confidence);

        let notional = pricer.config.tree_config.initial_balance;
        if notional > f64::EPSILON {
            result.clean_price = mean_pv / notional * 100.0;
            result.dirty_price = result.clean_price;
        }
        result.pv_std_error = std_error;
        result.pv_confidence_interval = (mean_pv - 1.96 * std_error, mean_pv + 1.96 * std_error);
        result.pricing_mode = pricing_mode.to_string();
        result.tranche_results = self
            .tranche_stats
            .into_iter()
            .map(|stats| stats.finalize(self.currency, self.num_paths, pricer.config.es_confidence))
            .collect();

        result
    }
}

fn expected_shortfall(losses: &mut [f64], confidence: f64) -> f64 {
    if losses.is_empty() {
        return 0.0;
    }
    losses.sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));
    let tail = (1.0 - confidence).clamp(0.0, 1.0);
    let tail_count = (tail * losses.len() as f64).ceil().max(1.0) as usize;
    let tail_count = tail_count.min(losses.len());
    losses.iter().take(tail_count).sum::<f64>() / tail_count as f64
}

trait TrancheDurationSetter {
    fn with_credit_duration(self, duration: f64) -> Self;
}

impl TrancheDurationSetter for TranchePricingResult {
    fn with_credit_duration(mut self, duration: f64) -> Self {
        self.credit_duration = duration;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::structured_credit::pricing::stochastic::tree::{
        BranchingSpec, ScenarioTreeConfig,
    };
    use crate::instruments::fixed_income::structured_credit::{
        DealType, DefaultModelSpec, Pool, PoolAsset, RecoveryModelSpec, Seniority, Tranche,
        TrancheCoupon, TrancheStructure,
    };
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::Money;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 1).expect("valid date")
    }

    fn test_discount_curve() -> std::sync::Arc<DiscountCurve> {
        std::sync::Arc::new(
            DiscountCurve::builder("USD-OIS")
                .base_date(test_date())
                .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
                .build()
                .expect("curve"),
        )
    }

    fn test_instrument() -> StructuredCredit {
        let maturity = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");
        let mut pool = Pool::new("POOL", DealType::ABS, Currency::USD);
        pool.assets.push(PoolAsset::fixed_rate_bond(
            "A1",
            Money::new(1_000_000.0, Currency::USD),
            0.06,
            maturity,
            DayCount::Thirty360,
        ));
        let tranche = Tranche::new(
            "A",
            0.0,
            100.0,
            Seniority::Senior,
            Money::new(1_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            maturity,
        )
        .expect("tranche");
        let mut instrument = StructuredCredit::new_abs(
            "ABS",
            pool,
            TrancheStructure::new(vec![tranche]).expect("structure"),
            test_date(),
            maturity,
            "USD-OIS",
        )
        .with_payment_calendar("nyse");
        instrument.credit_model.default_spec = DefaultModelSpec::constant_cdr(0.0);
        instrument.credit_model.recovery_spec = RecoveryModelSpec::with_lag(0.40, 0);
        instrument
    }

    #[test]
    fn monte_carlo_one_path_prices_waterfall_cashflows() {
        let instrument = test_instrument();
        let market = MarketContext::new().insert((*test_discount_curve()).clone());
        let config = StochasticPricerConfig::new(
            test_date(),
            test_discount_curve(),
            ScenarioTreeConfig::new(12, 1.0, BranchingSpec::fixed(2)),
        )
        .with_pricing_mode(PricingMode::MonteCarlo {
            num_paths: 1,
            antithetic: false,
        });
        let pricer = StochasticPricer::new(config);

        let result = pricer.price(&instrument, &market).expect("price");

        assert_eq!(result.num_paths, 1);
        assert_eq!(result.tranche_results.len(), 1);
        assert!(result.npv.amount().is_finite());
    }

    #[test]
    fn hybrid_mode_errors_until_conditional_suffix_is_implemented() {
        let instrument = test_instrument();
        let market = MarketContext::new().insert((*test_discount_curve()).clone());
        let config = StochasticPricerConfig::new(
            test_date(),
            test_discount_curve(),
            ScenarioTreeConfig::new(12, 1.0, BranchingSpec::fixed(2)),
        )
        .with_pricing_mode(PricingMode::Hybrid {
            tree_periods: 3,
            mc_paths: 100,
        });
        let pricer = StochasticPricer::new(config);

        let err = pricer
            .price(&instrument, &market)
            .expect_err("hybrid should fail");

        assert!(err.to_string().contains("Hybrid mode"));
    }
}
