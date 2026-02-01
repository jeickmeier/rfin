//! NDF types and implementations.
//!
//! Defines the `Ndf` instrument for non-deliverable forward contracts on
//! restricted currencies. Supports both pre-fixing (forward rate estimation)
//! and post-fixing (observed rate) valuation modes.

use crate::instruments::common::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Quote convention for NDF contract rates.
///
/// NDFs can be quoted in two conventions depending on the market:
///
/// # BasePerSettlement (default)
///
/// Rate is quoted as units of base currency per one unit of settlement currency.
/// Example: USD/CNY = 7.25 means 7.25 CNY per 1 USD.
///
/// Settlement formula:
/// ```text
/// Settlement = Notional_base × (1/F_contract - 1/F_fixing)
/// ```
///
/// This is the standard convention for most Asian NDF markets (CNY, KRW, INR, etc.)
/// where the restricted currency is the base and USD is the settlement currency.
///
/// # SettlementPerBase
///
/// Rate is quoted as units of settlement currency per one unit of base currency.
/// Example: CNY/USD = 0.138 means 0.138 USD per 1 CNY.
///
/// Settlement formula:
/// ```text
/// Settlement = Notional_base × (F_fixing - F_contract)
/// ```
///
/// This is less common but may be used in some markets or for consistency with
/// other FX instruments that quote in this direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NdfQuoteConvention {
    /// Rate quoted as base currency per settlement currency (e.g., 7.25 CNY per USD).
    /// Settlement = Notional_base × (1/F_contract - 1/F_fixing)
    #[default]
    BasePerSettlement,
    /// Rate quoted as settlement currency per base currency (e.g., 0.138 USD per CNY).
    /// Settlement = Notional_base × (F_fixing - F_contract)
    SettlementPerBase,
}

impl std::fmt::Display for NdfQuoteConvention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NdfQuoteConvention::BasePerSettlement => write!(f, "base_per_settlement"),
            NdfQuoteConvention::SettlementPerBase => write!(f, "settlement_per_base"),
        }
    }
}

impl std::str::FromStr for NdfQuoteConvention {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "base_per_settlement" | "basepersettlement" | "bps" => {
                Ok(NdfQuoteConvention::BasePerSettlement)
            }
            "settlement_per_base" | "settlementperbase" | "spb" => {
                Ok(NdfQuoteConvention::SettlementPerBase)
            }
            other => Err(format!("Unknown NDF quote convention: {}", other)),
        }
    }
}

/// Official NDF fixing source/benchmark.
///
/// NDF settlements reference official fixing rates published by central banks
/// or designated fixing bodies. Using the correct fixing source is critical
/// for proper settlement calculations.
///
/// # Market Standards
///
/// | Currency | Fixing Source | Publisher | Settlement |
/// |----------|---------------|-----------|------------|
/// | CNY | PBOC | People's Bank of China | USD T+2 |
/// | CNH | CNHFIX | Treasury Markets Association (HK) | USD T+2 |
/// | INR | RBI | Reserve Bank of India | USD T+2 |
/// | KRW | KFTC | Korea Financial Telecommunications | USD T+1 |
/// | BRL | PTAX | Banco Central do Brasil | USD T+2 |
/// | TWD | TAIFX | Taipei Forex Inc. | USD T+2 |
/// | PHP | PHP BVAL | Bankers Association of the Philippines | USD T+1 |
/// | IDR | JISDOR | Bank Indonesia | USD T+2 |
/// | MYR | BNM | Bank Negara Malaysia | USD T+2 |
///
/// # Example
///
/// ```rust
/// use finstack_valuations::instruments::fx::ndf::{Ndf, NdfFixingSource, NdfQuoteConvention};
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use finstack_core::money::Money;
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let ndf = Ndf::builder()
///     .id(InstrumentId::new("USDCNY-NDF"))
///     .base_currency(Currency::CNY)
///     .settlement_currency(Currency::USD)
///     .fixing_date(Date::from_calendar_date(2025, Month::March, 13).unwrap())
///     .maturity_date(Date::from_calendar_date(2025, Month::March, 15).unwrap())
///     .notional(Money::new(10_000_000.0, Currency::CNY))
///     .contract_rate(7.25)
///     .settlement_curve_id(CurveId::new("USD-OIS"))
///     .quote_convention(NdfQuoteConvention::BasePerSettlement)
///     .fixing_source_enum_opt(Some(NdfFixingSource::Pboc))
///     .build()
///     .expect("Valid NDF");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NdfFixingSource {
    /// PBOC - People's Bank of China CNY/USD fixing.
    /// Published daily at 9:15 AM Beijing time.
    Pboc,
    /// CNHFIX - Treasury Markets Association CNH/USD fixing (offshore CNY).
    /// Published daily at 11:15 AM Hong Kong time.
    Cnhfix,
    /// RBI - Reserve Bank of India INR/USD reference rate.
    /// Published daily around 1:30 PM Mumbai time.
    Rbi,
    /// KFTC - Korea Financial Telecommunications and Clearings Institute.
    /// KRW/USD fixing published at 3:30 PM Seoul time.
    Kftc,
    /// PTAX - Banco Central do Brasil BRL/USD reference rate.
    /// Published daily, settlement uses PTAX 800 (closing rate).
    Ptax,
    /// TAIFX - Taipei Forex Inc. TWD/USD fixing.
    /// Published daily at 11:00 AM Taipei time.
    Taifx,
    /// BVAL - Bankers Association of the Philippines PHP/USD reference rate.
    /// Also known as PHP BVAL or PDEx.
    PhpBval,
    /// JISDOR - Jakarta Interbank Spot Dollar Rate (Bank Indonesia).
    /// IDR/USD fixing published daily at 10:00 AM Jakarta time.
    Jisdor,
    /// BNM - Bank Negara Malaysia MYR/USD fixing.
    /// Published daily at 3:30 PM Kuala Lumpur time.
    Bnm,
    /// Custom or other fixing source not covered by the enum.
    Other,
}

