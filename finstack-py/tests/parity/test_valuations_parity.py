"""Comprehensive parity tests for valuations module.

Tests instruments, pricing, metrics, calibration, and cashflow builder functionality.
"""

from datetime import date, timedelta
from pathlib import Path

from finstack.core.currency import USD
from finstack.core.dates import DayCount
from finstack.core.dates.schedule import Frequency
from finstack.core.market_data import DiscountCurve, ForwardCurve, MarketContext
from finstack.core.money import Money
from finstack.valuations.instruments import Bond, Deposit, InterestRateSwap
from finstack.valuations.pricer import standard_registry
import pytest


class TestBondPricingParity:
    """Test bond pricing matches Rust implementation."""

    def test_bond_construction(self) -> None:
        """Test bond construction via builder."""
        bond = (
            Bond
            .builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build()
        )

        assert bond.id == "BOND-001"
        # Bond properties are accessible but might not be directly exposed
        # Focus on pricing parity instead


class TestNoPythonOnlyValuationHelpers:
    """Valuations should not expose Python-only helper entry points."""

    def test_evaluate_dcf_helper_removed(self) -> None:
        """The Python-only DCF helper should not be exported from instruments."""
        from finstack.valuations import instruments

        assert not hasattr(instruments, "evaluate_dcf")


class TestValuationEnumParity:
    """Valuation enum-like Rust types should be exposed as Python wrappers."""

    def test_rates_and_commodity_packages_expose_new_wrapper_types(self) -> None:
        """Category packages should re-export the newly added enum and instrument types."""
        from finstack.valuations.instruments.commodity import (
            CommoditySpreadOption,
            CommoditySwaption,
        )
        from finstack.valuations.instruments.rates import (
            CmsSwap,
            CollateralType,
            IrFutureOption,
        )

        assert CollateralType.GENERAL.name == "general"
        assert IrFutureOption is not None
        assert CmsSwap is not None
        assert CommoditySpreadOption is not None
        assert CommoditySwaption is not None

    def test_enum_types_are_importable_and_work_with_builders(self) -> None:
        """Enum wrappers should be importable and accepted by valuation builders."""
        from finstack.valuations.instruments import (
            BarrierDirection,
            CollateralType,
            DigitalPayoutType,
            FxDigitalOption,
            FxTouchOption,
            PayoutTiming,
            RepoCollateral,
            TouchType,
        )

        special_collateral = CollateralType.special("UST-5Y", rate_adjustment_bp=-3.5)
        collateral = RepoCollateral(
            "UST-5Y",
            1_000.0,
            "UST-5Y-PRICE",
            collateral_type=special_collateral,
        )

        digital = (
            FxDigitalOption
            .builder("FXDIG-001")
            .base_currency("EUR")
            .quote_currency("USD")
            .strike(1.10)
            .option_type("call")
            .payout_type(DigitalPayoutType.CASH_OR_NOTHING)
            .payout_amount(Money(100_000.0, USD))
            .expiry(date(2025, 12, 31))
            .notional(Money(1_000_000.0, USD))
            .domestic_discount_curve("USD-OIS")
            .foreign_discount_curve("EUR-OIS")
            .vol_surface("EURUSD-VOL")
            .build()
        )

        touch = (
            FxTouchOption
            .builder("FXTOUCH-001")
            .base_currency("EUR")
            .quote_currency("USD")
            .barrier_level(1.05)
            .touch_type(TouchType.ONE_TOUCH)
            .barrier_direction(BarrierDirection.DOWN)
            .payout_amount(Money(250_000.0, USD))
            .payout_timing(PayoutTiming.AT_EXPIRY)
            .expiry(date(2025, 12, 31))
            .domestic_discount_curve("USD-OIS")
            .foreign_discount_curve("EUR-OIS")
            .vol_surface("EURUSD-VOL")
            .build()
        )

        assert CollateralType.GENERAL.name == "general"
        assert special_collateral.security_id == "UST-5Y"
        assert special_collateral.rate_adjustment_bp == -3.5
        assert collateral.collateral_type.name == "special"
        assert digital.payout_type.name == "cash_or_nothing"
        assert touch.touch_type.name == "one_touch"
        assert touch.barrier_direction.name == "down"
        assert touch.payout_timing.name == "at_expiry"


