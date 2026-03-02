import datetime as dt

from finstack.core.dates import DayCountContextState
from finstack.core.market_data import PriceCurve, VolatilityIndexCurve
import pytest


def test_daycount_context_state_roundtrips_bus_basis() -> None:
    state = DayCountContextState(bus_basis=252)
    ctx = state.to_context()

    assert ctx.bus_basis == 252
    assert ctx.to_state().bus_basis == 252


def test_volatility_index_curve_default_extrapolation_matches_flat_forward() -> None:
    base_date = dt.date(2024, 1, 2)
    knots = [(0.0, 20.0), (1.0, 22.0)]

    default_curve = VolatilityIndexCurve("VIX", base_date, knots)
    explicit_curve = VolatilityIndexCurve("VIX", base_date, knots, extrapolation="flat_forward")

    t_eval = 2.0
    assert default_curve.forward_level(t_eval) == pytest.approx(explicit_curve.forward_level(t_eval))


def test_price_curve_default_extrapolation_matches_flat_zero() -> None:
    base_date = dt.date(2024, 1, 2)
    knots = [(0.0, 100.0), (1.0, 110.0)]

    default_curve = PriceCurve("CL", base_date, knots)
    explicit_curve = PriceCurve("CL", base_date, knots, extrapolation="flat_zero")

    t_eval = 2.0
    assert default_curve.price(t_eval) == pytest.approx(explicit_curve.price(t_eval))
