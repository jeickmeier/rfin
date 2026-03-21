//! Core instrument traits and metadata infrastructure.
//!
//! Provides the fundamental [`Instrument`] trait that all financial instruments
//! implement, along with [`Attributes`] for tagging, selection, and scenario filtering.
//!
//! # Key Types
//!
//! - [`Instrument`]: Unified trait combining identity, attributes, and pricing methods
//! - [`Attributes`]: Tag-based metadata for categorization and scenario selection
//!
//! # Examples
//!
//! ## Basic Instrument Usage
//!
//! ```rust
//! use finstack_valuations::instruments::{Bond, Instrument};
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::dates::create_date;
//! use finstack_core::types::Rate;
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let issue = create_date(2025, Month::January, 15)?;
//! let maturity = create_date(2030, Month::January, 15)?;
//! let bond = Bond::fixed(
//!     "BOND-001",
//!     Money::new(1_000_000.0, Currency::USD),
//!     Rate::from_percent(5.0),
//!     issue,
//!     maturity,
//!     "USD-OIS"
//! )?;
//!
//! // Instrument trait methods
//! assert_eq!(bond.id(), "BOND-001");
//! # Ok(())
//! # }
//! ```
//!
//! ## Attributes and Selection
//!
//! ```rust
//! use finstack_valuations::instruments::Attributes;
//!
//! let attrs = Attributes::new()
//!     .with_tag("high-yield")
//!     .with_tag("energy")
//!     .with_meta("sector", "oil-gas")
//!     .with_meta("rating", "BB+");
//!
//! assert!(attrs.has_tag("high-yield"));
//! assert_eq!(attrs.get_meta("sector"), Some("oil-gas"));
//!
//! // Selector matching
//! assert!(attrs.matches_selector("tag:energy"));
//! assert!(attrs.matches_selector("meta:rating=BB+"));
//! assert!(attrs.matches_selector("*")); // Matches all
//! ```

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common_impl::dependencies::MarketDependencies;
use crate::metrics::risk::MarketHistory;
use crate::metrics::MetricId;
use crate::pricer::{
    actionable_unknown_pricer_message, shared_standard_registry, InstrumentType, ModelKey,
    PricerRegistry, PricingError,
};
use finstack_core::config::FinstackConfig;
use finstack_core::market_data::context::MarketContext;
pub use finstack_core::types::Attributes;
use finstack_core::{currency::Currency, dates::Date, money::Money, types::CurveId};
use smallvec::SmallVec;
use std::any::Any;
use std::sync::Arc;

// =============================================================================
// Pricing Options
// =============================================================================

/// Optional overrides for a pricing-and-metrics request.
///
/// This struct consolidates optional parameters for `Instrument::price_with_metrics`,
/// replacing the proliferation of `_with_config`, `_with_market_history` variants.
///
/// # Examples
///
/// ## Basic usage (no options)
///
/// ```ignore
/// let result = instrument.price_with_metrics(
///     &market,
///     as_of,
///     &metrics,
///     PricingOptions::default(),
/// )?;
/// ```
///
/// ## With custom config
///
/// ```ignore
/// let opts = PricingOptions::default().with_config(&my_config);
/// let result = instrument.price_with_metrics(&market, as_of, &metrics, opts)?;
/// ```
///
/// ## With market history for VaR
///
/// ```ignore
/// let opts = PricingOptions::default().with_market_history(history);
/// let result = instrument.price_with_metrics(&market, as_of, &metrics, opts)?;
/// ```
#[derive(Clone, Default)]
pub struct PricingOptions {
    /// Optional configuration for metric computation (bump sizes, tolerances, etc.)
    pub config: Option<Arc<FinstackConfig>>,
    /// Optional market history for Historical VaR / Expected Shortfall metrics
    pub market_history: Option<Arc<MarketHistory>>,
    /// Optional explicit pricing model override.
    ///
    /// When `None`, [`Instrument::price_with_metrics`] uses
    /// [`Instrument::default_model`]. Set this to select a different registered
    /// pricing path, such as hazard-rate or tree/OAS pricing, without dropping
    /// down to [`crate::pricer::PricerRegistry`] directly.
    pub model: Option<ModelKey>,
    /// Optional explicit pricer registry override.
    pub registry: Option<Arc<PricerRegistry>>,
}

impl PricingOptions {
    /// Create new pricing options with no extras.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the configuration for metric computation.
    ///
    /// The config controls sensitivity bump sizes and other calculation parameters.
    pub fn with_config(mut self, cfg: &FinstackConfig) -> Self {
        self.config = Some(Arc::new(cfg.clone()));
        self
    }

    /// Set the market history for Historical VaR / Expected Shortfall.
    ///
    /// Required for computing `MetricId::HVar` and `MetricId::ExpectedShortfall`.
    pub fn with_market_history(mut self, history: Arc<MarketHistory>) -> Self {
        self.market_history = Some(history);
        self
    }

    /// Set the pricing model for this pricing request.
    ///
    /// Most callers can stay on [`Instrument::price_with_metrics`] and use this
    /// override only when they need a non-default registered model.
    pub fn with_model(mut self, model: ModelKey) -> Self {
        self.model = Some(model);
        self
    }

    /// Set an explicit pricer registry override for this pricing request.
    pub fn with_registry(mut self, registry: Arc<PricerRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }
}
/// Type alias for curve ID collections that are typically small (0-2 items).
///
/// Most instruments depend on 1-2 curves. Using SmallVec avoids heap allocation
/// for the common case while still supporting instruments with more curve dependencies.
pub type CurveIdVec = SmallVec<[CurveId; 2]>;

/// Trait-object alias for instrument values used by portfolio/scenario plumbing.
pub type DynInstrument = dyn Instrument;