impl NdfFixingSource {
    /// Get the typical currency for this fixing source.
    ///
    /// Note: CNHFIX returns CNY since offshore CNY (CNH) is typically
    /// represented as CNY in most currency enums.
    pub fn typical_currency(&self) -> Option<Currency> {
        match self {
            NdfFixingSource::Pboc => Some(Currency::CNY),
            // CNHFIX is for offshore CNY, typically mapped to CNY
            NdfFixingSource::Cnhfix => Some(Currency::CNY),
            NdfFixingSource::Rbi => Some(Currency::INR),
            NdfFixingSource::Kftc => Some(Currency::KRW),
            NdfFixingSource::Ptax => Some(Currency::BRL),
            NdfFixingSource::Taifx => Some(Currency::TWD),
            NdfFixingSource::PhpBval => Some(Currency::PHP),
            NdfFixingSource::Jisdor => Some(Currency::IDR),
            NdfFixingSource::Bnm => Some(Currency::MYR),
            NdfFixingSource::Other => None,
        }
    }

    /// Get the typical fixing offset (business days before settlement).
    /// Most NDFs fix T-2, but some (KRW, PHP) fix T-1.
    pub fn typical_fixing_offset(&self) -> i64 {
        match self {
            NdfFixingSource::Kftc => 1,    // KRW fixes T-1
            NdfFixingSource::PhpBval => 1, // PHP fixes T-1
            _ => 2,                        // Most currencies fix T-2
        }
    }
}

impl std::fmt::Display for NdfFixingSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NdfFixingSource::Pboc => write!(f, "PBOC"),
            NdfFixingSource::Cnhfix => write!(f, "CNHFIX"),
            NdfFixingSource::Rbi => write!(f, "RBI"),
            NdfFixingSource::Kftc => write!(f, "KFTC"),
            NdfFixingSource::Ptax => write!(f, "PTAX"),
            NdfFixingSource::Taifx => write!(f, "TAIFX"),
            NdfFixingSource::PhpBval => write!(f, "PHP_BVAL"),
            NdfFixingSource::Jisdor => write!(f, "JISDOR"),
            NdfFixingSource::Bnm => write!(f, "BNM"),
            NdfFixingSource::Other => write!(f, "OTHER"),
        }
    }
}

impl std::str::FromStr for NdfFixingSource {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "PBOC" => Ok(NdfFixingSource::Pboc),
            "CNHFIX" | "CNH_FIX" => Ok(NdfFixingSource::Cnhfix),
            "RBI" => Ok(NdfFixingSource::Rbi),
            "KFTC" => Ok(NdfFixingSource::Kftc),
            "PTAX" => Ok(NdfFixingSource::Ptax),
            "TAIFX" => Ok(NdfFixingSource::Taifx),
            "PHP_BVAL" | "PHPBVAL" | "BVAL" | "PDEX" => Ok(NdfFixingSource::PhpBval),
            "JISDOR" => Ok(NdfFixingSource::Jisdor),
            "BNM" => Ok(NdfFixingSource::Bnm),
            "OTHER" => Ok(NdfFixingSource::Other),
            other => Err(format!("Unknown NDF fixing source: {}", other)),
        }
    }
}

