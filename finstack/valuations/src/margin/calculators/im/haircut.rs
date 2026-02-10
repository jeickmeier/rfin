//! Haircut-based initial margin calculator.
//!
//! Standard methodology for repos and securities financing transactions
//! where IM is calculated as a percentage of collateral value.

use crate::instruments::common_impl::traits::Instrument;
use crate::margin::calculators::traits::{ImCalculator, ImResult};
use crate::margin::types::{CollateralAssetClass, EligibleCollateralSchedule, ImMethodology};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Haircut-based initial margin calculator.
///
/// Calculates IM based on collateral value and asset-class-specific haircuts.
/// This is the standard methodology for repos, securities lending, and
/// other secured financing transactions.
///
/// # Formula
///
/// ```text
/// IM = Collateral_Value × Haircut
/// Total_Haircut = Base_Haircut + FX_Addon (if currency mismatch)
/// ```
///
/// # BCBS-IOSCO Haircut Schedule
///
/// Standard haircuts by asset class:
/// - Cash: 0% (8% FX addon if currency mismatch)
/// - Government bonds ≤1yr: 0.5%
/// - Government bonds 1-5yr: 2%
/// - Government bonds >5yr: 4%
/// - Corporate bonds IG: 2-8%
/// - Equity: 15%
///
/// # Example
///
/// ```rust,no_run
/// use finstack_valuations::instruments::Instrument;
/// use finstack_valuations::margin::{EligibleCollateralSchedule, HaircutImCalculator, ImCalculator};
/// use finstack_core::dates::Date;
/// use finstack_core::market_data::context::MarketContext;
/// use time::macros::date;
///
/// # fn main() -> finstack_core::Result<()> {
/// let schedule = EligibleCollateralSchedule::us_treasuries();
/// let calc = HaircutImCalculator::new(schedule);
///
/// # let repo: &dyn Instrument = todo!("provide a repo / secured financing instrument");
/// # let context = MarketContext::new();
/// # let as_of: Date = date!(2025-01-01);
/// let im = calc.calculate(repo, &context, as_of)?;
/// println!("Haircut IM: {}", im.amount);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct HaircutImCalculator {
    /// Eligible collateral schedule with haircuts
    eligible_collateral: EligibleCollateralSchedule,

    /// Default collateral asset class to assume
    default_asset_class: CollateralAssetClass,

    /// Whether to apply FX haircut addon
    apply_fx_addon: bool,
}

impl HaircutImCalculator {
    /// Create a new haircut calculator with the given collateral schedule.
    #[must_use]
    pub fn new(eligible_collateral: EligibleCollateralSchedule) -> Self {
        Self {
            eligible_collateral,
            default_asset_class: CollateralAssetClass::GovernmentBonds,
            apply_fx_addon: false,
        }
    }

    /// Create a calculator for US Treasury collateral.
    #[must_use]
    pub fn us_treasuries() -> Self {
        Self::new(EligibleCollateralSchedule::us_treasuries())
    }

    /// Create a calculator with BCBS-IOSCO standard haircuts.
    #[must_use]
    pub fn bcbs_standard() -> Self {
        Self::new(EligibleCollateralSchedule::bcbs_standard())
    }

    /// Set whether to apply FX haircut addon.
    #[must_use]
    pub fn with_fx_addon(mut self, apply: bool) -> Self {
        self.apply_fx_addon = apply;
        self
    }

    /// Set the default asset class.
    #[must_use]
    pub fn with_default_asset_class(mut self, asset_class: CollateralAssetClass) -> Self {
        self.default_asset_class = asset_class;
        self
    }

    /// Calculate haircut IM for a given collateral value and asset class.
    pub fn calculate_for_collateral(
        &self,
        collateral_value: Money,
        asset_class: &CollateralAssetClass,
        currency_mismatch: bool,
    ) -> Result<Money> {
        let haircut = match self.eligible_collateral.haircut_for(asset_class) {
            Some(h) => h,
            None => asset_class.standard_haircut()?,
        };

        let total_haircut = if currency_mismatch && self.apply_fx_addon {
            haircut + asset_class.fx_addon()?
        } else {
            haircut
        };

        Ok(collateral_value * total_haircut)
    }

    /// Get the haircut for an asset class.
    pub fn haircut_for(&self, asset_class: &CollateralAssetClass) -> Result<f64> {
        match self.eligible_collateral.haircut_for(asset_class) {
            Some(h) => Ok(h),
            None => asset_class.standard_haircut(),
        }
    }
}

impl ImCalculator for HaircutImCalculator {
    fn calculate(
        &self,
        instrument: &dyn Instrument,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        // Prefer actual notional as collateral value; fall back to PV if unavailable
        let collateral_value = instrument
            .as_cashflow_provider()
            .and_then(|cp| cp.notional())
            .map(|n| Money::new(n.amount().abs(), n.currency()))
            .unwrap_or_else(|| {
                instrument
                    .value(context, as_of)
                    .map(|pv| Money::new(pv.amount().abs(), pv.currency()))
                    .unwrap_or_else(|_| Money::new(0.0, finstack_core::currency::Currency::USD))
            });

        let im_amount = self.calculate_for_collateral(
            collateral_value,
            &self.default_asset_class,
            self.apply_fx_addon,
        )?;

        let mut breakdown = finstack_core::HashMap::default();
        breakdown.insert(self.default_asset_class.to_string(), im_amount);

        Ok(ImResult::with_breakdown(
            im_amount,
            ImMethodology::Haircut,
            as_of,
            2, // Short MPOR for repos
            breakdown,
        ))
    }

    fn methodology(&self) -> ImMethodology {
        ImMethodology::Haircut
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn haircut_calculation() {
        let calc = HaircutImCalculator::us_treasuries();

        let collateral = Money::new(10_000_000.0, Currency::USD);
        let im = calc
            .calculate_for_collateral(collateral, &CollateralAssetClass::GovernmentBonds, false)
            .expect("calculation ok");

        // Should apply ~1-2% haircut
        assert!(im.amount() > 0.0);
        assert!(im.amount() < 500_000.0); // Less than 5%
    }

    #[test]
    fn fx_addon_applied() {
        let calc = HaircutImCalculator::bcbs_standard().with_fx_addon(true);

        let collateral = Money::new(10_000_000.0, Currency::USD);

        let im_no_fx = calc
            .calculate_for_collateral(collateral, &CollateralAssetClass::Cash, false)
            .expect("calculation ok");

        let im_with_fx = calc
            .calculate_for_collateral(collateral, &CollateralAssetClass::Cash, true)
            .expect("calculation ok");

        // Cash with FX mismatch should have 8% haircut
        assert_eq!(im_no_fx.amount(), 0.0); // Cash has 0% haircut
        assert_eq!(im_with_fx.amount(), 800_000.0); // 8% FX addon
    }

    #[test]
    fn default_haircuts() {
        assert_eq!(
            CollateralAssetClass::Cash
                .standard_haircut()
                .expect("default class should be configured"),
            0.0
        );
        assert_eq!(
            CollateralAssetClass::Equity
                .standard_haircut()
                .expect("default class should be configured"),
            0.15
        );
        assert_eq!(
            CollateralAssetClass::Gold
                .standard_haircut()
                .expect("default class should be configured"),
            0.15
        );
    }
}
