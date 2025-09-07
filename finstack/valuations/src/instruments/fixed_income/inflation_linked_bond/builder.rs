use super::types::{
    DeflationProtection, IndexationMethod,
};
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::inflation_index::InflationLag;
use finstack_core::money::Money;
use finstack_core::F;

// Recreate the ILBBuilder in this module and keep the public API unchanged.

#[derive(Default)]
pub struct ILBBuilder {
    id: Option<String>,
    notional: Option<Money>,
    real_coupon: Option<F>,
    freq: Option<Frequency>,
    dc: Option<DayCount>,
    issue: Option<Date>,
    maturity: Option<Date>,
    base_index: Option<F>,
    base_date: Option<Date>,
    indexation_method: Option<IndexationMethod>,
    lag: Option<InflationLag>,
    deflation_protection: Option<DeflationProtection>,
    bdc: Option<BusinessDayConvention>,
    stub: Option<StubKind>,
    calendar_id: Option<&'static str>,
    disc_id: Option<&'static str>,
    inflation_id: Option<&'static str>,
    quoted_clean: Option<F>,
}

impl ILBBuilder {
    pub fn new() -> Self { Self::default() }

    pub fn id(mut self, value: impl Into<String>) -> Self { self.id = Some(value.into()); self }
    pub fn notional(mut self, value: Money) -> Self { self.notional = Some(value); self }
    pub fn real_coupon(mut self, value: F) -> Self { self.real_coupon = Some(value); self }
    pub fn freq(mut self, value: Frequency) -> Self { self.freq = Some(value); self }
    pub fn dc(mut self, value: DayCount) -> Self { self.dc = Some(value); self }
    pub fn issue(mut self, value: Date) -> Self { self.issue = Some(value); self }
    pub fn maturity(mut self, value: Date) -> Self { self.maturity = Some(value); self }
    pub fn base_index(mut self, value: F) -> Self { self.base_index = Some(value); self }
    pub fn base_date(mut self, value: Date) -> Self { self.base_date = Some(value); self }
    pub fn indexation_method(mut self, value: IndexationMethod) -> Self { self.indexation_method = Some(value); self }
    pub fn lag(mut self, value: InflationLag) -> Self { self.lag = Some(value); self }
    pub fn deflation_protection(mut self, value: DeflationProtection) -> Self { self.deflation_protection = Some(value); self }
    pub fn bdc(mut self, value: BusinessDayConvention) -> Self { self.bdc = Some(value); self }
    pub fn stub(mut self, value: StubKind) -> Self { self.stub = Some(value); self }
    pub fn calendar_id(mut self, value: &'static str) -> Self { self.calendar_id = Some(value); self }
    pub fn disc_id(mut self, value: &'static str) -> Self { self.disc_id = Some(value); self }
    pub fn inflation_id(mut self, value: &'static str) -> Self { self.inflation_id = Some(value); self }
    pub fn quoted_clean(mut self, value: F) -> Self { self.quoted_clean = Some(value); self }

    pub fn build(self) -> finstack_core::Result<super::types::InflationLinkedBond> {
        let issue = self.issue.ok_or_else(|| finstack_core::Error::from(
            finstack_core::error::InputError::Invalid))?;
        Ok(super::types::InflationLinkedBond {
            id: self.id.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            notional: self.notional.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            real_coupon: self.real_coupon.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            freq: self.freq.unwrap_or_else(Frequency::semi_annual),
            dc: self.dc.unwrap_or(DayCount::ActAct),
            issue,
            maturity: self.maturity.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            base_index: self.base_index.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            base_date: self.base_date.unwrap_or(issue),
            indexation_method: self.indexation_method.unwrap_or(IndexationMethod::TIPS),
            lag: self.lag.unwrap_or(InflationLag::Months(3)),
            deflation_protection: self.deflation_protection.unwrap_or(DeflationProtection::MaturityOnly),
            bdc: self.bdc.unwrap_or(BusinessDayConvention::Following),
            stub: self.stub.unwrap_or(StubKind::None),
            calendar_id: self.calendar_id,
            disc_id: self.disc_id.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            inflation_id: self.inflation_id.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            quoted_clean: self.quoted_clean,
            attributes: crate::instruments::traits::Attributes::new(),
        })
    }
}


