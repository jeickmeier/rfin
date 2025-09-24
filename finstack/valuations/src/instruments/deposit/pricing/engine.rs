//! Core deposit pricing engine and shared helpers.
//!
//! Provides fundamental pricing logic for a single‑period money‑market deposit.
//! Pricing is deterministic and uses the instrument day‑count convention to
//! compute the accrual year fraction for simple interest. Discounting uses the
//! discount curve's own time basis for date mapping to ensure currency‑safe
//! and policy‑visible valuation consistent with other instruments.
//!
//! # Examples
//! ```rust
//! use finstack_core::{dates::*, money::Money, currency::Currency};
//! use finstack_core::market_data::term_structures::{discount_curve::DiscountCurve, CurveBuilder};
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_valuations::instruments::deposit::Deposit;
//! use finstack_valuations::instruments::deposit::pricing::engine::DepositEngine;
//! use time::Month;
//!
//! let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//! let disc = DiscountCurve::builder("USD-OIS")
//!     .base_date(base)
//!     .knots([(0.0, 1.0), (1.0, 0.98)])
//!     .build()
//!     .unwrap();
//! let ctx = MarketContext::new().insert_discount(disc);
//!
//! let dep = Deposit::builder()
//!     .id(finstack_core::types::InstrumentId::new("DEP1"))
//!     .notional(Money::new(1_000_000.0, Currency::USD))
//!     .start(base)
//!     .end(Date::from_calendar_date(2025, Month::July, 1).unwrap())
//!     .day_count(DayCount::Act360)
//!     .disc_id(finstack_core::types::CurveId::new("USD-OIS"))
//!     .build()
//!     .unwrap();
//!
//! let pv = DepositEngine::pv(&dep, &ctx).unwrap();
//! assert!(pv.amount().is_finite());
//! ```

use crate::instruments::deposit::types::Deposit;
use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Common deposit pricing engine providing core calculation methods.
pub struct DepositEngine;

impl DepositEngine {
    /// Calculates the present value of a simple deposit.
    ///
    /// # Arguments
    /// - `deposit` — Deposit instrument parameters
    /// - `context` — Market context containing the discount curve
    ///
    /// # Returns
    /// Net present value of the two cashflows as a `Money` amount.
    ///
    /// # Notes
    /// - Accrual uses the instrument's `day_count`
    /// - Discounting uses the curve's own day‑count via `df_on_date_curve`
    pub fn pv(deposit: &Deposit, context: &MarketContext) -> Result<Money> {
        let disc = context.get_ref::<DiscountCurve>(deposit.disc_id.clone())?;

        // Accrual factor (instrument basis)
        let yf = deposit
            .day_count
            .year_fraction(deposit.start, deposit.end, DayCountCtx::default())?
            .max(0.0);

        // Quoted simple rate (default to 0 when not provided)
        let r = deposit.quote_rate.unwrap_or(0.0);

        // Redemption amount at maturity
        let redemption = deposit.notional * (1.0 + r * yf);

        // Discount both legs using the curve's own time basis
        let df_start = disc.df_on_date_curve(deposit.start);
        let df_end = disc.df_on_date_curve(deposit.end);

        // PV = -Notional * DF(start) + Redemption * DF(end)
        let currency = deposit.notional.currency();
        let pv = -deposit.notional.amount() * df_start + redemption.amount() * df_end;
        Ok(Money::new(pv, currency))
    }
}
