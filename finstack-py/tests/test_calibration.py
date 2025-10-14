"""Test suite for calibration functionality in finstack.valuations.calibration."""

import datetime as dt

import pytest

import finstack
from finstack.core.dates.schedule import Frequency
from finstack.core.market_data import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.term_structures import DiscountCurve

cal = finstack.valuations.calibration


def _make_discount_curve(base_date: dt.date) -> DiscountCurve:
    knots = [
        (0.0, 1.0),
        (0.25, 0.9960),
        (0.5, 0.9920),
        (1.0, 0.9850),
        (2.0, 0.9700),
    ]
    return DiscountCurve("USD-OIS", base_date, knots)


def test_solver_kind_and_multicurve_helpers() -> None:
    """Test SolverKind and MultiCurveConfig functionality."""
    hybrid = cal.SolverKind.from_name("hybrid")
    assert hybrid == cal.SolverKind.HYBRID
    assert hybrid.name == "hybrid"
    assert hash(hybrid) == hash(cal.SolverKind.HYBRID)

    lm = cal.SolverKind.LEVENBERG_MARQUARDT
    assert lm.__repr__() == "SolverKind('levenberg_marquardt')"
    assert str(lm) == "levenberg_marquardt"

    mc = cal.MultiCurveConfig(False, False)
    assert not mc.calibrate_basis
    assert not mc.enforce_separation
    mc2 = mc.with_calibrate_basis(True).with_enforce_separation(True)
    assert mc2.calibrate_basis
    assert mc2.enforce_separation
    assert "MultiCurveConfig" in repr(mc2)


def test_calibration_config_builder_and_mutators() -> None:
    """Test CalibrationConfig builder and mutator methods."""
    base_cfg = cal.CalibrationConfig(
        1e-8,
        50,
        True,
        None,
        True,
        cal.SolverKind.BRENT,
        cal.MultiCurveConfig(False, False),
        {"ACME": "senior"},
    )

    assert pytest.approx(base_cfg.tolerance) == 1e-8
    assert base_cfg.max_iterations == 50
    assert base_cfg.use_parallel
    assert base_cfg.random_seed == 42
    assert base_cfg.verbose
    assert base_cfg.solver_kind.name == "brent"
    assert base_cfg.multi_curve_config.calibrate_basis is False
    assert base_cfg.entity_seniority["ACME"] == "senior"

    tuned = (
        base_cfg.with_tolerance(1e-6)
        .with_max_iterations(10)
        .with_parallel(False)
        .with_random_seed(1234)
        .with_verbose(False)
        .with_solver_kind(cal.SolverKind.NEWTON)
        .with_multi_curve_config(cal.MultiCurveConfig(True, True))
    )
    tuned = tuned.with_entity_seniority({"ACME": "senior_secured", "FOO": "junior"})
    tuned = tuned.with_random_seed(None)
    assert tuned.tolerance == 1e-6
    assert tuned.max_iterations == 10
    assert not tuned.use_parallel
    assert tuned.random_seed is None
    assert not tuned.verbose
    assert tuned.solver_kind == cal.SolverKind.NEWTON
    assert tuned.multi_curve_config.calibrate_basis
    assert tuned.entity_seniority["FOO"] == "junior"

    cfg_repr = tuned.__repr__()
    assert "CalibrationConfig" in cfg_repr

    standard_cfg = cal.CalibrationConfig.multi_curve()
    assert standard_cfg.multi_curve_config.enforce_separation