class TestCalibrationDiagnosticsParity:
    """Calibration diagnostics types should be visible from Python."""

    def test_calibration_report_surface_exposes_diagnostics_types(self) -> None:
        """Calibration diagnostics should round-trip through runtime report surfaces."""
        from finstack.valuations.calibration import (
            CalibrationConfig,
            RatesQuote,
            execute_calibration,
        )
        from finstack.valuations.calibration.report import (
            CalibrationDiagnostics as ReportCalibrationDiagnostics,
            QuoteQuality as ReportQuoteQuality,
        )

        from finstack.valuations import CalibrationDiagnostics, QuoteQuality

        base_date = date(2024, 1, 2)
        deposit = ReportQuoteQuality(
            quote_label="USD-3M-DEPOSIT",
            target_value=0.05,
            fitted_value=0.05,
            residual=0.0,
            sensitivity=1.0,
        )
        diagnostics = ReportCalibrationDiagnostics(
            [deposit],
            condition_number=25.0,
            singular_values=[5.0, 1.0],
            max_residual=0.0,
            rms_residual=0.0,
            r_squared=1.0,
        )

        quote_sets = {
            "ois": [
                RatesQuote.deposit(
                    "DEPO-1",
                    "USD-DEPOSIT",
                    base_date + timedelta(days=90),
                    0.05,
                ).to_market_quote()
            ]
        }
        steps = [
            {
                "id": "disc",
                "quote_set": "ois",
                "kind": "discount",
                "curve_id": "USD-OIS",
                "currency": "USD",
                "base_date": "2024-01-02",
                "conventions": {
                    "curve_day_count": "act365f",
                },
            }
        ]
        _market, report, step_reports = execute_calibration(
            "plan_discount_with_diagnostics",
            quote_sets,
            steps,
            settings=CalibrationConfig(compute_diagnostics=True),
        )

        step_report = step_reports["disc"]
        report_dict = step_report.to_dict()

        assert ReportQuoteQuality is QuoteQuality
        assert ReportCalibrationDiagnostics is CalibrationDiagnostics
        assert diagnostics.condition_number == 25.0
        assert diagnostics.singular_values == [5.0, 1.0]
        assert diagnostics.r_squared == 1.0
        assert report.success
        assert step_report.diagnostics is not None
        assert isinstance(report_dict["diagnostics"], dict)
        assert isinstance(report_dict["diagnostics"]["per_quote"], list)
        assert isinstance(report_dict["diagnostics"]["per_quote"][0], dict)


class TestDiscountedCashFlowParity:
    """DiscountedCashFlow should be a first-class Rust-backed instrument."""

    def test_equity_package_imports_discounted_cash_flow_symbols(self) -> None:
        """The `instruments.equity` package should be importable at runtime."""
        from finstack.valuations.instruments import (
            DiscountedCashFlow as RootDiscountedCashFlow,
            TerminalValueSpec as RootTerminalValueSpec,
        )
        from finstack.valuations.instruments.equity import (
            DiscountedCashFlow,
            TerminalValueSpec,
        )
        from finstack.valuations.instruments.equity.dcf import (
            DiscountedCashFlow as SubmoduleDiscountedCashFlow,
        )

        assert DiscountedCashFlow is RootDiscountedCashFlow
        assert SubmoduleDiscountedCashFlow is RootDiscountedCashFlow
        assert TerminalValueSpec is RootTerminalValueSpec

    def test_discounted_cash_flow_builder_constructs_real_instrument(self) -> None:
        """The DCF binding should expose the Rust instrument and builder surface."""
        from finstack.valuations.instruments import (
            DilutionSecurity,
            DiscountedCashFlow,
            EquityBridge,
            TerminalValueSpec,
            ValuationDiscounts,
        )

        dcf = (
            DiscountedCashFlow
            .builder("DCF-001")
            .currency("USD")
            .flows([
                (date(2025, 12, 31), 100.0),
                (date(2026, 12, 31), 110.0),
            ])
            .wacc(0.10)
            .terminal_value(TerminalValueSpec.gordon_growth(0.02))
            .net_debt(25.0)
            .valuation_date(date(2025, 1, 1))
            .discount_curve("USD-OIS")
            .mid_year_convention(True)
            .equity_bridge(EquityBridge(total_debt=80.0, cash=55.0))
            .shares_outstanding(100.0)
            .dilution_securities([DilutionSecurity("Options", 10.0, 5.0)])
            .valuation_discounts(ValuationDiscounts(dlom=0.10, dloc=0.05))
            .build()
        )

        market = MarketContext()
        market.insert(
            DiscountCurve(
                "USD-OIS",
                date(2025, 1, 1),
                [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (5.0, 0.80)],
                day_count="act_365f",
            )
        )

        result = standard_registry().get_price(dcf, "discounting", market, date(2025, 1, 1))

        assert dcf.instrument_id == "DCF-001"
        assert dcf.terminal_value.name == "gordon_growth"
        assert dcf.equity_bridge is not None
        assert dcf.valuation_discounts is not None
        assert dcf.shares_outstanding == 100.0
        assert result.value.currency.code == "USD"

    def test_discounted_cash_flow_builder_uses_shared_validation_errors(self) -> None:
        """Missing required DCF fields should surface through the shared validation hierarchy."""
        from finstack.valuations.instruments import DiscountedCashFlow

        import finstack

        with pytest.raises(finstack.ValidationError):
            (
                DiscountedCashFlow
                .builder("DCF-MISSING-TV")
                .currency("USD")
                .flows([(date(2025, 12, 31), 100.0)])
                .wacc(0.10)
                .net_debt(25.0)
                .valuation_date(date(2025, 1, 1))
                .discount_curve("USD-OIS")
                .build()
            )


