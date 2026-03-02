from datetime import date

from finstack.valuations.instruments import MertonAssetDynamics, MertonBarrierType, MertonModel


def test_merton_dd() -> None:
    m = MertonModel(asset_value=100, asset_vol=0.20, debt_barrier=80, risk_free_rate=0.05)
    dd = m.distance_to_default(1.0)
    assert abs(dd - 1.2657) < 0.01


def test_merton_pd() -> None:
    m = MertonModel(asset_value=100, asset_vol=0.20, debt_barrier=80, risk_free_rate=0.05)
    pd = m.default_probability(1.0)
    assert 0.0 < pd < 1.0


def test_implied_spread() -> None:
    m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)
    spread = m.implied_spread(5.0, 0.40)
    assert spread > 0.0


def test_from_equity_roundtrip() -> None:
    m = MertonModel.from_equity(equity_value=25.0, equity_vol=0.50, total_debt=80.0, risk_free_rate=0.05)
    assert m.distance_to_default(1.0) > 0


def test_from_cds_spread() -> None:
    m = MertonModel.from_cds_spread(
        cds_spread_bp=200.0, recovery=0.40, total_debt=80.0, risk_free_rate=0.04, maturity=5.0, asset_value=100.0
    )
    assert m.asset_vol > 0


def test_credit_grades() -> None:
    m = MertonModel.credit_grades(
        equity_value=25.0,
        equity_vol=0.50,
        total_debt=80.0,
        risk_free_rate=0.04,
        barrier_uncertainty=0.30,
        mean_recovery=0.40,
    )
    assert m.asset_value > 0


def test_to_hazard_curve() -> None:
    m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)
    hc = m.to_hazard_curve("TEST", date(2026, 3, 1), recovery=0.40)
    assert hc is not None


def test_implied_equity() -> None:
    m = MertonModel(asset_value=100, asset_vol=0.20, debt_barrier=80, risk_free_rate=0.05)
    equity, eq_vol = m.implied_equity(1.0)
    assert equity > 0
    assert eq_vol > 0


def test_properties() -> None:
    m = MertonModel(asset_value=100, asset_vol=0.20, debt_barrier=80, risk_free_rate=0.05)
    assert m.asset_value == 100.0
    assert m.asset_vol == 0.20
    assert m.debt_barrier == 80.0
    assert m.risk_free_rate == 0.05


def test_asset_dynamics_enum() -> None:
    gbm = MertonAssetDynamics.GEOMETRIC_BROWNIAN
    assert "geometric" in str(gbm).lower()
    jd = MertonAssetDynamics.jump_diffusion(0.5, -0.05, 0.10)
    assert "jump" in str(jd).lower()


def test_barrier_type_enum() -> None:
    term = MertonBarrierType.TERMINAL
    assert "terminal" in str(term).lower()
    fp = MertonBarrierType.first_passage(0.05)
    assert "first" in str(fp).lower() or "passage" in str(fp).lower()


def test_repr() -> None:
    m = MertonModel(asset_value=100, asset_vol=0.20, debt_barrier=80, risk_free_rate=0.05)
    r = repr(m)
    assert "MertonModel" in r


def test_with_dynamics() -> None:
    jd = MertonAssetDynamics.jump_diffusion(0.5, -0.05, 0.10)
    fp = MertonBarrierType.first_passage(0.02)
    m = MertonModel(asset_value=100, asset_vol=0.20, debt_barrier=80, risk_free_rate=0.05, dynamics=jd, barrier_type=fp)
    pd = m.default_probability(5.0)
    assert 0.0 < pd < 1.0
