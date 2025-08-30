//! Covenant evaluation and management system.

pub mod engine;

pub use engine::{
    ConsequenceApplication, CovenantBreach, CovenantEngine, CovenantSpec, CovenantTestSpec,
    CovenantWindow, InstrumentMutator,
};
