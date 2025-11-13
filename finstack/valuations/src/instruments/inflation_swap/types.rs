//! Zero-coupon Inflation Swap types and pricing implementation.

use crate::instruments::common::traits::Attributes;
use finstack_core::market_data::scalars::inflation_index::InflationLag;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_core::types::{CurveId, InstrumentId};

/// Direction from the perspective of paying fixed real vs receiving inflation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PayReceiveInflation {
    /// Pay fixed (real) leg, receive inflation leg
    PayFixed,
    /// Receive fixed (real) leg, pay inflation leg
    ReceiveFixed,
}

impl std::fmt::Display for PayReceiveInflation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayReceiveInflation::PayFixed => write!(f, "pay_fixed"),
            PayReceiveInflation::ReceiveFixed => write!(f, "receive_fixed"),
        }
    }
}

impl std::str::FromStr for PayReceiveInflation {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "pay_fixed" | "pay" => Ok(PayReceiveInflation::PayFixed),
            "receive_fixed" | "receive" => Ok(PayReceiveInflation::ReceiveFixed),
            other => Err(format!("Unknown inflation swap pay/receive: {}", other)),
        }
    }
}

/// Inflation swap definition (boilerplate)
///
/// Minimal fields to represent a zero-coupon inflation swap. We keep this
/// intentionally compact until full pricing is implemented.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct InflationSwap {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Notional in quote currency
    pub notional: Money,
    /// Start date of indexation
    pub start: Date,
    /// Maturity date
    pub maturity: Date,
    /// Fixed real rate (as decimal)
    pub fixed_rate: f64,
    /// Inflation index identifier (e.g., US-CPI-U)
    pub inflation_index_id: CurveId,
    /// Discount curve identifier (quote currency)
    pub discount_curve_id: CurveId,
    /// Day count for any accrual-style metrics if needed
    pub dc: DayCount,
    /// Trade side
    pub side: PayReceiveInflation,
    /// Optional contract-level lag override (if set, overrides index lag)
    #[builder(optional)]
    pub lag_override: Option<InflationLag>,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl InflationSwap {}

impl InflationSwap {
    /// Create a canonical example zero-coupon inflation swap (US CPI, 5Y).
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use time::Month;
        InflationSwapBuilder::new()
            .id(InstrumentId::new("INFLSWAP-USD-5Y"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(Date::from_calendar_date(2024, Month::January, 15).unwrap())
            .maturity(Date::from_calendar_date(2029, Month::January, 15).unwrap())
            .fixed_rate(0.02)
            .inflation_index_id(CurveId::new("US-CPI"))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .dc(DayCount::Act365F)
            .side(PayReceiveInflation::PayFixed)
            .attributes(Attributes::new())
            .build()
            .expect("Example InflationSwap construction should not fail")
    }

    fn projected_index_ratio(
        &self,
        curves: &MarketContext,
        discount_base: Date,
    ) -> finstack_core::Result<f64> {
        let inflation_index = curves.inflation_index_ref(self.inflation_index_id.as_str());
        let inflation_curve = curves.get_inflation_ref(self.inflation_index_id.as_str())?;

        let i_start = if let Some(index) = inflation_index {
            index.value_on(self.start)?
        } else {
            let t_start = DayCount::Act365F
                .year_fraction(
                    discount_base,
                    self.start,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            if t_start <= 0.0 {
                inflation_curve.base_cpi()
            } else {
                inflation_curve.cpi(t_start)
            }
        };

        if i_start <= 0.0 {
            return Err(finstack_core::error::InputError::NonPositiveValue.into());
        }

        let lag_policy = if let Some(override_lag) = self.lag_override {
            override_lag
        } else if let Some(index) = inflation_index {
            index.lag()
        } else {
            InflationLag::None
        };

        let lagged_maturity = match lag_policy {
            InflationLag::None => self.maturity,
            InflationLag::Months(m) => finstack_core::dates::add_months(self.maturity, -(m as i32)),
            InflationLag::Days(d) => self.maturity - time::Duration::days(d as i64),
            _ => self.maturity,
        };

        let t_maturity_infl = DayCount::Act365F
            .year_fraction(
                discount_base,
                lagged_maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        let i_maturity_projected = if t_maturity_infl <= 0.0 {
            inflation_curve.base_cpi()
        } else {
            inflation_curve.cpi(t_maturity_infl)
        };

        Ok(i_maturity_projected / i_start)
    }

    /// Calculate PV of the fixed leg (real rate leg)
    pub fn pv_fixed_leg(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves.get_discount_ref(self.discount_curve_id.as_str())?;

        let tau_accrual = self.dc.year_fraction(
            self.start,
            self.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        let fixed_payment = self.notional * ((1.0 + self.fixed_rate).powf(tau_accrual) - 1.0);

        let t_discount = DayCount::Act365F
            .year_fraction(
                as_of,
                self.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df = disc.df(t_discount);

        Ok(fixed_payment * df)
    }

    /// Calculate PV of the inflation leg
    pub fn pv_inflation_leg(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves.get_discount_ref(self.discount_curve_id.as_str())?;
        let index_ratio = self.projected_index_ratio(curves, as_of)?;
        let inflation_payment = self.notional * (index_ratio - 1.0);

        let t_discount = DayCount::Act365F
            .year_fraction(
                as_of,
                self.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df = disc.df(t_discount);

        Ok(inflation_payment * df)
    }

    /// Fixed rate that sets the swap's present value to zero (par real rate)
    pub fn par_rate(&self, curves: &MarketContext) -> finstack_core::Result<f64> {
        let disc = curves.get_discount_ref(self.discount_curve_id.as_str())?;
        let base = disc.base_date();
        let index_ratio = self.projected_index_ratio(curves, base)?;

        if index_ratio <= 0.0 {
            return Err(finstack_core::error::InputError::NonPositiveValue.into());
        }

        let tau = self.dc.year_fraction(
            self.start,
            self.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if tau <= 0.0 {
            return Ok(0.0);
        }

        Ok(index_ratio.powf(1.0 / tau) - 1.0)
    }

    /// Net present value of the instrument via legs
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        let pv_fixed = self.pv_fixed_leg(curves, as_of)?;
        let pv_inflation = self.pv_inflation_leg(curves, as_of)?;
        match self.side {
            PayReceiveInflation::ReceiveFixed => pv_fixed - pv_inflation,
            PayReceiveInflation::PayFixed => pv_inflation - pv_fixed,
        }
    }
}

impl crate::instruments::common::traits::Instrument for InflationSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::InflationSwap
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
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for InflationSwap {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}
