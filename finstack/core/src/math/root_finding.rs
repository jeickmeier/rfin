//! Root-finding algorithms: Brent, Newton-Raphson, and safeguarded Newton.
//!
//! All solvers are deterministic for a given function and interval.

/// Brent's method for finding a root of `f` on `[lo, hi]`.
///
/// Requirements: `f(lo)` and `f(hi)` must have opposite signs.
pub fn brent<F>(mut f: F, lo: f64, hi: f64, tol: f64, max_iter: usize) -> Result<f64, crate::Error>
where
    F: FnMut(f64) -> f64,
{
    use crate::error::InputError;

    let flo = f(lo);
    let fhi = f(hi);
    if !(flo.is_finite() && fhi.is_finite()) || flo == 0.0 {
        return Ok(lo);
    }
    if fhi == 0.0 {
        return Ok(hi);
    }
    if flo.signum() == fhi.signum() {
        return Err(InputError::Invalid.into());
    }

    let mut a = lo;
    let mut b = hi;
    let mut fa = flo;
    let mut fb = fhi;
    let mut c = a;
    let mut fc = fa;
    let mut d = b - a;
    let mut e = d;

    for _ in 0..max_iter {
        if fb.signum() == fc.signum() {
            c = a;
            fc = fa;
            d = b - a;
            e = d;
        }
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        // Convergence checks
        let tol1 = 2.0 * f64::EPSILON * b.abs() + 0.5 * tol;
        let xm = 0.5 * (c - b);
        if xm.abs() <= tol1 || fb == 0.0 {
            return Ok(b);
        }

        if e.abs() >= tol1 && fa.abs() > fb.abs() {
            // Attempt inverse quadratic interpolation or secant
            let s = fb / fa;
            let (p, q) = if a == c {
                // Secant method
                (2.0 * xm * s, 1.0 - s)
            } else {
                // Inverse quadratic interpolation
                let q1 = fa / fc;
                let r = fb / fc;
                let p = s * (2.0 * xm * q1 * (q1 - r) - (b - a) * (r - 1.0));
                let q = (q1 - 1.0) * (r - 1.0) * (s - 1.0);
                (p, q)
            };
            let mut p = p;
            let mut q = q;
            if p > 0.0 {
                q = -q;
            } else {
                p = -p;
            }
            let cond1 = 2.0 * p < 3.0 * xm * q - (tol1 * q).abs();
            let cond2 = p < (e * q).abs() * 0.5;
            if cond1 && cond2 {
                e = d;
                d = p / q;
            } else {
                d = xm;
                e = d;
            }
        } else {
            d = xm;
            e = d;
        }

        a = b;
        fa = fb;
        if d.abs() > tol1 {
            b += d;
        } else {
            b += tol1.copysign(xm);
        }
        fb = f(b);
    }

    Ok(b)
}

