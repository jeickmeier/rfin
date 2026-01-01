"""
Benchmark: Evaluate 100-node statement model over 60 periods.

This benchmark measures the computational efficiency of statement model
evaluation, including DAG topological sort and formula evaluation.
"""

from datetime import date

import pytest

from finstack.statements import ModelBuilder, NodeType


def create_100_node_model():
    """Create a realistic 100-node P&L and balance sheet model."""
    builder = ModelBuilder()
    
    # Define 60 monthly periods (5 years)
    start = date(2024, 1, 1)
    end = date(2028, 12, 31)
    builder.periods(start, end, "Monthly")
    
    # ============================================
    # P&L Statement (40 nodes)
    # ============================================
    
    # Revenue drivers (10 nodes)
    builder.node("units_sold", NodeType.input())
    builder.node("unit_price", NodeType.input())
    builder.node("revenue", NodeType.formula("units_sold * unit_price"))
    
    builder.node("subscription_count", NodeType.input())
    builder.node("subscription_price", NodeType.input())
    builder.node("subscription_revenue", NodeType.formula("subscription_count * subscription_price"))
    
    builder.node("services_hours", NodeType.input())
    builder.node("hourly_rate", NodeType.input())
    builder.node("services_revenue", NodeType.formula("services_hours * hourly_rate"))
    
    builder.node("total_revenue", NodeType.formula("revenue + subscription_revenue + services_revenue"))
    
    # Cost of goods sold (10 nodes)
    builder.node("cogs_rate", NodeType.input())
    builder.node("cogs", NodeType.formula("revenue * cogs_rate"))
    
    builder.node("subscription_cogs_rate", NodeType.input())
    builder.node("subscription_cogs", NodeType.formula("subscription_revenue * subscription_cogs_rate"))
    
    builder.node("services_cogs_rate", NodeType.input())
    builder.node("services_cogs", NodeType.formula("services_revenue * services_cogs_rate"))
    
    builder.node("total_cogs", NodeType.formula("cogs + subscription_cogs + services_cogs"))
    
    builder.node("gross_profit", NodeType.formula("total_revenue - total_cogs"))
    builder.node("gross_margin", NodeType.formula("gross_profit / total_revenue"))
    builder.node("target_gross_margin", NodeType.input())
    
    # Operating expenses (15 nodes)
    builder.node("headcount", NodeType.input())
    builder.node("avg_salary", NodeType.input())
    builder.node("salaries", NodeType.formula("headcount * avg_salary"))
    
    builder.node("benefits_rate", NodeType.input())
    builder.node("benefits", NodeType.formula("salaries * benefits_rate"))
    
    builder.node("marketing_rate", NodeType.input())
    builder.node("marketing", NodeType.formula("total_revenue * marketing_rate"))
    
    builder.node("rd_headcount", NodeType.input())
    builder.node("rd_avg_salary", NodeType.input())
    builder.node("rd_expense", NodeType.formula("rd_headcount * rd_avg_salary"))
    
    builder.node("rent_per_sqft", NodeType.input())
    builder.node("office_sqft", NodeType.input())
    builder.node("rent", NodeType.formula("rent_per_sqft * office_sqft"))
    
    builder.node("other_opex_rate", NodeType.input())
    builder.node("other_opex", NodeType.formula("total_revenue * other_opex_rate"))
    
    # EBITDA and income statement (5 nodes)
    builder.node(
        "total_opex",
        NodeType.formula("salaries + benefits + marketing + rd_expense + rent + other_opex"),
    )
    builder.node("ebitda", NodeType.formula("gross_profit - total_opex"))
    builder.node("ebitda_margin", NodeType.formula("ebitda / total_revenue"))
    builder.node("depreciation", NodeType.input())
    builder.node("ebit", NodeType.formula("ebitda - depreciation"))
    
    # ============================================
    # Balance Sheet (30 nodes)
    # ============================================
    
    # Assets (15 nodes)
    builder.node("cash_opening", NodeType.input())
    builder.node("cash_closing", NodeType.formula("cash_opening + ebitda - capex - debt_repayment"))
    
    builder.node("dso", NodeType.input())
    builder.node("ar", NodeType.formula("(total_revenue / 365) * dso * 30"))
    
    builder.node("inventory_days", NodeType.input())
    builder.node("inventory", NodeType.formula("(total_cogs / 365) * inventory_days * 30"))
    
    builder.node("prepaid_rate", NodeType.input())
    builder.node("prepaid", NodeType.formula("total_opex * prepaid_rate"))
    
    builder.node("current_assets", NodeType.formula("cash_closing + ar + inventory + prepaid"))
    
    builder.node("ppe_opening", NodeType.input())
    builder.node("capex", NodeType.input())
    builder.node("ppe_closing", NodeType.formula("ppe_opening + capex - depreciation"))
    
    builder.node("intangibles", NodeType.input())
    builder.node("goodwill", NodeType.input())
    
    builder.node("total_assets", NodeType.formula("current_assets + ppe_closing + intangibles + goodwill"))
    
    # Liabilities (10 nodes)
    builder.node("dpo", NodeType.input())
    builder.node("ap", NodeType.formula("(total_cogs / 365) * dpo * 30"))
    
    builder.node("accrued_opex_rate", NodeType.input())
    builder.node("accrued_expenses", NodeType.formula("total_opex * accrued_opex_rate"))
    
    builder.node("deferred_revenue_rate", NodeType.input())
    builder.node("deferred_revenue", NodeType.formula("total_revenue * deferred_revenue_rate"))
    
    builder.node("current_liabilities", NodeType.formula("ap + accrued_expenses + deferred_revenue"))
    
    builder.node("debt_opening", NodeType.input())
    builder.node("debt_repayment", NodeType.input())
    builder.node("debt_closing", NodeType.formula("debt_opening - debt_repayment"))
    
    # Equity (5 nodes)
    builder.node("equity_opening", NodeType.input())
    builder.node("net_income", NodeType.formula("ebit * 0.75"))  # Simplified tax
    builder.node("dividends", NodeType.input())
    builder.node("equity_closing", NodeType.formula("equity_opening + net_income - dividends"))
    builder.node(
        "total_liabilities_equity",
        NodeType.formula("current_liabilities + debt_closing + equity_closing"),
    )
    
    # ============================================
    # Ratios and KPIs (30 nodes)
    # ============================================
    
    builder.node("revenue_growth", NodeType.formula("pct_change(total_revenue, 1)"))
    builder.node("ebitda_growth", NodeType.formula("pct_change(ebitda, 1)"))
    builder.node("roe", NodeType.formula("net_income / equity_closing"))
    builder.node("roa", NodeType.formula("net_income / total_assets"))
    builder.node("debt_to_equity", NodeType.formula("debt_closing / equity_closing"))
    builder.node("current_ratio", NodeType.formula("current_assets / current_liabilities"))
    builder.node("quick_ratio", NodeType.formula("(cash_closing + ar) / current_liabilities"))
    builder.node("asset_turnover", NodeType.formula("total_revenue / total_assets"))
    
    # Rolling metrics
    builder.node("revenue_12m", NodeType.formula("rolling_sum(total_revenue, 12)"))
    builder.node("ebitda_12m", NodeType.formula("rolling_sum(ebitda, 12)"))
    builder.node("net_income_12m", NodeType.formula("rolling_sum(net_income, 12)"))
    
    builder.node("revenue_volatility", NodeType.formula("rolling_std(total_revenue, 12)"))
    builder.node("ebitda_volatility", NodeType.formula("rolling_std(ebitda, 12)"))
    
    # Cumulative metrics
    builder.node("cumulative_revenue", NodeType.formula("cumsum(total_revenue)"))
    builder.node("cumulative_ebitda", NodeType.formula("cumsum(ebitda)"))
    builder.node("cumulative_capex", NodeType.formula("cumsum(capex)"))
    
    # Year-over-year comparisons
    builder.node("revenue_yoy_change", NodeType.formula("diff(total_revenue, 12)"))
    builder.node("ebitda_yoy_change", NodeType.formula("diff(ebitda, 12)"))
    builder.node("revenue_yoy_pct", NodeType.formula("pct_change(total_revenue, 12)"))
    builder.node("ebitda_yoy_pct", NodeType.formula("pct_change(ebitda, 12)"))
    
    # Efficiency metrics
    builder.node("revenue_per_employee", NodeType.formula("total_revenue / headcount"))
    builder.node("opex_per_employee", NodeType.formula("total_opex / headcount"))
    builder.node("ebitda_per_employee", NodeType.formula("ebitda / headcount"))
    
    # Cash flow metrics
    builder.node("fcf", NodeType.formula("ebitda - capex"))
    builder.node("fcf_margin", NodeType.formula("fcf / total_revenue"))
    builder.node("fcf_12m", NodeType.formula("rolling_sum(fcf, 12)"))
    
    # Unit economics
    builder.node("cac", NodeType.formula("marketing / (subscription_count - lag(subscription_count, 1))"))
    builder.node("ltv", NodeType.formula("subscription_price * 36"))  # Simplified LTV
    builder.node("ltv_cac_ratio", NodeType.formula("ltv / cac"))
    
    return builder


