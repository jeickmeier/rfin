pub mod builder;
pub mod specs;

pub use specs::JsAmortizationSpec;

pub use builder::{
    JsCashFlowSchedule as CashFlowSchedule, JsCashflowBuilder as CashflowBuilder,
    JsCouponType as CouponType, JsFixedCouponSpec as FixedCouponSpec,
    JsFloatCouponParams as FloatCouponParams, JsFloatingCouponSpec as FloatingCouponSpec,
    JsScheduleParams as ScheduleParams,
};
