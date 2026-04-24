"""Test that domain subpackages are importable with expected exports."""

import json
from pathlib import Path

from finstack.core.market_data import MarketContext

from finstack.portfolio import aggregate_full_cashflows


class TestCoreNamespace:
    """Verify the core subpackage and its nested modules."""

    def test_core_submodules(self) -> None:
        """All core submodules should be importable from finstack.core."""
        from finstack.core import config, currency, dates, market_data, math, money, types  # noqa: F401

    def test_core_currency_exports(self) -> None:
        """Currency module should export Currency class."""
        from finstack.core.currency import Currency

        assert callable(Currency)

    def test_core_money_exports(self) -> None:
        """Money module should export Money class."""
        from finstack.core.money import Money

        assert callable(Money)

    def test_core_dates_exports(self) -> None:
        """Dates module should export day-count and period types."""
        from finstack.core.dates import (  # noqa: F401
            DayCount,
            DayCountContext,
            PeriodId,
            build_periods,
        )

    def test_core_math_linalg_exports(self) -> None:
        """Math.linalg should export Cholesky functions and constants."""
        from finstack.core.math.linalg import (  # noqa: F401
            DIAGONAL_TOLERANCE,
            SINGULAR_THRESHOLD,
            SYMMETRY_TOLERANCE,
            CholeskyError,
            cholesky_decomposition,
            cholesky_solve,
        )

    def test_core_market_data_exports(self) -> None:
        """Market data module should export curve and FX types."""
        from finstack.core.market_data import (  # noqa: F401
            DiscountCurve,
            ForwardCurve,
            FxConversionPolicy,
            FxMatrix,
            MarketContext,
        )


class TestAnalyticsNamespace:
    """Verify the analytics subpackage."""

    def test_analytics_exports(self) -> None:
        """Analytics should export Performance class and standalone functions."""
        from finstack.analytics import (  # noqa: F401
            Performance,
            comp_sum,
            comp_total,
            expected_shortfall,
            max_drawdown,
            mean_return,
            sharpe,
            simple_returns,
            sortino,
            to_drawdown_series,
            value_at_risk,
            volatility,
        )

    def test_analytics_does_not_export_legacy_rolling_values(self) -> None:
        """Legacy rolling `_values` helpers should not remain on the public namespace."""
        from finstack import analytics

        assert not hasattr(analytics, "rolling_sharpe_values")
        assert not hasattr(analytics, "rolling_sortino_values")
        assert not hasattr(analytics, "rolling_volatility_values")


class TestCashflowsNamespace:
    """Verify the cashflows subpackage."""

    def test_cashflows_exports(self) -> None:
        """Cashflows should expose the JSON bridge functions."""
        from finstack.cashflows import (  # noqa: F401
            accrued_interest,
            bond_from_cashflows,
            build_cashflow_schedule,
            dated_flows,
            validate_cashflow_schedule,
        )


class TestCorrelationNamespace:
    """Verify the correlation subpackage nested under valuations."""

    def test_correlation_exports(self) -> None:
        """Correlation should export copula, recovery, factor, and Bernoulli types."""
        from finstack.valuations.correlation import (  # noqa: F401
            Copula,
            CopulaSpec,
            CorrelatedBernoulli,
            FactorModel,
            FactorSpec,
            MultiFactorModel,
            RecoveryModel,
            RecoverySpec,
            SingleFactorModel,
            TwoFactorModel,
            cholesky_decompose,
            correlation_bounds,
            joint_probabilities,
            validate_correlation_matrix,
        )

    def test_correlation_accessible_via_valuations(self) -> None:
        """``finstack.valuations.correlation`` is importable as a submodule attribute."""
        from finstack import valuations

        assert valuations.correlation.CopulaSpec is not None


class TestMonteCarloNamespace:
    """Verify the monte_carlo subpackage."""

    def test_monte_carlo_exports(self) -> None:
        """Monte Carlo should export engine, pricer, and result types."""
        from finstack.monte_carlo import (  # noqa: F401
            EuropeanPricer,
            LsmcPricer,
            McEngine,
            MonteCarloResult,
            PathDependentPricer,
            price_european_call,
            price_european_put,
        )


