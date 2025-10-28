//! Multi-curve context for interest rate derivatives.
//!
//! Separates discounting (OIS) from forwarding (IBOR/SOFR) curves,
//! which is essential for post-crisis rate derivative pricing.

use std::collections::HashMap;
use std::sync::Arc;

/// Tenor specification for IBOR-like rates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tenor {
    /// 1 Month
    M1,
    /// 3 Months
    M3,
    /// 6 Months
    M6,
    /// 12 Months
    M12,
}

impl Tenor {
    /// Get year fraction for tenor.
    pub fn year_fraction(&self) -> f64 {
        match self {
            Tenor::M1 => 1.0 / 12.0,
            Tenor::M3 => 3.0 / 12.0,
            Tenor::M6 => 6.0 / 12.0,
            Tenor::M12 => 1.0,
        }
    }

    /// Get number of months.
    pub fn months(&self) -> u32 {
        match self {
            Tenor::M1 => 1,
            Tenor::M3 => 3,
            Tenor::M6 => 6,
            Tenor::M12 => 12,
        }
    }
}

/// Tenor basis spread (additive spread between two IBOR tenors).
///
/// Example: 3M LIBOR = 6M LIBOR + basis
#[derive(Clone, Debug)]
pub struct TenorBasis {
    /// Reference tenor (e.g., 6M)
    pub reference: Tenor,
    /// Spread tenor (e.g., 3M)
    pub spread_tenor: Tenor,
    /// Additive spread in basis points (e.g., -15 bps means 3M trades 15bp below 6M)
    pub spread_bps: f64,
}

impl TenorBasis {
    /// Create a new tenor basis.
    pub fn new(reference: Tenor, spread_tenor: Tenor, spread_bps: f64) -> Self {
        Self {
            reference,
            spread_tenor,
            spread_bps,
        }
    }

    /// Get spread as decimal (bps / 10000).
    pub fn spread_decimal(&self) -> f64 {
        self.spread_bps / 10_000.0
    }
}

/// Abstract discount curve interface.
///
/// Provides discount factors DF(t) = exp(-∫₀ᵗ r(s) ds).
pub trait DiscountCurve: Send + Sync {
    /// Get discount factor from time 0 to time t.
    fn discount_factor(&self, t: f64) -> f64;

    /// Get instantaneous forward rate at time t.
    fn forward_rate(&self, t: f64) -> f64;

    /// Get zero rate for maturity t.
    fn zero_rate(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return 0.0;
        }
        -self.discount_factor(t).ln() / t
    }
}

/// Abstract forward curve interface.
///
/// Provides forward rates F(t, T) for a specific IBOR tenor.
pub trait ForwardCurve: Send + Sync {
    /// Get forward rate from time t for the curve's tenor.
    fn forward_rate(&self, t: f64) -> f64;

    /// Get the curve's tenor.
    fn tenor(&self) -> Tenor;
}

/// Simple flat curve implementation (for testing/simple cases).
#[derive(Clone, Debug)]
pub struct FlatCurve {
    rate: f64,
}

impl FlatCurve {
    /// Create a flat curve with constant rate.
    pub fn new(rate: f64) -> Self {
        Self { rate }
    }
}

impl DiscountCurve for FlatCurve {
    fn discount_factor(&self, t: f64) -> f64 {
        (-self.rate * t).exp()
    }

    fn forward_rate(&self, _t: f64) -> f64 {
        self.rate
    }
}

/// Simple flat forward curve implementation.
#[derive(Clone, Debug)]
pub struct FlatForwardCurve {
    tenor: Tenor,
    rate: f64,
}

impl FlatForwardCurve {
    /// Create a flat forward curve.
    pub fn new(tenor: Tenor, rate: f64) -> Self {
        Self { tenor, rate }
    }
}

impl ForwardCurve for FlatForwardCurve {
    fn forward_rate(&self, _t: f64) -> f64 {
        self.rate
    }

    fn tenor(&self) -> Tenor {
        self.tenor
    }
}

/// Multi-curve context for interest rate derivative pricing.
///
/// Maintains separate curves for discounting (OIS) and forwarding (IBOR),
/// along with tenor basis adjustments.
///
/// # Example
///
/// ```rust,ignore
/// use finstack_valuations::instruments::common::mc::multi_curve::*;
///
/// // Create OIS curve for discounting
/// let ois = Arc::new(FlatCurve::new(0.04));
///
/// // Create IBOR curves for different tenors
/// let mut ibor_curves = HashMap::new();
/// ibor_curves.insert(Tenor::M3, Arc::new(FlatForwardCurve::new(Tenor::M3, 0.045)));
/// ibor_curves.insert(Tenor::M6, Arc::new(FlatForwardCurve::new(Tenor::M6, 0.046)));
///
/// // Define tenor basis (3M vs 6M)
/// let tenor_basis = vec![
///     TenorBasis::new(Tenor::M6, Tenor::M3, -15.0), // 3M trades 15bp below 6M
/// ];
///
/// let context = MultiCurveContext::new(ois, ibor_curves, tenor_basis);
/// ```
#[derive(Clone)]
pub struct MultiCurveContext {
    /// OIS curve for risk-free discounting
    pub ois_curve: Arc<dyn DiscountCurve>,
    
    /// IBOR forward curves by tenor
    pub ibor_curves: HashMap<Tenor, Arc<dyn ForwardCurve>>,
    
    /// Tenor basis adjustments
    pub tenor_basis: Vec<TenorBasis>,
}