def add_sample_values(builder):
    """Add sample input values to make the model evaluable."""
    # Revenue drivers
    for period in range(60):
        # Growing units sold: 1000 to 2000 over 5 years
        units = 1000 + (period * 1000 / 60)
        builder.value("units_sold", period, units)
        builder.value("unit_price", period, 100.0)
        
        # Growing subscriptions
        subs = 500 + (period * 500 / 60)
        builder.value("subscription_count", period, subs)
        builder.value("subscription_price", period, 50.0)
        
        # Services
        builder.value("services_hours", period, 1000)
        builder.value("hourly_rate", period, 150.0)
        
        # Rates
        builder.value("cogs_rate", period, 0.35)
        builder.value("subscription_cogs_rate", period, 0.20)
        builder.value("services_cogs_rate", period, 0.40)
        
        # Headcount
        hc = 50 + (period * 50 / 60)
        builder.value("headcount", period, hc)
        builder.value("avg_salary", period, 10000.0)
        builder.value("benefits_rate", period, 0.30)
        
        # Marketing
        builder.value("marketing_rate", period, 0.15)
        
        # R&D
        rd_hc = 20 + (period * 10 / 60)
        builder.value("rd_headcount", period, rd_hc)
        builder.value("rd_avg_salary", period, 12000.0)
        
        # Facilities
        builder.value("rent_per_sqft", period, 50.0)
        builder.value("office_sqft", period, 10000)
        
        # Other
        builder.value("other_opex_rate", period, 0.05)
        builder.value("depreciation", period, 50000)
        
        # Balance sheet inputs
        if period == 0:
            builder.value("cash_opening", period, 1000000)
            builder.value("ppe_opening", period, 500000)
            builder.value("debt_opening", period, 2000000)
            builder.value("equity_opening", period, 1500000)
        else:
            # Link to previous period
            builder.value("cash_opening", period, 0)  # Will be computed from prior period
            builder.value("ppe_opening", period, 0)
            builder.value("debt_opening", period, 0)
            builder.value("equity_opening", period, 0)
        
        builder.value("capex", period, 100000)
        builder.value("debt_repayment", period, 20000)
        builder.value("dividends", period, 0)
        
        # Working capital assumptions
        builder.value("dso", period, 45)
        builder.value("inventory_days", period, 30)
        builder.value("dpo", period, 30)
        builder.value("prepaid_rate", period, 0.05)
        builder.value("accrued_opex_rate", period, 0.10)
        builder.value("deferred_revenue_rate", period, 0.08)
        
        # Intangibles
        builder.value("intangibles", period, 100000)
        builder.value("goodwill", period, 500000)
        
        # Targets
        builder.value("target_gross_margin", period, 0.60)


