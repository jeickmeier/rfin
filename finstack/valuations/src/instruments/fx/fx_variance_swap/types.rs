//! FX variance swap type definitions and pricing logic.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::models::bs_price;
use crate::instruments::common::parameters::OptionType;
use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::Attributes;
use crate::instruments::common::traits::CurveDependencies;
use crate::instruments::common::traits::Instrument as InstrumentTrait;
use crate::instruments::common::traits::InstrumentCurves;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DateExt, DayCount, DayCountCtx, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::stats::{realized_variance, RealizedVarMethod};
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Side of the variance swap (pay or receive variance).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PayReceive {
    /// Pay variance (short variance)
    Pay,
    /// Receive variance (long variance)
    Receive,
}

impl std::fmt::Display for PayReceive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayReceive::Pay => write!(f, "pay"),
            PayReceive::Receive => write!(f, "receive"),
        }
    }
}

impl std::str::FromStr for PayReceive {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "pay" | "payer" | "short" => Ok(PayReceive::Pay),
            "receive" | "receiver" | "long" => Ok(PayReceive::Receive),
            other => Err(format!("Unknown variance swap pay/receive: {}", other)),
        }
    }
}

impl PayReceive {
    /// Get the sign multiplier for PV calculation.
    pub fn sign(&self) -> f64 {
        match self {
            PayReceive::Pay => -1.0,
            PayReceive::Receive => 1.0,
        }
    }
}

/// FX variance swap instrument.
///
/// Payoff: Notional * (Realized Variance - Strike Variance)
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FxVarianceSwap {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Base currency (foreign)
    pub base_currency: Currency,
    /// Quote currency (domestic)
    pub quote_currency: Currency,
    /// Optional spot identifier used to look up historical series.
    #[builder(optional)]
    pub spot_id: Option<String>,
    /// Variance notional (in quote currency units)
    pub notional: Money,
    /// Strike variance (annualized)
    pub strike_variance: f64,
    /// Start date of observation period
    pub start_date: Date,
    /// Maturity/settlement date
    pub maturity: Date,
    /// Observation frequency
    pub observation_freq: Tenor,
    /// Method for calculating realized variance
    pub realized_var_method: RealizedVarMethod,
    /// Pay/receive variance
    pub side: PayReceive,
    /// Domestic currency discount curve ID
    pub domestic_discount_curve_id: CurveId,
    /// Foreign currency discount curve ID
    pub foreign_discount_curve_id: CurveId,
    /// FX volatility surface ID
    pub vol_surface_id: CurveId,
    /// Day count convention for time calculations
    pub day_count: DayCount,
    /// Attributes for scenario selection
    pub attributes: Attributes,
}

impl FxVarianceSwap {
    /// Create a canonical example FX variance swap (EUR/USD, 1Y).
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        use time::Month;
        FxVarianceSwapBuilder::new()
            .id(InstrumentId::new("FXVAR-EURUSD-1Y"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .spot_id("EURUSD".to_string())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .strike_variance(0.04)
            .start_date(
                Date::from_calendar_date(2024, Month::January, 2).expect("Valid example date"),
            )
            .maturity(
                Date::from_calendar_date(2025, Month::January, 2).expect("Valid example date"),
            )
            .observation_freq(Tenor::daily())
            .realized_var_method(RealizedVarMethod::CloseToClose)
            .side(PayReceive::Receive)
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .vol_surface_id(CurveId::new("EURUSD-VOL"))
            .day_count(DayCount::Act365F)
            .attributes(Attributes::new())
            .build()
            .expect("Example FxVarianceSwap construction should not fail")
    }

    fn validate_as_of(&self, context: &MarketContext, as_of: Date) -> Result<()> {
        let dom = context.get_discount_ref(self.domestic_discount_curve_id.as_str())?;
        let for_curve = context.get_discount_ref(self.foreign_discount_curve_id.as_str())?;
        let dom_base = dom.base_date();
        let for_base = for_curve.base_date();
        if as_of < dom_base || as_of < for_base {
            return Err(finstack_core::Error::Validation(format!(
                "FxVarianceSwap valuation as_of date ({}) precedes curve base date (dom {}, for {}).",
                as_of, dom_base, for_base
            )));
        }
        Ok(())
    }

    fn series_id(&self) -> String {
        if let Some(id) = &self.spot_id {
            id.clone()
        } else {
            format!("{}{}", self.base_currency, self.quote_currency)
        }
    }

    fn spot_rate(&self, context: &MarketContext, as_of: Date) -> Result<f64> {
        if let Some(fx) = context.fx() {
            let rate = fx
                .rate(FxQuery::new(self.base_currency, self.quote_currency, as_of))?
                .rate;
            return Ok(rate);
        }
        let spot_id = self.series_id();
        let scalar = context.price(&spot_id).map_err(|_| {
            finstack_core::Error::Input(finstack_core::InputError::NotFound { id: spot_id })
        })?;
        let spot = match scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };
        Ok(spot)
    }

