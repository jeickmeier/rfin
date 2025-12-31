//! Deterministic cashflow discounting pricer for term loans.
//!
//! This module provides the standard pricer for term loans using:
//! - Complete cashflow generation (DDTL draws, interest, amortization, PIK, fees)
//! - Discounting to present value using the instrument's discount curve
//! - PIK interest capitalization (excluded from PV, increases outstanding)
//!
//! # Pricing Methodology
//!
//! The pricer follows these steps:
//! 1. Generate full internal cashflow schedule via [`generate_cashflows`]
//! 2. Filter to cash flows only (exclude PIK capitalization)
//! 3. Discount flows using the discount curve anchored to `as_of` date
//! 4. Return present value in loan currency
//!
//! # PIK Treatment
//!
//! Payment-in-kind (PIK) interest is:
//! - Capitalized into outstanding principal (affects principal path)
//! - **Excluded from PV calculation** (not a cash flow to holder)
//! - Reflected in final redemption amount
//!
//! This follows institutional market practice where PIK increases debt balance
//! rather than generating cash flows.
//!
//! # Examples
//!
//! ```rust
//! use finstack_valuations::instruments::term_loan::TermLoan;
//! use finstack_valuations::instruments::term_loan::pricing::TermLoanDiscountingPricer;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let loan = TermLoan::example();
//! let market = MarketContext::new();
//! let as_of = Date::from_calendar_date(2025, Month::January, 15)?;
//!
//! // Price using deterministic discounting
//! // let pv = TermLoanDiscountingPricer::price(&loan, &market, as_of)?;
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//!
//! - [`generate_cashflows`] for cashflow generation details
//! - [`super::types::TermLoan`] for the instrument type

use crate::instruments::common::traits::Instrument;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

use super::cashflows::generate_cashflows;
use super::types::TermLoan;

/// Term loan pricer using deterministic cashflow discounting.
///
/// Prices term loans by generating complete cashflow schedules and discounting
/// to present value. Handles all term loan features including DDTL, PIK, covenants,
/// and amortization.
///
/// # Pricing Method
///
/// Uses full-fidelity cashflow generation with:
/// - Time-dependent outstanding principal (DDTL draws, amortization, PIK)
/// - Floating rate projection with floors/caps
/// - Covenant-driven margin adjustments
/// - Fee accruals (commitment, usage, upfront)
/// - PIK capitalization (excluded from PV)
///
/// # Thread Safety
///
/// This pricer is stateless and thread-safe (`Send + Sync`).
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::term_loan::TermLoan;
/// use finstack_valuations::instruments::term_loan::pricing::TermLoanDiscountingPricer;
/// use finstack_valuations::pricer::Pricer;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pricer = TermLoanDiscountingPricer;
/// let loan = TermLoan::example();
/// let market = MarketContext::new();
/// let as_of = Date::from_calendar_date(2025, Month::January, 15)?;
///
/// // Price using the Pricer trait
/// // let result = pricer.price_dyn(&loan, &market, as_of)?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct TermLoanDiscountingPricer;

impl TermLoanDiscountingPricer {
    /// Price a term loan using deterministic cashflows and discounting.
    ///
    /// Generates complete cashflow schedule including DDTL draws, interest, fees,
    /// amortization, and redemptions, then discounts cash flows to present value.
    ///
    /// # Arguments
    ///
    /// * `loan` - The term loan instrument to price
    /// * `market` - Market context with discount curves and forward rate data
    /// * `as_of` - Valuation date (cashflows before this date are excluded)
    ///
    /// # Returns
    ///
    /// Present value of the loan in the loan's currency. Represents the fair value
    /// to a holder on the valuation date.
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - Discount curve is not found in market context
    /// - Cashflow generation fails (invalid dates, schedule errors)
    /// - Currency mismatch in flows
    /// - Forward rate projection fails for floating rate loans
    ///
    /// # PIK Treatment
    ///
    /// PIK (payment-in-kind) interest is capitalized into outstanding principal and excluded
    /// from PV calculation. Only cash flows are discounted:
    /// - Coupons (Fixed, FloatReset, Stub)
    /// - Amortization
    /// - Redemptions (Notional)
    /// - Fees
    ///
    /// PIK capitalization affects the outstanding principal path and is reflected in the
    /// final redemption amount.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use finstack_valuations::instruments::term_loan::TermLoan;
    /// use finstack_valuations::instruments::term_loan::pricing::TermLoanDiscountingPricer;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let loan = TermLoan::example();
    /// let market = MarketContext::new();
    /// let as_of = Date::from_calendar_date(2025, Month::January, 15)?;
    ///
    /// // let pv = TermLoanDiscountingPricer::price(&loan, &market, as_of)?;
    /// # Ok(())
    /// # }
    /// ```
    pub(crate) fn price(
        loan: &TermLoan,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        use finstack_core::cashflow::CFKind;

        // Build full cashflow schedule
        let schedule = generate_cashflows(loan, market, as_of)?;

        // Retrieve discount curve and discount flows to `as_of` using date-based DF mapping.
        // This ensures valuation is anchored on the valuation date rather than the curve's
        // internal base date.
        let disc = market.get_discount(loan.discount_curve_id.as_str())?;

        // Filter flows: exclude PIK (capitalized interest) from PV
        // PIK increases outstanding and is repaid via principal redemption
        let flows: Vec<(finstack_core::dates::Date, Money)> = schedule
            .flows
            .iter()
            .filter(|cf| cf.kind != CFKind::PIK)
            .map(|cf| (cf.date, cf.amount))
            .collect();

        crate::instruments::common::discountable::npv_by_date(disc.as_ref(), as_of, &flows)
    }
}

impl Pricer for TermLoanDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::TermLoan, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let loan = instrument
            .as_any()
            .downcast_ref::<TermLoan>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::TermLoan, instrument.key())
            })?;

        // Use the provided as_of date for valuation
        let pv = Self::price(loan, market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(loan.id(), as_of, pv))
    }
}
