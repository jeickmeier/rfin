//! Equity Total Return Swap instrument definitions.
//!
//! This module provides the [`EquityTotalReturnSwap`] instrument for synthetic
//! equity index or single-stock exposure.

use crate::impl_instrument_base;
use crate::{
    cashflow::builder::ScheduleParams,
    cashflow::traits::CashflowProvider,
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
use rust_decimal::Decimal;
use time::macros::date;

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
/// use finstack_valuations::instruments::equity::equity_trs::EquityTotalReturnSwap;
///
/// let trs = EquityTotalReturnSwap::example();
/// // let pv = trs.value(&market_context, as_of_date)?;
/// ```
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_spec: Option<OtcMarginSpec>,
    /// Dividend withholding tax rate for net return calculation.
    ///
    /// Specifies the fraction of dividends withheld for tax (e.g., 0.15 for 15% withholding).
    /// When set to 0.0 (default), the TRS passes through 100% of dividends (gross return).
    /// When set to a positive value, the dividend return component is reduced:
    /// ```text
    /// net_dividend_return = gross_dividend_return × (1 - dividend_tax_rate)
    /// ```
    ///
    /// # Market Context
    ///
    /// Withholding tax varies by jurisdiction and investor domicile:
    /// - US qualified dividends: typically 0% for domestic investors
    /// - US non-qualified: up to 30% for foreign investors (varies by treaty)
    /// - European: varies by country (15-30% typical)
    #[serde(default)]
    #[builder(default)]
    pub dividend_tax_rate: f64,
    /// Optional discrete cash dividends `(ex_date, amount)` for the underlying.
    ///
    /// When non-empty, pricing uses explicit period dividend pass-through and does
    /// not add continuous-yield dividend return to avoid double counting.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub discrete_dividends: Vec<(Date, f64)>,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

impl EquityTotalReturnSwap {
    /// Create a canonical example equity TRS for testing and documentation.
    ///
    /// Returns a 1-year SPX total return swap with quarterly resets.
    pub fn example() -> Self {
        // SAFETY: All inputs are compile-time validated constants
        Self::builder()
            .id(InstrumentId::new("TRS-SPX-1Y"))
            .notional(Money::new(5_000_000.0, Currency::USD))
            .underlying(EquityUnderlyingParams {
                ticker: "SPX".to_string(),
                spot_id: "SPX-SPOT".to_string(),
                div_yield_id: Some(CurveId::new("SPX-DIV")),
                contract_size: 1.0,
                currency: Currency::USD,
            })
            .financing(FinancingLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                forward_curve_id: CurveId::new("USD-SOFR-3M"),
                spread_bp: Decimal::from(75),
                day_count: DayCount::Act360,
            })
            .schedule(TrsScheduleSpec::from_params(
                date!(2024 - 01 - 01),
                date!(2025 - 01 - 01),
                ScheduleParams {
                    freq: Tenor::quarterly(),
                    dc: DayCount::Act360,
                    bdc: BusinessDayConvention::Following,
                    calendar_id: "weekends_only".to_string(),
                    stub: StubKind::None,
                    end_of_month: false,
                    payment_lag_days: 0,
                },
            ))
            .side(TrsSide::ReceiveTotalReturn)
            .initial_level_opt(None)
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| unreachable!("Example TRS with valid constants should never fail"))
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
    /// ```text
    /// let spy_trs = EquityTotalReturnSwap::replicate_etf(
    ///     "SPY",
    ///     "SPY-SPOT",
    ///     Money::new(10_000_000.0, Currency::USD),
    ///     financing_spec,
    ///     schedule_spec,
    ///     Some("SPY-DIV"),
    /// )?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    pub fn replicate_etf(
        etf_ticker: &str,
        spot_id: &str,
        notional: Money,
        financing: FinancingLegSpec,
        schedule: TrsScheduleSpec,
        div_yield_id: Option<&str>,
    ) -> Result<Self> {
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
    }

    /// Validates the equity TRS configuration.
    ///
    /// Checks for common configuration errors:
    /// - Dividend tax rate set without dividend yield ID
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    fn validate(&self) -> Result<()> {
        // Warn if dividend_tax_rate is set but no dividend yield is provided
        if self.dividend_tax_rate > 0.0
            && self.underlying.div_yield_id.is_none()
            && self.discrete_dividends.is_empty()
        {
            return Err(finstack_core::Error::Validation(format!(
                "EquityTRS '{}' has dividend_tax_rate={:.2}% but no div_yield_id is set. \
                 Set underlying.div_yield_id to enable dividend return calculation, \
                 provide discrete_dividends, or set dividend_tax_rate to 0.0 if dividends are not applicable.",
                self.id.as_str(),
                self.dividend_tax_rate * 100.0
            )));
        }
        Ok(())
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
        crate::instruments::equity::equity_trs::pricer::pv_total_return_leg(self, curves, as_of)
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
        use crate::instruments::common_impl::pricing::TrsEngine;
        TrsEngine::pv_financing_leg(
            &self.financing,
            &self.schedule,
            self.notional,
            curves,
            as_of,
        )
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
        use crate::instruments::common_impl::pricing::TrsEngine;
        TrsEngine::financing_annuity(
            &self.financing,
            &self.schedule,
            self.notional,
            curves,
            as_of,
        )
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl crate::instruments::common_impl::traits::Instrument for EquityTotalReturnSwap {
    impl_instrument_base!(crate::pricer::InstrumentType::EquityTotalReturnSwap);

    fn value(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        // Validate configuration
        self.validate()?;

        // Calculate total return leg PV
        let total_return_pv = self.pv_total_return_leg(curves, as_of)?;

        // Calculate financing leg PV
        let financing_pv = self.pv_financing_leg(curves, as_of)?;

        // Net PV depends on side
        let net_pv = match self.side {
            TrsSide::ReceiveTotalReturn => total_return_pv.checked_sub(financing_pv)?,
            TrsSide::PayTotalReturn => financing_pv.checked_sub(total_return_pv)?,
        };

        Ok(net_pv)
    }

    fn as_cashflow_provider(&self) -> Option<&dyn CashflowProvider> {
        Some(self)
    }

    fn as_marginable(&self) -> Option<&dyn crate::margin::traits::Marginable> {
        Some(self)
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.schedule.end)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.schedule.start)
    }
}

impl CashflowProvider for EquityTotalReturnSwap {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn build_full_schedule(
        &self,
        _context: &MarketContext,
        _as_of: Date,
    ) -> Result<crate::cashflow::builder::CashFlowSchedule> {
        // For TRS, we return the expected payment dates
        // Actual amounts depend on realized returns
        let period_schedule = self.schedule.period_schedule()?;

        let mut flows = Vec::new();
        for date in period_schedule.dates.iter().skip(1) {
            // Add a placeholder flow for each payment date
            // In practice, the amount would be determined at fixing
            flows.push((*date, Money::new(0.0, self.notional.currency())));
        }

        Ok(crate::cashflow::traits::schedule_from_dated_flows(
            flows,
            self.notional(),
            self.financing.day_count,
        ))
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for EquityTotalReturnSwap {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.financing.discount_curve_id.clone())
            .forward(self.financing.forward_curve_id.clone())
            .build()
    }
}
