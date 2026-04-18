//! Margin calculation result types.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::HashMap;
use finstack_margin::{ImMethodology, NettingSetId, SimmRiskClass, SimmSensitivities};
use std::fmt;

use crate::types::PositionId;

/// Error returned when attempting to aggregate margin results with mismatched currencies.
///
/// This error occurs when [`PortfolioMarginResult::add_netting_set`] is called with a
/// netting set margin in a currency different from the portfolio's base currency.
/// Use [`PortfolioMarginResult::add_netting_set_with_fx`] to handle cross-currency
/// aggregation with explicit FX conversion.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CurrencyMismatchError {
    /// The netting set that caused the error.
    pub netting_set_id: NettingSetId,
    /// Currency of the netting set margin.
    pub netting_set_currency: Currency,
    /// Expected base currency of the portfolio margin result.
    pub base_currency: Currency,
}

impl fmt::Display for CurrencyMismatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Currency mismatch for netting set {:?}: expected {} but got {}. \
             Use add_netting_set_with_fx() for cross-currency aggregation.",
            self.netting_set_id, self.base_currency, self.netting_set_currency
        )
    }
}

impl std::error::Error for CurrencyMismatchError {}

/// Margin results for a single netting set.
#[derive(Debug, Clone)]
pub struct NettingSetMargin {
    /// Netting set identifier
    pub netting_set_id: NettingSetId,
    /// Calculation date
    pub as_of: Date,
    /// Initial margin requirement
    pub initial_margin: Money,
    /// Variation margin requirement
    pub variation_margin: Money,
    /// Total margin (IM + positive VM)
    pub total_margin: Money,
    /// Number of positions in the netting set
    pub position_count: usize,
    /// IM methodology used
    pub im_methodology: ImMethodology,
    /// Aggregated sensitivities (for SIMM breakdown)
    pub sensitivities: Option<SimmSensitivities>,
    /// Breakdown by risk class (for SIMM)
    pub im_breakdown: HashMap<String, Money>,
}

impl NettingSetMargin {
    /// Create a new netting set margin result.
    ///
    /// # Returns
    ///
    /// Netting-set margin result with total margin computed as
    /// `initial_margin + max(variation_margin, 0)`.
    #[must_use]
    pub fn new(
        netting_set_id: NettingSetId,
        as_of: Date,
        initial_margin: Money,
        variation_margin: Money,
        position_count: usize,
        im_methodology: ImMethodology,
    ) -> Self {
        let currency = initial_margin.currency();
        let total = Money::new(
            initial_margin.amount() + variation_margin.amount().max(0.0),
            currency,
        );

        Self {
            netting_set_id,
            as_of,
            initial_margin,
            variation_margin,
            total_margin: total,
            position_count,
            im_methodology,
            sensitivities: None,
            im_breakdown: HashMap::default(),
        }
    }

    /// Add SIMM breakdown information.
    ///
    /// # Returns
    ///
    /// The updated result for fluent chaining.
    pub fn with_simm_breakdown(
        mut self,
        sensitivities: SimmSensitivities,
        breakdown: HashMap<String, Money>,
    ) -> Self {
        self.sensitivities = Some(sensitivities);
        self.im_breakdown = breakdown;
        self
    }

    /// Check if this is a cleared netting set.
    ///
    /// # Returns
    ///
    /// `true` when the netting set identifier represents a cleared venue.
    #[must_use]
    pub fn is_cleared(&self) -> bool {
        self.netting_set_id.is_cleared()
    }
}

/// Portfolio-wide margin calculation results.
#[derive(Debug, Clone)]
pub struct PortfolioMarginResult {
    /// Calculation date
    pub as_of: Date,
    /// Base currency for aggregated figures
    pub base_currency: Currency,
    /// Total initial margin across all netting sets
    pub total_initial_margin: Money,
    /// Total variation margin across all netting sets
    pub total_variation_margin: Money,
    /// Total margin requirement
    pub total_margin: Money,
    /// Results by netting set
    pub by_netting_set: HashMap<NettingSetId, NettingSetMargin>,
    /// Number of positions included in margin calculation
    pub total_positions: usize,
    /// Number of positions without margin specs (excluded)
    pub positions_without_margin: usize,
    /// Positions whose sensitivity or VM valuation failed during aggregation.
    pub degraded_positions: Vec<(PositionId, String)>,
}

