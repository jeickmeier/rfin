//! Build context for quote-to-instrument construction.

use finstack_core::dates::Date;
use std::collections::HashMap;

/// Context for building instruments from market quotes.
///
/// Provides the necessary environment (valuation date, notional, curve mappings) to construct
/// a concrete instrument instance that matches the quote. The context is used by all builders
/// to resolve dates, configure instruments, and map curve identifiers.
///
/// # Invariants
///
/// - `as_of` is the valuation date for all instruments built with this context
/// - `notional` is used consistently across all instruments unless overridden
/// - `curve_ids` maps role names (e.g., "discount", "forward", "credit") to curve identifiers
///
/// # Examples
///
/// Basic usage:
/// ```rust
/// use finstack_valuations::market::build::context::BuildCtx;
/// use finstack_core::dates::Date;
/// use std::collections::HashMap;
///
/// let as_of = Date::from_calendar_date(2024, time::Month::January, 2).unwrap();
/// let mut curve_ids = HashMap::new();
/// curve_ids.insert("discount".to_string(), "USD-OIS".to_string());
/// curve_ids.insert("forward".to_string(), "USD-SOFR".to_string());
///
/// let ctx = BuildCtx::new(as_of, 1_000_000.0, curve_ids);
/// assert_eq!(ctx.curve_id("discount"), Some(&"USD-OIS".to_string()));
/// ```
///
/// With default curve fallback:
/// ```rust
/// use finstack_valuations::market::build::context::BuildCtx;
/// use finstack_core::dates::Date;
/// use std::collections::HashMap;
///
/// let ctx = BuildCtx::new(
///     Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
///     1_000_000.0,
///     HashMap::new(), // Empty - builders will use currency-based defaults
/// );
/// ```
#[derive(Clone, Debug)]
pub struct BuildCtx {
    /// The valuation date for which the instrument is being built.
    ///
    /// This date is used to calculate spot dates, fixing dates, and all other
    /// date-dependent parameters in the constructed instrument.
    pub as_of: Date,
    /// The notional amount to use for the instrument.
    ///
    /// Calibration typically uses a standard notional (e.g., 1M or 10k) but this allows override.
    /// The notional is applied consistently across all instruments built with this context.
    pub notional: f64,
    /// Mapping of curve roles to curve IDs.
    ///
    /// Common roles include:
    /// - `"discount"`: Discount curve for present value calculations
    /// - `"forward"`: Forward curve for floating rate projections
    /// - `"credit"`: Credit curve for CDS instruments
    ///
    /// If a role is not found, builders will fall back to currency-based defaults
    /// (e.g., using the currency string as the curve ID).
    pub curve_ids: HashMap<String, String>,
    /// Optional attributes or tags for instrument metadata.
    ///
    /// These attributes can be used to attach custom metadata to instruments
    /// during construction.
    pub attributes: HashMap<String, String>,
}

impl BuildCtx {
    /// Create a new build context.
    ///
    /// # Arguments
    ///
    /// * `as_of` - Valuation date for all instruments built with this context
    /// * `notional` - Standard notional amount to use for instruments
    /// * `curve_ids` - Mapping of curve roles to curve identifiers
    ///
    /// # Returns
    ///
    /// A new `BuildCtx` with empty attributes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::build::context::BuildCtx;
    /// use finstack_core::dates::Date;
    /// use std::collections::HashMap;
    ///
    /// let ctx = BuildCtx::new(
    ///     Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
    ///     1_000_000.0,
    ///     HashMap::new(),
    /// );
    /// ```
    pub fn new(as_of: Date, notional: f64, curve_ids: HashMap<String, String>) -> Self {
        Self {
            as_of,
            notional,
            curve_ids,
            attributes: HashMap::new(),
        }
    }

    /// Get a curve ID by role name.
    ///
    /// # Arguments
    ///
    /// * `role` - The curve role name (e.g., "discount", "forward", "credit")
    ///
    /// # Returns
    ///
    /// `Some(curve_id)` if the role is mapped, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::build::context::BuildCtx;
    /// use finstack_core::dates::Date;
    /// use std::collections::HashMap;
    ///
    /// let mut curve_ids = HashMap::new();
    /// curve_ids.insert("discount".to_string(), "USD-OIS".to_string());
    ///
    /// let ctx = BuildCtx::new(
    ///     Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
    ///     1_000_000.0,
    ///     curve_ids,
    /// );
    ///
    /// assert_eq!(ctx.curve_id("discount"), Some(&"USD-OIS".to_string()));
    /// assert_eq!(ctx.curve_id("forward"), None);
    /// ```
    pub fn curve_id(&self, role: &str) -> Option<&String> {
        self.curve_ids.get(role)
    }
}
