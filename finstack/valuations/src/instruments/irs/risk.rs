//! IRS RiskMeasurable implementation.
//!
//! Provides `RiskMeasurable` for `InterestRateSwap`, composing standard
//! metrics and maturity bucketing into a `RiskReport`.

use crate::instruments::common::traits::Instrument;
use crate::instruments::irs::types::InterestRateSwap;
use crate::metrics::{RiskBucket, RiskMeasurable, RiskReport};
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;

impl RiskMeasurable for InterestRateSwap {
    fn risk_report(
        &self,
        curves: &MarketContext,
        as_of: Date,
        _bucket_spec: Option<&[RiskBucket]>,
    ) -> finstack_core::Result<RiskReport> {
        use crate::metrics::MetricContext;
        use crate::metrics::{standard_registry, MetricId};
        use std::sync::Arc;

        let mut report = RiskReport::new(self.id.as_str(), self.notional.currency());

        // Base PV
        let base_value = self.value(curves, as_of)?;

        // Metric context
        let mut context = MetricContext::new(
            Arc::new(self.clone()),
            Arc::new(curves.clone()),
            as_of,
            base_value,
        );

        // Core risk metrics
        let registry = standard_registry();
        let risk_metrics = [MetricId::Dv01, MetricId::Annuity, MetricId::ParRate];
        for metric_id in &risk_metrics {
            if let Ok(metrics) = registry.compute(&[metric_id.clone()], &mut context) {
                if let Some(value) = metrics.get(metric_id) {
                    report = report.with_metric(metric_id.as_str(), *value);
                }
            }
        }

        // Maturity bucket
        let years_to_maturity = self
            .fixed
            .dc
            .year_fraction(
                as_of,
                self.fixed.end,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        let bucket = if years_to_maturity <= 2.0 {
            RiskBucket {
                id: "2Y".to_string(),
                tenor_years: Some(years_to_maturity),
                classification: Some("Short".to_string()),
            }
        } else if years_to_maturity <= 5.0 {
            RiskBucket {
                id: "5Y".to_string(),
                tenor_years: Some(years_to_maturity),
                classification: Some("Medium".to_string()),
            }
        } else if years_to_maturity <= 10.0 {
            RiskBucket {
                id: "10Y".to_string(),
                tenor_years: Some(years_to_maturity),
                classification: Some("Long".to_string()),
            }
        } else {
            RiskBucket {
                id: "30Y".to_string(),
                tenor_years: Some(years_to_maturity),
                classification: Some("Ultra-Long".to_string()),
            }
        };

        report = report.with_bucket(bucket);

        // Meta
        report
            .meta
            .insert("side".to_string(), format!("{:?}", self.side));
        report
            .meta
            .insert("fixed_rate".to_string(), format!("{:.4}", self.fixed.rate));
        report.meta.insert(
            "float_spread_bp".to_string(),
            format!("{:.1}", self.float.spread_bp),
        );

        Ok(report)
    }

    fn default_risk_buckets(&self) -> Option<Vec<RiskBucket>> {
        Some(vec![
            RiskBucket {
                id: "2Y".to_string(),
                tenor_years: Some(2.0),
                classification: Some("Short".to_string()),
            },
            RiskBucket {
                id: "5Y".to_string(),
                tenor_years: Some(5.0),
                classification: Some("Medium".to_string()),
            },
            RiskBucket {
                id: "10Y".to_string(),
                tenor_years: Some(10.0),
                classification: Some("Long".to_string()),
            },
            RiskBucket {
                id: "30Y".to_string(),
                tenor_years: Some(30.0),
                classification: Some("Ultra-Long".to_string()),
            },
        ])
    }
}
