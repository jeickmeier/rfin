//! Pricing overrides for market-quoted instruments.

use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Optional parameters that override model pricing with market quotes.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PricingOverrides {
    /// Quoted clean price for bond yield calculations
    pub quoted_clean_price: Option<f64>,
    /// Rho bump size in decimal (default 0.0001 = 1bp)
    pub rho_bump_decimal: Option<f64>,
    /// Vega bump size in decimal (default 0.01 = 1%)
    pub vega_bump_decimal: Option<f64>,
    /// Implied volatility (overrides vol surface)
    pub implied_volatility: Option<f64>,
    /// Quoted spread (for credit instruments)
    pub quoted_spread_bp: Option<f64>,
    /// Upfront payment (for CDS, convertibles)
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
    /// Term loan specific overrides
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub term_loan: Option<TermLoanOverrides>,

    // ----- Tree Pricing Overrides -----
    /// Number of time steps for tree-based pricing (e.g., 100)
    pub tree_steps: Option<usize>,
    /// Volatility for tree-based pricing (annualized).
    /// Interpretation depends on the model (Normal vs Lognormal).
    pub tree_volatility: Option<f64>,

    // ----- Scenario Shock Fields -----
    /// Scenario price shock as decimal percentage (e.g., -0.05 for -5% price shock).
    ///
    /// When set, the model price is multiplied by (1 + scenario_price_shock_pct).
    /// This allows scenario analysis to apply uniform price shocks to instruments.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub scenario_price_shock_pct: Option<f64>,

    /// Scenario spread shock in basis points (e.g., 50.0 for +50bp spread shock).
    ///
    /// When set, this spread shock is added to the instrument's pricing spread.
    /// For credit instruments, this translates to a wider/tighter spread.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub scenario_spread_shock_bp: Option<f64>,
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

    /// Set quoted spread
    pub fn with_spread_bp(mut self, spread_bp: f64) -> Self {
        self.quoted_spread_bp = Some(spread_bp);
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

    /// Set custom volatility bump size (as absolute vol, e.g., 0.01 for 1% vol).
    ///
    /// Overrides both standard and adaptive calculations when set.
    pub fn with_vol_bump(mut self, bump_pct: f64) -> Self {
        self.vol_bump_pct = Some(bump_pct);
        self
    }

    /// Set custom rate bump size (in basis points, e.g., 1.0 for 1bp).
    ///
    /// Overrides both standard and adaptive calculations when set.
    pub fn with_rate_bump(mut self, bump_bp: f64) -> Self {
        self.rate_bump_bp = Some(bump_bp);
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
        use finstack_core::error::InputError;
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
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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
            finstack_core::error::Error::Input(finstack_core::error::InputError::NegativeValue) => {
            }
            e => panic!("unexpected error: {e:?}"),
        }
    }
}
