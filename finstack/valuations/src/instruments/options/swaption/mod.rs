//! Swaption (option on interest rate swap) implementation with SABR volatility.

pub mod metrics;

use super::models::{SABRModel, SABRParameters};
use super::OptionType;
use finstack_core::math::norm_cdf;
// use crate::results::ValuationResult;
use crate::instruments::traits::Attributes;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
// use finstack_core::market_data::MarketContext;
use finstack_core::market_data::traits::Discount;
use finstack_core::money::Money;
use finstack_core::{Error, Result, F};

/// Swaption settlement type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwaptionSettlement {
    /// Physical delivery of underlying swap
    Physical,
    /// Cash settlement based on swap value
    Cash,
}

/// Swaption exercise style
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwaptionExercise {
    /// European exercise (only at expiry)
    European,
    /// Bermudan exercise (on specific dates)
    Bermudan,
    /// American exercise (any time)
    American,
}

/// Swaption instrument
#[derive(Clone, Debug)]
pub struct Swaption {
    /// Unique identifier
    pub id: String,
    /// Option type (payer or receiver)
    pub option_type: OptionType,
    /// Notional amount
    pub notional: Money,
    /// Strike rate (fixed rate of underlying swap)
    pub strike_rate: F,
    /// Option expiry date
    pub expiry: Date,
    /// Underlying swap start date
    pub swap_start: Date,
    /// Underlying swap end date
    pub swap_end: Date,
    /// Fixed leg frequency
    pub fixed_freq: Frequency,
    /// Floating leg frequency
    pub float_freq: Frequency,
    /// Day count convention
    pub day_count: DayCount,
    /// Exercise style
    pub exercise: SwaptionExercise,
    /// Settlement type
    pub settlement: SwaptionSettlement,
    /// Discount curve identifier
    pub disc_id: &'static str,
    /// Forward curve identifier
    pub forward_id: &'static str,
    /// Volatility surface identifier
    pub vol_id: &'static str,
    /// Implied volatility (if known, overrides vol surface and SABR)
    pub implied_vol: Option<F>,
    /// SABR parameters (if calibrated)
    pub sabr_params: Option<SABRParameters>,
    /// Additional attributes
    pub attributes: Attributes,
}

impl Swaption {
    /// Create new European payer swaption
    #[allow(clippy::too_many_arguments)]
    pub fn new_payer(
        id: impl Into<String>,
        notional: Money,
        strike_rate: F,
        expiry: Date,
        swap_start: Date,
        swap_end: Date,
        disc_id: &'static str,
        forward_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            option_type: OptionType::Call, // Payer swaption is like a call
            notional,
            strike_rate,
            expiry,
            swap_start,
            swap_end,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            day_count: DayCount::Thirty360,
            exercise: SwaptionExercise::European,
            settlement: SwaptionSettlement::Physical,
            disc_id,
            forward_id,
            vol_id,
            implied_vol: None,
            sabr_params: None,
            attributes: Attributes::default(),
        }
    }

    /// Create new European receiver swaption
    #[allow(clippy::too_many_arguments)]
    pub fn new_receiver(
        id: impl Into<String>,
        notional: Money,
        strike_rate: F,
        expiry: Date,
        swap_start: Date,
        swap_end: Date,
        disc_id: &'static str,
        forward_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            option_type: OptionType::Put, // Receiver swaption is like a put
            notional,
            strike_rate,
            expiry,
            swap_start,
            swap_end,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            day_count: DayCount::Thirty360,
            exercise: SwaptionExercise::European,
            settlement: SwaptionSettlement::Physical,
            disc_id,
            forward_id,
            vol_id,
            implied_vol: None,
            sabr_params: None,
            attributes: Attributes::default(),
        }
    }

    /// Set SABR parameters for pricing
    pub fn with_sabr(mut self, params: SABRParameters) -> Self {
        self.sabr_params = Some(params);
        self
    }

    /// Calculate swap annuity (PV of $1 paid on fixed leg)
    fn swap_annuity(&self, disc: &dyn Discount) -> Result<F> {
        let base_date = disc.base_date();
        let mut annuity = 0.0;

        // Generate fixed leg schedule via centralized builder
        let sched = crate::cashflow::builder::build_dates(
            self.swap_start,
            self.swap_end,
            self.fixed_freq,
            StubKind::None,
            BusinessDayConvention::Following,
            None,
        );
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }

        let mut prev = dates[0];
        for &d in &dates[1..] {
            let t = self.year_fraction(base_date, d, self.day_count)?;
            let accrual = self.year_fraction(prev, d, self.day_count)?;
            let df = disc.df(t);
            annuity += accrual * df;
            prev = d;
        }

        Ok(annuity)
    }

    /// Calculate forward swap rate
    fn forward_swap_rate(&self, disc: &dyn Discount) -> Result<F> {
        let base_date = disc.base_date();

        // Calculate PV of floating leg (approximately par at inception)
        let t_start = self.year_fraction(base_date, self.swap_start, self.day_count)?;
        let t_end = self.year_fraction(base_date, self.swap_end, self.day_count)?;

        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);

        // Forward swap rate = (df_start - df_end) / annuity
        let annuity = self.swap_annuity(disc)?;

        Ok((df_start - df_end) / annuity)
    }

    /// Price using Black's model (baseline)
    pub fn black_price(&self, disc: &dyn Discount, volatility: F) -> Result<Money> {
        let base_date = disc.base_date();
        let time_to_expiry = self.year_fraction(base_date, self.expiry, self.day_count)?;

        if time_to_expiry <= 0.0 {
            // Option has expired
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let forward_rate = self.forward_swap_rate(disc)?;
        let annuity = self.swap_annuity(disc)?;

        // Black's formula
        let variance = volatility.powi(2) * time_to_expiry;
        let d1 = ((forward_rate / self.strike_rate).ln() + 0.5 * variance) / variance.sqrt();
        let d2 = d1 - variance.sqrt();

        let value = match self.option_type {
            OptionType::Call => {
                // Payer swaption - annuity already contains discounting
                annuity * (forward_rate * norm_cdf(d1) - self.strike_rate * norm_cdf(d2))
            }
            OptionType::Put => {
                // Receiver swaption - annuity already contains discounting
                annuity * (self.strike_rate * norm_cdf(-d2) - forward_rate * norm_cdf(-d1))
            }
        };

        Ok(Money::new(
            value * self.notional.amount(),
            self.notional.currency(),
        ))
    }

    /// Price using SABR model
    pub fn sabr_price(&self, disc: &dyn Discount) -> Result<Money> {
        let sabr_params = self.sabr_params.as_ref().ok_or(Error::Internal)?; // No SABR parameters

        let model = SABRModel::new(sabr_params.clone());

        let base_date = disc.base_date();
        let time_to_expiry = self.year_fraction(base_date, self.expiry, self.day_count)?;

        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let forward_rate = self.forward_swap_rate(disc)?;

        // Get SABR implied volatility
        let sabr_vol = model.implied_volatility(forward_rate, self.strike_rate, time_to_expiry)?;

        // Price using Black's formula with SABR vol
        self.black_price(disc, sabr_vol)
    }

    // Removed ad-hoc add_period: schedule comes from ScheduleBuilder

    /// Calculate year fraction
    fn year_fraction(&self, start: Date, end: Date, dc: DayCount) -> Result<F> {
        dc.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
    }
}