/// Metadata for instrument categorization, tagging, and scenario selection.
///
/// Attributes provide a flexible tagging system for organizing instruments,
/// applying scenarios, and filtering portfolios. Tags are simple strings for
/// broad categorization, while metadata key-value pairs store structured information.
///
/// # Tag-Based Selection
///
/// Tags enable coarse-grained filtering:
/// - Asset class: "equity", "fixed-income", "credit"
/// - Risk profile: "high-yield", "investment-grade"
/// - Sector: "technology", "financials", "energy"
/// - Custom: Any domain-specific categories
///
/// # Metadata Pairs
///
/// Key-value metadata stores structured attributes:
/// - Credit ratings: `("rating", "AA+")`
/// - Geographic region: `("region", "north-america")`
/// - Counterparty: `("counterparty", "JPMORGAN")`
/// - Desk/book: `("desk", "rates-trading")`
///
/// # Selector Patterns
///
/// Attributes support pattern-based selection for scenarios:
/// - `"*"`: Matches all instruments
/// - `"tag:high-yield"`: Matches instruments with the "high-yield" tag
/// - `"meta:sector=technology"`: Matches instruments with sector metadata
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::Attributes;
///
/// let mut attrs = Attributes::new()
///     .with_tag("corporate")
///     .with_tag("investment-grade")
///     .with_meta("issuer", "AAPL")
///     .with_meta("rating", "AA+");
///
/// // Check tags
/// assert!(attrs.has_tag("corporate"));
/// assert!(!attrs.has_tag("high-yield"));
///
/// // Access metadata
/// assert_eq!(attrs.get_meta("issuer"), Some("AAPL"));
/// assert_eq!(attrs.get_meta("rating"), Some("AA+"));
///
/// // Pattern matching
/// assert!(attrs.matches_selector("tag:corporate"));
/// assert!(attrs.matches_selector("meta:issuer=AAPL"));
/// assert!(!attrs.matches_selector("tag:high-yield"));
/// ```
/// Unified instrument trait combining identity, attributes, and pricing.
///
/// This is the primary trait for all financial instruments in the valuation framework.
/// It provides a consistent interface for instrument identification, metadata access,
/// and pricing operations. All concrete instrument types (Bond, Swap, Option, etc.)
/// implement this trait.
///
/// # Core Responsibilities
///
/// 1. **Identity**: Unique instrument identifiers and type information
/// 2. **Metadata**: Tags and attributes for categorization and scenario selection
/// 3. **Pricing**: Present value calculation with optional risk metrics
/// 4. **Type Safety**: Strongly-typed instrument dispatch and downcasting
///
/// # Pricing Methods
///
/// The trait provides two pricing methods with different performance characteristics:
///
/// - [`value()`](Instrument::value): Fast NPV-only calculation (no metrics)
/// - [`price_with_metrics()`](Instrument::price_with_metrics): NPV plus requested risk metrics
///
/// `value()` is the canonical rounded pricing path and returns a [`Money`] amount
/// in the instrument's reporting currency. [`value_raw()`](Instrument::value_raw)
/// should be used only when the caller needs the same economics before currency
/// rounding, typically for finite-difference risk calculations.
///
/// # Implementation Guidelines
///
/// Instruments should:
/// - Return unique, stable identifiers from `id()`
/// - Map to the correct `InstrumentType` variant in `key()`
/// - Implement efficient `value()` for hot paths (portfolio aggregation)
/// - Compute metrics on-demand in `price_with_metrics()`
/// - Support `clone_box()` for trait object cloning
///
/// # Examples
///
/// ## Basic Pricing
///
/// ```rust
/// use finstack_valuations::instruments::{Bond, Instrument};
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_core::dates::create_date;
/// use finstack_core::types::Rate;
/// use finstack_core::market_data::context::MarketContext;
/// use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let issue = create_date(2025, Month::January, 15)?;
/// let maturity = create_date(2030, Month::January, 15)?;
/// let bond = Bond::fixed(
///     "BOND-001",
///     Money::new(1_000_000.0, Currency::USD),
///     Rate::from_percent(5.0),
///     issue,
///     maturity,
///     "USD-OIS"
/// )?;
///
/// let market = MarketContext::new();
/// let as_of = create_date(2025, Month::January, 1)?;
///
/// // Fast NPV calculation
/// // let pv = bond.value(&market, as_of)?;
/// // println!("PV: {}", pv);
/// # Ok(())
/// # }
/// ```
///
/// ## Pricing with Metrics
///
/// ```rust
/// use finstack_valuations::instruments::{Bond, Instrument};
/// use finstack_valuations::metrics::MetricId;
/// # use finstack_core::currency::Currency;
/// # use finstack_core::money::Money;
/// # use finstack_core::dates::create_date;
/// # use finstack_core::types::Rate;
/// # use finstack_core::market_data::context::MarketContext;
/// # use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let issue = create_date(2025, Month::January, 15)?;
/// # let maturity = create_date(2030, Month::January, 15)?;
/// # let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
/// #     Rate::from_percent(5.0), issue, maturity, "USD-OIS");
/// # let market = MarketContext::new();
/// # let as_of = create_date(2025, Month::January, 1)?;
///
/// // Request specific metrics
/// let metrics = vec![MetricId::Ytm, MetricId::DurationMod, MetricId::Dv01];
/// // let result = bond.price_with_metrics(&market, as_of, &metrics)?;
///
/// // Access metrics
/// // println!("YTM: {:.2}%", result.measures["ytm"] * 100.0);
/// // println!("Duration: {:.2}", result.measures["duration_mod"]);
/// # Ok(())
/// # }
/// ```
///
/// ## Trait Object Usage
///
/// ```rust
/// use finstack_valuations::instruments::{Bond, Instrument};
/// # use finstack_core::currency::Currency;
/// # use finstack_core::money::Money;
/// # use finstack_core::dates::create_date;
/// # use finstack_core::types::Rate;
/// # use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let issue = create_date(2025, Month::January, 15)?;
/// # let maturity = create_date(2030, Month::January, 15)?;
/// let bond = Bond::fixed(
///     "BOND-001",
///     Money::new(1_000_000.0, Currency::USD),
///     Rate::from_percent(5.0),
///     issue,
///     maturity,
///     "USD-OIS"
/// )?;
///
/// // Use as trait object
/// let instrument: Box<dyn Instrument> = Box::new(bond);
/// assert_eq!(instrument.id(), "BOND-001");
///
/// // Clone trait object
/// let cloned = instrument.clone_box();
/// assert_eq!(cloned.id(), "BOND-001");
/// # Ok(())
/// # }
/// ```
/// Implements the standard boilerplate methods for the [`Instrument`] trait.
///
/// Most instruments store their ID as `self.id` (an `InstrumentId`), attributes as
/// `self.attributes`, and have a fixed [`InstrumentType`] key. This macro provides
/// default implementations for the mechanical methods, leaving only the
/// instrument-specific methods (`value`, `market_dependencies`, etc.) to be
/// implemented manually.
///
/// # Requirements
///
/// The implementing type must:
/// - Have a field `id: InstrumentId` (with `.as_str()` method)
/// - Have a field `attributes: Attributes`
/// - Implement `Clone`
///
/// # Example
///
/// ```rust,ignore
/// impl Instrument for MyInstrument {
///     impl_instrument_base!(InstrumentType::MyInstrument);
///
///     fn value(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
///         // instrument-specific pricing logic
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_instrument_base {
    ($key:expr) => {
        fn id(&self) -> &str {
            self.id.as_str()
        }

        fn key(&self) -> $crate::pricer::InstrumentType {
            $key
        }

        fn as_any(&self) -> &dyn ::std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
            self
        }

        fn attributes(&self) -> &$crate::instruments::common_impl::traits::Attributes {
            &self.attributes
        }

        fn attributes_mut(&mut self) -> &mut $crate::instruments::common_impl::traits::Attributes {
            &mut self.attributes
        }

        fn clone_box(&self) -> Box<$crate::instruments::DynInstrument> {
            Box::new(self.clone())
        }
    };
}

