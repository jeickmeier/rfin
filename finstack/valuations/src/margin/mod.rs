//! Margin and collateral management for financial instruments.
//!
//! This module re-exports the standalone `finstack-margin` crate and keeps the
//! concrete `Marginable` implementations for valuations instruments.

pub use finstack_margin::calculators;
pub use finstack_margin::constants;
pub use finstack_margin::metrics;
pub use finstack_margin::registry;
pub use finstack_margin::types;

pub use finstack_margin::calculators::im::schedule::ScheduleAssetClass;
pub use finstack_margin::calculators::im::simm::SimmVersion;
pub use finstack_margin::{
    CcpMarginInputSource, CcpMethodology, ClearingHouseImCalculator, ClearingStatus,
    CollateralAssetClass, CollateralEligibility, ConcentrationBreach, CsaSpec,
    EligibleCollateralSchedule, HaircutImCalculator, ImCalculator, ImMethodology, ImParameters,
    ImResult, InstrumentMarginResult, InternalModelImCalculator, InternalModelInputSource,
    MarginCall, MarginCallTiming, MarginCallType, MarginTenor, Marginable, MaturityConstraints,
    NettingSetId, OtcMarginSpec, RepoMarginSpec, RepoMarginType, ScheduleImCalculator,
    SimmCalculator, SimmCreditSector, SimmRiskClass, SimmSensitivities, VmCalculator, VmParameters,
    VmResult,
};

mod impls;
