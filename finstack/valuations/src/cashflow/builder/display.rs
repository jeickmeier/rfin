//! DataFrame display for cashflow schedules.
//!
//! Provides structured tabular output using Polars DataFrames for cashflow
//! schedules, including separate columns for each flow type, calculated rates,
//! and outstanding principal tracking.

use super::schedule::CashFlowSchedule;
use finstack_core::cashflow::primitives::CFKind;
use finstack_core::prelude::*;
use polars::prelude::*;
use std::collections::HashMap;

impl CashFlowSchedule {
    /// Convert cashflow schedule to a Polars DataFrame for tabular display.
    ///
    /// The DataFrame includes:
    /// - `date`: Payment dates
    /// - `fixed`: Fixed coupon cash payments (includes stub periods)
    /// - `float_reset`: Floating rate cash payments
    /// - `pik`: Payment-in-kind interest (capitalizes, not cash)
    /// - `amortization`: Principal repayments (cash)
    /// - `fee`: Fee payments (typically negative = outflow)
    /// - `total_payment`: Total cash received by investor (fixed + float + amortization)
    /// - `cash_rate_pct`: Annualized cash coupon rate (%)
    /// - `pik_rate_pct`: Annualized PIK coupon rate (%)
    /// - `float_rate_pct`: Annualized all-in floating rate (%)
    /// - `outstanding_notional`: Outstanding principal (negative = investor exposure)
    ///
    /// Notes:
    /// - Stub periods are combined with `fixed` (both are fixed-rate cash coupons)
    /// - `total_payment` = cash the investor takes home each period
    /// - `outstanding_notional` tracks principal only (unaffected by interest payments)
    ///
    /// # Errors
    /// Returns an error if DataFrame construction fails.
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_valuations::cashflow::builder::CashflowBuilder;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use time::Month;
    ///
    /// let issue = finstack_core::dates::Date::from_calendar_date(2025, Month::January, 1).unwrap();
    /// let maturity = finstack_core::dates::Date::from_calendar_date(2027, Month::January, 1).unwrap();
    ///
    /// let schedule = CashflowBuilder::new()
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .build()
    ///     .unwrap();
    ///
    /// let df = schedule.to_dataframe().unwrap();
    /// println!("{}", df);
    /// ```
    pub fn to_dataframe(&self) -> PolarsResult<DataFrame> {
        // Group flows by date for aggregation
        let mut flow_map: HashMap<Date, FlowRow> = HashMap::new();

        for cf in &self.flows {
            let row = flow_map.entry(cf.date).or_insert_with(|| FlowRow {
                date: cf.date,
                notional: 0.0,
                fixed: 0.0,
                float_reset: 0.0,
                pik: 0.0,
                amortization: 0.0,
                fee: 0.0,
                accrual_factor: cf.accrual_factor,
            });

            let amount = cf.amount.amount();
            match cf.kind {
                CFKind::Notional => row.notional += amount,
                CFKind::Fixed => row.fixed += amount,
                CFKind::FloatReset => row.float_reset += amount,
                CFKind::PIK => row.pik += amount,
                CFKind::Amortization => row.amortization += amount,
                // Combine stub with fixed for display (both are fixed-rate cash coupons)
                CFKind::Stub => row.fixed += amount,
                CFKind::Fee => row.fee += amount,
                _ => {}
            }

            // Use the first non-zero accrual factor for this date
            if cf.accrual_factor > 0.0 && row.accrual_factor == 0.0 {
                row.accrual_factor = cf.accrual_factor;
            }
        }

        // Sort by date
        let mut rows: Vec<FlowRow> = flow_map.into_values().collect();
        rows.sort_by_key(|r| r.date);

        // Calculate rates and outstanding as we iterate
        let mut dates = Vec::with_capacity(rows.len());
        let mut fixed_col = Vec::with_capacity(rows.len());
        let mut float_col = Vec::with_capacity(rows.len());
        let mut pik_col = Vec::with_capacity(rows.len());
        let mut amort_col = Vec::with_capacity(rows.len());
        let mut fee_col = Vec::with_capacity(rows.len());
        let mut total_payment_col = Vec::with_capacity(rows.len());
        let mut cash_rate_col = Vec::with_capacity(rows.len());
        let mut pik_rate_col = Vec::with_capacity(rows.len());
        let mut float_rate_col = Vec::with_capacity(rows.len());
        let mut outstanding_col = Vec::with_capacity(rows.len());

        let mut outstanding: f64 = 0.0;

        for row in rows {
            // Store date as days since epoch for Polars
            dates.push(row.date.to_julian_day());

            // Store amounts
            fixed_col.push(if row.fixed != 0.0 {
                Some(row.fixed)
            } else {
                None
            });
            float_col.push(if row.float_reset != 0.0 {
                Some(row.float_reset)
            } else {
                None
            });
            pik_col.push(if row.pik != 0.0 { Some(row.pik) } else { None });
            amort_col.push(if row.amortization != 0.0 {
                Some(row.amortization)
            } else {
                None
            });
            fee_col.push(if row.fee != 0.0 { Some(row.fee) } else { None });

            // Calculate total payment (cash the investor receives this period)
            // Includes: fixed coupons, floating coupons, amortization
            // Excludes: PIK (not cash), initial notional (outflow), fees (typically outflow)
            let total_payment = row.fixed + row.float_reset + row.amortization;
            total_payment_col.push(if total_payment != 0.0 {
                Some(total_payment)
            } else {
                None
            });

            // Calculate rates (before updating outstanding so we use pre-flow outstanding)
            // Note: stub is already included in row.fixed due to aggregation above
            let cash_interest = row.fixed + row.float_reset;
            let cash_rate = if cash_interest > 0.0 && outstanding != 0.0 && row.accrual_factor > 0.0
            {
                Some((cash_interest / outstanding.abs()) / row.accrual_factor * 100.0)
            } else {
                None
            };

            let pik_rate = if row.pik > 0.0 && outstanding != 0.0 && row.accrual_factor > 0.0 {
                Some((row.pik / outstanding.abs()) / row.accrual_factor * 100.0)
            } else {
                None
            };

            let float_rate =
                if row.float_reset > 0.0 && outstanding != 0.0 && row.accrual_factor > 0.0 {
                    Some((row.float_reset / outstanding.abs()) / row.accrual_factor * 100.0)
                } else {
                    None
                };

            cash_rate_col.push(cash_rate);
            pik_rate_col.push(pik_rate);
            float_rate_col.push(float_rate);

            // Update outstanding principal
            outstanding += row.notional; // Initial/final notional
            outstanding -= row.pik; // PIK increases outstanding (debt grows)
            outstanding += row.amortization; // Amortization reduces outstanding (debt shrinks)

            outstanding_col.push(outstanding);
        }

        // Build DataFrame
        let df = DataFrame::new(vec![
            Column::new("date".into(), dates),
            Column::new("fixed".into(), fixed_col),
            Column::new("float_reset".into(), float_col),
            Column::new("pik".into(), pik_col),
            Column::new("amortization".into(), amort_col),
            Column::new("fee".into(), fee_col),
            Column::new("total_payment".into(), total_payment_col),
            Column::new("cash_rate_pct".into(), cash_rate_col),
            Column::new("pik_rate_pct".into(), pik_rate_col),
            Column::new("float_rate_pct".into(), float_rate_col),
            Column::new("outstanding_notional".into(), outstanding_col),
        ])?;

        Ok(df)
    }
}

