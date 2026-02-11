//! Pricing overrides for market-quoted instruments.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{Bps, Percentage};

/// Policy for evaluating volatility surfaces outside their calibrated grid.
///
/// Market-standard production systems typically make this choice explicit because
/// extrapolation can materially affect PV and greeks.
///
/// # Market Standards
///
/// - **Error**: Conservative approach for production systems; forces explicit handling.
/// - **Clamp**: Simple flat extrapolation; common for quick prototyping.
/// - **LinearInVariance**: Market-standard for equity/FX; preserves no-arbitrage conditions
///   better than linear-in-vol by extrapolating in total variance space (σ²T).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum VolSurfaceExtrapolation {
    /// Fail fast if `(expiry, strike)` is out of bounds.
    #[default]
    Error,
    /// Flat extrapolation to the nearest edge (clamp to grid).
    Clamp,
    /// Linear extrapolation in total variance space (σ²T).
    ///
    /// This is the market-standard approach for equity and FX volatility surfaces
    /// because it preserves the no-arbitrage condition that total variance must
    /// increase with time. The extrapolated volatility is computed as:
    ///
    /// ```text
    /// σ(T_extrap) = sqrt(σ²(T_edge) * T_edge / T_extrap + slope * (T_extrap - T_edge) / T_extrap)
    /// ```
    ///
    /// where `slope` is derived from the variance gradient at the edge.
    ///
    /// # When to Use
    ///
    /// - Long-dated option pricing where expiries exceed the calibrated grid
    /// - Scenario analysis requiring extrapolation to extreme tenors
    /// - Bootstrapping procedures that need consistent variance behavior
    ///
    /// # References
    ///
    /// - Gatheral, J. (2006). *The Volatility Surface*. Chapter 3.
    /// - Fengler, M. R. (2009). "Arbitrage-free smoothing of the implied volatility surface."
    LinearInVariance,
}