/// Common interface implemented by all valuation instruments.
///
/// This trait defines the minimal identity, typing, metadata, and clone
/// behavior required for instrument dispatch, pricing, and serialization-safe
/// handling across the library.
pub trait Instrument: Send + Sync {
    /// Get the instrument's unique identifier.
    ///
    /// Returns a stable string identifier that uniquely identifies this
    /// instrument within a portfolio or system. IDs should be immutable
    /// and consistent across serialization boundaries.
    ///
    /// # Returns
    ///
    /// String slice containing the instrument ID
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use finstack_core::types::Rate;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed(
    ///     "US-TREASURY-5Y-001",
    ///     Money::new(1_000_000.0, Currency::USD),
    ///     Rate::from_percent(5.0),
    ///     issue,
    ///     maturity,
    ///     "USD-OIS"
    /// )?;
    ///
    /// assert_eq!(bond.id(), "US-TREASURY-5Y-001");
    /// # Ok(())
    /// # }
    /// ```
    fn id(&self) -> &str;

    /// Get the strongly-typed instrument key for pricer dispatch.
    ///
    /// Returns the `InstrumentType` enum variant corresponding to this
    /// instrument's type. Used by the pricing registry to route instruments
    /// to the correct pricer implementation.
    ///
    /// # Returns
    ///
    /// `InstrumentType` enum variant (e.g., `InstrumentType::Bond`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// use finstack_valuations::pricer::InstrumentType;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use finstack_core::types::Rate;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     Rate::from_percent(5.0), issue, maturity, "USD-OIS")?;
    ///
    /// assert_eq!(bond.key(), InstrumentType::Bond);
    /// # Ok(())
    /// # }
    /// ```
    fn key(&self) -> InstrumentType;

    /// Access to the concrete type for downcasting.
    ///
    /// Enables downcasting from `dyn Instrument` trait objects to concrete
    /// instrument types using `Any::downcast_ref()`.
    ///
    /// # Returns
    ///
    /// Reference to `dyn Any` for dynamic type checking
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use finstack_core::types::Rate;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     Rate::from_percent(5.0), issue, maturity, "USD-OIS")?;
    ///
    /// let instrument: &dyn Instrument = &bond;
    /// let concrete_bond: Option<&Bond> = instrument.as_any().downcast_ref::<Bond>();
    /// assert!(concrete_bond.is_some());
    /// # Ok(())
    /// # }
    /// ```
    fn as_any(&self) -> &dyn Any;

    /// Access to the concrete type for mutable downcasting.
    ///
    /// Enables mutable downcasting from `dyn Instrument` trait objects to
    /// concrete instrument types using `Any::downcast_mut()`. Used by
    /// scenario adapters that need to modify typed instrument state
    /// (e.g., correlation shocks on `StructuredCredit`).
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Expose this instrument as a [`CashflowProvider`] when supported.
    ///
    /// This hook enables generic components (e.g., scenario time-roll and
    /// attribution engines) to obtain cashflow schedules without relying on
    /// manual downcasting for each concrete instrument type.
    ///
    /// Instruments that implement [`CashflowProvider`] should override this
    /// method to return `Some(self)`. The default implementation returns
    /// `None`, indicating that no cashflow schedule is available.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::cashflow::CashflowProvider;
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use finstack_core::types::Rate;
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed(
    ///     "BOND-001",
    ///     Money::new(1_000_000.0, Currency::USD),
    ///     Rate::from_percent(5.0),
    ///     issue,
    ///     maturity,
    ///     "USD-OIS"
    /// )?;
    ///
    /// let inst: &dyn Instrument = &bond;
    /// if let Some(cf) = inst.as_cashflow_provider() {
    ///     let curves = MarketContext::new();
    ///     let _schedule = cf.build_dated_flows(&curves, issue)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn as_cashflow_provider(&self) -> Option<&dyn CashflowProvider> {
        None
    }

    /// Pre-seed a metric context with instrument-specific cached data.
    ///
    /// This hook allows instruments to supply pre-computed values (e.g., cashflows,
    /// discount curve ID, notional) into a [`MetricContext`](crate::metrics::MetricContext)
    /// before the metrics pipeline runs, avoiding redundant computation.
    ///
    /// The default implementation is a no-op. Instruments with expensive cashflow
    /// generation (e.g., structured credit waterfall simulation) should override
    /// this to pre-seed the context.
    ///
    /// # Arguments
    ///
    /// * `context` - Mutable metric context to seed
    /// * `market` - Market data context (for computing cashflows if needed)
    /// * `as_of` - Valuation date
    fn seed_metric_context(
        &self,
        _context: &mut crate::metrics::MetricContext,
        _market: &MarketContext,
        _as_of: Date,
    ) {
        // Default no-op
    }

    /// Expose this instrument as a [`finstack_margin::Marginable`] when supported.
    ///
    /// This hook enables the portfolio margin aggregator to obtain margin
    /// sensitivities without relying on manual downcasting for each concrete
    /// instrument type.
    ///
    /// Instruments that implement [`finstack_margin::Marginable`] should override
    /// this method to return `Some(self)`. The default implementation returns
    /// `None`, indicating that the instrument does not support margin calculations.
    fn as_marginable(&self) -> Option<&dyn finstack_margin::Marginable> {
        None
    }

