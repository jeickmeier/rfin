//! Gaussian Copula pricing model for CDS tranches.
//!
//! Implements the industry-standard base correlation approach for pricing
//! synthetic CDO tranches using a one-factor Gaussian Copula model.

use crate::instruments::fixed_income::cds_tranche::numerical::{
    standard_normal_cdf, standard_normal_inv_cdf, GaussHermiteQuadrature,
};
use crate::instruments::fixed_income::cds_tranche::{CdsTranche, TrancheSide};
use crate::market_data::{CreditIndexData, ValuationMarketContext};
use finstack_core::dates::Date;
use finstack_core::market_data::traits::Discount;
use finstack_core::prelude::*;
use finstack_core::F;

/// Parameters for the Gaussian Copula pricing model.
#[derive(Clone, Debug)]
pub struct GaussianCopulaParams {
    /// Number of quadrature points for numerical integration (5, 7, or 10)
    pub quadrature_order: u8,
    /// Whether to use issuer-specific curves if available
    pub use_issuer_curves: bool,
    /// Minimum correlation value for numerical stability
    pub min_correlation: F,
    /// Maximum correlation value for numerical stability  
    pub max_correlation: F,
}

impl Default for GaussianCopulaParams {
    fn default() -> Self {
        Self {
            quadrature_order: 7,     // Good balance of accuracy and performance
            use_issuer_curves: true, // Use heterogeneous modeling when available
            min_correlation: 0.01,   // Numerical stability floor
            max_correlation: 0.99,   // Numerical stability ceiling
        }
    }
}

/// Gaussian Copula pricing engine for CDS tranches.
pub struct GaussianCopulaModel {
    params: GaussianCopulaParams,
}

impl Default for GaussianCopulaModel {
    fn default() -> Self {
        Self::new()
    }
}

impl GaussianCopulaModel {
    /// Create a new Gaussian Copula model with default parameters.
    pub fn new() -> Self {
        Self {
            params: GaussianCopulaParams::default(),
        }
    }

    /// Create a new model with custom parameters.
    pub fn with_params(params: GaussianCopulaParams) -> Self {
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
        market_ctx: &ValuationMarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Get the credit index data
        let index_data = market_ctx.get_credit_index(tranche.credit_index_id)?;

        // Get the discount curve
        let discount_curve = market_ctx.discount(tranche.disc_id)?;

        // Calculate expected tranche loss using base correlation approach
        let expected_loss =
            self.calculate_expected_tranche_loss(tranche, index_data, tranche.maturity)?;

        // Calculate present values of premium and protection legs
        let pv_premium = self.calculate_premium_leg_pv(
            tranche,
            index_data,
            discount_curve.as_ref(),
            as_of,
            expected_loss,
        )?;

        let pv_protection = self.calculate_protection_leg_pv(
            tranche,
            index_data,
            discount_curve.as_ref(),
            as_of,
            expected_loss,
        )?;

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
    /// the base correlation curve.
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

        // Clamp correlations for numerical stability
        let corr_attach =
            corr_attach.clamp(self.params.min_correlation, self.params.max_correlation);
        let corr_detach =
            corr_detach.clamp(self.params.min_correlation, self.params.max_correlation);

        // Calculate expected losses for equity tranches [0, A] and [0, D]
        let el_to_attach =
            self.calculate_equity_tranche_loss(attach_pct, corr_attach, index_data, maturity)?;

        let el_to_detach =
            self.calculate_equity_tranche_loss(detach_pct, corr_detach, index_data, maturity)?;

        // The [A, D] tranche loss is the difference
        let tranche_loss = el_to_detach - el_to_attach;

        // Convert from percentage to currency amount
        let total_portfolio_notional = index_data.num_constituents as F * tranche.notional.amount();
        Ok(tranche_loss * total_portfolio_notional / 100.0)
    }

    /// Calculate expected loss for an equity tranche [0, K] using Gaussian Copula.
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
        let num_constituents = index_data.num_constituents as usize;
        let recovery_rate = index_data.recovery_rate;

