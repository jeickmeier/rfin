//! Carr-Madan discrete variance replication integral.
//!
//! Provides a shared implementation of the log-contract replication approach
//! (Carr & Madan, 1998) used by both equity and FX variance swap pricers.

use crate::instruments::common_impl::parameters::market::OptionType;

/// Carr-Madan discrete variance replication integral.
///
/// Computes forward variance from a discrete strike grid using the
/// log-contract replication approach (Carr & Madan, 1998).
///
/// `vol_fn(t, k)` returns implied volatility at time `t` and strike `k`.
/// `bs_price_fn(strike, vol, option_type)` returns the Black-Scholes option price.
/// All other parameters (spot, rates, etc.) should be captured by the closures.
///
/// Returns `None` if the result is non-finite or non-positive.
pub fn carr_madan_forward_variance(
    strikes: &[f64],
    forward: f64,
    risk_free_rate: f64,
    time_to_expiry: f64,
    vol_fn: impl Fn(f64, f64) -> f64,
    bs_price_fn: impl Fn(f64, f64, OptionType) -> f64,
) -> Option<f64> {
    if strikes.len() < 3 || !forward.is_finite() || forward <= 0.0 {
        return None;
    }

    // Strike grid must be strictly increasing and finite. Carr-Madan integration
    // assumes a monotone integration domain; non-monotonic or duplicate strikes
    // produce silently nonsensical variance because `dk` (the trapezoidal width)
    // becomes zero or negative.
    if !strikes.iter().all(|k| k.is_finite() && *k > 0.0) {
        return None;
    }
    if !strikes.windows(2).all(|w| w[0] < w[1]) {
        return None;
    }

    // Find the highest strike at or below the forward
    let k0_idx = {
        let mut idx = 0usize;
        for (i, &k) in strikes.iter().enumerate() {
            if k <= forward {
                idx = i;
            } else {
                break;
            }
        }
        idx
    };
    let k0 = strikes[k0_idx].max(1e-12);

    let mut sum = 0.0;
    for i in 0..strikes.len() {
        let k = strikes[i].max(1e-12);
        let dk = if i == 0 {
            strikes[1] - strikes[0]
        } else if i + 1 == strikes.len() {
            strikes[i] - strikes[i - 1]
        } else {
            0.5 * (strikes[i + 1] - strikes[i - 1])
        };

        let vol = vol_fn(time_to_expiry, k).max(1e-8);

        let qk = if i == k0_idx {
            0.5 * (bs_price_fn(k, vol, OptionType::Put) + bs_price_fn(k, vol, OptionType::Call))
        } else if k < forward {
            bs_price_fn(k, vol, OptionType::Put)
        } else {
            bs_price_fn(k, vol, OptionType::Call)
        };

        sum += (dk / (k * k)) * qk;
    }

    let variance = (2.0 * (risk_free_rate * time_to_expiry).exp() / time_to_expiry) * sum
        - (1.0 / time_to_expiry) * ((forward / k0 - 1.0).powi(2));

    if variance.is_finite() && variance > 0.0 {
        Some(variance)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::models::closed_form::vanilla::bs_price;

    #[test]
    fn test_carr_madan_atm_flat_vol() {
        let vol = 0.20;
        let t = 1.0;
        let fwd = 100.0;
        let r = 0.05;
        let spot = fwd;
        let strikes: Vec<f64> = (50..=150).map(|k| k as f64).collect();
        let vol_fn = |_t: f64, _k: f64| vol;
        let bs_fn =
            |k: f64, v: f64, opt: OptionType| -> f64 { bs_price(spot, k, r, 0.0, v, t, opt) };
        let variance = carr_madan_forward_variance(&strikes, fwd, r, t, vol_fn, bs_fn)
            .expect("Expected Some variance");
        assert!(
            (variance - vol * vol).abs() < 0.01,
            "Expected ~{}, got {}",
            vol * vol,
            variance
        );
    }

    #[test]
    fn test_carr_madan_returns_none_for_too_few_strikes() {
        let vol_fn = |_t: f64, _k: f64| 0.2;
        let bs_fn = |_k: f64, _v: f64, _opt: OptionType| -> f64 { 1.0 };
        assert!(
            carr_madan_forward_variance(&[100.0, 101.0], 100.0, 0.05, 1.0, vol_fn, bs_fn).is_none()
        );
    }

    #[test]
    fn test_carr_madan_returns_none_for_invalid_forward() {
        let vol_fn = |_t: f64, _k: f64| 0.2;
        let bs_fn = |_k: f64, _v: f64, _opt: OptionType| -> f64 { 1.0 };
        let strikes: Vec<f64> = (50..=150).map(|k| k as f64).collect();
        assert!(
            carr_madan_forward_variance(&strikes, f64::NAN, 0.05, 1.0, vol_fn, bs_fn).is_none()
        );
        assert!(carr_madan_forward_variance(&strikes, -1.0, 0.05, 1.0, vol_fn, bs_fn).is_none());
    }

    #[test]
    fn test_carr_madan_rejects_non_monotonic_strike_grid() {
        let vol_fn = |_t: f64, _k: f64| 0.2;
        let bs_fn = |_k: f64, _v: f64, _opt: OptionType| -> f64 { 1.0 };
        // Non-monotonic — third entry breaks the ordering
        let strikes = vec![80.0, 100.0, 95.0, 120.0];
        assert!(
            carr_madan_forward_variance(&strikes, 100.0, 0.05, 1.0, vol_fn, bs_fn).is_none(),
            "non-monotonic strike grid must be rejected"
        );
        // Duplicate strike — the dk for the duplicate is zero
        let dup_strikes = vec![80.0, 100.0, 100.0, 120.0];
        assert!(
            carr_madan_forward_variance(&dup_strikes, 100.0, 0.05, 1.0, vol_fn, bs_fn).is_none(),
            "duplicate strikes must be rejected"
        );
    }

    #[test]
    fn test_carr_madan_rejects_non_finite_strike() {
        let vol_fn = |_t: f64, _k: f64| 0.2;
        let bs_fn = |_k: f64, _v: f64, _opt: OptionType| -> f64 { 1.0 };
        let strikes = vec![80.0, f64::NAN, 120.0];
        assert!(carr_madan_forward_variance(&strikes, 100.0, 0.05, 1.0, vol_fn, bs_fn).is_none());
    }
}
