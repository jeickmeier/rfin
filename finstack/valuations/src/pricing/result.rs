//! Result type for pricing.
//! 
use finstack_core::prelude::*;
use finstack_core::F;
use hashbrown::HashMap;

/// Complete valuation result with NPV and computed metrics.
/// 
/// Contains the instrument's present value along with all requested
/// risk measures and metadata about the calculation.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValuationResult {
    /// Unique identifier for the instrument.
    pub instrument_id: String,
    /// Valuation date.
    pub as_of: Date,
    /// Present value of the instrument.
    pub value: Money,
    /// Computed risk measures and metrics.
    pub measures: HashMap<String, F>,
    /// Metadata about the calculation (timing, precision, etc.).
    pub meta: ResultsMeta,
}

impl ValuationResult {
    /// Create a basic valuation result with just NPV.
    /// 
    /// # Example
    /// ```rust
    /// use finstack_valuations::pricing::result::ValuationResult;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    /// 
    /// let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    /// let value = Money::new(1_000_000.0, Currency::USD);
    /// let result = ValuationResult::stamped("BOND001", as_of, value);
    /// assert_eq!(result.instrument_id, "BOND001");
    /// assert_eq!(result.value, value);
    /// ```
    pub fn stamped<S: Into<String>>(instrument_id: S, as_of: Date, value: Money) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            as_of,
            value,
            measures: HashMap::new(),
            meta: finstack_core::config::results_meta(),
        }
    }
}


