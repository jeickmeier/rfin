//! Portfolio-level P&L attribution.
//!
//! Aggregates instrument-level attribution across all positions in a portfolio,
//! with currency conversion to portfolio base currency.

use crate::error::{Error, Result};
use crate::portfolio::Portfolio;
use crate::position::PositionUnit;
use crate::types::PositionId;
use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::summation::neumaier_sum;
use finstack_core::money::{fx::FxQuery, Money};
use finstack_valuations::attribution::{
    attribute_pnl_metrics_based, attribute_pnl_parallel, default_attribution_metrics,
    AttributionMethod, CorrelationsAttribution, CreditCurvesAttribution, FxAttribution,
    InflationCurvesAttribution, PnlAttribution, RatesCurvesAttribution, ScalarsAttribution,
    VolAttribution,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Portfolio-level P&L attribution result.
///
/// Aggregates P&L attribution across all positions with currency conversion
/// to portfolio base currency.
///
/// # FX Translation Effects
///
/// For positions denominated in currencies other than the portfolio's base currency,
/// the attribution includes FX translation effects. The `total_pnl` field represents
/// the **all-in P&L** including both:
///
/// 1. **Instrument-level P&L** converted to base currency at T₁ FX rates
/// 2. **FX translation P&L** from the revaluation of opening principal
///
/// The decomposition is:
///
/// ```text
/// total_pnl = sum(factor_pnl) + fx_translation_pnl + residual
/// ```
///
/// Where `factor_pnl` includes carry, rates, credit, vol, etc. (each already
/// converted to base currency), and `fx_translation_pnl` captures:
///
/// - Translation of P&L flow: `PnL_native × (FX_T1 - FX_T0)`
/// - Revaluation of opening principal: `Val_T0_native × (FX_T1 - FX_T0)`
///
/// # Note on by_position Attribution
///
/// The `by_position` map contains instrument-currency attribution before FX
/// translation effects are applied. To reconcile with `total_pnl`, apply the
/// FX rates and add the principal revaluation effect.
///
/// # Conventions
///
/// The portfolio-level aggregates are reported in portfolio base currency,
/// while `by_position` remains in each instrument's native currency so callers
/// can inspect raw instrument attribution before FX translation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioAttribution {
    /// Total portfolio P&L in base currency.
    ///
    /// This is the **all-in P&L** that includes:
    /// - All factor attributions converted to base currency at T₁ rates
    /// - FX translation effects from opening principal revaluation
    ///
    /// Note: This differs from a simple sum of factor attributions because
    /// cross-currency positions include FX translation P&L on the principal.
    pub total_pnl: Money,

    /// Carry P&L (theta + accruals) in base currency.
    pub carry: Money,

    /// Interest rate curves P&L in base currency.
    pub rates_curves_pnl: Money,

    /// Credit hazard curves P&L in base currency.
    pub credit_curves_pnl: Money,

    /// Inflation curves P&L in base currency.
    pub inflation_curves_pnl: Money,

    /// Base correlation curves P&L in base currency.
    pub correlations_pnl: Money,

    /// FX rate changes P&L in base currency.
    ///
    /// This captures FX exposure within instruments (e.g., cross-currency swaps),
    /// not the translation effect from converting instrument P&L to base currency.
    pub fx_pnl: Money,

    /// FX translation P&L from converting instrument-currency values to base currency.
    ///
    /// For cross-currency positions, this includes:
    /// - Translation of P&L flow: effect of FX rate change on the reported P&L
    /// - Revaluation of opening principal: effect of FX change on T₀ position value
    ///
    /// This is separate from `fx_pnl` which captures FX exposure within instruments.
    pub fx_translation_pnl: Money,

    /// Implied volatility changes P&L in base currency.
    pub vol_pnl: Money,

    /// Model parameters P&L in base currency.
    pub model_params_pnl: Money,

    /// Market scalars P&L in base currency.
    pub market_scalars_pnl: Money,

    /// Residual P&L (unexplained) in base currency.
    pub residual: Money,

    /// Attribution by position in instrument-native currency.
    ///
    /// Note: These values are in each instrument's native currency and do not
    /// include FX translation effects. Use the portfolio-level aggregates for
    /// base-currency totals.
    pub by_position: IndexMap<PositionId, PnlAttribution>,

    /// Aggregate rates curves detail (optional).
    pub rates_detail: Option<RatesCurvesAttribution>,

    /// Aggregate credit curves detail (optional).
    pub credit_detail: Option<CreditCurvesAttribution>,

    /// Aggregate inflation curves detail (optional).
    pub inflation_detail: Option<InflationCurvesAttribution>,

    /// Aggregate correlations detail (optional).
    pub correlations_detail: Option<CorrelationsAttribution>,

    /// Aggregate FX detail (optional).
    pub fx_detail: Option<FxAttribution>,

    /// Aggregate volatility detail (optional).
    pub vol_detail: Option<VolAttribution>,

    /// Aggregate scalars detail (optional).
    pub scalars_detail: Option<ScalarsAttribution>,
}

