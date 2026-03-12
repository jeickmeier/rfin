"""Comprehensive parity tests for statements Python bindings.

This test suite verifies that the Python bindings for finstack-statements
provide full parity with the Rust crate across all major features.
"""

from datetime import date

from finstack.core.dates import PeriodId
from finstack.core.money import Money
import pytest

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
    SensitivityAnalyzer,
    SensitivityConfig,
    SensitivityMode,
    StatementResult,
    TableBuilder,
    TornadoEntry,
    generate_tornado_chart,
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


class TestRealEstateTemplateValidationParity:
    """Lease template validation should come from Rust-backed validation methods."""

    def test_lease_spec_validate(self) -> None:
        """LeaseSpec should expose Rust-backed validation."""
        from finstack.statements import LeaseSpec

        lease = LeaseSpec(
            "lease_1",
            PeriodId.quarter(2025, 1),
            1000.0,
            occupancy=1.0,
        )
        lease.validate()

    def test_lease_spec_validate_rejects_invalid_occupancy(self) -> None:
        """LeaseSpec.validate should reject occupancy outside [0, 1]."""
        from finstack.statements import LeaseSpec

        lease = LeaseSpec(
            "lease_1",
            PeriodId.quarter(2025, 1),
            1000.0,
            occupancy=1.5,
        )
        with pytest.raises(ValueError, match="occupancy"):
            lease.validate()

    def test_renewal_spec_validate_rejects_invalid_probability(self) -> None:
        """RenewalSpec.validate should reject probability outside [0, 1]."""
        from finstack.statements import RenewalSpec

        renewal = RenewalSpec(
            downtime_periods=1,
            term_periods=4,
            probability=1.5,
        )
        with pytest.raises(ValueError, match="probability"):
            renewal.validate()

    def test_lease_spec_v2_validate_rejects_invalid_occupancy(self) -> None:
        """LeaseSpecV2.validate should reject occupancy outside [0, 1]."""
        from finstack.statements import LeaseSpecV2

        lease = LeaseSpecV2(
            node_id="lease_2",
            start=PeriodId.quarter(2025, 1),
            base_rent=1500.0,
            occupancy=-0.1,
        )
        with pytest.raises(ValueError, match="occupancy"):
            lease.validate()

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
        builder = mixed.build()

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
        builder = mixed.build()

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

    def test_generate_tornado_chart_sorts_by_swing(self) -> None:
        """Generated tornado entries should be sorted by swing magnitude."""
        period = PeriodId.quarter(2025, 1)
        builder = ModelBuilder.new("tornado_test")
        builder.periods("2025Q1..Q1", None)
        builder.value("revenue", [(period, AmountOrScalar.scalar(100000.0))])
        builder.value("opex", [(period, AmountOrScalar.scalar(40000.0))])
        builder.compute("gross_profit", "revenue - opex")
        model = builder.build()

        analyzer = SensitivityAnalyzer(model)
        config = SensitivityConfig(SensitivityMode.DIAGONAL)
        config.add_parameter(ParameterSpec.with_percentages("revenue", period, 100000.0, [-10.0, 0.0, 10.0]))
        config.add_parameter(ParameterSpec.with_percentages("opex", period, 40000.0, [-25.0, 0.0, 25.0]))
        config.add_target_metric("gross_profit")

        result = analyzer.run(config)
        entries = generate_tornado_chart(result, "gross_profit@2025Q1")

        assert [entry.parameter_id for entry in entries] == ["revenue", "opex"]
        assert abs(entries[0].swing) == pytest.approx(20000.0)
        assert abs(entries[1].swing) == pytest.approx(20000.0)

    def test_generate_tornado_chart_uses_requested_period(self) -> None:
        """Metric period selectors should drive tornado metric extraction."""
        q1 = PeriodId.quarter(2025, 1)
        q2 = PeriodId.quarter(2025, 2)
        builder = ModelBuilder.new("tornado_period_test")
        builder.periods("2025Q1..Q2", None)
        builder.value(
            "revenue",
            [
                (q1, AmountOrScalar.scalar(100000.0)),
                (q2, AmountOrScalar.scalar(200000.0)),
            ],
        )
        builder.value(
            "opex",
            [
                (q1, AmountOrScalar.scalar(40000.0)),
                (q2, AmountOrScalar.scalar(70000.0)),
            ],
        )
        builder.compute("gross_profit", "revenue - opex")
        model = builder.build()

        analyzer = SensitivityAnalyzer(model)
        config = SensitivityConfig(SensitivityMode.DIAGONAL)
        config.add_parameter(ParameterSpec.with_percentages("revenue", q1, 100000.0, [-10.0, 0.0, 10.0]))
        config.add_parameter(ParameterSpec.with_percentages("opex", q1, 40000.0, [-25.0, 0.0, 25.0]))
        config.add_target_metric("gross_profit")

        result = analyzer.run(config)
        entries = generate_tornado_chart(result, "gross_profit@2025Q2")

        assert [entry.downside_impact for entry in entries] == [0.0, 0.0]
        assert [entry.upside_impact for entry in entries] == [0.0, 0.0]


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
        builder = mixed.build()

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
        results_restored = StatementResult.from_json(results_json)
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


