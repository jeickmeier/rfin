//! Parameter grouping structures to reduce builder complexity.
//!
//! These structures group related parameters together, making builders more ergonomic
//! and reducing the number of individual optional fields.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::F;

/// Market data references for instrument pricing.
///
/// Groups commonly used curve and surface identifiers that most instruments need.
#[derive(Clone, Debug)]
pub struct MarketRefs {
    /// Discount curve ID for present value calculations
    pub disc_id: &'static str,
    /// Optional forward curve ID (for floating rate instruments)
    pub fwd_id: Option<&'static str>,
    /// Optional volatility surface ID (for option instruments)
    pub vol_id: Option<&'static str>,
    /// Optional credit/hazard curve ID (for credit instruments)
    pub credit_id: Option<&'static str>,
}

impl MarketRefs {
    /// Create market refs with just discount curve (most common case)
    pub fn discount_only(disc_id: &'static str) -> Self {
        Self {
            disc_id,
            fwd_id: None,
            vol_id: None,
            credit_id: None,
        }
    }

    /// Create market refs for rates instruments (discount + forward)
    pub fn rates(disc_id: &'static str, fwd_id: &'static str) -> Self {
        Self {
            disc_id,
            fwd_id: Some(fwd_id),
            vol_id: None,
            credit_id: None,
        }
    }

    /// Create market refs for options (discount + volatility)
    pub fn option(disc_id: &'static str, vol_id: &'static str) -> Self {
        Self {
            disc_id,
            fwd_id: None,
            vol_id: Some(vol_id),
            credit_id: None,
        }
    }

    /// Create market refs for credit instruments (discount + credit)
    pub fn credit(disc_id: &'static str, credit_id: &'static str) -> Self {
        Self {
            disc_id,
            fwd_id: None,
            vol_id: None,
            credit_id: Some(credit_id),
        }
    }

    /// Add forward curve
    pub fn with_forward(mut self, fwd_id: &'static str) -> Self {
        self.fwd_id = Some(fwd_id);
        self
    }

    /// Add volatility surface
    pub fn with_volatility(mut self, vol_id: &'static str) -> Self {
        self.vol_id = Some(vol_id);
        self
    }

    /// Add credit curve
    pub fn with_credit(mut self, credit_id: &'static str) -> Self {
        self.credit_id = Some(credit_id);
        self
    }
}

/// Instrument schedule parameters for payment dates and accruals.
///
/// Groups all the scheduling-related parameters that many instruments share.
/// This is distinct from cashflow::builder::ScheduleParams to avoid naming conflicts.
#[derive(Clone, Debug)]
pub struct InstrumentScheduleParams {
    /// Payment frequency
    pub frequency: Frequency,
    /// Day count convention for accruals
    pub day_count: DayCount,
    /// Business day convention for payment date adjustments
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments
    pub calendar_id: Option<&'static str>,
    /// Stub period handling
    pub stub: StubKind,
}

impl InstrumentScheduleParams {
    /// Standard quarterly schedule with Act/360 and Following BDC
    pub fn quarterly_act360() -> Self {
        Self {
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        }
    }

    /// Standard semi-annual schedule with 30/360 and ModifiedFollowing BDC
    pub fn semiannual_30360() -> Self {
        Self {
            frequency: Frequency::semi_annual(),
            day_count: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
        }
    }

    /// Standard annual schedule with Act/Act and Following BDC
    pub fn annual_actact() -> Self {
        Self {
            frequency: Frequency::annual(),
            day_count: DayCount::ActAct,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        }
    }

    /// USD market standard (quarterly, Act/360, ModifiedFollowing)
    pub fn usd_standard() -> Self {
        Self {
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("USD"),
            stub: StubKind::None,
        }
    }

    /// EUR market standard (semi-annual, 30/360, ModifiedFollowing)
    pub fn eur_standard() -> Self {
        Self {
            frequency: Frequency::semi_annual(),
            day_count: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("EUR"),
            stub: StubKind::None,
        }
    }

    /// Convert to cashflow builder ScheduleParams
    pub fn to_cashflow_schedule_params(&self) -> crate::cashflow::builder::ScheduleParams {
        crate::cashflow::builder::ScheduleParams {
            freq: self.frequency,
            dc: self.day_count,
            bdc: self.bdc,
            calendar_id: self.calendar_id,
            stub: self.stub,
        }
    }
}

/// Date range specification for instruments.
///
/// Simplifies specifying start and end dates for legs, periods, and option terms.
#[derive(Clone, Debug)]
pub struct DateRange {
    /// Start date
    pub start: Date,
    /// End date
    pub end: Date,
}

impl DateRange {
    /// Create a new date range
    pub fn new(start: Date, end: Date) -> Self {
        Self { start, end }
    }

    /// Create date range from start date and tenor in years
    pub fn from_tenor(start: Date, tenor_years: F) -> Self {
        let end = start + time::Duration::days((tenor_years * 365.25) as i64);
        Self { start, end }
    }

