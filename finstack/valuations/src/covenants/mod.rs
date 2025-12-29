//! Covenant evaluation and management system.

pub mod engine;
pub mod forward;
/// Covenant report types and structures
pub mod mod_types;
/// Covenant threshold schedules and interpolation
pub mod schedule;

pub use engine::{
    ConsequenceApplication, Covenant, CovenantBreach, CovenantConsequence, CovenantEngine,
    CovenantScope, CovenantSpec, CovenantTestSpec, CovenantType, CovenantWindow, InstrumentMutator,
    SpringingCondition, ThresholdTest,
};
pub use forward::{
    forecast_breaches_generic, forecast_covenant_generic, CovenantForecast as GenericCovenantForecast,
    CovenantForecastConfig, FutureBreach, McConfig, ModelTimeSeries,
};
pub use mod_types::CovenantReport;
pub use schedule::{threshold_for_date, ThresholdSchedule};
