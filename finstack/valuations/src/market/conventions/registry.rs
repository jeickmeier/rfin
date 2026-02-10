//! Global registry for market conventions.

use super::defs::{
    CdsConventions, InflationSwapConventions, IrFutureConventions, OptionConventions,
    RateIndexConventions, SwaptionConventions,
};
use super::ids::{
    CdsConventionKey, IndexId, InflationSwapConventionId, IrFutureContractId, OptionConventionId,
    SwaptionConventionId,
};
use finstack_core::HashMap;
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
/// use finstack_valuations::market::conventions::ConventionRegistry;
/// use finstack_valuations::market::conventions::ids::IndexId;
///
/// let registry = ConventionRegistry::try_global()?;
/// let conv = registry.require_rate_index(&IndexId::new("USD-SOFR-OIS"))?;
/// assert_eq!(conv.currency, finstack_core::currency::Currency::USD);
/// # Ok::<(), finstack_core::Error>(())
/// ```
#[derive(Debug, Default)]
pub struct ConventionRegistry {
    /// Registry of Rate Index conventions.
    rate_index: HashMap<IndexId, RateIndexConventions>,
    /// Registry of CDS conventions.
    cds: HashMap<CdsConventionKey, CdsConventions>,
    /// Registry of Swaption conventions.
    swaption: HashMap<SwaptionConventionId, SwaptionConventions>,
    /// Registry of Inflation Swap conventions.
    inflation_swap: HashMap<InflationSwapConventionId, InflationSwapConventions>,
    /// Registry of Option conventions.
    option: HashMap<OptionConventionId, OptionConventions>,
    /// Registry of Interest Rate Futures conventions.
    ir_future: HashMap<IrFutureContractId, IrFutureConventions>,
}

impl ConventionRegistry {
    /// Create a new registry from in-memory maps.
    ///
    /// This constructor is primarily used for testing. In production, use `global()`
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

    fn not_found(id: impl Into<String>) -> Error {
        finstack_core::InputError::NotFound { id: id.into() }.into()
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
    /// use finstack_valuations::market::conventions::ConventionRegistry;
    ///
    /// # fn example() -> finstack_core::Result<()> {
    /// let registry = ConventionRegistry::try_global()?;
    /// // Registry is now initialized and ready to use
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if any embedded conventions file is corrupted or malformed.
    /// This is intentional: corrupted embedded data represents a build/packaging error
    /// that cannot be recovered at runtime and should fail fast during startup.
    ///
    /// For callers that prefer structured error handling (e.g., bindings / services),
    /// use [`try_global()`](Self::try_global).
    #[allow(clippy::expect_used)]
    #[allow(dead_code)]
    pub(crate) fn global() -> &'static Self {
        Self::try_global().expect("Failed to load embedded conventions registry")
    }

    /// Access the global singleton registry (fallible).
    ///
    /// This is the error-returning variant of `global()`. It is preferred in
    /// library/binding contexts where panics are unacceptable.
    pub fn try_global() -> Result<&'static Self> {
        static REGISTRY: OnceLock<ConventionRegistry> = OnceLock::new();
        if let Some(reg) = REGISTRY.get() {
            return Ok(reg);
        }

        // Build outside the lock. If multiple threads race here, all but one `set` will lose and
        // we will return the stored singleton.
        let built = ConventionRegistry {
            rate_index: super::loaders::rate_index::load_registry()?,
            cds: super::loaders::cds::load_registry()?,
            swaption: super::loaders::swaption::load_registry()?,
            inflation_swap: super::loaders::inflation_swap::load_registry()?,
            option: super::loaders::option::load_registry()?,
            ir_future: super::loaders::ir_future::load_registry()?,
        };
        let _ = REGISTRY.set(built);

