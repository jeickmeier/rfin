//! Interest rate option instrument implementation using Black model.

pub mod metrics;

use crate::instruments::traits::Attributes;
// use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;

use finstack_core::dates::{Date, DayCount, Frequency};

use super::{ExerciseStyle, SettlementType};
use finstack_core::math::{norm_cdf, norm_pdf};

/// Type of interest rate option
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateOptionType {
    /// Cap (series of caplets)
    Cap,
    /// Floor (series of floorlets)
    Floor,
    /// Caplet (single period cap)
    Caplet,
    /// Floorlet (single period floor)
    Floorlet,
}

/// Interest rate option instrument (Black model)
#[derive(Clone, Debug)]
pub struct InterestRateOption {
    /// Unique instrument identifier
    pub id: String,
    /// Option type
    pub rate_option_type: RateOptionType,
    /// Notional amount
    pub notional: Money,
    /// Strike rate (as decimal, e.g., 0.05 for 5%)
    pub strike_rate: F,
    /// Start date of underlying period
    pub start_date: Date,
    /// End date of underlying period
    pub end_date: Date,
    /// Payment frequency for caps/floors
    pub frequency: Frequency,
    /// Day count convention
    pub day_count: DayCount,
    /// Exercise style
    pub exercise_style: ExerciseStyle,
    /// Settlement type
    pub settlement: SettlementType,
    /// Discount curve identifier
    pub disc_id: &'static str,
    /// Forward curve identifier
    pub forward_id: &'static str,
    /// Volatility surface identifier
    pub vol_id: &'static str,
    /// Implied volatility (if known, overrides vol surface)
    pub implied_vol: Option<F>,
    /// Additional attributes
    pub attributes: Attributes,
}

impl InterestRateOption {
    /// Create a new interest rate option
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        rate_option_type: RateOptionType,
        notional: Money,
        strike_rate: F,
        start_date: Date,
        end_date: Date,
        frequency: Frequency,
        day_count: DayCount,
        disc_id: &'static str,
        forward_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            rate_option_type,
            notional,
            strike_rate,
            start_date,
            end_date,
            frequency,
            day_count,
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Cash,
            disc_id,
            forward_id,
            vol_id,
            implied_vol: None,
            attributes: Attributes::new(),
        }
    }

    /// Create a cap instrument
    #[allow(clippy::too_many_arguments)]
    pub fn new_cap(
        id: impl Into<String>,
        notional: Money,
        strike_rate: F,
        start_date: Date,
        end_date: Date,
        frequency: Frequency,
        day_count: DayCount,
        disc_id: &'static str,
        forward_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        Self::new(
            id,
            RateOptionType::Cap,
            notional,
            strike_rate,
            start_date,
            end_date,
            frequency,
            day_count,
            disc_id,
            forward_id,
            vol_id,
        )
    }

    /// Create a floor instrument
    #[allow(clippy::too_many_arguments)]
    pub fn new_floor(
        id: impl Into<String>,
        notional: Money,
        strike_rate: F,
        start_date: Date,
        end_date: Date,
        frequency: Frequency,
        day_count: DayCount,
        disc_id: &'static str,
        forward_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        Self::new(
            id,
            RateOptionType::Floor,
            notional,
            strike_rate,
            start_date,
            end_date,
            frequency,
            day_count,
            disc_id,
            forward_id,
            vol_id,
        )
    }

    /// Calculate caplet/floorlet price using Black's model
    ///
    /// # Arguments
    /// * `forward_rate` - Forward rate for the period
    /// * `df` - Discount factor to payment date
    /// * `sigma` - Black implied volatility
    /// * `t` - Time to option expiry in years
    /// * `tau` - Year fraction for the payment period
    pub fn black_price_caplet_floorlet(
        &self,
        forward_rate: F,
        df: F,
        sigma: F,
        t: F,
        tau: F,
    ) -> finstack_core::Result<Money> {
        if t <= 0.0 {
            // Option expired
            let payoff = match self.rate_option_type {
                RateOptionType::Caplet | RateOptionType::Cap => {
                    (forward_rate - self.strike_rate).max(0.0)
                }
                RateOptionType::Floorlet | RateOptionType::Floor => {
                    (self.strike_rate - forward_rate).max(0.0)
                }
            };
            return Ok(Money::new(
                payoff * tau * self.notional.amount() * df,
                self.notional.currency(),
            ));
        }

        // Black's formula for caplet/floorlet
        let d1 = if sigma > 0.0 && t > 0.0 {
            ((forward_rate / self.strike_rate).ln() + 0.5 * sigma * sigma * t) / (sigma * t.sqrt())
        } else {
            0.0
        };
        let d2 = d1 - sigma * t.sqrt();

        let price = match self.rate_option_type {
            RateOptionType::Caplet | RateOptionType::Cap => {
                df * tau
                    * self.notional.amount()
                    * (forward_rate * norm_cdf(d1) - self.strike_rate * norm_cdf(d2))
            }
            RateOptionType::Floorlet | RateOptionType::Floor => {
                df * tau
                    * self.notional.amount()
                    * (self.strike_rate * norm_cdf(-d2) - forward_rate * norm_cdf(-d1))
            }
        };

        Ok(Money::new(price, self.notional.currency()))
    }

    /// Calculate option delta
    pub fn delta(&self, forward_rate: F, sigma: F, t: F) -> F {
        if t <= 0.0 || sigma <= 0.0 {
            return match self.rate_option_type {
                RateOptionType::Caplet | RateOptionType::Cap => {
                    if forward_rate > self.strike_rate { 1.0 } else { 0.0 }
                }
                RateOptionType::Floorlet | RateOptionType::Floor => {
                    if forward_rate < self.strike_rate { -1.0 } else { 0.0 }
                }
            };
        }

        let d1 =
            ((forward_rate / self.strike_rate).ln() + 0.5 * sigma * sigma * t) / (sigma * t.sqrt());

        match self.rate_option_type {
            RateOptionType::Caplet | RateOptionType::Cap => norm_cdf(d1),
            RateOptionType::Floorlet | RateOptionType::Floor => -norm_cdf(-d1),
        }
    }

    /// Calculate option gamma
    pub fn gamma(&self, forward_rate: F, sigma: F, t: F) -> F {
        if t <= 0.0 || sigma <= 0.0 || forward_rate <= 0.0 {
            return 0.0;
        }

        let d1 =
            ((forward_rate / self.strike_rate).ln() + 0.5 * sigma * sigma * t) / (sigma * t.sqrt());

        norm_pdf(d1) / (forward_rate * sigma * t.sqrt())
    }

    /// Calculate option vega
    pub fn vega(&self, forward_rate: F, sigma: F, t: F) -> F {
        if t <= 0.0 || forward_rate <= 0.0 {
            return 0.0;
        }

        let d1 = if sigma > 0.0 {
            ((forward_rate / self.strike_rate).ln() + 0.5 * sigma * sigma * t) / (sigma * t.sqrt())
        } else {
            0.0
        };

        forward_rate * norm_pdf(d1) * t.sqrt() / 100.0 // Per 1% vega
    }
}