impl PortfolioMarginResult {
    /// Create a new portfolio margin result.
    ///
    /// # Returns
    ///
    /// Empty portfolio margin report initialized in the supplied base currency.
    #[must_use]
    pub fn new(as_of: Date, base_currency: Currency) -> Self {
        Self {
            as_of,
            base_currency,
            total_initial_margin: Money::new(0.0, base_currency),
            total_variation_margin: Money::new(0.0, base_currency),
            total_margin: Money::new(0.0, base_currency),
            by_netting_set: HashMap::default(),
            total_positions: 0,
            positions_without_margin: 0,
            degraded_positions: Vec::new(),
        }
    }

    /// Add a netting set margin result.
    ///
    /// # Errors
    ///
    /// Returns an error if the netting set margin currency differs from the
    /// portfolio's base currency. Cross-currency margin aggregation requires
    /// explicit FX conversion via [`Self::add_netting_set_with_fx`].
    pub fn add_netting_set(
        &mut self,
        result: NettingSetMargin,
    ) -> Result<(), CurrencyMismatchError> {
        // Validate currency matches base currency
        let ns_currency = result.initial_margin.currency();
        if ns_currency != self.base_currency {
            return Err(CurrencyMismatchError {
                netting_set_id: result.netting_set_id.clone(),
                netting_set_currency: ns_currency,
                base_currency: self.base_currency,
            });
        }

        let im = result.initial_margin.amount();
        let vm = result.variation_margin.amount();

        self.total_initial_margin =
            Money::new(self.total_initial_margin.amount() + im, self.base_currency);
        self.total_variation_margin = Money::new(
            self.total_variation_margin.amount() + vm,
            self.base_currency,
        );
        self.total_margin = Money::new(
            self.total_margin.amount() + result.total_margin.amount(),
            self.base_currency,
        );
        self.total_positions += result.position_count;
        self.by_netting_set
            .insert(result.netting_set_id.clone(), result);

        Ok(())
    }

    /// Add a netting set margin result with explicit FX conversion.
    ///
    /// Use this method when the netting set margin is in a different currency
    /// than the portfolio's base currency. The provided FX rate converts from
    /// the netting set currency to the base currency.
    ///
    /// # Arguments
    ///
    /// * `result` - The netting set margin to add
    /// * `fx_rate` - FX rate to convert from netting set currency to base currency
    ///   (e.g., if netting set is EUR and base is USD, rate is EUR/USD)
    pub fn add_netting_set_with_fx(&mut self, result: NettingSetMargin, fx_rate: f64) {
        if !fx_rate.is_finite() || fx_rate <= 0.0 {
            tracing::error!(
                netting_set_id = ?result.netting_set_id,
                fx_rate,
                "Invalid FX rate for margin aggregation; must be positive and finite"
            );
            return;
        }

        let im = result.initial_margin.amount() * fx_rate;
        let vm = result.variation_margin.amount() * fx_rate;
        let total = result.total_margin.amount() * fx_rate;

        self.total_initial_margin =
            Money::new(self.total_initial_margin.amount() + im, self.base_currency);
        self.total_variation_margin = Money::new(
            self.total_variation_margin.amount() + vm,
            self.base_currency,
        );
        self.total_margin = Money::new(self.total_margin.amount() + total, self.base_currency);
        self.total_positions += result.position_count;
        self.by_netting_set
            .insert(result.netting_set_id.clone(), result);
    }

    /// Get the number of netting sets.
    ///
    /// # Returns
    ///
    /// Number of tracked netting sets.
    #[must_use]
    pub fn netting_set_count(&self) -> usize {
        self.by_netting_set.len()
    }