def test_amount_or_scalar_is_amount() -> None:
    """Test is_amount property on AmountOrScalar."""
    from finstack.core.currency import USD
    from finstack.statements.types import AmountOrScalar

    scalar = AmountOrScalar.scalar(42.0)
    assert scalar.is_scalar is True
    assert scalar.is_amount is False

    amount = AmountOrScalar.amount(100.0, USD)
    assert amount.is_scalar is False
    assert amount.is_amount is True


def test_amount_or_scalar_as_money() -> None:
    """Test as_money method on AmountOrScalar."""
    from finstack.core.currency import USD
    from finstack.statements.types import AmountOrScalar

    scalar = AmountOrScalar.scalar(42.0)
    assert scalar.as_money() is None

    amount = AmountOrScalar.amount(100.0, USD)
    money = amount.as_money()
    assert money is not None
    assert money.amount == pytest.approx(100.0)


def test_statement_result_get_money() -> None:
    """Test get_money method on StatementResult."""
    from finstack.core.currency import USD
    from finstack.core.dates import PeriodId
    from finstack.core.money import Money

    from finstack.statements import Evaluator, ModelBuilder

    builder = ModelBuilder.new("money_test")
    builder.periods("2025Q1..Q2", None)
    builder.value_money(
        "revenue",
        [
            (PeriodId.quarter(2025, 1), Money(100000.0, USD)),
            (PeriodId.quarter(2025, 2), Money(110000.0, USD)),
        ],
    )
    model = builder.build()

    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    q1 = PeriodId.quarter(2025, 1)
    money = results.get_money("revenue", q1)
    assert money is not None
    assert money.amount == pytest.approx(100000.0)

    # Non-existent node should return None
    assert results.get_money("nonexistent", q1) is None


def test_statement_result_get_scalar() -> None:
    """Test get_scalar method on StatementResult."""
    from finstack.core.dates import PeriodId

    from finstack.statements import Evaluator, ModelBuilder

    builder = ModelBuilder.new("scalar_test")
    builder.periods("2025Q1..Q2", None)
    builder.value_scalar(
        "margin",
        [
            (PeriodId.quarter(2025, 1), 0.35),
            (PeriodId.quarter(2025, 2), 0.40),
        ],
    )
    model = builder.build()

    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    q1 = PeriodId.quarter(2025, 1)
    scalar = results.get_scalar("margin", q1)
    assert scalar is not None
    assert scalar == pytest.approx(0.35)

    # Non-existent node should return None
    assert results.get_scalar("nonexistent", q1) is None


