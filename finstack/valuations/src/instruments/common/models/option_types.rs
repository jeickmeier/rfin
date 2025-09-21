//! Common option-related enums reused across instruments.

/// Option type (Call or Put)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionType {
    /// Call option (right to buy)
    Call,
    /// Put option (right to sell)
    Put,
}

/// Option exercise style
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExerciseStyle {
    /// European option (exercise only at maturity)
    European,
    /// American option (exercise any time before maturity)
    American,
    /// Bermudan option (exercise on specific dates)
    Bermudan,
}

/// Settlement type for options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettlementType {
    /// Physical delivery of underlying
    Physical,
    /// Cash settlement
    Cash,
}
