//! Commodity option instrument definition and pricing logic.

use crate::instruments::common::models::trees::binomial_tree::BinomialTree;
use crate::instruments::common::parameters::OptionMarketParams;
use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::{
    Attributes, CurveDependencies, CurveIdVec, Instrument, InstrumentCurves,
};
use crate::instruments::{ExerciseStyle, OptionType, PricingOverrides, SettlementType};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;
use smallvec::smallvec;

/// Commodity option (option on commodity forward or spot).
///
/// # Pricing
///
/// - **European options**: Black-76 model using the forward price from the `PriceCurve`
/// - **American options**: Binomial tree (Leisen-Reimer) with cost-of-carry derived from
///   the forward/spot relationship
///
/// # American Option Assumptions
///
/// For American exercise, the model requires a spot price to build the binomial tree.
/// If `spot_price_id` is provided, it uses that spot price. Otherwise, the forward
/// price is used as a proxy for spot, which may underestimate early exercise value.
/// The convenience yield (cost-of-carry) is implied from the forward/spot ratio:
/// `q = r - ln(F/S)/T`
///
/// # Forward Price Retrieval
///
/// Forward prices are retrieved from a `PriceCurve` (not a `ForwardCurve`).
/// The curve must be added via `MarketContext::insert_price_curve()`.
/// If `quoted_forward` is provided, it overrides the curve lookup.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CommodityOption {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Commodity type (e.g., "Energy", "Metal", "Agricultural").
    pub commodity_type: String,
    /// Ticker or symbol (e.g., "CL" for WTI, "GC" for Gold).
    pub ticker: String,
    /// Strike price per unit.
    pub strike: f64,
    /// Option type (call or put).
    pub option_type: OptionType,
    /// Exercise style (European or American).
    pub exercise_style: ExerciseStyle,
    /// Option expiry date.
    pub expiry: Date,
    /// Contract quantity in units.
    pub quantity: f64,
    /// Unit of measurement (e.g., "BBL", "MT", "OZ").
    pub unit: String,
    /// Contract multiplier (typically 1.0 for OTC options).
    pub multiplier: f64,
    /// Settlement type (physical or cash).
    pub settlement: SettlementType,
    /// Currency for pricing.
    pub currency: Currency,
    /// Forward/futures curve ID for price interpolation.
    pub forward_curve_id: CurveId,
    /// Discount curve ID for present value.
    pub discount_curve_id: CurveId,
    /// Volatility surface ID for implied vol.
    pub vol_surface_id: CurveId,
    /// Optional spot price ID (for spot-based pricing and American options).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub spot_price_id: Option<String>,
    /// Optional quoted forward price (overrides curve lookup).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub quoted_forward: Option<f64>,
    /// Day count convention for time to expiry.
    pub day_count: DayCount,
    /// Pricing overrides (implied vol, tree steps, etc.).
    pub pricing_overrides: PricingOverrides,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