impl_instrument!(
    InterestRateOption,
    "InterestRateOption",
    pv = |s, curves, as_of| {
        use crate::cashflow::builder::schedule_utils::build_dates;
        use finstack_core::dates::{BusinessDayConvention, StubKind};

        // Get market curves
        let disc_curve = curves.discount(s.disc_id)?;
        let fwd_curve = curves.forecast(s.forward_id)?;
        let vol_surface = if s.implied_vol.is_none() {
            Some(curves.vol_surface(s.vol_id)?)
        } else {
            None
        };

        let mut total_pv = finstack_core::money::Money::new(0.0, s.notional.currency());

        // For single caplet/floorlet, price directly
        if matches!(
            s.rate_option_type,
            RateOptionType::Caplet | RateOptionType::Floorlet
        ) {
            let time_to_fixing = s.day_count.year_fraction(as_of, s.start_date, finstack_core::dates::DayCountCtx::default())?;
            let time_to_payment = s.day_count.year_fraction(as_of, s.end_date, finstack_core::dates::DayCountCtx::default())?;
            let period_length = s.day_count.year_fraction(s.start_date, s.end_date, finstack_core::dates::DayCountCtx::default())?;

            if time_to_fixing <= 0.0 {
                // Option expired - intrinsic value only
                let forward_rate = fwd_curve.rate(time_to_fixing.max(0.0));
                let intrinsic = match s.rate_option_type {
                    RateOptionType::Caplet => (forward_rate - s.strike_rate).max(0.0),
                    RateOptionType::Floorlet => (s.strike_rate - forward_rate).max(0.0),
                    _ => 0.0,
                };
                let df = disc_curve.df(time_to_payment);
                return Ok(finstack_core::money::Money::new(
                    intrinsic * period_length * s.notional.amount() * df,
                    s.notional.currency(),
                ));
            }

            let forward_rate = fwd_curve.rate_period(time_to_fixing, time_to_payment);
            let df = disc_curve.df(time_to_payment);

            let sigma = if let Some(impl_vol) = s.implied_vol {
                impl_vol
            } else if let Some(vol_surf) = &vol_surface {
                vol_surf.value_clamped(time_to_fixing, s.strike_rate)
            } else {
                return Err(finstack_core::error::InputError::NotFound { id: "cap_floor_rate_index".to_string() }.into());
            };

            return s.black_price_caplet_floorlet(
                forward_rate,
                df,
                sigma,
                time_to_fixing,
                period_length,
            );
        }

        // For cap/floor, price as portfolio of caplets/floorlets
        let schedule = build_dates(
            s.start_date,
            s.end_date,
            s.frequency,
            StubKind::None,
            BusinessDayConvention::Following,
            None,
        );

        if schedule.dates.len() < 2 {
            return Ok(total_pv);
        }

        // Price each caplet/floorlet
        let mut prev_date = schedule.dates[0];
        for &payment_date in &schedule.dates[1..] {
            let fixing_date = prev_date; // Simplified: fixing at period start
            let time_to_fixing = s.day_count.year_fraction(as_of, fixing_date, finstack_core::dates::DayCountCtx::default())?;
            let time_to_payment = s.day_count.year_fraction(as_of, payment_date, finstack_core::dates::DayCountCtx::default())?;
            let period_length = s.day_count.year_fraction(fixing_date, payment_date, finstack_core::dates::DayCountCtx::default())?;

            if time_to_fixing > 0.0 {
                // Only price future caplets/floorlets
                let forward_rate = fwd_curve.rate_period(time_to_fixing, time_to_payment);
                let df = disc_curve.df(time_to_payment);

                let sigma = if let Some(impl_vol) = s.implied_vol {
                    impl_vol
                } else if let Some(vol_surf) = &vol_surface {
                    vol_surf.value_clamped(time_to_fixing, s.strike_rate)
                } else {
                    return Err(finstack_core::error::InputError::NotFound { id: "cap_floor_rate_index".to_string() }.into());
                };

                let caplet_price = s.black_price_caplet_floorlet(
                    forward_rate,
                    df,
                    sigma,
                    time_to_fixing,
                    period_length,
                )?;

                total_pv = (total_pv + caplet_price)?;
            }

            prev_date = payment_date;
        }

        Ok(total_pv)
    }
);

