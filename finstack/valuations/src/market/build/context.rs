use finstack_core::dates::Date;
use std::collections::HashMap;

/// Context for building instruments from quotes.
///
/// This provides the necessary environment (valuation date, notional, curve mappings)
/// to construct a concrete instrument instance that matches the quote.
#[derive(Clone, Debug)]
pub struct BuildCtx {
    /// The valuation date for which the instrument is being built.
    pub as_of: Date,
    /// The notional amount to use for the instrument.
    /// Calibration typically uses a standard notional (e.g. 1M or 10k) but this allows override.
    pub notional: f64,
    /// Mapping of curve roles (e.g. "discount", "forward", "hazard") to curve IDs.
    pub curve_ids: HashMap<String, String>,
    /// Optional attributes or tags.
    pub attributes: HashMap<String, String>,
}

impl BuildCtx {
    /// Create a new build context.
    pub fn new(as_of: Date, notional: f64, curve_ids: HashMap<String, String>) -> Self {
        Self {
            as_of,
            notional,
            curve_ids,
            attributes: HashMap::new(),
        }
    }

    /// Helper to get a curve ID by role.
    pub fn curve_id(&self, role: &str) -> Option<&String> {
        self.curve_ids.get(role)
    }
}