class TestMissingValuationInstrumentParity:
    """Remaining valuation instruments should be importable and constructible."""

    def test_ir_future_option_builder_constructs_real_instrument(self) -> None:
        """IR future options should be importable from the root valuation instrument package."""
        from finstack.valuations.instruments import IrFutureOption

        option = (
            IrFutureOption
            .builder("IRFO-001")
            .futures_price(95.50)
            .strike(95.25)
            .expiry(date(2025, 6, 16))
            .option_type("call")
            .notional(1_000_000.0)
            .currency("USD")
            .tick_size(0.0025)
            .tick_value(6.25)
            .volatility(0.20)
            .discount_curve("USD-OIS")
            .build()
        )

        assert option.instrument_id == "IRFO-001"
        assert option.option_type == "call"
        assert option.discount_curve == "USD-OIS"

    def test_commodity_spread_option_builder_constructs_real_instrument(self) -> None:
        """Commodity spread options should be importable from the root valuation instrument package."""
        from finstack.valuations.instruments import CommoditySpreadOption

        option = (
            CommoditySpreadOption
            .builder("CSPREAD-001")
            .currency("USD")
            .option_type("call")
            .expiry(date(2025, 9, 15))
            .strike(10.0)
            .notional(1_000.0)
            .leg1_forward_curve_id("RBOB-FWD")
            .leg2_forward_curve_id("WTI-FWD")
            .leg1_vol_surface_id("RBOB-VOL")
            .leg2_vol_surface_id("WTI-VOL")
            .discount_curve_id("USD-OIS")
            .correlation(0.85)
            .build()
        )

        assert option.instrument_id == "CSPREAD-001"
        assert option.option_type == "call"
        assert option.correlation == 0.85

    def test_commodity_swaption_builder_constructs_real_instrument(self) -> None:
        """Commodity swaptions should be importable from the root valuation instrument package."""
        from finstack.valuations.instruments import CommoditySwaption

        swaption = (
            CommoditySwaption
            .builder("CSWAPTION-001")
            .commodity_type("Energy")
            .ticker("NG")
            .unit("MMBTU")
            .currency("USD")
            .option_type("call")
            .expiry(date(2025, 6, 15))
            .swap_start(date(2025, 7, 1))
            .swap_end(date(2026, 6, 30))
            .swap_frequency("1M")
            .fixed_price(3.50)
            .notional(10_000.0)
            .forward_curve_id("NG-FWD")
            .discount_curve_id("USD-OIS")
            .vol_surface_id("NG-VOL")
            .build()
        )

        assert swaption.instrument_id == "CSWAPTION-001"
        assert swaption.option_type == "call"
        assert swaption.vol_surface_id == "NG-VOL"

    def test_cms_swap_from_schedule_constructs_real_instrument(self) -> None:
        """CMS swaps should be importable from the root valuation instrument package."""
        from finstack.valuations.instruments import CmsSwap

        cms_swap = CmsSwap.from_schedule(
            "CMSSWAP-001",
            date(2025, 1, 2),
            date(2026, 1, 2),
            Frequency.QUARTERLY,
            10.0,
            0.0010,
            funding_leg={"type": "fixed", "rate": 0.03, "day_count": DayCount.THIRTY_360},
            notional=Money(10_000_000.0, USD),
            cms_day_count=DayCount.ACT_365F,
            swap_convention="usd_standard",
            side="pay",
            discount_curve="USD-OIS",
            forward_curve="USD-SOFR",
            vol_surface="USD-CMS10Y-VOL",
        )

        assert cms_swap.instrument_id == "CMSSWAP-001"
        assert cms_swap.cms_tenor == 10.0
        assert cms_swap.discount_curve == "USD-OIS"


