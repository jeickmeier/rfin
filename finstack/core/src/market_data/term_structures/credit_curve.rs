//! Credit curves for risky discounting and credit spread calculations.

use crate::error::InputError;
use crate::types::CurveId;
use crate::market_data::traits::TermStructure;
use crate::prelude::*;
use crate::F;

/// Interpolation method for credit spreads.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Interpolation {
    /// Linear interpolation
    Linear,
    /// Log-linear interpolation
    LogLinear,
    /// Cubic spline interpolation
    CubicSpline,
    /// Monotone convex interpolation
    MonotoneConvex,
}

/// Seniority level for credit exposures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Seniority {
    /// Senior secured debt
    SeniorSecured,
    /// Senior unsecured debt
    Senior,
    /// Subordinated debt
    Subordinated,
    /// Junior/mezzanine debt
    Junior,
}

impl std::fmt::Display for Seniority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Seniority::SeniorSecured => write!(f, "SeniorSecured"),
            Seniority::Senior => write!(f, "Senior"),
            Seniority::Subordinated => write!(f, "Subordinated"),
            Seniority::Junior => write!(f, "Junior"),
        }
    }
}

/// Credit curve for computing survival probabilities and risky discount factors.
///
/// Models the credit risk of an issuer at a specific seniority level.
/// Uses credit spreads or hazard rates to compute survival probabilities.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CreditCurve {
    /// Curve identifier (typically issuer + seniority)
    pub id: CurveId,
    /// Issuer name
    pub issuer: String,
    /// Debt seniority level
    pub seniority: Seniority,
    /// Recovery rate (fraction of notional recovered in default)
    pub recovery_rate: F,
    /// Tenors in years from curve base date
    pub tenors: Vec<F>,
    /// Credit spreads in basis points at each tenor
    pub spreads_bp: Vec<F>,
    /// Interpolation method for spreads
    pub interpolation: Interpolation,
    /// Base date for the curve
    pub base_date: Date,
}

impl CreditCurve {
    /// Create a new credit curve builder.
    pub fn builder(id: &'static str) -> CreditCurveBuilder {
        CreditCurveBuilder::new(id)
    }

    /// Get interpolated spread at a given tenor in basis points.
    pub fn spread_bp(&self, tenor_years: F) -> F {
        if tenor_years <= 0.0 {
            return 0.0;
        }

        // Find bracketing points
        match self.find_bracket(tenor_years) {
            Some((i, j)) => {
                let x1 = self.tenors[i];
                let x2 = self.tenors[j];
                let y1 = self.spreads_bp[i];
                let y2 = self.spreads_bp[j];

                match self.interpolation {
                    Interpolation::Linear => {
                        let t = (tenor_years - x1) / (x2 - x1);
                        y1 + t * (y2 - y1)
                    }
                    Interpolation::LogLinear => {
                        if y1 <= 0.0 || y2 <= 0.0 {
                            // Fall back to linear if spreads are non-positive
                            let t = (tenor_years - x1) / (x2 - x1);
                            y1 + t * (y2 - y1)
                        } else {
                            let log_y1 = y1.ln();
                            let log_y2 = y2.ln();
                            let t = (tenor_years - x1) / (x2 - x1);
                            (log_y1 + t * (log_y2 - log_y1)).exp()
                        }
                    }
                    _ => {
                        // For other methods, default to linear
                        let t = (tenor_years - x1) / (x2 - x1);
                        y1 + t * (y2 - y1)
                    }
                }
            }
            None => {
                // Extrapolate flat
                if tenor_years < self.tenors[0] {
                    self.spreads_bp[0]
                } else {
                    *self.spreads_bp.last().unwrap()
                }
            }
        }
    }

    /// Compute survival probability to a given date.
    ///
    /// Uses the approximation: P(survival) = exp(-spread * t / 10000)
    /// where spread is in basis points and t is time in years.
    pub fn survival_probability(&self, date: Date) -> F {
        let tenor = self.year_fraction_from_base(date);
        if tenor <= 0.0 {
            return 1.0;
        }

        let spread_bp = self.spread_bp(tenor);
        let hazard_rate = spread_bp / 10000.0 / (1.0 - self.recovery_rate);
        (-hazard_rate * tenor).exp()
    }

    /// Compute risky discount factor (survival probability * risk-free DF).
    ///
    /// Note: This method only returns the credit component. The caller must
    /// multiply by the risk-free discount factor.
    pub fn credit_discount_factor(&self, date: Date) -> F {
        self.survival_probability(date)
    }

    /// Compute the risky PV01 (present value of 1bp spread move).
    ///
    /// This represents the sensitivity to a 1bp parallel shift in credit spreads.
    pub fn risky_pv01(&self, date: Date) -> F {
        let tenor = self.year_fraction_from_base(date);
        if tenor <= 0.0 {
            return 0.0;
        }

        // Approximate as: tenor * survival_probability * 0.0001
        let sp = self.survival_probability(date);
        tenor * sp * 0.0001
    }

    fn find_bracket(&self, tenor: F) -> Option<(usize, usize)> {
        if self.tenors.is_empty() {
            return None;
        }

        // Binary search for bracketing interval
        let pos = self
            .tenors
            .binary_search_by(|x| x.partial_cmp(&tenor).unwrap_or(std::cmp::Ordering::Equal));

        match pos {
            Ok(i) => {
                // Exact match
                if i == 0 && self.tenors.len() > 1 {
                    Some((0, 1))
                } else if i > 0 {
                    Some((i - 1, i))
                } else {
                    None
                }
            }
            Err(i) => {
                if i == 0 || i >= self.tenors.len() {
                    None
                } else {
                    Some((i - 1, i))
                }
            }
        }
    }

