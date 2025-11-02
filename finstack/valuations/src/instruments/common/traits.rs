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

use crate::metrics::MetricId;
use crate::pricer::InstrumentType;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::any::Any;

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
pub struct Attributes {
    /// User-defined tags for categorization (e.g., "high-yield", "energy").
    pub tags: HashSet<String>,
    /// Key-value metadata pairs for structured attributes.
    pub meta: HashMap<String, String>,
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
/// use finstack_core::market_data::MarketContext;
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
/// # use finstack_core::market_data::MarketContext;
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
    /// use finstack_core::market_data::MarketContext;
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
    /// use finstack_core::market_data::MarketContext;
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
    /// // println!("DV01: {}", result.measures.get("dv01").unwrap());
    /// # Ok(())
    /// # }
    /// ```
    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult>;
}

// Note: Methods formerly on the `Attributable` trait are now default methods on `Instrument`.