/// Optional parameters that override model pricing with market quotes.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PricingOverrides {
    /// Quoted clean price for bond yield calculations
    pub quoted_clean_price: Option<f64>,
    /// Rho bump size in **decimal rate** units (default `0.0001 = 1bp`).
    ///
    /// Note: internal curve-bump APIs often take bump sizes in **bp** units (`1.0 = 1bp`).
    /// Prefer using [`PricingOverrides::rho_bump_bp`] when wiring into `BumpSpec::parallel_bp`
    /// or `metrics::bump_discount_curve_parallel` to avoid unit mistakes.
    pub rho_bump_decimal: Option<f64>,
    /// Vega bump size in decimal (default 0.01 = 1%)
    pub vega_bump_decimal: Option<f64>,
    /// Implied volatility (overrides vol surface)
    pub implied_volatility: Option<f64>,
    /// Volatility surface extrapolation policy when `implied_volatility` is not set.
    #[serde(default)]
    pub vol_surface_extrapolation: VolSurfaceExtrapolation,
    /// Quoted spread (for credit instruments)
    pub quoted_spread_bp: Option<f64>,
    /// PV adjustment at valuation date (for CDS, CDSIndex, convertibles).
    ///
    /// This is an **already-discounted** adjustment to the net present value.
    /// It is added directly to the NPV without further discounting.
    ///
    /// # Sign Convention
    ///
    /// - Positive value: increases NPV (e.g., premium received)
    /// - Negative value: decreases NPV (e.g., premium paid)
    ///
    /// # Relationship to CDS Dated Upfront
    ///
    /// For CDS, this is distinct from `CreditDefaultSwap.upfront: Option<(Date, Money)>`:
    /// - **`upfront_payment`**: PV adjustment at `as_of`, added directly
    /// - **`CreditDefaultSwap.upfront`**: Dated cashflow, discounted from payment date
    ///
    /// Both can be set simultaneously without double-counting.
    pub upfront_payment: Option<Money>,
    /// Optional YTM bump size for numerical metrics (e.g., convexity/duration), in decimal (1 bp = 1e-4)
    pub ytm_bump_decimal: Option<f64>,
    /// Theta period for time decay calculations (e.g., "1D", "1W", "1M", "3M")
    pub theta_period: Option<String>,
    /// MC seed scenario override for deterministic greek calculations
    ///
    /// When computing greeks via finite differences, this allows specifying
    /// a scenario name (e.g., "delta_up", "vega_down") to derive deterministic
    /// seeds. If None, uses default seed or derives from instrument ID + "base".
    pub mc_seed_scenario: Option<String>,
    /// Enable adaptive bump sizes based on volatility and moneyness
    ///
    /// When true, bump sizes are scaled based on:
    /// - Volatility level (higher vol → larger bumps)
    /// - Time to expiry (longer dated → larger bumps)
    /// - Moneyness (deep ITM/OTM → smaller bumps)
    ///
    /// Default: false (use fixed bump sizes)
    pub adaptive_bumps: bool,
    /// Custom spot bump size override (as percentage, e.g., 0.01 for 1%)
    ///
    /// When set, overrides both standard and adaptive spot bump calculations.
    pub spot_bump_pct: Option<f64>,
    /// Custom volatility bump size override (as absolute vol, e.g., 0.01 for 1% vol)
    ///
    /// When set, overrides both standard and adaptive volatility bump calculations.
    pub vol_bump_pct: Option<f64>,
    /// Custom rate bump size override (in basis points, e.g., 1.0 for 1bp)
    ///
    /// When set, overrides both standard and adaptive rate bump calculations.
    pub rate_bump_bp: Option<f64>,
    /// Custom credit spread bump size override (in basis points, e.g., 1.0 for 1bp).
    ///
    /// Used by CS01 calculations that bump par spreads / hazard calibration quotes.
    pub credit_spread_bump_bp: Option<f64>,
    /// Term loan specific overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_loan: Option<TermLoanOverrides>,

    // ----- Tree Pricing Overrides -----
    /// Number of time steps for tree-based pricing (e.g., 100)
    pub tree_steps: Option<usize>,
    /// Volatility for tree-based pricing (annualized).
    /// Interpretation depends on the model (Normal vs Lognormal).
    pub tree_volatility: Option<f64>,

    // ----- Callability / Exercise Friction -----
    /// Exercise friction cost for issuer/borrower calls, expressed as **cents per 100 of par**.
    ///
    /// This models the real-world costs of refinancing / reissue (fees, OID, documentation),
    /// by requiring the issuer/borrower to see sufficient economic benefit before exercising.
    ///
    /// ## Convention
    /// - `0.0` (or `None`) means frictionless optimal exercise (pure model)
    /// - `50.0` means **$0.50 per $100** of outstanding principal (0.50 points)
    /// - `200.0` means **$2.00 per $100** of outstanding principal (2.00 points)
    ///
    /// The friction affects the **exercise decision threshold**, but redemption still occurs
    /// at the contractual call price.
    pub call_friction_cents: Option<f64>,

    // ----- Scenario Shock Fields -----
    /// Scenario price shock as decimal percentage (e.g., -0.05 for -5% price shock).
    ///
    /// When set, the model price is multiplied by (1 + scenario_price_shock_pct).
    /// This allows scenario analysis to apply uniform price shocks to instruments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_price_shock_pct: Option<f64>,

    /// Scenario spread shock in basis points (e.g., 50.0 for +50bp spread shock).
    ///
    /// When set, this spread shock is added to the instrument's pricing spread.
    /// For credit instruments, this translates to a wider/tighter spread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_spread_shock_bp: Option<f64>,
}

impl PricingOverrides {
    /// Rho bump size expressed in **basis points** (bp) suitable for curve bump APIs.
    ///
    /// Conversions:
    /// - `0.0001` (decimal) = `1.0` (bp)
    /// - `0.0010` (decimal) = `10.0` (bp)
    ///
    /// This helper exists to prevent accidental \(10{,}000\times\) unit errors when
    /// calling APIs that expect bp units.
    pub fn rho_bump_bp(&self) -> f64 {
        self.rho_bump_decimal.unwrap_or(0.0001) * 10000.0
    }
}

impl PricingOverrides {
    /// Create empty pricing overrides
    pub fn none() -> Self {
        Self::default()
    }

    /// Set quoted clean price
    pub fn with_clean_price(mut self, price: f64) -> Self {
        self.quoted_clean_price = Some(price);
        self
    }

    /// Set implied volatility
    pub fn with_implied_vol(mut self, vol: f64) -> Self {
        self.implied_volatility = Some(vol);
        self
    }