/// Non-Deliverable Forward (NDF) instrument.
///
/// Represents a cash-settled forward contract on a restricted currency pair.
/// The position is long base currency (restricted) and short settlement currency.
///
/// # Quote Convention
///
/// NDFs support two quote conventions via the `quote_convention` field:
///
/// - **BasePerSettlement** (default): Rate quoted as base per settlement (e.g., 7.25 CNY/USD)
/// - **SettlementPerBase**: Rate quoted as settlement per base (e.g., 0.138 USD/CNY)
///
/// See [`NdfQuoteConvention`] for details on the settlement formulas.
///
/// # Pricing
///
/// ## Pre-Fixing (fixing_rate = None)
/// Forward rate is estimated via covered interest rate parity or fallback.
///
/// ## Post-Fixing (fixing_rate = Some)
/// Uses the observed fixing rate for settlement calculation.
///
/// The settlement formula depends on `quote_convention`:
///
/// **BasePerSettlement:**
/// ```text
/// Settlement = Notional_base × (1/F_contract - 1/F_fixing)
/// PV = Settlement × DF_settlement(T)
/// ```
///
/// **SettlementPerBase:**
/// ```text
/// Settlement = Notional_base × (F_fixing - F_contract)
/// PV = Settlement × DF_settlement(T)
/// ```
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fx::ndf::{Ndf, NdfQuoteConvention};
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use finstack_core::money::Money;
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let ndf = Ndf::builder()
///     .id(InstrumentId::new("USDCNY-NDF-3M"))
///     .base_currency(Currency::CNY)
///     .settlement_currency(Currency::USD)
///     .fixing_date(Date::from_calendar_date(2025, Month::March, 13).unwrap())
///     .maturity_date(Date::from_calendar_date(2025, Month::March, 15).unwrap())
///     .notional(Money::new(10_000_000.0, Currency::CNY))
///     .contract_rate(7.25)
///     .settlement_curve_id(CurveId::new("USD-OIS"))
///     .quote_convention(NdfQuoteConvention::BasePerSettlement)
///     .build()
///     .expect("Valid NDF");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Ndf {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Base currency (restricted/non-deliverable currency, numerator).
    pub base_currency: Currency,
    /// Settlement currency (freely convertible, typically USD, denominator and PV currency).
    pub settlement_currency: Currency,
    /// Fixing date (rate observation date, typically T-2 before maturity).
    pub fixing_date: Date,
    /// Maturity/settlement date.
    pub maturity_date: Date,
    /// Notional amount in base currency.
    pub notional: Money,
    /// Contract forward rate. Interpretation depends on `quote_convention`:
    /// - BasePerSettlement: base per settlement (e.g., 7.25 CNY per USD)
    /// - SettlementPerBase: settlement per base (e.g., 0.138 USD per CNY)
    pub contract_rate: f64,
    /// Settlement currency discount curve ID.
    pub settlement_curve_id: CurveId,
    /// Quote convention for contract_rate and fixing_rate.
    pub quote_convention: NdfQuoteConvention,
    /// Optional foreign (base) currency discount curve ID.
    /// If not provided, forward rate estimation uses settlement curve as fallback.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub foreign_curve_id: Option<CurveId>,
    /// Observed fixing rate. Interpretation depends on `quote_convention`.
    /// If Some, NDF is post-fixing.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fixing_rate: Option<f64>,
    /// Official fixing source/benchmark enum for type-safe specification.
    ///
    /// Use this field for validated fixing sources.
    /// See [`NdfFixingSource`] for supported benchmarks and their typical currencies.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fixing_source_enum: Option<NdfFixingSource>,
    /// Optional spot rate override for forward rate calculation.
    /// Interpretation depends on `quote_convention`.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub spot_rate_override: Option<f64>,
    /// Optional base currency calendar.
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub base_calendar_id: Option<String>,
    /// Optional settlement currency calendar.
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub settlement_calendar_id: Option<String>,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

