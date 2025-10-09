//! Macro to implement TrancheValuationExt for structured credit instruments.

/// Macro to implement TrancheValuationExt for any StructuredCreditInstrument
#[macro_export]
macro_rules! impl_tranche_valuation_ext {
    ($type:ty) => {
        impl $crate::instruments::common::structured_credit::TrancheValuationExt for $type {
            /// Generate cashflows for a specific tranche after waterfall allocation
            fn get_tranche_cashflows(
                &self,
                tranche_id: &str,
                context: &finstack_core::market_data::MarketContext,
                as_of: finstack_core::dates::Date,
            ) -> finstack_core::Result<$crate::instruments::common::structured_credit::TrancheCashflowResult> {
                use $crate::instruments::common::structured_credit::StructuredCreditInstrument;
                <Self as StructuredCreditInstrument>::generate_specific_tranche_cashflows(
                    self,
                    tranche_id,
                    context,
                    as_of,
                )
            }

            /// Calculate present value for a specific tranche
            fn value_tranche(
                &self,
                tranche_id: &str,
                context: &finstack_core::market_data::MarketContext,
                as_of: finstack_core::dates::Date,
            ) -> finstack_core::Result<finstack_core::money::Money> {
                let cashflows = self.get_tranche_cashflows(tranche_id, context, as_of)?;
                let disc = context.curves.get_discount_ref(self.discount_curve_id())?;
                
                let mut pv = finstack_core::money::Money::new(0.0, self.pool().base_currency());
                for (date, amount) in &cashflows.cashflows {
                    if *date > as_of {
                        let df = disc.df_on_date_curve(*date);
                        let flow_pv = finstack_core::money::Money::new(amount.amount() * df, amount.currency());
                        pv = pv.checked_add(flow_pv)?;
                    }
                }
                
                Ok(pv)
            }

            /// Get full valuation with metrics for a specific tranche
            fn value_tranche_with_metrics(
                &self,
                tranche_id: &str,
                context: &finstack_core::market_data::MarketContext,
                as_of: finstack_core::dates::Date,
                metrics: &[$crate::metrics::MetricId],
            ) -> finstack_core::Result<$crate::instruments::common::structured_credit::TrancheValuation> {
                use $crate::instruments::common::structured_credit::{
                    calculate_tranche_cs01, calculate_tranche_duration,
                    calculate_tranche_wal, calculate_tranche_z_spread,
                };
                
                // Get tranche-specific cashflows
                let cashflow_result = self.get_tranche_cashflows(tranche_id, context, as_of)?;
                
                // Calculate PV
                let pv = self.value_tranche(tranche_id, context, as_of)?;
                
                // Get tranche for notional
                let tranche = self.tranches()
                    .tranches
                    .iter()
                    .find(|t| t.id.as_str() == tranche_id)
                    .ok_or_else(|| {
                        finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                            id: format!("tranche:{}", tranche_id),
                        })
                    })?;
                
                let notional = tranche.original_balance.amount();
                
                // Calculate prices
                let dirty_price = if notional > 0.0 {
                    (pv.amount() / notional) * 100.0
                } else {
                    0.0
                };
                
                // Simple accrued calculation
                let accrued = finstack_core::money::Money::new(0.0, pv.currency());
                let clean_price = dirty_price; // Simplified
                
                // Calculate metrics
                let wal = calculate_tranche_wal(&cashflow_result, as_of)?;
                
                let disc = context.curves.get_discount_ref(self.discount_curve_id())?;
                let modified_duration = calculate_tranche_duration(
                    &cashflow_result.cashflows,
                    disc,
                    as_of,
                    pv,
                )?;
                
                // Solve Z-spread against OIS (risk-free) if available; otherwise fall back to discount curve
                let ois = context
                    .curves
                    .get_discount_ref("USD_OIS")
                    .unwrap_or(disc);
                let z_spread = calculate_tranche_z_spread(
                    &cashflow_result.cashflows,
                    ois,
                    pv,
                    as_of,
                )?;
                
                let z_spread_decimal = z_spread / 10_000.0;
                let cs01 = calculate_tranche_cs01(
                    &cashflow_result.cashflows,
                    ois,
                    z_spread_decimal,
                    as_of,
                )?;
                
                // Simple YTM calculation
                let ytm = 0.05; // Placeholder
                
                // Build metrics map
                let mut metric_values = std::collections::HashMap::new();
                for metric in metrics {
                    use $crate::metrics::MetricId;
                    match metric {
                        MetricId::WAL => metric_values.insert(MetricId::WAL, wal),
                        MetricId::ModifiedDuration => metric_values.insert(MetricId::ModifiedDuration, modified_duration),
                        MetricId::ZSpread => metric_values.insert(MetricId::ZSpread, z_spread),
                        MetricId::Cs01 => metric_values.insert(MetricId::Cs01, cs01),
                        _ => None,
                    };
                }
                
                Ok($crate::instruments::common::structured_credit::TrancheValuation {
                    tranche_id: tranche_id.to_string(),
                    pv,
                    clean_price,
                    dirty_price,
                    accrued,
                    wal,
                    modified_duration,
                    z_spread_bps: z_spread,
                    cs01,
                    ytm,
                    metrics: metric_values,
                })
            }
        }
    };
}