    /// Set implied volatility using a typed percentage.
    pub fn with_implied_vol_pct(mut self, vol: Percentage) -> Self {
        self.implied_volatility = Some(vol.as_decimal());
        self
    }

    /// Set volatility surface extrapolation policy.
    pub fn with_vol_surface_extrapolation(mut self, policy: VolSurfaceExtrapolation) -> Self {
        self.vol_surface_extrapolation = policy;
        self
    }

    /// Use linear-in-variance extrapolation for vol surfaces.
    ///
    /// This is the market-standard approach for equity/FX volatility surfaces
    /// when extrapolation is required beyond the calibrated grid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
    ///
    /// let overrides = PricingOverrides::none().with_linear_in_variance_extrapolation();
    /// ```
    pub fn with_linear_in_variance_extrapolation(mut self) -> Self {
        self.vol_surface_extrapolation = VolSurfaceExtrapolation::LinearInVariance;
        self
    }

    /// Set quoted spread
    pub fn with_spread_bp(mut self, spread_bp: f64) -> Self {
        self.quoted_spread_bp = Some(spread_bp);
        self
    }

    /// Set quoted spread using a typed basis-point value.
    pub fn with_spread_bps(mut self, spread_bp: Bps) -> Self {
        self.quoted_spread_bp = Some(spread_bp.as_bps() as f64);
        self
    }

    /// Set upfront payment
    pub fn with_upfront(mut self, upfront: Money) -> Self {
        self.upfront_payment = Some(upfront);
        self
    }

    /// Set custom YTM bump size (decimal). For 1 bp, pass 1e-4.
    pub fn with_ytm_bump_decimal(mut self, bump: f64) -> Self {
        self.ytm_bump_decimal = Some(bump);
        self
    }

    /// Set custom YTM bump size using basis points.
    pub fn with_ytm_bump_bps(mut self, bump: Bps) -> Self {
        self.ytm_bump_decimal = Some(bump.as_decimal());
        self
    }

    /// Set theta period for time decay calculations.
    pub fn with_theta_period(mut self, period: impl Into<String>) -> Self {
        self.theta_period = Some(period.into());
        self
    }

    /// Set MC seed scenario for deterministic greek calculations.
    ///
    /// The scenario name (e.g., "delta_up", "vega_down") is used to derive
    /// a deterministic seed from the instrument ID, ensuring reproducibility.
    pub fn with_mc_seed_scenario(mut self, scenario: impl Into<String>) -> Self {
        self.mc_seed_scenario = Some(scenario.into());
        self
    }

    /// Enable adaptive bump sizes for greek calculations.
    ///
    /// Adaptive bumps scale based on volatility, time to expiry, and moneyness
    /// to improve numerical stability for extreme parameter values.
    pub fn with_adaptive_bumps(mut self, enable: bool) -> Self {
        self.adaptive_bumps = enable;
        self
    }

    /// Set custom spot bump size (as percentage, e.g., 0.01 for 1%).
    ///
    /// Overrides both standard and adaptive calculations when set.
    pub fn with_spot_bump(mut self, bump_pct: f64) -> Self {
        self.spot_bump_pct = Some(bump_pct);
        self
    }

    /// Set custom spot bump size using a typed percentage.
    pub fn with_spot_bump_pct(mut self, bump_pct: Percentage) -> Self {
        self.spot_bump_pct = Some(bump_pct.as_decimal());
        self
    }

    /// Set custom volatility bump size (as absolute vol, e.g., 0.01 for 1% vol).
    ///
    /// Overrides both standard and adaptive calculations when set.
    pub fn with_vol_bump(mut self, bump_pct: f64) -> Self {
        self.vol_bump_pct = Some(bump_pct);
        self
    }

    /// Set custom volatility bump size using a typed percentage.
    pub fn with_vol_bump_pct(mut self, bump_pct: Percentage) -> Self {
        self.vol_bump_pct = Some(bump_pct.as_decimal());
        self
    }

    /// Set custom rate bump size (in basis points, e.g., 1.0 for 1bp).
    ///
    /// Overrides both standard and adaptive calculations when set.
    pub fn with_rate_bump(mut self, bump_bp: f64) -> Self {
        self.rate_bump_bp = Some(bump_bp);
        self
    }