    /// Create date range from start date and number of months
    pub fn from_months(start: Date, months: i32) -> Self {
        let end = finstack_core::dates::add_months(start, months);
        Self { start, end }
    }

    /// Duration in years using Act/365F
    pub fn years(&self) -> F {
        DayCount::Act365F
            .year_fraction(
                self.start,
                self.end,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0)
    }
}

/// Option-specific parameters.
///
/// Groups parameters common to all option instruments.
#[derive(Clone, Debug)]
pub struct OptionParams {
    /// Strike price/rate
    pub strike: F,
    /// Option expiry date
    pub expiry: Date,
    /// Option type (Call/Put)
    pub option_type: crate::instruments::options::OptionType,
    /// Exercise style (European/American/Bermudan)
    pub exercise_style: crate::instruments::options::ExerciseStyle,
    /// Settlement type (Cash/Physical)
    pub settlement: crate::instruments::options::SettlementType,
}

impl OptionParams {
    /// Create new option parameters
    pub fn new(
        strike: F,
        expiry: Date,
        option_type: crate::instruments::options::OptionType,
    ) -> Self {
        Self {
            strike,
            expiry,
            option_type,
            exercise_style: crate::instruments::options::ExerciseStyle::European,
            settlement: crate::instruments::options::SettlementType::Cash,
        }
    }

    /// Create European call option parameters
    pub fn european_call(strike: F, expiry: Date) -> Self {
        Self::new(strike, expiry, crate::instruments::options::OptionType::Call)
    }

    /// Create European put option parameters
    pub fn european_put(strike: F, expiry: Date) -> Self {
        Self::new(strike, expiry, crate::instruments::options::OptionType::Put)
    }

    /// Set exercise style
    pub fn with_exercise_style(
        mut self,
        style: crate::instruments::options::ExerciseStyle,
    ) -> Self {
        self.exercise_style = style;
        self
    }

    /// Set settlement type
    pub fn with_settlement(mut self, settlement: crate::instruments::options::SettlementType) -> Self {
        self.settlement = settlement;
        self
    }
}

/// Equity underlying parameters for options.
///
/// Groups equity-specific market data references.
#[derive(Clone, Debug)]
pub struct EquityUnderlyingParams {
    /// Underlying ticker/identifier
    pub ticker: String,
    /// Spot price identifier in market data
    pub spot_id: &'static str,
    /// Optional dividend yield identifier
    pub dividend_yield_id: Option<&'static str>,
    /// Contract size (shares per contract)
    pub contract_size: F,
}

impl EquityUnderlyingParams {
    /// Create equity underlying parameters
    pub fn new(ticker: impl Into<String>, spot_id: &'static str) -> Self {
        Self {
            ticker: ticker.into(),
            spot_id,
            dividend_yield_id: None,
            contract_size: 1.0,
        }
    }

    /// Set dividend yield identifier
    pub fn with_dividend_yield(mut self, div_yield_id: &'static str) -> Self {
        self.dividend_yield_id = Some(div_yield_id);
        self
    }

    /// Set contract size
    pub fn with_contract_size(mut self, size: F) -> Self {
        self.contract_size = size;
        self
    }
}

/// FX underlying parameters for FX options and swaps.
#[derive(Clone, Debug)]
pub struct FxUnderlyingParams {
    /// Base currency (being priced)
    pub base_currency: Currency,
    /// Quote currency (pricing currency)
    pub quote_currency: Currency,
    /// Domestic discount curve ID (quote currency)
    pub domestic_disc_id: &'static str,
    /// Foreign discount curve ID (base currency)
    pub foreign_disc_id: &'static str,
}

impl FxUnderlyingParams {
    /// Create FX underlying parameters
    pub fn new(
        base_currency: Currency,
        quote_currency: Currency,
        domestic_disc_id: &'static str,
        foreign_disc_id: &'static str,
    ) -> Self {
        Self {
            base_currency,
            quote_currency,
            domestic_disc_id,
            foreign_disc_id,
        }
    }

    /// Standard USD/EUR pair
    pub fn usd_eur() -> Self {
        Self::new(Currency::EUR, Currency::USD, "USD-OIS", "EUR-OIS")
    }

    /// Standard GBP/USD pair
    pub fn gbp_usd() -> Self {
        Self::new(Currency::GBP, Currency::USD, "USD-OIS", "GBP-OIS")
    }
}

/// Credit parameters for CDS and credit options.
#[derive(Clone, Debug)]
pub struct CreditParams {
    /// Reference entity name
    pub reference_entity: String,
    /// Recovery rate assumption
    pub recovery_rate: F,
    /// Credit/hazard curve identifier
    pub credit_id: &'static str,
}

impl CreditParams {
    /// Create credit parameters
    pub fn new(
        reference_entity: impl Into<String>,
        recovery_rate: F,
        credit_id: &'static str,
    ) -> Self {
        Self {
            reference_entity: reference_entity.into(),
            recovery_rate,
            credit_id,
        }
    }

    /// Standard investment grade parameters (40% recovery)
    pub fn investment_grade(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::new(reference_entity, 0.4, credit_id)
    }