    /// Calculate present value of the FX variance swap.
    pub fn npv(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        self.validate_as_of(context, as_of)?;

        let dom = context.get_discount_ref(self.domestic_discount_curve_id.as_str())?;

        if as_of >= self.maturity {
            let prices = self.get_historical_prices(context, as_of)?;
            if prices.is_empty() {
                return Ok(Money::new(0.0, self.notional.currency()));
            }
            let realized_var = realized_variance(
                &prices,
                self.realized_var_method,
                self.annualization_factor(),
            );
            return Ok(self.payoff(realized_var));
        }

        if as_of < self.start_date {
            let forward_var = self.remaining_forward_variance(context, as_of)?;
            let undiscounted = self.payoff(forward_var);
            let t = self
                .day_count
                .year_fraction(as_of, self.maturity, DayCountCtx::default())?;
            let df = dom.df(t.max(0.0));
            return Ok(undiscounted * df);
        }

        let realized = self.partial_realized_variance(context, as_of)?;
        let forward = self.remaining_forward_variance(context, as_of)?;
        let w = self.realized_fraction_by_observations(as_of);
        let expected_var = realized * w + forward * (1.0 - w);
        let undiscounted = self.payoff(expected_var);
        let t = self
            .day_count
            .year_fraction(as_of, self.maturity, DayCountCtx::default())?;
        let df = dom.df(t.max(0.0));
        Ok(undiscounted * df)
    }

    /// Calculate payoff given realized variance.
    pub fn payoff(&self, realized_variance: f64) -> Money {
        let variance_diff = realized_variance - self.strike_variance;
        Money::new(
            self.notional.amount() * variance_diff * self.side.sign(),
            self.notional.currency(),
        )
    }

    /// Get observation dates based on frequency.
    pub fn observation_dates(&self) -> Vec<Date> {
        let mut dates = Vec::new();
        let mut current = self.start_date;

        if let Some(months_step) = self.observation_freq.months() {
            while current <= self.maturity {
                dates.push(current);
                current = current.add_months(months_step as i32);
                if current > self.maturity {
                    break;
                }
            }
        } else if let Some(days_step) = self.observation_freq.days() {
            while current <= self.maturity {
                dates.push(current);
                current += time::Duration::days(days_step as i64);
                if current > self.maturity {
                    break;
                }
            }
        } else {
            while current <= self.maturity {
                dates.push(current);
                current += time::Duration::days(1);
                if current > self.maturity {
                    break;
                }
            }
        }

        if dates.is_empty() || dates.last() != Some(&self.maturity) {
            dates.push(self.maturity);
        }

        dates
    }

    /// Calculate annualization factor based on observation frequency.
    pub fn annualization_factor(&self) -> f64 {
        if let Some(months) = self.observation_freq.months() {
            return match months {
                1 => 12.0,
                3 => 4.0,
                6 => 2.0,
                12 => 1.0,
                _ => 252.0,
            };
        }
        if let Some(days) = self.observation_freq.days() {
            return match days {
                1 => 252.0,
                7 => 52.0,
                14 => 26.0,
                _ => 365.0 / days as f64,
            };
        }
        252.0
    }

    /// Calculate realized fraction based on observation counts.
    pub fn realized_fraction_by_observations(&self, as_of: Date) -> f64 {
        let all = self.observation_dates();
        if all.is_empty() {
            return 0.0;
        }
        if as_of <= self.start_date {
            return 0.0;
        }
        if as_of >= self.maturity {
            return 1.0;
        }
        let total = all.len() as f64;
        let realized = all.iter().filter(|&&d| d <= as_of).count() as f64;
        (realized / total).clamp(0.0, 1.0)
    }

