//! Workout management system for loan restructuring and recovery.

pub mod engine;

pub use engine::{
    WorkoutEngine,
    WorkoutState,
    WorkoutPolicy,
    WorkoutStrategy,
    WorkoutEvent,
    WorkoutApplication,
    DefaultTrigger,
    RateModification,
    PrincipalModification,
    RecoveryWaterfall,
    RecoveryTier,
    RecoveryAnalysis,
    ClaimAmount,
};
