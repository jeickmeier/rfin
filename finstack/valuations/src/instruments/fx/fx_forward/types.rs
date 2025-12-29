//! FX Forward types and implementations.
//!
//! Defines the `FxForward` instrument for outright forward contracts on
//! currency pairs. Pricing uses covered interest rate parity (CIRP) with
//! optional contract rate override.

use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::{Attributes, CurveIdVec};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;
use smallvec::smallvec;
use time::macros::date;

/// FX forward (outright forward) instrument.
///
/// Represents a commitment to exchange one currency for another at a specified
/// future date at a predetermined rate. The position is long base currency
/// (foreign) and short quote currency (domestic).
///
/// # Pricing
///
/// Forward value is calculated using covered interest rate parity:
/// ```text
/// F_market = S × DF_foreign(T) / DF_domestic(T)
/// PV = notional × (F_market - F_contract) × DF_domestic(T)
/// ```
/// where:
/// - S = spot FX rate (from FxMatrix or spot_rate_override)
/// - DF_foreign(T) = discount factor in base currency to maturity
/// - DF_domestic(T) = discount factor in quote currency to maturity
/// - F_contract = contract_rate (if provided, else F_market for at-market forward)
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fx_forward::FxForward;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use finstack_core::money::Money;
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let forward = FxForward::builder()
///     .id(InstrumentId::new("EURUSD-FWD-6M"))
///     .base_currency(Currency::EUR)
///     .quote_currency(Currency::USD)
///     .maturity_date(Date::from_calendar_date(2025, Month::June, 15).unwrap())
///     .notional(Money::new(1_000_000.0, Currency::EUR))
///     .domestic_discount_curve_id(CurveId::new("USD-OIS"))
///     .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
///     .build()
///     .expect("Valid forward");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FxForward {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Base currency (foreign currency, numerator of the pair).
    pub base_currency: Currency,
    /// Quote currency (domestic currency, denominator of the pair, PV currency).
    pub quote_currency: Currency,
    /// Maturity/settlement date.
    pub maturity_date: Date,
    /// Notional amount in base currency.
    pub notional: Money,
    /// Contract forward rate (quote per base). If None, valued at-market.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub contract_rate: Option<f64>,
    /// Domestic (quote currency) discount curve ID.
    pub domestic_discount_curve_id: CurveId,
    /// Foreign (base currency) discount curve ID.
    pub foreign_discount_curve_id: CurveId,
    /// Optional spot rate override (quote per base). If None, source from FxMatrix.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub spot_rate_override: Option<f64>,
    /// Optional base currency calendar for business day adjustment.
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub base_calendar_id: Option<String>,
    /// Optional quote currency calendar for business day adjustment.
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub quote_calendar_id: Option<String>,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

impl FxForward {
    /// Create a canonical example FX forward for testing and documentation.
    ///
    /// Returns a 6-month EUR/USD forward with realistic parameters.
    pub fn example() -> Self {
        // SAFETY: All inputs are compile-time validated constants
        Self::builder()
            .id(InstrumentId::new("EURUSD-FWD-6M"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .maturity_date(date!(2025 - 06 - 15))
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .contract_rate_opt(Some(1.12))
            .attributes(Attributes::new().with_tag("fx").with_meta("pair", "EURUSD"))
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example FX forward with valid constants should never fail")
            })
    }

