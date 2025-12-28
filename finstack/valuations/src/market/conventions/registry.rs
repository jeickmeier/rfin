//! Global registry for market conventions.

use super::defs::{
    CdsConventions, InflationSwapConventions, IrFutureConventions, OptionConventions,
    RateIndexConventions, SwaptionConventions,
};
use super::ids::{
    CdsConventionKey, IndexId, InflationSwapConventionId, IrFutureContractId, OptionConventionId,
    SwaptionConventionId,
};
use finstack_core::collections::HashMap;
use finstack_core::{Error, Result};
use std::sync::OnceLock;

/// Global registry of market conventions.
///
/// This registry provides a single source of truth for convention lookups, ensuring strict
/// handling of missing data. Conventions are loaded from embedded JSON data on first access
/// and cached for the lifetime of the program.
///
/// # Thread Safety
///
/// The registry is thread-safe and can be accessed concurrently from multiple threads.
/// The singleton is initialized lazily on first access.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::conventions::registry::ConventionRegistry;
/// use finstack_valuations::market::conventions::ids::IndexId;
///
/// let registry = ConventionRegistry::global();
/// let conv = registry.require_rate_index(&IndexId::new("USD-SOFR-OIS"))?;
/// assert_eq!(conv.currency, finstack_core::currency::Currency::USD);
/// # Ok::<(), finstack_core::Error>(())
/// ```
#[derive(Debug, Default)]
pub struct ConventionRegistry {
    /// Registry of Rate Index conventions.
    pub rate_index: HashMap<IndexId, RateIndexConventions>,
    /// Registry of CDS conventions.
    pub cds: HashMap<CdsConventionKey, CdsConventions>,
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
    ///
    /// This constructor is primarily used for testing. In production, use [`global()`](Self::global)
    /// to access the singleton registry loaded from embedded JSON data.
    ///
    /// # Arguments
    ///
    /// * `rate_index` - Map of rate index IDs to conventions
    /// * `cds` - Map of CDS convention keys to conventions
    /// * `swaption` - Map of swaption convention IDs to conventions
    /// * `inflation_swap` - Map of inflation swap convention IDs to conventions
    /// * `option` - Map of option convention IDs to conventions
    /// * `ir_future` - Map of IR future contract IDs to conventions
    ///
    /// # Returns
    ///
    /// A new `ConventionRegistry` instance.
    pub fn new(
        rate_index: HashMap<IndexId, RateIndexConventions>,
        cds: HashMap<CdsConventionKey, CdsConventions>,
        swaption: HashMap<SwaptionConventionId, SwaptionConventions>,
        inflation_swap: HashMap<InflationSwapConventionId, InflationSwapConventions>,
        option: HashMap<OptionConventionId, OptionConventions>,
        ir_future: HashMap<IrFutureContractId, IrFutureConventions>,
    ) -> Self {
        Self {
            rate_index,
            cds,
            swaption,
            inflation_swap,
            option,
            ir_future,
        }
    }

    /// Access the global singleton registry.
    ///
    /// This will be initialized with embedded JSON data on the first call. The registry
    /// is loaded from embedded JSON files in `data/conventions/` and cached for the
    /// lifetime of the program.
    ///
    /// # Returns
    ///
    /// A reference to the global `ConventionRegistry` instance.
    ///
    /// # Panics
    ///
    /// Panics if convention data cannot be loaded from embedded JSON files. This should
    /// only occur if the build process is misconfigured.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::conventions::registry::ConventionRegistry;
    ///
    /// let registry = ConventionRegistry::global();
    /// // Registry is now initialized and ready to use
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if any embedded conventions file is corrupted or malformed.
    /// This is intentional: corrupted embedded data represents a build/packaging error
    /// that cannot be recovered at runtime and should fail fast during startup.
    #[allow(clippy::expect_used)]
    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<ConventionRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| ConventionRegistry {
            rate_index: super::loaders::rate_index::load_registry()
                .expect("Failed to load embedded rate index conventions registry"),
            cds: super::loaders::cds::load_registry()
                .expect("Failed to load embedded CDS conventions registry"),
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
    /// # Arguments
    ///
    /// * `id` - The rate index identifier
    ///
    /// # Returns
    ///
    /// `Ok(&RateIndexConventions)` if found, or `Err` with a validation error if not found.
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` if the index is not found in the registry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::conventions::registry::ConventionRegistry;
    /// use finstack_valuations::market::conventions::ids::IndexId;
    ///
    /// let registry = ConventionRegistry::global();
    /// let conv = registry.require_rate_index(&IndexId::new("USD-SOFR-OIS"))?;
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
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
    /// # Arguments
    ///
    /// * `key` - The CDS convention key (currency + doc clause)
    ///
    /// # Returns
    ///
    /// `Ok(&CdsConventions)` if found, or `Err` with a validation error if not found.
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` if the key is not found in the registry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::conventions::registry::ConventionRegistry;
    /// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
    /// use finstack_core::currency::Currency;
    ///
    /// let registry = ConventionRegistry::global();
    /// let key = CdsConventionKey {
    ///     currency: Currency::USD,
    ///     doc_clause: CdsDocClause::IsdaNa,
    /// };
    /// let conv = registry.require_cds(&key)?;
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn require_cds(&self, key: &CdsConventionKey) -> Result<&CdsConventions> {
        self.cds.get(key).ok_or_else(|| {
            Error::Validation(format!(
                "Missing CDS conventions for '{}'. check cds_conventions.json",
                key
            ))
        })
    }

    /// Resolve conventions for a Swaption.
    ///
    /// # Arguments
    ///
    /// * `id` - The swaption convention identifier
    ///
    /// # Returns
    ///
    /// `Ok(&SwaptionConventions)` if found, or `Err` with a validation error if not found.
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` if the ID is not found in the registry.
    pub fn require_swaption(&self, id: &SwaptionConventionId) -> Result<&SwaptionConventions> {
        self.swaption.get(id).ok_or_else(|| {
            Error::Validation(format!(
                "Missing swaption conventions for '{}'. check swaption_conventions.json",
                id
            ))
        })
    }

    /// Resolve conventions for an Inflation Swap.
    ///
    /// # Arguments
    ///
    /// * `id` - The inflation swap convention identifier
    ///
    /// # Returns
    ///
    /// `Ok(&InflationSwapConventions)` if found, or `Err` with a validation error if not found.
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` if the ID is not found in the registry.
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
    ///
    /// # Arguments
    ///
    /// * `id` - The option convention identifier
    ///
    /// # Returns
    ///
    /// `Ok(&OptionConventions)` if found, or `Err` with a validation error if not found.
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` if the ID is not found in the registry.
    pub fn require_option(&self, id: &OptionConventionId) -> Result<&OptionConventions> {
        self.option.get(id).ok_or_else(|| {
            Error::Validation(format!(
                "Missing option conventions for '{}'. check option_conventions.json",
                id
            ))
        })
    }

    /// Resolve conventions for an Interest Rate Future contract.
    ///
    /// # Arguments
    ///
    /// * `id` - The IR future contract identifier
    ///
    /// # Returns
    ///
    /// `Ok(&IrFutureConventions)` if found, or `Err` with a validation error if not found.
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` if the ID is not found in the registry.
    pub fn require_ir_future(&self, id: &IrFutureContractId) -> Result<&IrFutureConventions> {
        self.ir_future.get(id).ok_or_else(|| {
            Error::Validation(format!(
                "Missing IR future conventions for '{}'. check ir_future_conventions.json",
                id
            ))
        })
    }
}