    /// Get immutable reference to instrument attributes.
    ///
    /// Returns attributes containing tags and metadata for categorization
    /// and scenario selection.
    ///
    /// # Returns
    ///
    /// Reference to `Attributes`
    fn attributes(&self) -> &Attributes;

    /// Get mutable reference to instrument attributes.
    ///
    /// Allows modifying tags and metadata after instrument construction.
    ///
    /// # Returns
    ///
    /// Mutable reference to `Attributes`
    fn attributes_mut(&mut self) -> &mut Attributes;

    /// Get mutable reference to the full pricing overrides bag.
    ///
    /// This remains available as a compatibility hook while the public surface
    /// transitions away from a single catch-all overrides struct.
    fn pricing_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        None
    }

    /// Get immutable reference to the full pricing overrides bag.
    fn pricing_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        None
    }

    /// Get mutable reference to scenario-only pricing adjustments.
    ///
    /// # Returns
    ///
    /// `Some(&mut ScenarioPricingOverrides)` if the instrument supports scenario adjustments,
    /// `None` otherwise.
    ///
    /// # Default Implementation
    ///
    /// Returns a mutable view into
    /// [`crate::instruments::pricing_overrides::PricingOverrides::scenario`] when
    /// the instrument exposes the compatibility overrides wrapper.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
    /// fn apply_price_shock(instrument: &mut dyn Instrument, shock_pct: f64) {
    ///     if let Some(overrides) = instrument.scenario_overrides_mut() {
    ///         overrides.scenario_price_shock_pct = Some(shock_pct);
    ///     }
    /// }
    /// ```
    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::ScenarioPricingOverrides> {
        self.pricing_overrides_mut()
            .map(|overrides| &mut overrides.scenario)
    }

    /// Get immutable reference to scenario-only pricing adjustments.
    ///
    /// # Returns
    ///
    /// `Some(&ScenarioPricingOverrides)` if the instrument supports scenario adjustments,
    /// `None` otherwise.
    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::ScenarioPricingOverrides> {
        self.pricing_overrides()
            .map(|overrides| &overrides.scenario)
    }

    /// Clone this instrument as a boxed trait object.
    ///
    /// Enables cloning instruments behind trait objects. Required because
    /// `Clone` cannot be made into a trait object directly.
    ///
    /// # Returns
    ///
    /// Boxed clone of the instrument
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use finstack_core::types::Rate;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     Rate::from_percent(5.0), issue, maturity, "USD-OIS")?;
    ///
    /// let instrument: Box<dyn Instrument> = Box::new(bond);
    /// let cloned = instrument.clone_box();
    /// assert_eq!(cloned.id(), instrument.id());
    /// # Ok(())
    /// # }
    /// ```
    fn clone_box(&self) -> Box<DynInstrument>;

    /// Returns a normalized instrument for computing spread/yield metrics.
    ///
    /// When a non-discounting model (hazard, MC, OAS) produces a price, the
    /// standard metrics pipeline solves z-spread, YTM, duration, etc. against
    /// the instrument's cashflows.  For instruments with non-standard cashflow
    /// structures (e.g. PIK bonds with compounded terminal notional), the
    /// "native" z-spread is not comparable to a standard cash-pay bond.
    ///
    /// This method returns a version of the instrument whose cashflows are
    /// normalized to the standard (discount-model) basis.  The default
    /// implementation returns `self.clone_box()` — instruments with standard
    /// cashflows need no override.  Bonds override this to convert PIK coupon
    /// type to Cash so that spread metrics are on a cash-equivalent basis.
    ///
    /// This method affects spread- and yield-style metrics only. It should not be
    /// used to change the economic basis of PV itself or unrelated risk measures.
    fn metrics_equivalent(&self) -> Box<DynInstrument> {
        self.clone_box()
    }

    // === Pricing Methods ===

    /// Compute the present value only (fast path, no metrics).
    ///
    /// This is the performance-optimized method for obtaining just the NPV
    /// without computing any risk metrics. Use this in hot paths like
    /// portfolio aggregation where metrics are not needed.
    ///
    /// The returned [`Money`] is the canonical rounded pricing output. Callers
    /// that need pre-rounding arithmetic for sensitivities should use
    /// [`Instrument::value_raw`] rather than inferring precision from `Money`.
    ///
    /// # Arguments
    ///
    /// * `market` - Market data context containing discount curves, forward curves,
    ///   volatility surfaces, and other pricing inputs
    /// * `as_of` - Valuation date (T+0)
    ///
    /// # Returns
    ///
    /// Present value in the instrument's native currency
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Required market curves are missing
    /// - Instrument parameters are invalid
    /// - Numerical computation fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use finstack_core::types::Rate;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     Rate::from_percent(5.0), issue, maturity, "USD-OIS")?;
    ///
    /// let market = MarketContext::new();
    /// let as_of = create_date(2025, Month::January, 1)?;
    ///
    /// // Fast NPV-only calculation
    /// // let pv = bond.value(&market, as_of)?;
    /// // assert_eq!(pv.currency(), Currency::USD);
    /// # Ok(())
    /// # }
    /// ```
    fn value(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<Money>;

    /// Compute the present value as raw f64 (high precision path for risk calculations).
    ///
    /// This method returns the NPV as an unrounded f64, avoiding the precision loss
    /// that occurs when Money rounds to currency decimal places (e.g., 2 for USD).
    /// This is critical for finite difference sensitivity calculations where small
    /// PV differences matter (e.g., bucketed DV01, key-rate sensitivities).
    ///
    /// # Arguments
    ///
    /// * `market` - Market data context containing discount curves, forward curves, etc.
    /// * `as_of` - Valuation date (T+0)
    ///
    /// # Returns
    ///
    /// Present value as raw f64 without currency rounding
    ///
    /// # Default Implementation
    ///
    /// The default implementation delegates to `value()` and extracts the amount.
    /// Instruments with internal high-precision pricing should override this method
    /// to return the raw value before Money wrapping for better sensitivity accuracy.
    ///
    /// `value_raw()` should represent the same economics as [`Instrument::value`]
    /// before currency rounding, not a different pricing convention.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Used internally by risk calculators for high-precision sensitivities
    /// use finstack_core::currency::Currency;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::money::Money;
    /// use finstack_core::types::Rate;
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// use time::macros::date;
    ///
    /// # fn main() -> finstack_core::Result<()> {
    /// let instrument = Bond::fixed(
    ///     "BOND-001",
    ///     Money::new(1_000_000.0, Currency::USD),
    ///     Rate::from_percent(5.0),
    ///     date!(2025-01-15),
    ///     date!(2030-01-15),
    ///     "USD-OIS",
    /// )?;
    /// let market = MarketContext::new();
    /// let bumped_market = MarketContext::new();
    /// let as_of = date!(2025-01-15);
    /// let bump_bp = 1e-4; // 1bp = 0.0001
    ///
    /// let base_pv = instrument.value_raw(&market, as_of)?;
    /// let bumped_pv = instrument.value_raw(&bumped_market, as_of)?;
    /// let dv01 = (bumped_pv - base_pv) / bump_bp;
    /// # let _ = dv01;
    /// # Ok(())
    /// # }
    /// ```
    fn value_raw(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        Ok(self.value(market, as_of)?.amount())
    }

    /// Return the default pricing model for this instrument.
    ///
    /// Most instruments use [`ModelKey::Discounting`]. Instruments whose native
    /// pricing path uses a different model should override this method so the
    /// canonical pricing API preserves existing behavior.
    fn default_model(&self) -> ModelKey {
        ModelKey::Discounting
    }

    /// Compute present value with specified risk metrics.
    ///
    /// This method computes NPV plus any requested risk metrics (duration, DV01,
    /// Greeks, etc.). Metrics are computed on-demand based on the provided list,
    /// enabling efficient calculation of only the required sensitivities.
    ///
    /// This is the canonical pricing entry point for most callers. By default,
    /// it prices with [`Instrument::default_model`]. To force a different
    /// registered pricing path, pass [`PricingOptions::with_model`].
    ///
    /// PV is always returned in [`crate::results::ValuationResult::value`].
    /// Requested metrics are stored in `ValuationResult::measures` and must be
    /// interpreted using their [`crate::metrics::MetricId`] contracts.
    ///
    /// # Arguments
    ///
    /// * `market` - Market data context containing all pricing inputs
    /// * `as_of` - Valuation date (T+0)
    /// * `metrics` - List of metric IDs to compute (e.g., `MetricId::Dv01`)
    /// * `options` - Optional overrides for config, market history, registry,
    ///   or explicit model selection
    ///
    /// # Returns
    ///
    /// `ValuationResult` containing:
    /// - Present value
    /// - Computed risk metrics
    /// - Calculation metadata (timing, precision, etc.)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Required market data is missing
    /// - Metric calculation fails
    /// - Instrument configuration is invalid
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::{Bond, Instrument, PricingOptions};
    /// use finstack_valuations::metrics::MetricId;
    /// use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use finstack_core::types::Rate;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     Rate::from_percent(5.0), issue, maturity, "USD-OIS")?;
    ///
    /// let market = MarketContext::new();
    /// let as_of = create_date(2025, Month::January, 1)?;
    ///
    /// // Request specific metrics
    /// let metrics_to_compute = vec![
    ///     MetricId::Ytm,
    ///     MetricId::DurationMod,
    ///     MetricId::Dv01,
    /// ];
    ///
    /// let result = bond.price_with_metrics(
    ///     &market,
    ///     as_of,
    ///     &metrics_to_compute,
    ///     PricingOptions::default(),
    /// )?;
    /// println!("NPV: {}", result.value);
    /// println!("DV01: {}", result.measures.get("dv01").expect("should succeed"));
    /// # let _ = result;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::{Bond, Instrument, PricingOptions};
    /// use finstack_valuations::metrics::MetricId;
    /// use finstack_valuations::pricer::ModelKey;
    /// use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use finstack_core::types::Rate;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     Rate::from_percent(5.0), issue, maturity, "USD-OIS")?;
    ///
    /// let market = MarketContext::new();
    /// let as_of = create_date(2025, Month::January, 1)?;
    ///
    /// let result = bond.price_with_metrics(
    ///     &market,
    ///     as_of,
    ///     &[MetricId::Dv01],
    ///     PricingOptions::default().with_model(ModelKey::HazardRate),
    /// )?;
    /// # let _ = result;
    /// # Ok(())
    /// # }
    /// ```
    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
        options: PricingOptions,
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let PricingOptions {
            config,
            market_history,
            model,
            registry,
        } = options;
        let model = model.unwrap_or_else(|| self.default_model());
        let registry = registry.unwrap_or_else(shared_standard_registry);
        let instrument = self.clone_box();
        let registry_options = PricingOptions {
            config,
            market_history,
            model: None,
            registry: None,
        };

        registry
            .price_with_metrics(
                instrument.as_ref(),
                model,
                market,
                as_of,
                metrics,
                registry_options,
            )
            .map_err(|e| match e {
                PricingError::UnknownPricer(key) => actionable_unknown_pricer_message(key)
                    .map(finstack_core::Error::Validation)
                    .unwrap_or_else(|| PricingError::UnknownPricer(key).into()),
                other => other.into(),
            })
    }

    // === Market Data Introspection (for Attribution) ===

    /// Unified market data dependencies for this instrument.
    ///
    /// This is the canonical dependency surface and should be overridden by
    /// all instruments to declare their market data needs.
    fn market_dependencies(&self) -> finstack_core::Result<MarketDependencies> {
        Ok(MarketDependencies::new())
    }

    /// FX exposure for this instrument.
    ///
    /// Returns the currency pair if this instrument has FX exposure.
    /// Used by FX attribution to measure spot rate changes.
    ///
    /// Default implementation returns `None`.
    ///
    /// # Returns
    ///
    /// `Some((base, quote))` if FX-sensitive, `None` otherwise
    ///
    /// # Examples
    ///
    /// FX Forward would return `Some((USD, EUR))` for a USD/EUR forward.
    fn fx_exposure(&self) -> Option<(Currency, Currency)> {
        None
    }

    /// Dividend schedule ID for this instrument.
    ///
    /// Returns the dividend schedule ID if this instrument depends on dividends.
    /// Used by dividend attribution.
    ///
    /// Default implementation returns `None`.
    ///
    /// # Returns
    ///
    /// `Some(schedule_id)` if dividend-sensitive, `None` otherwise
    fn dividend_schedule_id(&self) -> Option<CurveId> {
        None
    }

    /// Funding curve ID for this instrument.
    ///
    /// Returns the funding or repo curve used to finance the position for
    /// carry cost calculations.
    ///
    /// Default implementation returns `None`.
    fn funding_curve_id(&self) -> Option<CurveId> {
        None
    }

    /// Convert this instrument to its JSON representation for serialization.
    ///
    /// This method enables serialization of instruments by converting them to
    /// the `InstrumentJson` tagged union. Instruments that support serialization
    /// should override this method to return `Some(instrument_json)`.
    ///
    /// Default implementation returns `None`, indicating that serialization
    /// is not supported for this instrument type.
    ///
    /// # Returns
    ///
    /// `Some(InstrumentJson)` if conversion is supported, `None` otherwise
    fn to_instrument_json(&self) -> Option<crate::instruments::InstrumentJson> {
        None
    }

    /// Get the instrument's expiry or maturity date, if applicable.
    ///
    /// Returns the date at which the instrument expires, matures, or otherwise
    /// terminates. This is used by theta calculations to cap the roll date
    /// and by other time-dependent calculations.
    ///
    /// # Returns
    ///
    /// - `Some(Date)` for instruments with a defined expiry/maturity
    /// - `None` for instruments without a clear expiry (e.g., equity spot positions)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::{Bond, Instrument};
    ///
    /// let bond = Bond::example().unwrap();
    /// if let Some(maturity) = bond.expiry() {
    ///     println!("Bond matures on: {}", maturity);
    /// }
    /// ```
    fn expiry(&self) -> Option<Date> {
        None
    }

    /// Get the instrument's effective start/value date, if applicable.
    ///
    /// Returns the date at which the instrument's economics begin (e.g., accrual start,
    /// effective date, or issue date). This is used by shared metrics such as DfStart
    /// and year-fraction calculations.
    ///
    /// # Returns
    ///
    /// - `Some(Date)` for instruments with a defined effective start/value date
    /// - `None` for instruments without a clear start (e.g., equity spot positions)
    fn effective_start_date(&self) -> Option<Date> {
        None
    }
}

