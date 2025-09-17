//! Parameter grouping structures to reduce builder complexity.
//!
//! These structures group related parameters together, making builders more ergonomic
//! and reducing the number of individual optional fields.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::{id::IndexId, CurveId};
use finstack_core::F;

// FRAParams removed: use instrument builder directly.

// IRFutureParams removed: use instrument builder directly.

/// Market data references for instrument pricing.
///
/// Groups commonly used curve and surface identifiers that most instruments need.
#[derive(Clone, Debug)]
pub struct MarketRefs {
    /// Discount curve ID for present value calculations
    pub disc_id: CurveId,
    /// Optional forward curve ID (for floating rate instruments)
    pub fwd_id: Option<CurveId>,
    /// Optional volatility surface ID (for option instruments)
    pub vol_id: Option<CurveId>,
    /// Optional credit/hazard curve ID (for credit instruments)
    pub credit_id: Option<CurveId>,
}

impl MarketRefs {
    /// Create market refs with just discount curve (most common case)
    pub fn discount_only(disc_id: impl Into<CurveId>) -> Self {
        Self {
            disc_id: disc_id.into(),
            fwd_id: None,
            vol_id: None,
            credit_id: None,
        }
    }

    /// Create market refs for rates instruments (discount + forward)
    pub fn rates(disc_id: impl Into<CurveId>, fwd_id: impl Into<CurveId>) -> Self {
        Self {
            disc_id: disc_id.into(),
            fwd_id: Some(fwd_id.into()),
            vol_id: None,
            credit_id: None,
        }
    }

    /// Create market refs for options (discount + volatility)
    pub fn option(disc_id: impl Into<CurveId>, vol_id: impl Into<CurveId>) -> Self {
        Self {
            disc_id: disc_id.into(),
            fwd_id: None,
            vol_id: Some(vol_id.into()),
            credit_id: None,
        }
    }

    /// Create market refs for credit instruments (discount + credit)
    pub fn credit(disc_id: impl Into<CurveId>, credit_id: impl Into<CurveId>) -> Self {
        Self {
            disc_id: disc_id.into(),
            fwd_id: None,
            vol_id: None,
            credit_id: Some(credit_id.into()),
        }
    }

    /// Add forward curve
    pub fn with_forward(mut self, fwd_id: impl Into<CurveId>) -> Self {
        self.fwd_id = Some(fwd_id.into());
        self
    }

    /// Add volatility surface
    pub fn with_volatility(mut self, vol_id: impl Into<CurveId>) -> Self {
        self.vol_id = Some(vol_id.into());
        self
    }

    /// Add credit curve
    pub fn with_credit(mut self, credit_id: impl Into<CurveId>) -> Self {
        self.credit_id = Some(credit_id.into());
        self
    }
}

/// Instrument schedule parameters for payment dates and accruals.
///
/// Groups all the scheduling-related parameters that many instruments share.
/// This is distinct from cashflow::builder::ScheduleParams to avoid naming conflicts.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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
        Self::new(
            strike,
            expiry,
            crate::instruments::options::OptionType::Call,
        )
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
    pub fn with_settlement(
        mut self,
        settlement: crate::instruments::options::SettlementType,
    ) -> Self {
        self.settlement = settlement;
        self
    }
}

/// Equity underlying parameters for options.
///
/// Groups equity-specific market data references.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EquityUnderlyingParams {
    /// Underlying ticker/identifier
    pub ticker: String,
    /// Spot price identifier in market data
    pub spot_id: String,
    /// Optional dividend yield identifier
    pub dividend_yield_id: Option<String>,
    /// Contract size (shares per contract)
    pub contract_size: F,
}

impl EquityUnderlyingParams {
    /// Create equity underlying parameters
    pub fn new(ticker: impl Into<String>, spot_id: impl Into<String>) -> Self {
        Self {
            ticker: ticker.into(),
            spot_id: spot_id.into(),
            dividend_yield_id: None,
            contract_size: 1.0,
        }
    }

