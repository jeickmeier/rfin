#[inline]
pub(crate) fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol.max(1e-15)
}