// Note: Methods formerly on the `Attributable` trait are now default methods on `Instrument`.

// -----------------------------------------------------------------------------
// Curve Dependencies
// -----------------------------------------------------------------------------

/// Trait for instruments to declare all their curve dependencies.
///
/// This trait enables type-safe discovery of all curves used by an instrument,
/// eliminating the need for runtime downcasting. It's primarily used by risk
/// calculators (e.g., DV01) to identify which curves should be bumped.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::{CurveDependencies, InstrumentCurves};
/// use finstack_core::types::CurveId;
///
/// struct Bond {
///     discount_curve_id: CurveId,
/// }
///
/// impl CurveDependencies for Bond {
///     fn curve_dependencies(&self) -> finstack_core::Result<InstrumentCurves> {
///         InstrumentCurves::builder()
///             .discount(self.discount_curve_id.clone())
///             .build()
///     }
/// }
/// ```
pub trait CurveDependencies {
    /// Return all curves used by this instrument, categorized by type.
    fn curve_dependencies(&self) -> finstack_core::Result<InstrumentCurves>;
}

/// Collection of curves used by an instrument, categorized by type.
///
/// This struct provides a type-safe way to declare curve dependencies
/// for risk calculations. Uses `SmallVec` internally to avoid heap
/// allocation for the common case (1-2 curves per category).
#[derive(Default, Clone, Debug)]
pub struct InstrumentCurves {
    /// Discount curves used by the instrument (including primary and foreign).
    pub discount_curves: CurveIdVec,
    /// Forward/projection curves used by the instrument.
    pub forward_curves: CurveIdVec,
    /// Credit/hazard curves used by the instrument.
    pub credit_curves: CurveIdVec,
}