    /// Construct an FX forward from trade date and tenor using joint calendar spot roll.
    ///
    /// # Arguments
    ///
    /// * `id` - Instrument identifier
    /// * `base_currency` - Foreign currency (numerator)
    /// * `quote_currency` - Domestic currency (denominator)
    /// * `trade_date` - Trade date
    /// * `tenor_days` - Days from spot to maturity
    /// * `notional` - Notional in base currency
    /// * `domestic_discount_curve_id` - Quote currency discount curve
    /// * `foreign_discount_curve_id` - Base currency discount curve
    /// * `base_calendar_id` - Optional base currency calendar
    /// * `quote_calendar_id` - Optional quote currency calendar
    /// * `spot_lag_days` - Spot lag (typically 2, or 1 for USD/CAD)
    /// * `bdc` - Business day convention
    #[allow(clippy::too_many_arguments)]
    pub fn from_trade_date(
        id: impl Into<InstrumentId>,
        base_currency: Currency,
        quote_currency: Currency,
        trade_date: Date,
        tenor_days: i64,
        notional: Money,
        domestic_discount_curve_id: impl Into<CurveId>,
        foreign_discount_curve_id: impl Into<CurveId>,
        base_calendar_id: Option<String>,
        quote_calendar_id: Option<String>,
        spot_lag_days: u32,
        bdc: finstack_core::dates::BusinessDayConvention,
    ) -> finstack_core::Result<Self> {
        use crate::instruments::common::fx_dates::{adjust_joint_calendar, roll_spot_date};

        let spot_date = roll_spot_date(
            trade_date,
            spot_lag_days,
            bdc,
            base_calendar_id.as_deref(),
            quote_calendar_id.as_deref(),
        )?;
        let maturity_unadjusted = spot_date + time::Duration::days(tenor_days);
        let maturity_date = adjust_joint_calendar(
            maturity_unadjusted,
            bdc,
            base_calendar_id.as_deref(),
            quote_calendar_id.as_deref(),
        )?;

        Self::builder()
            .id(id.into())
            .base_currency(base_currency)
            .quote_currency(quote_currency)
            .maturity_date(maturity_date)
            .notional(notional)
            .domestic_discount_curve_id(domestic_discount_curve_id.into())
            .foreign_discount_curve_id(foreign_discount_curve_id.into())
            .base_calendar_id_opt(base_calendar_id)
            .quote_calendar_id_opt(quote_calendar_id)
            .attributes(Attributes::new())
            .build()
    }

    /// Create an FX forward with forward points instead of outright rate.
    ///
    /// Forward points are the difference between the forward rate and spot rate,
    /// typically quoted in pips (1/10000 for most pairs).
    ///
    /// # Arguments
    ///
    /// * `spot_rate` - Current spot rate (quote per base)
    /// * `forward_points` - Forward points (in same units as spot, e.g., 0.0050 for 50 pips)
    pub fn with_forward_points(mut self, spot_rate: f64, forward_points: f64) -> Self {
        self.contract_rate = Some(spot_rate + forward_points);
        self.spot_rate_override = Some(spot_rate);
        self
    }

    /// Compute present value in quote currency.
    ///
    /// Uses covered interest rate parity to compute the market forward rate,
    /// then values the position based on the difference between market and
    /// contract rates.
    pub fn npv(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        use finstack_core::money::fx::FxQuery;

        // If maturity has passed, value is zero
        if self.maturity_date < as_of {
            return Ok(Money::new(0.0, self.quote_currency));
        }

        // Get discount curves
        let domestic_disc = market.get_discount_ref(self.domestic_discount_curve_id.as_str())?;
        let foreign_disc = market.get_discount_ref(self.foreign_discount_curve_id.as_str())?;

        // Discount factors from as_of to maturity
        let df_domestic = domestic_disc.df_between_dates(as_of, self.maturity_date)?;
        let df_foreign = foreign_disc.df_between_dates(as_of, self.maturity_date)?;

        // Resolve spot rate
        let spot = if let Some(rate) = self.spot_rate_override {
            rate
        } else if let Some(fx) = market.fx.as_ref() {
            (**fx)
                .rate(FxQuery::new(self.base_currency, self.quote_currency, as_of))?
                .rate
        } else {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound {
                    id: "fx_matrix".to_string(),
                },
            ));
        };

        // Compute market forward rate via CIRP: F = S × DF_foreign / DF_domestic
        let market_forward = spot * df_foreign / df_domestic;

        // Contract rate (if None, at-market forward has zero PV)
        let contract_fwd = self.contract_rate.unwrap_or(market_forward);

        // Validate notional currency
        if self.notional.currency() != self.base_currency {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ));
        }
        let n_base = self.notional.amount();

        // PV = notional × (F_market - F_contract) × DF_domestic
        // Long base currency means we profit when market forward > contract forward
        let pv = n_base * (market_forward - contract_fwd) * df_domestic;

        Ok(Money::new(pv, self.quote_currency))
    }

    /// Compute the market forward rate via covered interest rate parity.
    pub fn market_forward_rate(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        use finstack_core::money::fx::FxQuery;

        let domestic_disc = market.get_discount_ref(self.domestic_discount_curve_id.as_str())?;
        let foreign_disc = market.get_discount_ref(self.foreign_discount_curve_id.as_str())?;

        let df_domestic = domestic_disc.df_between_dates(as_of, self.maturity_date)?;
        let df_foreign = foreign_disc.df_between_dates(as_of, self.maturity_date)?;

        let spot = if let Some(rate) = self.spot_rate_override {
            rate
        } else if let Some(fx) = market.fx.as_ref() {
            (**fx)
                .rate(FxQuery::new(self.base_currency, self.quote_currency, as_of))?
                .rate
        } else {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound {
                    id: "fx_matrix".to_string(),
                },
            ));
        };

        Ok(spot * df_foreign / df_domestic)
    }
}

