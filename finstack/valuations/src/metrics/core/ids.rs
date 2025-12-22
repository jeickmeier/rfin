//! Strongly-typed metric identifiers for compile-time validation.
//!
//! Provides a comprehensive set of metric IDs covering bond, IRS, deposit,
//! and risk metrics. Each ID is strongly-typed to prevent runtime errors
//! and enable compile-time validation of metric dependencies.
//!
//! # Metric Categories
//!
//! - **Bond metrics**: Yield, duration, convexity, pricing, credit spreads
//! - **IRS metrics**: DV01, annuity factors, par rates, present values
//! - **Deposit metrics**: Discount factors, par rates, year fractions
//! - **Risk metrics**: DV01 (standard for all parallel rate sensitivity), CS01, BucketedDV01, BucketedCS01, Theta, and all standardized "01" sensitivity metrics
//! - **Standardized sensitivity metrics**: Dividend01, Inflation01, Prepayment01, Default01, Severity01, Conversion01, CollateralHaircut01, CollateralPrice01, Nav01, Carry01, Hurdle01, Dv01Domestic, Dv01Foreign, Fx01, Npv01, SpreadDv01, Correlation01, FxVega, ConvexityAdjustmentRisk
//! - **Custom metrics**: User-defined metrics with dynamic identifiers

use finstack_core::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

/// Strongly-typed metric identifier.
///
/// Provides compile-time validation, autocomplete support, and safe refactoring
/// when metric names change. Covers bond, IRS, deposit, and risk metrics.
///
/// See unit tests and `examples/` for usage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricId(Cow<'static, str>);

#[allow(non_upper_case_globals)] // PascalCase names maintained for backward compatibility
impl MetricId {
    /// Creates a custom metric ID.
    ///
    /// Use this for user-defined metrics that aren't part of the standard set.
    /// Custom metrics are stored as strings and can have any identifier.
    pub fn custom(id: impl Into<String>) -> Self {
        MetricId(Cow::Owned(id.into()))
    }

    /// Converts to string representation for compatibility.
    ///
    /// Returns a lowercase, snake_case string that can be used for
    /// serialization, logging, or API interfaces.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Checks if this is a custom (non-standard) metric.
    ///
    /// Returns `true` if the metric was created via `custom()` and is not
    /// part of the standard set.
    pub fn is_custom(&self) -> bool {
        !metric_lookup().contains_key(self.as_str())
    }

    /// Parses a string into a MetricId with strict validation.
    ///
    /// Unlike `FromStr`, this method returns an error for unknown metric names
    /// rather than creating a custom metric. Use this for user-provided inputs
    /// where typos should be caught, not silently accepted.
    ///
    /// # Errors
    ///
    /// Returns `Error::UnknownMetric` if the string does not match any standard
    /// metric. The error includes the invalid metric name and a list of all
    /// available standard metrics.
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_valuations::metrics::MetricId;
    ///
    /// // Parse known metric - succeeds
    /// let dv01 = MetricId::parse_strict("dv01").unwrap();
    /// assert_eq!(dv01, MetricId::Dv01);
    ///
    /// // Case insensitive
    /// let theta = MetricId::parse_strict("THETA").unwrap();
    /// assert_eq!(theta, MetricId::Theta);
    ///
    /// // Unknown metric - fails with error
    /// let result = MetricId::parse_strict("dv01x");
    /// assert!(result.is_err());
    /// ```
    ///
    /// # Migration from FromStr
    ///
    /// If you need backward compatibility (accept custom metrics), use `FromStr::from_str`
    /// or the `.parse()` method which never fails:
    ///
    /// ```
    /// use finstack_valuations::metrics::MetricId;
    /// use std::str::FromStr;
    ///
    /// // FromStr allows custom metrics (backwards compatible)
    /// let custom = MetricId::from_str("my_custom_metric").unwrap();
    /// assert!(custom.is_custom());
    ///
    /// // Strict parsing rejects unknown metrics
    /// let result = MetricId::parse_strict("my_custom_metric");
    /// assert!(result.is_err());
    /// ```
    pub fn parse_strict(s: &str) -> finstack_core::Result<Self> {
        let lower = s.to_lowercase();
        if let Some(id) = metric_lookup().get(&lower) {
            Ok(id.clone())
        } else {
            Err(finstack_core::Error::unknown_metric(
                s,
                Self::ALL_STANDARD
                    .iter()
                    .map(|m| m.as_str().to_string())
                    .collect(),
            ))
        }
    }

    // ========================================================================
    // Core Risk Metrics
    // ========================================================================

    /// Time decay (theta) - 1D Day Time decay P&L
    pub const Theta: Self = Self(Cow::Borrowed("theta"));

    /// Dollar value of 01 (DV01) - Standard for all parallel rate sensitivity
    pub const Dv01: Self = Self(Cow::Borrowed("dv01"));

    /// Credit spread sensitivity (CS01) - Parallel shift in credit spread (quote spreads only)
    pub const Cs01: Self = Self(Cow::Borrowed("cs01"));