def test_evaluator_with_market_context() -> None:
    """Test Evaluator.with_market_context returns EvaluatorWithContext."""
    from datetime import date

    from finstack.core.dates import PeriodId
    from finstack.core.market_data import MarketContext

    from finstack.statements import AmountOrScalar, Evaluator, ModelBuilder

    builder = ModelBuilder.new("ctx_test")
    builder.periods("2025Q1..Q2", None)
    builder.value(
        "revenue",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100.0)),
        ],
    )
    model = builder.build()

    evaluator = Evaluator.new()
    market_ctx = MarketContext()
    ctx_evaluator = evaluator.with_market_context(market_ctx, date(2025, 1, 1))

    # Should be able to evaluate with the context evaluator
    results = ctx_evaluator.evaluate(model)
    assert results is not None
    assert results.get("revenue", PeriodId.quarter(2025, 1)) == pytest.approx(100.0)


def test_node_value_type_enum() -> None:
    """Test NodeValueType enum exposure."""
    from finstack.core.currency import USD
    from finstack.statements.types import NodeValueType

    scalar = NodeValueType.SCALAR
    assert scalar is not None
    assert scalar.currency is None

    monetary = NodeValueType.monetary(USD)
    assert monetary is not None
    assert monetary.currency is not None


def test_statement_result_node_value_types() -> None:
    """Test node_value_types property on StatementResult."""
    from finstack.core.currency import USD
    from finstack.core.dates import PeriodId
    from finstack.core.money import Money

    from finstack.statements import Evaluator, ModelBuilder

    builder = ModelBuilder.new("value_types_test")
    builder.periods("2025Q1..Q2", None)
    builder.value_money(
        "revenue",
        [
            (PeriodId.quarter(2025, 1), Money(100000.0, USD)),
            (PeriodId.quarter(2025, 2), Money(110000.0, USD)),
        ],
    )
    builder.value_scalar(
        "margin",
        [
            (PeriodId.quarter(2025, 1), 0.35),
            (PeriodId.quarter(2025, 2), 0.36),
        ],
    )
    model = builder.build()

    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    value_types = results.node_value_types
    assert isinstance(value_types, dict)
    # Monetary and scalar nodes should be reflected in value_types


def test_statement_result_cs_cashflows_none() -> None:
    """Test cs_cashflows is None when no capital structure."""
    from finstack.core.dates import PeriodId

    from finstack.statements import AmountOrScalar, Evaluator, ModelBuilder

    builder = ModelBuilder.new("no_cs_test")
    builder.periods("2025Q1..Q2", None)
    builder.value(
        "revenue",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(100.0)),
        ],
    )
    model = builder.build()

    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    # No capital structure, so cs_cashflows should be None
    assert results.cs_cashflows is None


def test_backtest_forecast() -> None:
    """Test forecast backtesting metrics."""
    from finstack.statements.analysis import backtest_forecast

    actual = [100.0, 110.0, 105.0, 115.0]
    forecast = [98.0, 112.0, 104.0, 116.0]

    metrics = backtest_forecast(actual, forecast)
    assert metrics.mae > 0.0
    assert metrics.mape > 0.0
    assert metrics.rmse >= metrics.mae  # RMSE >= MAE always
    assert metrics.n == 4

    summary = metrics.summary()
    assert "MAE" in summary
    assert "MAPE" in summary
    assert "RMSE" in summary


def test_backtest_forecast_perfect() -> None:
    """Test perfect forecast has zero errors."""
    from finstack.statements.analysis import backtest_forecast

    actual = [100.0, 110.0, 120.0]
    metrics = backtest_forecast(actual, actual)
    assert metrics.mae == pytest.approx(0.0)
    assert metrics.rmse == pytest.approx(0.0)


def test_backtest_forecast_mismatched_lengths() -> None:
    """Test error on mismatched array lengths."""
    from finstack.statements.analysis import backtest_forecast

    with pytest.raises(RuntimeError, match="same length"):
        backtest_forecast([1.0, 2.0], [1.0])


def test_credit_context_metrics_import() -> None:
    """Test CreditContextMetrics struct is importable."""
    from finstack.statements.analysis import CreditContextMetrics, compute_credit_context

    assert CreditContextMetrics is not None
    assert compute_credit_context is not None