    /// Set custom rate bump size using a typed basis-point value.
    pub fn with_rate_bump_bps(mut self, bump_bp: Bps) -> Self {
        self.rate_bump_bp = Some(bump_bp.as_bps() as f64);
        self
    }

    /// Set custom credit spread bump size (in basis points, e.g., 1.0 for 1bp).
    pub fn with_credit_spread_bump(mut self, bump_bp: f64) -> Self {
        self.credit_spread_bump_bp = Some(bump_bp);
        self
    }

    /// Set custom credit spread bump size using a typed basis-point value.
    pub fn with_credit_spread_bump_bps(mut self, bump_bp: Bps) -> Self {
        self.credit_spread_bump_bp = Some(bump_bp.as_bps() as f64);
        self
    }

    /// Set number of time steps for tree-based pricing.
    pub fn with_tree_steps(mut self, steps: usize) -> Self {
        self.tree_steps = Some(steps);
        self
    }

    /// Set volatility for tree-based pricing.
    pub fn with_tree_volatility(mut self, vol: f64) -> Self {
        self.tree_volatility = Some(vol);
        self
    }

    /// Set volatility for tree-based pricing using a typed percentage.
    pub fn with_tree_volatility_pct(mut self, vol: Percentage) -> Self {
        self.tree_volatility = Some(vol.as_decimal());
        self
    }

    /// Set issuer/borrower call exercise friction, in **cents per 100** of par.
    pub fn with_call_friction_cents(mut self, cents: f64) -> Self {
        self.call_friction_cents = Some(cents);
        self
    }

    /// Apply a scenario price shock (as decimal percentage).
    ///
    /// The shock is applied as a multiplier: `price * (1 + shock_pct)`.
    /// For example, -0.05 represents a -5% price shock.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
    ///
    /// // Apply a -10% price shock
    /// let overrides = PricingOverrides::none().with_price_shock_pct(-0.10);
    /// assert_eq!(overrides.scenario_price_shock_pct, Some(-0.10));
    /// ```
    pub fn with_price_shock_pct(mut self, shock_pct: f64) -> Self {
        self.scenario_price_shock_pct = Some(shock_pct);
        self
    }

    /// Apply a scenario spread shock (in basis points).
    ///
    /// The shock is added to any existing spread: `spread + shock_bp`.
    /// For example, 50.0 represents a +50bp spread widening.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
    ///
    /// // Apply a +25bp spread widening
    /// let overrides = PricingOverrides::none().with_spread_shock_bp(25.0);
    /// assert_eq!(overrides.scenario_spread_shock_bp, Some(25.0));
    /// ```
    pub fn with_spread_shock_bp(mut self, shock_bp: f64) -> Self {
        self.scenario_spread_shock_bp = Some(shock_bp);
        self
    }

    /// Apply a scenario spread shock using a typed basis-point value.
    pub fn with_spread_shock_bps(mut self, shock_bp: Bps) -> Self {
        self.scenario_spread_shock_bp = Some(shock_bp.as_bps() as f64);
        self
    }

    /// Clear any scenario shocks applied to this override.
    pub fn clear_scenario_shocks(&mut self) {
        self.scenario_price_shock_pct = None;
        self.scenario_spread_shock_bp = None;
    }

    /// Check if any scenario shock is applied.
    pub fn has_scenario_shock(&self) -> bool {
        self.scenario_price_shock_pct.is_some() || self.scenario_spread_shock_bp.is_some()
    }
}
// tests moved to end of file to satisfy clippy::items_after_test_module