#[derive(Debug, Clone)]
struct FlowRow {
    date: Date,
    notional: f64,
    fixed: f64,
    float_reset: f64,
    pik: f64,
    amortization: f64,
    fee: f64,
    accrual_factor: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::types::FixedCouponSpec;
    use crate::cashflow::builder::{CashflowBuilder, ScheduleParams};
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    fn test_dataframe_basic_structure() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let schedule_params = ScheduleParams {
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        let fixed_spec = FixedCouponSpec {
            coupon_type: crate::cashflow::builder::types::CouponType::Cash,
            rate: 0.05,
            freq: schedule_params.freq,
            dc: schedule_params.dc,
            bdc: schedule_params.bdc,
            calendar_id: schedule_params.calendar_id,
            stub: schedule_params.stub,
        };

        let schedule = CashflowBuilder::new()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .fixed_cf(fixed_spec)
            .build()
            .unwrap();

        let df = schedule.to_dataframe().unwrap();

        // Verify DataFrame structure
        assert!(df.height() > 0);
        assert_eq!(df.width(), 11); // 11 columns

        // Verify column names by checking they exist
        assert!(df.column("date").is_ok());
        assert!(df.column("fixed").is_ok());
        assert!(df.column("total_payment").is_ok());
        assert!(df.column("outstanding_notional").is_ok());
        assert!(df.column("cash_rate_pct").is_ok());
    }
}