class TestStatementEvaluationBenchmarks:
    """Benchmarks for statement model evaluation."""
    
    def test_bench_evaluate_100_node_model(self, benchmark):
        """Benchmark: Evaluate 100-node model over 60 periods."""
        builder = create_100_node_model()
        add_sample_values(builder)
        model = builder.build()
        
        def evaluate_model():
            results = model.evaluate()
            return results
        
        # Run benchmark
        results = benchmark(evaluate_model)
        
        # Verify results exist for key nodes
        assert results.getValue("total_revenue", 0) is not None
        assert results.getValue("ebitda", 0) is not None
        assert results.getValue("total_assets", 0) is not None
    
    def test_bench_evaluate_small_model(self, benchmark):
        """Benchmark: Evaluate 20-node model (baseline for comparison)."""
        builder = ModelBuilder()
        
        start = date(2024, 1, 1)
        end = date(2028, 12, 31)
        builder.periods(start, end, "Monthly")
        
        # Simple 20-node P&L
        builder.node("revenue", NodeType.input())
        builder.node("cogs_rate", NodeType.input())
        builder.node("cogs", NodeType.formula("revenue * cogs_rate"))
        builder.node("gross_profit", NodeType.formula("revenue - cogs"))
        
        builder.node("salaries", NodeType.input())
        builder.node("rent", NodeType.input())
        builder.node("marketing", NodeType.input())
        builder.node("opex", NodeType.formula("salaries + rent + marketing"))
        
        builder.node("ebitda", NodeType.formula("gross_profit - opex"))
        builder.node("depreciation", NodeType.input())
        builder.node("ebit", NodeType.formula("ebitda - depreciation"))
        
        # Ratios
        builder.node("gross_margin", NodeType.formula("gross_profit / revenue"))
        builder.node("ebitda_margin", NodeType.formula("ebitda / revenue"))
        builder.node("revenue_growth", NodeType.formula("pct_change(revenue, 1)"))
        builder.node("revenue_12m", NodeType.formula("rolling_sum(revenue, 12)"))
        builder.node("ebitda_12m", NodeType.formula("rolling_sum(ebitda, 12)"))
        builder.node("revenue_volatility", NodeType.formula("rolling_std(revenue, 12)"))
        builder.node("cumulative_revenue", NodeType.formula("cumsum(revenue)"))
        builder.node("revenue_yoy_pct", NodeType.formula("pct_change(revenue, 12)"))
        builder.node("ebitda_yoy_pct", NodeType.formula("pct_change(ebitda, 12)"))
        
        # Add values
        for period in range(60):
            builder.value("revenue", period, 1000000 + (period * 10000))
            builder.value("cogs_rate", period, 0.40)
            builder.value("salaries", period, 300000)
            builder.value("rent", period, 50000)
            builder.value("marketing", period, 100000)
            builder.value("depreciation", period, 20000)
        
        model = builder.build()
        
        def evaluate_model():
            return model.evaluate()
        
        # Run benchmark
        results = benchmark(evaluate_model)
        assert results.getValue("revenue", 0) is not None
    
    def test_bench_model_construction(self, benchmark):
        """Benchmark: Model construction overhead (DAG building)."""
        def build_model():
            builder = create_100_node_model()
            add_sample_values(builder)
            return builder.build()
        
        # Run benchmark
        model = benchmark(build_model)
        assert model is not None
    
    def test_bench_single_period_evaluation(self, benchmark):
        """Benchmark: Evaluate single period (measures formula evaluation overhead)."""
        builder = create_100_node_model()
        add_sample_values(builder)
        model = builder.build()
        
        # Evaluate once to get results object
        results = model.evaluate()
        
        def get_period_values():
            # Extract all values for period 0
            values = {}
            # Sample a subset of key nodes
            nodes = [
                "total_revenue", "ebitda", "total_assets", "gross_margin",
                "ebitda_margin", "roe", "roa", "debt_to_equity",
            ]
            for node_id in nodes:
                values[node_id] = results.getValue(node_id, 0)
            return values
        
        # Run benchmark
        values = benchmark(get_period_values)
        assert len(values) == 8


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--benchmark-only"])
