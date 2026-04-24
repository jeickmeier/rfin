"""Tests for the capital-structure Python bindings.

Covers the spec wrappers (EcfSweepSpec, PikToggleSpec, WaterfallSpec) and the
ModelBuilder extensions that attach debt instruments + waterfall configuration
to a FinancialModelSpec.
"""

from __future__ import annotations

from datetime import date

from finstack.core.currency import Currency
from finstack.core.money import Money
import pytest

from finstack import statements


class TestEcfSweepSpec:
    def test_construct_with_minimum_args(self) -> None:
        ecf = statements.EcfSweepSpec(ebitda_node="ebitda", sweep_percentage=0.5)
        assert ecf.ebitda_node == "ebitda"
        assert ecf.sweep_percentage == 0.5
        assert ecf.target_instrument_id is None

    def test_construct_with_all_args(self) -> None:
        ecf = statements.EcfSweepSpec(
            ebitda_node="ebitda",
            sweep_percentage=0.75,
            taxes_node="taxes",
            capex_node="capex",
            working_capital_node="wc",
            cash_interest_node="cs.interest_expense_cash.total",
            target_instrument_id="TL-A",
        )
        assert ecf.target_instrument_id == "TL-A"

    def test_json_roundtrip(self) -> None:
        ecf = statements.EcfSweepSpec(ebitda_node="ebitda", sweep_percentage=0.5, target_instrument_id="X")
        restored = statements.EcfSweepSpec.from_json(ecf.to_json())
        assert restored.to_json() == ecf.to_json()


class TestPikToggleSpec:
    def test_defaults_min_periods_to_zero(self) -> None:
        pik = statements.PikToggleSpec(liquidity_metric="cash", threshold=1e6)
        assert pik.min_periods_in_pik == 0

    def test_json_roundtrip(self) -> None:
        pik = statements.PikToggleSpec(
            liquidity_metric="cash",
            threshold=1e6,
            target_instrument_ids=["A", "B"],
            min_periods_in_pik=3,
        )
        restored = statements.PikToggleSpec.from_json(pik.to_json())
        assert restored.to_json() == pik.to_json()


class TestWaterfallSpec:
    def test_default_priority_order(self) -> None:
        ws = statements.WaterfallSpec()
        assert ws.priority_of_payments == [
            "fees",
            "interest",
            "amortization",
            "sweep",
            "equity",
        ]
        assert not ws.has_ecf_sweep
        assert not ws.has_pik_toggle

    def test_custom_priority(self) -> None:
        ws = statements.WaterfallSpec(priority_of_payments=["interest", "sweep", "equity"])
        assert ws.priority_of_payments == ["interest", "sweep", "equity"]

    def test_rejects_unknown_priority_token(self) -> None:
        with pytest.raises(ValueError, match="unknown payment priority"):
            statements.WaterfallSpec(priority_of_payments=["nonexistent"])

    def test_with_ecf_and_pik(self) -> None:
        ecf = statements.EcfSweepSpec(ebitda_node="ebitda", sweep_percentage=0.5)
        pik = statements.PikToggleSpec(liquidity_metric="cash", threshold=1e6)
        ws = statements.WaterfallSpec(ecf_sweep=ecf, pik_toggle=pik)
        assert ws.has_ecf_sweep
        assert ws.has_pik_toggle

    def test_validate_accepts_standard_priority(self) -> None:
        ecf = statements.EcfSweepSpec(ebitda_node="ebitda", sweep_percentage=0.5)
        ws = statements.WaterfallSpec(ecf_sweep=ecf)
        ws.validate()  # should not raise

    def test_validate_rejects_sweep_after_equity(self) -> None:
        ecf = statements.EcfSweepSpec(ebitda_node="ebitda", sweep_percentage=0.5)
        ws = statements.WaterfallSpec(priority_of_payments=["equity", "sweep"], ecf_sweep=ecf)
        with pytest.raises(ValueError, match=r"Sweep.*must precede.*Equity"):
            ws.validate()

    def test_json_roundtrip(self) -> None:
        ecf = statements.EcfSweepSpec(ebitda_node="ebitda", sweep_percentage=0.25)
        ws = statements.WaterfallSpec(available_cash_node="free_cash_flow", ecf_sweep=ecf)
        restored = statements.WaterfallSpec.from_json(ws.to_json())
        assert restored.to_json() == ws.to_json()


