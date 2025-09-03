use std::boxed::Box;

use crate::{
    market_data::{interp::{InterpFn, ExtrapolationPolicy}, utils::validate_knots},
    F,
};

/// Monotone cubic-Hermite discount-factor interpolator (PCHIP / Fritsch-Carlson).
///
/// The constructor pre-computes first-derivative slopes that guarantee the
/// resulting piece-wise cubic is *shape preserving* when the input discount
/// factors are monotone (strictly decreasing and positive).  Evaluation is
/// O(log N) thanks to binary search on the knot vector.
///
/// See unit tests and `examples/` for usage.
#[derive(Debug)]
pub struct CubicHermite {
    knots: Box<[F]>, // strictly increasing times
    dfs: Box<[F]>,   // discount factors (positive)
    ms: Box<[F]>,    // first-derivative values at each knot
    extrapolation_policy: ExtrapolationPolicy,
}

impl CubicHermite {
    /// Construct a new monotone **cubic‐Hermite** interpolator.
    ///
    /// # Arguments
    /// * `knots` – strictly ascending knot times (years).
    /// * `dfs`   – corresponding discount factors (> 0).
    #[allow(clippy::boxed_local)]
    pub fn new(knots: Box<[F]>, dfs: Box<[F]>) -> crate::Result<Self> {
        debug_assert_eq!(knots.len(), dfs.len());
        // Basic validation – at least two points and strictly ascending times.
        validate_knots(&knots)?;
        // Validate discount factors (positive).
        crate::market_data::utils::validate_dfs(&dfs, false)?;

        // Pre-compute monotone slopes (PCHIP / Fritsch-Carlson).
        let ms = compute_monotone_slopes(&knots, &dfs);

        Ok(Self { 
            knots, 
            dfs, 
            ms, 
            extrapolation_policy: ExtrapolationPolicy::default() 
        })
    }

    /// Extrapolate to the left of the first knot based on the extrapolation policy.
    fn extrapolate_left(&self, x: F) -> F {
        match self.extrapolation_policy {
            ExtrapolationPolicy::FlatZero => self.dfs[0],
            ExtrapolationPolicy::FlatForward => {
                // Flat-forward: extend using the slope from the first knot
                let x0 = self.knots[0];
                let slope = self.ms[0];
                let dx = x - x0;
                // Linear extrapolation: f(x) = f(x0) + m0 * (x - x0)
                self.dfs[0] + slope * dx
            }
        }
    }

    /// Extrapolate to the right of the last knot based on the extrapolation policy.
    fn extrapolate_right(&self, x: F) -> F {
        match self.extrapolation_policy {
            ExtrapolationPolicy::FlatZero => *self.dfs.last().unwrap(),
            ExtrapolationPolicy::FlatForward => {
                // Flat-forward: extend using the slope from the last knot
                let n = self.knots.len();
                let x_last = self.knots[n - 1];
                let slope = self.ms[n - 1];
                let dx = x - x_last;
                // Linear extrapolation: f(x) = f(x_last) + m_last * (x - x_last)
                self.dfs[n - 1] + slope * dx
            }
        }
    }

    /// Compute the derivative for left extrapolation.
    fn extrapolate_left_prime(&self, _x: F) -> F {
        match self.extrapolation_policy {
            ExtrapolationPolicy::FlatZero => 0.0, // Flat extrapolation has zero derivative
            ExtrapolationPolicy::FlatForward => self.ms[0], // Constant slope from first knot
        }
    }

    /// Compute the derivative for right extrapolation.
    fn extrapolate_right_prime(&self, _x: F) -> F {
        match self.extrapolation_policy {
            ExtrapolationPolicy::FlatZero => 0.0, // Flat extrapolation has zero derivative
            ExtrapolationPolicy::FlatForward => self.ms[self.ms.len() - 1], // Constant slope from last knot
        }
    }

    // Shared `locate_segment` from utils is used.
}

impl InterpFn for CubicHermite {
    fn interp(&self, x: F) -> F {
        // Handle extrapolation based on policy
        if x <= self.knots[0] {
            return self.extrapolate_left(x);
        }
        if x >= *self.knots.last().unwrap() {
            return self.extrapolate_right(x);
        }
        
        // Fast-path: exact knot value → short-circuit.
        if let Ok(idx) = self.knots.binary_search_by(|k| k.partial_cmp(&x).unwrap()) {
            return self.dfs[idx];
        }

        // Interior interpolation using cubic Hermite
        let i = crate::market_data::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[i];
        let x1 = self.knots[i + 1];
        let h = x1 - x0;
        // Normalised coordinate t ∈ (0,1).
        let t = (x - x0) / h;
        let t2 = t * t;
        let t3 = t2 * t;

        // Basis functions.
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        // Values and slopes.
        let f0 = self.dfs[i];
        let f1 = self.dfs[i + 1];
        let m0 = self.ms[i];
        let m1 = self.ms[i + 1];

        // Cubic Hermite formula.
        h00 * f0 + h10 * h * m0 + h01 * f1 + h11 * h * m1
    }

