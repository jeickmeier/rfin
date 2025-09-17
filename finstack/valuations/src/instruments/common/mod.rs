//! Common parameter grouping structures for instrument builders.
//!
//! This module provides reusable parameter groups that eliminate the need for
//! dozens of individual optional fields in instrument builders.

pub mod parameter_groups;

pub use parameter_groups::{
    validate_currency_consistency, BinomialTreeParams, CDSConstructionParams,
    CDSIndexConstructionParams, CDSIndexParams, CDSTrancheParams, CreditOptionParams,
    CreditParams, EquityOptionParams, EquityUnderlyingParams,
    FxOptionParams, FxSwapParams, FxUnderlyingParams, IndexUnderlyingParams,
    InflationLinkedBondParams, InstrumentScheduleParams, InterestRateOptionParams,
    LoanFacilityParams, LoanFeeParams, MarketRefs, OptionMarketParams,
    OptionParams, PricingOverrides, SABRModelParams, SwaptionParams,
};