impl Ndf {
    /// Create a canonical example NDF for testing and documentation.
    ///
    /// Returns a 3-month USD/CNY NDF with realistic parameters.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("USDCNY-NDF-3M"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(
                Date::from_calendar_date(2025, time::Month::March, 13).expect("Valid example date"),
            )
            .maturity_date(
                Date::from_calendar_date(2025, time::Month::March, 15).expect("Valid example date"),
            )
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .quote_convention(NdfQuoteConvention::BasePerSettlement)
            .fixing_source_enum_opt(Some(NdfFixingSource::Pboc))
            .attributes(
                Attributes::new()
                    .with_tag("ndf")
                    .with_meta("pair", "USDCNY"),
            )
            .build()
            .expect("Example NDF construction should not fail")
    }

    /// Validate that the fixing source is appropriate for the base currency.
    ///
    /// Returns an error if the fixing source enum is set and doesn't match
    /// the expected currency for that benchmark.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fx::ndf::{Ndf, NdfFixingSource, NdfQuoteConvention};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use finstack_core::money::Money;
    /// use finstack_core::types::{CurveId, InstrumentId};
    /// use time::Month;
    ///
    /// // This is valid: CNY with PBOC fixing
    /// let ndf_cny = Ndf::builder()
    ///     .id(InstrumentId::new("USDCNY"))
    ///     .base_currency(Currency::CNY)
    ///     .settlement_currency(Currency::USD)
    ///     .fixing_date(Date::from_calendar_date(2025, Month::March, 13).unwrap())
    ///     .maturity_date(Date::from_calendar_date(2025, Month::March, 15).unwrap())
    ///     .notional(Money::new(10_000_000.0, Currency::CNY))
    ///     .contract_rate(7.25)
    ///     .settlement_curve_id(CurveId::new("USD-OIS"))
    ///     .quote_convention(NdfQuoteConvention::BasePerSettlement)
    ///     .fixing_source_enum_opt(Some(NdfFixingSource::Pboc))
    ///     .build()
    ///     .unwrap();
    /// assert!(ndf_cny.validate_fixing_source().is_ok());
    /// ```
    pub fn validate_fixing_source(&self) -> Result<()> {
        if let Some(fixing_source) = &self.fixing_source_enum {
            if let Some(expected_ccy) = fixing_source.typical_currency() {
                if expected_ccy != self.base_currency {
                    return Err(finstack_core::Error::Validation(format!(
                        "Fixing source {} is typically used for {} but NDF base currency is {}. \
                         Consider using the appropriate fixing source for this currency.",
                        fixing_source, expected_ccy, self.base_currency
                    )));
                }
            }
        }
        Ok(())
    }

    /// Get the effective fixing source as a string.
    ///
    /// Returns the enum display name if `fixing_source_enum` is set.
    pub fn effective_fixing_source(&self) -> Option<String> {
        self.fixing_source_enum
            .as_ref()
            .map(|fixing_enum| fixing_enum.to_string())
    }

    /// Construct an NDF from trade date and tenor using standard fixing offset.
    ///
    /// # Arguments
    ///
    /// * `id` - Instrument identifier
    /// * `base_currency` - Restricted currency (numerator)
    /// * `settlement_currency` - Convertible currency (denominator)
    /// * `trade_date` - Trade date
    /// * `tenor_days` - Days from spot to maturity
    /// * `notional` - Notional in base currency
    /// * `contract_rate` - Contract forward rate
    /// * `settlement_curve_id` - Settlement currency discount curve
    /// * `base_calendar_id` - Optional base currency calendar
    /// * `settlement_calendar_id` - Optional settlement currency calendar
    /// * `spot_lag_days` - Spot lag (typically 2)
    /// * `fixing_offset_days` - Days before maturity for fixing (typically 2)
    /// * `bdc` - Business day convention
    #[allow(clippy::too_many_arguments)]
    pub fn from_trade_date(
        id: impl Into<InstrumentId>,
        base_currency: Currency,
        settlement_currency: Currency,
        trade_date: Date,
        tenor_days: i64,
        notional: Money,
        contract_rate: f64,
        settlement_curve_id: impl Into<CurveId>,
        base_calendar_id: Option<String>,
        settlement_calendar_id: Option<String>,
        spot_lag_days: u32,
        fixing_offset_days: i64,
        bdc: finstack_core::dates::BusinessDayConvention,
    ) -> finstack_core::Result<Self> {
        use crate::instruments::common::fx_dates::{adjust_joint_calendar, roll_spot_date};

        let spot_date = roll_spot_date(
            trade_date,
            spot_lag_days,
            bdc,
            base_calendar_id.as_deref(),
            settlement_calendar_id.as_deref(),
        )?;
        let maturity_unadjusted = spot_date + time::Duration::days(tenor_days);
        let maturity_date = adjust_joint_calendar(
            maturity_unadjusted,
            bdc,
            base_calendar_id.as_deref(),
            settlement_calendar_id.as_deref(),
        )?;

        // Fixing date is typically T-2 before maturity
        let fixing_unadjusted = maturity_date - time::Duration::days(fixing_offset_days);
        let fixing_date = adjust_joint_calendar(
            fixing_unadjusted,
            finstack_core::dates::BusinessDayConvention::Preceding,
            base_calendar_id.as_deref(),
            settlement_calendar_id.as_deref(),
        )?;

        Self::builder()
            .id(id.into())
            .base_currency(base_currency)
            .settlement_currency(settlement_currency)
            .fixing_date(fixing_date)
            .maturity_date(maturity_date)
            .notional(notional)
            .contract_rate(contract_rate)
            .settlement_curve_id(settlement_curve_id.into())
            .quote_convention(NdfQuoteConvention::BasePerSettlement)
            .base_calendar_id_opt(base_calendar_id)
            .settlement_calendar_id_opt(settlement_calendar_id)
            .attributes(Attributes::new())
            .build()
    }

    /// Set the observed fixing rate (transitions NDF to post-fixing mode).
    pub fn with_fixing_rate(mut self, fixing_rate: f64) -> Self {
        self.fixing_rate = Some(fixing_rate);
        self
    }

    /// Check if NDF is in post-fixing mode.
    pub fn is_fixed(&self) -> bool {
        self.fixing_rate.is_some()
    }

    /// Estimate the forward rate when in pre-fixing mode.
    ///
    /// The forward rate is estimated in the same convention as `quote_convention`:
    /// - **BasePerSettlement**: Returns base per settlement (e.g., CNY/USD)
    /// - **SettlementPerBase**: Returns settlement per base (e.g., USD/CNY)
    fn estimate_forward_rate(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        use finstack_core::money::fx::FxQuery;

        // Determine which direction to query based on convention
        let (from_ccy, to_ccy) = match self.quote_convention {
            NdfQuoteConvention::BasePerSettlement => {
                // Rate is base/settlement, query base->settlement (returns base per settlement)
                (self.base_currency, self.settlement_currency)
            }
            NdfQuoteConvention::SettlementPerBase => {
                // Rate is settlement/base, query settlement->base (returns settlement per base)
                (self.settlement_currency, self.base_currency)
            }
        };

        // Try to get spot rate in the appropriate convention
        let spot = if let Some(rate) = self.spot_rate_override {
            rate
        } else if let Some(fx) = market.fx() {
            match (**fx).rate(FxQuery::new(from_ccy, to_ccy, as_of)) {
                Ok(fx_rate) => fx_rate.rate,
                Err(_) => {
                    // Try inverse and flip
                    let inverse = (**fx).rate(FxQuery::new(to_ccy, from_ccy, as_of))?;
                    1.0 / inverse.rate
                }
            }
        } else {
            // No FX matrix, use contract rate as proxy (simplified)
            return Ok(self.contract_rate);
        };

        // Get settlement discount factor
        let settlement_disc = market.get_discount(self.settlement_curve_id.as_str())?;
        let df_settlement = settlement_disc.df_between_dates(as_of, self.maturity_date)?;

        // If foreign curve available, use CIRP
        if let Some(ref foreign_curve_id) = self.foreign_curve_id {
            if let Ok(foreign_disc) = market.get_discount(foreign_curve_id.as_str()) {
                let df_foreign = foreign_disc.df_between_dates(as_of, self.maturity_date)?;
                // Forward rate via covered interest rate parity
                // For BasePerSettlement: F = S × DF_base / DF_settlement
                // For SettlementPerBase: F = S × DF_settlement / DF_base
                let forward = match self.quote_convention {
                    NdfQuoteConvention::BasePerSettlement => spot * df_foreign / df_settlement,
                    NdfQuoteConvention::SettlementPerBase => spot * df_settlement / df_foreign,
                };
                return Ok(forward);
            }
        }

        // Fallback for restricted currencies: assume flat basis (F ≈ S adjusted for time)
        // This is a simplification; in practice you'd use NDF market quotes or basis curves
        // For now, use the settlement curve alone: F = S (no adjustment for restricted currency rate)
        Ok(spot)
    }

    /// Set the quote convention.
    pub fn with_quote_convention(mut self, convention: NdfQuoteConvention) -> Self {
        self.quote_convention = convention;
        self
    }
}

