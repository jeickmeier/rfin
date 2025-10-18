"""Valuations cash-flow builder exposing complex coupon windows, PIK splits, and amortization."""

from .builder import (
    CouponType,
    ScheduleParams,
    FixedCouponSpec,
    FloatCouponParams,
    FloatingCouponSpec,
    CashflowBuilder,
    CashFlowSchedule,
    FeeBase,
    FeeSpec,
    FixedWindow,
    FloatWindow,
)

__all__ = [
    "CouponType",
    "ScheduleParams", 
    "FixedCouponSpec",
    "FloatCouponParams",
    "FloatingCouponSpec",
    "CashflowBuilder",
    "CashFlowSchedule",
    "FeeBase",
    "FeeSpec",
    "FixedWindow",
    "FloatWindow",
]
