//! Covenant evaluation and management system.

pub mod engine;

pub use engine::{
    CovenantEngine,
    CovenantSpec,
    CovenantTestSpec,
    CovenantWindow,
    CovenantBreach,
    ConsequenceApplication,
    InstrumentMutator,
};