        REGISTRY.get().ok_or_else(|| {
            Error::Validation("ConventionRegistry::try_global failed to initialize".to_string())
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
    /// `Ok(&RateIndexConventions)` if found, or `Err` with an `InputError::NotFound` if missing.
    ///
    /// # Errors
    ///
    /// Returns `InputError::NotFound` if the index is not found in the registry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::conventions::ConventionRegistry;
    /// use finstack_valuations::market::conventions::ids::IndexId;
    ///
    /// let registry = ConventionRegistry::try_global()?;
    /// let conv = registry.require_rate_index(&IndexId::new("USD-SOFR-OIS"))?;
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn require_rate_index(&self, id: &IndexId) -> Result<&RateIndexConventions> {
        self.rate_index
            .get(id)
            .ok_or_else(|| Self::not_found(id.to_string()))
    }

    /// Resolve conventions for a CDS key.
    ///
    /// # Arguments
    ///
    /// * `key` - The CDS convention key (currency + doc clause)
    ///
    /// # Returns
    ///
    /// `Ok(&CdsConventions)` if found, or `Err` with an `InputError::NotFound` if missing.
    ///
    /// # Errors
    ///
    /// Returns `InputError::NotFound` if the key is not found in the registry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::conventions::ConventionRegistry;
    /// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
    /// use finstack_core::currency::Currency;
    ///
    /// let registry = ConventionRegistry::try_global()?;
    /// let key = CdsConventionKey {
    ///     currency: Currency::USD,
    ///     doc_clause: CdsDocClause::IsdaNa,
    /// };
    /// let conv = registry.require_cds(&key)?;
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn require_cds(&self, key: &CdsConventionKey) -> Result<&CdsConventions> {
        self.cds
            .get(key)
            .ok_or_else(|| Self::not_found(key.to_string()))
    }

    /// Resolve conventions for a Swaption.
    ///
    /// # Arguments
    ///
    /// * `id` - The swaption convention identifier
    ///
    /// # Returns
    ///
    /// `Ok(&SwaptionConventions)` if found, or `Err` with an `InputError::NotFound` if missing.
    ///
    /// # Errors
    ///
    /// Returns `InputError::NotFound` if the ID is not found in the registry.
    pub fn require_swaption(&self, id: &SwaptionConventionId) -> Result<&SwaptionConventions> {
        self.swaption
            .get(id)
            .ok_or_else(|| Self::not_found(id.to_string()))
    }

    /// Resolve conventions for an Inflation Swap.
    ///
    /// # Arguments
    ///
    /// * `id` - The inflation swap convention identifier
    ///
    /// # Returns
    ///
    /// `Ok(&InflationSwapConventions)` if found, or `Err` with an `InputError::NotFound` if missing.
    ///
    /// # Errors
    ///
    /// Returns `InputError::NotFound` if the ID is not found in the registry.
    pub fn require_inflation_swap(
        &self,
        id: &InflationSwapConventionId,
    ) -> Result<&InflationSwapConventions> {
        self.inflation_swap
            .get(id)
            .ok_or_else(|| Self::not_found(id.to_string()))
    }

    /// Resolve conventions for an Option.
    ///
    /// # Arguments
    ///
    /// * `id` - The option convention identifier
    ///
    /// # Returns
    ///
    /// `Ok(&OptionConventions)` if found, or `Err` with an `InputError::NotFound` if missing.
    ///
    /// # Errors
    ///
    /// Returns `InputError::NotFound` if the ID is not found in the registry.
    pub fn require_option(&self, id: &OptionConventionId) -> Result<&OptionConventions> {
        self.option
            .get(id)
            .ok_or_else(|| Self::not_found(id.to_string()))
    }

    /// Resolve conventions for an Interest Rate Future contract.
    ///
    /// # Arguments
    ///
    /// * `id` - The IR future contract identifier
    ///
    /// # Returns
    ///
    /// `Ok(&IrFutureConventions)` if found, or `Err` with an `InputError::NotFound` if missing.
    ///
    /// # Errors
    ///
    /// Returns `InputError::NotFound` if the ID is not found in the registry.
    pub fn require_ir_future(&self, id: &IrFutureContractId) -> Result<&IrFutureConventions> {
        self.ir_future
            .get(id)
            .ok_or_else(|| Self::not_found(id.to_string()))
    }
}