    fn year_fraction_from_base(&self, date: Date) -> F {
        use crate::market_data::term_structures::discount_curve::DiscountCurve;
        DiscountCurve::year_fraction(self.base_date, date, DayCount::Act365F)
    }
}

impl TermStructure for CreditCurve {
    fn id(&self) -> &CurveId {
        &self.id
    }
}

/// Builder for creating credit curves.
pub struct CreditCurveBuilder {
    id: &'static str,
    issuer: Option<String>,
    seniority: Option<Seniority>,
    recovery_rate: Option<F>,
    tenors: Vec<F>,
    spreads_bp: Vec<F>,
    interpolation: Interpolation,
    base_date: Option<Date>,
}

impl CreditCurveBuilder {
    /// Create a new builder with the given curve ID.
    pub fn new(id: &'static str) -> Self {
        Self {
            id,
            issuer: None,
            seniority: None,
            recovery_rate: None,
            tenors: Vec::new(),
            spreads_bp: Vec::new(),
            interpolation: Interpolation::Linear,
            base_date: None,
        }
    }

    /// Set the issuer name.
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Set the seniority level.
    pub fn seniority(mut self, seniority: Seniority) -> Self {
        self.seniority = Some(seniority);
        self
    }

    /// Set the recovery rate (0.0 to 1.0).
    pub fn recovery_rate(mut self, rate: F) -> Self {
        self.recovery_rate = Some(rate);
        self
    }

    /// Set the base date for the curve.
    pub fn base_date(mut self, date: Date) -> Self {
        self.base_date = Some(date);
        self
    }

    /// Add a single spread point.
    pub fn add_spread(mut self, tenor_years: F, spread_bp: F) -> Self {
        self.tenors.push(tenor_years);
        self.spreads_bp.push(spread_bp);
        self
    }

    /// Set all spread points at once.
    pub fn spreads<I>(mut self, points: I) -> Self
    where
        I: IntoIterator<Item = (F, F)>,
    {
        for (tenor, spread) in points {
            self.tenors.push(tenor);
            self.spreads_bp.push(spread);
        }
        self
    }

    /// Set the interpolation method.
    pub fn interpolation(mut self, method: Interpolation) -> Self {
        self.interpolation = method;
        self
    }

    /// Build the credit curve.
    pub fn build(self) -> Result<CreditCurve> {
        // Validate inputs
        let issuer = self.issuer.unwrap_or_else(|| "Unknown".to_string());
        let seniority = self.seniority.unwrap_or(Seniority::Senior);
        let recovery_rate = self.recovery_rate.unwrap_or(0.4); // Standard 40% recovery
        let base_date = self.base_date.ok_or(InputError::NotFound)?;

        if self.tenors.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }

        if self.tenors.len() != self.spreads_bp.len() {
            return Err(InputError::DimensionMismatch.into());
        }

        // Sort by tenor
        let mut points: Vec<_> = self.tenors.into_iter().zip(self.spreads_bp).collect();
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let (tenors, spreads_bp): (Vec<_>, Vec<_>) = points.into_iter().unzip();

        Ok(CreditCurve {
            id: CurveId::new(self.id),
            issuer,
            seniority,
            recovery_rate,
            tenors,
            spreads_bp,
            interpolation: self.interpolation,
            base_date,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_credit_curve_creation() {
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let curve = CreditCurve::builder("AAPL_SENIOR")
            .issuer("Apple Inc.")
            .seniority(Seniority::Senior)
            .recovery_rate(0.4)
            .base_date(base)
            .spreads(vec![(0.5, 50.0), (1.0, 60.0), (2.0, 75.0), (5.0, 100.0)])
            .build()
            .unwrap();

        assert_eq!(curve.issuer, "Apple Inc.");
        assert_eq!(curve.seniority, Seniority::Senior);
        assert_eq!(curve.recovery_rate, 0.4);
    }

    #[test]
    fn test_spread_interpolation() {
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let curve = CreditCurve::builder("TEST")
            .base_date(base)
            .spreads(vec![(1.0, 50.0), (2.0, 100.0)])
            .build()
            .unwrap();

        // At pillar points
        assert_eq!(curve.spread_bp(1.0), 50.0);
        assert_eq!(curve.spread_bp(2.0), 100.0);

        // Interpolated
        assert_eq!(curve.spread_bp(1.5), 75.0);

        // Extrapolated flat
        assert_eq!(curve.spread_bp(0.5), 50.0);
        assert_eq!(curve.spread_bp(3.0), 100.0);
    }

    #[test]
    fn test_survival_probability() {
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let one_year = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let curve = CreditCurve::builder("TEST")
            .base_date(base)
            .recovery_rate(0.4)
            .spreads(vec![(1.0, 100.0)]) // 100bp spread
            .build()
            .unwrap();

        let sp = curve.survival_probability(one_year);

        // With 100bp spread and 40% recovery:
        // hazard_rate = 0.01 / 0.6 ≈ 0.01667
        // survival = exp(-0.01667) ≈ 0.9835
        assert!((sp - 0.9835).abs() < 0.001);
    }

    #[test]
    fn test_risky_pv01() {
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let one_year = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let curve = CreditCurve::builder("TEST")
            .base_date(base)
            .recovery_rate(0.4)
            .spreads(vec![(1.0, 100.0)])
            .build()
            .unwrap();

        let rpv01 = curve.risky_pv01(one_year);

        // Should be approximately 1 year * survival_prob * 0.0001
        let expected = 1.0 * 0.9835 * 0.0001;
        assert!((rpv01 - expected).abs() < 0.00001);
    }
}