    /// Bucketed DV01 risk - Pointwise sensitivity to yield curve
    pub const BucketedDv01: Self = Self(Cow::Borrowed("bucketed_dv01"));

    /// Bucketed Credit Spread Risk - Pointwise sensitivity to credit spread
    pub const BucketedCs01: Self = Self(Cow::Borrowed("bucketed_cs01"));

    // ========================================================================
    // FX Spot Metrics
    // ========================================================================

    /// Spot rate
    pub const SpotRate: Self = Self(Cow::Borrowed("spot_rate"));

    /// Base amount
    pub const BaseAmount: Self = Self(Cow::Borrowed("base_amount"));

    /// Quote amount
    pub const QuoteAmount: Self = Self(Cow::Borrowed("quote_amount"));

    /// Inverse rate
    pub const InverseRate: Self = Self(Cow::Borrowed("inverse_rate"));

    // ========================================================================
    // Equity Metrics
    // ========================================================================

    /// Equity price per share - TODO: Remove this as input to metrics
    pub const EquityPricePerShare: Self = Self(Cow::Borrowed("equity_price_per_share"));

    /// Equity shares (effective) - TODO: Remove this as input to metrics
    pub const EquityShares: Self = Self(Cow::Borrowed("equity_shares"));

    /// Equity dividend yield (annualized, decimal) - TODO: Remove this as input to metrics
    pub const EquityDividendYield: Self = Self(Cow::Borrowed("equity_dividend_yield"));

    /// Equity forward price per share
    pub const EquityForwardPrice: Self = Self(Cow::Borrowed("equity_forward_price"));

    // ========================================================================
    // Bond Metrics
    // ========================================================================

    /// Dirty price (includes accrued interest)
    pub const DirtyPrice: Self = Self(Cow::Borrowed("dirty_price"));

    /// Clean price (excludes accrued interest)
    pub const CleanPrice: Self = Self(Cow::Borrowed("clean_price"));

    /// Accrued interest since last coupon payment
    pub const Accrued: Self = Self(Cow::Borrowed("accrued"));

    /// Accrued interest (alias of `accrued` for compatibility) - TODO: Remove this
    pub const AccruedInterest: Self = Self(Cow::Borrowed("accrued_interest"));

    /// Yield to maturity
    pub const Ytm: Self = Self(Cow::Borrowed("ytm"));

    /// Yield to worst
    pub const Ytw: Self = Self(Cow::Borrowed("ytw"));

    /// Macaulay duration
    pub const DurationMac: Self = Self(Cow::Borrowed("duration_mac"));

    /// Modified duration
    pub const DurationMod: Self = Self(Cow::Borrowed("duration_mod"));

    /// Convexity
    pub const Convexity: Self = Self(Cow::Borrowed("convexity"));

    // ========================================================================
    // Spread Metrics
    // ========================================================================

    /// Z-spread - Zero-vol spread
    pub const ZSpread: Self = Self(Cow::Borrowed("z_spread"));

    /// OAS - Option-adjusted spread
    pub const Oas: Self = Self(Cow::Borrowed("oas"));

    /// Embedded option value for callable/putable bonds (in currency units)
    ///
    /// For callable bonds: V_call = P_straight - P_callable (positive, issuer owns call)
    /// For putable bonds: V_put = P_putable - P_straight (positive, investor owns put)
    /// Returns 0 for bonds without embedded options.
    pub const EmbeddedOptionValue: Self = Self(Cow::Borrowed("embedded_option_value"));

    /// I-spread - Yield over interpolated swap curve
    pub const ISpread: Self = Self(Cow::Borrowed("i_spread"));

    /// Discount margin for floating-rate bonds (decimal; 0.01 = 100 bps)
    pub const DiscountMargin: Self = Self(Cow::Borrowed("discount_margin"));

    /// G-spread - Govvie spread
    pub const GSpread: Self = Self(Cow::Borrowed("g_spread"));

    /// TODO: We should rationalize these ASW metrics to a single market standard ASW metric
    /// ASW-spread - Asset swap spread (par)
    pub const ASWSpread: Self = Self(Cow::Borrowed("asw_spread"));

    /// Par asset swap spread
    pub const ASWPar: Self = Self(Cow::Borrowed("asw_par"));

    /// Market (price) asset swap spread
    pub const ASWMarket: Self = Self(Cow::Borrowed("asw_market"));

    /// Par asset swap spread using forward curve (requires FloatingCouponSpec or explicit forward)
    pub const ASWParFwd: Self = Self(Cow::Borrowed("asw_par_fwd"));

    /// Market asset swap spread using forward curve (requires FloatingCouponSpec or explicit forward)
    pub const ASWMarketFwd: Self = Self(Cow::Borrowed("asw_market_fwd"));

    // ========================================================================
    // IRS Metrics
    // ========================================================================

    /// Annuity factor for fixed leg
    pub const Annuity: Self = Self(Cow::Borrowed("annuity"));

    /// Par swap rate
    pub const ParRate: Self = Self(Cow::Borrowed("par_rate"));

    /// Present value of 01 (PV01) - TODO: Change this to valude of shifting coupon by 1bp
    pub const Pv01: Self = Self(Cow::Borrowed("pv01"));