impl_instrument!(
    Swaption,
    "Swaption",
    pv = |s, curves, _as_of| {
        let disc = curves.discount(s.disc_id)?;
        if s.sabr_params.is_some() {
            s.sabr_price(disc.as_ref())
        } else {
            // Get volatility from surface
            let time_to_expiry = s.year_fraction(disc.base_date(), s.expiry, s.day_count)?;
            let vol = if let Some(impl_vol) = s.implied_vol {
                impl_vol
            } else {
                let vol_surface = curves.vol_surface(s.vol_id)?;
                vol_surface.value_clamped(time_to_expiry, s.strike_rate)
            };
            s.black_price(disc.as_ref(), vol)
        }
    },
);

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

    fn create_test_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots(vec![
                (0.0, 1.0),
                (1.0, 0.97),
                (2.0, 0.94),
                (5.0, 0.85),
                (10.0, 0.70),
            ])
            .build()
            .unwrap()
    }

    #[test]
    fn test_swaption_creation() {
        let expiry = Date::from_calendar_date(2025, time::Month::June, 1).unwrap();
        let swap_start = Date::from_calendar_date(2025, time::Month::June, 1).unwrap();
        let swap_end = Date::from_calendar_date(2030, time::Month::June, 1).unwrap();

        let swaption = Swaption::new_payer(
            "5Y5Y-PAYER",
            Money::new(10_000_000.0, Currency::USD),
            0.03, // 3% strike
            expiry,
            swap_start,
            swap_end,
            "USD-OIS",
            "USD-LIBOR-3M",
            "USD-SWAPTION-VOL",
        );

        assert_eq!(swaption.strike_rate, 0.03);
        assert_eq!(swaption.option_type, OptionType::Call);
    }

    #[test]
    fn test_swaption_black_pricing() {
        let _base_date = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let expiry = Date::from_calendar_date(2026, time::Month::January, 1).unwrap();
        let swap_start = expiry;
        let swap_end = Date::from_calendar_date(2031, time::Month::January, 1).unwrap();

        let swaption = Swaption::new_payer(
            "1Y5Y-PAYER",
            Money::new(10_000_000.0, Currency::USD),
            0.035, // 3.5% strike
            expiry,
            swap_start,
            swap_end,
            "USD-OIS",
            "USD-LIBOR-3M",
            "USD-SWAPTION-VOL",
        );

        let curve = create_test_curve();
        let price = swaption.black_price(&curve, 0.25).unwrap(); // 25% vol

        // Price should be positive
        assert!(price.amount() > 0.0);
    }

    #[test]
    fn test_swaption_with_sabr() {
        let _base_date = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let expiry = Date::from_calendar_date(2026, time::Month::January, 1).unwrap();
        let swap_start = expiry;
        let swap_end = Date::from_calendar_date(2031, time::Month::January, 1).unwrap();

        // Create SABR parameters
        let sabr_params = SABRParameters::new(0.01, 0.5, 0.3, -0.2).unwrap();

        let swaption = Swaption::new_receiver(
            "1Y5Y-RECEIVER",
            Money::new(10_000_000.0, Currency::USD),
            0.025, // 2.5% strike
            expiry,
            swap_start,
            swap_end,
            "USD-OIS",
            "USD-LIBOR-3M",
            "USD-SWAPTION-VOL",
        )
        .with_sabr(sabr_params);

        let curve = create_test_curve();
        let price = swaption.sabr_price(&curve).unwrap();

        // Price should be positive
        assert!(price.amount() > 0.0);

        // Compare with Black price
        let black_price = swaption.black_price(&curve, 0.20).unwrap();

        // Prices should be different (SABR accounts for smile)
        assert!((price.amount() - black_price.amount()).abs() > 0.01);
    }
}

// Generate standard Attributable implementation using macro