impl crate::instruments::common::traits::CurveDependencies for Ndf {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        let mut builder = crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.settlement_curve_id.clone());

        if let Some(ref foreign_curve) = self.foreign_curve_id {
            builder = builder.discount(foreign_curve.clone());
        }

        builder.build()
    }
}

impl crate::instruments::common::traits::Instrument for Ndf {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::Ndf
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

    fn market_dependencies(&self) -> crate::instruments::common::dependencies::MarketDependencies {
        let mut deps =
            crate::instruments::common::dependencies::MarketDependencies::from_curve_dependencies(
                self,
            );
        deps.add_fx_pair(self.base_currency, self.settlement_currency);
        deps
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // If maturity has passed, value is zero
        if self.maturity_date < as_of {
            return Ok(Money::new(0.0, self.settlement_currency));
        }

        // Get settlement discount curve
        let settlement_disc = market.get_discount(self.settlement_curve_id.as_str())?;
        let df_settlement = settlement_disc.df_between_dates(as_of, self.maturity_date)?;

        // Determine the forward rate to use
        let effective_forward = if let Some(fixed_rate) = self.fixing_rate {
            // Post-fixing: use observed rate
            fixed_rate
        } else if as_of >= self.fixing_date {
            // Past fixing date but no rate set - this is an error condition
            return Err(finstack_core::Error::Validation(format!(
                "NDF {} is past fixing date ({}) but no fixing_rate is set. \
                 Use with_fixing_rate() to set the observed rate.",
                self.id, self.fixing_date
            )));
        } else {
            // Pre-fixing: estimate forward rate
            self.estimate_forward_rate(market, as_of)?
        };

        // Validate notional currency
        if self.notional.currency() != self.base_currency {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: self.base_currency,
                actual: self.notional.currency(),
            });
        }
        let n_base = self.notional.amount();

        // Compute settlement amount based on quote convention
        let settlement_amount = match self.quote_convention {
            NdfQuoteConvention::BasePerSettlement => {
                // Rate is base per settlement (e.g., 7.25 CNY per USD)
                // Settlement = N_base × (1/F_contract - 1/F_fixing)
                // Positive when F_fixing > F_contract (base currency depreciated)
                n_base * (1.0 / self.contract_rate - 1.0 / effective_forward)
            }
            NdfQuoteConvention::SettlementPerBase => {
                // Rate is settlement per base (e.g., 0.138 USD per CNY)
                // Settlement = N_base × (F_fixing - F_contract)
                // Positive when F_fixing > F_contract (base currency appreciated)
                n_base * (effective_forward - self.contract_rate)
            }
        };

        let pv = settlement_amount * df_settlement;
        Ok(Money::new(pv, self.settlement_currency))
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
            None,
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_ndf_creation() {
        let ndf = Ndf::builder()
            .id(InstrumentId::new("TEST-NDF"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2025, Month::March, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2025, Month::March, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .quote_convention(NdfQuoteConvention::BasePerSettlement)
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        assert_eq!(ndf.id.as_str(), "TEST-NDF");
        assert_eq!(ndf.base_currency, Currency::CNY);
        assert_eq!(ndf.settlement_currency, Currency::USD);
        assert_eq!(ndf.contract_rate, 7.25);
        assert!(!ndf.is_fixed());
    }

    #[test]
    fn test_ndf_example() {
        let ndf = Ndf::example();
        assert_eq!(ndf.id.as_str(), "USDCNY-NDF-3M");
        assert_eq!(ndf.base_currency, Currency::CNY);
        assert_eq!(ndf.settlement_currency, Currency::USD);
        assert!(ndf.attributes.has_tag("ndf"));
    }

    #[test]
    fn test_ndf_with_fixing_rate() {
        let ndf = Ndf::example().with_fixing_rate(7.30);
        assert!(ndf.is_fixed());
        assert_eq!(ndf.fixing_rate, Some(7.30));
    }

    #[test]
    fn test_ndf_instrument_trait() {
        use crate::instruments::common::traits::Instrument;

        let ndf = Ndf::example();

        assert_eq!(ndf.id(), "USDCNY-NDF-3M");
        assert_eq!(ndf.key(), crate::pricer::InstrumentType::Ndf);
        assert!(ndf.attributes().has_tag("ndf"));
    }

    #[test]
    fn test_ndf_curve_dependencies() {
        use crate::instruments::common::traits::CurveDependencies;

        let ndf = Ndf::example();
        let deps = ndf.curve_dependencies();

        assert_eq!(deps.discount_curves.len(), 1);
    }

    #[test]
    fn test_ndf_with_foreign_curve() {
        use crate::instruments::common::traits::CurveDependencies;

        let ndf = Ndf::builder()
            .id(InstrumentId::new("TEST-NDF"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2025, Month::March, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2025, Month::March, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .quote_convention(NdfQuoteConvention::BasePerSettlement)
            .foreign_curve_id_opt(Some(CurveId::new("CNY-OIS")))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        let deps = ndf.curve_dependencies();
        assert_eq!(deps.discount_curves.len(), 2);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_ndf_serde_roundtrip() {
        let ndf = Ndf::example();
        let json = serde_json::to_string(&ndf).expect("serialize");
        let deserialized: Ndf = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(ndf.id.as_str(), deserialized.id.as_str());
        assert_eq!(ndf.base_currency, deserialized.base_currency);
        assert_eq!(ndf.settlement_currency, deserialized.settlement_currency);
    }

    #[test]
    fn test_ndf_quote_convention_with_builder() {
        let ndf = Ndf::builder()
            .id(InstrumentId::new("TEST-NDF"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2025, Month::March, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2025, Month::March, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(0.138) // USD per CNY
            .quote_convention(NdfQuoteConvention::SettlementPerBase)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        assert_eq!(ndf.quote_convention, NdfQuoteConvention::SettlementPerBase);
    }

    #[test]
    fn test_ndf_with_quote_convention() {
        let ndf = Ndf::example().with_quote_convention(NdfQuoteConvention::SettlementPerBase);
        assert_eq!(ndf.quote_convention, NdfQuoteConvention::SettlementPerBase);
    }

    #[test]
    fn test_ndf_quote_convention_display_and_parse() {
        let bps = NdfQuoteConvention::BasePerSettlement;
        let spb = NdfQuoteConvention::SettlementPerBase;

        assert_eq!(bps.to_string(), "base_per_settlement");
        assert_eq!(spb.to_string(), "settlement_per_base");

        assert_eq!(
            "base_per_settlement"
                .parse::<NdfQuoteConvention>()
                .expect("valid convention"),
            NdfQuoteConvention::BasePerSettlement
        );
        assert_eq!(
            "settlement_per_base"
                .parse::<NdfQuoteConvention>()
                .expect("valid convention"),
            NdfQuoteConvention::SettlementPerBase
        );
        assert_eq!(
            "bps"
                .parse::<NdfQuoteConvention>()
                .expect("valid convention"),
            NdfQuoteConvention::BasePerSettlement
        );
        assert_eq!(
            "spb"
                .parse::<NdfQuoteConvention>()
                .expect("valid convention"),
            NdfQuoteConvention::SettlementPerBase
        );
    }

    #[test]
    fn test_ndf_base_per_settlement_settlement_formula() {
        // Test the settlement formula for BasePerSettlement convention
        // Contract rate: 7.25 CNY/USD
        // Fixing rate: 7.30 CNY/USD (CNY depreciated)
        // Notional: 10,000,000 CNY
        //
        // Expected settlement (in USD):
        // = 10,000,000 * (1/7.25 - 1/7.30)
        // = 10,000,000 * (0.13793 - 0.13699)
        // = 10,000,000 * 0.00094
        // ≈ 9,430 USD (positive, we receive)

        let contract_rate = 7.25;
        let fixing_rate = 7.30;
        let notional = 10_000_000.0;

        let settlement: f64 = notional * (1.0 / contract_rate - 1.0 / fixing_rate);
        assert!(settlement > 0.0, "Settlement should be positive");
        assert!(
            (settlement - 9430.0).abs() < 100.0,
            "Settlement should be approximately 9,430 USD"
        );
    }

    #[test]
    fn test_ndf_settlement_per_base_settlement_formula() {
        // Test the settlement formula for SettlementPerBase convention
        // Contract rate: 0.138 USD/CNY
        // Fixing rate: 0.140 USD/CNY (CNY appreciated)
        // Notional: 10,000,000 CNY
        //
        // Expected settlement (in USD):
        // = 10,000,000 * (0.140 - 0.138)
        // = 10,000,000 * 0.002
        // = 20,000 USD (positive, we receive)

        let contract_rate = 0.138;
        let fixing_rate = 0.140;
        let notional = 10_000_000.0;

        let settlement: f64 = notional * (fixing_rate - contract_rate);
        assert!(settlement > 0.0, "Settlement should be positive");
        assert!(
            (settlement - 20_000.0).abs() < 1.0,
            "Settlement should be exactly 20,000 USD"
        );
    }

    #[test]
    fn test_ndf_past_fixing_without_rate_errors() {
        use crate::instruments::common::traits::Instrument;
        use finstack_core::market_data::context::MarketContext;
        use finstack_core::market_data::term_structures::DiscountCurve;

        // Create a simple market context
        let as_of = Date::from_calendar_date(2025, Month::March, 14).expect("valid date");
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (1.0, 0.95)])
            .build()
            .expect("should build");
        let market = MarketContext::new().insert_discount(curve);

        // Create an NDF that's past fixing date but without fixing_rate
        let ndf = Ndf::builder()
            .id(InstrumentId::new("TEST-NDF"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2025, Month::March, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2025, Month::March, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .quote_convention(NdfQuoteConvention::BasePerSettlement)
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        // value() should error because we're past fixing date without a fixing rate
        let result = ndf.value(&market, as_of);
        assert!(
            result.is_err(),
            "Should error when past fixing without rate"
        );
        let err_msg = result.expect_err("expected an error").to_string();
        assert!(
            err_msg.contains("past fixing date"),
            "Error should mention past fixing date: {}",
            err_msg
        );
    }

    #[test]
    fn test_ndf_fixing_source_enum_display_and_parse() {
        // Test display
        assert_eq!(NdfFixingSource::Pboc.to_string(), "PBOC");
        assert_eq!(NdfFixingSource::Cnhfix.to_string(), "CNHFIX");
        assert_eq!(NdfFixingSource::Rbi.to_string(), "RBI");
        assert_eq!(NdfFixingSource::Kftc.to_string(), "KFTC");
        assert_eq!(NdfFixingSource::Ptax.to_string(), "PTAX");

        // Test parse
        assert_eq!(
            "PBOC".parse::<NdfFixingSource>().expect("valid source"),
            NdfFixingSource::Pboc
        );
        assert_eq!(
            "CNHFIX".parse::<NdfFixingSource>().expect("valid source"),
            NdfFixingSource::Cnhfix
        );
        assert_eq!(
            "cnh_fix".parse::<NdfFixingSource>().expect("valid source"),
            NdfFixingSource::Cnhfix
        );
        assert_eq!(
            "RBI".parse::<NdfFixingSource>().expect("valid source"),
            NdfFixingSource::Rbi
        );
    }

    #[test]
    fn test_ndf_fixing_source_typical_currency() {
        assert_eq!(
            NdfFixingSource::Pboc.typical_currency(),
            Some(Currency::CNY)
        );
        // CNHFIX maps to CNY (offshore CNY uses same currency code in most systems)
        assert_eq!(
            NdfFixingSource::Cnhfix.typical_currency(),
            Some(Currency::CNY)
        );
        assert_eq!(NdfFixingSource::Rbi.typical_currency(), Some(Currency::INR));
        assert_eq!(
            NdfFixingSource::Kftc.typical_currency(),
            Some(Currency::KRW)
        );
        assert_eq!(
            NdfFixingSource::Ptax.typical_currency(),
            Some(Currency::BRL)
        );
        assert_eq!(NdfFixingSource::Other.typical_currency(), None);
    }

    #[test]
    fn test_ndf_fixing_source_typical_fixing_offset() {
        // Most currencies fix T-2
        assert_eq!(NdfFixingSource::Pboc.typical_fixing_offset(), 2);
        assert_eq!(NdfFixingSource::Rbi.typical_fixing_offset(), 2);
        assert_eq!(NdfFixingSource::Ptax.typical_fixing_offset(), 2);

        // KRW and PHP fix T-1
        assert_eq!(NdfFixingSource::Kftc.typical_fixing_offset(), 1);
        assert_eq!(NdfFixingSource::PhpBval.typical_fixing_offset(), 1);
    }

    #[test]
    fn test_ndf_validate_fixing_source_valid() {
        let ndf = Ndf::builder()
            .id(InstrumentId::new("USDCNY"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2025, Month::March, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2025, Month::March, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .quote_convention(NdfQuoteConvention::BasePerSettlement)
            .fixing_source_enum_opt(Some(NdfFixingSource::Pboc))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        // CNY with PBOC is valid
        assert!(ndf.validate_fixing_source().is_ok());
    }

    #[test]
    fn test_ndf_validate_fixing_source_mismatch_warns() {
        let ndf = Ndf::builder()
            .id(InstrumentId::new("USDINR"))
            .base_currency(Currency::INR)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2025, Month::March, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2025, Month::March, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::INR))
            .contract_rate(83.50)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .quote_convention(NdfQuoteConvention::BasePerSettlement)
            .fixing_source_enum_opt(Some(NdfFixingSource::Pboc)) // Wrong! PBOC is for CNY
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        // INR with PBOC is a mismatch
        let result = ndf.validate_fixing_source();
        assert!(result.is_err(), "Should warn about fixing source mismatch");
        let err_msg = result.expect_err("expected an error").to_string();
        assert!(
            err_msg.contains("CNY") && err_msg.contains("INR"),
            "Error should mention currency mismatch: {}",
            err_msg
        );
    }

    #[test]
    fn test_ndf_effective_fixing_source() {
        // With enum set
        let ndf_enum = Ndf::builder()
            .id(InstrumentId::new("USDCNY"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2025, Month::March, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2025, Month::March, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .quote_convention(NdfQuoteConvention::BasePerSettlement)
            .fixing_source_enum_opt(Some(NdfFixingSource::Pboc))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        assert_eq!(ndf_enum.effective_fixing_source(), Some("PBOC".to_string()));
    }
}
