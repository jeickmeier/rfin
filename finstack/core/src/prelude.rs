//! finstack-core prelude: commonly used finstack types
//!
//! Import this module to get a convenient set of core finstack types
//! without having to import each one individually.
//! Downstream crates should prefer `use finstack_core::prelude::*;`.
//!
//! Note: Polars functionality should be imported directly where needed:
//! ```rust
//! use polars::prelude::*;
//! ```
//!
//! # Most-used quick guide
//!
//! - `Currency`, `Money`: currency-safe amounts
//! - `Date`, `BusinessDayConvention`, `adjust`: business-day logic
//! - `ScheduleBuilder`, `Period`, `DayCount`: scheduling & accruals
//! - `MarketContext`, `FxProvider`, `FxMatrix`: market data and FX
//! - `ResultsMeta`, `RoundingContext`: IO boundary stamping
//!
//! Example:
//! ```rust
//! use finstack_core::prelude::*;
//! use time::Month;
//!
//! let cal = finstack_core::dates::calendar::Target2;
//! let d = Date::from_calendar_date(2025, Month::January, 4).unwrap();
//! let adj = adjust(d, BusinessDayConvention::Following, &cal).unwrap();
//! let amt = Money::new(100.0, Currency::EUR);
//! assert!(adj >= d);
//! assert_eq!(amt.currency(), Currency::EUR);
//! ```

// Core re-exports (keep list focused on ergonomic entry points)
pub use crate::currency::Currency;
pub use crate::dates::{
    adjust, available_calendars, build_periods, next_cds_date, next_imm, third_wednesday,
    BusinessDayConvention, Date, DateExt, DayCount, HolidayCalendar, OffsetDateTime,
    OffsetDateTimeExt, Period, PeriodId, PeriodKey, ScheduleBuilder, StubKind,
};
pub use crate::error::{Error, InputError};
pub use crate::types::{
    Bps, CounterpartyId, CurveId, Id, InstrumentId, Percentage, PortfolioId, PositionId, Rate,
    ScenarioId, Timestamp, TradeId, TypeTag,
};
// Expression engine - only re-export public items
pub use crate::expr::CompiledExpr;
pub use crate::market_data::{
    scalars::inflation_index::{
        InflationIndex, InflationIndexBuilder, InflationInterpolation, InflationLag,
    },
    interp::{InterpFn, InterpStyle},
    scalars::{MarketScalar, ScalarTimeSeries, SeriesInterpolation},
    traits::{Discount, Forward, Inflation as InflationTs, Surface, Survival, TermStructure},
    MarketContext,
};
pub use crate::money::{
    fx::{FxConversionPolicy, FxMatrix, FxProvider, FxQuery, FxRate, FxRateResult},
    Money,
};
pub use crate::Result;

// Math utilities
pub use crate::math::{
    solver::{BrentSolver, HybridSolver, NewtonSolver, Solver},
    stats::{correlation, covariance, mean, mean_var, variance},
    summation::{kahan_sum, pairwise_sum, stable_sum},
};

// Configuration and policy management
pub use crate::config::{
    ingest_scale_for, output_scale_for, results_meta, rounding_context_from, FinstackConfig,
    NumericMode, ResultsMeta, RoundingContext, RoundingMode, RoundingPolicy,
};