/// Safeguarded Newton step inside a bracket `[lo, hi]`.
/// Falls back to bisection when derivative is small or proposes an out-of-bracket step.
pub fn newton_bracketed<F, G>(
    mut f: F,
    mut df: G,
    lo: f64,
    hi: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, crate::Error>
where
    F: FnMut(f64) -> f64,
    G: FnMut(f64) -> f64,
{
    use crate::error::InputError;

    let flo = f(lo);
    let fhi = f(hi);
    if flo.signum() == fhi.signum() {
        return Err(InputError::Invalid.into());
    }
    let mut a = lo;
    let mut b = hi;
    let mut fa = flo;

    let mut x = 0.5 * (a + b);
    for _ in 0..max_iter {
        let fx = f(x);
        if fx.abs() <= tol {
            return Ok(x);
        }
        let dfx = df(x);
        let step = if dfx.abs() > f64::EPSILON {
            -fx / dfx
        } else {
            0.0
        };
        let mut x_new = x + step;
        if x_new <= a || x_new >= b || step.abs() < tol {
            x_new = 0.5 * (a + b);
        }
        let f_new = f(x_new);
        if fa.signum() != f_new.signum() {
            b = x_new;
        } else {
            a = x_new;
            fa = f_new;
        }
        x = x_new;
        if (b - a).abs() <= tol {
            return Ok(0.5 * (a + b));
        }
    }
    Ok(x)
}

/// Newton-Raphson root finding algorithm (unbounded).
///
/// Finds a root of `f` using Newton's method with derivative `f_prime`.
/// This is faster than Brent's method when it converges but less robust.
///
/// # Arguments
/// * `f` - The function to find the root of
/// * `f_prime` - The derivative of `f`
/// * `x0` - Initial guess
/// * `tol` - Tolerance for convergence
/// * `max_iter` - Maximum number of iterations
///
/// # Returns
/// The root if found within tolerance and iteration limits.
pub fn newton_raphson<F, G>(
    f: F,
    f_prime: G,
    x0: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, crate::Error>
where
    F: Fn(f64) -> f64,
    G: Fn(f64) -> f64,
{
    use crate::error::InputError;
    
    let mut x = x0;
    
    for _ in 0..max_iter {
        let fx = f(x);
        
        // Check for convergence
        if fx.abs() < tol {
            return Ok(x);
        }
        
        let fpx = f_prime(x);
        
        // Avoid division by zero
        if fpx.abs() < 1e-10 {
            return Err(InputError::Invalid.into());
        }
        
        let x_new = x - fx / fpx;
        
        // Check for convergence in x
        if (x_new - x).abs() < tol {
            return Ok(x_new);
        }
        
        x = x_new;
        
        // Keep rate within reasonable bounds for financial applications
        if !(-0.999..=10.0).contains(&x) {
            return Err(InputError::Invalid.into());
        }
    }
    
    Err(InputError::Invalid.into())
}

/// Brent's method with automatic bracketing interval search.
///
/// More robust version of Brent's method that attempts to find a bracketing
/// interval if the initial interval doesn't bracket a root.
pub fn brent_with_bracketing<F>(
    f: F,
    mut a: f64,
    mut b: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, crate::Error>
where
    F: Fn(f64) -> f64,
{
    use crate::error::InputError;
    
    let mut fa = f(a);
    let mut fb = f(b);
    
    // Check if root is already at boundaries
    if fa.abs() < tol {
        return Ok(a);
    }
    if fb.abs() < tol {
        return Ok(b);
    }
    
    // Ensure bracketing
    if fa * fb > 0.0 {
        // Try to find bracketing interval
        let (new_a, new_b) = find_bracketing_interval(&f, a, b)?;
        a = new_a;
        b = new_b;
        fa = f(a);
        fb = f(b);
    }
    
    let mut c = a;
    let mut fc = fa;
    let mut _d = b - a;  // Track step size (used for algorithm state)
    let mut e = b - a;
    
    for _ in 0..max_iter {
        if fb.abs() < fa.abs() {
            // Swap
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut fa, &mut fb);
        }
        
        let tolerance = 2.0 * f64::EPSILON * b.abs() + tol;
        let m = 0.5 * (c - b);
        
        if m.abs() <= tolerance || fb.abs() < tol {
            return Ok(b);
        }
        
        if e.abs() >= tolerance && fa.abs() > fb.abs() {
            let s = if (a - c).abs() < f64::EPSILON {
                // Linear interpolation
                fb / fa
            } else {
                // Inverse quadratic interpolation
                let q = fa / fc;
                let r = fb / fc;
                let p = 2.0 * m * q * (q - r) - (b - a) * (r - 1.0);
                let q_val = (q - 1.0) * (r - 1.0);
                p / q_val
            };
            
            let s = s.max(-0.75 * m).min(0.75 * m);
            let _d = e;  // Track previous step size (for algorithm correctness)
            e = s;
        } else {
            // Bisection
            let _d = m;  // Track step size (for algorithm correctness)
            e = m;
        }
        
        a = b;
        fa = fb;
        
        if e.abs() > tolerance {
            b += e;
        } else {
            b += if m > 0.0 { tolerance } else { -tolerance };
        }
        
        fb = f(b);
        
        if fa * fb > 0.0 {
            c = a;
            fc = fa;
            _d = b - a;
            e = _d;
        }
    }
    
    Err(InputError::Invalid.into())
}

/// Try to find a bracketing interval for the root.
///
/// Given an initial interval [a, b], attempts to find values where
/// f(a) and f(b) have opposite signs by expanding the interval.
pub fn find_bracketing_interval<F>(
    f: &F,
    initial_a: f64,
    initial_b: f64,
) -> Result<(f64, f64), crate::Error>
where
    F: Fn(f64) -> f64,
{
    use crate::error::InputError;
    
    // Try expanding the interval
    let mut a = initial_a;
    let mut b = initial_b;
    
    for _ in 0..10 {
        let fa = f(a);
        let fb = f(b);
        
        if fa * fb < 0.0 {
            return Ok((a, b));
        }
        
        // Expand interval
        a = a * 2.0 - 1.0;
        b = b * 2.0 + 1.0;
        
        // Keep within reasonable bounds
        a = a.max(-0.999);
        b = b.min(10.0);
    }
    
    Err(InputError::Invalid.into())
}