    /// Present value of fixed leg
    pub const PvFixed: Self = Self(Cow::Borrowed("pv_fixed"));

    /// Present value of floating leg
    pub const PvFloat: Self = Self(Cow::Borrowed("pv_float"));

    // ========================================================================
    // Deposit Metrics
    // ========================================================================

    /// Year fraction - TODO: Do we need this?
    pub const Yf: Self = Self(Cow::Borrowed("yf"));

    /// Discount factor at start date - TODO: Do we need this?
    pub const DfStart: Self = Self(Cow::Borrowed("df_start"));

    /// Discount factor at end date - TODO: Do we need this?
    pub const DfEnd: Self = Self(Cow::Borrowed("df_end"));

    /// Deposit par rate
    pub const DepositParRate: Self = Self(Cow::Borrowed("deposit_par_rate"));

    /// Discount factor at end date from quote - TODO: Do we need this?
    pub const DfEndFromQuote: Self = Self(Cow::Borrowed("df_end_from_quote"));

    /// Quote rate - TODO: Is both deposit par rate and quote rate the same thing?
    pub const QuoteRate: Self = Self(Cow::Borrowed("quote_rate"));

    // ========================================================================
    // CDS Metrics
    // ========================================================================

    /// Par spread for CDS
    pub const ParSpread: Self = Self(Cow::Borrowed("par_spread"));

    /// Risky PV01 for CDS
    pub const RiskyPv01: Self = Self(Cow::Borrowed("risky_pv01"));

    /// Protection leg present value
    pub const ProtectionLegPv: Self = Self(Cow::Borrowed("protection_leg_pv"));

    /// Premium leg present value
    pub const PremiumLegPv: Self = Self(Cow::Borrowed("premium_leg_pv"));

    /// Jump-to-default amount
    pub const JumpToDefault: Self = Self(Cow::Borrowed("jump_to_default"));

    /// Expected loss
    pub const ExpectedLoss: Self = Self(Cow::Borrowed("expected_loss"));

    /// Default probability
    pub const DefaultProbability: Self = Self(Cow::Borrowed("default_probability"));

    /// Expected recovery rate
    pub const Recovery01: Self = Self(Cow::Borrowed("recovery_01"));

    // ========================================================================
    // Option Metrics
    // ========================================================================

    /// Delta (price sensitivity to underlying)
    pub const Delta: Self = Self(Cow::Borrowed("delta"));

    /// Gamma (delta sensitivity to underlying)
    pub const Gamma: Self = Self(Cow::Borrowed("gamma"));

    /// Vega (price sensitivity to volatility)
    pub const Vega: Self = Self(Cow::Borrowed("vega"));

    /// Bucketed Vega (surface point sensitivities)
    pub const BucketedVega: Self = Self(Cow::Borrowed("bucketed_vega"));

    /// Rho (price sensitivity to interest rates)
    pub const Rho: Self = Self(Cow::Borrowed("rho"));

    /// Foreign Rho (price sensitivity to foreign interest rates)
    pub const ForeignRho: Self = Self(Cow::Borrowed("foreign_rho"));

    /// Forward curve PV01 (price sensitivity to a 1bp forward curve bump)
    pub const ForwardPv01: Self = Self(Cow::Borrowed("forward_pv01"));

    /// Vanna (delta sensitivity to volatility)
    pub const Vanna: Self = Self(Cow::Borrowed("vanna"));

    /// Volga (vega sensitivity to volatility)
    pub const Volga: Self = Self(Cow::Borrowed("volga"));

    /// Veta (theta sensitivity to volatility)
    pub const Veta: Self = Self(Cow::Borrowed("veta"));

    /// Interest rate convexity (IRS, similar concept to bond convexity)
    pub const IrConvexity: Self = Self(Cow::Borrowed("ir_convexity"));

    /// Credit spread gamma (second derivative w.r.t spreads)
    pub const CsGamma: Self = Self(Cow::Borrowed("cs_gamma"));

    /// Inflation convexity (second derivative w.r.t inflation)
    pub const InflationConvexity: Self = Self(Cow::Borrowed("inflation_convexity"));

    /// Charm (rho sensitivity to volatility)
    pub const Charm: Self = Self(Cow::Borrowed("charm"));

    /// Color (gamma sensitivity to time)
    pub const Color: Self = Self(Cow::Borrowed("color"));

    /// Speed (gamma sensitivity to underlying)
    pub const Speed: Self = Self(Cow::Borrowed("speed"));

    /// Implied volatility (from price)
    pub const ImpliedVol: Self = Self(Cow::Borrowed("implied_vol"));

    // ========================================================================
    // Variance Swap Metrics
    // ========================================================================

    /// Vega expressed per variance point (variance swap sensitivity)
    pub const VarianceVega: Self = Self(Cow::Borrowed("variance_vega"));

    /// Expected variance under the pricing model
    pub const ExpectedVariance: Self = Self(Cow::Borrowed("variance_expected"));

    /// Realized variance computed from observed paths
    pub const RealizedVariance: Self = Self(Cow::Borrowed("variance_realized"));