    /// High yield parameters (30% recovery)
    pub fn high_yield(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::new(reference_entity, 0.3, credit_id)
    }
}

/// Pricing overrides for market-quoted instruments.
///
/// Optional parameters that override model pricing with market quotes.
#[derive(Clone, Debug, Default)]
pub struct PricingOverrides {
    /// Quoted clean price (for bonds)
    pub quoted_clean_price: Option<F>,
    /// Implied volatility (overrides vol surface)
    pub implied_volatility: Option<F>,
    /// Quoted spread (for credit instruments)
    pub quoted_spread_bp: Option<F>,
    /// Upfront payment (for CDS, convertibles)
    pub upfront_payment: Option<Money>,
}

impl PricingOverrides {
    /// Create empty pricing overrides
    pub fn none() -> Self {
        Self::default()
    }

    /// Set quoted clean price
    pub fn with_clean_price(mut self, price: F) -> Self {
        self.quoted_clean_price = Some(price);
        self
    }

    /// Set implied volatility
    pub fn with_implied_vol(mut self, vol: F) -> Self {
        self.implied_volatility = Some(vol);
        self
    }

    /// Set quoted spread
    pub fn with_spread_bp(mut self, spread_bp: F) -> Self {
        self.quoted_spread_bp = Some(spread_bp);
        self
    }

    /// Set upfront payment
    pub fn with_upfront(mut self, upfront: Money) -> Self {
        self.upfront_payment = Some(upfront);
        self
    }
}

/// Loan facility parameters for various loan types.
///
/// Groups parameters common to all loan facilities.
#[derive(Clone, Debug)]
pub struct LoanFacilityParams {
    /// Total facility/commitment amount
    pub commitment: Money,
    /// Currently drawn amount
    pub drawn_amount: Option<Money>,
    /// Facility expiry date (when draws are no longer allowed)
    pub facility_expiry: Date,
    /// Final maturity date
    pub maturity: Date,
    /// Borrower entity identifier
    pub borrower: Option<String>,
}

impl LoanFacilityParams {
    /// Create loan facility parameters
    pub fn new(
        commitment: Money,
        facility_expiry: Date,
        maturity: Date,
    ) -> Self {
        Self {
            commitment,
            drawn_amount: None,
            facility_expiry,
            maturity,
            borrower: None,
        }
    }

    /// Set initial drawn amount
    pub fn with_drawn_amount(mut self, amount: Money) -> Self {
        self.drawn_amount = Some(amount);
        self
    }

    /// Set borrower entity
    pub fn with_borrower(mut self, borrower: impl Into<String>) -> Self {
        self.borrower = Some(borrower.into());
        self
    }

    /// Create a fully drawn term loan (commitment = drawn)
    pub fn term_loan(amount: Money, maturity: Date) -> Self {
        Self {
            commitment: amount,
            drawn_amount: Some(amount),
            facility_expiry: maturity, // No additional draws
            maturity,
            borrower: None,
        }
    }

    /// Create an undrawn revolving facility
    pub fn revolver(commitment: Money, facility_expiry: Date, maturity: Date) -> Self {
        Self {
            commitment,
            drawn_amount: None, // Starts undrawn
            facility_expiry,
            maturity,
            borrower: None,
        }
    }
}

/// Fee structure parameters for loans.
///
/// Groups fee-related parameters that many loan types share.
#[derive(Clone, Debug, Default)]
pub struct LoanFeeParams {
    /// Commitment fee rate (annual) on undrawn amounts
    pub commitment_fee_rate: F,
    /// Optional ticking fee rate (annual) on undrawn amounts
    pub ticking_fee_rate: Option<F>,
    /// Optional origination fee (one-time)
    pub origination_fee: Option<Money>,
    /// Optional amendment/modification fees
    pub amendment_fees: Vec<(Date, Money)>,
}

impl LoanFeeParams {
    /// Create standard fee parameters with commitment fee
    pub fn standard(commitment_fee_rate: F) -> Self {
        Self {
            commitment_fee_rate,
            ticking_fee_rate: None,
            origination_fee: None,
            amendment_fees: Vec::new(),
        }
    }

    /// Add ticking fee
    pub fn with_ticking_fee(mut self, rate: F) -> Self {
        self.ticking_fee_rate = Some(rate);
        self
    }

    /// Add origination fee
    pub fn with_origination_fee(mut self, fee: Money) -> Self {
        self.origination_fee = Some(fee);
        self
    }
}

/// Helper functions for working with parameter groups in builders.
/// 
/// These functions assist in converting parameter groups to final instrument specifications.
pub fn validate_currency_consistency(amounts: &[Money]) -> finstack_core::Result<()> {
    if amounts.is_empty() {
        return Ok(());
    }
    
    let expected_currency = amounts[0].currency();
    for amount in amounts.iter().skip(1) {
        if amount.currency() != expected_currency {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: expected_currency,
                actual: amount.currency(),
            });
        }
    }
    Ok(())
}