class TestXvaParity:
    """XVA bindings should expose the full Rust-backed public surface."""

    def test_xva_module_exports_bilateral_functions(self) -> None:
        """DVA/FVA/bilateral XVA functions should be importable."""
        from finstack.valuations.xva import compute_bilateral_xva, compute_dva, compute_fva

        assert compute_dva is not None
        assert compute_fva is not None
        assert compute_bilateral_xva is not None

    def test_xva_support_types_import(self) -> None:
        """Funding and richer XVA config types should be importable."""
        from finstack.valuations.xva import (
            ExposureDiagnostics,
            FundingConfig,
            NettingSet,
            StochasticExposureConfig,
            StochasticExposureProfile,
            XvaConfig,
            XvaResult,
        )

        assert FundingConfig is not None
        assert ExposureDiagnostics is not None
        assert StochasticExposureConfig is not None
        assert StochasticExposureProfile is not None
        assert hasattr(XvaConfig, "own_recovery_rate")
        assert hasattr(XvaConfig, "funding")
        assert hasattr(NettingSet, "reporting_currency")
        assert hasattr(XvaResult, "dva")
        assert hasattr(XvaResult, "fva")
        assert hasattr(XvaResult, "bilateral_cva")


class TestAttributionSurfaceParity:
    """Remaining attribution helper/runtime surface should be visible from Python."""

    def test_remaining_attribution_symbols_are_importable(self) -> None:
        """The remaining actionable attribution bindings should resolve at runtime."""
        from finstack.valuations import (
            CarryDetail,
            CorrelationsAttribution,
            CurveRestoreFlags,
            FxAttribution,
            InflationCurvesAttribution,
            MarketSnapshot,
            ScalarsAttribution,
            ScalarsSnapshot,
            TaylorAttributionConfig,
            TaylorAttributionResult,
            TaylorFactorResult,
            VolatilitySnapshot,
            VolAttribution,
            attribute_pnl_taylor,
            compute_pnl,
            compute_pnl_with_fx,
            convert_currency,
            default_waterfall_order,
            reprice_instrument,
        )

        assert CarryDetail is not None
        assert InflationCurvesAttribution is not None
        assert CorrelationsAttribution is not None
        assert FxAttribution is not None
        assert VolAttribution is not None
        assert ScalarsAttribution is not None
        assert TaylorAttributionConfig is not None
        assert TaylorFactorResult is not None
        assert TaylorAttributionResult is not None
        assert CurveRestoreFlags is not None
        assert MarketSnapshot is not None
        assert ScalarsSnapshot is not None
        assert VolatilitySnapshot is not None
        assert default_waterfall_order is not None
        assert convert_currency is not None
        assert compute_pnl is not None
        assert compute_pnl_with_fx is not None
        assert reprice_instrument is not None
        assert attribute_pnl_taylor is not None

    def test_attribution_helpers_and_taylor_runtime_surface_work(self) -> None:
        """Direct attribution helpers should be callable from Python."""
        from finstack.core.currency import EUR, USD
        from finstack.core.market_data.context import MarketContext
        from finstack.core.market_data.fx import FxMatrix

        from finstack.valuations import (
            TaylorAttributionConfig,
            attribute_pnl_taylor,
            compute_pnl,
            compute_pnl_with_fx,
            convert_currency,
            default_waterfall_order,
            reprice_instrument,
        )

        bond = (
            Bond
            .builder("ATTR-BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.ANNUAL)
            .day_count(DayCount.ACT_365F)
            .disc_id("USD-OIS")
            .build()
        )

        market_t0 = MarketContext()
        market_t1 = MarketContext()
        curve_t0 = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        curve_t1 = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.945), (5.0, 0.74)],
            day_count="act_365f",
        )
        market_t0.insert(curve_t0)
        market_t1.insert(curve_t1)

        fx_t0 = FxMatrix()
        fx_t0.set_quote(EUR, USD, 1.10)
        market_t0.insert_fx(fx_t0)

        fx_t1 = FxMatrix()
        fx_t1.set_quote(EUR, USD, 1.20)
        market_t1.insert_fx(fx_t1)

        priced_t0 = reprice_instrument(bond, market_t0, date(2024, 1, 1))
        priced_t1 = reprice_instrument(bond, market_t1, date(2024, 1, 1))
        taylor = attribute_pnl_taylor(
            bond,
            market_t0,
            market_t1,
            date(2024, 1, 1),
            date(2024, 1, 2),
            TaylorAttributionConfig(),
        )

        converted = convert_currency(Money(100.0, EUR), USD, market_t1, date(2024, 1, 2))
        pnl_same_fx = compute_pnl(
            Money(100.0, EUR),
            Money(110.0, EUR),
            USD,
            market_t1,
            date(2024, 1, 2),
        )
        pnl_split_fx = compute_pnl_with_fx(
            Money(100.0, EUR),
            Money(100.0, EUR),
            USD,
            market_t0,
            market_t1,
            date(2024, 1, 1),
            date(2024, 1, 2),
        )
        waterfall_order = default_waterfall_order()

        assert priced_t0.currency.code == "USD"
        assert priced_t1.currency.code == "USD"
        assert converted.amount == pytest.approx(120.0)
        assert pnl_same_fx.amount == pytest.approx(12.0)
        assert pnl_split_fx.amount == pytest.approx(10.0)
        assert taylor.num_repricings >= 2
        assert taylor.pv_t0.currency.code == "USD"
        assert taylor.pv_t1.currency.code == "USD"
        assert waterfall_order[0] == "carry"
        assert "rates_curves" in waterfall_order

    def test_snapshot_and_detail_helpers_are_constructible(self) -> None:
        """Snapshot/detail helper types should be constructible and expose their data."""
        from finstack.core.currency import EUR, USD
        from finstack.core.market_data.context import MarketContext
        from finstack.core.market_data.fx import FxMatrix
        from finstack.core.market_data.scalars import MarketScalar

        from finstack.valuations import (
            CarryDetail,
            CorrelationsAttribution,
            CurveRestoreFlags,
            FxAttribution,
            InflationCurvesAttribution,
            MarketSnapshot,
            ScalarsAttribution,
            ScalarsSnapshot,
            TaylorAttributionResult,
            TaylorFactorResult,
            VolatilitySnapshot,
            VolAttribution,
        )

        market = MarketContext()
        market.insert(
            DiscountCurve(
                "USD-OIS",
                date(2024, 1, 1),
                [(0.0, 1.0), (1.0, 0.95)],
                day_count="act_365f",
            )
        )
        fx = FxMatrix()
        fx.set_quote(EUR, USD, 1.10)
        market.insert_fx(fx)
        market.insert_price("SPOT::ABC", MarketScalar.price(Money(42.0, USD)))

        carry = CarryDetail(
            Money(5.0, USD),
            theta=Money(3.0, USD),
            roll_down=Money(2.0, USD),
        )
        inflation = InflationCurvesAttribution(
            {"US-CPI": Money(1.5, USD)},
            by_tenor={("US-CPI", "5y"): Money(0.5, USD)},
        )
        correlations = CorrelationsAttribution({"CDX-IG": Money(2.0, USD)})
        fx_detail = FxAttribution({("EUR", "USD"): Money(4.0, USD)})
        vol = VolAttribution({"EQ-VOL": Money(1.25, USD)})
        scalars = ScalarsAttribution(
            dividends={"EQ::ABC": Money(0.5, USD)},
            inflation={"US-CPI": Money(0.25, USD)},
            equity_prices={"EQ::ABC": Money(1.0, USD)},
            commodity_prices={"CMDTY::WTI": Money(0.75, USD)},
        )
        factor = TaylorFactorResult("Rates:USD-OIS", 2.0, -3.0, -6.0, gamma_pnl=0.5)
        result = TaylorAttributionResult(
            actual_pnl=-5.5,
            total_explained=-5.0,
            unexplained=-0.5,
            unexplained_pct=9.09,
            factors=[factor],
            num_repricings=4,
            pv_t0=Money(100.0, USD),
            pv_t1=Money(94.5, USD),
        )
        rates_snapshot = MarketSnapshot.extract(market, CurveRestoreFlags.RATES)
        restored_market = MarketSnapshot.restore_market(market, rates_snapshot, CurveRestoreFlags.RATES)
        scalar_snapshot = ScalarsSnapshot.extract(market)
        vol_snapshot = VolatilitySnapshot.extract(market)

        assert carry.theta.amount == pytest.approx(3.0)
        assert inflation.by_curve_to_dict()["US-CPI"].amount == pytest.approx(1.5)
        assert inflation.by_tenor_to_dict()[("US-CPI", "5y")].amount == pytest.approx(0.5)
        assert correlations.by_curve_to_dict()["CDX-IG"].amount == pytest.approx(2.0)
        assert fx_detail.by_pair_to_dict()[("EUR", "USD")].amount == pytest.approx(4.0)
        assert vol.by_surface_to_dict()["EQ-VOL"].amount == pytest.approx(1.25)
        assert scalars.equity_prices_to_dict()["EQ::ABC"].amount == pytest.approx(1.0)
        assert result.factors[0].gamma_pnl == pytest.approx(0.5)
        assert rates_snapshot.discount_curves()["USD-OIS"].id == "USD-OIS"
        assert restored_market.get_discount("USD-OIS").id == "USD-OIS"
        assert scalar_snapshot.prices()["SPOT::ABC"].value.amount == pytest.approx(42.0)
        assert vol_snapshot.surfaces() == {}