    /// Variance notional exposure (payout multiplier)
    pub const VarianceNotional: Self = Self(Cow::Borrowed("variance_notional"));

    /// Strike volatility equivalent (sqrt of strike variance)
    pub const VarianceStrikeVol: Self = Self(Cow::Borrowed("variance_strike_vol"));

    /// Time to maturity as used in the variance swap conventions
    pub const VarianceTimeToMaturity: Self = Self(Cow::Borrowed("variance_time_to_maturity"));

    // ========================================================================
    // Other Risk Metrics
    // ========================================================================

    /// Dividend yield sensitivity per basis point
    pub const Dividend01: Self = Self(Cow::Borrowed("dividend01"));

    /// Inflation curve sensitivity per basis point
    pub const Inflation01: Self = Self(Cow::Borrowed("inflation01"));

    /// Prepayment rate sensitivity per basis point
    pub const Prepayment01: Self = Self(Cow::Borrowed("prepayment01"));

    /// Default rate sensitivity per basis point
    pub const Default01: Self = Self(Cow::Borrowed("default01"));

    /// Loss severity sensitivity per 1% change
    pub const Severity01: Self = Self(Cow::Borrowed("severity01"));

    /// Conversion ratio/price sensitivity per 1% change
    pub const Conversion01: Self = Self(Cow::Borrowed("conversion01"));

    /// Collateral haircut sensitivity per basis point
    pub const CollateralHaircut01: Self = Self(Cow::Borrowed("collateral_haircut01"));

    /// Collateral price sensitivity per 1% change
    pub const CollateralPrice01: Self = Self(Cow::Borrowed("collateral_price01"));

    /// NAV sensitivity per 1% change (private markets funds)
    pub const Nav01: Self = Self(Cow::Borrowed("nav01"));

    /// GP carry sensitivity per basis point (private markets funds)
    pub const Carry01: Self = Self(Cow::Borrowed("carry01"));

    /// Hurdle rate sensitivity per basis point (private markets funds)
    pub const Hurdle01: Self = Self(Cow::Borrowed("hurdle01"));

    /// DV01 for domestic currency (FX Swap)
    pub const Dv01Domestic: Self = Self(Cow::Borrowed("dv01_domestic"));

    /// DV01 for foreign currency (FX Swap)
    pub const Dv01Foreign: Self = Self(Cow::Borrowed("dv01_foreign"));

    /// FX spot rate sensitivity per basis point
    pub const Fx01: Self = Self(Cow::Borrowed("fx01"));

    /// NPV sensitivity per basis point (inflation swaps)
    pub const Npv01: Self = Self(Cow::Borrowed("npv01"));

    /// Running coupon sensitivity per basis point (CDS Tranche)
    pub const SpreadDv01: Self = Self(Cow::Borrowed("spread_dv01"));

    /// Correlation sensitivity per 1% change (unified for all correlation risks)
    pub const Correlation01: Self = Self(Cow::Borrowed("correlation01"));

    /// FX volatility sensitivity per 1% change (quanto options)
    pub const FxVega: Self = Self(Cow::Borrowed("fx_vega"));

    /// Convexity adjustment risk (CMS options)
    pub const ConvexityAdjustmentRisk: Self = Self(Cow::Borrowed("convexity_adjustment_risk"));

    // ========================================================================
    // TRS Metrics
    // ========================================================================

    /// Financing annuity for TRS
    pub const FinancingAnnuity: Self = Self(Cow::Borrowed("financing_annuity"));

    /// Index delta for TRS
    pub const IndexDelta: Self = Self(Cow::Borrowed("index_delta"));

    // ========================================================================
    // Basis Swap Metrics
    // ========================================================================

    /// PV of primary floating leg (includes spread)
    pub const PvPrimary: Self = Self(Cow::Borrowed("pv_primary"));

    /// PV of reference floating leg
    pub const PvReference: Self = Self(Cow::Borrowed("pv_reference"));

    /// Annuity of primary leg
    pub const AnnuityPrimary: Self = Self(Cow::Borrowed("annuity_primary"));

    /// Annuity of reference leg
    pub const AnnuityReference: Self = Self(Cow::Borrowed("annuity_reference"));

    /// DV01 of primary leg
    pub const Dv01Primary: Self = Self(Cow::Borrowed("dv01_primary"));

    /// DV01 of reference leg
    pub const Dv01Reference: Self = Self(Cow::Borrowed("dv01_reference"));

    /// Par spread for basis swap
    pub const BasisParSpread: Self = Self(Cow::Borrowed("basis_par_spread"));

    // ========================================================================
    // Repo Metrics
    // ========================================================================

    /// Market value of collateral
    pub const CollateralValue: Self = Self(Cow::Borrowed("collateral_value"));

    /// Required collateral value (with haircut)
    pub const RequiredCollateral: Self = Self(Cow::Borrowed("required_collateral"));

    /// Collateral coverage ratio
    pub const CollateralCoverage: Self = Self(Cow::Borrowed("collateral_coverage"));

    /// Repo interest amount
    pub const RepoInterest: Self = Self(Cow::Borrowed("repo_interest"));

