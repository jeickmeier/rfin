//! Workout management system for loan restructuring and recovery.

pub mod engine;

pub use engine::{
    ClaimAmount, DefaultTrigger, PrincipalModification, RateModification, RecoveryAnalysis,
    RecoveryTier, RecoveryWaterfall, WorkoutApplication, WorkoutEngine, WorkoutEvent,
    WorkoutPolicy, WorkoutState, WorkoutStrategy,
};