    /// Set dividend yield identifier
    pub fn with_dividend_yield(mut self, div_yield_id: impl Into<String>) -> Self {
        self.dividend_yield_id = Some(div_yield_id.into());
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

    /// ISDA standard senior unsecured parameters (40% recovery)
    pub fn senior_unsecured(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::new(reference_entity, 0.4, credit_id)
    }

    /// ISDA standard subordinated parameters (20% recovery)
    pub fn subordinated(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::new(reference_entity, 0.2, credit_id)
    }

    /// Standard investment grade parameters (40% recovery) - alias for senior_unsecured
    pub fn investment_grade(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::senior_unsecured(reference_entity, credit_id)
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
    pub fn new(commitment: Money, facility_expiry: Date, maturity: Date) -> Self {
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

/// Parameters for fixed income index underlying (for TRS and similar instruments)
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexUnderlyingParams {
    /// Index identifier (e.g., "CDX.IG", "HY.BOND.INDEX")
    pub index_id: IndexId,
    /// Base currency of the index
    pub base_currency: Currency,
    /// Optional yield curve/scalar identifier for carry calculation
    pub yield_id: Option<String>,
    /// Optional duration identifier for risk calculations
    pub duration_id: Option<String>,
    /// Optional convexity identifier for risk calculations
    pub convexity_id: Option<String>,
    /// Contract size (index units per contract, defaults to 1.0)
    pub contract_size: F,
}

impl IndexUnderlyingParams {
    /// Create index underlying parameters
    pub fn new(index_id: impl Into<String>, base_currency: Currency) -> Self {
        Self {
            index_id: IndexId::new(index_id),
            base_currency,
            yield_id: None,
            duration_id: None,
            convexity_id: None,
            contract_size: 1.0,
        }
    }

    /// Set yield identifier
    pub fn with_yield(mut self, yield_id: impl Into<String>) -> Self {
        self.yield_id = Some(yield_id.into());
        self
    }

    /// Set duration identifier
    pub fn with_duration(mut self, duration_id: impl Into<String>) -> Self {
        self.duration_id = Some(duration_id.into());
        self
    }

    /// Set convexity identifier
    pub fn with_convexity(mut self, convexity_id: impl Into<String>) -> Self {
        self.convexity_id = Some(convexity_id.into());
        self
    }

    /// Set contract size
    pub fn with_contract_size(mut self, size: F) -> Self {
        self.contract_size = size;
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

/// Option market parameters for pricing models.
///
/// Groups market data parameters commonly used in option pricing functions.
#[derive(Clone, Debug)]
pub struct OptionMarketParams {
    /// Current spot/forward price
    pub spot: F,
    /// Strike price
    pub strike: F,
    /// Risk-free rate
    pub rate: F,
    /// Volatility
    pub volatility: F,
    /// Time to expiry in years
    pub time_to_expiry: F,
    /// Dividend yield or cost of carry
    pub dividend_yield: F,
    /// Option type (Call/Put)
    pub option_type: crate::instruments::options::OptionType,
}

impl OptionMarketParams {
    /// Create option market parameters
    pub fn new(
        spot: F,
        strike: F,
        rate: F,
        volatility: F,
        time_to_expiry: F,
        dividend_yield: F,
        option_type: crate::instruments::options::OptionType,
    ) -> Self {
        Self {
            spot,
            strike,
            rate,
            volatility,
            time_to_expiry,
            dividend_yield,
            option_type,
        }
    }

    /// Create call option market parameters
    pub fn call(spot: F, strike: F, rate: F, volatility: F, time_to_expiry: F) -> Self {
        Self::new(
            spot,
            strike,
            rate,
            volatility,
            time_to_expiry,
            0.0, // No dividend yield
            crate::instruments::options::OptionType::Call,
        )
    }

    /// Create put option market parameters
    pub fn put(spot: F, strike: F, rate: F, volatility: F, time_to_expiry: F) -> Self {
        Self::new(
            spot,
            strike,
            rate,
            volatility,
            time_to_expiry,
            0.0, // No dividend yield
            crate::instruments::options::OptionType::Put,
        )
    }

    /// Set dividend yield
    pub fn with_dividend_yield(mut self, dividend_yield: F) -> Self {
        self.dividend_yield = dividend_yield;
        self
    }
}

/// SABR model parameters for volatility calibration.
///
/// Groups SABR model parameters used in calibration functions.
#[derive(Clone, Debug)]
pub struct SABRModelParams {
    /// Alpha parameter (ATM volatility)
    pub alpha: F,
    /// Nu parameter (volatility of volatility)
    pub nu: F,
    /// Rho parameter (correlation)
    pub rho: F,
    /// Beta parameter (CEV parameter, typically fixed)
    pub beta: F,
}

impl SABRModelParams {
    /// Create SABR model parameters
    pub fn new(alpha: F, nu: F, rho: F, beta: F) -> Self {
        Self { alpha, nu, rho, beta }
    }

    /// Standard equity SABR parameters (beta = 1.0)
    pub fn equity_standard(alpha: F, nu: F, rho: F) -> Self {
        Self::new(alpha, nu, rho, 1.0)
    }

    /// Standard rates SABR parameters (beta = 0.5)
    pub fn rates_standard(alpha: F, nu: F, rho: F) -> Self {
        Self::new(alpha, nu, rho, 0.5)
    }
}

/// Swaption-specific parameters.
///
/// Groups swaption parameters beyond basic option parameters.
#[derive(Clone, Debug)]
pub struct SwaptionParams {
    /// Notional amount
    pub notional: Money,
    /// Strike rate (fixed rate)
    pub strike_rate: F,
    /// Swaption expiry date
    pub expiry: Date,
    /// Underlying swap start date
    pub swap_start: Date,
    /// Underlying swap end date
    pub swap_end: Date,
    /// Payer/receiver side
    pub side: crate::instruments::fixed_income::irs::PayReceive,
}

impl SwaptionParams {
    /// Create payer swaption parameters
    pub fn payer(
        notional: Money,
        strike_rate: F,
        expiry: Date,
        swap_start: Date,
        swap_end: Date,
    ) -> Self {
        Self {
            notional,
            strike_rate,
            expiry,
            swap_start,
            swap_end,
            side: crate::instruments::fixed_income::irs::PayReceive::PayFixed,
        }
    }

    /// Create receiver swaption parameters
    pub fn receiver(
        notional: Money,
        strike_rate: F,
        expiry: Date,
        swap_start: Date,
        swap_end: Date,
    ) -> Self {
        Self {
            notional,
            strike_rate,
            expiry,
            swap_start,
            swap_end,
            side: crate::instruments::fixed_income::irs::PayReceive::ReceiveFixed,
        }
    }
}

/// Binomial tree configuration parameters.
///
/// Groups parameters for configuring binomial tree option pricing models.
#[derive(Clone, Debug)]
pub struct BinomialTreeParams {
    /// Number of time steps
    pub steps: usize,
    /// Optional early exercise steps for Bermudan options
    pub exercise_steps: Option<Vec<usize>>,
}

impl BinomialTreeParams {
    /// Create binomial tree parameters
    pub fn new(steps: usize) -> Self {
        Self {
            steps,
            exercise_steps: None,
        }
    }

    /// Add specific exercise steps for Bermudan options
    pub fn with_exercise_steps(mut self, exercise_steps: Vec<usize>) -> Self {
        self.exercise_steps = Some(exercise_steps);
        self
    }
}

/// Equity option specific parameters.
///
/// Groups parameters specific to equity options, including Money-denominated strike.
#[derive(Clone, Debug)]
pub struct EquityOptionParams {
    /// Strike price in Money (includes currency)
    pub strike: Money,
    /// Option expiry date
    pub expiry: Date,
    /// Option type (Call/Put)
    pub option_type: crate::instruments::options::OptionType,
    /// Exercise style (European/American/Bermudan)
    pub exercise_style: crate::instruments::options::ExerciseStyle,
    /// Settlement type (Cash/Physical)
    pub settlement: crate::instruments::options::SettlementType,
    /// Contract size (shares per contract)
    pub contract_size: F,
}

impl EquityOptionParams {
    /// Create new equity option parameters
    pub fn new(
        strike: Money,
        expiry: Date,
        option_type: crate::instruments::options::OptionType,
        contract_size: F,
    ) -> Self {
        Self {
            strike,
            expiry,
            option_type,
            exercise_style: crate::instruments::options::ExerciseStyle::European,
            settlement: crate::instruments::options::SettlementType::Physical,
            contract_size,
        }
    }

    /// Create European call option parameters
    pub fn european_call(strike: Money, expiry: Date, contract_size: F) -> Self {
        Self::new(
            strike,
            expiry,
            crate::instruments::options::OptionType::Call,
            contract_size,
        )
    }

    /// Create European put option parameters  
    pub fn european_put(strike: Money, expiry: Date, contract_size: F) -> Self {
        Self::new(
            strike,
            expiry,
            crate::instruments::options::OptionType::Put,
            contract_size,
        )
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
    pub fn with_settlement(
        mut self,
        settlement: crate::instruments::options::SettlementType,
    ) -> Self {
        self.settlement = settlement;
        self
    }
}

/// FX option specific parameters.
///
/// Groups parameters specific to FX options.
#[derive(Clone, Debug)]
pub struct FxOptionParams {
    /// Strike rate (FX rate)
    pub strike: F,
    /// Option expiry date
    pub expiry: Date,
    /// Option type (Call/Put)
    pub option_type: crate::instruments::options::OptionType,
    /// Exercise style (European/American/Bermudan)
    pub exercise_style: crate::instruments::options::ExerciseStyle,
    /// Settlement type (Cash/Physical)
    pub settlement: crate::instruments::options::SettlementType,
    /// Notional amount
    pub notional: Money,
}

impl FxOptionParams {
    /// Create new FX option parameters
    pub fn new(
        strike: F,
        expiry: Date,
        option_type: crate::instruments::options::OptionType,
        notional: Money,
    ) -> Self {
        Self {
            strike,
            expiry,
            option_type,
            exercise_style: crate::instruments::options::ExerciseStyle::European,
            settlement: crate::instruments::options::SettlementType::Physical,
            notional,
        }
    }

    /// Create European call option parameters
    pub fn european_call(strike: F, expiry: Date, notional: Money) -> Self {
        Self::new(
            strike,
            expiry,
            crate::instruments::options::OptionType::Call,
            notional,
        )
    }

    /// Create European put option parameters  
    pub fn european_put(strike: F, expiry: Date, notional: Money) -> Self {
        Self::new(
            strike,
            expiry,
            crate::instruments::options::OptionType::Put,
            notional,
        )
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
    pub fn with_settlement(
        mut self,
        settlement: crate::instruments::options::SettlementType,
    ) -> Self {
        self.settlement = settlement;
        self
    }
}

/// Interest rate option specific parameters.
///
/// Groups parameters specific to interest rate options (caps/floors).
#[derive(Clone, Debug)]
pub struct InterestRateOptionParams {
    /// Type of rate option (Cap/Floor)
    pub rate_option_type: crate::instruments::options::cap_floor::RateOptionType,
    /// Notional amount
    pub notional: Money,
    /// Strike rate
    pub strike_rate: F,
    /// Payment frequency
    pub frequency: Frequency,
    /// Day count convention
    pub day_count: DayCount,
}

impl InterestRateOptionParams {
    /// Create new interest rate option parameters
    pub fn new(
        rate_option_type: crate::instruments::options::cap_floor::RateOptionType,
        notional: Money,
        strike_rate: F,
        frequency: Frequency,
        day_count: DayCount,
    ) -> Self {
        Self {
            rate_option_type,
            notional,
            strike_rate,
            frequency,
            day_count,
        }
    }

    /// Create cap parameters
    pub fn cap(
        notional: Money,
        strike_rate: F,
        frequency: Frequency,
        day_count: DayCount,
    ) -> Self {
        Self::new(
            crate::instruments::options::cap_floor::RateOptionType::Cap,
            notional,
            strike_rate,
            frequency,
            day_count,
        )
    }

    /// Create floor parameters
    pub fn floor(
        notional: Money,
        strike_rate: F,
        frequency: Frequency,
        day_count: DayCount,
    ) -> Self {
        Self::new(
            crate::instruments::options::cap_floor::RateOptionType::Floor,
            notional,
            strike_rate,
            frequency,
            day_count,
        )
    }
}

/// CDS Index specific parameters.
///
/// Groups parameters specific to CDS indices.
#[derive(Clone, Debug)]
pub struct CDSIndexParams {
    /// Index name (e.g., "CDX.NA.IG", "iTraxx Europe")
    pub index_name: String,
    /// Index series number
    pub series: u16,
    /// Index version number
    pub version: u16,
    /// Fixed coupon in basis points
    pub fixed_coupon_bp: F,
}

impl CDSIndexParams {
    /// Create new CDS index parameters
    pub fn new(
        index_name: impl Into<String>,
        series: u16,
        version: u16,
        fixed_coupon_bp: F,
    ) -> Self {
        Self {
            index_name: index_name.into(),
            series,
            version,
            fixed_coupon_bp,
        }
    }

    /// Create CDX North America Investment Grade parameters
    pub fn cdx_na_ig(series: u16, version: u16, fixed_coupon_bp: F) -> Self {
        Self::new("CDX.NA.IG", series, version, fixed_coupon_bp)
    }

    /// Create CDX North America High Yield parameters
    pub fn cdx_na_hy(series: u16, version: u16, fixed_coupon_bp: F) -> Self {
        Self::new("CDX.NA.HY", series, version, fixed_coupon_bp)
    }

    /// Create iTraxx Europe parameters
    pub fn itraxx_europe(series: u16, version: u16, fixed_coupon_bp: F) -> Self {
        Self::new("iTraxx Europe", series, version, fixed_coupon_bp)
    }
}

/// Credit option specific parameters.
///
/// Groups parameters specific to credit options (options on CDS).
#[derive(Clone, Debug)]
pub struct CreditOptionParams {
    /// Strike spread in basis points
    pub strike_spread_bp: F,
    /// Option expiry date
    pub expiry: Date,
    /// Underlying CDS maturity date
    pub cds_maturity: Date,
    /// Notional amount
    pub notional: Money,
    /// Option type (Call/Put)
    pub option_type: crate::instruments::options::OptionType,
}

impl CreditOptionParams {
    /// Create new credit option parameters
    pub fn new(
        strike_spread_bp: F,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
        option_type: crate::instruments::options::OptionType,
    ) -> Self {
        Self {
            strike_spread_bp,
            expiry,
            cds_maturity,
            notional,
            option_type,
        }
    }

    /// Create credit call option parameters (option to buy protection)
    pub fn call(
        strike_spread_bp: F,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
    ) -> Self {
        Self::new(
            strike_spread_bp,
            expiry,
            cds_maturity,
            notional,
            crate::instruments::options::OptionType::Call,
        )
    }

    /// Create credit put option parameters (option to sell protection)
    pub fn put(
        strike_spread_bp: F,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
    ) -> Self {
        Self::new(
            strike_spread_bp,
            expiry,
            cds_maturity,
            notional,
            crate::instruments::options::OptionType::Put,
        )
    }
}

/// FX Swap specific parameters.
///
/// Groups parameters specific to FX swaps.
#[derive(Clone, Debug)]
pub struct FxSwapParams {
    /// Near leg date
    pub near_date: Date,
    /// Far leg date  
    pub far_date: Date,
    /// Base notional amount
    pub base_notional: Money,
    /// Optional near leg rate (if fixed)
    pub near_rate: Option<F>,
    /// Optional far leg rate (if fixed)
    pub far_rate: Option<F>,
}

impl FxSwapParams {
    /// Create new FX swap parameters
    pub fn new(
        near_date: Date,
        far_date: Date,
        base_notional: Money,
    ) -> Self {
        Self {
            near_date,
            far_date,
            base_notional,
            near_rate: None,
            far_rate: None,
        }
    }

    /// Set near leg rate
    pub fn with_near_rate(mut self, rate: F) -> Self {
        self.near_rate = Some(rate);
        self
    }

    /// Set far leg rate
    pub fn with_far_rate(mut self, rate: F) -> Self {
        self.far_rate = Some(rate);
        self
    }
}

/// Inflation-linked bond specific parameters.
///
/// Groups parameters specific to inflation-linked bonds.
#[derive(Clone, Debug)]
pub struct InflationLinkedBondParams {
    /// Notional amount
    pub notional: Money,
    /// Real coupon rate
    pub real_coupon: F,
    /// Issue date
    pub issue: Date,
    /// Maturity date
    pub maturity: Date,
    /// Base index value at issue
    pub base_index: F,
    /// Payment frequency
    pub frequency: Frequency,
    /// Day count convention
    pub day_count: DayCount,
}

impl InflationLinkedBondParams {
    /// Create new inflation-linked bond parameters
    pub fn new(
        notional: Money,
        real_coupon: F,
        issue: Date,
        maturity: Date,
        base_index: F,
        frequency: Frequency,
        day_count: DayCount,
    ) -> Self {
        Self {
            notional,
            real_coupon,
            issue,
            maturity,
            base_index,
            frequency,
            day_count,
        }
    }

    /// Create US TIPS parameters (semi-annual, Act/Act)
    pub fn tips(
        notional: Money,
        real_coupon: F,
        issue: Date,
        maturity: Date,
        base_index: F,
    ) -> Self {
        Self::new(
            notional,
            real_coupon,
            issue,
            maturity,
            base_index,
            Frequency::semi_annual(),
            DayCount::ActAct,
        )
    }

    /// Create UK linker parameters (semi-annual, Act/Act)
    pub fn uk_linker(
        notional: Money,
        real_coupon: F,
        issue: Date,
        maturity: Date,
        base_index: F,
    ) -> Self {
        Self::new(
            notional,
            real_coupon,
            issue,
            maturity,
            base_index,
            Frequency::semi_annual(),
            DayCount::ActAct,
        )
    }
}

/// CDS Tranche specific parameters.
///
/// Groups parameters specific to CDS tranches.
#[derive(Clone, Debug)]
pub struct CDSTrancheParams {
    /// Index name (e.g., "CDX.NA.IG", "iTraxx Europe")
    pub index_name: String,
    /// Index series
    pub series: u16,
    /// Attachment point as percentage
    pub attach_pct: F,
    /// Detachment point as percentage
    pub detach_pct: F,
    /// Notional amount
    pub notional: Money,
    /// Maturity date
    pub maturity: Date,
    /// Running coupon in basis points
    pub running_coupon_bp: F,
}

impl CDSTrancheParams {
    /// Create new CDS tranche parameters
    pub fn new(
        index_name: impl Into<String>,
        series: u16,
        attach_pct: F,
        detach_pct: F,
        notional: Money,
        maturity: Date,
        running_coupon_bp: F,
    ) -> Self {
        Self {
            index_name: index_name.into(),
            series,
            attach_pct,
            detach_pct,
            notional,
            maturity,
            running_coupon_bp,
        }
    }

    /// Create equity tranche parameters (0-3% typically)
    pub fn equity_tranche(
        index_name: impl Into<String>,
        series: u16,
        notional: Money,
        maturity: Date,
        running_coupon_bp: F,
    ) -> Self {
        Self::new(index_name, series, 0.0, 0.03, notional, maturity, running_coupon_bp)
    }

    /// Create mezzanine tranche parameters (3-7% typically)
    pub fn mezzanine_tranche(
        index_name: impl Into<String>,
        series: u16,
        notional: Money,
        maturity: Date,
        running_coupon_bp: F,
    ) -> Self {
        Self::new(index_name, series, 0.03, 0.07, notional, maturity, running_coupon_bp)
    }
}

/// Complete CDS construction parameters.
///
/// Groups all parameters needed for CDS construction to reduce argument count.
#[derive(Clone, Debug)]
pub struct CDSConstructionParams {
    /// Notional amount
    pub notional: Money,
    /// Protection side (pay/receive)
    pub side: crate::instruments::fixed_income::cds::PayReceive,
    /// CDS convention
    pub convention: crate::instruments::fixed_income::cds::CDSConvention,
    /// Spread in basis points
    pub spread_bp: F,
}

impl CDSConstructionParams {
    /// Create new CDS construction parameters
    pub fn new(
        notional: Money,
        side: crate::instruments::fixed_income::cds::PayReceive,
        convention: crate::instruments::fixed_income::cds::CDSConvention,
        spread_bp: F,
    ) -> Self {
        Self {
            notional,
            side,
            convention,
            spread_bp,
        }
    }

    /// Create standard protection buyer parameters
    pub fn buy_protection(
        notional: Money,
        spread_bp: F,
    ) -> Self {
        Self::new(
            notional,
            crate::instruments::fixed_income::cds::PayReceive::PayProtection,
            crate::instruments::fixed_income::cds::CDSConvention::IsdaNa,
            spread_bp,
        )
    }

    /// Create standard protection seller parameters
    pub fn sell_protection(
        notional: Money,
        spread_bp: F,
    ) -> Self {
        Self::new(
            notional,
            crate::instruments::fixed_income::cds::PayReceive::ReceiveProtection,
            crate::instruments::fixed_income::cds::CDSConvention::IsdaNa,
            spread_bp,
        )
    }
}

/// Complete CDS Index construction parameters.
///
/// Groups all parameters needed for CDS Index construction to reduce argument count.
#[derive(Clone, Debug)]
pub struct CDSIndexConstructionParams {
    /// Notional amount
    pub notional: Money,
    /// Protection side (pay/receive)
    pub side: crate::instruments::fixed_income::cds::PayReceive,
    /// CDS convention
    pub convention: crate::instruments::fixed_income::cds::CDSConvention,
}

impl CDSIndexConstructionParams {
    /// Create new CDS index construction parameters
    pub fn new(
        notional: Money,
        side: crate::instruments::fixed_income::cds::PayReceive,
        convention: crate::instruments::fixed_income::cds::CDSConvention,
    ) -> Self {
        Self {
            notional,
            side,
            convention,
        }
    }

    /// Create standard protection buyer parameters
    pub fn buy_protection(notional: Money) -> Self {
        Self::new(
            notional,
            crate::instruments::fixed_income::cds::PayReceive::PayProtection,
            crate::instruments::fixed_income::cds::CDSConvention::IsdaNa,
        )
    }
}
