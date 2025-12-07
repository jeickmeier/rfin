"""Comprehensive parity tests for statements Python bindings.

This test suite verifies that the Python bindings for finstack-statements
provide full parity with the Rust crate across all major features.
"""

from datetime import date

import pytest

from finstack.core.dates import PeriodId
from finstack.core.money import Money
from finstack.statements import (
    Alignment,
    AmountOrScalar,
    CreditAssessmentReport,
    DebtSummaryReport,
    DependencyGraph,
    DependencyTracer,
    Evaluator,
    ExtensionRegistry,
    FinancialModelSpec,
    ForecastSpec,
    FormulaExplainer,
    ModelBuilder,
    ParameterSpec,
    PLSummaryReport,
    Registry,
    Results,
    SensitivityAnalyzer,
    SensitivityConfig,
    SensitivityMode,
    TableBuilder,
    TornadoEntry,
    render_tree_ascii,
    render_tree_detailed,
)


class TestDataFrameExport:
    """Test DataFrame export methods."""

    def test_to_polars_long(self) -> None:
        """Test long-format DataFrame export."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100000.0)),
            ],
        )
        builder.value(
            "cogs",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(60000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(60000.0)),
            ],
        )
        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        df = results.to_polars_long()
        assert df is not None
        assert len(df) > 0

    def test_to_polars_wide(self) -> None:
        """Test wide-format DataFrame export."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100000.0)),
            ],
        )
        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        df = results.to_polars_wide()
        assert df is not None
        assert len(df) > 0

    def test_to_polars_long_filtered(self) -> None:
        """Test filtered long-format DataFrame export."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100000.0)),
            ],
        )
        builder.value(
            "cogs",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(60000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(60000.0)),
            ],
        )
        builder.compute("gross_profit", "revenue - cogs")
        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        df = results.to_polars_long_filtered(["revenue", "cogs"])
        assert df is not None
        assert len(df) > 0


class TestCapitalStructure:
    """Test capital structure builder methods."""

    def test_add_bond(self) -> None:
        """Test bond instrument creation."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q4", None)

        from finstack.core.currency import USD

        notional = Money(10_000_000.0, USD)
        issue_date = date(2025, 1, 1)
        maturity_date = date(2030, 1, 1)

        builder.add_bond("BOND-001", notional, 0.05, issue_date, maturity_date, "USD-OIS")
        model = builder.build()

        assert model.capital_structure is not None
        assert len(model.capital_structure.debt_instruments) == 1

    def test_add_swap(self) -> None:
        """Test interest rate swap creation."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q4", None)

        from finstack.core.currency import USD

        notional = Money(5_000_000.0, USD)
        start_date = date(2025, 1, 1)
        maturity_date = date(2030, 1, 1)

        builder.add_swap(
            "SWAP-001",
            notional,
            0.04,
            start_date,
            maturity_date,
            "USD-OIS",
            "USD-SOFR-3M",
        )
        model = builder.build()

        assert model.capital_structure is not None
        assert len(model.capital_structure.debt_instruments) == 1

    def test_add_custom_debt(self) -> None:
        """Test custom debt instrument via JSON spec."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q4", None)

        spec = {
            "type": "term_loan",
            "notional": 10_000_000.0,
            "currency": "USD",
        }
        builder.add_custom_debt("TL-A", spec)
        model = builder.build()

        assert model.capital_structure is not None
        assert len(model.capital_structure.debt_instruments) == 1


class TestMetricsIntegration:
    """Test metrics integration methods."""

    def test_with_builtin_metrics(self) -> None:
        """Test loading built-in metrics."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100000.0)),
            ],
        )
        builder.value("cogs", [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(40000.0))])
        builder.with_builtin_metrics()
        model = builder.build()

        # Should have the base metrics plus builtin metrics
        assert len(model.nodes) >= 2

    def test_add_metric(self) -> None:
        """Test adding a specific metric from built-in registry."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100000.0)),
            ],
        )
        builder.value("cogs", [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(40000.0))])
        builder.add_metric("fin.gross_margin")
        model = builder.build()

        # Should have base metrics plus gross_margin
        assert "fin.gross_margin" in model.nodes

    def test_add_metric_from_registry(self) -> None:
        """Test adding metric from custom registry."""
        registry = Registry.new()
        registry.load_builtins()

        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100000.0)),
            ],
        )
        builder.value("cogs", [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(40000.0))])
        builder.add_metric_from_registry("fin.gross_margin", registry)
        model = builder.build()

        assert "fin.gross_margin" in model.nodes


