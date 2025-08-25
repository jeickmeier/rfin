//! Root-finding algorithms: Brent and safeguarded Newton.
//!
//! Both solvers are deterministic for a given function and interval.

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
