//! Volatility Index Future types and implementation.
//!
//! Defines the `VolatilityIndexFuture` instrument for VIX, VXN, VSTOXX, and
//! similar volatility index futures. These contracts allow market participants
//! to gain exposure to expected future volatility levels.
//!
//! # Contract Specifications
//!
//! VIX futures are traded on CBOE with the following standard specs:
//! - Multiplier: $1,000 per index point
//! - Tick size: 0.05 index points ($50 per tick)
//! - Settlement: Cash-settled to SOQ (Special Opening Quotation)
//!
//! # Pricing
//!
//! The present value of a volatility index future is:
//! ```text
//! NPV = (Quoted_Price - Forward_Vol) × Multiplier × Contracts × Position_Sign
//! ```
//! where:
//! - Quoted_Price = Market price of the future
//! - Forward_Vol = Interpolated forward volatility from vol index curve
//! - Multiplier = Contract multiplier (typically 1000 for VIX)
//! - Position_Sign = +1 for long, -1 for short
//!
//! Unlike interest rate futures, VIX futures do not require convexity
//! adjustments because the underlying is already a volatility measure.
//!
//! # References
//!
//! - CBOE (2019). "VIX Futures Contract Specifications."
//! - Whaley, R. E. (2009). "Understanding the VIX." *Journal of Portfolio Management*.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::traits::Attributes;
use crate::instruments::ir_future::Position;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use time::macros::date;

/// Volatility Index Future instrument.
///
/// Represents a futures contract on a volatility index such as VIX, VXN,
/// or VSTOXX. These contracts provide exposure to expected future volatility.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::vol_index_future::{
///     VolatilityIndexFuture, VolIndexContractSpecs,
/// };
/// use finstack_valuations::instruments::ir_future::Position;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use finstack_core::money::Money;
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let future = VolatilityIndexFuture::builder()
///     .id(InstrumentId::new("VIX-FUT-2025M03"))
///     .notional(Money::new(100_000.0, Currency::USD))
///     .expiry_date(Date::from_calendar_date(2025, Month::March, 19).unwrap())
///     .settlement_date(Date::from_calendar_date(2025, Month::March, 19).unwrap())
///     .quoted_price(21.50)
///     .position(Position::Long)
///     .contract_specs(VolIndexContractSpecs::default())
///     .discount_curve_id(CurveId::new("USD-OIS"))
///     .vol_index_curve_id(CurveId::new("VIX"))
///     .build()
///     .expect("Valid future");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct VolatilityIndexFuture {
    /// Unique identifier.
    pub id: InstrumentId,
    /// Notional exposure in currency units. PV is scaled by
    /// `notional.amount() / (multiplier × quoted_price)` to represent
    /// the number of contracts.
    pub notional: Money,
    /// Future expiry date (typically 30 days before VIX settlement).
    pub expiry_date: Date,
    /// Settlement date (SOQ calculation date).
    pub settlement_date: Date,
    /// Quoted future price (index points, e.g., 21.50).
    pub quoted_price: f64,
    /// Position side (Long or Short).
    pub position: Position,
    /// Contract specifications.
    #[builder(default)]
    pub contract_specs: VolIndexContractSpecs,
    /// Discount curve identifier for present value calculations.
    pub discount_curve_id: CurveId,
    /// Volatility index forward curve identifier.
    pub vol_index_curve_id: CurveId,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

/// Contract specifications for volatility index futures.
///
/// VIX futures have standardized specifications set by CBOE:
/// - Standard multiplier: $1,000 per index point
/// - Minimum tick: 0.05 index points ($50)
/// - Weekly and monthly expiries available
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VolIndexContractSpecs {
    /// Contract multiplier (USD per index point).
    /// VIX standard: 1000 (each point = $1,000)
    pub multiplier: f64,
    /// Tick size in index points.
    /// VIX standard: 0.05 points
    pub tick_size: f64,
    /// Tick value in currency units.
    /// VIX standard: $50 per tick (0.05 × 1000)
    pub tick_value: f64,
    /// Index identifier (e.g., "VIX", "VXN", "VSTOXX").
    pub index_id: String,
}

impl Default for VolIndexContractSpecs {
    fn default() -> Self {
        Self {
            multiplier: 1000.0,
            tick_size: 0.05,
            tick_value: 50.0,
            index_id: "VIX".to_string(),
        }
    }
}

impl VolIndexContractSpecs {
    /// Create specs for standard VIX futures.
    pub fn vix() -> Self {
        Self::default()
    }

    /// Create specs for Mini VIX futures.
    pub fn mini_vix() -> Self {
        Self {
            multiplier: 100.0,
            tick_size: 0.05,
            tick_value: 5.0,
            index_id: "VIX".to_string(),
        }
    }