def test_quote_constructors_cover_all_variants() -> None:
    """Test that quote constructors cover all quote variants."""
    future_specs = cal.FutureSpecs(1_000_000.0, 1_000_000.0, 3, "ACT/360", 0.1)
    assert future_specs.day_count.name.replace("_", "/").upper() == "ACT/360"
    assert future_specs.convexity_adjustment == pytest.approx(0.1)
    assert "FutureSpecs" in repr(future_specs)

    depo = cal.RatesQuote.deposit(dt.date(2024, 4, 1), 0.02, "ACT/360")
    fra = cal.RatesQuote.fra(dt.date(2024, 5, 1), dt.date(2024, 8, 1), 0.021, "ACT/360")
    fut = cal.RatesQuote.future(dt.date(2024, 6, 1), 99.1, future_specs)
    swap = cal.RatesQuote.swap(
        dt.date(2025, 6, 1),
        0.0225,
        Frequency.QUARTERLY,
        Frequency.SEMI_ANNUAL,
        "ACT/360",
        "30/360",
        "USD-SOFR",
    )
    basis = cal.RatesQuote.basis_swap(
        dt.date(2026, 6, 1),
        "USD-SOFR-3M",
        "USD-SOFR-6M",
        15.0,
        Frequency.QUARTERLY,
        Frequency.SEMI_ANNUAL,
        "ACT/360",
        "ACT/360",
        "USD",
    )

    assert depo.kind == "deposit"
    assert "RatesQuote" in repr(depo)
    assert fra.kind == "fra"
    assert fut.kind == "future"
    assert swap.kind == "swap"
    assert basis.kind == "basis_swap"

    cds = cal.CreditQuote.cds("ACME", dt.date(2026, 6, 1), 120.0, 0.4, "USD")
    cds_upfront = cal.CreditQuote.cds_upfront("ACME", dt.date(2026, 6, 1), 2.5, 500.0, 0.35, "USD")
    tranche = cal.CreditQuote.cds_tranche("CDX.NA.IG", 0.0, 3.0, dt.date(2027, 6, 1), 10.0, 100.0)

    assert cds.kind == "cds"
    assert cds_upfront.kind == "cds_upfront"
    assert tranche.kind == "cds_tranche"

    option_vol = cal.VolQuote.option_vol("SPX", dt.date(2024, 12, 20), 4200.0, 0.25, "Call")
    swaption_vol = cal.VolQuote.swaption_vol(dt.date(2024, 9, 1), dt.date(2029, 9, 1), 0.02, 0.3, "ATM")
    assert option_vol.kind == "option"
    assert swaption_vol.kind == "swaption"

    zc_inflation = cal.InflationQuote.inflation_swap(dt.date(2027, 1, 1), 0.015, "CPI-US")
    yoy_inflation = cal.InflationQuote.yoy_inflation_swap(dt.date(2027, 1, 1), 0.0175, "CPI-US", Frequency.ANNUAL)
    assert zc_inflation.kind == "inflation_swap"
    assert yoy_inflation.kind == "yoy_inflation_swap"

    mq_rates = depo.to_market_quote()
    mq_credit = cds.to_market_quote()
    mq_vol = option_vol.to_market_quote()
    mq_infl = yoy_inflation.to_market_quote()

    assert mq_rates.kind == "rates"
    assert mq_credit.kind == "credit"
    assert mq_vol.kind == "vol"
    assert mq_infl.kind == "inflation"

    assert "MarketQuote" in repr(cal.MarketQuote.from_rates(depo))
    assert "MarketQuote" in repr(cal.MarketQuote.from_credit(cds))
    assert "MarketQuote" in repr(cal.MarketQuote.from_vol(option_vol))
    assert "MarketQuote" in repr(cal.MarketQuote.from_inflation(yoy_inflation))


def test_simple_calibration_flow_and_report() -> None:
    """Test simple calibration flow and report generation."""
    base_date = dt.date(2024, 1, 2)

    custom_cfg = (
        cal.CalibrationConfig.multi_curve()
        .with_solver_kind(cal.SolverKind.HYBRID)
        .with_max_iterations(20)
        .with_random_seed(7)
    )
    calibration = cal.SimpleCalibration(base_date, "USD", config=custom_cfg)

    calibration.set_multi_curve_config(cal.MultiCurveConfig(True, True))
    calibration.add_entity_seniority("ACME", "senior")
    calibration.set_entity_seniority({"BAR": "subordinated"})
    calibration.add_entity_seniority("FOO", "junior")

    returned_cfg = calibration.config
    assert returned_cfg.max_iterations == 20
    assert returned_cfg.entity_seniority["FOO"] == "junior"

    quotes = [cal.RatesQuote.deposit(base_date.replace(year=2024, month=2, day=2), 0.02, "ACT/360").to_market_quote()]

    market_ctx, report = calibration.calibrate(quotes)

    stats = market_ctx.stats()
    assert stats["total_curves"] >= 0

    assert report.success
    assert report.iterations >= 0
    assert "calibration" in report.convergence_reason.lower()
    assert isinstance(report.residuals, dict)
    assert isinstance(report.metadata, dict)
    assert report.objective_value >= 0.0

    report_dict = report.to_dict()
    assert report_dict["success"] == report.success
    assert isinstance(report_dict["residuals"], dict)