impl CommodityOption {
    /// Create a canonical example commodity option for testing and documentation.
    ///
    /// Returns a WTI European call option.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("WTI-OPT-2025M06"))
            .commodity_type("Energy".to_string())
            .ticker("CL".to_string())
            .strike(75.0)
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(
                Date::from_calendar_date(2025, time::Month::June, 15).expect("valid example date"),
            )
            .quantity(1000.0)
            .unit("BBL".to_string())
            .multiplier(1.0)
            .settlement(SettlementType::Cash)
            .currency(Currency::USD)
            .forward_curve_id(CurveId::new("WTI-FORWARD"))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_surface_id(CurveId::new("WTI-VOL"))
            .day_count(DayCount::Act365F)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example commodity option construction should not fail")
    }

    /// Calculate the net present value of this commodity option.
    pub fn npv(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        let t = self.time_to_expiry(as_of)?;
        if t <= 0.0 {
            let underlying = if let Some(spot) = self.spot_price(market)? {
                spot
            } else {
                self.forward_price(market, as_of)?
            };
            let intrinsic = self.intrinsic_value(underlying);
            return Ok(Money::new(
                intrinsic * self.quantity * self.multiplier,
                self.currency,
            ));
        }

        let inputs = self.collect_inputs(market, as_of)?;

        let unit_price = match self.exercise_style {
            ExerciseStyle::European => black76_unit_price(
                inputs.forward,
                self.strike,
                inputs.sigma,
                inputs.t,
                inputs.df,
                self.option_type,
            ),
            ExerciseStyle::American | ExerciseStyle::Bermudan => {
                let steps = self.pricing_overrides.tree_steps.unwrap_or(201);
                let tree = BinomialTree::leisen_reimer_odd(steps);
                let params = OptionMarketParams {
                    spot: inputs.spot,
                    strike: self.strike,
                    rate: inputs.r,
                    dividend_yield: inputs.q,
                    volatility: inputs.sigma,
                    time_to_expiry: inputs.t,
                    option_type: self.option_type,
                };
                tree.price_american(&params)?
            }
        };

        Ok(Money::new(
            unit_price * self.quantity * self.multiplier,
            self.currency,
        ))
    }

    fn intrinsic_value(&self, underlying: f64) -> f64 {
        match self.option_type {
            OptionType::Call => (underlying - self.strike).max(0.0),
            OptionType::Put => (self.strike - underlying).max(0.0),
        }
    }

    fn time_to_expiry(&self, as_of: Date) -> Result<f64> {
        self.day_count
            .year_fraction(as_of, self.expiry, DayCountCtx::default())
            .map(|t| t.max(0.0))
    }

    fn collect_inputs(&self, market: &MarketContext, as_of: Date) -> Result<CommodityOptionInputs> {
        let disc = market.get_discount(self.discount_curve_id.as_str())?;
        let curve_dc = disc.day_count();
        let t_rate = curve_dc.year_fraction(as_of, self.expiry, DayCountCtx::default())?;
        let r = disc.zero(t_rate.max(0.0));
        let t = self.time_to_expiry(as_of)?;

        let sigma = if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let surface = market.surface(self.vol_surface_id.as_str())?;
            surface.value_clamped(t, self.strike)
        };

        let forward = self.forward_price(market, as_of)?;
        let df = disc.df_between_dates(as_of, self.expiry)?;

        let spot = if let Some(spot) = self.spot_price(market)? {
            spot
        } else {
            forward
        };
        let q = implied_carry(spot, forward, r, t);

        Ok(CommodityOptionInputs {
            forward,
            spot,
            r,
            q,
            sigma,
            t,
            df,
        })
    }

    fn spot_price(&self, market: &MarketContext) -> Result<Option<f64>> {
        let Some(spot_id) = &self.spot_price_id else {
            return Ok(None);
        };
        let scalar = market.price(spot_id)?;
        let spot = match scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };
        Ok(Some(spot))
    }

    /// Get the forward price for this option.
    ///
    /// Uses `quoted_forward` if provided, otherwise retrieves from the `PriceCurve`
    /// specified by `forward_curve_id`. If no `PriceCurve` is found but `spot_price_id`
    /// is provided, falls back to cost-of-carry model: F = S × exp(r × T).
    pub fn forward_price(&self, market: &MarketContext, as_of: Date) -> Result<f64> {
        // 1. Direct override takes precedence
        if let Some(price) = self.quoted_forward {
            return Ok(price);
        }

        let t = DayCount::Act365F
            .year_fraction(as_of, self.expiry, DayCountCtx::default())?
            .max(0.0);

        // 2. Try to get price from PriceCurve
        if let Ok(price_curve) = market.get_price_curve(self.forward_curve_id.as_str()) {
            return Ok(price_curve.price(t));
        }

        // 3. Fallback: cost-of-carry model if spot is available
        if let Some(spot) = self.spot_price(market)? {
            let disc = market.get_discount(self.discount_curve_id.as_str())?;
            let r = disc.zero(t);
            return Ok(spot * (r * t).exp());
        }

        // 4. No PriceCurve and no spot - error with helpful message
        Err(finstack_core::Error::Input(
            finstack_core::error::InputError::NotFound {
                id: format!(
                    "PriceCurve '{}' not found. \
                     Use MarketContext::insert_price_curve() to add a commodity forward price curve.",
                    self.forward_curve_id
                ),
            },
        ))
    }
}

struct CommodityOptionInputs {
    forward: f64,
    spot: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    df: f64,
}

fn implied_carry(spot: f64, forward: f64, r: f64, t: f64) -> f64 {
    if t <= 0.0 || spot <= 0.0 || forward <= 0.0 {
        return r;
    }
    let carry = (forward / spot).ln() / t;
    r - carry
}

fn black76_unit_price(
    forward: f64,
    strike: f64,
    sigma: f64,
    t: f64,
    df: f64,
    option_type: OptionType,
) -> f64 {
    if t <= 0.0 || sigma <= 0.0 {
        let intrinsic = match option_type {
            OptionType::Call => (forward - strike).max(0.0),
            OptionType::Put => (strike - forward).max(0.0),
        };
        return intrinsic * df;
    }

    let d1 = crate::instruments::common::models::d1_black76(forward, strike, sigma, t);
    let d2 = crate::instruments::common::models::d2_black76(forward, strike, sigma, t);

    let price = match option_type {
        OptionType::Call => {
            forward * finstack_core::math::norm_cdf(d1) - strike * finstack_core::math::norm_cdf(d2)
        }
        OptionType::Put => {
            strike * finstack_core::math::norm_cdf(-d2)
                - forward * finstack_core::math::norm_cdf(-d1)
        }
    };
    price * df
}

impl CurveDependencies for CommodityOption {
    fn curve_dependencies(&self) -> InstrumentCurves {
        InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_curve_id.clone())
            .build()
    }
}

impl Instrument for CommodityOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CommodityOption
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    fn value(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        self.npv(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> Result<crate::results::ValuationResult> {
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
        smallvec![self.discount_curve_id.clone()]
    }

    fn spot_id(&self) -> Option<&str> {
        self.spot_price_id.as_deref()
    }

    fn vol_surface_id(&self) -> Option<CurveId> {
        Some(self.vol_surface_id.clone())
    }
}

impl HasDiscountCurve for CommodityOption {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for CommodityOption {
    fn forward_curve_ids(&self) -> Vec<CurveId> {
        vec![self.forward_curve_id.clone()]
    }
}