struct PositionAttributionData {
    position_id: PositionId,
    pos_attr: PnlAttribution,
    val_t0_native: Money,
    inst_ccy: Currency,
}

fn attribute_single_position(
    position: &crate::position::Position,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    config: &FinstackConfig,
    method: &AttributionMethod,
) -> Result<PositionAttributionData> {
    let (mut pos_attr, val_t0_native_unit) = match method {
        AttributionMethod::Parallel => {
            let attr = attribute_pnl_parallel(
                &position.instrument,
                market_t0,
                market_t1,
                as_of_t0,
                as_of_t1,
                config,
                None,
            )
            .map_err(|e| Error::ValuationError {
                position_id: position.position_id.clone(),
                message: format!("Attribution failed: {}", e),
            })?;

            let val_t0 = position
                .instrument
                .value(market_t0, as_of_t0)
                .map_err(|e| Error::ValuationError {
                    position_id: position.position_id.clone(),
                    message: format!("Attribution T0 valuation failed: {}", e),
                })?;

            (attr, val_t0)
        }

        AttributionMethod::Waterfall(ref order) => {
            let attr = finstack_valuations::attribution::attribute_pnl_waterfall(
                &position.instrument,
                market_t0,
                market_t1,
                as_of_t0,
                as_of_t1,
                config,
                order.clone(),
                false,
                None,
            )
            .map_err(|e| Error::ValuationError {
                position_id: position.position_id.clone(),
                message: format!("Attribution failed: {}", e),
            })?;

            let val_t0 = position
                .instrument
                .value(market_t0, as_of_t0)
                .map_err(|e| Error::ValuationError {
                    position_id: position.position_id.clone(),
                    message: format!("Attribution T0 valuation failed: {}", e),
                })?;

            (attr, val_t0)
        }

        AttributionMethod::MetricsBased => {
            let metrics = default_attribution_metrics();

            let val_t0 = position
                .instrument
                .price_with_metrics(
                    market_t0,
                    as_of_t0,
                    &metrics,
                    finstack_valuations::instruments::PricingOptions::default(),
                )
                .map_err(|e: finstack_core::Error| Error::ValuationError {
                    position_id: position.position_id.clone(),
                    message: format!("Attribution T0 valuation failed: {}", e),
                })?;

            let val_t1 = position
                .instrument
                .price_with_metrics(
                    market_t1,
                    as_of_t1,
                    &metrics,
                    finstack_valuations::instruments::PricingOptions::default(),
                )
                .map_err(|e: finstack_core::Error| Error::ValuationError {
                    position_id: position.position_id.clone(),
                    message: format!("Attribution T1 valuation failed: {}", e),
                })?;

            let attr = attribute_pnl_metrics_based(
                &position.instrument,
                market_t0,
                market_t1,
                &val_t0,
                &val_t1,
                as_of_t0,
                as_of_t1,
            )
            .map_err(|e| Error::ValuationError {
                position_id: position.position_id.clone(),
                message: format!("Attribution failed: {}", e),
            })?;

            (attr, val_t0.value)
        }

        AttributionMethod::Taylor(ref taylor_config) => {
            let attr = finstack_valuations::attribution::attribute_pnl_taylor_compat(
                &position.instrument,
                market_t0,
                market_t1,
                as_of_t0,
                as_of_t1,
                taylor_config,
            )
            .map_err(|e| Error::ValuationError {
                position_id: position.position_id.clone(),
                message: format!("Taylor attribution failed: {}", e),
            })?;

            let val_t0 = position
                .instrument
                .value(market_t0, as_of_t0)
                .map_err(|e| Error::ValuationError {
                    position_id: position.position_id.clone(),
                    message: format!("Attribution T0 valuation failed: {}", e),
                })?;

            (attr, val_t0)
        }
    };

    let scale_factor = match position.unit {
        PositionUnit::Percentage => position.quantity / 100.0,
        _ => position.quantity,
    };
    pos_attr.scale(scale_factor);
    let val_t0_native = position.scale_value(val_t0_native_unit);
    let inst_ccy = pos_attr.total_pnl.currency();

    Ok(PositionAttributionData {
        position_id: position.position_id.clone(),
        pos_attr,
        val_t0_native,
        inst_ccy,
    })
}

