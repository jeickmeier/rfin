//! Equity Index Future types and implementation.
//!
//! Defines the `EquityIndexFuture` instrument for equity index futures such as
//! E-mini S&P 500 (ES), E-mini Nasdaq-100 (NQ), Euro Stoxx 50 (FESX), DAX (FDAX),
//! FTSE 100 (Z), and Nikkei 225 (NK).
//!
//! # Pricing
//!
//! Two pricing modes are supported:
//!
//! 1. **Mark-to-Market** (when `quoted_price` is provided):
//!    ```text
//!    NPV = (quoted_price - entry_price) × multiplier × quantity × position_sign
//!    ```
//!
//! 2. **Fair Value** (cost-of-carry model):
//!    ```text
//!    F = S₀ × exp((r - q) × T)
//!    NPV = (F - entry_price) × multiplier × quantity × position_sign
//!    ```
//!
//! where:
//! - S₀ = Current spot index level
//! - r = Risk-free rate (from discount curve)
//! - q = Continuous dividend yield
//! - T = Time to expiry in years
//!
//! # References
//!
//! - Hull, J. C. (2018). "Options, Futures, and Other Derivatives." Chapter 5.
//! - CME Group. "E-mini S&P 500 Futures Contract Specifications."

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::traits::Attributes;
use crate::instruments::ir_future::Position;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use time::macros::date;

/// Contract specifications for equity index futures.
///
/// Contains exchange-specific contract parameters such as multiplier,
/// tick size, and settlement method.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::equity_index_future::EquityFutureSpecs;
///
/// // E-mini S&P 500 specifications
/// let es_specs = EquityFutureSpecs::sp500_emini();
/// assert_eq!(es_specs.multiplier, 50.0);
/// assert_eq!(es_specs.tick_size, 0.25);
/// assert_eq!(es_specs.tick_value, 12.50);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EquityFutureSpecs {
    /// Contract multiplier (USD per index point).
    /// E-mini S&P 500: 50.0 (each point = $50)
    pub multiplier: f64,
    /// Tick size in index points.
    /// E-mini S&P 500: 0.25 points
    pub tick_size: f64,
    /// Tick value in currency units.
    /// E-mini S&P 500: $12.50 per tick (0.25 × 50)
    pub tick_value: f64,
    /// Settlement method description.
    pub settlement_method: String,
}

impl Default for EquityFutureSpecs {
    fn default() -> Self {
        Self::sp500_emini()
    }
}

impl EquityFutureSpecs {
    /// Create specs for E-mini S&P 500 futures (ES).
    ///
    /// CME contract specifications:
    /// - Multiplier: $50 per index point
    /// - Tick size: 0.25 index points
    /// - Tick value: $12.50 per tick
    /// - Settlement: Cash settled to Special Opening Quotation (SOQ)
    pub fn sp500_emini() -> Self {
        Self {
            multiplier: 50.0,
            tick_size: 0.25,
            tick_value: 12.50,
            settlement_method: "Cash settled to Special Opening Quotation".to_string(),
        }
    }

    /// Create specs for E-mini Nasdaq-100 futures (NQ).
    ///
    /// CME contract specifications:
    /// - Multiplier: $20 per index point
    /// - Tick size: 0.25 index points
    /// - Tick value: $5.00 per tick
    /// - Settlement: Cash settled to Special Opening Quotation (SOQ)
    pub fn nasdaq100_emini() -> Self {
        Self {
            multiplier: 20.0,
            tick_size: 0.25,
            tick_value: 5.00,
            settlement_method: "Cash settled to Special Opening Quotation".to_string(),
        }
    }

    /// Create specs for Micro E-mini S&P 500 futures (MES).
    ///
    /// CME contract specifications:
    /// - Multiplier: $5 per index point (1/10 of E-mini)
    /// - Tick size: 0.25 index points
    /// - Tick value: $1.25 per tick
    pub fn sp500_micro_emini() -> Self {
        Self {
            multiplier: 5.0,
            tick_size: 0.25,
            tick_value: 1.25,
            settlement_method: "Cash settled to Special Opening Quotation".to_string(),
        }
    }

