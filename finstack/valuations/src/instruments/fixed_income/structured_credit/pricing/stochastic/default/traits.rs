//! Stochastic default trait definition.
//!
//! The [`StochasticDefault`] trait provides a common interface for all
//! default models that incorporate systematic risk factors and correlation.

/// Macroeconomic credit factors affecting default rates.
///
/// These are economy-wide factors that influence default behavior,
/// distinct from individual loan-level `CreditFactors` in the types module.
#[derive(Clone, Debug, Default)]
pub struct MacroCreditFactors {
    /// Unemployment rate (e.g., 0.05 for 5%)
    pub unemployment: f64,
    /// GDP growth rate (e.g., 0.02 for 2%)
    pub gdp_growth: f64,
    /// House price appreciation (e.g., 0.03 for 3%)
    pub hpa: f64,
    /// Credit spread level (e.g., 0.01 for 100bp)
    pub credit_spread: f64,
}

/// Stochastic default model interface.
///
/// Implementations provide conditional default rates given:
/// - Loan seasoning (months since origination)
/// - Systematic factor realizations
/// - Macroeconomic credit factors
///
/// # Mathematical Framework
///
/// General form:
/// ```text
/// MDR(t, Z) = f(base_mdr, Z, credit_factors)
/// ```
///
/// where:
/// - Z is the systematic factor realization(s)
/// - credit_factors include macroeconomic conditions
pub trait StochasticDefault: Send + Sync + std::fmt::Debug {
    /// Conditional MDR (monthly default rate) given factor realizations.
    ///
    /// Returns the monthly default rate conditional on:
    /// - `seasoning`: Months since origination
    /// - `factors`: Systematic factor values [credit_factor, ...]
    /// - `macro_factors`: Macroeconomic conditions
    fn conditional_mdr(
        &self,
        seasoning: u32,
        factors: &[f64],
        macro_factors: &MacroCreditFactors,
    ) -> f64;

    /// Generate the default distribution for a portfolio.
    ///
    /// Given n loans with individual PDs and a factor realization,
    /// compute the distribution of number of defaults.
    ///
    /// # Arguments
    /// * `n` - Number of loans
    /// * `pds` - Individual default probabilities (length n or 1 for homogeneous)
    /// * `factors` - Systematic factor realizations
    /// * `correlation` - Asset correlation
    ///
    /// # Returns
    /// Vector of probabilities P(k defaults) for k = 0, 1, ..., n
    fn default_distribution(
        &self,
        n: usize,
        pds: &[f64],
        factors: &[f64],
        correlation: f64,
    ) -> Vec<f64>;

    /// Asset correlation parameter.
    fn correlation(&self) -> f64;

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str;

    /// Number of factors used by the model.
    fn num_factors(&self) -> usize {
        1
    }

    /// Expected (unconditional) MDR at given seasoning.
    fn expected_mdr(&self, seasoning: u32) -> f64;
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_credit_factors_default() {
        let factors = MacroCreditFactors::default();
        assert_eq!(factors.unemployment, 0.0);
        assert_eq!(factors.gdp_growth, 0.0);
    }
}
