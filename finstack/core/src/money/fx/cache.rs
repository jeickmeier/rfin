use crate::currency::Currency;
use crate::dates::Date;

use super::types::FxConversionPolicy;

/// Pair key helper used internally for maps
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Pair(pub(crate) Currency, pub(crate) Currency);

/// Query-sensitive cache key for provider-observed FX rates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct QueryKey {
    pub(crate) from: Currency,
    pub(crate) to: Currency,
    pub(crate) on: Date,
    pub(crate) policy: FxConversionPolicy,
}
