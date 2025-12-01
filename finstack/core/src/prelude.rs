//! Commonly used types and functions from finstack-core.
//!
//! Import this module to get a convenient set of core finstack types
//! without having to import each one individually. This is the recommended
//! entry point for most users.
//!
//! # Usage
//!
//! ```rust
//! use finstack_core::prelude::*;
//! ```
//!
//! # What's Included
//!
//! ## Core Types
//! - [`Currency`], [`Money`]: Currency-safe monetary amounts
//! - [`Date`], [`OffsetDateTime`]: Date and time primitives
//! - [`Error`], [`InputError`], [`Result`]: Error handling
//!
//! ## Financial Identifiers
//! - [`CurveId`], [`InstrumentId`], [`IndexId`], [`PriceId`]: Type-safe IDs
//! - [`Rate`], [`Bps`], [`Percentage`]: Interest rate representations
//!
//! ## Date Operations
//! - [`BusinessDayConvention`], [`adjust`]: Business day adjustments
//! - [`DayCount`]: Day count conventions (Act/360, 30/360, etc.)
//! - [`ScheduleBuilder`], [`Period`]: Schedule generation
//! - [`create_date`], [`next_imm`], [`third_wednesday`]: Date utilities
//!
//! ## Market Data
//! - [`MarketContext`]: Aggregated market data container
//! - [`FxProvider`], [`FxMatrix`]: Foreign exchange rates
//! - [`MarketScalar`], [`InflationIndex`]: Market observables
//!
//! ## Configuration
//! - [`FinstackConfig`], [`RoundingMode`]: Numeric precision settings
//! - [`ResultsMeta`], [`RoundingContext`]: Result metadata stamping
//!
//! ## Mathematical Utilities
//! - [`Solver`], [`BrentSolver`], [`NewtonSolver`]: Root finding
//! - [`mean`], [`variance`], [`correlation`]: Statistical functions
//! - [`kahan_sum`], [`stable_sum`]: Numerically stable summation
//!
//! Example:
//! ```rust
//! use finstack_core::prelude::*;
//! use time::Month;
//!
//! let cal = finstack_core::dates::calendar::TARGET2;
//! let d = Date::from_calendar_date(2025, Month::January, 4).unwrap();
//! let adj = adjust(d, BusinessDayConvention::Following, &cal).unwrap();
//! let amt = Money::new(100.0, Currency::EUR);
//! assert!(adj >= d);
//! assert_eq!(amt.currency(), Currency::EUR);
//! ```

// Core re-exports (keep list focused on ergonomic entry points)
pub use crate::currency::Currency;
pub use crate::dates::{
    adjust, available_calendars, build_periods, create_date, next_cds_date,
    next_imm, third_wednesday, BusinessDayConvention, Date, DateExt, DayCount, HolidayCalendar,
    OffsetDateTime, OffsetDateTimeExt, Period, PeriodId, ScheduleBuilder, StubKind,
};
pub use crate::error::{Error, InputError};
pub use crate::types::{
    moodys_warf_factor, Bps, CreditRating, CurveId, Id, IndexId, InstrumentId, Percentage, PriceId,
    Rate, RatingFactorTable, Timestamp, TypeTag,
};
// Expression engine - only re-export public items
pub use crate::expr::CompiledExpr;
pub use crate::market_data::{
    context::MarketContext,
    scalars::inflation_index::{
        InflationIndex, InflationIndexBuilder, InflationInterpolation, InflationLag,
    },
    scalars::{MarketScalar, ScalarTimeSeries, SeriesInterpolation},
    traits::Discounting,
};
pub use crate::math::interp::{InterpFn, InterpStyle};
pub use crate::money::{
    fx::{FxConversionPolicy, FxMatrix, FxProvider, FxQuery, FxRate, FxRateResult},
    Money,
};
pub use crate::Result;

// Math utilities
pub use crate::math::{
    solver::{BrentSolver, NewtonSolver, Solver},
    stats::{correlation, covariance, mean, mean_var, variance},
    summation::{kahan_sum, pairwise_sum, stable_sum},
};

// Configuration and policy management
pub use crate::config::{
    results_meta, rounding_context_from, FinstackConfig, NumericMode, ResultsMeta, RoundingContext,
    RoundingMode, RoundingPolicy,
};
