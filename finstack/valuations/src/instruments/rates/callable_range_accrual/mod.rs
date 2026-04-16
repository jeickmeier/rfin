//! Callable Range Accrual - range accrual with Bermudan call provision.
//!
//! Extends the range accrual structure with an issuer call right,
//! creating a more complex pricing problem that requires backward
//! induction (LSMC or trinomial tree with HW1F).
//!
//! # See Also
//!
//! - [`CallableRangeAccrual`] for instrument definition
//! - [`RangeAccrualSpec`] for the underlying range accrual parameters
//! - [`crate::instruments::rates::shared::bermudan_call::BermudanCallProvision`]

pub(crate) mod metrics;
pub(crate) mod types;

pub use types::{CallableRangeAccrual, RangeAccrualSpec};