    fn interp_prime(&self, x: F) -> F {
        // Handle extrapolation based on policy
        if x <= self.knots[0] {
            return self.extrapolate_left_prime(x);
        }
        if x >= *self.knots.last().unwrap() {
            return self.extrapolate_right_prime(x);
        }
        
        // For exact knot values, return the precomputed slope
        if let Ok(idx) = self.knots.binary_search_by(|k| k.partial_cmp(&x).unwrap()) {
            return self.ms[idx];
        }

        let i = crate::market_data::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[i];
        let x1 = self.knots[i + 1];
        let h = x1 - x0;
        // Normalised coordinate t ∈ (0,1).
        let t = (x - x0) / h;
        let t2 = t * t;

        // Derivative of basis functions w.r.t. t.
        let h00_prime = 6.0 * t2 - 6.0 * t;
        let h10_prime = 3.0 * t2 - 4.0 * t + 1.0;
        let h01_prime = -6.0 * t2 + 6.0 * t;
        let h11_prime = 3.0 * t2 - 2.0 * t;

        // Values and slopes.
        let f0 = self.dfs[i];
        let f1 = self.dfs[i + 1];
        let m0 = self.ms[i];
        let m1 = self.ms[i + 1];

        // Derivative w.r.t. t.
        let df_dt = h00_prime * f0 + h10_prime * h * m0 + h01_prime * f1 + h11_prime * h * m1;
        
        // Convert to derivative w.r.t. x using chain rule: df/dx = (df/dt) * (dt/dx) = (df/dt) / h
        df_dt / h
    }

    fn set_extrapolation_policy(&mut self, policy: ExtrapolationPolicy) {
        self.extrapolation_policy = policy;
    }

    fn extrapolation_policy(&self) -> ExtrapolationPolicy {
        self.extrapolation_policy
    }
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

/// Compute shape-preserving slopes using the Fritsch-Carlson monotone scheme
/// (a.k.a. PCHIP slopes).
#[inline]
fn compute_monotone_slopes(xs: &[F], ys: &[F]) -> Box<[F]> {
    let n = xs.len();
    debug_assert_eq!(n, ys.len());

    // When only two points are available we fall back to linear.
    if n == 2 {
        let slope = (ys[1] - ys[0]) / (xs[1] - xs[0]);
        return vec![slope, slope].into_boxed_slice();
    }

    let mut ms = vec![0.0; n];

    // Compute intervals in a single iterator pass.
    let (h, delta): (Vec<F>, Vec<F>) = xs
        .windows(2)
        .zip(ys.windows(2))
        .map(|(xw, yw)| {
            let hi = xw[1] - xw[0];
            let di = (yw[1] - yw[0]) / hi;
            (hi, di)
        })
        .unzip();

    // Interior points.
    for i in 1..n - 1 {
        if delta[i - 1] == 0.0 || delta[i] == 0.0 || delta[i - 1].signum() != delta[i].signum() {
            ms[i] = 0.0;
        } else {
            let w1 = 2.0 * h[i] + h[i - 1];
            let w2 = h[i] + 2.0 * h[i - 1];
            ms[i] = (w1 + w2) / (w1 / delta[i - 1] + w2 / delta[i]);
        }
    }

    // Endpoints (monotone one-sided estimates).
    // m0
    {
        let mut m0 = ((2.0 * h[0] + h[1]) * delta[0] - h[0] * delta[1]) / (h[0] + h[1]);
        if m0.signum() != delta[0].signum() {
            m0 = 0.0;
        } else if delta[0].signum() != delta[1].signum() && m0.abs() > 3.0 * delta[0].abs() {
            m0 = 3.0 * delta[0];
        }
        ms[0] = m0;
    }
    // m_{n-1}
    {
        let mut mn = ((2.0 * h[n - 2] + h[n - 3]) * delta[n - 2] - h[n - 2] * delta[n - 3])
            / (h[n - 2] + h[n - 3]);
        if mn.signum() != delta[n - 2].signum() {
            mn = 0.0;
        } else if delta[n - 2].signum() != delta[n - 3].signum()
            && mn.abs() > 3.0 * delta[n - 2].abs()
        {
            mn = 3.0 * delta[n - 2];
        }
        ms[n - 1] = mn;
    }

    ms.into_boxed_slice()
}