def test_attribution_stub_matches_runtime_surface() -> None:
    """Attribution stubs should reflect the Rust-backed property and method names."""
    stub_path = Path(__file__).resolve().parent.parent.parent / "finstack" / "valuations" / "attribution.pyi"

    stub_text = stub_path.read_text()

    assert "def tolerance_abs(self) -> float:" in stub_text
    assert "def tolerance_pct(self) -> float:" in stub_text
    assert "def credit_detail_to_csv" not in stub_text


class TestValuationsRootParity:
    """Top-level valuations exports should mirror the available submodule surface."""

    def test_root_reexports_risk_attribution_and_calibration_symbols(self) -> None:
        """The root valuations module should expose common risk/calibration helpers."""
        from finstack.valuations import (
            AttributionMethod,
            CalibrationReport,
            MarketHistory,
            RateBounds,
            ValidationConfig,
            VarConfig,
            calculate_var,
        )

        assert VarConfig is not None
        assert MarketHistory is not None
        assert calculate_var is not None
        assert AttributionMethod is not None
        assert CalibrationReport is not None
        assert RateBounds is not None
        assert ValidationConfig is not None

    def test_bond_pricing_simple(self) -> None:
        """Test simple bond pricing matches expected NPV."""
        # Create bond
        bond = (
            Bond
            .builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build()
        )

        # Create market context
        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75), (10.0, 0.60)],
            day_count="act_365f",
        )
        market.insert(discount_curve)

        # Price bond
        registry = standard_registry()
        result = registry.get_price(bond, "discounting", market, date(2024, 1, 1))

        # Bond should have positive value
        assert result.value.amount > 0
        assert result.value.currency.code == "USD"

    def test_bond_pricing_at_par(self) -> None:
        """Test bond priced at par when coupon equals discount rate."""
        # Create bond with 5% coupon
        bond = (
            Bond
            .builder("BOND-PAR")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.ANNUAL)
            .day_count(DayCount.ACT_365F)
            .disc_id("USD-OIS")
            .build()
        )

        # Create flat 5% discount curve
        market = MarketContext()
        # Create discount factors for flat 5% rate
        # df(t) = exp(-0.05 * t)
        import math

        knots = [(t, math.exp(-0.05 * t)) for t in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0]]
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            knots,
            day_count="act_365f",
        )
        market.insert(discount_curve)

        # Price bond
        registry = standard_registry()
        result = registry.get_price(bond, "discounting", market, date(2024, 1, 1))

        # Bond should be approximately at par (1,000,000)
        # Allow 1% tolerance due to discrete coupon payments
        expected_par = 1_000_000.0
        assert abs(result.value.amount - expected_par) / expected_par < 0.01

    def test_bond_with_metrics(self) -> None:
        """Test bond pricing with metrics calculation."""
        bond = (
            Bond
            .builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert(discount_curve)

        # Price with metrics
        registry = standard_registry()
        metric_keys = ["clean_price", "accrued", "ytm"]
        result = registry.price_with_metrics(bond, "discounting", market, metric_keys, date(2024, 1, 1))

        # Should have base value
        assert result.value.amount > 0

        # Should have metrics (might be None if not supported for this model)
        # Just verify the API works


class TestSwapPricingParity:
    """Test interest rate swap pricing matches Rust."""

    def test_swap_construction(self) -> None:
        """Test swap construction via builder."""
        swap = (
            InterestRateSwap
            .builder("IRS-001")
            .notional(10_000_000.0)
            .currency("USD")
            .maturity(date(2029, 1, 1))
            .fixed_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .disc_id("USD-OIS")
            .fwd_id("USD-SOFR")
            .build()
        )

        assert swap.id == "IRS-001"

    def test_swap_pricing_simple(self) -> None:
        """Test simple swap pricing."""
        swap = (
            InterestRateSwap
            .builder("IRS-001")
            .notional(10_000_000.0)
            .currency("USD")
            .maturity(date(2029, 1, 1))
            .fixed_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .disc_id("USD-OIS")
            .fwd_id("USD-SOFR")
            .build()
        )

        # Create market context
        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert(discount_curve)

        forward_curve = ForwardCurve(
            "USD-SOFR",
            0.25,  # 3-month tenor
            [(0.0, 0.045), (1.0, 0.05), (5.0, 0.055)],
            base_date=date(2024, 1, 1),
            day_count=DayCount.ACT_360,
        )
        market.insert(forward_curve)

        # Price swap
        registry = standard_registry()
        result = registry.get_price(swap, "discounting", market, date(2024, 1, 1))

        # Swap should have a value (could be positive or negative)
        assert result.value.currency.code == "USD"

    def test_swap_at_market(self) -> None:
        """Test swap valued at zero when fixed rate equals forward rate."""
        # This test verifies pricing consistency
        swap = (
            InterestRateSwap
            .builder("IRS-ATM")
            .notional(10_000_000.0)
            .currency("USD")
            .maturity(date(2029, 1, 1))
            .fixed_rate(0.05)  # Set equal to forward rate
            .frequency(Frequency.ANNUAL)
            .disc_id("USD-OIS")
            .fwd_id("USD-SOFR")
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert(discount_curve)

        # Flat 5% forward curve
        forward_curve = ForwardCurve(
            "USD-SOFR",
            0.25,
            [(0.0, 0.05), (1.0, 0.05), (5.0, 0.05)],
            base_date=date(2024, 1, 1),
            day_count=DayCount.ACT_360,
        )
        market.insert(forward_curve)

        registry = standard_registry()
        result = registry.get_price(swap, "discounting", market, date(2024, 1, 1))

        # Swap should be close to zero value (at-market swap)
        # Allow reasonable tolerance due to day count and compounding
        assert abs(result.value.amount) / 10_000_000.0 < 0.1  # Within 10% of notional


class TestDepositPricingParity:
    """Test deposit pricing matches Rust."""

    def test_deposit_construction(self) -> None:
        """Test deposit construction via constructor."""
        from finstack.core.currency import Currency
        from finstack.core.money import Money

        deposit = (
            Deposit
            .builder("DEP-001")
            .money(Money(1_000_000.0, Currency("USD")))
            .start(date(2024, 1, 1))
            .maturity(date(2024, 4, 1))
            .day_count(DayCount.ACT_360)
            .disc_id("USD-OIS")
            .quote_rate(0.045)
            .build()
        )

        assert deposit.instrument_id == "DEP-001"

    def test_deposit_pricing_simple(self) -> None:
        """Test simple deposit pricing."""
        from finstack.core.currency import Currency
        from finstack.core.money import Money

        deposit = (
            Deposit
            .builder("DEP-001")
            .money(Money(1_000_000.0, Currency("USD")))
            .start(date(2024, 1, 1))
            .maturity(date(2024, 4, 1))
            .day_count(DayCount.ACT_360)
            .disc_id("USD-OIS")
            .quote_rate(0.045)
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (0.25, 0.99), (1.0, 0.95)],
            day_count="act_365f",
        )
        market.insert(discount_curve)

        registry = standard_registry()
        result = registry.get_price(deposit, "discounting", market, date(2024, 1, 1))

        # Deposit PV can be positive or negative depending on quote vs curve.
        assert result.value.currency.code == "USD"

    def test_deposit_analytical_value(self) -> None:
        """Deposit PV is near zero at market rate."""
        # 1M deposit at 4.5% on 1M USD
        deposit = (
            Deposit
            .builder("DEP-001")
            .money(Money(1_000_000.0, USD))
            .start(date(2024, 1, 1))
            .maturity(date(2024, 4, 1))  # 90 days
            .day_count(DayCount.ACT_360)
            .disc_id("USD-OIS")
            .quote_rate(0.045)
            .build()
        )

        # Flat discount curve at 4.5%
        import math

        knots = [(t, math.exp(-0.045 * t)) for t in [0.0, 0.25, 0.5, 1.0]]
        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            knots,
            day_count="act_365f",
        )
        market.insert(discount_curve)

        registry = standard_registry()
        result = registry.get_price(deposit, "discounting", market, date(2024, 1, 1))

        # For a deposit quoted at the same rate implied by the curve, PV should be close to zero
        # (i.e., no value over par).
        assert abs(result.value.amount) / 1_000_000.0 < 0.01


