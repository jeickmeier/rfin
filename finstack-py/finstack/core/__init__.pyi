"""Core financial primitives and utilities.

This module provides the fundamental building blocks for financial computations:
- Currency: ISO-4217 currency metadata and identification
- Money: Currency-tagged monetary amounts with safe arithmetic
- Dates: Calendar, day count, and period utilities
- Market Data: Curves, surfaces, FX, and market context
- Math: Distributions, integration, and numerical solvers
- Config: Global rounding and scaling policies
- Explain: Explainability infrastructure for computation tracing
- Volatility: Volatility conventions and pricing models
- Expr: Expression engine for formula evaluation
- Types: Type-safe identifiers and rate helpers
"""

from __future__ import annotations
from . import analytics
from . import cashflow
from . import config
from . import currency
from . import dates
from . import explain
from . import expr
from . import factor_model
from . import market_data
from . import math
from . import money
from . import types
from . import volatility
from . import volatility_models
from .dates.calendar import (
    adjust as adjust,
    available_calendar_codes as available_calendar_codes,
    available_calendars as available_calendars,
    get_calendar as get_calendar,
)
from .dates.imm import (
    imm_option_expiry as imm_option_expiry,
    next_cds_date as next_cds_date,
    next_equity_option_expiry as next_equity_option_expiry,
    next_imm as next_imm,
    next_imm_option_expiry as next_imm_option_expiry,
    third_friday as third_friday,
    third_wednesday as third_wednesday,
)
from .dates.periods import (
    build_fiscal_periods as build_fiscal_periods,
    build_periods as build_periods,
)
from .dates.utils import (
    add_months as add_months,
    date_to_days_since_epoch as date_to_days_since_epoch,
    days_in_month as days_in_month,
    days_since_epoch_to_date as days_since_epoch_to_date,
    is_leap_year as is_leap_year,
    last_day_of_month as last_day_of_month,
)

def require(condition: bool, message: str) -> None:
    """Assert that a condition is true, raising ``ValidationError`` otherwise."""
    ...

def require_or(condition: bool, message: str) -> None:
    """Assert that a condition is true, raising ``ValidationError`` otherwise."""
    ...

__all__ = [
    "analytics",
    "cashflow",
    "config",
    "currency",
    "dates",
    "explain",
    "expr",
    "factor_model",
    "market_data",
    "math",
    "money",
    "types",
    "volatility",
    "volatility_models",
    "adjust",
    "available_calendar_codes",
    "available_calendars",
    "get_calendar",
    "next_imm",
    "next_cds_date",
    "next_imm_option_expiry",
    "imm_option_expiry",
    "next_equity_option_expiry",
    "third_friday",
    "third_wednesday",
    "build_periods",
    "build_fiscal_periods",
    "add_months",
    "last_day_of_month",
    "days_in_month",
    "is_leap_year",
    "date_to_days_since_epoch",
    "days_since_epoch_to_date",
    "require",
    "require_or",
]
