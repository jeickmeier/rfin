//! Clearing house IM calculator.
//!
//! Stub implementation for CCP-specific margin methodologies.
//! In production, this would interface with CCP margin APIs or
//! replicate their VaR/SPAN-based calculations.

use crate::instruments::common::traits::Instrument;
use crate::margin::calculators::traits::{ImCalculator, ImResult};
use crate::margin::types::ImMethodology;
use crate::pricer::InstrumentType;
use finstack_core::collections::HashMap;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use std::sync::Arc;

/// CCP methodology type.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CcpMethodology {
    /// LCH SwapClear (VaR-based for IRS)
    LchSwapClear,
    /// LCH CDSClear
    LchCdsClear,
    /// CME Clearing (SPAN-based)
    Cme,
    /// ICE Clear Credit (for CDS/CDX)
    IceClearCredit,
    /// ICE Clear US
    IceClearUs,
    /// JSCC (Japan)
    Jscc,
    /// Eurex
    Eurex,
    /// Generic VaR-based
    GenericVaR {
        /// Confidence level (e.g., 0.99 for 99%)
        confidence: f64,
        /// Lookback period in days
        lookback_days: u32,
    },
}

impl std::fmt::Display for CcpMethodology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CcpMethodology::LchSwapClear => write!(f, "LCH SwapClear"),
            CcpMethodology::LchCdsClear => write!(f, "LCH CDSClear"),
            CcpMethodology::Cme => write!(f, "CME"),
            CcpMethodology::IceClearCredit => write!(f, "ICE Clear Credit"),
            CcpMethodology::IceClearUs => write!(f, "ICE Clear US"),
            CcpMethodology::Jscc => write!(f, "JSCC"),
            CcpMethodology::Eurex => write!(f, "Eurex"),
            CcpMethodology::GenericVaR { confidence, .. } => {
                write!(f, "Generic VaR ({:.0}%)", confidence * 100.0)
            }
        }
    }
}

impl CcpMethodology {
    /// Get the typical margin period of risk for this CCP.
    #[must_use]
    pub fn mpor_days(&self) -> u32 {
        match self {
            CcpMethodology::LchSwapClear => 5,
            CcpMethodology::LchCdsClear => 5,
            CcpMethodology::Cme => 5,
            CcpMethodology::IceClearCredit => 5,
            CcpMethodology::IceClearUs => 5,
            CcpMethodology::Jscc => 5,
            CcpMethodology::Eurex => 5,
            CcpMethodology::GenericVaR { .. } => 5,
        }
    }

    /// Get a conservative IM rate as percentage of notional.
    ///
    /// These are rough approximations for initial implementation.
    /// Real CCP margins are much more sophisticated.
    #[must_use]
    pub fn conservative_rate(&self) -> f64 {
        match self {
            CcpMethodology::LchSwapClear => 0.02,   // ~2% for IRS
            CcpMethodology::LchCdsClear => 0.08,    // ~8% for CDS
            CcpMethodology::Cme => 0.03,            // ~3% average
            CcpMethodology::IceClearCredit => 0.10, // ~10% for CDX
            CcpMethodology::IceClearUs => 0.05,     // ~5% average
            CcpMethodology::Jscc => 0.03,
            CcpMethodology::Eurex => 0.03,
            CcpMethodology::GenericVaR { .. } => 0.05,
        }
    }

    /// Choose a CCP methodology based on a CCP name and instrument type.
    #[must_use]
    pub fn from_ccp_name(ccp: &str, instrument_type: InstrumentType) -> Self {
        let normalized = ccp.trim().to_ascii_lowercase();
        let is_credit = is_credit_instrument(instrument_type);

        if normalized.contains("lch") {
            if is_credit || normalized.contains("cds") {
                CcpMethodology::LchCdsClear
            } else {
                CcpMethodology::LchSwapClear
            }
        } else if normalized.contains("ice") {
            if is_credit || normalized.contains("credit") || normalized.contains("cds") {
                CcpMethodology::IceClearCredit
            } else {
                CcpMethodology::IceClearUs
            }
        } else if normalized.contains("cme") {
            CcpMethodology::Cme
        } else if normalized.contains("jscc") {
            CcpMethodology::Jscc
        } else if normalized.contains("eurex") {
            CcpMethodology::Eurex
        } else {
            CcpMethodology::GenericVaR {
                confidence: 0.99,
                lookback_days: 250,
            }
        }
    }
}

/// CCP margin input source for external VaR/SPAN results.
pub trait CcpMarginInputSource: Send + Sync {
    /// Return a CCP-supplied IM amount when available.
    fn initial_margin(
        &self,
        instrument: &dyn Instrument,
        context: &MarketContext,
        as_of: Date,
        methodology: &CcpMethodology,
    ) -> Option<Money>;

