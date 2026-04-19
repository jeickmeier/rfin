//! Split from `builder.rs` for readability.

use super::*;

impl CashFlowBuilder {
    /// Adds a fixed coupon specification.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fixed_cf(&mut self, spec: FixedCouponSpec) -> &mut Self {
        self.push_full_horizon_coupon(
            "fixed_cf",
            spec.schedule_params(),
            CouponSpec::Fixed { rate: spec.rate },
            spec.coupon_type,
        )
    }

    /// Adds a floating coupon specification.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn floating_cf(&mut self, spec: FloatingCouponSpec) -> &mut Self {
        self.push_full_horizon_coupon(
            "floating_cf",
            Self::schedule_from_floating_spec(&spec),
            CouponSpec::Float {
                rate_spec: spec.rate_spec,
            },
            spec.coupon_type,
        )
    }

    /// Adds a fixed coupon window with its own schedule and payment split (cash/PIK/split).
    ///
    /// Internal helper used by `fixed_stepup` / `fixed_to_float` etc. Prefer the
    /// spec-level entry points (`fixed_cf`, `fixed_stepup`, `fixed_to_float`).
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub(crate) fn add_fixed_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        rate: f64,
        schedule: ScheduleParams,
        split: CouponType,
    ) -> &mut Self {
        debug_assert!(
            rate.is_finite(),
            "add_fixed_coupon_window: rate is not finite ({rate})"
        );
        let Some(rate_decimal) =
            self.decimal_from_f64_or_record_error("add_fixed_coupon_window", "rate", rate)
        else {
            return self;
        };
        self.push_coupon_window(
            start,
            end,
            schedule,
            CouponSpec::Fixed { rate: rate_decimal },
            split,
        )
    }

    /// Adds a floating coupon window with its own schedule and payment split.
    ///
    /// Internal helper used by `float_margin_stepup` / `fixed_to_float` etc.
    /// Prefer the spec-level entry points (`floating_cf`, `float_margin_stepup`,
    /// `fixed_to_float`).
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub(crate) fn add_float_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        spec: FloatingCouponSpec,
    ) -> &mut Self {
        self.push_coupon_window(
            start,
            end,
            Self::schedule_from_floating_spec(&spec),
            CouponSpec::Float {
                rate_spec: spec.rate_spec,
            },
            spec.coupon_type,
        )
    }
}
