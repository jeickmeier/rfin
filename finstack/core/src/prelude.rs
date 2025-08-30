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

// Core re-exports (keep list focused on ergonomic entry points)
pub use crate::currency::Currency;
pub use crate::dates::{
    adjust, available_calendars, build_periods, next_cds_date, next_imm, third_wednesday,
    BusinessDayConvention, Date, DateExt, DayCount, HolidayCalendar,
    OffsetDateTime, OffsetDateTimeExt, Period, PeriodId,
    PeriodKey, ScheduleBuilder, StubKind,
};
pub use crate::error::{Error, InputError};
pub use crate::types::{
    Bps, CounterpartyId, CurveId, Id, InstrumentId, Percentage, PortfolioId, PositionId,
    Rate, ScenarioId, Timestamp, TradeId, TypeTag,
};
// Expression engine - only re-export public items
pub use crate::expr::CompiledExpr;
pub use crate::market_data::{
    inflation_index::{InflationIndex, InflationIndexBuilder, InflationInterpolation, InflationLag},
    interp::{InterpFn, InterpStyle},
    primitives::{MarketScalar, ScalarTimeSeries, SeriesInterpolation},
    MarketContext,
    traits::{Discount, Forward, Inflation as InflationTs, Surface, Survival, TermStructure},
};
pub use crate::money::{
    fx::{
        ClosureCheckResult, FxCacheConfig, FxConversionPolicy, FxMatrix, FxPolicyMeta, FxProvider,
        FxRate,
    },
    Money,
};
pub use crate::Result;

// Validation framework
pub use crate::validation::{
    LengthValidator, RangeValidator, ValidationResult, ValidationStatus, ValidationWarning,
    Validator, ValidatorExt,
};

// Math utilities
pub use crate::math::{
    root_finding::{brent, newton_bracketed},
    stats::{correlation, covariance, mean, mean_var, variance},
    summation::{kahan_sum, pairwise_sum, stable_sum},
};

// Configuration and policy management
pub use crate::config::{
    ingest_scale_for, output_scale_for, results_meta, rounding_context_from, FinstackConfig,
    NumericMode, ResultsMeta, RoundingContext, RoundingMode, RoundingPolicy,
};
