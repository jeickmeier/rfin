//! NDF types and implementations.
//!
//! Defines the `Ndf` instrument for non-deliverable forward contracts on
//! restricted currencies. Supports both pre-fixing (forward rate estimation)
//! and post-fixing (observed rate) valuation modes.

use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::{Attributes, CurveIdVec};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;
use smallvec::smallvec;

/// Non-Deliverable Forward (NDF) instrument.
///
/// Represents a cash-settled forward contract on a restricted currency pair.
/// The position is long base currency (restricted) and short settlement currency.
///
/// # Pricing
///
/// ## Pre-Fixing (fixing_rate = None)
/// Forward rate is estimated via covered interest rate parity or fallback:
/// ```text
/// PV = notional × (F_market - contract_rate) × DF_settlement(T)
/// ```
///
/// ## Post-Fixing (fixing_rate = Some)
/// Uses the observed fixing rate:
/// ```text
/// PV = notional × (fixing_rate - contract_rate) × DF_settlement(T)
/// ```
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::ndf::Ndf;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use finstack_core::money::Money;
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let ndf = Ndf::builder()
///     .id(InstrumentId::new("USDCNY-NDF-3M"))
///     .base_currency(Currency::CNY)
///     .settlement_currency(Currency::USD)
///     .fixing_date(Date::from_calendar_date(2025, Month::March, 13).unwrap())
///     .maturity_date(Date::from_calendar_date(2025, Month::March, 15).unwrap())
///     .notional(Money::new(10_000_000.0, Currency::CNY))
///     .contract_rate(7.25)
///     .settlement_curve_id(CurveId::new("USD-OIS"))
///     .build()
///     .expect("Valid NDF");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Ndf {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Base currency (restricted/non-deliverable currency, numerator).
    pub base_currency: Currency,
    /// Settlement currency (freely convertible, typically USD, denominator and PV currency).
    pub settlement_currency: Currency,
    /// Fixing date (rate observation date, typically T-2 before maturity).
    pub fixing_date: Date,
    /// Maturity/settlement date.
    pub maturity_date: Date,
    /// Notional amount in base currency.
    pub notional: Money,
    /// Contract forward rate (base per settlement, e.g., 7.25 CNY per USD).
    pub contract_rate: f64,
    /// Settlement currency discount curve ID.
    pub settlement_curve_id: CurveId,
    /// Optional foreign (base) currency discount curve ID.
    /// If not provided, forward rate estimation uses settlement curve as fallback.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub foreign_curve_id: Option<CurveId>,
    /// Observed fixing rate (base per settlement). If Some, NDF is post-fixing.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fixing_rate: Option<f64>,
    /// Fixing source/benchmark (e.g., "CNHFIX", "RBI", "PTAX").
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fixing_source: Option<String>,
    /// Optional spot rate override for forward rate calculation.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub spot_rate_override: Option<f64>,
    /// Optional base currency calendar.
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub base_calendar_id: Option<String>,
    /// Optional settlement currency calendar.
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub settlement_calendar_id: Option<String>,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

