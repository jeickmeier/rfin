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
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let issue = create_date(2025, Month::January, 15)?;
//! let maturity = create_date(2030, Month::January, 15)?;
//! let bond = Bond::fixed(
//!     "BOND-001",
//!     Money::new(1_000_000.0, Currency::USD),
//!     0.05,
//!     issue,
//!     maturity,
//!     "USD-OIS"
//! );
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
//! use finstack_valuations::instruments::common::traits::Attributes;
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
use crate::metrics::MetricId;
use crate::pricer::InstrumentType;
use finstack_core::market_data::context::MarketContext;
use finstack_core::{currency::Currency, dates::Date, money::Money, types::CurveId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};

/// Type alias for curve ID collections that are typically small (0-2 items).
///
/// Most instruments depend on 1-2 curves. Using SmallVec avoids heap allocation
/// for the common case while still supporting instruments with more curve dependencies.
pub type CurveIdVec = SmallVec<[CurveId; 2]>;

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
/// use finstack_valuations::instruments::common::traits::Attributes;
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
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Attributes {
    /// User-defined tags for categorization (e.g., "high-yield", "energy").
    ///
    /// Stored as an ordered set to ensure deterministic serialization and stable iteration.
    pub tags: BTreeSet<String>,
    /// Key-value metadata pairs for structured attributes.
    ///
    /// Stored as an ordered map to ensure deterministic serialization and stable iteration.
    pub meta: BTreeMap<String, String>,
}