    /// Create specs for Euro Stoxx 50 futures (FESX).
    ///
    /// Eurex contract specifications:
    /// - Multiplier: €10 per index point
    /// - Tick size: 1.0 index point
    /// - Tick value: €10.00 per tick
    pub fn euro_stoxx_50() -> Self {
        Self {
            multiplier: 10.0,
            tick_size: 1.0,
            tick_value: 10.0,
            settlement_method: "Cash settled to final settlement price".to_string(),
        }
    }

    /// Create specs for DAX futures (FDAX).
    ///
    /// Eurex contract specifications:
    /// - Multiplier: €25 per index point
    /// - Tick size: 0.5 index points
    /// - Tick value: €12.50 per tick
    pub fn dax() -> Self {
        Self {
            multiplier: 25.0,
            tick_size: 0.5,
            tick_value: 12.5,
            settlement_method: "Cash settled to final settlement price".to_string(),
        }
    }

    /// Create specs for FTSE 100 futures (Z).
    ///
    /// ICE contract specifications:
    /// - Multiplier: £10 per index point
    /// - Tick size: 0.5 index points
    /// - Tick value: £5.00 per tick
    pub fn ftse_100() -> Self {
        Self {
            multiplier: 10.0,
            tick_size: 0.5,
            tick_value: 5.0,
            settlement_method: "Cash settled to Exchange Delivery Settlement Price".to_string(),
        }
    }

    /// Create specs for Nikkei 225 futures (NK).
    ///
    /// CME/OSE contract specifications:
    /// - Multiplier: ¥500 per index point (CME dollar-denominated uses $5)
    /// - Tick size: 5.0 index points
    /// - Tick value: ¥2,500 per tick
    pub fn nikkei_225() -> Self {
        Self {
            multiplier: 500.0,
            tick_size: 5.0,
            tick_value: 2500.0,
            settlement_method: "Cash settled to Special Quotation".to_string(),
        }
    }
}

/// Equity Index Future instrument.
///
/// Represents a futures contract on an equity index such as S&P 500, Nasdaq-100,
/// Euro Stoxx 50, DAX, FTSE 100, or Nikkei 225.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::equity_index_future::{
///     EquityIndexFuture, EquityFutureSpecs,
/// };
/// use finstack_valuations::instruments::ir_future::Position;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let future = EquityIndexFuture::builder()
///     .id(InstrumentId::new("ES-2025M03"))
///     .index_ticker("SPX".to_string())
///     .currency(Currency::USD)
///     .quantity(10.0)
///     .expiry_date(Date::from_calendar_date(2025, Month::March, 21).unwrap())
///     .last_trading_date(Date::from_calendar_date(2025, Month::March, 20).unwrap())
///     .position(Position::Long)
///     .contract_specs(EquityFutureSpecs::sp500_emini())
///     .discount_curve_id(CurveId::new("USD-OIS"))
///     .index_price_id("SPX-SPOT".to_string())
///     .build()
///     .expect("Valid future");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct EquityIndexFuture {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Index ticker symbol (e.g., "SPX", "NDX", "SX5E").
    pub index_ticker: String,
    /// Settlement currency.
    pub currency: Currency,
    /// Number of contracts (positive for long exposure).
    pub quantity: f64,
    /// Future expiry/settlement date.
    pub expiry_date: Date,
    /// Last trading date (typically one day before expiry).
    pub last_trading_date: Date,
    /// Entry price at trade inception (optional for new trades).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub entry_price: Option<f64>,
    /// Current quoted market price (if available, overrides fair value).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub quoted_price: Option<f64>,
    /// Position side (Long or Short).
    pub position: Position,
    /// Contract specifications.
    #[builder(default)]
    pub contract_specs: EquityFutureSpecs,
    /// Discount curve identifier for present value calculations.
    pub discount_curve_id: CurveId,
    /// Index spot price identifier for fair value calculation.
    pub index_price_id: String,
    /// Optional dividend yield identifier for fair value calculation.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub dividend_yield_id: Option<String>,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