class TestModelBuilderCapitalStructure:
    @pytest.fixture
    def usd(self) -> Currency:
        return Currency("USD")

    def test_add_bond_appears_in_model_json(self, usd: Currency) -> None:
        b = statements.ModelBuilder("deal")
        b.add_bond(
            id="BOND-001",
            notional=Money(10_000_000.0, usd),
            coupon_rate=0.05,
            issue_date=date(2025, 1, 15),
            maturity_date=date(2030, 1, 15),
            discount_curve_id="USD-OIS",
        )
        b.periods("2025Q1..Q4", None)
        b.value("revenue", [("2025Q1", 1.0)])
        model = b.build()
        js = model.to_json()
        assert '"capital_structure"' in js
        assert '"BOND-001"' in js

    def test_add_swap_appears_in_model_json(self, usd: Currency) -> None:
        b = statements.ModelBuilder("deal")
        b.add_swap(
            id="SWAP-001",
            notional=Money(5_000_000.0, usd),
            fixed_rate=0.04,
            start_date=date(2025, 1, 15),
            maturity_date=date(2030, 1, 15),
            discount_curve_id="USD-OIS",
            forward_curve_id="USD-SOFR-3M",
        )
        b.periods("2025Q1..Q4", None)
        b.value("x", [("2025Q1", 1.0)])
        model = b.build()
        assert '"SWAP-001"' in model.to_json()

    def test_add_custom_debt_passes_through_spec(self) -> None:
        b = statements.ModelBuilder("deal")
        b.add_custom_debt("TL-A", '{"notional": 25000000.0}')
        b.periods("2025Q1..Q1", None)
        b.value("x", [("2025Q1", 1.0)])
        model = b.build()
        assert '"TL-A"' in model.to_json()

    def test_reporting_currency_and_fx_policy(self, usd: Currency) -> None:
        b = statements.ModelBuilder("deal")
        b.reporting_currency(usd)
        b.fx_policy("period_end")
        b.periods("2025Q1..Q1", None)
        b.value("x", [("2025Q1", 1.0)])
        b.add_custom_debt("T", "{}")  # force capital_structure to serialize
        js = b.build().to_json()
        assert '"reporting_currency":"USD"' in js
        assert '"fx_policy":"period_end"' in js

    def test_fx_policy_rejects_unknown_variant(self) -> None:
        b = statements.ModelBuilder("deal")
        with pytest.raises(ValueError, match="invalid fx_policy"):
            b.fx_policy("bogus_policy")

    def test_waterfall_attaches_to_model(self) -> None:
        ecf = statements.EcfSweepSpec(ebitda_node="ebitda", sweep_percentage=0.5, target_instrument_id="TL-A")
        ws = statements.WaterfallSpec(ecf_sweep=ecf)
        b = statements.ModelBuilder("deal")
        b.add_custom_debt("TL-A", "{}")
        b.waterfall(ws)
        b.periods("2025Q1..Q1", None)
        b.value("x", [("2025Q1", 1.0)])
        js = b.build().to_json()
        assert '"waterfall"' in js
        assert '"ecf_sweep"' in js

    def test_capital_structure_methods_work_before_periods(self, usd: Currency) -> None:
        """Capital-structure methods must be state-agnostic (both NeedPeriods and Ready)."""
        b = statements.ModelBuilder("deal")
        # Attach capital structure BEFORE periods
        b.add_bond(
            id="B1",
            notional=Money(1_000_000.0, usd),
            coupon_rate=0.05,
            issue_date=date(2025, 1, 15),
            maturity_date=date(2030, 1, 15),
            discount_curve_id="USD-OIS",
        )
        b.reporting_currency(usd)
        # Then transition through periods and node addition
        b.periods("2025Q1..Q1", None)
        b.value("x", [("2025Q1", 1.0)])
        model = b.build()
        assert '"B1"' in model.to_json()