impl Attributes {
    /// Create empty attributes with no tags or metadata.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::common::traits::Attributes;
    ///
    /// let attrs = Attributes::new();
    /// assert!(attrs.tags.is_empty());
    /// assert!(attrs.meta.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single tag for categorization.
    ///
    /// Tags are case-sensitive strings used for broad instrument classification.
    /// Returns `self` for method chaining.
    ///
    /// # Arguments
    ///
    /// * `tag` - Tag string to add (e.g., "high-yield", "energy")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::common::traits::Attributes;
    ///
    /// let attrs = Attributes::new()
    ///     .with_tag("corporate")
    ///     .with_tag("investment-grade");
    ///
    /// assert!(attrs.has_tag("corporate"));
    /// assert!(attrs.has_tag("investment-grade"));
    /// ```
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.insert(tag.to_string());
        self
    }

    /// Add multiple tags at once.
    ///
    /// Convenience method for adding several tags in a single call.
    /// Returns `self` for method chaining.
    ///
    /// # Arguments
    ///
    /// * `tags` - Slice of tag strings to add
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::common::traits::Attributes;
    ///
    /// let attrs = Attributes::new()
    ///     .with_tags(&["equity", "technology", "growth"]);
    ///
    /// assert_eq!(attrs.tags.len(), 3);
    /// assert!(attrs.has_tag("technology"));
    /// ```
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        for tag in tags {
            self.tags.insert(tag.to_string());
        }
        self
    }

    /// Add a metadata key-value pair.
    ///
    /// Metadata stores structured attributes beyond simple tags.
    /// Overwrites existing values for the same key. Returns `self` for chaining.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key (e.g., "sector", "rating")
    /// * `value` - Metadata value (e.g., "technology", "AA+")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::common::traits::Attributes;
    ///
    /// let attrs = Attributes::new()
    ///     .with_meta("sector", "financials")
    ///     .with_meta("region", "north-america");
    ///
    /// assert_eq!(attrs.get_meta("sector"), Some("financials"));
    /// assert_eq!(attrs.get_meta("region"), Some("north-america"));
    /// ```
    pub fn with_meta(mut self, key: &str, value: &str) -> Self {
        self.meta.insert(key.to_string(), value.to_string());
        self
    }

    /// Check if a specific tag exists.
    ///
    /// Tag matching is case-sensitive and exact.
    ///
    /// # Arguments
    ///
    /// * `tag` - Tag to check for
    ///
    /// # Returns
    ///
    /// `true` if the tag exists, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::common::traits::Attributes;
    ///
    /// let attrs = Attributes::new().with_tag("high-yield");
    ///
    /// assert!(attrs.has_tag("high-yield"));
    /// assert!(!attrs.has_tag("investment-grade"));
    /// ```
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Get a metadata value by key.
    ///
    /// Returns the value associated with the key, or `None` if not found.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key to look up
    ///
    /// # Returns
    ///
    /// `Some(value)` if key exists, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::common::traits::Attributes;
    ///
    /// let attrs = Attributes::new().with_meta("issuer", "AAPL");
    ///
    /// assert_eq!(attrs.get_meta("issuer"), Some("AAPL"));
    /// assert_eq!(attrs.get_meta("unknown"), None);
    /// ```
    pub fn get_meta(&self, key: &str) -> Option<&str> {
        self.meta.get(key).map(|s| s.as_str())
    }

    /// Check if attributes match a selector pattern.
    ///
    /// Selector patterns support:
    /// - `"*"`: Matches all instruments (wildcard)
    /// - `"tag:name"`: Matches instruments with the specified tag
    /// - `"meta:key=value"`: Matches instruments with the specified metadata
    ///
    /// # Arguments
    ///
    /// * `selector` - Selector pattern string
    ///
    /// # Returns
    ///
    /// `true` if the selector matches, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::common::traits::Attributes;
    ///
    /// let attrs = Attributes::new()
    ///     .with_tag("corporate")
    ///     .with_meta("rating", "AA+");
    ///
    /// assert!(attrs.matches_selector("*"));
    /// assert!(attrs.matches_selector("tag:corporate"));
    /// assert!(attrs.matches_selector("meta:rating=AA+"));
    /// assert!(!attrs.matches_selector("tag:government"));
    /// assert!(!attrs.matches_selector("meta:rating=BBB"));
    /// ```
    pub fn matches_selector(&self, selector: &str) -> bool {
        if selector == "*" {
            return true;
        }
        if let Some(tag) = selector.strip_prefix("tag:") {
            return self.has_tag(tag);
        }
        if let Some(meta_spec) = selector.strip_prefix("meta:") {
            if let Some((key, value)) = meta_spec.split_once('=') {
                return self.get_meta(key) == Some(value);
            }
        }
        false
    }
}

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
/// use finstack_core::market_data::context::MarketContext;
/// use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let issue = create_date(2025, Month::January, 15)?;
/// let maturity = create_date(2030, Month::January, 15)?;
/// let bond = Bond::fixed(
///     "BOND-001",
///     Money::new(1_000_000.0, Currency::USD),
///     0.05,
///     issue,
///     maturity,
///     "USD-OIS"
/// );
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
/// # use finstack_core::market_data::context::MarketContext;
/// # use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let issue = create_date(2025, Month::January, 15)?;
/// # let maturity = create_date(2030, Month::January, 15)?;
/// # let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
/// #     0.05, issue, maturity, "USD-OIS");
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
/// # use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let issue = create_date(2025, Month::January, 15)?;
/// # let maturity = create_date(2030, Month::January, 15)?;
/// let bond = Bond::fixed(
///     "BOND-001",
///     Money::new(1_000_000.0, Currency::USD),
///     0.05,
///     issue,
///     maturity,
///     "USD-OIS"
/// );
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
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed(
    ///     "US-TREASURY-5Y-001",
    ///     Money::new(1_000_000.0, Currency::USD),
    ///     0.05,
    ///     issue,
    ///     maturity,
    ///     "USD-OIS"
    /// );
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
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     0.05, issue, maturity, "USD-OIS");
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
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     0.05, issue, maturity, "USD-OIS");
    ///
    /// let instrument: &dyn Instrument = &bond;
    /// let concrete_bond: Option<&Bond> = instrument.as_any().downcast_ref::<Bond>();
    /// assert!(concrete_bond.is_some());
    /// # Ok(())
    /// # }
    /// ```
    fn as_any(&self) -> &dyn Any;

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
    /// use finstack_valuations::cashflow::traits::CashflowProvider;
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed(
    ///     "BOND-001",
    ///     Money::new(1_000_000.0, Currency::USD),
    ///     0.05,
    ///     issue,
    ///     maturity,
    ///     "USD-OIS"
    /// );
    ///
    /// let inst: &dyn Instrument = &bond;
    /// if let Some(cf) = inst.as_cashflow_provider() {
    ///     let curves = MarketContext::new();
    ///     let _schedule = cf.build_schedule(&curves, issue)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn as_cashflow_provider(&self) -> Option<&dyn CashflowProvider> {
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

    /// Get mutable reference to pricing overrides for scenario shocks.
    ///
    /// Returns a mutable reference to the instrument's [`PricingOverrides`],
    /// allowing scenarios to apply price and spread shocks that affect
    /// actual pricing calculations.
    ///
    /// # Returns
    ///
    /// `Some(&mut PricingOverrides)` if the instrument supports pricing overrides,
    /// `None` otherwise.
    ///
    /// # Default Implementation
    ///
    /// Returns `None`. Instrument types that support scenario shocks should
    /// override this method to return their internal `PricingOverrides`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::Instrument;
    /// use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
    ///
    /// fn apply_price_shock(instrument: &mut dyn Instrument, shock_pct: f64) {
    ///     if let Some(overrides) = instrument.scenario_overrides_mut() {
    ///         overrides.scenario_price_shock_pct = Some(shock_pct);
    ///     }
    /// }
    /// ```
    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        None
    }

    /// Get immutable reference to pricing overrides for scenario shocks.
    ///
    /// # Returns
    ///
    /// `Some(&PricingOverrides)` if the instrument supports pricing overrides,
    /// `None` otherwise.
    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        None
    }

    /// Check if the instrument matches a selector pattern.
    ///
    /// Convenience method that delegates to `Attributes::matches_selector()`.
    /// See [`Attributes::matches_selector()`] for supported selector patterns.
    ///
    /// # Arguments
    ///
    /// * `selector` - Selector pattern (e.g., "tag:corporate", "meta:sector=tech")
    ///
    /// # Returns
    ///
    /// `true` if selector matches, `false` otherwise
    fn matches_selector(&self, selector: &str) -> bool {
        self.attributes().matches_selector(selector)
    }

    /// Check if the instrument has a specific tag.
    ///
    /// Convenience method that delegates to `Attributes::has_tag()`.
    ///
    /// # Arguments
    ///
    /// * `tag` - Tag to check for
    ///
    /// # Returns
    ///
    /// `true` if tag exists, `false` otherwise
    fn has_tag(&self, tag: &str) -> bool {
        self.attributes().has_tag(tag)
    }

    /// Get a metadata value by key.
    ///
    /// Convenience method that delegates to `Attributes::get_meta()`.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key to look up
    ///
    /// # Returns
    ///
    /// `Some(value)` if key exists, `None` otherwise
    fn get_meta(&self, key: &str) -> Option<&str> {
        self.attributes().get_meta(key)
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
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     0.05, issue, maturity, "USD-OIS");
    ///
    /// let instrument: Box<dyn Instrument> = Box::new(bond);
    /// let cloned = instrument.clone_box();
    /// assert_eq!(cloned.id(), instrument.id());
    /// # Ok(())
    /// # }
    /// ```
    fn clone_box(&self) -> Box<dyn Instrument>;

    // === Pricing Methods ===

    /// Compute the present value only (fast path, no metrics).
    ///
    /// This is the performance-optimized method for obtaining just the NPV
    /// without computing any risk metrics. Use this in hot paths like
    /// portfolio aggregation where metrics are not needed.
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
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     0.05, issue, maturity, "USD-OIS");
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
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Used internally by risk calculators for high-precision sensitivities
    /// use finstack_core::currency::Currency;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::money::Money;
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// use time::macros::date;
    ///
    /// # fn main() -> finstack_core::Result<()> {
    /// let instrument = Bond::fixed(
    ///     "BOND-001",
    ///     Money::new(1_000_000.0, Currency::USD),
    ///     0.05,
    ///     date!(2025-01-15),
    ///     date!(2030-01-15),
    ///     "USD-OIS",
    /// );
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

    /// Compute present value with specified risk metrics.
    ///
    /// This method computes NPV plus any requested risk metrics (duration, DV01,
    /// Greeks, etc.). Metrics are computed on-demand based on the provided list,
    /// enabling efficient calculation of only the required sensitivities.
    ///
    /// # Arguments
    ///
    /// * `market` - Market data context containing all pricing inputs
    /// * `as_of` - Valuation date (T+0)
    /// * `metrics` - List of metric IDs to compute (e.g., `MetricId::Dv01`)
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
    /// ```rust
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// use finstack_valuations::metrics::MetricId;
    /// use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     0.05, issue, maturity, "USD-OIS");
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
    /// // let result = bond.price_with_metrics(&market, as_of, &metrics_to_compute)?;
    /// // println!("NPV: {}", result.value);
    /// // println!("DV01: {}", result.measures.get("dv01").expect("should succeed"));
    /// # Ok(())
    /// # }
    /// ```
    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult>;

    /// Compute present value with specified risk metrics, using an explicit `FinstackConfig`.
    ///
    /// This method is identical to [`Instrument::price_with_metrics`] but allows callers to
    /// supply a `FinstackConfig` so user-facing defaults (e.g., risk bump sizes via
    /// `FinstackConfig.extensions["valuations.sensitivities.v1"]`) can be set once and
    /// persisted as part of a reproducible pipeline.
    ///
    /// # Notes
    ///
    /// - Sensitivities (DV01/CS01/vega/FD greeks) resolve their bump sizes from the supplied
    ///   `FinstackConfig` (via `valuations.sensitivities.v1`) for reproducibility.
    /// - If you don't need config-aware defaults, use [`Instrument::price_with_metrics`].
    fn price_with_metrics_with_config(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
        cfg: &finstack_core::config::FinstackConfig,
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;

        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::from(self.clone_box()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            Some(std::sync::Arc::new(cfg.clone())),
        )
    }

    // === Market Data Introspection (for Attribution) ===

    /// Discount curves required for pricing this instrument.
    ///
    /// Returns the list of discount curve IDs that this instrument depends on.
    /// Used by P&L attribution to identify which curve shifts affect the instrument.
    ///
    /// Default implementation returns empty vector. Instruments should override
    /// to return their actual discount curve dependencies.
    ///
    /// # Returns
    ///
    /// Vector of discount curve IDs (e.g., `["USD-OIS", "USD-SOFR"]`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::{Bond, Instrument};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::money::Money;
    /// # use finstack_core::dates::create_date;
    /// # use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let issue = create_date(2025, Month::January, 15)?;
    /// # let maturity = create_date(2030, Month::January, 15)?;
    /// let bond = Bond::fixed("BOND-001", Money::new(1_000_000.0, Currency::USD),
    ///     0.05, issue, maturity, "USD-OIS");
    ///
    /// let curves = bond.required_discount_curves();
    /// // Bond returns ["USD-OIS"]
    /// # Ok(())
    /// # }
    /// ```
    fn required_discount_curves(&self) -> CurveIdVec {
        SmallVec::new()
    }

    /// Hazard curves required for pricing this instrument.
    ///
    /// Returns the list of hazard curve IDs that this instrument depends on.
    /// Used by credit attribution to measure spread shifts.
    ///
    /// Default implementation returns empty collection.
    ///
    /// # Returns
    ///
    /// Collection of hazard curve IDs (e.g., `["CORP-AA", "CDX.NA.IG"]`)
    fn required_hazard_curves(&self) -> CurveIdVec {
        SmallVec::new()
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

    /// Spot price identifier for this instrument.
    ///
    /// Returns the market data key used to fetch the underlying spot or price level
    /// when computing delta/vega-style sensitivities.
    ///
    /// Default implementation returns `None`.
    fn spot_id(&self) -> Option<&str> {
        None
    }

    /// Volatility surface ID for this instrument.
    ///
    /// Returns the volatility surface ID if this instrument depends on implied vol.
    /// Used by vega attribution to measure vol shifts.
    ///
    /// Default implementation returns `None`.
    ///
    /// # Returns
    ///
    /// `Some(surface_id)` if vol-sensitive, `None` otherwise
    ///
    /// # Examples
    ///
    /// Equity option would return `Some("SPX-VOL")`.
    fn vol_surface_id(&self) -> Option<CurveId> {
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
/// use finstack_valuations::instruments::common::traits::{CurveDependencies, InstrumentCurves};
/// use finstack_core::types::CurveId;
///
/// struct Bond {
///     discount_curve_id: CurveId,
/// }
///
/// impl CurveDependencies for Bond {
///     fn curve_dependencies(&self) -> InstrumentCurves {
///         InstrumentCurves::builder()
///             .discount(self.discount_curve_id.clone())
///             .build()
///     }
/// }
/// ```
pub trait CurveDependencies {
    /// Return all curves used by this instrument, categorized by type.
    fn curve_dependencies(&self) -> InstrumentCurves;
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
    /// Add a discount curve.
    pub fn discount(mut self, curve_id: CurveId) -> Self {
        self.curves.discount_curves.push(curve_id);
        self
    }

    /// Add a forward curve.
    pub fn forward(mut self, curve_id: CurveId) -> Self {
        self.curves.forward_curves.push(curve_id);
        self
    }

    /// Add a credit/hazard curve.
    pub fn credit(mut self, curve_id: CurveId) -> Self {
        self.curves.credit_curves.push(curve_id);
        self
    }

    /// Build the final curve collection.
    pub fn build(self) -> InstrumentCurves {
        self.curves
    }
}

/// Identifies the type of rate curve for risk calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
/// use finstack_valuations::instruments::common::traits::{EquityDependencies, EquityInstrumentDeps};
///
/// struct EquityOption {
///     spot_id: String,
///     vol_surface_id: String,
/// }
///
/// impl EquityDependencies for EquityOption {
///     fn equity_dependencies(&self) -> EquityInstrumentDeps {
///         EquityInstrumentDeps::builder()
///             .spot(self.spot_id.clone())
///             .vol_surface(self.vol_surface_id.clone())
///             .build()
///     }
/// }
/// ```
pub trait EquityDependencies {
    /// Return equity market data dependencies for this instrument.
    fn equity_dependencies(&self) -> EquityInstrumentDeps;
}

/// Collection of equity market data used by an instrument.
///
/// This struct provides a type-safe way to declare equity market data dependencies
/// for risk calculations and generic metric implementations.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::common::traits::EquityInstrumentDeps;
///
/// let deps = EquityInstrumentDeps::builder()
///     .spot("AAPL")
///     .vol_surface("AAPL-VOL")
///     .build();
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
    /// use finstack_valuations::instruments::common::traits::EquityInstrumentDeps;
    ///
    /// let deps = EquityInstrumentDeps::builder()
    ///     .spot("SPX")
    ///     .vol_surface("SPX-VOL")
    ///     .build();
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
    /// use finstack_valuations::instruments::common::traits::EquityInstrumentDeps;
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
    /// use finstack_valuations::instruments::common::traits::EquityInstrumentDeps;
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
    pub fn build(self) -> EquityInstrumentDeps {
        self.deps
    }
}