    /// Funding risk (repo rate sensitivity)
    pub const FundingRisk: Self = Self(Cow::Borrowed("funding_risk"));

    /// Effective repo rate (adjusted for special collateral)
    pub const EffectiveRate: Self = Self(Cow::Borrowed("effective_rate"));

    /// Time to maturity in years
    pub const TimeToMaturity: Self = Self(Cow::Borrowed("time_to_maturity"));

    /// Implied collateral return
    pub const ImpliedCollateralReturn: Self = Self(Cow::Borrowed("implied_collateral_return"));

    // ========================================================================
    // Basket/ETF Metrics
    // ========================================================================

    /// Net Asset Value per share
    pub const Nav: Self = Self(Cow::Borrowed("nav"));

    /// Total basket value
    pub const BasketValue: Self = Self(Cow::Borrowed("basket_value"));

    /// Number of constituents in the basket
    pub const ConstituentCount: Self = Self(Cow::Borrowed("constituent_count"));

    /// Expense ratio as percentage
    pub const ExpenseRatio: Self = Self(Cow::Borrowed("expense_ratio"));

    /// Tracking error vs benchmark
    pub const TrackingError: Self = Self(Cow::Borrowed("tracking_error"));

    /// Utilization vs creation unit size
    pub const Utilization: Self = Self(Cow::Borrowed("utilization"));

    /// Premium/discount to NAV
    pub const PremiumDiscount: Self = Self(Cow::Borrowed("premium_discount"));

    // ========================================================================
    // Structured Credit Metrics
    // ========================================================================

    /// Weighted Average Life (years) - Expected principal repayment life
    pub const WAL: Self = Self(Cow::Borrowed("wal"));

    /// Weighted Average Maturity (years) - Pool maturity
    pub const WAM: Self = Self(Cow::Borrowed("wam"));

    /// Expected final payment date under base assumptions
    pub const ExpectedMaturity: Self = Self(Cow::Borrowed("expected_maturity"));

    /// Percentage of original pool balance remaining
    pub const PoolFactor: Self = Self(Cow::Borrowed("pool_factor"));

    /// Constant Prepayment Rate (annualized)
    pub const CPR: Self = Self(Cow::Borrowed("cpr"));

    /// Single Monthly Mortality (monthly prepayment rate)
    pub const SMM: Self = Self(Cow::Borrowed("smm"));

    /// Constant Default Rate (annualized)
    pub const CDR: Self = Self(Cow::Borrowed("cdr"));

    /// Loss Severity (1 - Recovery Rate)
    pub const LossSeverity: Self = Self(Cow::Borrowed("loss_severity"));

    /// Spread duration (time-weighted sensitivity to spread changes)
    pub const SpreadDuration: Self = Self(Cow::Borrowed("spread_duration"));

    /// DM01 - Discount margin sensitivity (for floating-rate CLO)
    pub const Dm01: Self = Self(Cow::Borrowed("dm01"));

    // ========================================================================
    // ABS-specific Metrics
    // ========================================================================

    /// Delinquency rate - Percentage of pool in delinquency
    pub const AbsDelinquency: Self = Self(Cow::Borrowed("abs_delinquency"));

    /// Charge-off rate - Percentage of pool charged off
    pub const AbsChargeOff: Self = Self(Cow::Borrowed("abs_charge_off"));

    /// Excess spread - Spread available to absorb losses
    pub const AbsExcessSpread: Self = Self(Cow::Borrowed("abs_excess_spread"));

    /// Credit enhancement level - Subordination as % of pool
    pub const AbsCreditEnhancement: Self = Self(Cow::Borrowed("abs_ce_level"));

    // ========================================================================
    // CLO-specific Metrics
    // ========================================================================

    /// Weighted Average Rating Factor
    pub const CloWarf: Self = Self(Cow::Borrowed("clo_warf"));

    /// Weighted Average Spread
    pub const CloWas: Self = Self(Cow::Borrowed("clo_was"));

    /// Weighted Average Coupon
    pub const CloWac: Self = Self(Cow::Borrowed("clo_wac"));

    /// Portfolio diversity score
    pub const CloDiversity: Self = Self(Cow::Borrowed("clo_diversity"));

    /// Overcollateralization ratio
    pub const CloOcRatio: Self = Self(Cow::Borrowed("clo_oc_ratio"));

    /// Interest coverage ratio
    pub const CloIcRatio: Self = Self(Cow::Borrowed("clo_ic_ratio"));

    /// Average recovery rate on defaults
    pub const CloRecoveryRate: Self = Self(Cow::Borrowed("clo_recovery_rate"));

    // ========================================================================
    // CMBS-specific Metrics
    // ========================================================================

    /// Debt Service Coverage Ratio
    pub const CmbsDscr: Self = Self(Cow::Borrowed("cmbs_dscr"));

    /// Weighted Average Loan-to-Value
    pub const CmbsWaltv: Self = Self(Cow::Borrowed("cmbs_waltv"));

    /// Credit Enhancement Level
    pub const CmbsCreditEnhancement: Self = Self(Cow::Borrowed("cmbs_ce_level"));

