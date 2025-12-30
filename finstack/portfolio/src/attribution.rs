//! Portfolio-level P&L attribution.
//!
//! Aggregates instrument-level attribution across all positions in a portfolio,
//! with currency conversion to portfolio base currency.

use crate::error::{PortfolioError, Result};
use crate::portfolio::Portfolio;
use crate::position::PositionUnit;
use crate::types::PositionId;
use finstack_core::config::FinstackConfig;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::{fx::FxQuery, Money};
use finstack_valuations::attribution::{
    attribute_pnl_metrics_based, attribute_pnl_parallel, default_attribution_metrics,
    AttributionMethod, CorrelationsAttribution, CreditCurvesAttribution, FxAttribution,
    InflationCurvesAttribution, PnlAttribution, RatesCurvesAttribution, ScalarsAttribution,
    VolAttribution,
};
use finstack_valuations::metrics::MetricId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Portfolio-level P&L attribution result.
///
/// Aggregates P&L attribution across all positions with currency conversion
/// to portfolio base currency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioAttribution {
    /// Total portfolio P&L in base currency.
    pub total_pnl: Money,

    /// Carry P&L (theta + accruals).
    pub carry: Money,

    /// Interest rate curves P&L.
    pub rates_curves_pnl: Money,

    /// Credit hazard curves P&L.
    pub credit_curves_pnl: Money,

    /// Inflation curves P&L.
    pub inflation_curves_pnl: Money,

    /// Base correlation curves P&L.
    pub correlations_pnl: Money,

    /// FX rate changes P&L.
    pub fx_pnl: Money,

    /// FX translation P&L from instrument currency to portfolio base currency.
    pub fx_translation_pnl: Money,

    /// Implied volatility changes P&L.
    pub vol_pnl: Money,

    /// Model parameters P&L.
    pub model_params_pnl: Money,

    /// Market scalars P&L.
    pub market_scalars_pnl: Money,

    /// Residual P&L.
    pub residual: Money,

    /// Attribution by position.
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

