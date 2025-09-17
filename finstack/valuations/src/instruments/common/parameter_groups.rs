//! Parameter grouping structures to reduce builder complexity.
//!
//! These structures group related parameters together, making builders more ergonomic
//! and reducing the number of individual optional fields.

use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::CurveId;
use finstack_core::F;

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

// OptionParams removed: inlined in option constructors.

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