    // ========================================================================
    // RMBS-specific Metrics
    // ========================================================================

    /// PSA prepayment speed (e.g., 100% PSA)
    pub const RmbsPsaSpeed: Self = Self(Cow::Borrowed("rmbs_psa_speed"));

    /// SDA default speed
    pub const RmbsSdaSpeed: Self = Self(Cow::Borrowed("rmbs_sda_speed"));

    /// Weighted Average LTV for RMBS
    pub const RmbsWaltv: Self = Self(Cow::Borrowed("rmbs_waltv"));

    /// Weighted Average FICO score
    pub const RmbsWafico: Self = Self(Cow::Borrowed("rmbs_wafico"));

    // ========================================================================
    // Inflation-Linked Bond Metrics
    // ========================================================================

    /// Real yield (inflation-adjusted)
    pub const RealYield: Self = Self(Cow::Borrowed("real_yield"));

    /// Inflation index ratio
    pub const IndexRatio: Self = Self(Cow::Borrowed("index_ratio"));

    /// Real duration (inflation-adjusted duration)
    pub const RealDuration: Self = Self(Cow::Borrowed("real_duration"));

    /// Breakeven inflation rate
    pub const BreakevenInflation: Self = Self(Cow::Borrowed("breakeven_inflation"));

    // ========================================================================
    // Private Equity / Private Markets Fund Metrics
    // ========================================================================

    /// LP (Limited Partner) Internal Rate of Return
    pub const LpIrr: Self = Self(Cow::Borrowed("lp_irr"));

    /// GP (General Partner) Internal Rate of Return
    pub const GpIrr: Self = Self(Cow::Borrowed("gp_irr"));

    /// LP Multiple on Invested Capital
    pub const MoicLp: Self = Self(Cow::Borrowed("moic_lp"));

    /// LP Distributions to Paid In (DPI ratio)
    pub const DpiLp: Self = Self(Cow::Borrowed("dpi_lp"));

    /// LP Total Value to Paid In (TVPI ratio)
    pub const TvpiLp: Self = Self(Cow::Borrowed("tvpi_lp"));

    /// Accrued carry amount for GP
    pub const CarryAccrued: Self = Self(Cow::Borrowed("carry_accrued"));

    // ========================================================================
    // DCF / Corporate Valuation Metrics
    // ========================================================================

    /// Enterprise value (present value of all operating cashflows + terminal value)
    pub const EnterpriseValue: Self = Self(Cow::Borrowed("enterprise_value"));

    /// Equity value (enterprise value less net debt)
    pub const EquityValue: Self = Self(Cow::Borrowed("equity_value"));

    /// Present value of terminal value
    pub const TerminalValuePV: Self = Self(Cow::Borrowed("terminal_value_pv"));

    // ========================================================================
    // VaR Metrics
    // ========================================================================

    /// Conditional second-order theta (gamma of theta)
    pub const ThetaGamma: Self = Self(Cow::Borrowed("theta_gamma"));

    /// Historical Value-at-Risk (95% confidence by default)
    pub const HVAR: Self = Self(Cow::Borrowed("hvar"));

    /// Expected Shortfall / Conditional VaR (CVaR)
    pub const EXPECTED_SHORTFALL: Self = Self(Cow::Borrowed("expected_shortfall"));

    // ========================================================================
    // ALL_STANDARD Array
    // ========================================================================

