use super::defs::{
    CdsConventions, InflationSwapConventions, IrFutureConventions, OptionConventions,
    RateIndexConventions, SwaptionConventions,
};
use super::ids::{
    CdsConventionKey, IndexId, InflationSwapConventionId, IrFutureContractId, OptionConventionId,
    SwaptionConventionId,
};
use finstack_core::{Error, Result};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Global registry of market conventions.
///
/// This registry provides a single source of truth for convention lookups,
/// ensuring strict handling of missing data.
#[derive(Debug, Default)]
pub struct ConventionRegistry {
    /// Registry of Rate Index conventions.
    pub rate_index: HashMap<IndexId, RateIndexConventions>,
    /// Registry of CDS conventions.
    pub cds: HashMap<CdsConventionKey, CdsConventions>,
    /// Registry of CDS Tranche conventions.
    pub cds_tranche: HashMap<CdsConventionKey, CdsConventions>,
    /// Registry of Swaption conventions.
    pub swaption: HashMap<SwaptionConventionId, SwaptionConventions>,
    /// Registry of Inflation Swap conventions.
    pub inflation_swap: HashMap<InflationSwapConventionId, InflationSwapConventions>,
    /// Registry of Option conventions.
    pub option: HashMap<OptionConventionId, OptionConventions>,
    /// Registry of Interest Rate Futures conventions.
    pub ir_future: HashMap<IrFutureContractId, IrFutureConventions>,
}

impl ConventionRegistry {
    /// Create a new registry from in-memory maps.
    pub fn new(
        rate_index: HashMap<IndexId, RateIndexConventions>,
        cds: HashMap<CdsConventionKey, CdsConventions>,
        cds_tranche: HashMap<CdsConventionKey, CdsConventions>,
        swaption: HashMap<SwaptionConventionId, SwaptionConventions>,
        inflation_swap: HashMap<InflationSwapConventionId, InflationSwapConventions>,
        option: HashMap<OptionConventionId, OptionConventions>,
        ir_future: HashMap<IrFutureContractId, IrFutureConventions>,
    ) -> Self {
        Self {
            rate_index,
            cds,
            cds_tranche,
            swaption,
            inflation_swap,
            option,
            ir_future,
        }
    }

    /// Access the global singleton registry.
    ///
    /// This will be initialized with embedded JSON data on the first call.
    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<ConventionRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| ConventionRegistry {
            rate_index: super::loaders::rate_index::load_registry()
                .expect("Failed to load embedded rate index conventions registry"),
            cds: super::loaders::cds::load_registry()
                .expect("Failed to load embedded CDS conventions registry"),
            cds_tranche: super::loaders::cds_tranche::load_registry()
                .expect("Failed to load embedded CDS tranche conventions registry"),
            swaption: super::loaders::swaption::load_registry()
                .expect("Failed to load embedded swaption conventions registry"),
            inflation_swap: super::loaders::inflation_swap::load_registry()
                .expect("Failed to load embedded inflation swap conventions registry"),
            option: super::loaders::option::load_registry()
                .expect("Failed to load embedded option conventions registry"),
            ir_future: super::loaders::ir_future::load_registry()
                .expect("Failed to load embedded IR future conventions registry"),
        })
    }

    /// Resolve conventions for a Rate Index.
    ///
    /// Errors if the index is not found.
    pub fn require_rate_index(&self, id: &IndexId) -> Result<&RateIndexConventions> {
        self.rate_index.get(id).ok_or_else(|| {
            Error::Validation(format!(
                "Missing rate index conventions for '{}'. check rate_index_conventions.json",
                id
            ))
        })
    }

    /// Resolve conventions for a CDS key.
    ///
    /// Errors if key not found.
    pub fn require_cds(&self, key: &CdsConventionKey) -> Result<&CdsConventions> {
        self.cds.get(key).ok_or_else(|| {
            Error::Validation(format!(
                "Missing CDS conventions for '{}'. check cds_conventions.json",
                key
            ))
        })
    }

    /// Resolve conventions for a CDS Tranche key.
    pub fn require_cds_tranche(&self, key: &CdsConventionKey) -> Result<&CdsConventions> {
        self.cds_tranche.get(key).ok_or_else(|| {
            Error::Validation(format!(
                "Missing CDS Tranche conventions for '{}'. check cds_tranche_conventions.json",
                key
            ))
        })
    }

    /// Resolve conventions for a Swaption.
    pub fn require_swaption(&self, id: &SwaptionConventionId) -> Result<&SwaptionConventions> {
        self.swaption.get(id).ok_or_else(|| {
            Error::Validation(format!(
                "Missing swaption conventions for '{}'. check swaption_conventions.json",
                id
            ))
        })
    }

    /// Resolve conventions for an Inflation Swap.
    pub fn require_inflation_swap(
        &self,
        id: &InflationSwapConventionId,
    ) -> Result<&InflationSwapConventions> {
        self.inflation_swap.get(id).ok_or_else(|| {
            Error::Validation(format!(
                "Missing inflation swap conventions for '{}'. check inflation_swap_conventions.json",
                id
            ))
        })
    }

    /// Resolve conventions for an Option.
    pub fn require_option(&self, id: &OptionConventionId) -> Result<&OptionConventions> {
        self.option.get(id).ok_or_else(|| {
            Error::Validation(format!(
                "Missing option conventions for '{}'. check option_conventions.json",
                id
            ))
        })
    }

    /// Resolve conventions for an Interest Rate Future contract.
    pub fn require_ir_future(
        &self,
        id: &IrFutureContractId,
    ) -> Result<&IrFutureConventions> {
        self.ir_future.get(id).ok_or_else(|| {
            Error::Validation(format!(
                "Missing IR future conventions for '{}'. check ir_future_conventions.json",
                id
            ))
        })
    }
}
