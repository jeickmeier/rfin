//! Traits and result types for margin calculators.

use crate::instruments::common::traits::Instrument;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use super::super::types::ImMethodology;

/// Initial margin calculation result.
///
/// Contains the calculated IM amount along with methodology details
/// and breakdown information.
#[derive(Debug, Clone, PartialEq)]
pub struct ImResult {
    /// Calculated initial margin amount
    pub amount: Money,

    /// Methodology used for calculation
    pub methodology: ImMethodology,

    /// Calculation date
    pub as_of: Date,

    /// Margin Period of Risk (days) used in calculation
    pub mpor_days: u32,

    /// Breakdown by risk class (if available)
    ///
    /// Keys are risk class names (e.g., "interest_rate", "credit", "equity")
    /// Values are IM amounts for that risk class
    pub breakdown: std::collections::HashMap<String, Money>,

    /// Any add-ons applied (e.g., jump-to-default for credit)
    pub addons: Vec<ImAddon>,
}

impl ImResult {
    /// Create a simple IM result with no breakdown.
    #[must_use]
    pub fn simple(amount: Money, methodology: ImMethodology, as_of: Date, mpor_days: u32) -> Self {
        Self {
            amount,
            methodology,
            as_of,
            mpor_days,
            breakdown: std::collections::HashMap::new(),
            addons: vec![],
        }
    }

    /// Create an IM result with breakdown by risk class.
    #[must_use]
    pub fn with_breakdown(
        amount: Money,
        methodology: ImMethodology,
        as_of: Date,
        mpor_days: u32,
        breakdown: std::collections::HashMap<String, Money>,
    ) -> Self {
        Self {
            amount,
            methodology,
            as_of,
            mpor_days,
            breakdown,
            addons: vec![],
        }
    }
}

/// IM add-on component.
///
/// Represents additional margin requirements beyond the base calculation,
/// such as jump-to-default risk for credit derivatives.
#[derive(Debug, Clone, PartialEq)]
pub struct ImAddon {
    /// Description of the add-on
    pub description: String,

    /// Add-on amount
    pub amount: Money,
}

/// Trait for initial margin calculators.
///
/// Implement this trait to provide custom IM calculation logic
/// for different methodologies or instrument types.
///
/// # Example Implementation
///
/// ```rust,no_run
/// use finstack_valuations::instruments::Instrument;
/// use finstack_valuations::margin::{ImCalculator, ImMethodology, ImResult};
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
/// use finstack_core::money::Money;
///
/// struct CustomImCalculator {
///     fixed_rate: f64,
/// }
///
/// impl ImCalculator for CustomImCalculator {
///     fn calculate(
///         &self,
///         instrument: &dyn Instrument,
///         context: &MarketContext,
///         as_of: Date,
///     ) -> finstack_core::Result<ImResult> {
///         let pv = instrument.value(context, as_of)?;
///         let im = Money::new(pv.amount().abs() * self.fixed_rate, pv.currency());
///         Ok(ImResult::simple(
///             im,
///             ImMethodology::InternalModel,
///             as_of,
///             10,
///         ))
///     }
///
///     fn methodology(&self) -> ImMethodology {
///         ImMethodology::InternalModel
///     }
/// }
/// ```
pub trait ImCalculator: Send + Sync {
    /// Calculate initial margin for an instrument.
    ///
    /// # Arguments
    ///
    /// * `instrument` - The financial instrument requiring IM
    /// * `context` - Market data context with curves and surfaces
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// [`ImResult`] containing the calculated IM amount and methodology details.
    fn calculate(
        &self,
        instrument: &dyn Instrument,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult>;

    /// Get the methodology this calculator implements.
    fn methodology(&self) -> ImMethodology;
}