    /// Create specs for VSTOXX futures.
    pub fn vstoxx() -> Self {
        Self {
            multiplier: 100.0,
            tick_size: 0.05,
            tick_value: 5.0,
            index_id: "VSTOXX".to_string(),
        }
    }
}

impl VolatilityIndexFuture {
    /// Create a canonical example VIX future for testing and documentation.
    pub fn example() -> Self {
        // SAFETY: All inputs are compile-time validated constants
        Self::builder()
            .id(InstrumentId::new("VIX-FUT-2025M03"))
            .notional(Money::new(100_000.0, Currency::USD))
            .expiry_date(date!(2025 - 03 - 19))
            .settlement_date(date!(2025 - 03 - 19))
            .quoted_price(21.50)
            .position(Position::Long)
            .contract_specs(VolIndexContractSpecs::vix())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example VIX future with valid constants should never fail")
            })
    }

    /// Calculate the number of contracts based on notional and quoted price.
    ///
    /// # Formula
    /// ```text
    /// contracts = notional / (multiplier × quoted_price)
    /// ```
    pub fn num_contracts(&self) -> f64 {
        let contract_value = self.contract_specs.multiplier * self.quoted_price;
        if contract_value > 0.0 {
            self.notional.amount() / contract_value
        } else {
            0.0
        }
    }

    /// Calculate the present value of this volatility index future.
    ///
    /// # Formula
    /// ```text
    /// NPV = (Quoted_Price - Forward_Vol) × Multiplier × Contracts × Position_Sign
    /// ```
    ///
    /// Note: Unlike equity or commodity futures, VIX futures are not discounted
    /// because the underlying (VIX) is already a measure of forward-looking
    /// volatility. The mark-to-market is essentially the difference between
    /// the quoted price and the fair forward level, scaled by contract terms.
    ///
    /// # Arguments
    ///
    /// * `context` - Market context with vol index curves
    ///
    /// # Returns
    ///
    /// Present value as Money in the notional currency.
    pub fn npv(&self, context: &MarketContext) -> finstack_core::Result<Money> {
        let pv = self.npv_raw(context)?;
        Ok(Money::new(pv, self.notional.currency()))
    }

    /// Calculate the raw present value as f64.
    pub fn npv_raw(&self, context: &MarketContext) -> finstack_core::Result<f64> {
        // Get the vol index curve
        let vol_curve = context.get_vol_index(&self.vol_index_curve_id)?;

        // Calculate time to settlement using the curve's day count
        let t = vol_curve
            .day_count()
            .year_fraction(
                vol_curve.base_date(),
                self.settlement_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);

        // Get forward volatility level at settlement
        let forward_vol = vol_curve.forward_level(t);

        // Position sign
        let sign = match self.position {
            Position::Long => 1.0,
            Position::Short => -1.0,
        };

        // Calculate number of contracts
        let contracts = self.num_contracts();

        // NPV = (Quoted - Forward) × Multiplier × Contracts × Sign
        // Long benefits when quoted > forward (we bought cheap)
        // Short benefits when quoted < forward (we sold expensive)
        let pv_per_contract = (self.quoted_price - forward_vol) * self.contract_specs.multiplier;
        let pv_total = sign * contracts * pv_per_contract;

        Ok(pv_total)
    }

    /// Get the forward volatility level at settlement.
    pub fn forward_vol(&self, context: &MarketContext) -> finstack_core::Result<f64> {
        let vol_curve = context.get_vol_index(&self.vol_index_curve_id)?;
        let t = vol_curve
            .day_count()
            .year_fraction(
                vol_curve.base_date(),
                self.settlement_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        Ok(vol_curve.forward_level(t))
    }

    /// Calculate DV01 (delta with respect to vol index level).
    ///
    /// Returns the P&L change for a 1-point increase in the vol index level.
    pub fn delta_vol(&self) -> f64 {
        let sign = match self.position {
            Position::Long => 1.0,
            Position::Short => -1.0,
        };
        // Delta = -Multiplier × Contracts × Sign
        // (negative because NPV increases when forward_vol decreases for long)
        -sign * self.num_contracts() * self.contract_specs.multiplier
    }
}

// =============================================================================
// Trait Implementations
// =============================================================================

impl crate::instruments::common::traits::Instrument for VolatilityIndexFuture {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::VolatilityIndexFuture
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

    fn value(&self, curves: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        self.npv(curves)
    }

    fn value_raw(&self, curves: &MarketContext, _as_of: Date) -> finstack_core::Result<f64> {
        self.npv_raw(curves)
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
        )
    }

    fn as_cashflow_provider(&self) -> Option<&dyn CashflowProvider> {
        Some(self)
    }
}