impl MultiCurveContext {
    /// Create a new multi-curve context.
    pub fn new(
        ois_curve: Arc<dyn DiscountCurve>,
        ibor_curves: HashMap<Tenor, Arc<dyn ForwardCurve>>,
        tenor_basis: Vec<TenorBasis>,
    ) -> Self {
        Self {
            ois_curve,
            ibor_curves,
            tenor_basis,
        }
    }

    /// Create a simple single-curve context (pre-crisis style).
    ///
    /// Uses the same curve for discounting and forwarding.
    pub fn single_curve(rate: f64) -> Self {
        let curve = Arc::new(FlatCurve::new(rate));
        let mut ibor_curves: HashMap<Tenor, Arc<dyn ForwardCurve>> = HashMap::new();
        
        for tenor in [Tenor::M1, Tenor::M3, Tenor::M6, Tenor::M12] {
            ibor_curves.insert(
                tenor,
                Arc::new(FlatForwardCurve::new(tenor, rate)) as Arc<dyn ForwardCurve>,
            );
        }

        Self {
            ois_curve: curve,
            ibor_curves,
            tenor_basis: Vec::new(),
        }
    }

    /// Get discount factor for time t (from OIS curve).
    pub fn discount_factor(&self, t: f64) -> f64 {
        self.ois_curve.discount_factor(t)
    }

    /// Get forward rate for specific tenor at time t.
    pub fn forward_rate(&self, tenor: Tenor, t: f64) -> f64 {
        if let Some(curve) = self.ibor_curves.get(&tenor) {
            let base_forward = curve.forward_rate(t);
            
            // Apply tenor basis adjustments
            let basis_adjustment = self.get_tenor_basis_adjustment(tenor);
            
            base_forward + basis_adjustment
        } else {
            // Fallback to OIS if tenor not found
            self.ois_curve.forward_rate(t)
        }
    }

    /// Get tenor basis adjustment for a given tenor.
    fn get_tenor_basis_adjustment(&self, tenor: Tenor) -> f64 {
        self.tenor_basis
            .iter()
            .filter(|basis| basis.spread_tenor == tenor)
            .map(|basis| basis.spread_decimal())
            .sum()
    }

    /// Check if context has curve for specified tenor.
    pub fn has_tenor(&self, tenor: Tenor) -> bool {
        self.ibor_curves.contains_key(&tenor)
    }

    /// Get all available tenors.
    pub fn available_tenors(&self) -> Vec<Tenor> {
        self.ibor_curves.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_curve() {
        let curve = FlatCurve::new(0.05);
        
        assert_eq!(curve.discount_factor(0.0), 1.0);
        assert!((curve.discount_factor(1.0) - 0.05_f64.exp().recip()).abs() < 1e-10);
        assert!((curve.forward_rate(0.5) - 0.05).abs() < 1e-10);
        assert!((curve.zero_rate(1.0) - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_flat_forward_curve() {
        let curve = FlatForwardCurve::new(Tenor::M3, 0.045);
        
        assert_eq!(curve.forward_rate(0.5), 0.045);
        assert_eq!(curve.tenor(), Tenor::M3);
    }

    #[test]
    fn test_tenor_year_fractions() {
        assert_eq!(Tenor::M1.year_fraction(), 1.0 / 12.0);
        assert_eq!(Tenor::M3.year_fraction(), 0.25);
        assert_eq!(Tenor::M6.year_fraction(), 0.5);
        assert_eq!(Tenor::M12.year_fraction(), 1.0);
    }

    #[test]
    fn test_tenor_basis() {
        let basis = TenorBasis::new(Tenor::M6, Tenor::M3, -15.0);
        
        assert_eq!(basis.spread_bps, -15.0);
        assert_eq!(basis.spread_decimal(), -0.0015);
    }

    #[test]
    fn test_single_curve_context() {
        let context = MultiCurveContext::single_curve(0.05);
        
        assert_eq!(context.discount_factor(1.0), (-0.05_f64).exp());
        assert_eq!(context.forward_rate(Tenor::M3, 0.5), 0.05);
        assert_eq!(context.forward_rate(Tenor::M6, 0.5), 0.05);
        
        assert!(context.has_tenor(Tenor::M3));
        assert_eq!(context.available_tenors().len(), 4);
    }

    #[test]
    fn test_multi_curve_context_with_basis() {
        let ois = Arc::new(FlatCurve::new(0.04)) as Arc<dyn DiscountCurve>;
        
        let mut ibor_curves: HashMap<Tenor, Arc<dyn ForwardCurve>> = HashMap::new();
        ibor_curves.insert(
            Tenor::M3,
            Arc::new(FlatForwardCurve::new(Tenor::M3, 0.045)),
        );
        ibor_curves.insert(
            Tenor::M6,
            Arc::new(FlatForwardCurve::new(Tenor::M6, 0.046)),
        );

        // 3M trades 15bp below 6M
        let tenor_basis = vec![TenorBasis::new(Tenor::M6, Tenor::M3, -15.0)];

        let context = MultiCurveContext::new(ois, ibor_curves, tenor_basis);

        // OIS discounting
        assert!((context.discount_factor(1.0) - (-0.04_f64).exp()).abs() < 1e-10);

        // 3M forward (with basis adjustment)
        let fwd_3m = context.forward_rate(Tenor::M3, 0.5);
        assert!((fwd_3m - (0.045 - 0.0015)).abs() < 1e-10); // 4.50% - 15bp = 4.35%

        // 6M forward (no basis adjustment to itself)
        let fwd_6m = context.forward_rate(Tenor::M6, 0.5);
        assert_eq!(fwd_6m, 0.046);
    }
}

