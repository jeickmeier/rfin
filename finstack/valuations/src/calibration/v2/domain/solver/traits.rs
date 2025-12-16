use finstack_core::prelude::*;

/// Result type for building time grid and initial guesses.
pub type TimeGridAndGuesses<Q> = (Vec<f64>, Vec<f64>, Vec<Q>);

/// Trait defining the specific physics for a bootstrapping process.
pub trait BootstrapTarget {
    /// Type of input quote (e.g., RatesQuote, CreditQuote).
    type Quote;

    /// Type of the curve being built (e.g., DiscountCurve, ForwardCurve).
    type Curve;

    /// Get the time (year fraction) for the knot corresponding to this quote.
    fn quote_time(&self, quote: &Self::Quote) -> Result<f64>;

    /// Build a temporary curve from a set of knots.
    ///
    /// This is called repeatedly during the solver loop.
    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve>;

    /// Build a temporary curve for the solver (fast path, lenient validation).
    fn build_curve_for_solver(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        self.build_curve(knots)
    }

    /// Build the final curve (strict validation).
    fn build_curve_final(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        self.build_curve(knots)
    }

    /// Calculate the pricing residual for a quote given the curve.
    ///
    /// Residual = Model Price - Market Price (or Rate).
    /// Result should be 0.0 when perfectly calibrated.
    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64>;

    /// Provide an initial guess for the solver for the next knot.
    fn initial_guess(&self, quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64>;

    /// Get scan points for root bracketing for the given quote.
    fn scan_points(&self, _quote: &Self::Quote, _initial_guess: f64) -> Result<Vec<f64>> {
        Ok(Vec::new())
    }

    /// Optional: Validate the solved value before accepting it.
    fn validate_knot(&self, _time: f64, _value: f64) -> Result<()> {
        Ok(())
    }
}

/// Trait defining the specific physics for a global optimization process.
pub trait GlobalSolveTarget {
    /// Type of input quote.
    type Quote;

    /// Type of the curve being built.
    type Curve;

    /// Build the time grid and initial guesses for the optimization.
    ///
    /// Returns (times, initial_params, active_quotes).
    fn build_time_grid_and_guesses(
        &self,
        quotes: &[Self::Quote],
    ) -> Result<TimeGridAndGuesses<Self::Quote>>;

    /// Build a curve from parameters (e.g., zero rates).
    fn build_curve_from_params(
        &self,
        times: &[f64],
        params: &[f64],
    ) -> Result<Self::Curve>;

    /// Calculate residuals for all quotes given the curve.
    ///
    /// Populates the `residuals` slice.
    fn calculate_residuals(
        &self,
        curve: &Self::Curve,
        quotes: &[Self::Quote],
        residuals: &mut [f64],
    ) -> Result<()>;
}

