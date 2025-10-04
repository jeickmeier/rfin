//! Weighted Average Rating Factor calculator for CLO

use crate::metrics::MetricContext;
use crate::instruments::common::structured_credit::CreditRating;

/// CLO WARF calculator - Moody's methodology
pub struct CloWarfCalculator;

impl crate::metrics::MetricCalculator for CloWarfCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::clo::Clo>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        let mut weighted_sum = 0.0;
        let mut total_balance = 0.0;

        for asset in &clo.pool.assets {
            let balance = asset.balance.amount();
            let rating_factor = asset
                .credit_quality
                .map(get_moody_rating_factor)
                .unwrap_or(3650.0); // Default to B-/CCC+ equivalent

            weighted_sum += balance * rating_factor;
            total_balance += balance;
        }

        if total_balance > 0.0 {
            Ok(weighted_sum / total_balance)
        } else {
            Ok(0.0)
        }
    }
}

/// Get Moody's rating factor for WARF calculation
fn get_moody_rating_factor(rating: CreditRating) -> f64 {
    match rating {
        CreditRating::AAA => 1.0,
        CreditRating::AA => 10.0,
        CreditRating::A => 40.0,
        CreditRating::BBB => 260.0,
        CreditRating::BB => 1350.0,
        CreditRating::B => 2720.0,
        CreditRating::CCC => 6500.0,
        CreditRating::CC => 8070.0,
        CreditRating::C => 10000.0,
        CreditRating::D => 10000.0,
        CreditRating::NR => 3650.0,
    }
}

