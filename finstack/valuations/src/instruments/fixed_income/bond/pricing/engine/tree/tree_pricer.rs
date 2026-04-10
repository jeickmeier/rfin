use super::super::super::super::types::Bond;
use super::bond_valuator::BondValuator;
use super::config::{TreeModelChoice, TreePricerConfig};
use crate::instruments::common_impl::models::trees::hull_white_tree::{
    HullWhiteTree, HullWhiteTreeConfig,
};
use crate::instruments::common_impl::models::trees::two_factor_rates_credit::{
    RatesCreditConfig, RatesCreditTree,
};
use crate::instruments::common_impl::models::{
    short_rate_keys, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;

/// Tree-based pricer for bonds with embedded options and OAS calculations.
///
/// Provides methods for calculating option-adjusted spread (OAS) for bonds with
/// embedded call/put options. Automatically selects between short-rate and
/// rates+credit tree models based on available market data.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::instruments::fixed_income::bond::pricing::engine::tree::TreePricer;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example().unwrap();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// let pricer = TreePricer::new();
/// // OAS in basis points
/// let oas_bp = pricer.calculate_oas(&bond, &market, as_of, 98.5)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct TreePricer {
    /// Pricer configuration (tree steps, volatility, convergence settings)
    config: TreePricerConfig,
}

