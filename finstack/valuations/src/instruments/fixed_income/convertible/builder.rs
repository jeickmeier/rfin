use super::types::{
    ConversionSpec, ConvertibleBond,
};
use crate::cashflow::builder::types::{FixedCouponSpec, FloatingCouponSpec};
use crate::instruments::fixed_income::bond::CallPutSchedule;
use finstack_core::dates::Date;
use finstack_core::money::Money;

impl_builder!(
    ConvertibleBond,
    ConvertibleBondBuilder,
    required: [
        id: String,
        notional: Money,
        issue: Date,
        maturity: Date,
        disc_id: &'static str,
        conversion: ConversionSpec
    ],
    optional: [
        underlying_equity_id: String,
        call_put: CallPutSchedule,
        fixed_coupon: FixedCouponSpec,
        floating_coupon: FloatingCouponSpec
    ]
);