impl InstrumentCurves {
    /// Create a new empty curve collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start building a curve collection.
    pub fn builder() -> InstrumentCurvesBuilder {
        InstrumentCurvesBuilder::default()
    }

    /// Iterator over all curves with their kind.
    pub fn all_with_kind(&self) -> impl Iterator<Item = (CurveId, RatesCurveKind)> + '_ {
        self.discount_curves
            .iter()
            .map(|c| (c.clone(), RatesCurveKind::Discount))
            .chain(
                self.forward_curves
                    .iter()
                    .map(|c| (c.clone(), RatesCurveKind::Forward)),
            )
            .chain(
                self.credit_curves
                    .iter()
                    .map(|c| (c.clone(), RatesCurveKind::Credit)),
            )
    }

    /// Check if any curves are defined.
    pub fn is_empty(&self) -> bool {
        self.discount_curves.is_empty()
            && self.forward_curves.is_empty()
            && self.credit_curves.is_empty()
    }

    /// Total number of curves.
    pub fn len(&self) -> usize {
        self.discount_curves.len() + self.forward_curves.len() + self.credit_curves.len()
    }
}

/// Builder for [`InstrumentCurves`].
#[derive(Default)]
pub struct InstrumentCurvesBuilder {
    curves: InstrumentCurves,
}

impl InstrumentCurvesBuilder {
    /// Add a discount curve (duplicates are ignored).
    pub fn discount(mut self, curve_id: CurveId) -> Self {
        if !self.curves.discount_curves.contains(&curve_id) {
            self.curves.discount_curves.push(curve_id);
        }
        self
    }

    /// Add a forward curve (duplicates are ignored).
    pub fn forward(mut self, curve_id: CurveId) -> Self {
        if !self.curves.forward_curves.contains(&curve_id) {
            self.curves.forward_curves.push(curve_id);
        }
        self
    }

    /// Add a credit/hazard curve (duplicates are ignored).
    pub fn credit(mut self, curve_id: CurveId) -> Self {
        if !self.curves.credit_curves.contains(&curve_id) {
            self.curves.credit_curves.push(curve_id);
        }
        self
    }

    /// Build the final curve collection.
    pub fn build(self) -> finstack_core::Result<InstrumentCurves> {
        Ok(self.curves)
    }
}

/// Identifies the type of rate curve for risk calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RatesCurveKind {
    /// Discount curve (used for present value discounting).
    Discount,
    /// Forward curve (used for floating rate projection).
    Forward,
    /// Credit/hazard curve (used for credit risk calculations).
    Credit,
}

// ================================================================================================
// Equity Market Data Dependencies
// ================================================================================================

