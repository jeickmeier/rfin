//! Equity Total Return Swap instrument definitions.
//!
//! This module provides the [`EquityTotalReturnSwap`] instrument for synthetic
//! equity index or single-stock exposure.

use crate::{
    cashflow::builder::ScheduleParams,
    cashflow::traits::{CashflowProvider, DatedFlows},
    instruments::common::parameters::{
        legs::FinancingLegSpec, trs_common::TrsScheduleSpec, trs_common::TrsSide,
        underlying::EquityUnderlyingParams,
    },
    instruments::Attributes,
    margin::types::OtcMarginSpec,
};
use finstack_core::{
    currency::Currency,
    dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor},
    market_data::context::MarketContext,
    money::Money,
    types::{CurveId, InstrumentId},
    Result,
};

/// Equity Total Return Swap instrument.
///
/// A TRS where the total return leg is based on an equity index or single stock.
/// The holder receives the total return (price appreciation + dividends) of the underlying
/// equity in exchange for paying a floating rate plus spread on the notional amount.
///
/// # Use Cases
///
/// - **Synthetic long exposure**: Gain equity index exposure without buying assets
/// - **Leverage**: Minimize upfront capital requirements
/// - **ETF replication**: Replicate equity ETF returns synthetically
/// - **Short exposure**: Easier than borrowing securities
///
/// # Example
///
/// ```
/// use finstack_valuations::instruments::equity_trs::EquityTotalReturnSwap;
///
/// let trs = EquityTotalReturnSwap::example();
/// // let pv = trs.npv(&market_context, as_of_date)?;
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct EquityTotalReturnSwap {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Notional amount for the swap.
    pub notional: Money,
    /// Underlying equity parameters (spot ID, dividend yield, contract size).
    pub underlying: EquityUnderlyingParams,
    /// Financing leg specification (curves, spread, day count).
    pub financing: FinancingLegSpec,
    /// Schedule specification (payment dates and frequency).
    pub schedule: TrsScheduleSpec,
    /// Trade side (receive/pay total return).
    pub side: TrsSide,
    /// Initial index level (if known, otherwise fetched from market).
    pub initial_level: Option<f64>,
    /// Optional OTC margin specification for VM/IM.
    ///
    /// Equity TRS use SIMM equity bucket for margin calculation.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub margin_spec: Option<OtcMarginSpec>,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

impl EquityTotalReturnSwap {
    /// Create a canonical example equity TRS for testing and documentation.
    ///
    /// Returns a 1-year SPX total return swap with quarterly resets.
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("TRS-SPX-1Y"))
            .notional(Money::new(5_000_000.0, Currency::USD))
            .underlying(EquityUnderlyingParams {
                ticker: "SPX".to_string(),
                spot_id: "SPX-SPOT".to_string(),
                div_yield_id: Some("SPX-DIV".to_string()),
                contract_size: 1.0,
                currency: Currency::USD,
            })
            .financing(FinancingLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                forward_curve_id: CurveId::new("USD-SOFR-3M"),
                spread_bp: 75.0,
                day_count: DayCount::Act360,
            })
            .schedule(TrsScheduleSpec::from_params(
                Date::from_calendar_date(2024, time::Month::January, 1)
                    .expect("Valid example date"),
                Date::from_calendar_date(2025, time::Month::January, 1)
                    .expect("Valid example date"),
                ScheduleParams {
                    freq: Tenor::quarterly(),
                    dc: DayCount::Act360,
                    bdc: BusinessDayConvention::Following,
                    calendar_id: None,
                    stub: StubKind::None,
                },
            ))
            .side(TrsSide::ReceiveTotalReturn)
            .initial_level_opt(None)
            .attributes(Attributes::new())
            .build()
            .expect("Example TRS construction should not fail")
    }

    /// Creates an equity TRS that replicates an ETF.
    ///
    /// This is a convenience constructor for creating TRS positions that synthetically
    /// replicate equity ETF exposure.
    ///
    /// # Arguments
    /// * `etf_ticker` — ETF ticker symbol (e.g., "SPY", "QQQ")
    /// * `spot_id` — Market data identifier for the ETF spot price
    /// * `notional` — Notional amount in the ETF's currency
    /// * `financing` — Financing leg specification
    /// * `schedule` — Payment schedule specification
    /// * `div_yield_id` — Optional dividend yield market data identifier
    ///
    /// # Example
    ///
    /// ```ignore
    /// let spy_trs = EquityTotalReturnSwap::replicate_etf(
    ///     "SPY",
    ///     "SPY-SPOT",
    ///     Money::new(10_000_000.0, Currency::USD),
    ///     financing_spec,
    ///     schedule_spec,
    ///     Some("SPY-DIV"),
    /// );
    /// ```
    pub fn replicate_etf(
        etf_ticker: &str,
        spot_id: &str,
        notional: Money,
        financing: FinancingLegSpec,
        schedule: TrsScheduleSpec,
        div_yield_id: Option<&str>,
    ) -> Self {
        let mut underlying = EquityUnderlyingParams::new(etf_ticker, spot_id, notional.currency());
        if let Some(div) = div_yield_id {
            underlying = underlying.with_dividend_yield(div);
        }

        Self::builder()
            .id(InstrumentId::new(format!("TRS-{}", etf_ticker)))
            .notional(notional)
            .underlying(underlying)
            .financing(financing)
            .schedule(schedule)
            .side(TrsSide::ReceiveTotalReturn)
            .initial_level_opt(None)
            .attributes(Attributes::new())
            .build()
            .expect("ETF replication TRS construction should not fail")
    }

    /// Calculates the net present value (NPV) of the equity TRS.
    ///
    /// # Arguments
    /// * `curves` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Net present value in the instrument's currency.
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        // Calculate total return leg PV
        let total_return_pv = self.pv_total_return_leg(curves, as_of)?;

        // Calculate financing leg PV
        let financing_pv = self.pv_financing_leg(curves, as_of)?;

        // Net PV depends on side
        let net_pv = match self.side {
            TrsSide::ReceiveTotalReturn => (total_return_pv - financing_pv)?,
            TrsSide::PayTotalReturn => (financing_pv - total_return_pv)?,
        };

        Ok(net_pv)
    }

    /// Calculates the present value of the total return leg.
    ///
    /// # Arguments
    /// * `curves` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Present value of the total return leg in the instrument's currency.
    pub fn pv_total_return_leg(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        crate::instruments::equity_trs::pricer::pv_total_return_leg(self, curves, as_of)
    }

    /// Calculates the present value of the financing leg.
    ///
    /// # Arguments
    /// * `curves` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Present value of the financing leg in the instrument's currency.
    pub fn pv_financing_leg(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        use crate::instruments::common::pricing::TrsEngine;
        TrsEngine::pv_financing_leg(&self.financing, &self.schedule, self.notional, curves, as_of)
    }

    /// Calculates the financing annuity for par spread calculation.
    ///
    /// # Arguments
    /// * `curves` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Financing annuity (sum of discounted year fractions × notional).
    pub fn financing_annuity(&self, curves: &MarketContext, as_of: Date) -> Result<f64> {
        use crate::instruments::common::pricing::TrsEngine;
        TrsEngine::financing_annuity(&self.financing, &self.schedule, self.notional, curves, as_of)
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl crate::instruments::common::traits::Instrument for EquityTotalReturnSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::EquityTotalReturnSwap
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
        )
    }

    fn as_cashflow_provider(&self) -> Option<&dyn CashflowProvider> {
        Some(self)
    }
}

impl CashflowProvider for EquityTotalReturnSwap {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn build_schedule(&self, _context: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        // For TRS, we return the expected payment dates
        // Actual amounts depend on realized returns
        let period_schedule = self.schedule.period_schedule()?;

        let mut flows = Vec::new();
        for date in period_schedule.dates.iter().skip(1) {
            // Add a placeholder flow for each payment date
            // In practice, the amount would be determined at fixing
            flows.push((*date, Money::new(0.0, self.notional.currency())));
        }

        Ok(flows)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for EquityTotalReturnSwap {
    fn discount_curve_id(&self) -> &CurveId {
        &self.financing.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for EquityTotalReturnSwap {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.financing.discount_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for EquityTotalReturnSwap {
    fn forward_curve_ids(&self) -> Vec<CurveId> {
        // TRS financing leg typically uses the same curve for projection
        vec![self.financing.discount_curve_id.clone()]
    }
}