    /// Get historical prices aligned to observation dates when available.
    pub fn get_historical_prices(&self, context: &MarketContext, as_of: Date) -> Result<Vec<f64>> {
        let series_id = self.series_id();
        if let Ok(series) = context.series(&series_id) {
            let dates: Vec<Date> = self
                .observation_dates()
                .into_iter()
                .filter(|&d| d <= as_of)
                .collect();
            if dates.len() >= 2 {
                return series.values_on(&dates);
            }
        }

        let spot = self.spot_rate(context, as_of)?;
        Ok(vec![spot])
    }

    /// Calculate partial realized variance for the elapsed period.
    pub fn partial_realized_variance(&self, context: &MarketContext, as_of: Date) -> Result<f64> {
        let prices = self.get_historical_prices(context, as_of)?;
        if prices.len() < 2 {
            return Ok(0.0);
        }
        Ok(realized_variance(
            &prices,
            self.realized_var_method,
            self.annualization_factor(),
        ))
    }

    /// Calculate implied forward variance for the remaining period.
    pub fn remaining_forward_variance(&self, context: &MarketContext, as_of: Date) -> Result<f64> {
        let t = self
            .day_count
            .year_fraction(as_of, self.maturity, DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let spot = self.spot_rate(context, as_of)?;
        let surface = context.surface_ref(self.vol_surface_id.as_str())?;

        let dom = context.get_discount_ref(self.domestic_discount_curve_id.as_str())?;
        let for_curve = context.get_discount_ref(self.foreign_discount_curve_id.as_str())?;
        let t_dom = dom
            .day_count()
            .year_fraction(as_of, self.maturity, DayCountCtx::default())?;
        let t_for =
            for_curve
                .day_count()
                .year_fraction(as_of, self.maturity, DayCountCtx::default())?;
        let df_dom = dom.df(t_dom.max(0.0));
        let df_for = for_curve.df(t_for.max(0.0));

        let r_d = -df_dom.ln() / t;
        let r_f = -df_for.ln() / t;
        let fwd = spot * ((r_d - r_f) * t).exp();

        let strikes = surface.strikes();
        if strikes.len() >= 3 && fwd.is_finite() && fwd > 0.0 {
            let mut k0_idx = 0usize;
            for (i, &k) in strikes.iter().enumerate() {
                if k <= fwd {
                    k0_idx = i;
                } else {
                    break;
                }
            }
            let k0 = strikes[k0_idx].max(1e-12);

            let mut sum = 0.0;
            for i in 0..strikes.len() {
                let k = strikes[i].max(1e-12);
                let dk = if i == 0 {
                    strikes[1] - strikes[0]
                } else if i + 1 == strikes.len() {
                    strikes[i] - strikes[i - 1]
                } else {
                    0.5 * (strikes[i + 1] - strikes[i - 1])
                };

                let vol = surface.value_clamped(t, k).max(1e-8);
                let call = bs_price(spot, k, r_d, r_f, vol, t, OptionType::Call);
                let put = bs_price(spot, k, r_d, r_f, vol, t, OptionType::Put);

                let qk = if i == k0_idx {
                    0.5 * (call + put)
                } else if k < fwd {
                    put
                } else {
                    call
                };

                sum += (dk / (k * k)) * qk;
            }

            let variance =
                (2.0 * (r_d * t).exp() / t) * sum - (1.0 / t) * ((fwd / k0 - 1.0).powi(2));
            if variance.is_finite() && variance > 0.0 {
                return Ok(variance);
            }
        }

        let vol_atm = surface.value_clamped(t, fwd.max(1e-12));
        if vol_atm.is_finite() && vol_atm > 0.0 {
            return Ok(vol_atm * vol_atm);
        }

        Ok(self.strike_variance)
    }
}

impl InstrumentTrait for FxVarianceSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FxVarianceSwap
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn InstrumentTrait> {
        Box::new(self.clone())
    }

    fn value(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
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
            None,
        )
    }

    fn as_cashflow_provider(&self) -> Option<&dyn CashflowProvider> {
        Some(self)
    }
}

impl HasDiscountCurve for FxVarianceSwap {
    fn discount_curve_id(&self) -> &CurveId {
        &self.domestic_discount_curve_id
    }
}

impl CurveDependencies for FxVarianceSwap {
    fn curve_dependencies(&self) -> InstrumentCurves {
        InstrumentCurves::builder()
            .discount(self.domestic_discount_curve_id.clone())
            .build()
    }
}

impl CashflowProvider for FxVarianceSwap {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn build_schedule(&self, _context: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        Ok(vec![(
            self.maturity,
            Money::new(0.0, self.notional.currency()),
        )])
    }
}