/// Trait for instruments that depend on equity market data.
///
/// Provides a unified interface for discovering equity-related market data dependencies,
/// similar to how [`CurveDependencies`] works for curves. This trait enables generic
/// metric calculators (e.g., finite difference greeks) to discover which equity market
/// data an instrument requires without runtime downcasting.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::{EquityDependencies, EquityInstrumentDeps};
///
/// struct EquityOption {
///     spot_id: String,
///     vol_surface_id: String,
/// }
///
/// impl EquityDependencies for EquityOption {
///     fn equity_dependencies(&self) -> finstack_core::Result<EquityInstrumentDeps> {
///         EquityInstrumentDeps::builder()
///             .spot(self.spot_id.clone())
///             .vol_surface(self.vol_surface_id.clone())
///             .build()
///     }
/// }
/// ```
pub trait EquityDependencies {
    /// Return equity market data dependencies for this instrument.
    fn equity_dependencies(&self) -> finstack_core::Result<EquityInstrumentDeps>;
}

/// Collection of equity market data used by an instrument.
///
/// This struct provides a type-safe way to declare equity market data dependencies
/// for risk calculations and generic metric implementations.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::EquityInstrumentDeps;
///
/// let deps = EquityInstrumentDeps::builder()
///     .spot("AAPL")
///     .vol_surface("AAPL-VOL")
///     .build()
///     .expect("infallible");
///
/// assert_eq!(deps.spot_id, Some("AAPL".to_string()));
/// assert_eq!(deps.vol_surface_id, Some("AAPL-VOL".to_string()));
/// ```
#[derive(Default, Clone, Debug)]
pub struct EquityInstrumentDeps {
    /// Spot price identifier (e.g., "AAPL", "SPX").
    ///
    /// This is used to look up the current equity price in the market context
    /// for pricing and sensitivity calculations.
    pub spot_id: Option<String>,

    /// Volatility surface identifier.
    ///
    /// This is used to look up implied volatilities for option pricing
    /// and volatility greeks (vega, volga, vanna).
    pub vol_surface_id: Option<String>,
}

impl EquityInstrumentDeps {
    /// Create a new empty equity dependencies collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start building an equity dependencies collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::EquityInstrumentDeps;
    ///
    /// let deps = EquityInstrumentDeps::builder()
    ///     .spot("SPX")
    ///     .vol_surface("SPX-VOL")
    ///     .build()
    ///     .expect("infallible");
    /// ```
    pub fn builder() -> EquityInstrumentDepsBuilder {
        EquityInstrumentDepsBuilder::default()
    }
}

/// Builder for [`EquityInstrumentDeps`].
///
/// Provides a fluent interface for constructing equity dependency declarations.
#[derive(Default)]
pub struct EquityInstrumentDepsBuilder {
    deps: EquityInstrumentDeps,
}

impl EquityInstrumentDepsBuilder {
    /// Add a spot price identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - Spot price identifier (e.g., "AAPL", "SPX")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::EquityInstrumentDeps;
    ///
    /// let deps = EquityInstrumentDeps::builder()
    ///     .spot("AAPL")
    ///     .build();
    /// ```
    pub fn spot(mut self, id: impl Into<String>) -> Self {
        self.deps.spot_id = Some(id.into());
        self
    }

    /// Add a volatility surface identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - Volatility surface identifier
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::EquityInstrumentDeps;
    ///
    /// let deps = EquityInstrumentDeps::builder()
    ///     .vol_surface("SPX-VOL")
    ///     .build();
    /// ```
    pub fn vol_surface(mut self, id: impl Into<String>) -> Self {
        self.deps.vol_surface_id = Some(id.into());
        self
    }

    /// Build the final equity dependencies collection.
    pub fn build(self) -> finstack_core::Result<EquityInstrumentDeps> {
        Ok(self.deps)
    }
}

// ================================================================================================
// Option risk metric providers
// ================================================================================================

/// Supported option greek requests for the consolidated provider API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionGreekKind {
    /// Cash delta in instrument metric convention.
    Delta,
    /// Cash gamma in instrument metric convention.
    Gamma,
    /// Cash vega per 1 vol point.
    Vega,
    /// Theta per instrument day-count convention.
    Theta,
    /// Domestic rho per 1bp.
    Rho,
    /// Foreign/dividend rho per 1bp.
    ForeignRho,
    /// Vanna in instrument bump convention.
    Vanna,
    /// Volga in instrument bump convention.
    Volga,
}

/// Inputs needed to request a specific option greek.
///
/// `base_pv` is required only for [`OptionGreekKind::Volga`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionGreeksRequest {
    /// The greek being requested.
    pub greek: OptionGreekKind,
    /// Base PV required by some greeks such as volga.
    pub base_pv: Option<f64>,
}

impl OptionGreeksRequest {
    /// Return the requested base PV or an error when it is required but missing.
    pub fn require_base_pv(self) -> finstack_core::Result<f64> {
        self.base_pv.ok_or_else(|| {
            finstack_core::Error::Validation(
                "OptionGreekKind::Volga requires base_pv in OptionGreeksRequest".to_string(),
            )
        })
    }
}

/// Sparse option greek payload returned by [`OptionGreeksProvider`].
///
/// Providers should populate the requested field when it is supported for the
/// instrument and leave unsupported greeks as `None`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OptionGreeks {
    /// Cash delta in instrument metric convention.
    pub delta: Option<f64>,
    /// Cash gamma in instrument metric convention.
    pub gamma: Option<f64>,
    /// Cash vega per 1 vol point.
    pub vega: Option<f64>,
    /// Theta per instrument day-count convention.
    pub theta: Option<f64>,
    /// Domestic rho per 1bp.
    pub rho_bp: Option<f64>,
    /// Foreign/dividend rho per 1bp.
    pub foreign_rho_bp: Option<f64>,
    /// Vanna in instrument bump convention.
    pub vanna: Option<f64>,
    /// Volga in instrument bump convention.
    pub volga: Option<f64>,
}

/// Consolidated option greek provider.
///
/// Implementations return a sparse [`OptionGreeks`] payload keyed by the
/// requested [`OptionGreekKind`]. Callers should interpret `None` as "not
/// supported for this instrument" rather than as a zero-valued greek.
pub trait OptionGreeksProvider {
    /// Return the requested greek in a sparse [`OptionGreeks`] payload.
    fn option_greeks(
        &self,
        market: &MarketContext,
        as_of: Date,
        request: &OptionGreeksRequest,
    ) -> finstack_core::Result<OptionGreeks>;
}