class TestValueMethods:
    """Test strongly-typed value methods."""

    def test_value_money(self) -> None:
        """Test value_money method."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)

        from finstack.core.currency import USD

        money_val = Money(100000.0, USD)
        builder.value_money("revenue", [(PeriodId.quarter(2025, 1), money_val)])
        model = builder.build()

        assert "revenue" in model.nodes

    def test_value_scalar(self) -> None:
        """Test value_scalar method."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.value_scalar("margin_pct", [(PeriodId.quarter(2025, 1), 0.35)])
        model = builder.build()

        assert "margin_pct" in model.nodes


class TestMixedNodeBuilder:
    """Test mixed node builder."""

    def test_mixed_node_with_values_and_forecast(self) -> None:
        """Test creating a mixed node with values and forecast."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q4", "2025Q1")  # Q1 is actuals, Q2-Q4 use forecast

        # Create mixed node
        mixed = builder.mixed("revenue")
        mixed.values([(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0))])
        mixed.forecast(ForecastSpec.forward_fill())
        mixed.name("Revenue (Actual + Forecast)")
        builder = mixed.finish()

        model = builder.build()
        assert "revenue" in model.nodes
        # Check that node_type is MIXED by comparing the inner enum value
        assert str(model.nodes["revenue"].node_type) == "Mixed"

    def test_mixed_node_with_formula(self) -> None:
        """Test mixed node with fallback formula."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q4", "2025Q1")

        mixed = builder.mixed("revenue")
        mixed.values([(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0))])
        mixed.formula("100000")
        builder = mixed.finish()

        model = builder.build()
        assert "revenue" in model.nodes