/// Perform P&L attribution for an entire portfolio.
///
/// Attributes each position's P&L and aggregates to portfolio base currency.
/// Each position is attributed using the specified method (Parallel, Waterfall,
/// or MetricsBased), and the results are converted to the portfolio's base
/// currency with explicit FX translation P&L tracking.
///
/// # Arguments
///
/// * `portfolio` - Portfolio to attribute
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `as_of_t0` - Valuation date at T₀ (typically yesterday for day-over-day)
/// * `as_of_t1` - Valuation date at T₁ (typically today for day-over-day)
/// * `config` - Finstack configuration
/// * `method` - Attribution methodology (Parallel, Waterfall, or MetricsBased)
///
/// # Returns
///
/// Portfolio-level attribution with per-position breakdown.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_portfolio::attribute_portfolio_pnl;
/// use finstack_valuations::attribution::AttributionMethod;
/// use finstack_core::config::FinstackConfig;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_portfolio::Portfolio;
/// use time::macros::date;
///
/// # fn main() -> finstack_portfolio::Result<()> {
/// let as_of_t0 = date!(2025-11-20);  // Yesterday
/// let as_of_t1 = date!(2025-11-21);  // Today
///
/// # let portfolio: Portfolio = unimplemented!("Provide your portfolio");
/// # let market_t0: MarketContext = unimplemented!("Provide market at t0");
/// # let market_t1: MarketContext = unimplemented!("Provide market at t1");
/// # let config: FinstackConfig = unimplemented!("Provide finstack config");
/// let attribution = attribute_portfolio_pnl(
///     &portfolio,
///     &market_t0,
///     &market_t1,
///     as_of_t0,
///     as_of_t1,
///     &config,
///     AttributionMethod::Parallel,
/// )?;
///
/// println!("Portfolio P&L: {}", attribution.total_pnl);
/// println!("Total Carry: {}", attribution.carry);
/// println!("FX Translation: {}", attribution.fx_translation_pnl);
///
/// // Drill down to specific position
/// if let Some(pos_attr) = attribution.by_position.get("POS_001") {
///     println!("Position POS_001 P&L: {}", pos_attr.total_pnl);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # References
///
/// - Parametric risk-reporting background:
///   `docs/REFERENCES.md#jpmorgan1996RiskMetrics`
pub fn attribute_portfolio_pnl(
    portfolio: &Portfolio,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    config: &FinstackConfig,
    method: AttributionMethod,
) -> Result<PortfolioAttribution> {
    let base_ccy = portfolio.base_ccy;

    #[cfg(feature = "parallel")]
    let position_data: Vec<PositionAttributionData> = {
        use rayon::prelude::*;
        portfolio
            .positions
            .par_iter()
            .map(|position| {
                attribute_single_position(
                    position, market_t0, market_t1, as_of_t0, as_of_t1, config, &method,
                )
            })
            .collect::<Result<Vec<_>>>()?
    };

    #[cfg(not(feature = "parallel"))]
    let position_data: Vec<PositionAttributionData> = portfolio
        .positions
        .iter()
        .map(|position| {
            attribute_single_position(
                position, market_t0, market_t1, as_of_t0, as_of_t1, config, &method,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let n = position_data.len();
    let mut total_pnl_vals: Vec<f64> = Vec::with_capacity(n);
    let mut carry_vals: Vec<f64> = Vec::with_capacity(n);
    let mut rates_vals: Vec<f64> = Vec::with_capacity(n);
    let mut credit_vals: Vec<f64> = Vec::with_capacity(n);
    let mut inflation_vals: Vec<f64> = Vec::with_capacity(n);
    let mut corr_vals: Vec<f64> = Vec::with_capacity(n);
    let mut fx_vals: Vec<f64> = Vec::with_capacity(n);
    let mut vol_vals: Vec<f64> = Vec::with_capacity(n);
    let mut model_vals: Vec<f64> = Vec::with_capacity(n);
    let mut scalars_vals: Vec<f64> = Vec::with_capacity(n);
    let mut residual_vals: Vec<f64> = Vec::with_capacity(n);
    let mut fx_translation_vals: Vec<f64> = Vec::new();

    let mut by_position: IndexMap<PositionId, PnlAttribution> = IndexMap::new();

    for data in position_data {
        let PositionAttributionData {
            position_id,
            pos_attr,
            val_t0_native,
            inst_ccy,
        } = data;

        let convert = |money: Money| -> Result<Money> {
            if money.currency() == base_ccy {
                Ok(money)
            } else {
                let fx_matrix = market_t1.fx().ok_or_else(|| {
                    Error::MissingMarketData("FX matrix not available".to_string())
                })?;
                let query = FxQuery::new(money.currency(), base_ccy, as_of_t1);
                let rate_result = fx_matrix
                    .rate(query)
                    .map_err(|_| Error::FxConversionFailed {
                        from: money.currency(),
                        to: base_ccy,
                    })?;
                Ok(Money::new(money.amount() * rate_result.rate, base_ccy))
            }
        };

        total_pnl_vals.push(convert(pos_attr.total_pnl)?.amount());
        carry_vals.push(convert(pos_attr.carry)?.amount());
        rates_vals.push(convert(pos_attr.rates_curves_pnl)?.amount());
        credit_vals.push(convert(pos_attr.credit_curves_pnl)?.amount());
        inflation_vals.push(convert(pos_attr.inflation_curves_pnl)?.amount());
        corr_vals.push(convert(pos_attr.correlations_pnl)?.amount());
        fx_vals.push(convert(pos_attr.fx_pnl)?.amount());
        vol_vals.push(convert(pos_attr.vol_pnl)?.amount());
        model_vals.push(convert(pos_attr.model_params_pnl)?.amount());
        scalars_vals.push(convert(pos_attr.market_scalars_pnl)?.amount());
        residual_vals.push(convert(pos_attr.residual)?.amount());

        if inst_ccy != base_ccy {
            let fx_t0 = market_t0.fx().ok_or_else(|| {
                Error::MissingMarketData("FX matrix at T0 not available".to_string())
            })?;
            let fx_t1 = market_t1.fx().ok_or_else(|| {
                Error::MissingMarketData("FX matrix at T1 not available".to_string())
            })?;

            let query_t0 = FxQuery::new(inst_ccy, base_ccy, as_of_t0);
            let rate_t0 = fx_t0
                .rate(query_t0)
                .map_err(|_| Error::FxConversionFailed {
                    from: inst_ccy,
                    to: base_ccy,
                })?;

            let query_t1 = FxQuery::new(inst_ccy, base_ccy, as_of_t1);
            let rate_t1 = fx_t1
                .rate(query_t1)
                .map_err(|_| Error::FxConversionFailed {
                    from: inst_ccy,
                    to: base_ccy,
                })?;

            let principal_amount = val_t0_native.amount();
            let principal_translation = principal_amount * (rate_t1.rate - rate_t0.rate);

            let total_translation = principal_translation;
            fx_translation_vals.push(total_translation);

            total_pnl_vals.push(total_translation);
        }

        by_position.insert(position_id, pos_attr);
    }

    let portfolio_attr = PortfolioAttribution {
        total_pnl: Money::new(neumaier_sum(total_pnl_vals), base_ccy),
        carry: Money::new(neumaier_sum(carry_vals), base_ccy),
        rates_curves_pnl: Money::new(neumaier_sum(rates_vals), base_ccy),
        credit_curves_pnl: Money::new(neumaier_sum(credit_vals), base_ccy),
        inflation_curves_pnl: Money::new(neumaier_sum(inflation_vals), base_ccy),
        correlations_pnl: Money::new(neumaier_sum(corr_vals), base_ccy),
        fx_pnl: Money::new(neumaier_sum(fx_vals), base_ccy),
        fx_translation_pnl: Money::new(neumaier_sum(fx_translation_vals), base_ccy),
        vol_pnl: Money::new(neumaier_sum(vol_vals), base_ccy),
        model_params_pnl: Money::new(neumaier_sum(model_vals), base_ccy),
        market_scalars_pnl: Money::new(neumaier_sum(scalars_vals), base_ccy),
        residual: Money::new(neumaier_sum(residual_vals), base_ccy),
        by_position,
        rates_detail: None,
        credit_detail: None,
        inflation_detail: None,
        correlations_detail: None,
        fx_detail: None,
        vol_detail: None,
        scalars_detail: None,
    };

    Ok(portfolio_attr)
}

impl PortfolioAttribution {
    /// Export portfolio attribution as CSV string.
    ///
    /// Returns summary row with total attribution by factor.
    pub fn to_csv(&self) -> String {
        let mut lines = Vec::new();

        // Header
        lines.push(
            "total,carry,rates_curves,credit_curves,inflation_curves,\
             correlations,fx,fx_translation,vol,model_params,market_scalars,residual"
                .to_string(),
        );

        // Data row
        lines.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            self.total_pnl.amount(),
            self.carry.amount(),
            self.rates_curves_pnl.amount(),
            self.credit_curves_pnl.amount(),
            self.inflation_curves_pnl.amount(),
            self.correlations_pnl.amount(),
            self.fx_pnl.amount(),
            self.fx_translation_pnl.amount(),
            self.vol_pnl.amount(),
            self.model_params_pnl.amount(),
            self.market_scalars_pnl.amount(),
            self.residual.amount(),
        ));

        lines.join("\n")
    }

    /// Export position-by-position detail as CSV string.
    ///
    /// Returns one row per position with full factor breakdown.
    pub fn position_detail_to_csv(&self) -> String {
        let mut lines = Vec::new();

        // Header
        lines.push(
            "position_id,total,carry,rates_curves,credit_curves,\
             inflation_curves,correlations,fx,vol,model_params,\
             market_scalars,residual"
                .to_string(),
        );

        // Data rows (one per position)
        for (position_id, pos_attr) in &self.by_position {
            lines.push(format!(
                "{},{},{},{},{},{},{},{},{},{},{},{}",
                position_id,
                pos_attr.total_pnl.amount(),
                pos_attr.carry.amount(),
                pos_attr.rates_curves_pnl.amount(),
                pos_attr.credit_curves_pnl.amount(),
                pos_attr.inflation_curves_pnl.amount(),
                pos_attr.correlations_pnl.amount(),
                pos_attr.fx_pnl.amount(),
                pos_attr.vol_pnl.amount(),
                pos_attr.model_params_pnl.amount(),
                pos_attr.market_scalars_pnl.amount(),
                pos_attr.residual.amount(),
            ));
        }

        lines.join("\n")
    }

    /// Generate explanation tree for portfolio attribution.
    pub fn explain(&self) -> String {
        let mut lines = Vec::new();

        let fmt = |amount: &Money, total: &Money| -> String {
            let pct = if total.amount().abs() > 1e-10 {
                (amount.amount() / total.amount()) * 100.0
            } else {
                0.0
            };
            format!("{} ({:.1}%)", amount, pct)
        };

        lines.push(format!("Portfolio P&L: {}", self.total_pnl));
        lines.push(format!("  ├─ Carry: {}", fmt(&self.carry, &self.total_pnl)));
        lines.push(format!(
            "  ├─ Rates Curves: {}",
            fmt(&self.rates_curves_pnl, &self.total_pnl)
        ));
        lines.push(format!(
            "  ├─ Credit Curves: {}",
            fmt(&self.credit_curves_pnl, &self.total_pnl)
        ));
        lines.push(format!(
            "  ├─ Inflation: {}",
            fmt(&self.inflation_curves_pnl, &self.total_pnl)
        ));
        lines.push(format!(
            "  ├─ Correlations: {}",
            fmt(&self.correlations_pnl, &self.total_pnl)
        ));
        lines.push(format!("  ├─ FX: {}", fmt(&self.fx_pnl, &self.total_pnl)));
        lines.push(format!(
            "  ├─ FX Translation: {}",
            fmt(&self.fx_translation_pnl, &self.total_pnl)
        ));
        lines.push(format!("  ├─ Vol: {}", fmt(&self.vol_pnl, &self.total_pnl)));
        lines.push(format!(
            "  ├─ Model Params: {}",
            fmt(&self.model_params_pnl, &self.total_pnl)
        ));
        lines.push(format!(
            "  ├─ Market Scalars: {}",
            fmt(&self.market_scalars_pnl, &self.total_pnl)
        ));
        lines.push(format!(
            "  └─ Residual: {}",
            fmt(&self.residual, &self.total_pnl)
        ));

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    fn sample_position_attr(
        position_id: &str,
        total: f64,
        carry: f64,
        residual: f64,
    ) -> PnlAttribution {
        let mut attr = PnlAttribution::new(
            Money::new(total, Currency::USD),
            position_id,
            date!(2026 - 01 - 02),
            date!(2026 - 01 - 03),
            AttributionMethod::Parallel,
        );
        attr.carry = Money::new(carry, Currency::USD);
        attr.rates_curves_pnl = Money::new(total - carry - residual, Currency::USD);
        attr.residual = Money::new(residual, Currency::USD);
        attr
    }

    #[test]
    fn test_portfolio_attribution_structure() {
        let base_ccy = Currency::USD;
        let zero = Money::new(0.0, base_ccy);

        let portfolio_attr = PortfolioAttribution {
            total_pnl: Money::new(1000.0, base_ccy),
            carry: Money::new(100.0, base_ccy),
            rates_curves_pnl: Money::new(500.0, base_ccy),
            credit_curves_pnl: zero,
            inflation_curves_pnl: zero,
            correlations_pnl: zero,
            fx_pnl: zero,
            fx_translation_pnl: zero,
            vol_pnl: zero,
            model_params_pnl: zero,
            market_scalars_pnl: zero,
            residual: Money::new(400.0, base_ccy),
            by_position: IndexMap::new(),
            rates_detail: None,
            credit_detail: None,
            inflation_detail: None,
            correlations_detail: None,
            fx_detail: None,
            vol_detail: None,
            scalars_detail: None,
        };

        let csv = portfolio_attr.to_csv();
        assert!(csv.contains("total"));
        assert!(csv.contains("1000"));
    }

    #[test]
    fn test_default_metrics_nonempty() {
        assert!(!default_attribution_metrics().is_empty());
    }

    #[test]
    fn test_position_detail_to_csv_includes_each_position_breakdown() {
        let mut by_position = IndexMap::new();
        by_position.insert(
            PositionId::from("POS_A"),
            sample_position_attr("POS_A", 120.0, 10.0, 5.0),
        );
        by_position.insert(
            PositionId::from("POS_B"),
            sample_position_attr("POS_B", -20.0, -2.0, 1.0),
        );

        let zero = Money::new(0.0, Currency::USD);
        let portfolio_attr = PortfolioAttribution {
            total_pnl: Money::new(100.0, Currency::USD),
            carry: Money::new(8.0, Currency::USD),
            rates_curves_pnl: Money::new(87.0, Currency::USD),
            credit_curves_pnl: zero,
            inflation_curves_pnl: zero,
            correlations_pnl: zero,
            fx_pnl: zero,
            fx_translation_pnl: zero,
            vol_pnl: zero,
            model_params_pnl: zero,
            market_scalars_pnl: zero,
            residual: Money::new(5.0, Currency::USD),
            by_position,
            rates_detail: None,
            credit_detail: None,
            inflation_detail: None,
            correlations_detail: None,
            fx_detail: None,
            vol_detail: None,
            scalars_detail: None,
        };

        let csv = portfolio_attr.position_detail_to_csv();
        assert!(csv.contains("position_id,total,carry"));
        assert!(csv.contains("POS_A,120"));
        assert!(csv.contains("POS_B,-20"));
    }

    #[test]
    fn test_explain_formats_percentages_and_zero_total_safely() {
        let zero = Money::new(0.0, Currency::USD);
        let explained = PortfolioAttribution {
            total_pnl: Money::new(200.0, Currency::USD),
            carry: Money::new(20.0, Currency::USD),
            rates_curves_pnl: Money::new(100.0, Currency::USD),
            credit_curves_pnl: Money::new(10.0, Currency::USD),
            inflation_curves_pnl: Money::new(5.0, Currency::USD),
            correlations_pnl: Money::new(15.0, Currency::USD),
            fx_pnl: Money::new(25.0, Currency::USD),
            fx_translation_pnl: Money::new(10.0, Currency::USD),
            vol_pnl: Money::new(5.0, Currency::USD),
            model_params_pnl: Money::new(5.0, Currency::USD),
            market_scalars_pnl: Money::new(3.0, Currency::USD),
            residual: Money::new(2.0, Currency::USD),
            by_position: IndexMap::new(),
            rates_detail: None,
            credit_detail: None,
            inflation_detail: None,
            correlations_detail: None,
            fx_detail: None,
            vol_detail: None,
            scalars_detail: None,
        };
        let rendered = explained.explain();
        assert!(rendered.contains("Portfolio P&L: USD 200.00"));
        assert!(rendered.contains("Carry: USD 20.00 (10.0%)"));
        assert!(rendered.contains("FX Translation: USD 10.00 (5.0%)"));
        assert!(rendered.contains("Residual: USD 2.00 (1.0%)"));

        let zero_total = PortfolioAttribution {
            total_pnl: zero,
            carry: Money::new(5.0, Currency::USD),
            rates_curves_pnl: zero,
            credit_curves_pnl: zero,
            inflation_curves_pnl: zero,
            correlations_pnl: zero,
            fx_pnl: zero,
            fx_translation_pnl: zero,
            vol_pnl: zero,
            model_params_pnl: zero,
            market_scalars_pnl: zero,
            residual: Money::new(-5.0, Currency::USD),
            by_position: IndexMap::new(),
            rates_detail: None,
            credit_detail: None,
            inflation_detail: None,
            correlations_detail: None,
            fx_detail: None,
            vol_detail: None,
            scalars_detail: None,
        };
        let zero_rendered = zero_total.explain();
        assert!(zero_rendered.contains("Carry: USD 5.00 (0.0%)"));
        assert!(zero_rendered.contains("Residual: USD -5.00 (0.0%)"));
    }
}