impl crate::instruments::common::traits::CurveDependencies for FxForward {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.domestic_discount_curve_id.clone())
            .discount(self.foreign_discount_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common::traits::Instrument for FxForward {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FxForward
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
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
        )
    }

    fn required_discount_curves(&self) -> CurveIdVec {
        smallvec![
            self.domestic_discount_curve_id.clone(),
            self.foreign_discount_curve_id.clone(),
        ]
    }
}

impl HasDiscountCurve for FxForward {
    fn discount_curve_id(&self) -> &CurveId {
        &self.domestic_discount_curve_id
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_fx_forward_creation() {
        let forward = FxForward::builder()
            .id(InstrumentId::new("TEST-FWD"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .maturity_date(Date::from_calendar_date(2025, Month::June, 15).expect("valid date"))
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        assert_eq!(forward.id.as_str(), "TEST-FWD");
        assert_eq!(forward.base_currency, Currency::EUR);
        assert_eq!(forward.quote_currency, Currency::USD);
        assert_eq!(forward.notional.amount(), 1_000_000.0);
    }

    #[test]
    fn test_fx_forward_example() {
        let forward = FxForward::example();
        assert_eq!(forward.id.as_str(), "EURUSD-FWD-6M");
        assert_eq!(forward.base_currency, Currency::EUR);
        assert_eq!(forward.quote_currency, Currency::USD);
        assert!(forward.attributes.has_tag("fx"));
    }

    #[test]
    fn test_fx_forward_with_forward_points() {
        let forward = FxForward::builder()
            .id(InstrumentId::new("FWD-POINTS"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .maturity_date(Date::from_calendar_date(2025, Month::June, 15).expect("valid date"))
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .attributes(Attributes::new())
            .build()
            .expect("should build")
            .with_forward_points(1.10, 0.0050);

        assert_eq!(forward.spot_rate_override, Some(1.10));
        assert!((forward.contract_rate.expect("contract rate set") - 1.105).abs() < 1e-10);
    }

    #[test]
    fn test_fx_forward_instrument_trait() {
        use crate::instruments::common::traits::Instrument;

        let forward = FxForward::example();

        assert_eq!(forward.id(), "EURUSD-FWD-6M");
        assert_eq!(forward.key(), crate::pricer::InstrumentType::FxForward);
        assert!(forward.attributes().has_tag("fx"));
    }

    #[test]
    fn test_fx_forward_curve_dependencies() {
        use crate::instruments::common::traits::CurveDependencies;

        let forward = FxForward::example();
        let deps = forward.curve_dependencies();

        assert_eq!(deps.discount_curves.len(), 2);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_fx_forward_serde_roundtrip() {
        let forward = FxForward::example();
        let json = serde_json::to_string(&forward).expect("serialize");
        let deserialized: FxForward = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(forward.id.as_str(), deserialized.id.as_str());
        assert_eq!(forward.base_currency, deserialized.base_currency);
        assert_eq!(forward.quote_currency, deserialized.quote_currency);
    }
}