class TestEvaluatorEnhancements:
    """Test evaluator enhancements."""

    def test_dependency_graph_from_model(self) -> None:
        """Test DependencyGraph.from_model()."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("revenue", "100000")
        builder.compute("cogs", "revenue * 0.4")
        builder.compute("gross_profit", "revenue - cogs")
        model = builder.build()

        graph = DependencyGraph.from_model(model)
        assert graph is not None

        # Should have topological order
        order = graph.topological_order()
        assert len(order) == 3
        assert "revenue" in order
        assert "cogs" in order
        assert "gross_profit" in order

    def test_dependency_graph_dependencies(self) -> None:
        """Test DependencyGraph.dependencies()."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("a", "10")
        builder.compute("b", "a * 2")
        builder.compute("c", "a + b")
        model = builder.build()

        graph = DependencyGraph.from_model(model)

        # Check dependencies
        c_deps = graph.dependencies("c")
        assert len(c_deps) == 2
        assert "a" in c_deps
        assert "b" in c_deps

    def test_dependency_graph_has_cycle(self) -> None:
        """Test cycle detection."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("a", "10")
        builder.compute("b", "a * 2")
        model = builder.build()

        graph = DependencyGraph.from_model(model)
        assert not graph.has_cycle()


class TestAnalysisModule:
    """Test sensitivity analysis module."""

    def test_sensitivity_analysis_diagonal(self) -> None:
        """Test diagonal sensitivity analysis."""
        builder = ModelBuilder.new("sensitivity_test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100000.0)),
            ],
        )
        builder.compute("cogs", "revenue * 0.6")
        builder.compute("gross_profit", "revenue - cogs")
        model = builder.build()

        analyzer = SensitivityAnalyzer(model)
        config = SensitivityConfig(SensitivityMode.DIAGONAL)

        param = ParameterSpec.with_percentages("revenue", PeriodId.quarter(2025, 1), 100000.0, [-10.0, 0.0, 10.0])
        config.add_parameter(param)
        config.add_target_metric("gross_profit")

        result = analyzer.run(config)
        assert len(result) == 3  # 3 perturbations

    def test_parameter_spec_with_percentages(self) -> None:
        """Test ParameterSpec.with_percentages()."""
        param = ParameterSpec.with_percentages("revenue", PeriodId.quarter(2025, 1), 100000.0, [-10.0, 0.0, 10.0])

        assert param.node_id == "revenue"
        assert param.base_value == pytest.approx(100000.0)
        assert len(param.perturbations) == 3

    def test_tornado_entry_creation(self) -> None:
        """Test TornadoEntry creation."""
        entry = TornadoEntry("revenue", -5000.0, 5000.0)
        assert entry.parameter_id == "revenue"
        assert entry.downside_impact == pytest.approx(-5000.0)
        assert entry.upside_impact == pytest.approx(5000.0)
        assert entry.swing == pytest.approx(10000.0)


class TestExplainModule:
    """Test explain module."""

    def test_dependency_tracer_direct_dependencies(self) -> None:
        """Test tracing direct dependencies."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("revenue", "100000")
        builder.compute("cogs", "revenue * 0.4")
        builder.compute("gross_profit", "revenue - cogs")
        model = builder.build()

        graph = DependencyGraph.from_model(model)
        tracer = DependencyTracer(model, graph)

        deps = tracer.direct_dependencies("gross_profit")
        assert len(deps) == 2
        assert "revenue" in deps
        assert "cogs" in deps

    def test_dependency_tracer_all_dependencies(self) -> None:
        """Test tracing transitive dependencies."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("a", "10")
        builder.compute("b", "a * 2")
        builder.compute("c", "a + b")
        model = builder.build()

        graph = DependencyGraph.from_model(model)
        tracer = DependencyTracer(model, graph)

        all_deps = tracer.all_dependencies("c")
        assert "a" in all_deps
        assert "b" in all_deps

    def test_dependency_tree(self) -> None:
        """Test dependency tree construction."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("revenue", "100000")
        builder.compute("cogs", "revenue * 0.4")
        builder.compute("gross_profit", "revenue - cogs")
        model = builder.build()

        graph = DependencyGraph.from_model(model)
        tracer = DependencyTracer(model, graph)

        tree = tracer.dependency_tree("gross_profit")
        assert tree.node_id == "gross_profit"
        assert len(tree.children) == 2
        assert tree.depth() >= 1

    def test_render_tree_ascii(self) -> None:
        """Test ASCII tree rendering."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("revenue", "100000")
        builder.compute("cogs", "revenue * 0.4")
        builder.compute("gross_profit", "revenue - cogs")
        model = builder.build()

        graph = DependencyGraph.from_model(model)
        tracer = DependencyTracer(model, graph)
        tree = tracer.dependency_tree("gross_profit")

        ascii_str = render_tree_ascii(tree)
        assert "gross_profit" in ascii_str
        assert len(ascii_str) > 0

    def test_render_tree_detailed(self) -> None:
        """Test tree rendering with values."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("revenue", "100000")
        builder.compute("cogs", "revenue * 0.4")
        builder.compute("gross_profit", "revenue - cogs")
        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        graph = DependencyGraph.from_model(model)
        tracer = DependencyTracer(model, graph)
        tree = tracer.dependency_tree("gross_profit")

        detailed = render_tree_detailed(tree, results, PeriodId.quarter(2025, 1))
        assert "60000.00" in detailed  # gross_profit value
        assert "100000.00" in detailed  # revenue value

    def test_formula_explainer(self) -> None:
        """Test formula explanation."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("revenue", "100000")
        builder.compute("cogs", "revenue * 0.4")
        builder.compute("gross_profit", "revenue - cogs")
        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        explainer = FormulaExplainer(model, results)
        period = PeriodId.quarter(2025, 1)
        explanation = explainer.explain("gross_profit", period)

        assert explanation.node_id == "gross_profit"
        assert explanation.final_value == pytest.approx(60000.0)
        assert explanation.formula_text == "revenue - cogs"

    def test_explanation_to_string(self) -> None:
        """Test explanation string representations."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("revenue", "100000")
        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        explainer = FormulaExplainer(model, results)
        explanation = explainer.explain("revenue", PeriodId.quarter(2025, 1))

        detailed = explanation.to_string_detailed()
        assert "revenue" in detailed

        compact = explanation.to_string_compact()
        assert len(compact) > 0


class TestReportsModule:
    """Test reports module."""

    def test_table_builder_basic(self) -> None:
        """Test basic table building."""
        table = TableBuilder()
        table.add_header("Name")
        table.add_header_with_alignment("Value", Alignment.RIGHT)
        table.add_row(["Revenue", "100M"])
        table.add_row(["COGS", "40M"])

        output = table.build()
        assert "Revenue" in output
        assert "100M" in output

    def test_table_builder_markdown(self) -> None:
        """Test Markdown table building."""
        table = TableBuilder()
        table.add_header("Item")
        table.add_header("Amount")
        table.add_row(["Revenue", "100M"])

        markdown = table.build_markdown()
        assert "Item" in markdown
        assert "|" in markdown

    def test_pl_summary_report(self) -> None:
        """Test P&L summary report."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("revenue", "100000")
        builder.compute("cogs", "40000")
        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        report = PLSummaryReport(results, ["revenue", "cogs"], [PeriodId.quarter(2025, 1)])
        output = report.to_string()
        assert "P&L Summary" in output
        assert "revenue" in output

    def test_credit_assessment_report(self) -> None:
        """Test credit assessment report."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.compute("revenue", "100000")
        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        report = CreditAssessmentReport(results, PeriodId.quarter(2025, 1))
        output = report.to_string()
        assert len(output) > 0

    def test_debt_summary_report(self) -> None:
        """Test debt summary report."""
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "total_debt",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(1000000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(1000000.0)),
            ],
        )
        model = builder.build()

        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        report = DebtSummaryReport(model, results, PeriodId.quarter(2025, 1))
        output = report.to_string()
        assert "Debt Summary" in output