class TestMarginNamespace:
    """Verify the margin subpackage."""

    def test_margin_exports(self) -> None:
        """Margin should export IM/VM types and CSA spec."""
        from finstack.margin import (  # noqa: F401
            CsaSpec,
            ImMethodology,
            ImResult,
            NettingSetId,
            VmCalculator,
            VmResult,
        )


class TestPortfolioNamespace:
    """Verify the portfolio subpackage."""

    def test_portfolio_exports(self) -> None:
        """Portfolio should export parsing, building, metric functions, and typed wrappers."""
        from finstack.portfolio import (  # noqa: F401
            Portfolio,
            PortfolioResult,
            PortfolioValuation,
            aggregate_full_cashflows,
            aggregate_metrics,
            build_portfolio_from_spec,
            parse_portfolio_spec,
            portfolio_result_get_metric,
            portfolio_result_total_value,
        )

    def test_portfolio_full_cashflows_empty_portfolio(self) -> None:
        """Full cashflow ladder should be exposed and preserve the rich empty shape."""
        spec_json = json.dumps({
            "id": "test_portfolio",
            "name": "Test",
            "base_ccy": "USD",
            "as_of": "2024-01-15",
            "entities": {},
            "positions": [],
        })
        cashflows = aggregate_full_cashflows(spec_json, MarketContext())
        assert len(cashflows) == 0
        assert cashflows.num_positions() == 0
        assert cashflows.num_issues() == 0

        result = json.loads(cashflows.to_json())
        assert result["events"] == []
        assert result["by_position"] == {}
        assert result["by_date"] == {}
        assert result["position_summaries"] == {}
        assert result["issues"] == []


class TestScenariosNamespace:
    """Verify the scenarios subpackage."""

    def test_scenarios_exports(self) -> None:
        """Scenarios should export spec builders and template functions."""
        from finstack.scenarios import (  # noqa: F401
            build_from_template,
            build_scenario_spec,
            build_template_component,
            compose_scenarios,
            list_builtin_template_metadata,
            list_builtin_templates,
            list_template_components,
            parse_scenario_spec,
            validate_scenario_spec,
        )


class TestStatementsNamespace:
    """Verify the statements subpackage."""

    def test_statements_exports(self) -> None:
        """Statements should export model spec and enum types."""
        from finstack.statements import (  # noqa: F401
            FinancialModelSpec,
            ForecastMethod,
            NodeId,
            NodeType,
            NumericMode,
        )


class TestStatementsAnalyticsNamespace:
    """Verify the statements_analytics subpackage."""

    def test_statements_analytics_exports(self) -> None:
        """Statements analytics should export sensitivity and variance functions."""
        from finstack.statements_analytics import (  # noqa: F401
            backtest_forecast,
            evaluate_scenario_set,
            run_sensitivity,
            run_variance,
        )


class TestValuationsNamespace:
    """Verify the valuations subpackage."""

    def test_valuations_exports(self) -> None:
        """Valuations should export ValuationResult and validation function."""
        from finstack.valuations import (  # noqa: F401
            ValuationResult,
            bs_cos_price,
            merton_jump_cos_price,
            validate_instrument_json,
            vg_cos_price,
        )

    def test_valuations_stub_exports_fourier_pricers(self) -> None:
        """Valuations stubs should declare the runtime Fourier pricing exports."""
        stub_path = Path(__file__).parents[1] / "finstack" / "valuations" / "__init__.pyi"
        stub = stub_path.read_text()
        for name in ("bs_cos_price", "vg_cos_price", "merton_jump_cos_price"):
            assert f'"{name}"' in stub
            assert f"def {name}(" in stub

    def test_valuations_instruments_namespace_exports(self) -> None:
        """Instrument helpers should be available from valuations.instruments."""
        from finstack.valuations import instruments

        assert hasattr(instruments, "validate_instrument_json")
        assert hasattr(instruments, "price_instrument")
        assert hasattr(instruments, "price_instrument_with_metrics")
        assert hasattr(instruments, "list_standard_metrics")