impl Ndf {
    /// Create a canonical example NDF for testing and documentation.
    ///
    /// Returns a 3-month USD/CNY NDF with realistic parameters.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("USDCNY-NDF-3M"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(
                Date::from_calendar_date(2025, time::Month::March, 13).expect("Valid example date"),
            )
            .maturity_date(
                Date::from_calendar_date(2025, time::Month::March, 15).expect("Valid example date"),
            )
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .fixing_source_opt(Some("CNHFIX".to_string()))
            .attributes(
                Attributes::new()
                    .with_tag("ndf")
                    .with_meta("pair", "USDCNY"),
            )
            .build()
            .expect("Example NDF construction should not fail")
    }

    /// Construct an NDF from trade date and tenor using standard fixing offset.
    ///
    /// # Arguments
    ///
    /// * `id` - Instrument identifier
    /// * `base_currency` - Restricted currency (numerator)
    /// * `settlement_currency` - Convertible currency (denominator)
    /// * `trade_date` - Trade date
    /// * `tenor_days` - Days from spot to maturity
    /// * `notional` - Notional in base currency
    /// * `contract_rate` - Contract forward rate
    /// * `settlement_curve_id` - Settlement currency discount curve
    /// * `base_calendar_id` - Optional base currency calendar
    /// * `settlement_calendar_id` - Optional settlement currency calendar
    /// * `spot_lag_days` - Spot lag (typically 2)
    /// * `fixing_offset_days` - Days before maturity for fixing (typically 2)
    /// * `bdc` - Business day convention
    #[allow(clippy::too_many_arguments)]
    pub fn from_trade_date(
        id: impl Into<InstrumentId>,
        base_currency: Currency,
        settlement_currency: Currency,
        trade_date: Date,
        tenor_days: i64,
        notional: Money,
        contract_rate: f64,
        settlement_curve_id: impl Into<CurveId>,
        base_calendar_id: Option<String>,
        settlement_calendar_id: Option<String>,
        spot_lag_days: u32,
        fixing_offset_days: i64,
        bdc: finstack_core::dates::BusinessDayConvention,
    ) -> finstack_core::Result<Self> {
        use crate::instruments::common::fx_dates::{adjust_joint_calendar, roll_spot_date};

        let spot_date = roll_spot_date(
            trade_date,
            spot_lag_days,
            bdc,
            base_calendar_id.as_deref(),
            settlement_calendar_id.as_deref(),
        )?;
        let maturity_unadjusted = spot_date + time::Duration::days(tenor_days);
        let maturity_date = adjust_joint_calendar(
            maturity_unadjusted,
            bdc,
            base_calendar_id.as_deref(),
            settlement_calendar_id.as_deref(),
        )?;

        // Fixing date is typically T-2 before maturity
        let fixing_unadjusted = maturity_date - time::Duration::days(fixing_offset_days);
        let fixing_date = adjust_joint_calendar(
            fixing_unadjusted,
            finstack_core::dates::BusinessDayConvention::Preceding,
            base_calendar_id.as_deref(),
            settlement_calendar_id.as_deref(),
        )?;

        Self::builder()
            .id(id.into())
            .base_currency(base_currency)
            .settlement_currency(settlement_currency)
            .fixing_date(fixing_date)
            .maturity_date(maturity_date)
            .notional(notional)
            .contract_rate(contract_rate)
            .settlement_curve_id(settlement_curve_id.into())
            .base_calendar_id_opt(base_calendar_id)
            .settlement_calendar_id_opt(settlement_calendar_id)
            .attributes(Attributes::new())
            .build()
    }

    /// Set the observed fixing rate (transitions NDF to post-fixing mode).
    pub fn with_fixing_rate(mut self, fixing_rate: f64) -> Self {
        self.fixing_rate = Some(fixing_rate);
        self
    }

    /// Check if NDF is in post-fixing mode.
    pub fn is_fixed(&self) -> bool {
        self.fixing_rate.is_some()
    }

    /// Compute present value in settlement currency.
    ///
    /// # Pre-Fixing Mode
    ///
    /// If `fixing_rate` is None and as_of < fixing_date:
    /// - Estimate forward rate via CIRP if foreign curve available
    /// - Otherwise use settlement curve fallback (simplified model for restricted currencies)
    ///
    /// # Post-Fixing Mode
    ///
    /// If `fixing_rate` is Some or as_of >= fixing_date:
    /// - Use the observed fixing rate for settlement calculation
    pub fn npv(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        // If maturity has passed, value is zero
        if self.maturity_date < as_of {
            return Ok(Money::new(0.0, self.settlement_currency));
        }

        // Get settlement discount curve
        let settlement_disc = market.get_discount(self.settlement_curve_id.as_str())?;
        let df_settlement = settlement_disc.df_between_dates(as_of, self.maturity_date)?;

        // Determine the forward rate to use
        let effective_forward = if let Some(fixed_rate) = self.fixing_rate {
            // Post-fixing: use observed rate
            fixed_rate
        } else if as_of >= self.fixing_date {
            // Past fixing date but no rate set - this is an error condition in practice,
            // but for robustness we use the contract rate (PV = 0)
            self.contract_rate
        } else {
            // Pre-fixing: estimate forward rate
            self.estimate_forward_rate(market, as_of)?
        };

        // Validate notional currency
        if self.notional.currency() != self.base_currency {
            return Err(finstack_core::Error::from(
                finstack_core::InputError::Invalid,
            ));
        }
        let n_base = self.notional.amount();

        // PV = notional_base × (F_effective - F_contract) / F_contract × DF_settlement
        // Note: For NDF, we convert base currency notional to settlement currency using contract rate
        // PV = (notional_base / contract_rate) × (F_effective - F_contract) × DF_settlement
        // Simplified: PV = notional_settlement × (F_effective/F_contract - 1) × DF_settlement
        //
        // Alternative convention (more common):
        // Settlement amount = notional_base × (1/F_contract - 1/F_fixing)
        // PV = settlement_amount × DF
        //
        // Using the second convention:
        let settlement_amount = n_base * (1.0 / self.contract_rate - 1.0 / effective_forward);
        let pv = settlement_amount * df_settlement;

        Ok(Money::new(pv, self.settlement_currency))
    }

    /// Estimate the forward rate when in pre-fixing mode.
    fn estimate_forward_rate(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        use finstack_core::money::fx::FxQuery;

        // Try to get spot rate
        let spot = if let Some(rate) = self.spot_rate_override {
            rate
        } else if let Some(fx) = market.fx() {
            // Query for base/settlement rate (e.g., CNY/USD)
            match (**fx).rate(FxQuery::new(
                self.base_currency,
                self.settlement_currency,
                as_of,
            )) {
                Ok(fx_rate) => fx_rate.rate,
                Err(_) => {
                    // Try inverse
                    let inverse = (**fx).rate(FxQuery::new(
                        self.settlement_currency,
                        self.base_currency,
                        as_of,
                    ))?;
                    1.0 / inverse.rate
                }
            }
        } else {
            // No FX matrix, use contract rate as proxy (simplified)
            return Ok(self.contract_rate);
        };

        // Get settlement discount factor
        let settlement_disc = market.get_discount(self.settlement_curve_id.as_str())?;
        let df_settlement = settlement_disc.df_between_dates(as_of, self.maturity_date)?;

        // If foreign curve available, use CIRP
        if let Some(ref foreign_curve_id) = self.foreign_curve_id {
            if let Ok(foreign_disc) = market.get_discount(foreign_curve_id.as_str()) {
                let df_foreign = foreign_disc.df_between_dates(as_of, self.maturity_date)?;
                // F = S × DF_foreign / DF_settlement
                return Ok(spot * df_foreign / df_settlement);
            }
        }

        // Fallback for restricted currencies: assume flat basis (F ≈ S adjusted for time)
        // This is a simplification; in practice you'd use NDF market quotes or basis curves
        // For now, use the settlement curve alone: F = S (no adjustment for restricted currency rate)
        Ok(spot)
    }
}

