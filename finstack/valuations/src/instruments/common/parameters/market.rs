//! Market parameter types for instrument pricing.

use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::CurveId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Option type for pricing
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum OptionType {
    /// Call option
    Call,
    /// Put option
    Put,
}

impl std::fmt::Display for OptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptionType::Call => write!(f, "call"),
            OptionType::Put => write!(f, "put"),
        }
    }
}

impl std::str::FromStr for OptionType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "call" | "buy" | "buy_protection" => Ok(OptionType::Call),
            "put" | "sell" | "sell_protection" => Ok(OptionType::Put),
            other => Err(format!("Unknown option type: {}", other)),
        }
    }
}

/// Exercise style for options
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ExerciseStyle {
    /// European exercise (only at expiry)
    #[default]
    European,
    /// American exercise (any time before/at expiry)
    American,
    /// Bermudan exercise (specific dates before expiry)
    Bermudan,
}

impl std::fmt::Display for ExerciseStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExerciseStyle::European => write!(f, "european"),
            ExerciseStyle::American => write!(f, "american"),
            ExerciseStyle::Bermudan => write!(f, "bermudan"),
        }
    }
}

impl std::str::FromStr for ExerciseStyle {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "european" => Ok(ExerciseStyle::European),
            "american" => Ok(ExerciseStyle::American),
            "bermudan" => Ok(ExerciseStyle::Bermudan),
            other => Err(format!("Unknown exercise style: {}", other)),
        }
    }
}

/// Settlement type for options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum SettlementType {
    /// Physical delivery
    Physical,
    /// Cash settlement
    Cash,
}

impl std::fmt::Display for SettlementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettlementType::Physical => write!(f, "physical"),
            SettlementType::Cash => write!(f, "cash"),
        }
    }
}

impl std::str::FromStr for SettlementType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "physical" => Ok(SettlementType::Physical),
            "cash" => Ok(SettlementType::Cash),
            other => Err(format!("Unknown settlement type: {}", other)),
        }
    }
}

/// Market parameters for equity options
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EquityOptionParams {
    /// Option strike price
    pub strike: f64,
    /// Option expiry date
    pub expiry: Date,
    /// Option type (Call/Put)
    pub option_type: OptionType,
    /// Exercise style (European/American/Bermudan)
    pub exercise_style: ExerciseStyle,
    /// Settlement type (Cash/Physical)
    pub settlement: SettlementType,
    /// Contract notional
    pub notional: Money,
}

impl EquityOptionParams {
    /// Create new equity option parameters
    pub fn new(strike: f64, expiry: Date, option_type: OptionType, notional: Money) -> Self {
        Self {
            strike,
            expiry,
            option_type,
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Physical,
            notional,
        }
    }

    /// Create European call parameters
    #[allow(clippy::expect_used)] // Builder with valid inputs should not fail
    pub fn european_call(strike: f64, expiry: Date, notional: Money) -> Self {
        Self::new(strike, expiry, OptionType::Call, notional)
    }

    /// Create European put parameters
    #[allow(clippy::expect_used)] // Builder with valid inputs should not fail
    pub fn european_put(strike: f64, expiry: Date, notional: Money) -> Self {
        Self::new(strike, expiry, OptionType::Put, notional)
    }

    /// Set exercise style
    pub fn with_exercise_style(mut self, style: ExerciseStyle) -> Self {
        self.exercise_style = style;
        self
    }

    /// Set settlement type
    pub fn with_settlement(mut self, settlement: SettlementType) -> Self {
        self.settlement = settlement;
        self
    }
}

/// Market parameters for FX options
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FxOptionParams {
    /// Strike rate (FX rate)
    pub strike: f64,
    /// Option expiry date
    pub expiry: Date,
    /// Option type (Call/Put)
    pub option_type: OptionType,
    /// Exercise style (European/American/Bermudan)
    pub exercise_style: ExerciseStyle,
    /// Settlement type (Cash/Physical)
    pub settlement: SettlementType,
    /// Notional amount
    pub notional: Money,
}

impl FxOptionParams {
    /// Create new FX option parameters
    pub fn new(strike: f64, expiry: Date, option_type: OptionType, notional: Money) -> Self {
        Self {
            strike,
            expiry,
            option_type,
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Physical,
            notional,
        }
    }

    /// Create European call option parameters
    #[allow(clippy::expect_used)] // Builder with valid inputs should not fail
    pub fn european_call(strike: f64, expiry: Date, notional: Money) -> Self {
        Self::new(strike, expiry, OptionType::Call, notional)
    }

    /// Create European put option parameters  
    #[allow(clippy::expect_used)] // Builder with valid inputs should not fail
    pub fn european_put(strike: f64, expiry: Date, notional: Money) -> Self {
        Self::new(strike, expiry, OptionType::Put, notional)
    }

    /// Set exercise style
    pub fn with_exercise_style(mut self, style: ExerciseStyle) -> Self {
        self.exercise_style = style;
        self
    }

    /// Set settlement type
    pub fn with_settlement(mut self, settlement: SettlementType) -> Self {
        self.settlement = settlement;
        self
    }
}

/// Credit parameters for CDS instruments
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CreditParams {
    /// Reference entity (issuer being protected)
    pub reference_entity: String,
    /// Recovery rate (0.0 to 1.0)
    pub recovery_rate: f64,
    /// Credit curve identifier
    pub credit_curve_id: CurveId,
}

impl CreditParams {
    /// Create new credit parameters
    pub fn new(
        reference_entity: impl Into<String>,
        recovery_rate: f64,
        credit_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            reference_entity: reference_entity.into(),
            recovery_rate,
            credit_curve_id: credit_curve_id.into(),
        }
    }

    /// Standard corporate credit with 40% recovery
    pub fn corporate_standard(
        reference_entity: impl Into<String>,
        credit_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self::new(reference_entity, 0.40, credit_curve_id)
    }

    /// Sovereign credit with 30% recovery
    pub fn sovereign_standard(
        reference_entity: impl Into<String>,
        credit_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self::new(reference_entity, 0.30, credit_curve_id)
    }
}

/// Interest rate option parameters (caps/floors)
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InterestRateOptionParams {
    /// Strike rate for the option
    pub strike: f64,
    /// Option expiry date
    pub expiry: Date,
    /// Option type (Cap/Floor)
    pub option_type: OptionType,
    /// Underlying rate tenor
    pub tenor: String,
    /// Day count convention
    pub day_count: DayCount,
    /// Notional amount
    pub notional: Money,
}

impl InterestRateOptionParams {
    /// Create new IR option parameters
    pub fn new(
        strike: f64,
        expiry: Date,
        option_type: OptionType,
        tenor: impl Into<String>,
        notional: Money,
    ) -> Self {
        Self {
            strike,
            expiry,
            option_type,
            tenor: tenor.into(),
            day_count: DayCount::Act360,
            notional,
        }
    }
}