    /// Optional MPOR override supplied by the CCP.
    fn mpor_days(&self, _methodology: &CcpMethodology) -> Option<u32> {
        None
    }
}

/// Clearing house IM calculator.
///
/// Provides IM calculation for cleared derivatives using CCP-specific methodologies.
/// This is a simplified implementation that uses conservative estimates.
///
/// # Real-World Implementation
///
/// In production, this would:
/// 1. Interface with CCP margin APIs (e.g., LCH SMART, CME CORE)
/// 2. Replicate VaR/SPAN calculations with historical scenarios
/// 3. Apply portfolio margining and cross-product netting
///
/// # Example
///
/// ```rust,no_run
/// use finstack_valuations::instruments::Instrument;
/// use finstack_valuations::margin::{ClearingHouseImCalculator, ImCalculator};
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
/// use time::macros::date;
///
/// # fn main() -> finstack_core::Result<()> {
/// let calc = ClearingHouseImCalculator::lch_swapclear();
/// # let cleared_swap: &dyn Instrument = todo!("provide a cleared swap instrument");
/// # let context = MarketContext::new();
/// # let as_of: Date = date!(2025-01-01);
/// let im = calc.calculate(cleared_swap, &context, as_of)?;
/// # let _ = im;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ClearingHouseImCalculator {
    /// CCP methodology
    pub methodology: CcpMethodology,
    /// Optional external CCP margin input source
    pub input_source: Option<Arc<dyn CcpMarginInputSource>>,
}

impl std::fmt::Debug for ClearingHouseImCalculator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClearingHouseImCalculator")
            .field("methodology", &self.methodology)
            .field(
                "input_source",
                &self
                    .input_source
                    .as_ref()
                    .map(|_| "<dyn CcpMarginInputSource>"),
            )
            .finish()
    }
}

impl ClearingHouseImCalculator {
    /// Create a new calculator for a specific CCP.
    #[must_use]
    pub fn new(methodology: CcpMethodology) -> Self {
        Self {
            methodology,
            input_source: None,
        }
    }

    /// Create a calculator from a CCP name and instrument type.
    #[must_use]
    pub fn for_ccp(ccp: &str, instrument_type: InstrumentType) -> Self {
        Self::new(CcpMethodology::from_ccp_name(ccp, instrument_type))
    }

    /// Create calculator for LCH SwapClear (IRS).
    #[must_use]
    pub fn lch_swapclear() -> Self {
        Self::new(CcpMethodology::LchSwapClear)
    }

    /// Create calculator for ICE Clear Credit (CDS/CDX).
    #[must_use]
    pub fn ice_clear_credit() -> Self {
        Self::new(CcpMethodology::IceClearCredit)
    }

    /// Create calculator for CME.
    #[must_use]
    pub fn cme() -> Self {
        Self::new(CcpMethodology::Cme)
    }

    /// Create a generic VaR-based calculator.
    #[must_use]
    pub fn generic_var(confidence: f64, lookback_days: u32) -> Self {
        Self::new(CcpMethodology::GenericVaR {
            confidence,
            lookback_days,
        })
    }

    /// Attach a CCP input source (VaR/SPAN outputs).
    #[must_use]
    pub fn with_input_source(mut self, source: Arc<dyn CcpMarginInputSource>) -> Self {
        self.input_source = Some(source);
        self
    }

    /// Calculate IM using conservative estimate.
    ///
    /// This is a simplified calculation. Real CCP margins use VaR/ES
    /// with historical scenarios.
    pub fn calculate_conservative(&self, notional: Money) -> Money {
        Money::new(notional.amount().abs(), notional.currency())
            * self.methodology.conservative_rate()
    }
}

impl ImCalculator for ClearingHouseImCalculator {
    fn calculate(
        &self,
        instrument: &dyn Instrument,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        // Use PV as proxy for notional
        let pv = instrument.value(context, as_of)?;
        let currency = pv.currency();
        let notional = Money::new(pv.amount().abs(), currency);

        let mut im_amount = self.calculate_conservative(notional);
        let mut mpor_days = self.methodology.mpor_days();
        if let Some(source) = &self.input_source {
            if let Some(amount) =
                source.initial_margin(instrument, context, as_of, &self.methodology)
            {
                im_amount = amount;
            }
            if let Some(override_mpor) = source.mpor_days(&self.methodology) {
                mpor_days = override_mpor;
            }
        }

        let mut breakdown = HashMap::default();
        breakdown.insert(self.methodology.to_string(), im_amount);

        Ok(ImResult::with_breakdown(
            im_amount,
            ImMethodology::ClearingHouse,
            as_of,
            mpor_days,
            breakdown,
        ))
    }

    fn methodology(&self) -> ImMethodology {
        ImMethodology::ClearingHouse
    }
}

