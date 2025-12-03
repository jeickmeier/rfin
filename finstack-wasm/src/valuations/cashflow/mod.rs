pub mod builder;
pub mod specs;

pub use specs::JsAmortizationSpec;

pub use builder::{
    JsCashFlowSchedule as CashFlowSchedule, JsCashflowBuilder as CashflowBuilder,
    JsCouponType as CouponType, JsFixedCouponSpec as FixedCouponSpec,
    JsFloatCouponParams as FloatCouponParams, JsFloatingCouponSpec as FloatingCouponSpec,
    JsScheduleParams as ScheduleParams,
};

/// Alias with Rust-style casing for symmetry with `CashFlowSchedule`.
/// The JavaScript-facing class name remains `CashflowBuilder`.
#[allow(dead_code)]
pub type CashFlowBuilder = CashflowBuilder;