    /// Get results for cleared vs bilateral netting sets.
    ///
    /// # Returns
    ///
    /// Tuple of `(cleared_total, bilateral_total)` in the portfolio base currency.
    #[must_use]
    pub fn cleared_bilateral_split(&self) -> (Money, Money) {
        let mut cleared = 0.0;
        let mut bilateral = 0.0;

        for result in self.by_netting_set.values() {
            if result.is_cleared() {
                cleared += result.total_margin.amount();
            } else {
                bilateral += result.total_margin.amount();
            }
        }

        (
            Money::new(cleared, self.base_currency),
            Money::new(bilateral, self.base_currency),
        )
    }

    /// Iterate over netting set results.
    ///
    /// # Returns
    ///
    /// Iterator over netting-set identifiers and their results.
    pub fn iter(&self) -> impl Iterator<Item = (&NettingSetId, &NettingSetMargin)> {
        self.by_netting_set.iter()
    }

    /// Record a degraded position with the corresponding error message.
    ///
    /// # Arguments
    ///
    /// * `position_id` - Position whose margin calculation degraded.
    /// * `message` - Human-readable reason for the degradation.
    pub fn add_degraded_position(&mut self, position_id: PositionId, message: impl Into<String>) {
        self.degraded_positions.push((position_id, message.into()));
    }
}