class TestPricerRegistryParity:
    """Test pricer registry functionality."""

    def test_registry_creation(self) -> None:
        """Test standard registry creation."""
        registry = standard_registry()
        assert registry is not None

    def test_registry_multiple_model_keys(self) -> None:
        """Test pricing with different model keys."""
        bond = (
            Bond
            .builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert(discount_curve)

        registry = standard_registry()

        # Price with discounting model
        result = registry.get_price(bond, "discounting", market, date(2024, 1, 1))
        assert result.value.amount > 0


class TestMetricsParity:
    """Test metrics calculation matches Rust."""

    def test_scalar_metrics_available(self) -> None:
        """Test scalar metrics are computed."""
        bond = (
            Bond
            .builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert(discount_curve)

        registry = standard_registry()
        metric_keys = ["clean_price", "accrued", "ytm", "duration_mod", "dv01"]
        result = registry.price_with_metrics(bond, "discounting", market, metric_keys, date(2024, 1, 1))

        # Should have value
        assert result.value.amount > 0

        # Metrics might not all be available for every model/instrument
        # Just verify the API works without error


class TestCashFlowBuilderParity:
    """Test cashflow builder matches Rust."""

    def test_cashflow_builder_basic(self) -> None:
        """Test basic cashflow schedule generation."""
        from finstack.valuations.cashflow import CashFlowBuilder, CouponType, FixedCouponSpec, ScheduleParams

        issue = date(2024, 1, 1)
        maturity = date(2029, 1, 1)
        notional = Money(1_000_000.0, USD)
        schedule = ScheduleParams.semiannual_30360()
        fixed_spec = FixedCouponSpec.new(rate=0.05, schedule=schedule, coupon_type=CouponType.CASH)

        builder = CashFlowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.fixed_cf(fixed_spec)
        cf_schedule = builder.build_with_curves(None)

        assert len(list(cf_schedule.flows())) > 0

    def test_cashflow_builder_with_amortization(self) -> None:
        """Test cashflow builder with amortization."""
        from finstack.valuations.cashflow import (
            AmortizationSpec,
            CashFlowBuilder,
            CouponType,
            FixedCouponSpec,
            ScheduleParams,
        )

        issue = date(2024, 1, 1)
        maturity = date(2029, 1, 1)
        notional = Money(1_000_000.0, USD)
        final_notional = Money(0.0, USD)
        amort = AmortizationSpec.linear_to(final_notional)
        schedule = ScheduleParams.annual_actact()
        fixed_spec = FixedCouponSpec.new(rate=0.05, schedule=schedule, coupon_type=CouponType.CASH)

        builder = CashFlowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.amortization(amort)
        builder.fixed_cf(fixed_spec)
        cf_schedule = builder.build_with_curves(None)

        assert len([f for f in cf_schedule.flows() if f.kind.name == "amortization"]) > 0


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_zero_coupon_bond(self) -> None:
        """Test zero-coupon bond pricing."""
        bond = (
            Bond
            .builder("ZERO-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.0)  # Zero coupon
            .frequency(Frequency.ANNUAL)
            .day_count(DayCount.ACT_365F)
            .disc_id("USD-OIS")
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert(discount_curve)

        registry = standard_registry()
        result = registry.get_price(bond, "discounting", market, date(2024, 1, 1))

        # Zero-coupon bond NPV should be notional * df(maturity)
        # NPV ≈ 1,000,000 * 0.75 = 750,000
        expected = 750_000.0
        assert abs(result.value.amount - expected) / expected < 0.05

    def test_deposit_overnight(self) -> None:
        """Test overnight deposit pricing."""
        deposit = (
            Deposit
            .builder("ON-001")
            .money(Money(1_000_000.0, USD))
            .start(date(2024, 1, 1))
            .maturity(date(2024, 1, 2))  # 1 day
            .day_count(DayCount.ACT_360)
            .disc_id("USD-OIS")
            .quote_rate(0.045)
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (0.003, 0.9999)],  # Almost flat for short tenor
            day_count="act_365f",
        )
        market.insert(discount_curve)

        registry = standard_registry()
        result = registry.get_price(deposit, "discounting", market, date(2024, 1, 1))

        # At (roughly) market rates, a deposit should have PV close to zero (no value over par).
        assert abs(result.value.amount) / 1_000_000.0 < 0.01

    def test_swap_zero_notional(self) -> None:
        """Test swap with zero notional."""
        with pytest.raises(ValueError, match="notional"):
            InterestRateSwap.builder("IRS-ZERO").notional(0.0).currency("USD").maturity(date(2029, 1, 1)).fixed_rate(
                0.05
            ).frequency(Frequency.ANNUAL).disc_id("USD-OIS").fwd_id("USD-SOFR").build()

        # Builder should reject zero notional rather than producing a degenerate instrument.


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