impl CashflowProvider for VolatilityIndexFuture {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>> {
        // Futures are daily settled (mark-to-market).
        // There is no future cashflow to discount.
        Ok(vec![])
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for VolatilityIndexFuture {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for VolatilityIndexFuture {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        // Only include discount curve for DV01 calculations
        // Vol index curve sensitivity is handled separately via delta_vol
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
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::market_data::term_structures::VolatilityIndexCurve;
    use time::Month;

    fn setup_market() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        // Create discount curve
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96)])
            .build()
            .expect("valid discount curve");

        // Create VIX forward curve - contango structure
        let vix = VolatilityIndexCurve::builder("VIX")
            .base_date(base_date)
            .spot_level(18.0)
            .knots([(0.0, 18.0), (0.25, 20.0), (0.5, 21.0), (1.0, 22.0)])
            .build()
            .expect("valid VIX curve");

        MarketContext::new()
            .insert_discount(disc)
            .insert_vol_index(vix)
    }

    #[test]
    fn test_at_market_future() {
        let market = setup_market();

        // Create a future at the forward price (should have zero NPV)
        let future = VolatilityIndexFuture::builder()
            .id(InstrumentId::new("VIX-ATM"))
            .notional(Money::new(20_000.0, Currency::USD)) // 1 contract at 20
            .expiry_date(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .settlement_date(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .quoted_price(20.0) // At the 3M forward level
            .position(Position::Long)
            .contract_specs(VolIndexContractSpecs::vix())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .build()
            .expect("valid future");

        let npv = future.npv(&market).expect("npv calculation");
        // At forward price, NPV should be approximately zero
        assert!(
            npv.amount().abs() < 100.0,
            "At-market future should have near-zero NPV, got {}",
            npv.amount()
        );
    }

    #[test]
    fn test_long_position_benefits_from_high_quote() {
        let market = setup_market();

        // Long position with quoted price above forward
        let future = VolatilityIndexFuture::builder()
            .id(InstrumentId::new("VIX-LONG"))
            .notional(Money::new(22_000.0, Currency::USD)) // ~1 contract
            .expiry_date(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .settlement_date(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .quoted_price(22.0) // Above the ~20 forward level
            .position(Position::Long)
            .contract_specs(VolIndexContractSpecs::vix())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .build()
            .expect("valid future");

        let npv = future.npv(&market).expect("npv calculation");
        // Long at 22, forward at ~20, so positive PV (we locked in high price to sell)
        assert!(
            npv.amount() > 0.0,
            "Long future above forward should have positive NPV"
        );
    }

    #[test]
    fn test_short_position_benefits_from_low_forward() {
        let market = setup_market();

        // Short position with quoted price above forward
        let future = VolatilityIndexFuture::builder()
            .id(InstrumentId::new("VIX-SHORT"))
            .notional(Money::new(22_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .settlement_date(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .quoted_price(22.0)
            .position(Position::Short)
            .contract_specs(VolIndexContractSpecs::vix())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .build()
            .expect("valid future");

        let npv = future.npv(&market).expect("npv calculation");
        // Short at 22, forward at ~20, so negative PV (we sold cheap)
        assert!(
            npv.amount() < 0.0,
            "Short future above forward should have negative NPV"
        );
    }

    #[test]
    fn test_delta_vol() {
        let future = VolatilityIndexFuture::builder()
            .id(InstrumentId::new("VIX-DELTA"))
            .notional(Money::new(20_000.0, Currency::USD)) // 1 contract at 20
            .expiry_date(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .settlement_date(Date::from_calendar_date(2025, Month::April, 1).expect("valid date"))
            .quoted_price(20.0)
            .position(Position::Long)
            .contract_specs(VolIndexContractSpecs::vix())
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_index_curve_id(CurveId::new("VIX"))
            .build()
            .expect("valid future");

        let delta = future.delta_vol();
        // Long 1 contract: delta = -1 × 1000 = -1000
        // (NPV decreases by $1000 for each 1-point increase in forward vol)
        assert!(
            (delta + 1000.0).abs() < 10.0,
            "Delta should be approximately -1000, got {}",
            delta
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_round_trip() {
        let future = VolatilityIndexFuture::example();
        let json = serde_json::to_string(&future).expect("json serialization");
        let recovered: VolatilityIndexFuture =
            serde_json::from_str(&json).expect("json deserialization");
        assert_eq!(future.id, recovered.id);
        assert!((future.quoted_price - recovered.quoted_price).abs() < 1e-10);
    }
}