// ── JSON wire format types ─────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
struct CurrencyTenorEntry {
    currency: Currency,
    tenor_bucket: String,
    value: f64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LabelTenorEntry {
    label: String,
    tenor_bucket: String,
    value: f64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LabelEntry {
    name: String,
    value: f64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CurrencyEntry {
    currency: Currency,
    value: f64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CurrencyPairEntry {
    base: Currency,
    quote: Currency,
    value: f64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CurvatureEntry {
    risk_class: SimmRiskClass,
    value: f64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SimmSensitivitiesWire {
    base_currency: Currency,
    ir_delta: Vec<CurrencyTenorEntry>,
    ir_vega: Vec<CurrencyTenorEntry>,
    credit_qualifying_delta: Vec<LabelTenorEntry>,
    credit_non_qualifying_delta: Vec<LabelTenorEntry>,
    equity_delta: Vec<LabelEntry>,
    equity_vega: Vec<LabelEntry>,
    fx_delta: Vec<CurrencyEntry>,
    fx_vega: Vec<CurrencyPairEntry>,
    commodity_delta: Vec<LabelEntry>,
    curvature: Vec<CurvatureEntry>,
}

impl From<&SimmSensitivities> for SimmSensitivitiesWire {
    fn from(s: &SimmSensitivities) -> Self {
        let mut ir_delta: Vec<CurrencyTenorEntry> = s
            .ir_delta
            .iter()
            .map(|((ccy, tenor), &v)| CurrencyTenorEntry {
                currency: *ccy,
                tenor_bucket: tenor.clone(),
                value: v,
            })
            .collect();
        ir_delta.sort_by(|a, b| {
            a.currency
                .cmp(&b.currency)
                .then_with(|| a.tenor_bucket.cmp(&b.tenor_bucket))
        });

        let mut ir_vega: Vec<CurrencyTenorEntry> = s
            .ir_vega
            .iter()
            .map(|((ccy, tenor), &v)| CurrencyTenorEntry {
                currency: *ccy,
                tenor_bucket: tenor.clone(),
                value: v,
            })
            .collect();
        ir_vega.sort_by(|a, b| {
            a.currency
                .cmp(&b.currency)
                .then_with(|| a.tenor_bucket.cmp(&b.tenor_bucket))
        });

        let mut credit_qualifying_delta: Vec<LabelTenorEntry> = s
            .credit_qualifying_delta
            .iter()
            .map(|((label, tenor), &v)| LabelTenorEntry {
                label: label.clone(),
                tenor_bucket: tenor.clone(),
                value: v,
            })
            .collect();
        credit_qualifying_delta.sort_by(|a, b| {
            a.label
                .cmp(&b.label)
                .then_with(|| a.tenor_bucket.cmp(&b.tenor_bucket))
        });

        let mut credit_non_qualifying_delta: Vec<LabelTenorEntry> = s
            .credit_non_qualifying_delta
            .iter()
            .map(|((label, tenor), &v)| LabelTenorEntry {
                label: label.clone(),
                tenor_bucket: tenor.clone(),
                value: v,
            })
            .collect();
        credit_non_qualifying_delta.sort_by(|a, b| {
            a.label
                .cmp(&b.label)
                .then_with(|| a.tenor_bucket.cmp(&b.tenor_bucket))
        });

        let mut equity_delta: Vec<LabelEntry> = s
            .equity_delta
            .iter()
            .map(|(name, &v)| LabelEntry {
                name: name.clone(),
                value: v,
            })
            .collect();
        equity_delta.sort_by(|a, b| a.name.cmp(&b.name));

        let mut equity_vega: Vec<LabelEntry> = s
            .equity_vega
            .iter()
            .map(|(name, &v)| LabelEntry {
                name: name.clone(),
                value: v,
            })
            .collect();
        equity_vega.sort_by(|a, b| a.name.cmp(&b.name));

        let mut fx_delta: Vec<CurrencyEntry> = s
            .fx_delta
            .iter()
            .map(|(ccy, &v)| CurrencyEntry {
                currency: *ccy,
                value: v,
            })
            .collect();
        fx_delta.sort_by(|a, b| a.currency.cmp(&b.currency));

        let mut fx_vega: Vec<CurrencyPairEntry> = s
            .fx_vega
            .iter()
            .map(|((base, quote), &v)| CurrencyPairEntry {
                base: *base,
                quote: *quote,
                value: v,
            })
            .collect();
        fx_vega.sort_by(|a, b| a.base.cmp(&b.base).then_with(|| a.quote.cmp(&b.quote)));

        let mut commodity_delta: Vec<LabelEntry> = s
            .commodity_delta
            .iter()
            .map(|(name, &v)| LabelEntry {
                name: name.clone(),
                value: v,
            })
            .collect();
        commodity_delta.sort_by(|a, b| a.name.cmp(&b.name));

        let mut curvature: Vec<CurvatureEntry> = s
            .curvature
            .iter()
            .map(|(rc, &v)| CurvatureEntry {
                risk_class: *rc,
                value: v,
            })
            .collect();
        curvature.sort_by_key(|e| format!("{:?}", e.risk_class));

        Self {
            base_currency: s.base_currency,
            ir_delta,
            ir_vega,
            credit_qualifying_delta,
            credit_non_qualifying_delta,
            equity_delta,
            equity_vega,
            fx_delta,
            fx_vega,
            commodity_delta,
            curvature,
        }
    }
}

impl From<SimmSensitivitiesWire> for SimmSensitivities {
    fn from(w: SimmSensitivitiesWire) -> Self {
        let mut s = SimmSensitivities::new(w.base_currency);
        for e in w.ir_delta {
            s.ir_delta.insert((e.currency, e.tenor_bucket), e.value);
        }
        for e in w.ir_vega {
            s.ir_vega.insert((e.currency, e.tenor_bucket), e.value);
        }
        for e in w.credit_qualifying_delta {
            s.credit_qualifying_delta
                .insert((e.label, e.tenor_bucket), e.value);
        }
        for e in w.credit_non_qualifying_delta {
            s.credit_non_qualifying_delta
                .insert((e.label, e.tenor_bucket), e.value);
        }
        for e in w.equity_delta {
            s.equity_delta.insert(e.name, e.value);
        }
        for e in w.equity_vega {
            s.equity_vega.insert(e.name, e.value);
        }
        for e in w.fx_delta {
            s.fx_delta.insert(e.currency, e.value);
        }
        for e in w.fx_vega {
            s.fx_vega.insert((e.base, e.quote), e.value);
        }
        for e in w.commodity_delta {
            s.commodity_delta.insert(e.name, e.value);
        }
        for e in w.curvature {
            s.curvature.insert(e.risk_class, e.value);
        }
        s
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NettingSetMarginWire {
    netting_set_id: NettingSetId,
    as_of: Date,
    initial_margin: Money,
    variation_margin: Money,
    total_margin: Money,
    position_count: usize,
    im_methodology: ImMethodology,
    sensitivities: Option<SimmSensitivitiesWire>,
    im_breakdown: HashMap<String, Money>,
}

impl From<&NettingSetMargin> for NettingSetMarginWire {
    fn from(m: &NettingSetMargin) -> Self {
        Self {
            netting_set_id: m.netting_set_id.clone(),
            as_of: m.as_of,
            initial_margin: m.initial_margin,
            variation_margin: m.variation_margin,
            total_margin: m.total_margin,
            position_count: m.position_count,
            im_methodology: m.im_methodology,
            sensitivities: m.sensitivities.as_ref().map(SimmSensitivitiesWire::from),
            im_breakdown: m.im_breakdown.clone(),
        }
    }
}

impl From<NettingSetMarginWire> for NettingSetMargin {
    fn from(w: NettingSetMarginWire) -> Self {
        Self {
            netting_set_id: w.netting_set_id,
            as_of: w.as_of,
            initial_margin: w.initial_margin,
            variation_margin: w.variation_margin,
            total_margin: w.total_margin,
            position_count: w.position_count,
            im_methodology: w.im_methodology,
            sensitivities: w.sensitivities.map(SimmSensitivities::from),
            im_breakdown: w.im_breakdown,
        }
    }
}

impl serde::Serialize for NettingSetMargin {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        NettingSetMarginWire::from(self).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for NettingSetMargin {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(NettingSetMarginWire::deserialize(deserializer)?.into())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct DegradedPositionWire {
    position_id: String,
    message: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PortfolioMarginResultWire {
    as_of: Date,
    base_currency: Currency,
    total_initial_margin: Money,
    total_variation_margin: Money,
    total_margin: Money,
    netting_sets: Vec<NettingSetMarginWire>,
    total_positions: usize,
    positions_without_margin: usize,
    degraded_positions: Vec<DegradedPositionWire>,
}

impl From<&PortfolioMarginResult> for PortfolioMarginResultWire {
    fn from(r: &PortfolioMarginResult) -> Self {
        let mut netting_sets: Vec<NettingSetMarginWire> = r
            .by_netting_set
            .values()
            .map(NettingSetMarginWire::from)
            .collect();
        netting_sets.sort_by(|a, b| {
            a.netting_set_id
                .to_string()
                .cmp(&b.netting_set_id.to_string())
        });
        let degraded_positions = r
            .degraded_positions
            .iter()
            .map(|(id, msg)| DegradedPositionWire {
                position_id: id.to_string(),
                message: msg.clone(),
            })
            .collect();
        Self {
            as_of: r.as_of,
            base_currency: r.base_currency,
            total_initial_margin: r.total_initial_margin,
            total_variation_margin: r.total_variation_margin,
            total_margin: r.total_margin,
            netting_sets,
            total_positions: r.total_positions,
            positions_without_margin: r.positions_without_margin,
            degraded_positions,
        }
    }
}

impl From<PortfolioMarginResultWire> for PortfolioMarginResult {
    fn from(w: PortfolioMarginResultWire) -> Self {
        let degraded_positions = w
            .degraded_positions
            .into_iter()
            .map(|d| (PositionId::new(d.position_id), d.message))
            .collect();
        let by_netting_set: HashMap<NettingSetId, NettingSetMargin> = w
            .netting_sets
            .into_iter()
            .map(|wire| {
                let ns = NettingSetMargin::from(wire);
                (ns.netting_set_id.clone(), ns)
            })
            .collect();
        Self {
            as_of: w.as_of,
            base_currency: w.base_currency,
            total_initial_margin: w.total_initial_margin,
            total_variation_margin: w.total_variation_margin,
            total_margin: w.total_margin,
            by_netting_set,
            total_positions: w.total_positions,
            positions_without_margin: w.positions_without_margin,
            degraded_positions,
        }
    }
}

impl serde::Serialize for PortfolioMarginResult {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PortfolioMarginResultWire::from(self).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for PortfolioMarginResult {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(PortfolioMarginResultWire::deserialize(deserializer)?.into())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::June, 15).expect("valid date")
    }

    #[test]
    fn test_netting_set_margin_creation() {
        let id = NettingSetId::bilateral("BANK_A", "CSA_001");
        let result = NettingSetMargin::new(
            id,
            test_date(),
            Money::new(5_000_000.0, Currency::USD),
            Money::new(1_000_000.0, Currency::USD),
            10,
            ImMethodology::Simm,
        );

        assert_eq!(result.initial_margin.amount(), 5_000_000.0);
        assert_eq!(result.variation_margin.amount(), 1_000_000.0);
        assert_eq!(result.total_margin.amount(), 6_000_000.0);
        assert!(!result.is_cleared());
    }

    #[test]
    fn test_portfolio_margin_aggregation() {
        let mut portfolio_result = PortfolioMarginResult::new(test_date(), Currency::USD);

        // Add bilateral netting set
        let bilateral = NettingSetMargin::new(
            NettingSetId::bilateral("BANK_A", "CSA_001"),
            test_date(),
            Money::new(5_000_000.0, Currency::USD),
            Money::new(1_000_000.0, Currency::USD),
            10,
            ImMethodology::Simm,
        );
        portfolio_result
            .add_netting_set(bilateral)
            .expect("same currency should succeed");

        // Add cleared netting set
        let cleared = NettingSetMargin::new(
            NettingSetId::cleared("LCH"),
            test_date(),
            Money::new(3_000_000.0, Currency::USD),
            Money::new(500_000.0, Currency::USD),
            5,
            ImMethodology::ClearingHouse,
        );
        portfolio_result
            .add_netting_set(cleared)
            .expect("same currency should succeed");

        assert_eq!(portfolio_result.netting_set_count(), 2);
        assert_eq!(portfolio_result.total_initial_margin.amount(), 8_000_000.0);
        assert_eq!(
            portfolio_result.total_variation_margin.amount(),
            1_500_000.0
        );
        assert_eq!(portfolio_result.total_positions, 15);

        let (cleared_total, bilateral_total) = portfolio_result.cleared_bilateral_split();
        assert_eq!(cleared_total.amount(), 3_500_000.0);
        assert_eq!(bilateral_total.amount(), 6_000_000.0);
    }

    #[test]
    fn test_currency_mismatch_error() {
        let mut portfolio_result = PortfolioMarginResult::new(test_date(), Currency::USD);

        // Try to add EUR netting set to USD portfolio
        let eur_netting_set = NettingSetMargin::new(
            NettingSetId::bilateral("BANK_B", "CSA_002"),
            test_date(),
            Money::new(1_000_000.0, Currency::EUR),
            Money::new(200_000.0, Currency::EUR),
            5,
            ImMethodology::Simm,
        );

        let result = portfolio_result.add_netting_set(eur_netting_set);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.netting_set_currency, Currency::EUR);
        assert_eq!(err.base_currency, Currency::USD);
    }

    #[test]
    fn test_add_netting_set_with_fx() {
        let mut portfolio_result = PortfolioMarginResult::new(test_date(), Currency::USD);

        // Add EUR netting set with FX conversion (EUR/USD = 1.10)
        let eur_netting_set = NettingSetMargin::new(
            NettingSetId::bilateral("BANK_B", "CSA_002"),
            test_date(),
            Money::new(1_000_000.0, Currency::EUR),
            Money::new(200_000.0, Currency::EUR),
            5,
            ImMethodology::Simm,
        );

        let eur_usd_rate = 1.10;
        portfolio_result.add_netting_set_with_fx(eur_netting_set, eur_usd_rate);

        // Verify conversion: 1M EUR * 1.10 = 1.1M USD
        assert!((portfolio_result.total_initial_margin.amount() - 1_100_000.0).abs() < 1e-9);
        assert!((portfolio_result.total_variation_margin.amount() - 220_000.0).abs() < 1e-9);
    }
}
