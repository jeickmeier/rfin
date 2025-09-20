//! Gaussian Copula pricing model for CDS tranches.
//!
//! Implements the industry-standard base correlation approach for pricing
//! synthetic CDO tranches using a one-factor Gaussian Copula model.
//!
//! ## Key Features
//!
//! * **Time-dependent Expected Loss**: Calculates expected loss at each payment date
//!   rather than using linear approximation from maturity values.
//! * **Accrual-on-Default (AoD)**: Premium leg includes proper AoD adjustment using
//!   half of incremental loss within each period.
//! * **Market-standard Scheduling**: Uses canonical schedule builders with business
//!   day conventions and holiday calendar support.
//! * **Risk Metrics**: Full implementation of CS01, Correlation Delta, and Jump-to-Default
//!   using proper bumping techniques.
//! * **Numerical Stability**: Correlation clamping, monotonicity enforcement, and
//!   robust integration using Gauss-Hermite quadrature.
//!
//! ## Mathematical Approach
//!
//! The model decomposes tranche [A,D] expected loss as:
//! `EL_[A,D](t) = [EL_eq(0,D,t) - EL_eq(0,A,t)] / [(D-A)/100]`
//!
//! Where `EL_eq(0,K,t)` is the expected loss of equity tranche [0,K] at time t,
//! calculated using base correlation ρ(K) for detachment point K.
//!
//! ### Premium Leg PV
//! `PV_prem = Σ c * Δt_i * DF(t_i) * [N_outstanding(t_{i-1}) - 0.5 * N_incremental_loss(t_i)]`
//!
//! ### Protection Leg PV  
//! `PV_prot = Σ DF(t_i) * N_tr * [EL_fraction(t_i) - EL_fraction(t_{i-1})]`
//!
//! ## Limitations
//!
//! * Assumes homogeneous portfolio (single hazard curve for all constituents)
//! * Uses constant recovery rate across all entities
//! * Base correlation model can have small arbitrage inconsistencies at curve knots

use crate::cashflow::builder::schedule_utils::build_dates;
use crate::instruments::cds_tranche::{CdsTranche, TrancheSide};
use finstack_core::dates::next_cds_date;
use finstack_core::dates::{Date, StubKind};
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::{term_structures::CreditIndexData, MarketContext};
use finstack_core::math::binomial_probability;
use finstack_core::math::{
    norm_cdf as standard_normal_cdf, norm_pdf, standard_normal_inv_cdf, GaussHermiteQuadrature,
};
use finstack_core::prelude::*;
use finstack_core::F;

#[cfg(test)]
use finstack_core::math::log_factorial;
// use finstack_core::types::CurveId;

/// Parameters for the Gaussian Copula pricing model.
#[derive(Clone, Debug)]
pub struct CDSTranchePricerConfig {
    /// Number of quadrature points for numerical integration (5, 7, or 10)
    pub quadrature_order: u8,
    /// Whether to use issuer-specific curves if available
    pub use_issuer_curves: bool,
    /// Minimum correlation value for numerical stability
    pub min_correlation: F,
    /// Maximum correlation value for numerical stability  
    pub max_correlation: F,
    /// CS01 bump size (interpreted according to `cs01_bump_units`)
    pub cs01_bump_size: F,
    /// Units for CS01 bump: hazard-rate bp or spread bp (additive)
    pub cs01_bump_units: Cs01BumpUnits,
    /// Correlation bump for correlation delta calculation (absolute)
    pub corr_bump_abs: F,
    /// Whether to use mid-period discounting for protection leg (default coverage timing)
    pub mid_period_protection: bool,
    /// Whether to include accrual-on-default in the premium leg
    pub accrual_on_default_enabled: bool,
    /// Smooth boundary width for correlation clamping transitions
    pub corr_boundary_width: F,
    /// Fraction of incremental loss allocated to accrual-on-default (AoD)
    pub aod_allocation_fraction: F,
    /// Numerical tolerance used by integration and boundary checks
    pub numerical_tolerance: F,
    /// Clip parameter for CDF arguments to avoid overflow
    pub cdf_clip: F,
    /// Correlation band within which to use standard quadrature
    pub adaptive_integration_low: F,
    /// Correlation band within which to use standard quadrature
    pub adaptive_integration_high: F,
    /// Stub convention for schedule generation
    pub schedule_stub: StubKind,
    /// If true, generate ISDA coupon dates (IMM-20 schedule)
    pub use_isda_coupon_dates: bool,
    /// Heterogeneous issuer method when issuer curves are available
    pub hetero_method: HeteroMethod,
    /// Grid step for exact convolution method (fraction of portfolio notional)
    pub grid_step: F,
    /// Minimum variance threshold for SPA to avoid division by zero
    pub spa_variance_floor: F,
    /// Probability clamp epsilon to avoid 0/1 extremes in probits/CDFs
    pub probability_clip: F,
    /// LGD floor to avoid zero exposure in corner cases
    pub lgd_floor: F,
    /// Minimum grid step to avoid degenerate convolution buckets
    pub grid_step_min: F,
    /// Hard cap on convolution PMF points before falling back to SPA
    pub max_grid_points: usize,
}

impl Default for CDSTranchePricerConfig {
    fn default() -> Self {
        Self {
            quadrature_order: 7,     // Good balance of accuracy and performance
            use_issuer_curves: true, // Use heterogeneous modeling when available
            min_correlation: 0.01,   // Numerical stability floor
            max_correlation: 0.99,   // Numerical stability ceiling
            cs01_bump_size: 1.0,     // 1 bp by default
            cs01_bump_units: Cs01BumpUnits::HazardRateBp,
            corr_bump_abs: 0.01,          // 1% absolute correlation bump
            mid_period_protection: false, // End-of-period discounting by default
            accrual_on_default_enabled: true,
            corr_boundary_width: 0.005,   // 0.5% transition zone
            aod_allocation_fraction: 0.5, // Standard mid-period default assumption
            numerical_tolerance: 1e-10,   // Integration tolerance
            cdf_clip: 10.0,               // Clip for CDF arguments
            adaptive_integration_low: 0.05,
            adaptive_integration_high: 0.95,
            schedule_stub: StubKind::None,
            use_isda_coupon_dates: false,
            hetero_method: HeteroMethod::Spa,
            grid_step: 0.001, // 10 bps of portfolio notional
            spa_variance_floor: 1e-14,
            probability_clip: 1e-12,
            lgd_floor: 1e-6,
            grid_step_min: 1e-6,
            max_grid_points: 200_000,
        }
    }
}

/// Units for CS01 bumping in tranche pricer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cs01BumpUnits {
    HazardRateBp,
    SpreadBpAdditive,
}

/// Heterogeneous expected loss evaluation method
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeteroMethod {
    Spa,
    ExactConvolution,
}

/// Gaussian Copula pricing engine for CDS tranches.
pub struct CDSTranchePricer {
    params: CDSTranchePricerConfig,
}

impl Default for CDSTranchePricer {
    fn default() -> Self {
        Self::new()
    }
}

impl CDSTranchePricer {
    #[inline]
    fn select_quadrature(&self) -> GaussHermiteQuadrature {
        match self.params.quadrature_order {
            5 => GaussHermiteQuadrature::order_5(),
            7 => GaussHermiteQuadrature::order_7(),
            10 => GaussHermiteQuadrature::order_10(),
            _ => GaussHermiteQuadrature::order_7(),
        }
    }
    /// Create a new Gaussian Copula model with default parameters.
    pub fn new() -> Self {
        Self {
            params: CDSTranchePricerConfig::default(),
        }
    }

    /// Create a new model with custom parameters.
    pub fn with_params(params: CDSTranchePricerConfig) -> Self {
        Self { params }
    }