impl EquityIndexFuture {
    /// Create a canonical example E-mini S&P 500 future for testing and documentation.
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("ES-2025M03"))
            .index_ticker("SPX".to_string())
            .currency(Currency::USD)
            .quantity(10.0)
            .expiry_date(date!(2025 - 03 - 21))
            .last_trading_date(date!(2025 - 03 - 20))
            .entry_price_opt(Some(4500.0))
            .quoted_price_opt(Some(4550.0))
            .position(Position::Long)
            .contract_specs(EquityFutureSpecs::sp500_emini())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .index_price_id("SPX-SPOT".to_string())
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example EquityIndexFuture with valid constants should never fail")
            })
    }

    /// Create an E-mini S&P 500 future with common defaults.
    ///
    /// # Arguments
    ///
    /// * `id` - Instrument identifier (e.g., "ESH5" for March 2025)
    /// * `quantity` - Number of contracts
    /// * `expiry_date` - Contract expiry date
    /// * `last_trading_date` - Last trading date
    /// * `entry_price` - Entry price (None for new trades)
    /// * `position` - Long or Short
    /// * `discount_curve_id` - Discount curve for PV calculations
    pub fn sp500_emini(
        id: impl Into<InstrumentId>,
        quantity: f64,
        expiry_date: Date,
        last_trading_date: Date,
        entry_price: Option<f64>,
        position: Position,
        discount_curve_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        Self::builder()
            .id(id.into())
            .index_ticker("SPX".to_string())
            .currency(Currency::USD)
            .quantity(quantity)
            .expiry_date(expiry_date)
            .last_trading_date(last_trading_date)
            .entry_price_opt(entry_price)
            .position(position)
            .contract_specs(EquityFutureSpecs::sp500_emini())
            .discount_curve_id(discount_curve_id.into())
            .index_price_id("SPX-SPOT".to_string())
            .attributes(Attributes::new())
            .build()
    }

    /// Create an E-mini Nasdaq-100 future with common defaults.
    ///
    /// # Arguments
    ///
    /// * `id` - Instrument identifier (e.g., "NQH5" for March 2025)
    /// * `quantity` - Number of contracts
    /// * `expiry_date` - Contract expiry date
    /// * `last_trading_date` - Last trading date
    /// * `entry_price` - Entry price (None for new trades)
    /// * `position` - Long or Short
    /// * `discount_curve_id` - Discount curve for PV calculations
    pub fn nasdaq100_emini(
        id: impl Into<InstrumentId>,
        quantity: f64,
        expiry_date: Date,
        last_trading_date: Date,
        entry_price: Option<f64>,
        position: Position,
        discount_curve_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        Self::builder()
            .id(id.into())
            .index_ticker("NDX".to_string())
            .currency(Currency::USD)
            .quantity(quantity)
            .expiry_date(expiry_date)
            .last_trading_date(last_trading_date)
            .entry_price_opt(entry_price)
            .position(position)
            .contract_specs(EquityFutureSpecs::nasdaq100_emini())
            .discount_curve_id(discount_curve_id.into())
            .index_price_id("NDX-SPOT".to_string())
            .attributes(Attributes::new())
            .build()
    }

    /// Get the position sign (+1 for Long, -1 for Short).
    pub fn position_sign(&self) -> f64 {
        match self.position {
            Position::Long => 1.0,
            Position::Short => -1.0,
        }
    }

    /// Calculate the notional value of the position.
    ///
    /// # Formula
    /// ```text
    /// notional = price × multiplier × quantity
    /// ```
    pub fn notional_value(&self, price: f64) -> f64 {
        price * self.contract_specs.multiplier * self.quantity
    }

    /// Calculate delta exposure (index point sensitivity).
    ///
    /// # Formula
    /// ```text
    /// delta = multiplier × quantity × position_sign
    /// ```
    ///
    /// This represents the USD P&L change for a 1-point move in the index.
    pub fn delta(&self) -> f64 {
        self.contract_specs.multiplier * self.quantity * self.position_sign()
    }

    /// Calculate the present value of this equity index future.
    ///
    /// Uses mark-to-market if `quoted_price` is available, otherwise
    /// calculates fair value using the cost-of-carry model.
    ///
    /// # Arguments
    ///
    /// * `context` - Market context with curves and prices
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// Present value as Money in the instrument's currency.
    pub fn npv(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        let pv = self.npv_raw(context, as_of)?;
        Ok(Money::new(pv, self.currency))
    }

    /// Calculate the raw present value as f64.
    pub fn npv_raw(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        // If expired, value is zero
        if self.expiry_date < as_of {
            return Ok(0.0);
        }

        // Use quoted price if available (mark-to-market)
        if let Some(quoted) = self.quoted_price {
            return self.price_quoted(quoted);
        }

        // Otherwise calculate fair value
        self.price_fair_value(context, as_of)
    }

    /// Price using quoted market price (mark-to-market).
    fn price_quoted(&self, quoted_price: f64) -> finstack_core::Result<f64> {
        let entry = self.entry_price.unwrap_or(0.0);
        let price_diff = quoted_price - entry;
        let pv = price_diff * self.contract_specs.multiplier * self.quantity * self.position_sign();
        Ok(pv)
    }

    /// Price using fair value (cost-of-carry model).
    fn price_fair_value(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        use finstack_core::dates::{DayCount, DayCountCtx};
        use finstack_core::market_data::scalars::MarketScalar;

        // Get spot level
        let spot = match context.price(&self.index_price_id)? {
            MarketScalar::Unitless(v) => *v,
            MarketScalar::Price(m) => m.amount(),
        };

        // Get discount curve and calculate time to expiry
        let disc = context.get_discount(&self.discount_curve_id)?;
        let t = DayCount::Act365F
            .year_fraction(as_of, self.expiry_date, DayCountCtx::default())?
            .max(0.0);

        // Get risk-free rate from discount curve
        let r = disc.zero(t);

        // Get dividend yield (default to 0 if not provided)
        let q = if let Some(ref div_id) = self.dividend_yield_id {
            match context.price(div_id) {
                Ok(MarketScalar::Unitless(v)) => *v,
                Ok(MarketScalar::Price(m)) => m.amount(),
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        // Calculate fair forward price: F = S₀ × exp((r - q) × T)
        let fair_value = spot * ((r - q) * t).exp();

        // Calculate PV relative to entry price
        let entry = self.entry_price.unwrap_or(0.0);
        let price_diff = fair_value - entry;
        let pv = price_diff * self.contract_specs.multiplier * self.quantity * self.position_sign();

        Ok(pv)
    }

    /// Get the fair forward price using cost-of-carry model.
    ///
    /// # Formula
    /// ```text
    /// F = S₀ × exp((r - q) × T)
    /// ```
    pub fn fair_forward(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        use finstack_core::dates::{DayCount, DayCountCtx};
        use finstack_core::market_data::scalars::MarketScalar;

        // Get spot level
        let spot = match context.price(&self.index_price_id)? {
            MarketScalar::Unitless(v) => *v,
            MarketScalar::Price(m) => m.amount(),
        };

        // Get discount curve and calculate time to expiry
        let disc = context.get_discount(&self.discount_curve_id)?;
        let t = DayCount::Act365F
            .year_fraction(as_of, self.expiry_date, DayCountCtx::default())?
            .max(0.0);

        // Get risk-free rate
        let r = disc.zero(t);

        // Get dividend yield
        let q = if let Some(ref div_id) = self.dividend_yield_id {
            match context.price(div_id) {
                Ok(MarketScalar::Unitless(v)) => *v,
                Ok(MarketScalar::Price(m)) => m.amount(),
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        // F = S₀ × exp((r - q) × T)
        Ok(spot * ((r - q) * t).exp())
    }
}

// =============================================================================
// Trait Implementations
// =============================================================================

impl crate::instruments::common::traits::Instrument for EquityIndexFuture {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::EquityIndexFuture
    }

    fn as_any(&self) -> &dyn std::any::Any {
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

    fn value(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        self.npv(curves, as_of)
    }

    fn value_raw(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        self.npv_raw(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn as_cashflow_provider(&self) -> Option<&dyn CashflowProvider> {
        Some(self)
    }

    fn spot_id(&self) -> Option<&str> {
        Some(&self.index_price_id)
    }
}

impl CashflowProvider for EquityIndexFuture {
    fn notional(&self) -> Option<Money> {
        // Notional based on entry price or quoted price
        let price = self.quoted_price.or(self.entry_price).unwrap_or(0.0);
        Some(Money::new(self.notional_value(price), self.currency))
    }

    fn build_full_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        // Futures are daily settled (mark-to-market). There are no residual flows.
        Ok(crate::cashflow::traits::schedule_from_dated_flows(
            Vec::new(),
            self.notional(),
        ))
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for EquityIndexFuture {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for EquityIndexFuture {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_equity_future_specs_sp500_emini() {
        let specs = EquityFutureSpecs::sp500_emini();
        assert_eq!(specs.multiplier, 50.0);
        assert_eq!(specs.tick_size, 0.25);
        assert_eq!(specs.tick_value, 12.50);
    }

    #[test]
    fn test_equity_future_specs_nasdaq100_emini() {
        let specs = EquityFutureSpecs::nasdaq100_emini();
        assert_eq!(specs.multiplier, 20.0);
        assert_eq!(specs.tick_size, 0.25);
        assert_eq!(specs.tick_value, 5.00);
    }

    #[test]
    fn test_equity_index_future_construction() {
        let future = EquityIndexFuture::builder()
            .id(InstrumentId::new("ES-TEST"))
            .index_ticker("SPX".to_string())
            .currency(Currency::USD)
            .quantity(10.0)
            .expiry_date(Date::from_calendar_date(2025, Month::March, 21).expect("valid test date"))
            .last_trading_date(
                Date::from_calendar_date(2025, Month::March, 20).expect("valid test date"),
            )
            .position(Position::Long)
            .contract_specs(EquityFutureSpecs::sp500_emini())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .index_price_id("SPX-SPOT".to_string())
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        assert_eq!(future.id.as_str(), "ES-TEST");
        assert_eq!(future.index_ticker, "SPX");
        assert_eq!(future.quantity, 10.0);
        assert_eq!(future.currency, Currency::USD);
    }

    #[test]
    fn test_equity_index_future_example() {
        let future = EquityIndexFuture::example();
        assert_eq!(future.id.as_str(), "ES-2025M03");
        assert_eq!(future.index_ticker, "SPX");
        assert_eq!(future.quantity, 10.0);
        assert_eq!(future.contract_specs.multiplier, 50.0);
    }

    #[test]
    fn test_sp500_emini_constructor() {
        let expiry = Date::from_calendar_date(2025, Month::March, 21).expect("valid test date");
        let last_trade = Date::from_calendar_date(2025, Month::March, 20).expect("valid test date");

        let future = EquityIndexFuture::sp500_emini(
            "ESH5",
            10.0,
            expiry,
            last_trade,
            Some(4500.0),
            Position::Long,
            "USD-OIS",
        )
        .expect("should build");

        assert_eq!(future.id.as_str(), "ESH5");
        assert_eq!(future.index_ticker, "SPX");
        assert_eq!(future.contract_specs.multiplier, 50.0);
        assert_eq!(future.entry_price, Some(4500.0));
    }

    #[test]
    fn test_nasdaq100_emini_constructor() {
        let expiry = Date::from_calendar_date(2025, Month::March, 21).expect("valid test date");
        let last_trade = Date::from_calendar_date(2025, Month::March, 20).expect("valid test date");

        let future = EquityIndexFuture::nasdaq100_emini(
            "NQH5",
            5.0,
            expiry,
            last_trade,
            Some(15000.0),
            Position::Short,
            "USD-OIS",
        )
        .expect("should build");

        assert_eq!(future.id.as_str(), "NQH5");
        assert_eq!(future.index_ticker, "NDX");
        assert_eq!(future.contract_specs.multiplier, 20.0);
        assert_eq!(future.position, Position::Short);
    }

    #[test]
    fn test_position_sign() {
        let mut future = EquityIndexFuture::example();
        assert_eq!(future.position_sign(), 1.0);

        future.position = Position::Short;
        assert_eq!(future.position_sign(), -1.0);
    }

    #[test]
    fn test_delta_calculation() {
        let future = EquityIndexFuture::example();
        // Long 10 ES contracts: delta = 50 × 10 × 1 = 500
        assert_eq!(future.delta(), 500.0);

        let mut short_future = future.clone();
        short_future.position = Position::Short;
        // Short 10 ES contracts: delta = 50 × 10 × (-1) = -500
        assert_eq!(short_future.delta(), -500.0);
    }

    #[test]
    fn test_notional_value() {
        let future = EquityIndexFuture::example();
        // At price 4500: notional = 4500 × 50 × 10 = 2,250,000
        assert_eq!(future.notional_value(4500.0), 2_250_000.0);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_round_trip() {
        let future = EquityIndexFuture::example();
        let json = serde_json::to_string(&future).expect("serialize");
        let recovered: EquityIndexFuture = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(future.id, recovered.id);
        assert_eq!(future.index_ticker, recovered.index_ticker);
        assert_eq!(future.quantity, recovered.quantity);
    }
}