        // Convert detachment from percent to number of constituents
        let detachment_notional = detachment_pct / 100.0;

        // Get the appropriate quadrature
        let quad = match self.params.quadrature_order {
            5 => GaussHermiteQuadrature::order_5(),
            7 => GaussHermiteQuadrature::order_7(),
            10 => GaussHermiteQuadrature::order_10(),
            _ => GaussHermiteQuadrature::order_7(), // Default fallback
        };

        // Calculate maturity in years for survival probability lookup
        let maturity_years = self.years_from_base(index_data, maturity);

        // Get default probability for the index (homogeneous assumption)
        let default_prob = self.get_default_probability(index_data, maturity_years)?;
        let default_threshold = standard_normal_inv_cdf(default_prob);

        // Integrate expected loss over all states of the market factor Z
        let expected_loss = quad.integrate(|z| {
            // Conditional default probability given market factor Z
            let conditional_default_prob =
                self.conditional_default_probability(default_threshold, correlation, z);

            // Expected loss of equity tranche conditional on Z
            self.conditional_equity_tranche_loss(
                num_constituents,
                detachment_notional,
                conditional_default_prob,
                recovery_rate,
            )
        });

        Ok(expected_loss)
    }

    /// Calculate conditional default probability given market factor Z.
    ///
    /// P(default | Z) = Φ((Φ⁻¹(PD) - √ρ * Z) / √(1-ρ))
    fn conditional_default_probability(
        &self,
        default_threshold: F,
        correlation: F,
        market_factor: F,
    ) -> F {
        let sqrt_rho = correlation.sqrt();
        let sqrt_one_minus_rho = (1.0 - correlation).sqrt();

        let conditional_threshold =
            (default_threshold - sqrt_rho * market_factor) / sqrt_one_minus_rho;
        standard_normal_cdf(conditional_threshold)
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

    /// Calculate present value of the premium leg.
    ///
    /// PV = Coupon * Σ(Δt_j * D(t_j) * E[TrancheNotional(t_j)])
    fn calculate_premium_leg_pv(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        discount_curve: &dyn Discount,
        as_of: Date,
        expected_loss_at_maturity: F,
    ) -> Result<F> {
        let coupon = tranche.running_coupon_bp / 10000.0; // Convert bp to decimal
        let initial_notional = tranche.notional.amount();

        // For simplicity, assume linear amortization of expected loss over time
        // In practice, this would use payment schedule dates
        let maturity_years = self.years_from_base(index_data, tranche.maturity);
        let payment_dates = self.generate_payment_schedule(tranche, as_of)?;

        let mut pv_premium = 0.0;

        for payment_date in payment_dates {
            let t = self.years_from_base(index_data, payment_date);
            if t <= 0.0 {
                continue;
            }

            // Approximate expected loss at this payment date (linear interpolation)
            let expected_loss_at_t = if t >= maturity_years {
                expected_loss_at_maturity
            } else {
                expected_loss_at_maturity * (t / maturity_years)
            };

            // Outstanding notional at this date
            let outstanding_notional = initial_notional - expected_loss_at_t;

            if outstanding_notional <= 0.0 {
                break; // Tranche fully written down
            }

            // Accrual period calculation
            let accrual_period = match tranche.payment_frequency {
                finstack_core::dates::Frequency::Months(m) => (m as F) / 12.0,
                finstack_core::dates::Frequency::Days(d) => (d as F) / 365.25,
                _ => 0.25, // Default to quarterly
            };
            let discount_factor = discount_curve.df(t);

            pv_premium += coupon * accrual_period * discount_factor * outstanding_notional;
        }

        Ok(pv_premium)
    }

    /// Calculate present value of the protection leg.
    ///
    /// PV = Σ(D(t_j) * (E[TrancheLoss(t_j)] - E[TrancheLoss(t_{j-1})]))
    fn calculate_protection_leg_pv(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        discount_curve: &dyn Discount,
        as_of: Date,
        expected_loss_at_maturity: F,
    ) -> Result<F> {
        let payment_dates = self.generate_payment_schedule(tranche, as_of)?;
        let maturity_years = self.years_from_base(index_data, tranche.maturity);

        let mut pv_protection = 0.0;
        let mut prev_expected_loss = 0.0;

        for payment_date in payment_dates {
            let t = self.years_from_base(index_data, payment_date);
            if t <= 0.0 {
                continue;
            }

            // Expected loss at this payment date (linear approximation)
            let expected_loss_at_t = if t >= maturity_years {
                expected_loss_at_maturity
            } else {
                expected_loss_at_maturity * (t / maturity_years)
            };

            // Incremental loss since last payment
            let incremental_loss = expected_loss_at_t - prev_expected_loss;

            if incremental_loss > 0.0 {
                let discount_factor = discount_curve.df(t);
                pv_protection += incremental_loss * discount_factor;
            }

            prev_expected_loss = expected_loss_at_t;
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
        dc.year_fraction(index_data.index_credit_curve.base_date(), date)
            .unwrap_or(0.0)
    }

    /// Generate payment schedule for the tranche.
    ///
    /// For now, this creates a simple quarterly schedule. In practice,
    /// this would use the scheduling logic from the cashflow builder.
    fn generate_payment_schedule(&self, tranche: &CdsTranche, as_of: Date) -> Result<Vec<Date>> {
        let mut dates = Vec::new();
        let freq_months = match tranche.payment_frequency {
            finstack_core::dates::Frequency::Months(m) => m as i32,
            finstack_core::dates::Frequency::Days(_) => 3, // Default to quarterly for days-based frequencies
            _ => 3,                                        // Default to quarterly
        };

        let mut current_date = as_of;
        while current_date < tranche.maturity {
            current_date = add_months(current_date, freq_months);
            if current_date <= tranche.maturity {
                dates.push(current_date);
            }
        }

        // Ensure maturity is included
        if dates.is_empty() || *dates.last().unwrap() != tranche.maturity {
            dates.push(tranche.maturity);
        }

        Ok(dates)
    }

    /// Calculate upfront amount for the tranche.
    ///
    /// This is the net present value at inception, representing the
    /// payment required to enter the position at the standard coupon.
    pub fn calculate_upfront(
        &self,
        tranche: &CdsTranche,
        market_ctx: &ValuationMarketContext,
        as_of: Date,
    ) -> Result<F> {
        let pv = self.price_tranche(tranche, market_ctx, as_of)?;
        Ok(pv.amount())
    }

    /// Calculate Spread DV01 (sensitivity to 1bp change in running coupon).
    pub fn calculate_spread_dv01(
        &self,
        tranche: &CdsTranche,
        market_ctx: &ValuationMarketContext,
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
        market_ctx: &ValuationMarketContext,
    ) -> Result<F> {
        let index_data = market_ctx.get_credit_index(tranche.credit_index_id)?;
        self.calculate_expected_tranche_loss(tranche, index_data, tranche.maturity)
    }

    /// Calculate CS01 (sensitivity to 1bp parallel shift in credit spreads).
    pub fn calculate_cs01(
        &self,
        _tranche: &CdsTranche,
        _market_ctx: &ValuationMarketContext,
        _as_of: Date,
    ) -> Result<F> {
        // This would require bumping the underlying credit curves by 1bp
        // For now, return a placeholder
        // TODO: Implement proper credit spread sensitivity calculation
        Ok(0.0)
    }

    /// Calculate correlation delta (sensitivity to correlation changes).
    pub fn calculate_correlation_delta(
        &self,
        _tranche: &CdsTranche,
        _market_ctx: &ValuationMarketContext,
        _as_of: Date,
    ) -> Result<F> {
        // This would require bumping the base correlation curve
        // For now, return a placeholder
        // TODO: Implement proper correlation sensitivity calculation
        Ok(0.0)
    }
}

/// Calculate binomial probability: P(X = k) where X ~ Binomial(n, p)
fn binomial_probability(n: usize, k: usize, p: F) -> F {
    if k > n {
        return 0.0;
    }
    if p <= 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    if p >= 1.0 {
        return if k == n { 1.0 } else { 0.0 };
    }

    // Use log-space calculation to avoid overflow for large n
    let log_prob =
        log_binomial_coefficient(n, k) + (k as F) * p.ln() + ((n - k) as F) * (1.0 - p).ln();
    log_prob.exp()
}

/// Calculate log of binomial coefficient: ln(n choose k)
fn log_binomial_coefficient(n: usize, k: usize) -> F {
    if k > n {
        return F::NEG_INFINITY;
    }
    if k == 0 || k == n {
        return 0.0;
    }

    // Use the more efficient calculation: ln(n!) - ln(k!) - ln((n-k)!)
    // Using Stirling's approximation for large values
    log_factorial(n) - log_factorial(k) - log_factorial(n - k)
}

/// Calculate log factorial using Stirling's approximation for large n.
fn log_factorial(n: usize) -> F {
    if n == 0 || n == 1 {
        return 0.0;
    }
    if n < 20 {
        // Exact calculation for small n: ln(n!) = ln(1) + ln(2) + ... + ln(n)
        (2..=n).map(|i| (i as F).ln()).sum()
    } else {
        // Stirling's approximation: ln(n!) ≈ n*ln(n) - n + 0.5*ln(2πn)
        let n_f = n as F;
        n_f * n_f.ln() - n_f + 0.5 * (2.0 * std::f64::consts::PI * n_f).ln()
    }
}

/// Add months to a date (simplified implementation).
/// In practice, this would use proper calendar arithmetic.
fn add_months(date: Date, months: i32) -> Date {
    // Simple approximation: add 30 days per month
    // TODO: Replace with proper calendar arithmetic
    date + time::Duration::days(30 * months as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::credit_index::CreditIndexData;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::{
        hazard_curve::HazardCurve, BaseCorrelationCurve,
    };
    use finstack_core::money::Money;
    use std::sync::Arc;
    use time::Month;

    fn sample_market_context() -> ValuationMarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Create discount curve
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
            .log_df()
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

        ValuationMarketContext::new()
            .with_discount(discount_curve)
            .with_credit_index("CDX.NA.IG.42", index_data)
    }

    fn sample_tranche() -> CdsTranche {
        let _issue_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        CdsTranche::new(
            "CDX_IG42_3_7_5Y",                                      // id
            "CDX.NA.IG.42",                                         // index_name
            42,                                                     // series
            3.0,                                                    // attach_pct (3%)
            7.0,                                                    // detach_pct (7%)
            Money::new(10_000_000.0, Currency::USD),                // $10MM notional
            maturity,                                               // maturity
            500.0,                                                  // running_coupon_bp (5%)
            finstack_core::dates::Frequency::quarterly(),           // payment_frequency
            finstack_core::dates::DayCount::Act360,                 // day_count
            finstack_core::dates::BusinessDayConvention::Following, // business_day_convention
            None,                                                   // calendar_id
            "USD-OIS",                                              // disc_id
            "CDX.NA.IG.42",                                         // credit_index_id
            TrancheSide::SellProtection,                            // side
        )
    }

    #[test]
    fn test_model_creation() {
        let model = GaussianCopulaModel::new();
        assert_eq!(model.params.quadrature_order, 7);
        assert!(model.params.use_issuer_curves);
    }

    #[test]
    fn test_conditional_default_probability() {
        let model = GaussianCopulaModel::new();
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
        let model = GaussianCopulaModel::new();
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
    fn test_expected_loss_calculation() {
        let model = GaussianCopulaModel::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();

        let expected_loss = model.calculate_expected_loss(&tranche, &market_ctx);
        assert!(expected_loss.is_ok());

        let loss = expected_loss.unwrap();
        assert!(loss >= 0.0); // Expected loss should be non-negative
        assert!(loss.is_finite());
    }
}