impl crate::instruments::common::traits::CurveDependencies for Ndf {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        let mut builder = crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.settlement_curve_id.clone());

        if let Some(ref foreign_curve) = self.foreign_curve_id {
            builder = builder.discount(foreign_curve.clone());
        }

        builder.build()
    }
}

impl crate::instruments::common::traits::Instrument for Ndf {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::Ndf
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn required_discount_curves(&self) -> CurveIdVec {
        let mut curves = smallvec![self.settlement_curve_id.clone()];
        if let Some(ref foreign_curve) = self.foreign_curve_id {
            curves.push(foreign_curve.clone());
        }
        curves
    }
}

impl HasDiscountCurve for Ndf {
    fn discount_curve_id(&self) -> &CurveId {
        &self.settlement_curve_id
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_ndf_creation() {
        let ndf = Ndf::builder()
            .id(InstrumentId::new("TEST-NDF"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2025, Month::March, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2025, Month::March, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        assert_eq!(ndf.id.as_str(), "TEST-NDF");
        assert_eq!(ndf.base_currency, Currency::CNY);
        assert_eq!(ndf.settlement_currency, Currency::USD);
        assert_eq!(ndf.contract_rate, 7.25);
        assert!(!ndf.is_fixed());
    }

    #[test]
    fn test_ndf_example() {
        let ndf = Ndf::example();
        assert_eq!(ndf.id.as_str(), "USDCNY-NDF-3M");
        assert_eq!(ndf.base_currency, Currency::CNY);
        assert_eq!(ndf.settlement_currency, Currency::USD);
        assert!(ndf.attributes.has_tag("ndf"));
    }

    #[test]
    fn test_ndf_with_fixing_rate() {
        let ndf = Ndf::example().with_fixing_rate(7.30);
        assert!(ndf.is_fixed());
        assert_eq!(ndf.fixing_rate, Some(7.30));
    }

    #[test]
    fn test_ndf_instrument_trait() {
        use crate::instruments::common::traits::Instrument;

        let ndf = Ndf::example();

        assert_eq!(ndf.id(), "USDCNY-NDF-3M");
        assert_eq!(ndf.key(), crate::pricer::InstrumentType::Ndf);
        assert!(ndf.attributes().has_tag("ndf"));
    }

    #[test]
    fn test_ndf_curve_dependencies() {
        use crate::instruments::common::traits::CurveDependencies;

        let ndf = Ndf::example();
        let deps = ndf.curve_dependencies();

        assert_eq!(deps.discount_curves.len(), 1);
    }

    #[test]
    fn test_ndf_with_foreign_curve() {
        use crate::instruments::common::traits::CurveDependencies;

        let ndf = Ndf::builder()
            .id(InstrumentId::new("TEST-NDF"))
            .base_currency(Currency::CNY)
            .settlement_currency(Currency::USD)
            .fixing_date(Date::from_calendar_date(2025, Month::March, 13).expect("valid date"))
            .maturity_date(Date::from_calendar_date(2025, Month::March, 15).expect("valid date"))
            .notional(Money::new(10_000_000.0, Currency::CNY))
            .contract_rate(7.25)
            .settlement_curve_id(CurveId::new("USD-OIS"))
            .foreign_curve_id_opt(Some(CurveId::new("CNY-OIS")))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        let deps = ndf.curve_dependencies();
        assert_eq!(deps.discount_curves.len(), 2);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_ndf_serde_roundtrip() {
        let ndf = Ndf::example();
        let json = serde_json::to_string(&ndf).expect("serialize");
        let deserialized: Ndf = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(ndf.id.as_str(), deserialized.id.as_str());
        assert_eq!(ndf.base_currency, deserialized.base_currency);
        assert_eq!(ndf.settlement_currency, deserialized.settlement_currency);
    }
}