impl PricingOverrides {
    /// Validate override values for finiteness and non-negativity; basic `theta_period` sanity.
    pub fn validate(&self) -> finstack_core::Result<()> {
        use finstack_core::InputError;
        let nonneg = |v: f64| v.is_finite() && v >= 0.0;
        if let Some(v) = self.quoted_clean_price {
            if !v.is_finite() {
                return Err(InputError::Invalid.into());
            }
        }
        if let Some(v) = self.implied_volatility {
            if !nonneg(v) {
                return Err(InputError::NegativeValue.into());
            }
        }
        if let Some(v) = self.quoted_spread_bp {
            if !nonneg(v) {
                return Err(InputError::NegativeValue.into());
            }
        }
        if let Some(v) = self.ytm_bump_decimal {
            if !nonneg(v) {
                return Err(InputError::NegativeValue.into());
            }
        }
        if let Some(v) = self.spot_bump_pct {
            if !nonneg(v) {
                return Err(InputError::NegativeValue.into());
            }
        }
        if let Some(v) = self.vol_bump_pct {
            if !nonneg(v) {
                return Err(InputError::NegativeValue.into());
            }
        }
        if let Some(v) = self.rate_bump_bp {
            if !nonneg(v) {
                return Err(InputError::NegativeValue.into());
            }
        }
        if let Some(steps) = self.tree_steps {
            if steps == 0 {
                return Err(InputError::Invalid.into());
            }
        }
        if let Some(v) = self.tree_volatility {
            if !nonneg(v) {
                return Err(InputError::NegativeValue.into());
            }
        }
        if let Some(v) = self.call_friction_cents {
            if !nonneg(v) {
                return Err(InputError::NegativeValue.into());
            }
        }
        if let Some(ref s) = self.theta_period {
            // Minimal sanity: allow forms like "1D", "1W", "1M", "1Y"
            let ok = s.len() >= 2
                && s[..s.len() - 1].chars().all(|c| c.is_ascii_digit())
                && matches!(s.chars().last(), Some('D' | 'W' | 'M' | 'Y'));
            if !ok {
                return Err(InputError::Invalid.into());
            }
        }
        Ok(())
    }
}

/// Term loan specific overrides for covenants and schedule adjustments.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TermLoanOverrides {
    /// Additional margin step-ups by date (bps)
    pub margin_add_bp_by_date: Vec<(Date, i32)>,
    /// Force PIK toggles by date
    pub pik_toggle_by_date: Vec<(Date, bool)>,
    /// Extra cash sweeps by date
    pub extra_cash_sweeps: Vec<(Date, Money)>,
    /// Draw stop date (earliest date after which draws are blocked)
    pub draw_stop_date: Option<Date>,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_default_and_positive_values() {
        let po = PricingOverrides::default()
            .with_clean_price(100.0)
            .with_ytm_bump_decimal(1e-4)
            .with_spot_bump(0.01)
            .with_vol_bump(0.01)
            .with_rate_bump(1.0);
        assert!(po.validate().is_ok());
    }

    #[test]
    fn validate_rejects_negative_values() {
        let po = PricingOverrides::default().with_vol_bump(-0.01);
        let err = po.validate().expect_err("should fail");
        match err {
            finstack_core::Error::Input(finstack_core::InputError::NegativeValue) => {}
            e => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_vol_surface_extrapolation_policies() {
        // Default is Error
        let po = PricingOverrides::default();
        assert_eq!(po.vol_surface_extrapolation, VolSurfaceExtrapolation::Error);

        // Can set to Clamp
        let po =
            PricingOverrides::none().with_vol_surface_extrapolation(VolSurfaceExtrapolation::Clamp);
        assert_eq!(po.vol_surface_extrapolation, VolSurfaceExtrapolation::Clamp);

        // Can set to LinearInVariance via dedicated method
        let po = PricingOverrides::none().with_linear_in_variance_extrapolation();
        assert_eq!(
            po.vol_surface_extrapolation,
            VolSurfaceExtrapolation::LinearInVariance
        );

        // Can set to LinearInVariance via general method
        let po = PricingOverrides::none()
            .with_vol_surface_extrapolation(VolSurfaceExtrapolation::LinearInVariance);
        assert_eq!(
            po.vol_surface_extrapolation,
            VolSurfaceExtrapolation::LinearInVariance
        );
    }

    #[test]
    fn test_vol_surface_extrapolation_serde() {
        // Test serialization roundtrip for all policies
        for policy in [
            VolSurfaceExtrapolation::Error,
            VolSurfaceExtrapolation::Clamp,
            VolSurfaceExtrapolation::LinearInVariance,
        ] {
            let po = PricingOverrides::none().with_vol_surface_extrapolation(policy);
            let json = serde_json::to_string(&po).expect("serialize");
            let roundtrip: PricingOverrides = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(roundtrip.vol_surface_extrapolation, policy);
        }

        // Check snake_case serialization
        let po = PricingOverrides::none().with_linear_in_variance_extrapolation();
        let json = serde_json::to_string(&po).expect("serialize");
        assert!(
            json.contains("linear_in_variance"),
            "Should serialize as snake_case: {}",
            json
        );
    }
}