impl TreePricer {
    /// Create a new tree pricer with default configuration.
    ///
    /// # Returns
    ///
    /// A `TreePricer` with default configuration (100 steps, 1% volatility).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::engine::tree::TreePricer;
    ///
    /// let pricer = TreePricer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            config: TreePricerConfig::default(),
        }
    }

    /// Create a tree pricer with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Custom tree pricer configuration
    ///
    /// # Returns
    ///
    /// A `TreePricer` with the specified configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::engine::tree::{TreePricer, TreePricerConfig};
    ///
    /// let config = TreePricerConfig::high_precision(0.015);
    /// let pricer = TreePricer::with_config(config);
    /// ```
    pub fn with_config(config: TreePricerConfig) -> Self {
        Self { config }
    }

    /// Calculate option-adjusted spread (OAS) for a bond.
    ///
    /// Solves for the constant spread that equates the tree price to the market price.
    /// Uses Brent's method for root finding, automatically selecting between short-rate
    /// and rates+credit tree models based on available market data.
    ///
    /// # OAS Convention
    ///
    /// Under either model the OAS is a **parallel shift to the calibrated risk-free
    /// short rate lattice** (in basis points). When the rates+credit two-factor tree
    /// is used, the hazard tree captures the credit spread independently, so the OAS
    /// represents the option-adjusted spread **over the risk-free curve** — consistent
    /// with the Bloomberg OAS convention for risky bonds.
    ///
    /// # Arguments
    ///
    /// * `bond` - The bond to calculate OAS for (must have call/put options)
    /// * `market_context` - Market context with discount and optionally hazard curves
    /// * `as_of` - Valuation date
    /// * `clean_price_pct_of_par` - Market clean price as percentage of par (e.g., 98.5)
    ///
    /// # Returns
    ///
    /// OAS in basis points (e.g., 150.0 means 150 basis points).
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - Discount curve is not found
    /// - Tree calibration fails
    /// - Root finding fails to converge
    /// - Bond is already matured
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::bond::Bond;
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::engine::tree::TreePricer;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example().unwrap();
    /// # let market = MarketContext::new();
    /// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
    /// let pricer = TreePricer::new();
    /// let oas_bp = pricer.calculate_oas(&bond, &market, as_of, 98.5)?;
    /// // oas_bp is in basis points
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn calculate_oas(
        &self,
        bond: &Bond,
        market_context: &MarketContext,
        as_of: Date,
        clean_price_pct_of_par: f64,
    ) -> Result<f64> {
        use crate::instruments::fixed_income::bond::pricing::settlement::QuoteDateContext;

        // Dirty target must use accrued at the quote/settlement date to match
        // the market convention used by YTM, Z-spread, and the quote engine.
        let quote_ctx = QuoteDateContext::new(bond, market_context, as_of)?;
        let dirty_target =
            quote_ctx.dirty_from_clean_pct(clean_price_pct_of_par, bond.notional.amount());
        // Choose model: if a hazard curve is present in MarketContext whose ID matches the bond's
        // discount ID (preferred) or the fallback pattern "{discount_curve_id}-CREDIT", use the rates+credit
        // two-factor tree; otherwise, fall back to short-rate.
        let mut use_rates_credit = false;
        let mut rc_tree: Option<RatesCreditTree> = None;
        let discount_curve = market_context.get_discount(&bond.discount_curve_id)?;
        // Align tree time basis with the discount curve's own day-count.
        if as_of >= bond.maturity {
            return Ok(0.0);
        }
        let dc_curve = discount_curve.day_count();
        let time_to_maturity = dc_curve.year_fraction(
            as_of,
            bond.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if time_to_maturity <= 0.0 {
            return Ok(0.0);
        }
        let hazard_curve = if let Some(hid) = bond.credit_curve_id.as_ref() {
            market_context.get_hazard(hid.as_str()).ok()
        } else {
            market_context
                .get_hazard(bond.discount_curve_id.as_str())
                .ok()
                .or_else(|| {
                    market_context
                        .get_hazard(format!("{}-CREDIT", bond.discount_curve_id.as_str()))
                        .ok()
                })
        };
        if let Some(hc) = hazard_curve.as_ref() {
            let cfg = RatesCreditConfig {
                steps: self.config.tree_steps,
                rate_vol: self.config.volatility,
                ..Default::default()
            };
            let mut tree = RatesCreditTree::new(cfg);
            tree.calibrate(discount_curve.as_ref(), hc.as_ref(), time_to_maturity)?;
            rc_tree = Some(tree);
            use_rates_credit = true;
        }

        // Resolve the effective HW parameters when using HullWhite model variants.
        // For HullWhiteCalibratedToSwaptions, attempt swaption calibration;
        // on failure, log a warning and fall back to HoLee.
        let effective_model = match &self.config.tree_model {
            TreeModelChoice::HullWhiteCalibratedToSwaptions {
                swaption_vol_surface_id,
            } if !use_rates_credit => Self::resolve_hw_calibrated(
                market_context,
                &discount_curve,
                swaption_vol_surface_id,
                time_to_maturity,
            ),
            other => other.clone(),
        };

        let mut sr_tree: Option<ShortRateTree> = None;
        let mut hw_tree: Option<HullWhiteTree> = None;

        if !use_rates_credit {
            match &effective_model {
                TreeModelChoice::HullWhite { kappa, sigma } => {
                    let hw_config = HullWhiteTreeConfig {
                        kappa: *kappa,
                        sigma: *sigma,
                        steps: self.config.tree_steps,
                        max_nodes: None,
                    };
                    hw_tree = Some(HullWhiteTree::calibrate(
                        hw_config,
                        discount_curve.as_ref(),
                        time_to_maturity,
                    )?);
                }
                TreeModelChoice::HoLee | TreeModelChoice::HullWhiteCalibratedToSwaptions { .. } => {
                    let tree_config = ShortRateTreeConfig {
                        steps: self.config.tree_steps,
                        volatility: self.config.volatility,
                        mean_reversion: self.config.mean_reversion,
                        ..Default::default()
                    };
                    let mut tree = ShortRateTree::new(tree_config);
                    tree.calibrate(discount_curve.as_ref(), time_to_maturity)?;
                    sr_tree = Some(tree);
                }
            }
        }

        let valuator = BondValuator::new(
            bond.clone(),
            market_context,
            as_of,
            time_to_maturity,
            self.config.tree_steps,
        )?;

        // Get initial short rate for state variables (needed by short-rate tree)
        let initial_rate = if let Some(tree) = sr_tree.as_ref() {
            tree.rate_at_node(0, 0).unwrap_or(0.03)
        } else {
            0.0 // Not used for rates+credit or HW tree
        };

        let objective_fn = |oas: f64| -> f64 {
            if use_rates_credit {
                let mut vars = StateVariables::default();
                vars.insert("oas", oas);
                if let Some(tree) = rc_tree.as_ref() {
                    match tree.price(vars, time_to_maturity, market_context, &valuator) {
                        Ok(model_price) => model_price - dirty_target,
                        Err(_) => 1.0e6,
                    }
                } else {
                    1.0e6
                }
            } else if let Some(ref tree) = hw_tree {
                // Hull-White trinomial tree: OAS applied inside backward induction
                let model_price = valuator.price_with_hw_tree(tree, oas);
                model_price - dirty_target
            } else {
                let mut vars = StateVariables::default();
                vars.insert(short_rate_keys::SHORT_RATE, initial_rate);
                vars.insert(short_rate_keys::OAS, oas);
                if let Some(tree) = sr_tree.as_ref() {
                    match tree.price(vars, time_to_maturity, market_context, &valuator) {
                        Ok(model_price) => model_price - dirty_target,
                        Err(_) => {
                            if oas > 0.0 {
                                1.0e6
                            } else {
                                -1.0e6
                            }
                        }
                    }
                } else {
                    1.0e6
                }
            }
        };

        let mut solver = BrentSolver::new()
            .tolerance(self.config.tolerance)
            .initial_bracket_size(self.config.initial_bracket_size_bp);
        // Respect the configured maximum iteration cap for OAS root-finding.
        solver.max_iterations = self.config.max_iterations;
        let initial_guess = 0.0;
        let oas_bp = solver.solve(objective_fn, initial_guess)?;
        Ok(oas_bp)
    }

    /// Attempt swaption-calibrated Hull-White. On failure, fall back to HoLee.
    ///
    /// Reads the swaption vol surface from the market context, converts grid
    /// points into `SwaptionQuote`s, and runs Levenberg-Marquardt calibration.
    fn resolve_hw_calibrated(
        market_context: &MarketContext,
        discount_curve: &std::sync::Arc<finstack_core::market_data::term_structures::DiscountCurve>,
        swaption_vol_surface_id: &str,
        time_to_maturity: f64,
    ) -> TreeModelChoice {
        use crate::calibration::hull_white::{
            calibrate_hull_white_to_swaptions_with_frequency, SwapFrequency, SwaptionQuote,
        };

        let surface = match market_context.get_surface(swaption_vol_surface_id) {
            Ok(s) => s,
            Err(_) => {
                tracing::warn!(
                    surface_id = swaption_vol_surface_id,
                    "Swaption vol surface not found in market context; \
                     falling back to HoLee tree model"
                );
                return TreeModelChoice::HoLee;
            }
        };

        // Build SwaptionQuote list from the surface grid.
        // Convention: expiries axis = swaption expiry (years),
        //             strikes axis = underlying swap tenor (years).
        // Each grid point is an ATM normal vol.
        let expiries = surface.expiries();
        let tenors = surface.strikes();
        let mut quotes = Vec::with_capacity(expiries.len() * tenors.len());
        for &expiry in expiries {
            // Only use swaptions expiring before the bond maturity
            if expiry > time_to_maturity || expiry <= 0.0 {
                continue;
            }
            for &tenor in tenors {
                if tenor <= 0.0 {
                    continue;
                }
                let vol = surface.value_clamped(expiry, tenor);
                if vol > 0.0 && vol.is_finite() {
                    quotes.push(SwaptionQuote {
                        expiry,
                        tenor,
                        volatility: vol,
                        is_normal_vol: true,
                    });
                }
            }
        }

        if quotes.len() < 2 {
            tracing::warn!(
                surface_id = swaption_vol_surface_id,
                n_valid = quotes.len(),
                "Insufficient swaption quotes from vol surface; \
                 falling back to HoLee tree model"
            );
            return TreeModelChoice::HoLee;
        }

        let dc = discount_curve.clone();
        let df_fn = move |t: f64| dc.df(t);

        match calibrate_hull_white_to_swaptions_with_frequency(
            &df_fn,
            &quotes,
            SwapFrequency::SemiAnnual,
        ) {
            Ok((params, report)) => {
                if report.success {
                    tracing::info!(
                        kappa = params.kappa,
                        sigma = params.sigma,
                        n_quotes = quotes.len(),
                        "Hull-White calibrated to swaptions"
                    );
                    TreeModelChoice::HullWhite {
                        kappa: params.kappa,
                        sigma: params.sigma,
                    }
                } else {
                    tracing::warn!(
                        reason = report.convergence_reason.as_str(),
                        "Swaption calibration did not converge; \
                         falling back to HoLee tree model"
                    );
                    TreeModelChoice::HoLee
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Swaption calibration failed; falling back to HoLee tree model"
                );
                TreeModelChoice::HoLee
            }
        }
    }
}

impl Default for TreePricer {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate option-adjusted spread for a bond given market price.
///
/// Convenience function using default tree configuration. This is a wrapper
/// around `TreePricer::new().calculate_oas()` for simple use cases.
///
/// # Arguments
///
/// * `bond` - The bond to calculate OAS for
/// * `market_context` - Market context with curves
/// * `as_of` - Valuation date
/// * `clean_price` - Market clean price as percentage of par
///
/// # Returns
///
/// OAS in basis points.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::instruments::fixed_income::bond::pricing::engine::tree::calculate_oas;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example().unwrap();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// let oas_bp = calculate_oas(&bond, &market, as_of, 98.5)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn calculate_oas(
    bond: &Bond,
    market_context: &MarketContext,
    as_of: Date,
    clean_price: f64,
) -> Result<f64> {
    let calculator = TreePricer::new();
    calculator.calculate_oas(bond, market_context, as_of, clean_price)
}