def test_discount_curve_calibrator_basic() -> None:
    """Test basic discount curve calibrator functionality."""
    base_date = dt.date(2024, 1, 2)
    quotes = [
        cal.RatesQuote.deposit(base_date + dt.timedelta(days=30), 0.02, "ACT/360"),
        cal.RatesQuote.swap(
            base_date + dt.timedelta(days=365),
            0.024,
            Frequency.ANNUAL,
            Frequency.SEMI_ANNUAL,
            "30/360",
            "ACT/360",
            "USD-SOFR",
        ),
    ]
    calibrator = cal.DiscountCurveCalibrator("USD-OIS", base_date, "USD")
    with pytest.raises(RuntimeError):
        calibrator.calibrate(quotes)


def test_forward_curve_calibrator_with_context() -> None:
    """Test forward curve calibrator with market context."""
    base_date = dt.date(2024, 1, 2)
    market = MarketContext()
    market.insert_discount(_make_discount_curve(base_date))
    fra = cal.RatesQuote.fra(
        base_date + dt.timedelta(days=90),
        base_date + dt.timedelta(days=180),
        0.031,
        "ACT/360",
    )
    calibrator = cal.ForwardCurveCalibrator(
        "USD-SOFR-3M",
        0.25,
        base_date,
        "USD",
        "USD-OIS",
    )
    with pytest.raises(RuntimeError):
        calibrator.calibrate([fra], market)


def test_hazard_curve_calibrator_basic() -> None:
    """Test basic hazard curve calibrator functionality."""
    base_date = dt.date(2024, 1, 2)
    market = MarketContext()
    market.insert_discount(_make_discount_curve(base_date))
    cds = cal.CreditQuote.cds("ACME", base_date + dt.timedelta(days=365), 120.0, 0.4, "USD")
    calibrator = cal.HazardCurveCalibrator(
        "ACME",
        "senior",
        0.4,
        base_date,
        "USD",
        "USD-OIS",
    )
    curve, report = calibrator.calibrate([cds], market)
    assert report.success
    assert curve.recovery_rate == pytest.approx(0.4)


def test_inflation_curve_calibrator_handles_empty_quotes() -> None:
    """Test that inflation curve calibrator handles empty quotes gracefully."""
    base_date = dt.date(2024, 1, 2)
    market = MarketContext()
    market.insert_discount(_make_discount_curve(base_date))
    calibrator = cal.InflationCurveCalibrator(
        "US-CPI",
        base_date,
        "USD",
        300.0,
        "USD-OIS",
    )
    curve, report = calibrator.calibrate([], market)
    assert report.success
    assert curve.cpi(0.25) == pytest.approx(300.0, rel=1e-6)


def test_vol_surface_calibrator_builds_surface() -> None:
    """Test that vol surface calibrator builds volatility surface correctly."""
    base_date = dt.date(2024, 1, 2)
    market = MarketContext()
    market.insert_discount(_make_discount_curve(base_date))
    market.insert_price("ACME", MarketScalar.unitless(100.0))
    market.insert_price("ACME-DIVYIELD", MarketScalar.unitless(0.01))
    quotes = [
        cal.VolQuote.option_vol("ACME", base_date + dt.timedelta(days=180), 90.0, 0.23, "Call"),
        cal.VolQuote.option_vol("ACME", base_date + dt.timedelta(days=180), 100.0, 0.20, "Call"),
        cal.VolQuote.option_vol("ACME", base_date + dt.timedelta(days=180), 110.0, 0.22, "Call"),
        cal.VolQuote.option_vol("ACME", base_date + dt.timedelta(days=365), 90.0, 0.24, "Call"),
        cal.VolQuote.option_vol("ACME", base_date + dt.timedelta(days=365), 100.0, 0.21, "Call"),
        cal.VolQuote.option_vol("ACME", base_date + dt.timedelta(days=365), 110.0, 0.23, "Call"),
    ]
    calibrator = (
        cal.VolSurfaceCalibrator("ACME-VOL", 1.0, [0.5, 1.0], [90.0, 100.0, 110.0])
        .with_base_date(base_date)
        .with_base_currency("USD")
        .with_discount_id("USD-OIS")
    )
    surface, report = calibrator.calibrate(quotes, market)
    assert report.success
    assert surface.value(0.5, 100.0) > 0.0


def test_validate_discount_curve_helpers() -> None:
    """Test discount curve validation helpers."""
    base_date = dt.date(2024, 1, 2)
    good_curve = _make_discount_curve(base_date)
    cal.validate_discount_curve(good_curve)

    bad_curve = DiscountCurve("BAD", base_date, [(0.0, 1.0), (0.5, 1.01)])
    with pytest.raises(ValueError, match="decreasing"):
        cal.validate_discount_curve(bad_curve)