// Legacy single-greek helpers remain crate-private while instrument implementations
// converge on `OptionGreeksProvider`.

/// Provide **cash delta** in the metric convention for this instrument.
///
/// Conventions:
/// - Return value is \(\partial PV / \partial S\) where \(S\) is the instrument’s chosen
///   underlying “spot” driver (equity spot, FX spot, forward, etc.).
/// - The value should already include instrument scaling (notional / quantity / multiplier).
/// - At/after expiry, return 0.0 unless the instrument explicitly defines an intrinsic
///   delta convention.
pub(crate) trait OptionDeltaProvider {
    /// Return cash delta per instrument conventions.
    fn option_delta(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Provide **cash gamma** in the metric convention for this instrument.
///
/// Conventions:
/// - Return value is \(\partial^2 PV / \partial S^2\) using the instrument’s chosen
///   underlying “spot” driver \(S\).
/// - The value should already include instrument scaling (notional / quantity / multiplier).
pub(crate) trait OptionGammaProvider {
    /// Return cash gamma per instrument conventions.
    fn option_gamma(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Provide **cash vega** (per 1 vol point) for this instrument.
///
/// Conventions:
/// - Return value is \(\partial PV / \partial \sigma\) scaled to a **0.01 absolute**
///   volatility move (1 vol point).
/// - The value should already include instrument scaling (notional / quantity / multiplier).
pub(crate) trait OptionVegaProvider {
    /// Return cash vega per instrument conventions (1 vol point).
    fn option_vega(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Provide theta (per day) in the instrument’s convention.
///
/// Conventions:
/// - Return value is the PV change for **one day of time decay** (usually negative for long options).
/// - The day basis (calendar vs trading days) is instrument-specific and must match the
///   instrument’s existing pricing/greeks conventions.
pub(crate) trait OptionThetaProvider {
    /// Return theta per instrument conventions (per day).
    fn option_theta(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Provide rho (domestic) per **1bp** move for this instrument.
///
/// Conventions:
/// - Return value is \(PV(r+1bp) - PV(r)\) for the relevant “domestic” discount driver.
/// - This should be a **finite-difference PV change**, not “per 1%” scaling.
pub(crate) trait OptionRhoProvider {
    /// Return domestic rho per instrument conventions (per 1bp).
    fn option_rho_bp(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Provide foreign/dividend rho per **1bp** move for this instrument.
///
/// Conventions:
/// - Return value is \(PV(q+1bp) - PV(q)\) where \(q\) is the foreign rate/dividend yield
///   driver used by the instrument.
pub(crate) trait OptionForeignRhoProvider {
    /// Return foreign/dividend rho per instrument conventions (per 1bp).
    fn option_foreign_rho_bp(
        &self,
        market: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64>;
}

/// Compute vanna in the instrument’s chosen bump conventions.
///
/// Conventions:
/// - Vanna is a mixed derivative (commonly \(\partial^2 PV / \partial S \partial \sigma\)).
/// - Implementations may use spot-then-vol or vol-then-spot bump logic as long as it is
///   consistent with the instrument’s historical behavior and bump size settings.
pub(crate) trait OptionVannaProvider {
    /// Return vanna per instrument conventions.
    fn option_vanna(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Trait for instruments that can compute volga in their chosen bump conventions.
///
/// `base_pv` should be the already computed PV amount at `as_of` for the same market.
pub(crate) trait OptionVolgaProvider {
    /// Return volga per instrument conventions.
    fn option_volga(
        &self,
        market: &MarketContext,
        as_of: Date,
        base_pv: f64,
    ) -> finstack_core::Result<f64>;
}

/// Implement standard equity-exotic trait boilerplate for instruments with
/// `spot_id`, `vol_surface_id`, `pricing_overrides`, `day_count` fields.
///
/// # Variants
///
/// - With `curve_deps`: also implements `CurveDependencies` using `discount_curve_id`.
/// - For types with custom `HasExpiry`, use the internal `@equity`, `@mc_overrides`,
///   `@mc_daycount` arms directly and implement `HasExpiry` manually.
#[macro_export]
macro_rules! impl_equity_exotic_traits {
    ($ty:ty, curve_deps: true) => {
        impl $crate::instruments::common_impl::traits::CurveDependencies for $ty {
            fn curve_dependencies(
                &self,
            ) -> finstack_core::Result<$crate::instruments::common_impl::traits::InstrumentCurves>
            {
                $crate::instruments::common_impl::traits::InstrumentCurves::builder()
                    .discount(self.discount_curve_id.clone())
                    .build()
            }
        }

        $crate::impl_equity_exotic_traits!(@inner $ty);
    };

    ($ty:ty) => {
        $crate::impl_equity_exotic_traits!(@inner $ty);
    };

    (@inner $ty:ty) => {
        $crate::impl_equity_exotic_traits!(@equity $ty);
        $crate::impl_equity_exotic_traits!(@mc_overrides $ty);
        $crate::impl_equity_exotic_traits!(@mc_daycount $ty);

        #[cfg(feature = "mc")]
        impl $crate::metrics::HasExpiry for $ty {
            fn expiry(&self) -> finstack_core::dates::Date {
                self.expiry
            }
        }
    };

    (@equity $ty:ty) => {
        impl $crate::instruments::common_impl::traits::EquityDependencies for $ty {
            fn equity_dependencies(
                &self,
            ) -> finstack_core::Result<
                $crate::instruments::common_impl::traits::EquityInstrumentDeps,
            > {
                $crate::instruments::common_impl::traits::EquityInstrumentDeps::builder()
                    .spot(self.spot_id.as_str())
                    .vol_surface(self.vol_surface_id.as_str())
                    .build()
            }
        }
    };

    (@mc_overrides $ty:ty) => {
        #[cfg(feature = "mc")]
        impl $crate::metrics::HasPricingOverrides for $ty {
            fn pricing_overrides_mut(
                &mut self,
            ) -> &mut $crate::instruments::PricingOverrides {
                &mut self.pricing_overrides
            }
        }
    };

    (@mc_daycount $ty:ty) => {
        #[cfg(feature = "mc")]
        impl $crate::metrics::HasDayCount for $ty {
            fn day_count(&self) -> finstack_core::dates::DayCount {
                self.day_count
            }
        }
    };
}