/// Default set of metrics for metrics-based attribution at the portfolio level.
///
/// This mirrors the standard metrics used by the valuations crate for
/// metrics-based attribution and should stay in sync with
/// `default_attribution_metrics` in `finstack-valuations`.
fn default_metrics_for_metrics_based() -> Vec<MetricId> {
    default_attribution_metrics()
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
    let zero = Money::new(0.0, base_ccy);

    let mut portfolio_attr = PortfolioAttribution {
        total_pnl: zero,
        carry: zero,
        rates_curves_pnl: zero,
        credit_curves_pnl: zero,
        inflation_curves_pnl: zero,
        correlations_pnl: zero,
        fx_pnl: zero,
        fx_translation_pnl: zero,
        vol_pnl: zero,
        model_params_pnl: zero,
        market_scalars_pnl: zero,
        residual: zero,
        by_position: IndexMap::new(),
        rates_detail: None,
        credit_detail: None,
        inflation_detail: None,
        correlations_detail: None,
        fx_detail: None,
        vol_detail: None,
        scalars_detail: None,
    };

    // Attribute each position
    for position in &portfolio.positions {
        // Perform instrument-level attribution and get T0 value
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
                .map_err(|e| PortfolioError::ValuationError {
                    position_id: position.position_id.clone(),
                    message: format!("Attribution failed: {}", e),
                })?;

                // Get T0 value for FX revaluation (on principal)
                let val_t0 = position
                    .instrument
                    .value(market_t0, as_of_t0)
                    .map_err(|e| PortfolioError::ValuationError {
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
                .map_err(|e| PortfolioError::ValuationError {
                    position_id: position.position_id.clone(),
                    message: format!("Attribution failed: {}", e),
                })?;

                // Get T0 value for FX revaluation (on principal)
                let val_t0 = position
                    .instrument
                    .value(market_t0, as_of_t0)
                    .map_err(|e| PortfolioError::ValuationError {
                        position_id: position.position_id.clone(),
                        message: format!("Attribution T0 valuation failed: {}", e),
                    })?;

                (attr, val_t0)
            }

            AttributionMethod::MetricsBased => {
                // For metrics-based attribution, compute valuations with the
                // standard attribution metrics set and delegate to the
                // valuations crate's metrics-based engine.
                let metrics = default_metrics_for_metrics_based();

                let val_t0 = position
                    .instrument
                    .price_with_metrics(market_t0, as_of_t0, &metrics)
                    .map_err(|e: finstack_core::Error| PortfolioError::ValuationError {
                        position_id: position.position_id.clone(),
                        message: format!("Attribution T0 valuation failed: {}", e),
                    })?;

                let val_t1 = position
                    .instrument
                    .price_with_metrics(market_t1, as_of_t1, &metrics)
                    .map_err(|e: finstack_core::Error| PortfolioError::ValuationError {
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
                .map_err(|e| PortfolioError::ValuationError {
                    position_id: position.position_id.clone(),
                    message: format!("Attribution failed: {}", e),
                })?;

                (attr, val_t0.value)
            }
        };

        // Scale attribution and T0 value using unit-aware scaling
        // For attribution, we still use direct quantity scaling since PnlAttribution.scale()
        // expects a scalar multiplier. The unit-aware logic is applied to the T0 value.
        let scale_factor = match position.unit {
            PositionUnit::Percentage => {
                // Percentage values are always in points: 50 = 50%
                position.quantity / 100.0
            }
            _ => position.quantity,
        };
        pos_attr.scale(scale_factor);
        let val_t0_native = position.scale_value(val_t0_native_unit);

        // Convert each factor to base currency
        let convert = |money: Money| -> Result<Money> {
            if money.currency() == base_ccy {
                Ok(money)
            } else {
                let fx_matrix = market_t1.fx().ok_or_else(|| {
                    PortfolioError::MissingMarketData("FX matrix not available".to_string())
                })?;
                let query = FxQuery::new(money.currency(), base_ccy, as_of_t1);
                let rate_result =
                    fx_matrix
                        .rate(query)
                        .map_err(|_| PortfolioError::FxConversionFailed {
                            from: money.currency(),
                            to: base_ccy,
                        })?;
                Ok(Money::new(money.amount() * rate_result.rate, base_ccy))
            }
        };

        // Aggregate to portfolio level
        let total_pnl_base = convert(pos_attr.total_pnl)?;
        let carry_base = convert(pos_attr.carry)?;
        let rates_base = convert(pos_attr.rates_curves_pnl)?;
        let credit_base = convert(pos_attr.credit_curves_pnl)?;
        let inflation_base = convert(pos_attr.inflation_curves_pnl)?;
        let corr_base = convert(pos_attr.correlations_pnl)?;
        let fx_base = convert(pos_attr.fx_pnl)?;
        let vol_base = convert(pos_attr.vol_pnl)?;
        let model_base = convert(pos_attr.model_params_pnl)?;
        let scalars_base = convert(pos_attr.market_scalars_pnl)?;
        let residual_base = convert(pos_attr.residual)?;

        portfolio_attr.total_pnl = portfolio_attr
            .total_pnl
            .checked_add(total_pnl_base)
            .map_err(PortfolioError::Core)?;
        portfolio_attr.carry = portfolio_attr
            .carry
            .checked_add(carry_base)
            .map_err(PortfolioError::Core)?;
        portfolio_attr.rates_curves_pnl = portfolio_attr
            .rates_curves_pnl
            .checked_add(rates_base)
            .map_err(PortfolioError::Core)?;
        portfolio_attr.credit_curves_pnl = portfolio_attr
            .credit_curves_pnl
            .checked_add(credit_base)
            .map_err(PortfolioError::Core)?;
        portfolio_attr.inflation_curves_pnl = portfolio_attr
            .inflation_curves_pnl
            .checked_add(inflation_base)
            .map_err(PortfolioError::Core)?;
        portfolio_attr.correlations_pnl = portfolio_attr
            .correlations_pnl
            .checked_add(corr_base)
            .map_err(PortfolioError::Core)?;
        portfolio_attr.fx_pnl = portfolio_attr
            .fx_pnl
            .checked_add(fx_base)
            .map_err(PortfolioError::Core)?;
        portfolio_attr.vol_pnl = portfolio_attr
            .vol_pnl
            .checked_add(vol_base)
            .map_err(PortfolioError::Core)?;
        portfolio_attr.model_params_pnl = portfolio_attr
            .model_params_pnl
            .checked_add(model_base)
            .map_err(PortfolioError::Core)?;
        portfolio_attr.market_scalars_pnl = portfolio_attr
            .market_scalars_pnl
            .checked_add(scalars_base)
            .map_err(PortfolioError::Core)?;
        portfolio_attr.residual = portfolio_attr
            .residual
            .checked_add(residual_base)
            .map_err(PortfolioError::Core)?;

        // FX translation P&L: effect of translating instrument-currency P&L
        // into portfolio base currency as FX rates move from T₀ to T₁.
        if pos_attr.total_pnl.currency() != base_ccy {
            let inst_ccy = pos_attr.total_pnl.currency();

            let fx_t0 = market_t0.fx().ok_or_else(|| {
                PortfolioError::MissingMarketData("FX matrix at T0 not available".to_string())
            })?;
            let fx_t1 = market_t1.fx().ok_or_else(|| {
                PortfolioError::MissingMarketData("FX matrix at T1 not available".to_string())
            })?;

            let query_t0 = FxQuery::new(inst_ccy, base_ccy, as_of_t0);
            let rate_t0 = fx_t0
                .rate(query_t0)
                .map_err(|_| PortfolioError::FxConversionFailed {
                    from: inst_ccy,
                    to: base_ccy,
                })?;

            let query_t1 = FxQuery::new(inst_ccy, base_ccy, as_of_t1);
            let rate_t1 = fx_t1
                .rate(query_t1)
                .map_err(|_| PortfolioError::FxConversionFailed {
                    from: inst_ccy,
                    to: base_ccy,
                })?;

            // 1. Translation of P&L Flow: (Pnl_Native * R1) - (Pnl_Native * R0)
            let pnl_amount = pos_attr.total_pnl.amount();
            let base_t0 = Money::new(pnl_amount * rate_t0.rate, base_ccy);
            let base_t1 = Money::new(pnl_amount * rate_t1.rate, base_ccy);

            let translation_of_pnl = base_t1.checked_sub(base_t0).map_err(PortfolioError::Core)?;

            // 2. Revaluation of Opening Principal: Val_T0_Native * (R1 - R0)
            // This captures the FX risk on the principal amount held.
            let principal_amount = val_t0_native.amount();
            let principal_base_t0 = Money::new(principal_amount * rate_t0.rate, base_ccy);
            let principal_base_t1 = Money::new(principal_amount * rate_t1.rate, base_ccy);

            let translation_of_principal = principal_base_t1
                .checked_sub(principal_base_t0)
                .map_err(PortfolioError::Core)?;

            // Total FX Translation P&L
            let total_translation = translation_of_pnl
                .checked_add(translation_of_principal)
                .map_err(PortfolioError::Core)?;

            portfolio_attr.fx_translation_pnl = portfolio_attr
                .fx_translation_pnl
                .checked_add(total_translation)
                .map_err(PortfolioError::Core)?;

            // Add principal translation to total portfolio P&L
            // (Note: translation_of_pnl is already included because we added
            // total_pnl_base = Pnl_Native * R1 above)
            portfolio_attr.total_pnl = portfolio_attr
                .total_pnl
                .checked_add(translation_of_principal)
                .map_err(PortfolioError::Core)?;
        }

        // Store position-level attribution
        portfolio_attr
            .by_position
            .insert(position.position_id.clone(), pos_attr);
    }

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
    use finstack_core::currency::Currency;
    use finstack_valuations::attribution::default_attribution_metrics;

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
    fn test_default_metrics_alignment() {
        assert_eq!(
            default_metrics_for_metrics_based(),
            default_attribution_metrics()
        );
    }
}