class TestComprehensiveParity:
    """Test comprehensive feature integration."""

    def test_full_workflow_with_all_features(self) -> None:
        """Test a complete workflow using multiple features."""
        # Build model with various features
        builder = ModelBuilder.new("comprehensive_test")
        builder.periods("2025Q1..Q4", "2025Q1")  # Q1 is actuals, Q2-Q4 use forecast

        # Add various node types
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110000.0)),
                (PeriodId.quarter(2025, 3), AmountOrScalar.scalar(120000.0)),
                (PeriodId.quarter(2025, 4), AmountOrScalar.scalar(130000.0)),
            ],
        )
        builder.compute("cogs", "revenue * 0.4")
        builder.compute("gross_profit", "revenue - cogs")

        # Add mixed node
        mixed = builder.mixed("opex")
        mixed.values([(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(20000.0))])
        mixed.forecast(ForecastSpec.forward_fill())
        builder = mixed.finish()

        builder.compute("ebitda", "gross_profit - opex")

        # Add metrics
        builder.add_metric("fin.gross_margin")

        model = builder.build()

        # Evaluate
        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Test DataFrame export
        df_long = results.to_polars_long()
        assert df_long is not None

        df_wide = results.to_polars_wide()
        assert df_wide is not None

        # Test dependency analysis
        graph = DependencyGraph.from_model(model)
        tracer = DependencyTracer(model, graph)

        # Test direct dependencies
        ebitda_deps = tracer.direct_dependencies("ebitda")
        assert "gross_profit" in ebitda_deps
        assert "opex" in ebitda_deps

        # Test dependency tree
        tree = tracer.dependency_tree("ebitda")
        assert tree.depth() >= 2

        # Test formula explanation
        explainer = FormulaExplainer(model, results)
        explanation = explainer.explain("ebitda", PeriodId.quarter(2025, 1))
        assert explanation.node_id == "ebitda"

        # Test reports
        report = PLSummaryReport(
            results,
            ["revenue", "cogs", "gross_profit", "ebitda"],
            [PeriodId.quarter(2025, 1), PeriodId.quarter(2025, 2)],
        )
        report_str = report.to_string()
        assert "P&L Summary" in report_str

    def test_serialization_roundtrip(self) -> None:
        """Test model and results serialization."""
        builder = ModelBuilder.new("roundtrip_test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100000.0)),
            ],
        )
        builder.compute("cogs", "revenue * 0.6")
        model = builder.build()

        # Serialize model
        model_json = model.to_json()
        assert len(model_json) > 0

        # Deserialize model
        model_restored = FinancialModelSpec.from_json(model_json)
        assert model_restored.id == "roundtrip_test"

        # Evaluate and serialize results
        evaluator = Evaluator.new()
        results = evaluator.evaluate(model_restored)

        results_json = results.to_json()
        assert len(results_json) > 0

        # Deserialize results
        results_restored = Results.from_json(results_json)
        assert results_restored.get("revenue", PeriodId.quarter(2025, 1)) == pytest.approx(100000.0)


class TestExtensions:
    """Test extension system."""

    def test_corkscrew_extension(self) -> None:
        """Test corkscrew extension."""
        # This is a basic test - corkscrew requires specific node patterns
        builder = ModelBuilder.new("test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100000.0)),
            ],
        )
        model = builder.build()

        evaluator = Evaluator.new()
        _results = evaluator.evaluate(model)

        _registry = ExtensionRegistry.new()
        # Note: We can't test execution without proper corkscrew nodes
        # This just verifies the binding works
        assert True  # Verify instantiation succeeded

    def test_extension_registry(self) -> None:
        """Test extension registry creation."""
        registry = ExtensionRegistry.new()
        assert registry is not None


class TestRegistry:
    """Test registry system."""

    def test_registry_load_builtins(self) -> None:
        """Test loading built-in metrics."""
        registry = Registry.new()
        registry.load_builtins()

        metrics = registry.list_metrics("fin")
        assert len(metrics) > 0

    def test_registry_get_metric(self) -> None:
        """Test getting a specific metric."""
        registry = Registry.new()
        registry.load_builtins()

        metric = registry.get("fin.gross_margin")
        assert metric is not None

    def test_registry_has_metric(self) -> None:
        """Test metric existence check."""
        registry = Registry.new()
        registry.load_builtins()

        assert registry.has_metric("fin.gross_margin")
        assert not registry.has_metric("nonexistent.metric")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