// Conversions and Attributable provided by macro

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_cap_creation() {
        let notional = Money::new(10_000_000.0, Currency::USD);
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        let cap = InterestRateOption::new_cap(
            "USD_CAP_3%",
            notional,
            0.03,
            start,
            end,
            Frequency::quarterly(),
            DayCount::Act360,
            "USD-OIS",
            "USD-LIBOR-3M",
            "USD-CAP-VOL",
        );

        assert_eq!(cap.id, "USD_CAP_3%");
        assert_eq!(cap.rate_option_type, RateOptionType::Cap);
        assert_eq!(cap.strike_rate, 0.03);
    }

    #[test]
    fn test_black_caplet_pricing() {
        let notional = Money::new(10_000_000.0, Currency::USD);
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2025, Month::April, 1).unwrap();

        let caplet = InterestRateOption {
            id: "CAPLET".to_string(),
            rate_option_type: RateOptionType::Caplet,
            notional,
            strike_rate: 0.03,
            start_date: start,
            end_date: end,
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Cash,
            disc_id: "USD-OIS",
            forward_id: "USD-LIBOR-3M",
            vol_id: "USD-CAP-VOL",
            implied_vol: Some(0.20),
            attributes: Attributes::new(),
        };

        // Test parameters
        let forward_rate = 0.035; // 3.5% forward rate
        let df = 0.99; // Discount factor
        let sigma = 0.20; // 20% volatility
        let t = 0.25; // 3 months to expiry
        let tau = 0.25; // 3-month period

        let price = caplet
            .black_price_caplet_floorlet(forward_rate, df, sigma, t, tau)
            .unwrap();

        // Caplet should have positive value when forward > strike
        assert!(price.amount() > 0.0);

        // Test Greeks
        let delta = caplet.delta(forward_rate, sigma, t);
        assert!(delta > 0.0 && delta < 1.0);

        let gamma = caplet.gamma(forward_rate, sigma, t);
        assert!(gamma > 0.0);
    }

    #[test]
    fn placeholder_no_swaptions_here() {
        // This test serves as a placeholder to document that swaption functionality
        // has been moved to the dedicated swaption module
    }
}