    /// All standard (non-custom) metric IDs.
    pub const ALL_STANDARD: &'static [MetricId] = &[
        MetricId::Theta,
        MetricId::Dv01,
        MetricId::Cs01,
        MetricId::BucketedDv01,
        MetricId::BucketedCs01,
        MetricId::SpotRate,
        MetricId::BaseAmount,
        MetricId::QuoteAmount,
        MetricId::InverseRate,
        MetricId::EquityPricePerShare,
        MetricId::EquityShares,
        MetricId::EquityDividendYield,
        MetricId::EquityForwardPrice,
        MetricId::DirtyPrice,
        MetricId::CleanPrice,
        MetricId::Accrued,
        MetricId::AccruedInterest,
        MetricId::Ytm,
        MetricId::Ytw,
        MetricId::DurationMac,
        MetricId::DurationMod,
        MetricId::Convexity,
        MetricId::ZSpread,
        MetricId::Oas,
        MetricId::EmbeddedOptionValue,
        MetricId::ISpread,
        MetricId::DiscountMargin,
        MetricId::GSpread,
        MetricId::ASWSpread,
        MetricId::ASWPar,
        MetricId::ASWMarket,
        MetricId::ASWParFwd,
        MetricId::ASWMarketFwd,
        MetricId::Annuity,
        MetricId::ParRate,
        MetricId::Pv01,
        MetricId::PvFixed,
        MetricId::PvFloat,
        MetricId::Yf,
        MetricId::DfStart,
        MetricId::DfEnd,
        MetricId::DepositParRate,
        MetricId::DfEndFromQuote,
        MetricId::QuoteRate,
        MetricId::ParSpread,
        MetricId::RiskyPv01,
        MetricId::ProtectionLegPv,
        MetricId::PremiumLegPv,
        MetricId::JumpToDefault,
        MetricId::ExpectedLoss,
        MetricId::DefaultProbability,
        MetricId::Recovery01,
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::BucketedVega,
        MetricId::Rho,
        MetricId::ForeignRho,
        MetricId::ForwardPv01,
        MetricId::Vanna,
        MetricId::Volga,
        MetricId::Veta,
        MetricId::IrConvexity,
        MetricId::CsGamma,
        MetricId::InflationConvexity,
        MetricId::Charm,
        MetricId::Color,
        MetricId::Speed,
        MetricId::ImpliedVol,
        MetricId::VarianceVega,
        MetricId::ExpectedVariance,
        MetricId::RealizedVariance,
        MetricId::VarianceNotional,
        MetricId::VarianceStrikeVol,
        MetricId::VarianceTimeToMaturity,
        MetricId::Dividend01,
        MetricId::Inflation01,
        MetricId::Prepayment01,
        MetricId::Default01,
        MetricId::Severity01,
        MetricId::Conversion01,
        MetricId::CollateralHaircut01,
        MetricId::CollateralPrice01,
        MetricId::Nav01,
        MetricId::Carry01,
        MetricId::Hurdle01,
        MetricId::Dv01Domestic,
        MetricId::Dv01Foreign,
        MetricId::Fx01,
        MetricId::Npv01,
        MetricId::SpreadDv01,
        MetricId::Correlation01,
        MetricId::FxVega,
        MetricId::ConvexityAdjustmentRisk,
        MetricId::FinancingAnnuity,
        MetricId::IndexDelta,
        MetricId::PvPrimary,
        MetricId::PvReference,
        MetricId::AnnuityPrimary,
        MetricId::AnnuityReference,
        MetricId::Dv01Primary,
        MetricId::Dv01Reference,
        MetricId::BasisParSpread,
        MetricId::CollateralValue,
        MetricId::RequiredCollateral,
        MetricId::CollateralCoverage,
        MetricId::RepoInterest,
        MetricId::FundingRisk,
        MetricId::EffectiveRate,
        MetricId::TimeToMaturity,
        MetricId::ImpliedCollateralReturn,
        MetricId::Nav,
        MetricId::BasketValue,
        MetricId::ConstituentCount,
        MetricId::ExpenseRatio,
        MetricId::TrackingError,
        MetricId::Utilization,
        MetricId::PremiumDiscount,
        MetricId::WAL,
        MetricId::WAM,
        MetricId::ExpectedMaturity,
        MetricId::PoolFactor,
        MetricId::CPR,
        MetricId::SMM,
        MetricId::CDR,
        MetricId::LossSeverity,
        MetricId::SpreadDuration,
        MetricId::Dm01,
        MetricId::AbsDelinquency,
        MetricId::AbsChargeOff,
        MetricId::AbsExcessSpread,
        MetricId::AbsCreditEnhancement,
        MetricId::CloWarf,
        MetricId::CloWas,
        MetricId::CloWac,
        MetricId::CloDiversity,
        MetricId::CloOcRatio,
        MetricId::CloIcRatio,
        MetricId::CloRecoveryRate,
        MetricId::CmbsDscr,
        MetricId::CmbsWaltv,
        MetricId::CmbsCreditEnhancement,
        MetricId::RmbsPsaSpeed,
        MetricId::RmbsSdaSpeed,
        MetricId::RmbsWaltv,
        MetricId::RmbsWafico,
        MetricId::RealYield,
        MetricId::IndexRatio,
        MetricId::RealDuration,
        MetricId::BreakevenInflation,
        MetricId::LpIrr,
        MetricId::GpIrr,
        MetricId::MoicLp,
        MetricId::DpiLp,
        MetricId::TvpiLp,
        MetricId::CarryAccrued,
        MetricId::EnterpriseValue,
        MetricId::EquityValue,
        MetricId::TerminalValuePV,
        MetricId::ThetaGamma,
        MetricId::HVAR,
        MetricId::EXPECTED_SHORTFALL,
    ];
}

impl fmt::Display for MetricId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Lazy lookup table for FromStr
static METRIC_LOOKUP: OnceLock<HashMap<String, MetricId>> = OnceLock::new();

fn metric_lookup() -> &'static HashMap<String, MetricId> {
    METRIC_LOOKUP.get_or_init(|| {
        let mut map = HashMap::default();
        map.reserve(MetricId::ALL_STANDARD.len());
        for m in MetricId::ALL_STANDARD {
            // Names are already lower snake_case
            map.insert(m.as_str().to_string(), m.clone());
        }
        map
    })
}

impl FromStr for MetricId {
    type Err = (); // Never fails since we have a catch-all Custom variant

