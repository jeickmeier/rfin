//! Metrics framework bindings for WASM.
//!
//! Provides access to financial metrics computation including bond metrics,
//! IRS metrics, options Greeks, credit sensitivities, and bucketed risk.
//!
//! ## Usage
//!
//! ```typescript
//! import { MetricRegistry, MetricId, Bond, MarketContext } from 'finstack-wasm';
//!
//! // Create standard registry
//! const registry = MetricRegistry.standard();
//!
//! // Create instrument and market
//! const bond = Bond.treasury(...);
//! const market = new MarketContext();
//! // ... populate market with curves
//!
//! // Compute single metric
//! const pv = registry.computeMetric(bond, market, "pv");
//!
//! // Compute multiple metrics (more efficient)
//! const metrics = registry.computeMetrics(bond, market, [
//!   "pv", "dv01", "duration_modified", "convexity", "ytm"
//! ]);
//!
//! console.log(`PV: ${metrics.get("pv")}`);
//! console.log(`DV01: ${metrics.get("dv01")}`);
//! ```
//!
//! ## Available Metrics
//!
//! ### Bond Metrics
//! - Present value: `pv`
//! - Pricing: `clean_price`, `dirty_price`, `accrued_interest`
//! - Yield: `ytm`, `ytw`
//! - Duration: `duration_modified`, `duration_macaulay`
//! - Risk: `dv01`, `convexity`
//! - Credit: `z_spread`, `i_spread`, `oas`, `asw`, `cs01`
//!
//! ### IRS Metrics
//! - Legs: `fixed_leg_pv`, `floating_leg_pv`
//! - Risk: `dv01`, `annuity`, `par_rate`
//!
//! ### Options Greeks
//! - First-order: `delta`, `vega`, `theta`, `rho`
//! - Second-order: `gamma`, `vanna`, `volga`, `veta`
//! - Third-order: `charm`, `color`, `speed`
//! - Other: `implied_vol`
//!
//! ### Credit Metrics
//! - Spread: `spread`, `cs01`, `hazard_cs01`
//! - Probabilities: `survival_probability`, `default_probability`
//! - Sensitivity: `recovery_01`
//!
//! ### Variance Swap Metrics
//! - `variance_vega`, `expected_variance`, `realized_variance`
//! - `variance_notional`, `variance_strike_vol`

pub mod ids;
pub mod registry;

pub use ids::JsMetricId;
pub use registry::JsMetricRegistry;
