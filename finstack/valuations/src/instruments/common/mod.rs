//! Common parameter grouping structures for instrument builders.
//!
//! This module provides reusable parameter groups that eliminate the need for
//! dozens of individual optional fields in instrument builders.

pub mod parameter_groups;

pub use parameter_groups::{
    DateRange, EquityUnderlyingParams, FxUnderlyingParams, CreditParams, 
    InstrumentScheduleParams, MarketRefs, OptionParams, PricingOverrides,
    LoanFacilityParams, LoanFeeParams, validate_currency_consistency
};