fn is_credit_instrument(instrument_type: InstrumentType) -> bool {
    matches!(
        instrument_type,
        InstrumentType::CDS
            | InstrumentType::CDSIndex
            | InstrumentType::CDSTranche
            | InstrumentType::CDSOption
            | InstrumentType::StructuredCredit
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use std::sync::Arc;

    #[derive(Clone)]
    struct TestInstrument {
        id: String,
        value: Money,
        attributes: crate::instruments::common::traits::Attributes,
    }

    impl TestInstrument {
        fn new(value: Money) -> Self {
            Self {
                id: "TEST-INST".to_string(),
                value,
                attributes: crate::instruments::common::traits::Attributes::default(),
            }
        }
    }

    impl Instrument for TestInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn key(&self) -> InstrumentType {
            InstrumentType::IRS
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
            &self.attributes
        }

        fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
            &mut self.attributes
        }

        fn clone_box(&self) -> Box<dyn Instrument> {
            Box::new(self.clone())
        }

        fn value(&self, _market: &MarketContext, _as_of: Date) -> Result<Money> {
            Ok(self.value)
        }

        fn price_with_metrics(
            &self,
            _market: &MarketContext,
            as_of: Date,
            _metrics: &[crate::metrics::MetricId],
        ) -> Result<crate::results::ValuationResult> {
            Ok(crate::results::ValuationResult::stamped(
                &self.id, as_of, self.value,
            ))
        }
    }

    #[test]
    fn ccp_methodology_display() {
        assert_eq!(CcpMethodology::LchSwapClear.to_string(), "LCH SwapClear");
        assert_eq!(
            CcpMethodology::IceClearCredit.to_string(),
            "ICE Clear Credit"
        );
    }

    #[test]
    fn conservative_rates() {
        assert_eq!(CcpMethodology::LchSwapClear.conservative_rate(), 0.02);
        assert_eq!(CcpMethodology::IceClearCredit.conservative_rate(), 0.10);
    }

    #[test]
    fn mpor_days() {
        assert_eq!(CcpMethodology::LchSwapClear.mpor_days(), 5);
        assert_eq!(CcpMethodology::Cme.mpor_days(), 5);
    }

    #[test]
    fn conservative_calculation() {
        let calc = ClearingHouseImCalculator::lch_swapclear();
        let notional = Money::new(100_000_000.0, Currency::USD);
        let im = calc.calculate_conservative(notional);

        // LCH SwapClear ~2%
        assert_eq!(im.amount(), 2_000_000.0);
    }

    #[test]
    fn ice_clear_credit_calculation() {
        let calc = ClearingHouseImCalculator::ice_clear_credit();
        let notional = Money::new(50_000_000.0, Currency::USD);
        let im = calc.calculate_conservative(notional);

        // ICE Clear Credit ~10%
        assert_eq!(im.amount(), 5_000_000.0);
    }

    #[test]
    fn ccp_name_mapping() {
        assert_eq!(
            CcpMethodology::from_ccp_name("LCH", InstrumentType::IRS),
            CcpMethodology::LchSwapClear
        );
        assert_eq!(
            CcpMethodology::from_ccp_name("LCH CDSClear", InstrumentType::CDS),
            CcpMethodology::LchCdsClear
        );
        assert_eq!(
            CcpMethodology::from_ccp_name("ICE", InstrumentType::CDSIndex),
            CcpMethodology::IceClearCredit
        );
    }

    #[derive(Debug)]
    struct TestInputSource {
        amount: Money,
        mpor_days: u32,
    }

    impl CcpMarginInputSource for TestInputSource {
        fn initial_margin(
            &self,
            _instrument: &dyn Instrument,
            _context: &MarketContext,
            _as_of: Date,
            _methodology: &CcpMethodology,
        ) -> Option<Money> {
            Some(self.amount)
        }

        fn mpor_days(&self, _methodology: &CcpMethodology) -> Option<u32> {
            Some(self.mpor_days)
        }
    }

    #[test]
    fn uses_ccp_input_source_when_available() {
        let calc = ClearingHouseImCalculator::lch_swapclear().with_input_source(Arc::new(
            TestInputSource {
                amount: Money::new(3_000_000.0, Currency::USD),
                mpor_days: 7,
            },
        ));
        let notional = Money::new(100_000_000.0, Currency::USD);
        let fallback = calc.calculate_conservative(notional);

        let fake_inst = TestInstrument::new(notional);
        let market = MarketContext::new();
        let as_of = Date::from_calendar_date(2024, time::Month::January, 1).expect("valid date");
        let im = calc.calculate(&fake_inst, &market, as_of).expect("im");

        assert_eq!(fallback.amount(), 2_000_000.0);
        assert_eq!(im.amount.amount(), 3_000_000.0);
        assert_eq!(im.mpor_days, 7);
    }
}
