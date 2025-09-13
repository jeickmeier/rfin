use super::types::Deposit;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::F;

impl_builder!(
    Deposit,
    DepositBuilder,
    required: [
        id: finstack_core::types::InstrumentId,
        notional: Money,
        start: Date,
        end: Date,
        day_count: DayCount,
        disc_id: finstack_core::types::CurveId
    ],
    optional: [
        quote_rate: F
    ]
);