class TestDcfCorporateValuation:
    """Test corporate DCF valuation module."""

    def test_dcf_options(self) -> None:
        """Test DcfOptions construction."""
        from finstack.statements.analysis import DcfOptions

        opts = DcfOptions(mid_year_convention=True, shares_outstanding=1000000.0)
        assert opts.mid_year_convention is True
        assert opts.shares_outstanding == pytest.approx(1000000.0)

    def test_dcf_types_import(self) -> None:
        """Test DCF types are importable."""
        from finstack.statements.analysis import (
            CorporateValuationResult,
            DcfOptions,
            evaluate_dcf,
            evaluate_dcf_with_market,
            evaluate_dcf_with_options,
        )

        assert DcfOptions is not None
        assert CorporateValuationResult is not None
        assert evaluate_dcf is not None
        assert evaluate_dcf_with_options is not None
        assert evaluate_dcf_with_market is not None


def test_goal_seek_function_import() -> None:
    """goal_seek should be importable from finstack.statements.analysis."""
    from finstack.statements.analysis import goal_seek

    assert goal_seek is not None


def test_monte_carlo_config() -> None:
    """Test MonteCarloConfig construction."""
    from finstack.statements.analysis import MonteCarloConfig

    config = MonteCarloConfig(n_paths=1000, seed=42)
    assert config.n_paths == 1000
    assert config.seed == 42

    config2 = config.with_percentiles([0.1, 0.5, 0.9])
    assert config2.percentiles == pytest.approx([0.1, 0.5, 0.9])


def test_monte_carlo_config_defaults() -> None:
    """Test MonteCarloConfig default percentiles."""
    from finstack.statements.analysis import MonteCarloConfig

    config = MonteCarloConfig(n_paths=500, seed=123)
    # Default percentiles are [0.05, 0.5, 0.95]
    assert len(config.percentiles) == 3
    assert config.percentiles[0] == pytest.approx(0.05)


def test_monte_carlo_config_repr() -> None:
    """Test MonteCarloConfig __repr__."""
    from finstack.statements.analysis import MonteCarloConfig

    config = MonteCarloConfig(n_paths=1000, seed=42)
    assert "MonteCarloConfig" in repr(config)
    assert "1000" in repr(config)
    assert "42" in repr(config)


def test_forecast_covenant_import() -> None:
    """Test covenant analysis functions are importable."""
    from finstack.statements.analysis import forecast_breaches, forecast_covenant, forecast_covenants

    assert forecast_breaches is not None
    assert forecast_covenant is not None
    assert forecast_covenants is not None


def test_corporate_analysis_builder_import() -> None:
    """Test CorporateAnalysisBuilder and related types are importable."""
    from finstack.statements.analysis import (
        CorporateAnalysis,
        CorporateAnalysisBuilder,
        CreditInstrumentAnalysis,
    )

    assert CorporateAnalysis is not None
    assert CorporateAnalysisBuilder is not None
    assert CreditInstrumentAnalysis is not None


def test_corporate_analysis_builder_basic() -> None:
    """Test CorporateAnalysisBuilder basic pipeline."""
    from finstack.core.dates import PeriodId
    from finstack.statements.analysis import CorporateAnalysisBuilder

    from finstack.statements import AmountOrScalar, ModelBuilder

    builder = ModelBuilder.new("corp_test")
    builder.periods("2025Q1..Q4", None)
    builder.value(
        "revenue",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110000.0)),
            (PeriodId.quarter(2025, 3), AmountOrScalar.scalar(120000.0)),
            (PeriodId.quarter(2025, 4), AmountOrScalar.scalar(130000.0)),
        ],
    )
    model = builder.build()

    analysis = CorporateAnalysisBuilder(model).analyze()

    assert analysis.statement is not None
    assert analysis.equity is None  # No DCF configured
    assert isinstance(analysis.credit, dict)
    assert len(analysis.credit) == 0  # No capital structure


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
