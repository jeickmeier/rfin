//! Covenant evaluation and management system.

pub mod engine;
pub mod forward;
/// Covenant report types and structures
pub mod mod_types;
/// Covenant threshold schedules and interpolation
pub mod schedule;

pub use engine::{
    ConsequenceApplication, CovenantBreach, CovenantEngine, CovenantSpec, CovenantTestSpec,
    CovenantWindow, InstrumentMutator,
};
pub use forward::{CovenantForecast as GenericCovenantForecast, CovenantForecastConfig};
pub use mod_types::CovenantReport;
pub use schedule::{threshold_for_date, ThresholdSchedule};
