//! Agency TBA (To-Be-Announced) forward types.
//!
//! TBA trades are forward contracts on agency MBS pools where the specific
//! pools to be delivered are not known at trade time. Instead, pools must
//! meet good delivery standards (coupon, term, agency).

use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::fixed_income::mbs_passthrough::{AgencyMbsPassthrough, AgencyProgram};
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// TBA term enumeration (original loan term).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TbaTerm {
    /// 15-year original term
    FifteenYear,
    /// 20-year original term
    TwentyYear,
    /// 30-year original term
    ThirtyYear,
}

impl TbaTerm {
    /// Get the term in years.
    pub fn years(&self) -> u32 {
        match self {
            TbaTerm::FifteenYear => 15,
            TbaTerm::TwentyYear => 20,
            TbaTerm::ThirtyYear => 30,
        }
    }

    /// Get the term in months.
    pub fn months(&self) -> u32 {
        self.years() * 12
    }
}

impl std::fmt::Display for TbaTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TbaTerm::FifteenYear => write!(f, "15Y"),
            TbaTerm::TwentyYear => write!(f, "20Y"),
            TbaTerm::ThirtyYear => write!(f, "30Y"),
        }
    }
}

/// TBA settlement information.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TbaSettlement {
    /// Good delivery (settlement) date
    pub settlement_date: Date,
    /// Last trading day (48 hours before settlement)
    pub notification_date: Date,
}

/// TBA (To-Be-Announced) forward trade.
///
/// Represents a forward contract to buy or sell agency MBS at a specified
/// price for a future settlement date. The specific pools delivered are
/// not known at trade time.
///
/// # Good Delivery Standards
///
/// TBA trades must meet SIFMA good delivery guidelines:
/// - Pool must be from the specified agency program
/// - Pool coupon must match the TBA coupon
/// - Pool term must match the TBA term
/// - Variance rules for face amount (±0.01% of trade amount)
///
/// # Pricing
///
/// TBA value is calculated as the difference between the forward value
/// of assumed pool characteristics and the trade price, discounted to
/// the valuation date.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fixed_income::tba::{AgencyTba, TbaTerm};
/// use finstack_valuations::instruments::fixed_income::mbs_passthrough::AgencyProgram;
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_core::dates::Date;
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let tba = AgencyTba::builder()
///     .id(InstrumentId::new("FN30-4.0-202403"))
///     .agency(AgencyProgram::Fnma)
///     .coupon(0.04)
///     .term(TbaTerm::ThirtyYear)
///     .settlement_year(2024)
///     .settlement_month(3)
///     .notional(Money::new(10_000_000.0, Currency::USD))
///     .trade_price(98.5)
///     .discount_curve_id(CurveId::new("USD-OIS"))
///     .build()
///     .expect("Valid TBA");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct AgencyTba {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Agency program (FNMA, FHLMC, GNMA).
    pub agency: AgencyProgram,
    /// Pass-through coupon rate (e.g., 0.04 for 4%).
    pub coupon: f64,
    /// Original loan term.
    pub term: TbaTerm,
    /// Settlement year.
    pub settlement_year: i32,
    /// Settlement month (1-12).
    pub settlement_month: u8,
    /// Trade notional (par amount).
    pub notional: Money,
    /// Trade price (percentage of par, e.g., 98.5).
    pub trade_price: f64,
    /// Trade date.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub trade_date: Option<Date>,
    /// Optional assumed pool for valuation.
    /// If not provided, generic pool characteristics are assumed.
    #[builder(optional)]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub assumed_pool: Option<Box<AgencyMbsPassthrough>>,
    /// Discount curve identifier.
    pub discount_curve_id: CurveId,
    /// Pricing overrides.
    #[builder(default)]
    #[cfg_attr(feature = "serde", serde(default))]
    pub pricing_overrides: PricingOverrides,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

impl AgencyTba {
    /// Create a canonical example TBA for testing and documentation.
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("FN30-4.0-202403"))
            .agency(AgencyProgram::Fnma)
            .coupon(0.04)
            .term(TbaTerm::ThirtyYear)
            .settlement_year(2024)
            .settlement_month(3)
            .notional(Money::new(10_000_000.0, Currency::USD))
            .trade_price(98.5)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(
                Attributes::new()
                    .with_tag("tba")
                    .with_tag("agency")
                    .with_meta("program", "fnma"),
            )
            .build()
            .unwrap_or_else(|_| unreachable!("Example TBA with valid constants should never fail"))
    }

    /// Builder helper for settlement month.
    pub fn settlement_month(mut self, year: i32, month: u8) -> Self {
        self.settlement_year = year;
        self.settlement_month = month;
        self
    }

    /// Get the settlement date (typically 3rd Wednesday of month).
    pub fn get_settlement_date(&self) -> finstack_core::Result<Date> {
        // SIFMA TBA settlement is typically the notification date + 2 business days
        // For simplicity, use mid-month as approximate settlement
        let month = time::Month::try_from(self.settlement_month)
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
        Date::from_calendar_date(self.settlement_year, month, 15)
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))
    }

    /// Get TBA identifier string (e.g., "FN30 4.0 Mar24").
    pub fn tba_identifier(&self) -> String {
        let agency_str = match self.agency {
            AgencyProgram::Fnma => "FN",
            AgencyProgram::Fhlmc => "FH",
            AgencyProgram::Gnma => "GN",
        };
        let term_str = match self.term {
            TbaTerm::FifteenYear => "15",
            TbaTerm::TwentyYear => "20",
            TbaTerm::ThirtyYear => "30",
        };
        let month_str = match self.settlement_month {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "???",
        };
        let year_str = (self.settlement_year % 100) as u8;

        format!(
            "{}{} {:.1} {}{:02}",
            agency_str,
            term_str,
            self.coupon * 100.0,
            month_str,
            year_str
        )
    }

    /// Calculate trade value (notional × price).
    pub fn trade_value(&self) -> Money {
        Money::new(
            self.notional.amount() * self.trade_price / 100.0,
            self.notional.currency(),
        )
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for AgencyTba {
    fn curve_dependencies(&self) -> crate::instruments::common_impl::traits::InstrumentCurves {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common_impl::traits::Instrument for AgencyTba {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::AgencyTba
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        crate::instruments::fixed_income::tba::pricer::price_tba(self, market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_tba_example() {
        let tba = AgencyTba::example();
        assert_eq!(tba.agency, AgencyProgram::Fnma);
        assert!((tba.coupon - 0.04).abs() < 1e-10);
        assert_eq!(tba.term, TbaTerm::ThirtyYear);
    }

    #[test]
    fn test_tba_identifier() {
        let tba = AgencyTba::example();
        let id = tba.tba_identifier();
        assert!(id.contains("FN30"));
        assert!(id.contains("4.0"));
    }

    #[test]
    fn test_trade_value() {
        let tba = AgencyTba::example();
        let value = tba.trade_value();
        // 10M at 98.5 = 9.85M
        assert!((value.amount() - 9_850_000.0).abs() < 1.0);
    }

    #[test]
    fn test_settlement_date() {
        let tba = AgencyTba::example();
        let settle = tba.get_settlement_date().expect("valid date");
        assert_eq!(settle.month(), Month::March);
        assert_eq!(settle.year(), 2024);
    }

    #[test]
    fn test_tba_term() {
        assert_eq!(TbaTerm::ThirtyYear.years(), 30);
        assert_eq!(TbaTerm::ThirtyYear.months(), 360);
        assert_eq!(TbaTerm::FifteenYear.years(), 15);
    }
}