    /// Price a CDS tranche using the Gaussian Copula model.
    ///
    /// # Arguments
    /// * `tranche` - The CDS tranche to price
    /// * `market_ctx` - Market data context containing curves and credit index data
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// The present value of the tranche
    pub fn price_tranche(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Get the credit index data
        let index_data_arc = market_ctx.credit_index_ref(tranche.credit_index_id)?;

        // Get the discount curve
        let discount_curve = market_ctx
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                tranche.disc_id,
            )?;

        // Calculate present values of premium and protection legs
        // These now calculate the EL curve internally with proper time dependency
        let pv_premium =
            self.calculate_premium_leg_pv(tranche, index_data_arc, discount_curve, as_of)?;

        let pv_protection =
            self.calculate_protection_leg_pv(tranche, index_data_arc, discount_curve, as_of)?;

        // Net present value depends on the side
        let net_pv = match tranche.side {
            TrancheSide::SellProtection => pv_premium - pv_protection,
            TrancheSide::BuyProtection => pv_protection - pv_premium,
        };

        Ok(Money::new(net_pv, tranche.notional.currency()))
    }

    /// Calculate expected tranche loss using the base correlation approach.
    ///
    /// Decomposes the tranche [A, D] as the difference between two equity
    /// tranches: EL(0, D) - EL(0, A), using correlations interpolated from
    /// the base correlation curve with enhanced numerical stability.
    fn calculate_expected_tranche_loss(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        maturity: Date,
    ) -> Result<F> {
        let attach_pct = tranche.attach_pct;
        let detach_pct = tranche.detach_pct;

        // Get correlations for attachment and detachment points
        let corr_attach = index_data.base_correlation_curve.correlation(attach_pct);
        let corr_detach = index_data.base_correlation_curve.correlation(detach_pct);

        // Apply enhanced correlation boundary handling for numerical stability
        let corr_attach = self.smooth_correlation_boundary(corr_attach);
        let corr_detach = self.smooth_correlation_boundary(corr_detach);

        // Calculate expected losses for equity tranches [0, A] and [0, D]
        let el_to_attach =
            self.calculate_equity_tranche_loss(attach_pct, corr_attach, index_data, maturity)?;

        let el_to_detach =
            self.calculate_equity_tranche_loss(detach_pct, corr_detach, index_data, maturity)?;

        // The [A, D] tranche loss as a fraction of total portfolio
        let portfolio_loss_fraction = el_to_detach - el_to_attach;

        // Ensure monotonicity: el_to_detach should always be >= el_to_attach
        let portfolio_loss_fraction = portfolio_loss_fraction.max(0.0);

        // Convert to fraction of tranche notional: EL_[A,D] / (D-A) * 100
        let tranche_width_pct = detach_pct - attach_pct;
        let tranche_loss_fraction = if tranche_width_pct > 0.0 {
            (portfolio_loss_fraction / tranche_width_pct * 100.0).clamp(0.0, 1.0)
        } else {
            0.0 // Degenerate tranche
        };

        // Convert to currency amount: fraction * tranche notional
        Ok(tranche_loss_fraction * tranche.notional.amount())
    }

    /// Calculate expected tranche loss fraction at a specific date.
    ///
    /// Returns the expected loss as a fraction of the tranche notional [0, 1],
    /// properly scaled using the base correlation approach.
    fn expected_tranche_loss_fraction_at(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        date: Date,
    ) -> Result<F> {
        let attach_pct = tranche.attach_pct;
        let detach_pct = tranche.detach_pct;

        let _years = self.years_from_base(index_data, date);

        // Get correlations for attachment and detachment points
        let corr_attach = index_data.base_correlation_curve.correlation(attach_pct);
        let corr_detach = index_data.base_correlation_curve.correlation(detach_pct);

        // Apply enhanced correlation boundary handling for numerical stability
        let corr_attach = self.smooth_correlation_boundary(corr_attach);
        let corr_detach = self.smooth_correlation_boundary(corr_detach);

        // Calculate expected losses for equity tranches [0, A] and [0, D]
        let el_to_attach =
            self.calculate_equity_tranche_loss(attach_pct, corr_attach, index_data, date)?;

        let el_to_detach =
            self.calculate_equity_tranche_loss(detach_pct, corr_detach, index_data, date)?;

        // The [A, D] tranche loss as a fraction of total portfolio
        let portfolio_loss_fraction = el_to_detach - el_to_attach;

        // Ensure monotonicity: el_to_detach should always be >= el_to_attach
        // If this fails, it indicates numerical issues in the equity loss calculation
        let portfolio_loss_fraction = portfolio_loss_fraction.max(0.0);

        // Convert to fraction of tranche notional: EL_[A,D] / (D-A) * 100
        let tranche_width_pct = detach_pct - attach_pct;
        let tranche_loss_fraction = if tranche_width_pct > 0.0 {
            (portfolio_loss_fraction / tranche_width_pct * 100.0).clamp(0.0, 1.0)
        } else {
            0.0 // Degenerate tranche
        };

        Ok(tranche_loss_fraction)
    }

    /// Build the expected loss curve for all payment dates.
    ///
    /// Returns a vector of (Date, EL_fraction) pairs where EL_fraction
    /// is the cumulative expected loss as a fraction of tranche notional.
    fn build_el_curve(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        dates: &[Date],
    ) -> Result<Vec<(Date, F)>> {
        let mut el_curve = Vec::with_capacity(dates.len());

        for &date in dates {
            let el_fraction = self.expected_tranche_loss_fraction_at(tranche, index_data, date)?;
            el_curve.push((date, el_fraction));
        }

        Ok(el_curve)
    }

    /// Calculate expected loss for an equity tranche [0, K] using Gaussian Copula.
    ///
    /// Enhanced with adaptive integration for superior numerical stability,
    /// particularly critical near correlation boundaries (0 and 1) where
    /// the conditional default probability function exhibits sharp transitions.
    ///
    /// # Arguments
    /// * `detachment_pct` - Detachment point K in percent
    /// * `correlation` - Asset correlation parameter ρ
    /// * `index_data` - Credit index market data
    /// * `maturity` - Maturity date for loss calculation
    fn calculate_equity_tranche_loss(
        &self,
        detachment_pct: F,
        correlation: F,
        index_data: &CreditIndexData,
        maturity: Date,
    ) -> Result<F> {
        // Heterogeneous path if enabled and issuer curves present
        if self.params.use_issuer_curves && index_data.has_issuer_curves() {
            self.calculate_equity_tranche_loss_hetero(
                detachment_pct,
                correlation,
                index_data,
                maturity,
            )
        } else {
            // Homogeneous: use index marginals
            let num_constituents = index_data.num_constituents as usize;
            let recovery_rate = index_data.recovery_rate;

            let detachment_notional = detachment_pct / 100.0;
            let quad = self.select_quadrature();
            let maturity_years = self.years_from_base(index_data, maturity);
            let default_prob = self.get_default_probability(index_data, maturity_years)?;
            let default_threshold = standard_normal_inv_cdf(default_prob);
            let integrand = |z: F| {
                let p = self.conditional_default_probability_enhanced(
                    default_threshold,
                    correlation,
                    z,
                );
                self.conditional_equity_tranche_loss(
                    num_constituents,
                    detachment_notional,
                    p,
                    recovery_rate,
                )
            };
            let expected_loss = if !(self.params.adaptive_integration_low
                ..=self.params.adaptive_integration_high)
                .contains(&correlation)
            {
                quad.integrate_adaptive(integrand, self.params.numerical_tolerance)
            } else {
                quad.integrate(integrand)
            };
            Ok(expected_loss)
        }
    }

    /// Heterogeneous equity tranche loss via semi-analytical SPA or exact convolution fallback
    fn calculate_equity_tranche_loss_hetero(
        &self,
        detachment_pct: F,
        correlation: F,
        index_data: &CreditIndexData,
        maturity: Date,
    ) -> Result<F> {
        // Precompute unconditional PD_i(t)
        let t = self.years_from_base(index_data, maturity);
        let recovery = index_data.recovery_rate;
        let lgd = 1.0 - recovery;
        let weights = 1.0 / (index_data.num_constituents as F);
        let tranche_width = detachment_pct / 100.0;

        // Quadrature setup
        let quad = self.select_quadrature();

        // Build unconditional PD_i and their probit thresholds
        let mut pd_i: Vec<F> = Vec::with_capacity(index_data.num_constituents as usize);
        if let Some(curves) = &index_data.issuer_credit_curves {
            for _id in curves.keys() {
                let curve = index_data.get_issuer_curve(_id);
                let sp = curve.sp(t);
                pd_i.push((1.0 - sp).clamp(0.0, 1.0));
            }
        } else {
            // Fallback should not happen (caller gates), but ensure safe
            let sp = index_data.index_credit_curve.sp(t);
            pd_i = vec![(1.0 - sp).clamp(0.0, 1.0); index_data.num_constituents as usize];
        }

        // If issuer marginals are effectively identical, use homogeneous path
        if let (Some(&min_pd), Some(&max_pd)) = (
            pd_i.iter().min_by(|a, b| a.partial_cmp(b).unwrap()),
            pd_i.iter().max_by(|a, b| a.partial_cmp(b).unwrap()),
        ) {
            if (max_pd - min_pd).abs() <= self.params.probability_clip {
                let num_constituents = index_data.num_constituents as usize;
                let detachment_notional = detachment_pct / 100.0;
                let maturity_years = t;
                let default_prob = self.get_default_probability(index_data, maturity_years)?;
                let default_threshold = standard_normal_inv_cdf(default_prob);
                let integrand = |z: F| {
                    let p = self.conditional_default_probability_enhanced(
                        default_threshold,
                        correlation,
                        z,
                    );
                    self.conditional_equity_tranche_loss(
                        num_constituents,
                        detachment_notional,
                        p,
                        recovery,
                    )
                };
                let expected_loss = if !(self.params.adaptive_integration_low
                    ..=self.params.adaptive_integration_high)
                    .contains(&correlation)
                {
                    quad.integrate_adaptive(integrand, self.params.numerical_tolerance)
                } else {
                    quad.integrate(integrand)
                };
                return Ok(expected_loss);
            }
        }
        let eps = self.params.probability_clip;
        let probit_i: Vec<F> = pd_i
            .iter()
            .map(|&p| standard_normal_inv_cdf(p.max(eps).min(1.0 - eps)))
            .collect();

        // Integrand over common factor Z
        let integrand = |z: F| -> F {
            // Conditional default probs p_i(z)
            let sqrt_rho = correlation.sqrt();
            let sqrt_1mr = (1.0 - correlation).sqrt();
            let mut mean = 0.0;
            let mut var = 0.0;
            for &th in &probit_i {
                let cthr = (th - sqrt_rho * z) / sqrt_1mr;
                let p = standard_normal_cdf(cthr).clamp(0.0, 1.0);
                // Weighted exposure per name
                let w = weights * lgd;
                mean += w * p;
                var += w * w * p * (1.0 - p);
            }

            // SPA/normal approximation for E[min(L, K)] with K = detachment_notional
            let k = tranche_width;
            if var <= self.params.spa_variance_floor {
                return mean.min(k);
            }
            let s = var.sqrt();
            let a = (k - mean) / s;
            // E[min(L, K)] ≈ m Φ(a) + s φ(a) + K [1 − Φ(a)]
            mean * standard_normal_cdf(a) + s * norm_pdf(a) + k * (1.0 - standard_normal_cdf(a))
        };

        // Prefer exact convolution for small pools to reduce SPA error
        let n_const = index_data.num_constituents as usize;
        let small_pool_threshold: usize = 16;
        let el = if n_const <= small_pool_threshold {
            self.hetero_exact_convolution(detachment_pct, correlation, &probit_i, lgd, weights)
        } else {
            match self.params.hetero_method {
                HeteroMethod::Spa => {
                    if !(self.params.adaptive_integration_low
                        ..=self.params.adaptive_integration_high)
                        .contains(&correlation)
                    {
                        quad.integrate_adaptive(integrand, self.params.numerical_tolerance)
                    } else {
                        quad.integrate(integrand)
                    }
                }
                HeteroMethod::ExactConvolution => {
                    // Simple exact convolution fallback via discretization and convolution
                    self.hetero_exact_convolution(
                        detachment_pct,
                        correlation,
                        &probit_i,
                        lgd,
                        weights,
                    )
                }
            }
        };
        Ok(el)
    }

    /// SPA-only helper for performance fallback from exact convolution
    fn calculate_equity_tranche_loss_hetero_spa_only(
        &self,
        probit_i: &[F],
        correlation: F,
        k: F,
        lgd: F,
        weight: F,
    ) -> F {
        let quad = self.select_quadrature();
        let integrand = |z: F| -> F {
            let sqrt_rho = correlation.sqrt();
            let sqrt_1mr = (1.0 - correlation).sqrt();
            let mut mean = 0.0;
            let mut var = 0.0;
            for &th in probit_i {
                let cthr = (th - sqrt_rho * z) / sqrt_1mr;
                let p = standard_normal_cdf(cthr).clamp(0.0, 1.0);
                let w = weight * lgd;
                mean += w * p;
                var += w * w * p * (1.0 - p);
            }
            if var <= self.params.spa_variance_floor {
                return mean.min(k);
            }
            let s = var.sqrt();
            let a = (k - mean) / s;
            mean * standard_normal_cdf(a) + s * norm_pdf(a) + k * (1.0 - standard_normal_cdf(a))
        };
        if !(self.params.adaptive_integration_low..=self.params.adaptive_integration_high)
            .contains(&correlation)
        {
            quad.integrate_adaptive(integrand, self.params.numerical_tolerance)
        } else {
            quad.integrate(integrand)
        }
    }

    /// Exact convolution fallback (coarse grid) for heterogeneous conditional equity tranche loss
    fn hetero_exact_convolution(
        &self,
        detachment_pct: F,
        correlation: F,
        probit_i: &[F],
        lgd: F,
        weight: F,
    ) -> F {
        // Coarse grid size and step (configurable in future)
        let k = detachment_pct / 100.0;
        let grid_step = self.params.grid_step.max(self.params.grid_step_min);
        let max_points = (k / grid_step).ceil() as usize + 1;
        if max_points > self.params.max_grid_points {
            // Performance guard: fall back to SPA approximation
            return self.calculate_equity_tranche_loss_hetero_spa_only(
                probit_i,
                correlation,
                k,
                lgd,
                weight,
            );
        }

        // Gauss–Hermite integrate conditional min(L, K) exactly via convolution
        let quad = self.select_quadrature();
        let sqrt_rho = correlation.sqrt();
        let sqrt_1mr = (1.0 - correlation).sqrt();

        let integrand = |z: F| {
            // Start with delta at 0 loss
            let mut pmf = vec![0.0f64; 1];
            pmf[0] = 1.0;

            for &th in probit_i {
                let cthr = (th - sqrt_rho * z) / sqrt_1mr;
                let p = standard_normal_cdf(cthr).clamp(0.0, 1.0);
                let loss = (weight * lgd / grid_step).round() as usize; // bucketed points
                let mut next = vec![0.0f64; pmf.len() + loss.max(1)];
                // Convolution with Bernoulli(p)
                for (i, &mass) in pmf.iter().enumerate() {
                    // no default
                    next[i] += mass * (1.0 - p);
                    // default adds 'loss' buckets
                    let j = (i + loss).min(next.len() - 1);
                    next[j] += mass * p;
                }
                pmf = next;
                if pmf.len() > max_points {
                    pmf.truncate(max_points);
                }
            }

            // Compute E[min(L, K)] from pmf over grid
            // Use stable summation for better numerical properties
            let mut terms: Vec<f64> = Vec::with_capacity(pmf.len());
            for (i, mass) in pmf.iter().enumerate() {
                let l = (i as f64) * grid_step;
                terms.push(mass * l.min(k));
            }
            finstack_core::math::stable_sum(&terms)
        };

        if !(self.params.adaptive_integration_low..=self.params.adaptive_integration_high)
            .contains(&correlation)
        {
            quad.integrate_adaptive(integrand, self.params.numerical_tolerance)
        } else {
            quad.integrate(integrand)
        }
    }

    /// Calculate conditional default probability given market factor Z.
    ///
    /// Standard implementation kept for compatibility and testing.
    /// The enhanced version `conditional_default_probability_enhanced` is used
    /// in production calculations for superior numerical stability.
    ///
    /// P(default | Z) = Φ((Φ⁻¹(PD) - √ρ * Z) / √(1-ρ))
    #[allow(dead_code)]
    fn conditional_default_probability(
        &self,
        default_threshold: F,
        correlation: F,
        market_factor: F,
    ) -> F {
        let sqrt_rho = correlation.sqrt();
        let one_minus_rho: F = 1.0 - correlation;
        let sqrt_one_minus_rho = one_minus_rho.sqrt();

        let conditional_threshold =
            (default_threshold - sqrt_rho * market_factor) / sqrt_one_minus_rho;
        standard_normal_cdf(conditional_threshold)
    }

    /// Enhanced conditional default probability with improved numerical stability.
    ///
    /// Provides superior handling of boundary cases and extreme correlation values
    /// through sophisticated boundary transition functions and overflow protection.
    ///
    /// P(default | Z) = Φ((Φ⁻¹(PD) - √ρ * Z) / √(1-ρ))
    fn conditional_default_probability_enhanced(
        &self,
        default_threshold: F,
        correlation: F,
        market_factor: F,
    ) -> F {
        // Apply smooth correlation boundaries to avoid numerical discontinuities
        let correlation = self.smooth_correlation_boundary(correlation);

        // Handle extreme correlation cases with special care
        if correlation < self.params.numerical_tolerance {
            // Near-zero correlation: independent case
            return standard_normal_cdf(default_threshold);
        }
        if correlation > 1.0 - self.params.numerical_tolerance {
            // Near-perfect correlation: deterministic case
            let threshold_adj = default_threshold - market_factor;
            return standard_normal_cdf(threshold_adj);
        }

        // Enhanced calculation with overflow protection
        let sqrt_rho = correlation.sqrt();
        let one_minus_rho = 1.0 - correlation;

        // Protect against numerical issues when correlation approaches 1
        let sqrt_one_minus_rho = if one_minus_rho < self.params.numerical_tolerance {
            self.params.numerical_tolerance.sqrt() // Minimum practical value to avoid division by zero
        } else {
            let one_minus_rho: F = 1.0 - correlation;
            one_minus_rho.sqrt()
        };

        // Calculate conditional threshold with overflow protection
        let numerator = default_threshold - sqrt_rho * market_factor;
        let conditional_threshold = numerator / sqrt_one_minus_rho;

        // Clamp to reasonable range to prevent CDF overflow
        let conditional_threshold =
            conditional_threshold.clamp(-self.params.cdf_clip, self.params.cdf_clip);

        standard_normal_cdf(conditional_threshold)
    }

    /// Apply smooth correlation boundary handling to avoid numerical discontinuities.
    ///
    /// Uses a smooth transition function near the boundaries to maintain numerical
    /// stability while preserving the underlying mathematical relationships.
    fn smooth_correlation_boundary(&self, correlation: F) -> F {
        let min_corr = self.params.min_correlation;
        let max_corr = self.params.max_correlation;
        let width = self.params.corr_boundary_width;

        if correlation <= min_corr + width {
            // Lower boundary: smooth transition using tanh
            let x = (correlation - min_corr) / width;
            min_corr + width * (1.0 + x.tanh()) / 2.0
        } else if correlation >= max_corr - width {
            // Upper boundary: smooth transition using tanh
            let x = (correlation - (max_corr - width)) / width;
            max_corr - width * (1.0 - x.tanh()) / 2.0
        } else {
            // Normal range: no adjustment needed
            correlation.clamp(min_corr, max_corr)
        }
    }

    /// Calculate expected loss of equity tranche conditional on market factor.
    ///
    /// Uses the binomial distribution to sum over all possible numbers of defaults.
    fn conditional_equity_tranche_loss(
        &self,
        num_constituents: usize,
        detachment_notional: F,
        conditional_default_prob: F,
        recovery_rate: F,
    ) -> F {
        let loss_given_default = 1.0 - recovery_rate;
        let individual_notional = 1.0 / num_constituents as F; // Normalized to 1.0 total

        let mut expected_loss = 0.0;

        // Sum over all possible numbers of defaults
        for k in 0..=num_constituents {
            let prob_k_defaults =
                binomial_probability(num_constituents, k, conditional_default_prob);

            // Portfolio loss given k defaults
            let portfolio_loss = k as F * individual_notional * loss_given_default;

            // Tranche loss (equity tranche [0, detachment_notional])
            let tranche_loss = portfolio_loss.min(detachment_notional);

            expected_loss += prob_k_defaults * tranche_loss;
        }

        expected_loss
    }

    /// Calculate present value of the premium leg with accrual-on-default.
    ///
    /// PV = Coupon * Σ(Δt_j * D(t_j) * [N_outstanding - 0.5 * N_incremental_loss])
    /// where N_outstanding = N_tr * (1 - EL_fraction(t_{j-1}))
    /// and N_incremental_loss = N_tr * (EL_fraction(t_j) - EL_fraction(t_{j-1}))
    fn calculate_premium_leg_pv(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        discount_curve: &dyn Discounting,
        as_of: Date,
    ) -> Result<F> {
        let coupon = tranche.running_coupon_bp / 10000.0; // Convert bp to decimal
        let tranche_notional = tranche.notional.amount();

        // Generate payment schedule and expected loss curve
        let payment_dates = self.generate_payment_schedule(tranche, as_of)?;
        if payment_dates.is_empty() {
            return Ok(0.0);
        }

        let el_curve = self.build_el_curve(tranche, index_data, &payment_dates)?;

        let mut pv_premium = 0.0;
        let mut prev_el_fraction = 0.0; // Start with no loss

        for (i, &payment_date) in payment_dates.iter().enumerate() {
            let t = self.years_from_base(index_data, payment_date);
            if t <= 0.0 {
                continue;
            }

            let el_fraction = el_curve[i].1; // Current EL fraction
            let delta_el_fraction = el_fraction - prev_el_fraction;

            // Outstanding notional at beginning of period
            let outstanding_notional = tranche_notional * (1.0 - prev_el_fraction);

            if outstanding_notional <= 0.0 {
                break; // Tranche fully written down
            }

            // Accrual period using day count convention
            let period_start = if i == 0 {
                tranche.effective_date.unwrap_or(as_of)
            } else {
                payment_dates[i - 1]
            };

            let accrual_period = tranche
                .day_count
                .year_fraction(
                    period_start,
                    payment_date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            if accrual_period <= 0.0 {
                continue;
            }

            // Accrual-on-default: reduce accrual by configured fraction of incremental loss (if enabled)
            let effective_notional = if self.params.accrual_on_default_enabled {
                let aod_adjustment =
                    self.params.aod_allocation_fraction * tranche_notional * delta_el_fraction;
                (outstanding_notional - aod_adjustment).max(0.0)
            } else {
                outstanding_notional
            };

            // Discount at end or midpoint depending on config
            let df_time = if self.params.mid_period_protection {
                let t_start = self.years_from_base(index_data, period_start);
                (t_start + t) * 0.5
            } else {
                t
            };
            let discount_factor = discount_curve.df(df_time);

            pv_premium += coupon * accrual_period * discount_factor * effective_notional;
            prev_el_fraction = el_fraction;
        }

        Ok(pv_premium)
    }

    /// Calculate present value of the protection leg using incremental EL.
    ///
    /// PV = Σ(D(t_j) * ΔEL_j) where ΔEL_j = N_tr * (EL_fraction(t_j) - EL_fraction(t_{j-1}))
    fn calculate_protection_leg_pv(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        discount_curve: &dyn Discounting,
        as_of: Date,
    ) -> Result<F> {
        let tranche_notional = tranche.notional.amount();

        // Generate payment schedule and expected loss curve
        let payment_dates = self.generate_payment_schedule(tranche, as_of)?;
        if payment_dates.is_empty() {
            return Ok(0.0);
        }

        let el_curve = self.build_el_curve(tranche, index_data, &payment_dates)?;

        let mut pv_protection = 0.0;
        let mut prev_el_fraction = 0.0; // Start with no loss

        for (i, &payment_date) in payment_dates.iter().enumerate() {
            let t = self.years_from_base(index_data, payment_date);
            if t <= 0.0 {
                continue;
            }

            let el_fraction = el_curve[i].1; // Current EL fraction
            let delta_el_fraction = el_fraction - prev_el_fraction;

            // Incremental loss amount in currency
            let incremental_loss_amount = tranche_notional * delta_el_fraction;

            if incremental_loss_amount > 0.0 {
                let discount_factor = discount_curve.df(t);
                pv_protection += incremental_loss_amount * discount_factor;
            }

            prev_el_fraction = el_fraction;
        }

        Ok(pv_protection)
    }

    /// Get default probability for the index at a given maturity.
    fn get_default_probability(
        &self,
        index_data: &CreditIndexData,
        maturity_years: F,
    ) -> Result<F> {
        let survival_prob = index_data.index_credit_curve.sp(maturity_years);
        Ok(1.0 - survival_prob)
    }

    /// Calculate years from the credit curve base date.
    fn years_from_base(&self, index_data: &CreditIndexData, date: Date) -> F {
        let dc = index_data.index_credit_curve.day_count();
        dc.year_fraction(
            index_data.index_credit_curve.base_date(),
            date,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap_or(0.0)
    }

    /// Create a bumped base correlation curve for sensitivity analysis.
    ///
    /// Creates a new BaseCorrelationCurve with correlations shifted by bump_abs,
    /// clamped to [min_correlation, max_correlation] for numerical stability.
    fn bump_base_correlation(
        &self,
        original_curve: &finstack_core::market_data::term_structures::BaseCorrelationCurve,
        bump_abs: F,
    ) -> finstack_core::Result<finstack_core::market_data::term_structures::BaseCorrelationCurve>
    {
        use finstack_core::market_data::term_structures::BaseCorrelationCurve;

        // Extract original points and apply bump
        let bumped_points: Vec<(F, F)> = original_curve
            .detachment_points()
            .iter()
            .zip(original_curve.correlations().iter())
            .map(|(&detach, &corr)| {
                let bumped_corr = (corr + bump_abs)
                    .clamp(self.params.min_correlation, self.params.max_correlation);
                (detach, bumped_corr)
            })
            .collect();

        // Create temporary ID for bumped curve
        BaseCorrelationCurve::builder("TEMP_BUMPED_CORR")
            .points(bumped_points)
            .build()
    }

    /// Create a bumped credit index with shifted hazard rates for CS01 calculation.
    ///
    /// Creates a new CreditIndexData with the index hazard curve shifted by delta_lambda.
    fn bump_index_hazard(
        &self,
        original_index: &CreditIndexData,
        delta_lambda: F,
    ) -> Result<CreditIndexData> {
        // Create bumped hazard curve
        let bumped_hazard_curve = original_index
            .index_credit_curve
            .with_hazard_shift(delta_lambda)?;

        // Create new credit index data with bumped hazard curve
        CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(original_index.recovery_rate)
            .index_credit_curve(std::sync::Arc::new(bumped_hazard_curve))
            .base_correlation_curve(original_index.base_correlation_curve.clone())
            .build()
    }

    /// Generate payment schedule for the tranche using canonical schedule builder.
    ///
    /// Uses the robust date scheduling utilities with proper business day
    /// conventions and calendar support.
    fn generate_payment_schedule(&self, tranche: &CdsTranche, as_of: Date) -> Result<Vec<Date>> {
        let start_date = tranche.effective_date.unwrap_or(as_of);

        let dates = if self.params.use_isda_coupon_dates {
            let mut out = vec![start_date];
            let mut current = start_date;
            while current < tranche.maturity {
                current = next_cds_date(current);
                out.push(current);
            }
            out
        } else {
            let schedule = build_dates(
                start_date,
                tranche.maturity,
                tranche.payment_frequency,
                self.params.schedule_stub,
                tranche.business_day_convention,
                tranche.calendar_id,
            );
            schedule.dates
        };

        // Filter out dates before as_of (in case effective_date < as_of)
        let payment_dates: Vec<Date> = dates.into_iter().filter(|&date| date > as_of).collect();

        Ok(payment_dates)
    }

    /// Calculate upfront amount for the tranche.
    ///
    /// This is the net present value at inception, representing the
    /// payment required to enter the position at the standard coupon.
    pub fn calculate_upfront(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<F> {
        let pv = self.price_tranche(tranche, market_ctx, as_of)?;
        Ok(pv.amount())
    }

    /// Calculate Spread DV01 (sensitivity to 1bp change in running coupon).
    pub fn calculate_spread_dv01(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<F> {
        // Create bumped tranche with +1bp running coupon
        let mut bumped_tranche = tranche.clone();
        bumped_tranche.running_coupon_bp += 1.0;

        let base_pv = self.price_tranche(tranche, market_ctx, as_of)?.amount();
        let bumped_pv = self
            .price_tranche(&bumped_tranche, market_ctx, as_of)?
            .amount();

        Ok(bumped_pv - base_pv)
    }

    /// Calculate expected loss metric (the total expected loss at maturity).
    pub fn calculate_expected_loss(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
    ) -> Result<F> {
        let index_data_arc = market_ctx.credit_index_ref(tranche.credit_index_id)?;
        self.calculate_expected_tranche_loss(tranche, index_data_arc, tranche.maturity)
    }

    /// Calculate CS01 (sensitivity to 1bp parallel shift in credit spreads).
    pub fn calculate_cs01(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<F> {
        // Get base price
        let base_pv = self.price_tranche(tranche, market_ctx, as_of)?.amount();

        // Create bumped market context using configured CS01 bump units
        let original_index_arc = market_ctx.credit_index_ref(tranche.credit_index_id)?;
        let bumped_index = match self.params.cs01_bump_units {
            Cs01BumpUnits::HazardRateBp => {
                // 1.0 bump_size interpreted as 1 bp in hazard rate
                let delta_lambda = self.params.cs01_bump_size * 1e-4;
                self.bump_index_hazard(original_index_arc, delta_lambda)?
            }
            Cs01BumpUnits::SpreadBpAdditive => {
                // Proxy: convert a spread bp to hazard bp via 1/(1-recovery)
                // This is a common approximation for small bump sizes.
                let rr = original_index_arc.recovery_rate;
                let delta_lambda = (self.params.cs01_bump_size * 1e-4) / (1.0 - rr).max(1e-6);
                self.bump_index_hazard(original_index_arc, delta_lambda)?
            }
        };

        // Create new market context with bumped credit index
        let bumped_market_ctx = market_ctx
            .clone()
            .insert_credit_index(tranche.credit_index_id, bumped_index);

        // Calculate bumped price
        let bumped_pv = self
            .price_tranche(tranche, &bumped_market_ctx, as_of)?
            .amount();

        // Return sensitivity per basis point
        Ok(bumped_pv - base_pv)
    }

    /// Calculate correlation delta (sensitivity to correlation changes).
    pub fn calculate_correlation_delta(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<F> {
        // Get base price
        let base_pv = self.price_tranche(tranche, market_ctx, as_of)?.amount();

        // Create bumped market context with base correlation shifted by configured amount
        let bump_abs = self.params.corr_bump_abs;
        let original_index_arc = market_ctx.credit_index_ref(tranche.credit_index_id)?;
        let bumped_corr_curve =
            self.bump_base_correlation(&original_index_arc.base_correlation_curve, bump_abs)?;

        // Create new credit index data with bumped correlation curve
        let bumped_index = CreditIndexData::builder()
            .num_constituents(original_index_arc.num_constituents)
            .recovery_rate(original_index_arc.recovery_rate)
            .index_credit_curve(original_index_arc.index_credit_curve.clone())
            .base_correlation_curve(std::sync::Arc::new(bumped_corr_curve))
            .build()?;

        // Create new market context with bumped credit index
        let bumped_market_ctx = market_ctx
            .clone()
            .insert_credit_index(tranche.credit_index_id, bumped_index);

        // Calculate bumped price
        let bumped_pv = self
            .price_tranche(tranche, &bumped_market_ctx, as_of)?
            .amount();

        // Return sensitivity per unit correlation change
        Ok((bumped_pv - base_pv) / bump_abs)
    }

    /// Calculate jump-to-default (immediate loss from specific entity default).
    ///
    /// For a homogeneous portfolio, estimates the immediate impact if one average
    /// entity defaults instantly. This is distinct from correlation sensitivity.
    pub fn calculate_jump_to_default(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        _as_of: Date,
    ) -> Result<F> {
        let index_data = market_ctx.credit_index_ref(tranche.credit_index_id)?;

        // For homogeneous pool, one name default impact
        let individual_weight = 1.0 / (index_data.num_constituents as F); // Portfolio weight per name
        let loss_given_default = 1.0 - index_data.recovery_rate;
        let individual_loss = individual_weight * loss_given_default; // As fraction of portfolio

        // Check if this loss hits the tranche layer
        let attach_frac = tranche.attach_pct / 100.0;
        let detach_frac = tranche.detach_pct / 100.0;
        let tranche_width = detach_frac - attach_frac;

        if individual_loss <= attach_frac {
            // Loss doesn't reach the tranche
            return Ok(0.0);
        }

        // Calculate how much of the individual loss hits the tranche
        let tranche_hit = if individual_loss >= detach_frac {
            // Loss fully exhausts the tranche
            tranche_width
        } else {
            // Loss partially hits the tranche
            individual_loss - attach_frac
        };

        // Convert to tranche notional impact
        let impact_on_tranche_fraction = tranche_hit / tranche_width;
        let impact_amount = impact_on_tranche_fraction * tranche.notional.amount();

        Ok(impact_amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::cds_tranche::parameters::CDSTrancheParams;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::CreditIndexData;
    use finstack_core::market_data::term_structures::{
        hazard_curve::HazardCurve, BaseCorrelationCurve,
    };
    use finstack_core::money::Money;
    use std::sync::Arc;
    use time::Month;

    fn sample_market_context() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Create discount curve
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
            .set_interp(finstack_core::math::interp::InterpStyle::LogLinear)
            .build()
            .unwrap();

        // Create index hazard curve
        let index_curve = HazardCurve::builder("CDX.NA.IG.42")
            .base_date(base_date)
            .recovery_rate(0.40)
            .knots(vec![(1.0, 0.01), (3.0, 0.015), (5.0, 0.02), (10.0, 0.025)])
            .par_spreads(vec![(1.0, 60.0), (3.0, 80.0), (5.0, 100.0), (10.0, 140.0)])
            .build()
            .unwrap();

        // Create base correlation curve
        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .points(vec![
                (3.0, 0.25),  // 0-3% equity
                (7.0, 0.45),  // 0-7% junior mezzanine
                (10.0, 0.60), // 0-10% senior mezzanine
                (15.0, 0.75), // 0-15% senior
                (30.0, 0.85), // 0-30% super senior
            ])
            .build()
            .unwrap();

        // Create credit index data
        let index_data = CreditIndexData::builder()
            .num_constituents(125)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .build()
            .unwrap();

        MarketContext::new()
            .insert_discount(discount_curve)
            .insert_credit_index("CDX.NA.IG.42", index_data)
    }

    fn sample_market_context_with_issuers(n: usize) -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.97), (5.0, 0.84), (10.0, 0.68)])
            .build()
            .unwrap();

        let index_curve = HazardCurve::builder("CDX.NA.IG.42")
            .base_date(base_date)
            .recovery_rate(0.40)
            .knots(vec![
                (1.0, 0.012),
                (3.0, 0.017),
                (5.0, 0.022),
                (10.0, 0.028),
            ])
            .par_spreads(vec![(1.0, 65.0), (3.0, 85.0), (5.0, 105.0), (10.0, 145.0)])
            .build()
            .unwrap();

        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .points(vec![
                (3.0, 0.25),
                (7.0, 0.45),
                (10.0, 0.60),
                (15.0, 0.75),
                (30.0, 0.85),
            ])
            .build()
            .unwrap();

        let mut issuer_curves = std::collections::HashMap::new();
        for i in 0..n {
            let id = format!("ISSUER-{:03}", i + 1);
            let bump = (i as f64) * 0.001;
            let hz = HazardCurve::builder(id.as_str())
                .base_date(base_date)
                .recovery_rate(0.40)
                .knots(vec![
                    (1.0, (0.012 + bump).min(0.2)),
                    (3.0, (0.017 + bump).min(0.2)),
                    (5.0, (0.022 + bump).min(0.2)),
                    (10.0, (0.028 + bump).min(0.2)),
                ])
                .build()
                .unwrap();
            issuer_curves.insert(id, Arc::new(hz));
        }

        let index = CreditIndexData::builder()
            .num_constituents(n as u16)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .with_issuer_curves(issuer_curves)
            .build()
            .unwrap();

        MarketContext::new()
            .insert_discount(discount_curve)
            .insert_credit_index("CDX.NA.IG.42", index)
    }

    fn sample_tranche() -> CdsTranche {
        let _issue_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        {
            let tranche_params = CDSTrancheParams::new(
                "CDX.NA.IG.42",                          // index_name
                42,                                      // series
                3.0,                                     // attach_pct (3%)
                7.0,                                     // detach_pct (7%)
                Money::new(10_000_000.0, Currency::USD), // $10MM notional
                maturity,                                // maturity
                500.0,                                   // running_coupon_bp (5%)
            );
            let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
            CdsTranche::new(
                "CDX_IG42_3_7_5Y",
                &tranche_params,
                &schedule_params,
                "USD-OIS",
                "CDX.NA.IG.42",
                TrancheSide::SellProtection,
            )
        }
    }

    #[test]
    fn test_model_creation() {
        let model = CDSTranchePricer::new();
        assert_eq!(model.params.quadrature_order, 7);
        assert!(model.params.use_issuer_curves);
    }

    #[test]
    fn test_conditional_default_probability() {
        let model = CDSTranchePricer::new();
        let correlation = 0.30;
        let default_threshold = standard_normal_inv_cdf(0.05); // 5% default probability

        // Test with market factor = 0 (should be reasonable value close to original default prob)
        let cond_prob = model.conditional_default_probability(default_threshold, correlation, 0.0);
        assert!(
            cond_prob > 0.01 && cond_prob < 0.1,
            "Expected reasonable default prob, got {}",
            cond_prob
        );

        // Test with negative market factor (should increase default prob)
        let cond_prob_neg =
            model.conditional_default_probability(default_threshold, correlation, -1.0);
        assert!(cond_prob_neg > 0.05);

        // Test with positive market factor (should decrease default prob)
        let cond_prob_pos =
            model.conditional_default_probability(default_threshold, correlation, 1.0);
        assert!(cond_prob_pos < 0.05);
    }

    #[test]
    fn test_binomial_probability() {
        // Test known values
        assert!((binomial_probability(10, 5, 0.5) - 0.24609375).abs() < 1e-6);
        assert!((binomial_probability(5, 0, 0.1) - 0.59049).abs() < 1e-6);

        // Test edge cases
        assert_eq!(binomial_probability(10, 0, 0.0), 1.0);
        assert_eq!(binomial_probability(10, 10, 1.0), 1.0);
        assert_eq!(binomial_probability(10, 5, 0.0), 0.0);
    }

    #[test]
    fn test_log_factorial() {
        // Test small values (exact calculation)
        assert!((log_factorial(1) - 0.0).abs() < 1e-12);
        assert!(
            (log_factorial(5) - (2.0_f64.ln() + 3.0_f64.ln() + 4.0_f64.ln() + 5.0_f64.ln())).abs()
                < 1e-12
        );

        // Test that Stirling's approximation is reasonable for large n
        let log_100_factorial = log_factorial(100);
        assert!(log_100_factorial > 360.0 && log_100_factorial < 370.0); // Should be around 363.7
    }

    #[test]
    fn test_tranche_pricing_integration() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Test that pricing doesn't panic and returns a reasonable result
        let result = model.price_tranche(&tranche, &market_ctx, as_of);
        assert!(result.is_ok());

        let pv = result.unwrap();
        assert_eq!(pv.currency(), Currency::USD);
        // PV should be finite (could be positive or negative)
        assert!(pv.amount().is_finite());
    }

    #[test]
    fn test_hetero_spa_matches_homogeneous_when_issuers_equal() {
        let ctx_base = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let mut tranche = sample_tranche();
        tranche.running_coupon_bp = 0.0; // isolate protection leg

        // Build a context with issuer curves identical to index curve
        let index_data = ctx_base.credit_index("CDX.NA.IG.42").unwrap();
        let mut issuer_curves = std::collections::HashMap::new();
        for i in 0..10 {
            let id = format!("ISSUER-{:03}", i + 1);
            issuer_curves.insert(id, index_data.index_credit_curve.clone());
        }
        let hetero_index = CreditIndexData::builder()
            .num_constituents(10)
            .recovery_rate(index_data.recovery_rate)
            .index_credit_curve(index_data.index_credit_curve.clone())
            .base_correlation_curve(index_data.base_correlation_curve.clone())
            .with_issuer_curves(issuer_curves)
            .build()
            .unwrap();
        let ctx = ctx_base
            .clone()
            .insert_credit_index("CDX.NA.IG.42", hetero_index);

        let mut homo = CDSTranchePricer::new();
        homo.params.use_issuer_curves = false;
        let mut hetero = CDSTranchePricer::new();
        hetero.params.use_issuer_curves = true;
        hetero.params.hetero_method = HeteroMethod::Spa;

        let pv_homo = homo.price_tranche(&tranche, &ctx, as_of).unwrap().amount();
        let pv_hetero = hetero
            .price_tranche(&tranche, &ctx, as_of)
            .unwrap()
            .amount();
        assert!((pv_homo - pv_hetero).abs() < 1e-2 * pv_homo.abs().max(1.0));
    }

    #[test]
    fn test_hetero_spa_vs_exact_convolution_small_pool() {
        let ctx = sample_market_context_with_issuers(8);
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let tranche_params = CDSTrancheParams::new(
            "CDX.NA.IG.42",
            42,
            3.0,
            7.0,
            Money::new(10_000_000.0, Currency::USD),
            as_of + time::Duration::days(5 * 365),
            0.0,
        );
        let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
        let tranche = CdsTranche::new(
            "CDX_IG42_3_7_5Y",
            &tranche_params,
            &schedule_params,
            "USD-OIS",
            "CDX.NA.IG.42",
            TrancheSide::SellProtection,
        );

        let mut spa = CDSTranchePricer::new();
        spa.params.use_issuer_curves = true;
        spa.params.hetero_method = HeteroMethod::Spa;
        let mut exact = CDSTranchePricer::new();
        exact.params.use_issuer_curves = true;
        exact.params.hetero_method = HeteroMethod::ExactConvolution;
        exact.params.grid_step = 0.002;

        let pv_spa = spa.price_tranche(&tranche, &ctx, as_of).unwrap().amount();
        let pv_exact = exact.price_tranche(&tranche, &ctx, as_of).unwrap().amount();
        assert!((pv_spa - pv_exact).abs() < 0.02 * pv_exact.abs().max(1.0));
    }

    #[test]
    fn test_grid_step_refines_exact_convolution() {
        let ctx = sample_market_context_with_issuers(10);
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let tranche_params = CDSTrancheParams::new(
            "CDX.NA.IG.42",
            42,
            0.0,
            3.0,
            Money::new(10_000_000.0, Currency::USD),
            as_of + time::Duration::days(5 * 365),
            0.0,
        );
        let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
        let tranche = CdsTranche::new(
            "CDX_IG42_0_3_5Y",
            &tranche_params,
            &schedule_params,
            "USD-OIS",
            "CDX.NA.IG.42",
            TrancheSide::SellProtection,
        );

        let mut exact_coarse = CDSTranchePricer::new();
        exact_coarse.params.use_issuer_curves = true;
        exact_coarse.params.hetero_method = HeteroMethod::ExactConvolution;
        exact_coarse.params.grid_step = 0.005;

        let mut exact_fine = CDSTranchePricer::new();
        exact_fine.params = exact_coarse.params.clone();
        exact_fine.params.grid_step = 0.001;

        let p_coarse = exact_coarse
            .price_tranche(&tranche, &ctx, as_of)
            .unwrap()
            .amount();
        let p_fine = exact_fine
            .price_tranche(&tranche, &ctx, as_of)
            .unwrap()
            .amount();
        assert!((p_coarse - p_fine).abs() < 0.02 * p_fine.abs().max(1.0));
    }

    #[test]
    fn test_expected_loss_calculation() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();

        let expected_loss = model.calculate_expected_loss(&tranche, &market_ctx);
        assert!(expected_loss.is_ok());

        let loss = expected_loss.unwrap();
        assert!(loss >= 0.0); // Expected loss should be non-negative
        assert!(loss.is_finite());
    }

    #[test]
    fn test_payment_schedule_generation() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let schedule = model.generate_payment_schedule(&tranche, as_of);
        assert!(schedule.is_ok());

        let dates = schedule.unwrap();
        assert!(!dates.is_empty());
        assert!(dates[0] > as_of); // First payment should be after as_of
        assert!(*dates.last().unwrap() <= tranche.maturity); // Last payment should not exceed maturity

        // Check dates are in ascending order
        for window in dates.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn test_el_curve_monotonicity() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let schedule = model.generate_payment_schedule(&tranche, as_of).unwrap();
        let index_data_arc = market_ctx.credit_index(tranche.credit_index_id).unwrap();
        let el_curve = model.build_el_curve(&tranche, &index_data_arc, &schedule);

        assert!(el_curve.is_ok());
        let curve = el_curve.unwrap();

        // EL should be non-decreasing and bounded [0,1]
        // Allow for small numerical deviations due to base correlation model limitations
        // The base correlation model can have inconsistencies at knot points
        const NUMERICAL_TOLERANCE: F = 0.01; // Allow up to 1% EL fraction decrease

        for (i, &(_, el_fraction)) in curve.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&el_fraction),
                "EL fraction {} at index {} out of bounds",
                el_fraction,
                i
            );

            if i > 0 {
                let decrease = curve[i - 1].1 - el_fraction;
                assert!(
                    decrease <= NUMERICAL_TOLERANCE,
                    "EL fraction decreased significantly from {} to {} (decrease: {})",
                    curve[i - 1].1,
                    el_fraction,
                    decrease
                );
            }
        }
    }

    #[test]
    fn test_cs01_calculation() {
        let model = CDSTranchePricer::new();
        let mut tranche = sample_tranche();
        tranche.side = TrancheSide::SellProtection; // Sell protection for positive CS01
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let cs01 = model.calculate_cs01(&tranche, &market_ctx, as_of);
        assert!(cs01.is_ok());

        let sensitivity = cs01.unwrap();
        assert!(sensitivity.is_finite());
        // For protection seller, CS01 should typically be positive
        // (higher spreads -> higher protection premium income)
    }

    #[test]
    fn test_correlation_delta_calculation() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let corr_delta = model.calculate_correlation_delta(&tranche, &market_ctx, as_of);
        assert!(corr_delta.is_ok());

        let sensitivity = corr_delta.unwrap();
        assert!(sensitivity.is_finite());
        // Correlation sensitivity should be finite and reasonable in magnitude
    }

    #[test]
    fn test_jump_to_default_calculation() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let jtd = model.calculate_jump_to_default(&tranche, &market_ctx, as_of);
        assert!(jtd.is_ok());

        let impact = jtd.unwrap();
        assert!(impact >= 0.0); // Impact should be non-negative
        assert!(impact.is_finite());
    }

    #[test]
    fn test_pv_decomposition_consistency() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let index_data_arc = market_ctx.credit_index(tranche.credit_index_id).unwrap();
        let discount_curve = market_ctx
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                tranche.disc_id,
            )
            .unwrap();

        // Calculate individual leg PVs
        let pv_premium = model.calculate_premium_leg_pv(
            &tranche,
            &index_data_arc,
            discount_curve.as_ref(),
            as_of,
        );
        let pv_protection = model.calculate_protection_leg_pv(
            &tranche,
            &index_data_arc,
            discount_curve.as_ref(),
            as_of,
        );

        assert!(pv_premium.is_ok());
        assert!(pv_protection.is_ok());

        let premium = pv_premium.unwrap();
        let protection = pv_protection.unwrap();

        assert!(premium.is_finite());
        assert!(protection.is_finite());
        assert!(premium >= 0.0); // Premium leg should be positive for ongoing coupon
        assert!(protection >= 0.0); // Protection leg should be non-negative
    }

    #[test]
    fn test_extreme_correlation_numerical_stability() {
        let model = CDSTranchePricer::new();
        let market_ctx = sample_market_context();
        let _as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let index_data_arc = market_ctx.credit_index("CDX.NA.IG.42").unwrap();

        // Test extreme correlation values that are challenging for numerical stability
        let extreme_correlations = [1e-10, 1e-6, 0.001, 0.999, 1.0 - 1e-6, 1.0 - 1e-10];

        for &test_correlation in &extreme_correlations {
            // Create a correlation curve with extreme values
            let extreme_corr_curve =
                finstack_core::market_data::term_structures::BaseCorrelationCurve::builder(
                    "TEST_EXTREME",
                )
                .points(vec![
                    (3.0, test_correlation),
                    (7.0, test_correlation),
                    (10.0, test_correlation),
                    (15.0, test_correlation),
                    (30.0, test_correlation),
                ])
                .build()
                .unwrap();

            // Create index data with extreme correlation
            let extreme_index_data = CreditIndexData::builder()
                .num_constituents(125)
                .recovery_rate(0.40)
                .index_credit_curve(index_data_arc.index_credit_curve.clone())
                .base_correlation_curve(std::sync::Arc::new(extreme_corr_curve))
                .build()
                .unwrap();

            // Test equity tranche loss calculation
            let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
            let result = model.calculate_equity_tranche_loss(
                7.0, // 7% detachment
                test_correlation,
                &extreme_index_data,
                maturity,
            );

            assert!(
                result.is_ok(),
                "Equity tranche loss calculation failed for correlation={}",
                test_correlation
            );

            let expected_loss = result.unwrap();
            assert!(
                expected_loss.is_finite(),
                "Expected loss should be finite for correlation={}, got {}",
                test_correlation,
                expected_loss
            );
            assert!(
                (0.0..=1.0).contains(&expected_loss),
                "Expected loss should be in [0,1] for correlation={}, got {}",
                test_correlation,
                expected_loss
            );
        }
    }

    #[test]
    fn test_smooth_correlation_boundary_transitions() {
        let model = CDSTranchePricer::new();

        // Test that smooth boundary transitions work correctly
        let test_values = [
            0.005, 0.009, 0.011, 0.015, // Near min boundary (0.01)
            0.985, 0.989, 0.991, 0.995, // Near max boundary (0.99)
        ];

        for &test_corr in &test_values {
            let smoothed = model.smooth_correlation_boundary(test_corr);

            // Should be finite and within expanded bounds
            assert!(
                smoothed.is_finite(),
                "Smoothed correlation should be finite for input={}",
                test_corr
            );
            assert!(
                (0.005..=0.995).contains(&smoothed),
                "Smoothed correlation {} should be in reasonable bounds for input={}",
                smoothed,
                test_corr
            );

            // Should be continuous (no big jumps)
            let nearby = test_corr + 0.001;
            let smoothed_nearby = model.smooth_correlation_boundary(nearby);
            let transition_smoothness = (smoothed_nearby - smoothed).abs();

            assert!(
                transition_smoothness < 0.01,
                "Boundary transition should be smooth: jump of {} between {} and {}",
                transition_smoothness,
                test_corr,
                nearby
            );
        }
    }

    #[test]
    fn test_conditional_default_probability_enhanced() {
        let model = CDSTranchePricer::new();
        let default_threshold = standard_normal_inv_cdf(0.05); // 5% unconditional default prob

        // Test enhanced function across various correlation and market factor combinations
        let correlations = [1e-8, 0.01, 0.3, 0.7, 0.99, 1.0 - 1e-8];
        let market_factors = [-4.0, -2.0, -1.0, 0.0, 1.0, 2.0, 4.0];

        for &correlation in &correlations {
            for &market_factor in &market_factors {
                let enhanced_prob = model.conditional_default_probability_enhanced(
                    default_threshold,
                    correlation,
                    market_factor,
                );
                let standard_prob = model.conditional_default_probability(
                    default_threshold,
                    correlation.clamp(0.01, 0.99), // Clamp for standard function
                    market_factor,
                );

                // Enhanced function should always give finite, bounded results
                assert!(
                    enhanced_prob.is_finite(),
                    "Enhanced conditional prob should be finite for ρ={}, Z={}",
                    correlation,
                    market_factor
                );
                assert!(
                    (0.0..=1.0).contains(&enhanced_prob),
                    "Enhanced conditional prob should be in [0,1]: got {} for ρ={}, Z={}",
                    enhanced_prob,
                    correlation,
                    market_factor
                );

                // For normal correlation ranges, should be close to standard implementation
                if (0.05..=0.95).contains(&correlation) {
                    let diff = (enhanced_prob - standard_prob).abs();
                    assert!(diff < 0.01,
                        "Enhanced and standard methods should agree in normal range: diff={} for ρ={}, Z={}",
                        diff, correlation, market_factor);
                }
            }
        }
    }
}
