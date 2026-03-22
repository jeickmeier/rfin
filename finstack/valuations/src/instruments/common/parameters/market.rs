//! Market parameter types for instrument pricing.

use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, Percentage, Rate};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

use serde::{Deserialize, Serialize};

/// Option type for pricing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlementType {
    /// Physical delivery
    Physical,
    /// Cash settlement
    Cash,
}

/// Position direction for futures and forwards.
///
/// Indicates whether the holder is long (buyer) or short (seller) of the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    /// Long position (buyer of futures/forward contract).
    ///
    /// Profits when the underlying price increases.
    #[default]
    Long,
    /// Short position (seller of futures/forward contract).
    ///
    /// Profits when the underlying price decreases.
    Short,
}

impl Position {
    /// Returns the sign multiplier for this position (+1.0 for Long, -1.0 for Short).
    #[inline]
    pub fn sign(&self) -> f64 {
        match self {
            Position::Long => 1.0,
            Position::Short => -1.0,
        }
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::Long => write!(f, "long"),
            Position::Short => write!(f, "short"),
        }
    }
}

impl std::str::FromStr for Position {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "long" | "buy" | "buyer" => Ok(Position::Long),
            "short" | "sell" | "seller" => Ok(Position::Short),
            other => Err(format!("Unknown position: {}", other)),
        }
    }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn european_call(strike: f64, expiry: Date, notional: Money) -> Self {
        Self::new(strike, expiry, OptionType::Call, notional)
    }

    /// Create European put parameters
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn european_call(strike: f64, expiry: Date, notional: Money) -> Self {
        Self::new(strike, expiry, OptionType::Call, notional)
    }

    /// Create European put option parameters
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Create new credit parameters using typed percentage recovery.
    pub fn new_pct(
        reference_entity: impl Into<String>,
        recovery_rate: Percentage,
        credit_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            reference_entity: reference_entity.into(),
            recovery_rate: recovery_rate.as_decimal(),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Create new IR option parameters using a typed strike rate.
    pub fn new_rate(
        strike: Rate,
        expiry: Date,
        option_type: OptionType,
        tenor: impl Into<String>,
        notional: Money,
    ) -> Self {
        Self {
            strike: strike.as_decimal(),
            expiry,
            option_type,
            tenor: tenor.into(),
            day_count: DayCount::Act360,
            notional,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::macros::date;

    #[test]
    fn enum_parsing_display_and_position_sign_cover_aliases() {
        assert_eq!(OptionType::Call.to_string(), "call");
        assert_eq!("buy".parse::<OptionType>(), Ok(OptionType::Call));
        assert_eq!("sell_protection".parse::<OptionType>(), Ok(OptionType::Put));
        assert!("weird".parse::<OptionType>().is_err());

        assert_eq!(ExerciseStyle::default(), ExerciseStyle::European);
        assert_eq!(
            "american".parse::<ExerciseStyle>(),
            Ok(ExerciseStyle::American)
        );
        assert_eq!(ExerciseStyle::Bermudan.to_string(), "bermudan");
        assert!("odd".parse::<ExerciseStyle>().is_err());

        assert_eq!(SettlementType::Cash.to_string(), "cash");
        assert_eq!(
            "physical".parse::<SettlementType>(),
            Ok(SettlementType::Physical)
        );
        assert!("gross".parse::<SettlementType>().is_err());

        assert_eq!(Position::default(), Position::Long);
        assert_eq!(Position::Long.sign(), 1.0);
        assert_eq!(Position::Short.sign(), -1.0);
        assert_eq!("buyer".parse::<Position>(), Ok(Position::Long));
        assert_eq!("sell".parse::<Position>(), Ok(Position::Short));
        assert!("flat".parse::<Position>().is_err());
    }

    #[test]
    fn equity_and_fx_option_builders_apply_defaults_and_overrides() {
        let expiry = date!(2026 - 06 - 15);
        let notional = Money::new(1_000_000.0, Currency::USD);

        let equity = EquityOptionParams::european_call(100.0, expiry, notional)
            .with_exercise_style(ExerciseStyle::American)
            .with_settlement(SettlementType::Cash);
        assert_eq!(equity.option_type, OptionType::Call);
        assert_eq!(equity.exercise_style, ExerciseStyle::American);
        assert_eq!(equity.settlement, SettlementType::Cash);

        let fx = FxOptionParams::european_put(1.12, expiry, notional)
            .with_exercise_style(ExerciseStyle::Bermudan)
            .with_settlement(SettlementType::Physical);
        assert_eq!(fx.option_type, OptionType::Put);
        assert_eq!(fx.exercise_style, ExerciseStyle::Bermudan);
        assert_eq!(fx.settlement, SettlementType::Physical);
    }

    #[test]
    fn credit_and_ir_option_typed_constructors_preserve_typed_inputs() {
        let credit = CreditParams::new_pct("ACME", Percentage::new(35.0), "ACME-CDS");
        assert_eq!(credit.reference_entity, "ACME");
        assert!((credit.recovery_rate - 0.35).abs() < 1e-12);
        assert_eq!(credit.credit_curve_id.as_str(), "ACME-CDS");

        let corp = CreditParams::corporate_standard("CORP", "CORP-CDS");
        let sov = CreditParams::sovereign_standard("UST", "UST-CDS");
        assert!((corp.recovery_rate - 0.40).abs() < 1e-12);
        assert!((sov.recovery_rate - 0.30).abs() < 1e-12);

        let ir = InterestRateOptionParams::new_rate(
            Rate::from_bps(325),
            date!(2027 - 01 - 01),
            OptionType::Put,
            "6M",
            Money::new(5_000_000.0, Currency::USD),
        );
        assert!((ir.strike - 0.0325).abs() < 1e-12);
        assert_eq!(ir.option_type, OptionType::Put);
        assert_eq!(ir.tenor, "6M");
        assert_eq!(ir.day_count, DayCount::Act360);
    }

    #[test]
    fn base_constructors_and_serde_roundtrip_preserve_defaults() {
        let expiry = date!(2026 - 06 - 15);
        let notional = Money::new(2_000_000.0, Currency::USD);

        let equity = EquityOptionParams::new(95.0, expiry, OptionType::Put, notional);
        let fx = FxOptionParams::new(1.05, expiry, OptionType::Call, notional);
        let credit = CreditParams::new("Issuer", 0.4, "ISSUER-CDS");
        let ir = InterestRateOptionParams::new(0.03, expiry, OptionType::Call, "3M", notional);

        assert_eq!(equity.exercise_style, ExerciseStyle::European);
        assert_eq!(equity.settlement, SettlementType::Physical);
        assert_eq!(fx.exercise_style, ExerciseStyle::European);
        assert_eq!(fx.settlement, SettlementType::Physical);
        assert_eq!(credit.recovery_rate, 0.4);
        assert_eq!(ir.day_count, DayCount::Act360);

        let equity_json = serde_json::to_string(&equity);
        let fx_json = serde_json::to_string(&fx);
        let credit_json = serde_json::to_string(&credit);
        let ir_json = serde_json::to_string(&ir);
        assert!(equity_json.is_ok());
        assert!(fx_json.is_ok());
        assert!(credit_json.is_ok());
        assert!(ir_json.is_ok());

        if let Ok(json) = equity_json {
            let roundtrip = serde_json::from_str::<EquityOptionParams>(&json);
            assert!(roundtrip.is_ok());
            if let Ok(back) = roundtrip {
                assert_eq!(back.option_type, OptionType::Put);
                assert_eq!(back.exercise_style, ExerciseStyle::European);
            }
        }
        if let Ok(json) = fx_json {
            let roundtrip = serde_json::from_str::<FxOptionParams>(&json);
            assert!(roundtrip.is_ok());
            if let Ok(back) = roundtrip {
                assert_eq!(back.option_type, OptionType::Call);
                assert_eq!(back.settlement, SettlementType::Physical);
            }
        }
        if let Ok(json) = credit_json {
            let roundtrip = serde_json::from_str::<CreditParams>(&json);
            assert!(roundtrip.is_ok());
            if let Ok(back) = roundtrip {
                assert_eq!(back.reference_entity, "Issuer");
                assert_eq!(back.credit_curve_id.as_str(), "ISSUER-CDS");
            }
        }
        if let Ok(json) = ir_json {
            let roundtrip = serde_json::from_str::<InterestRateOptionParams>(&json);
            assert!(roundtrip.is_ok());
            if let Ok(back) = roundtrip {
                assert_eq!(back.tenor, "3M");
                assert_eq!(back.option_type, OptionType::Call);
            }
        }
    }
}