    /// Parses a string into a MetricId (permissive mode).
    ///
    /// This method never fails - any unrecognized string becomes a custom metric.
    /// Standard metrics are matched case-insensitively in snake_case format.
    ///
    /// **For user-provided inputs**, prefer `MetricId::parse_strict()` which
    /// rejects unknown metrics instead of silently creating custom metrics.
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_valuations::metrics::MetricId;
    /// use std::str::FromStr;
    ///
    /// // Known metric - parsed as standard
    /// let dv01 = MetricId::from_str("dv01").unwrap();
    /// assert_eq!(dv01, MetricId::Dv01);
    /// assert!(!dv01.is_custom());
    ///
    /// // Unknown metric - becomes custom (no error)
    /// let custom = MetricId::from_str("my_metric").unwrap();
    /// assert!(custom.is_custom());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        if let Some(id) = metric_lookup().get(&lower) {
            Ok(id.clone())
        } else {
            Ok(MetricId::custom(lower))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_strict_known_metric() {
        // Test lowercase
        let dv01 = MetricId::parse_strict("dv01").unwrap();
        assert_eq!(dv01, MetricId::Dv01);
        assert!(!dv01.is_custom());

        // Test uppercase (case insensitive)
        let theta = MetricId::parse_strict("THETA").unwrap();
        assert_eq!(theta, MetricId::Theta);
        assert!(!theta.is_custom());

        // Test mixed case
        let cs01 = MetricId::parse_strict("Cs01").unwrap();
        assert_eq!(cs01, MetricId::Cs01);
        assert!(!cs01.is_custom());

        // Test various standard metrics
        let delta = MetricId::parse_strict("delta").unwrap();
        assert_eq!(delta, MetricId::Delta);

        let ytm = MetricId::parse_strict("ytm").unwrap();
        assert_eq!(ytm, MetricId::Ytm);

        let convexity = MetricId::parse_strict("convexity").unwrap();
        assert_eq!(convexity, MetricId::Convexity);
    }

    #[test]
    fn test_parse_strict_unknown_metric() {
        // Unknown metric should fail
        let result = MetricId::parse_strict("dv01x");
        assert!(result.is_err());

        // Check error contains metric name
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(err_msg.to_lowercase().contains("dv01x"));

        // Test other typos
        assert!(MetricId::parse_strict("theta2").is_err());
        assert!(MetricId::parse_strict("cs_01").is_err());
        assert!(MetricId::parse_strict("unknown_metric").is_err());
    }

    #[test]
    fn test_parse_strict_error_includes_available_metrics() {
        let result = MetricId::parse_strict("invalid_metric");
        assert!(result.is_err());

        // The error should be UnknownMetric variant
        match result.unwrap_err() {
            finstack_core::Error::UnknownMetric {
                metric_id,
                available,
            } => {
                assert_eq!(metric_id, "invalid_metric");
                // Should include standard metrics
                assert!(!available.is_empty());
                assert!(available.contains(&"dv01".to_string()));
                assert!(available.contains(&"theta".to_string()));
                assert!(available.contains(&"cs01".to_string()));
            }
            _ => panic!("Expected UnknownMetric error"),
        }
    }

    #[test]
    fn test_from_str_still_permissive() {
        // FromStr should still accept unknown metrics
        let known = MetricId::from_str("dv01").unwrap();
        assert_eq!(known, MetricId::Dv01);
        assert!(!known.is_custom());

        // Unknown metric becomes custom (no error)
        let custom = MetricId::from_str("my_custom_metric").unwrap();
        assert!(custom.is_custom());
        assert_eq!(custom.as_str(), "my_custom_metric");

        // Another unknown metric
        let custom2 = MetricId::from_str("user_defined_123").unwrap();
        assert!(custom2.is_custom());
    }

    #[test]
    fn test_parse_strict_vs_from_str_behavior() {
        // Known metric: both work the same
        let strict = MetricId::parse_strict("theta").unwrap();
        let permissive = MetricId::from_str("theta").unwrap();
        assert_eq!(strict, permissive);

        // Unknown metric: strict fails, permissive creates custom
        let strict_result = MetricId::parse_strict("custom_metric");
        assert!(strict_result.is_err());

        let permissive_result = MetricId::from_str("custom_metric").unwrap();
        assert!(permissive_result.is_custom());
    }

    #[test]
    fn test_custom_metric_creation() {
        let custom = MetricId::custom("my_metric");
        assert!(custom.is_custom());
        assert_eq!(custom.as_str(), "my_metric");

        // Custom metrics not in ALL_STANDARD
        assert!(!MetricId::ALL_STANDARD.contains(&custom));
    }

    #[test]
    fn test_all_standard_metrics_parseable_strict() {
        // Every standard metric should be parseable via parse_strict
        for metric in MetricId::ALL_STANDARD {
            let parsed = MetricId::parse_strict(metric.as_str()).unwrap();
            assert_eq!(&parsed, metric);
            assert!(!parsed.is_custom());
        }
    }

    #[test]
    fn test_case_insensitivity() {
        // Strict parsing is case insensitive
        let lower = MetricId::parse_strict("dv01").unwrap();
        let upper = MetricId::parse_strict("DV01").unwrap();
        let mixed = MetricId::parse_strict("Dv01").unwrap();

        assert_eq!(lower, upper);
        assert_eq!(lower, mixed);
        assert_eq!(lower, MetricId::Dv01);
    }
}
