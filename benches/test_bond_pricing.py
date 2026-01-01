"""
Benchmark: Price 1000 bonds with varying maturities and coupons.

This benchmark measures the FFI overhead and computational efficiency
of pricing a large portfolio of fixed-rate bonds.
"""

from datetime import date

import pytest

from finstack import Money
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations import Bond, create_standard_registry


def create_market_data():
    """Create market data for bond pricing."""
    # Build discount curve with realistic tenors
    knots = [
        (0.0, 1.0),
        (0.5, 0.98),
        (1.0, 0.96),
        (2.0, 0.92),
        (3.0, 0.88),
        (5.0, 0.80),
        (7.0, 0.72),
        (10.0, 0.62),
        (15.0, 0.48),
        (20.0, 0.38),
        (30.0, 0.25),
    ]
    
    base_date = date(2024, 1, 1)
    curve = DiscountCurve(
        id="USD.OIS",
        base_date=base_date,
        knots=knots,
    )
    
    market = MarketContext()
    market.insertDiscount(curve)
    return market


def create_bond_portfolio(num_bonds=1000):
    """Create a portfolio of bonds with varying maturities and coupons."""
    bonds = []
    notional = Money.fromCode(1_000_000, "USD")
    issue_date = date(2024, 1, 1)
    
    # Create bonds with varying maturities (1-30 years) and coupons (2%-6%)
    for i in range(num_bonds):
        # Distribute maturities across 1-30 years
        years = 1 + (i % 30)
        maturity_date = date(2024 + years, 1, 1)
        
        # Distribute coupons across 2%-6%
        coupon = 0.02 + ((i % 40) / 100.0)
        
        bond_id = f"BOND{i:04d}"
        bond = Bond.fixedSemiannual(
            bond_id,
            notional,
            coupon,
            issue_date,
            maturity_date,
            "USD.OIS",
        )
        bonds.append(bond)
    
    return bonds


class TestBondPricingBenchmarks:
    """Benchmarks for bond pricing operations."""
    
    def test_bench_price_1000_bonds(self, benchmark):
        """Benchmark: Price 1000 bonds sequentially."""
        market = create_market_data()
        bonds = create_bond_portfolio(1000)
        registry = create_standard_registry()
        
        def price_all_bonds():
            results = []
            for bond in bonds:
                result = registry.priceBond(bond, "discounting", market)
                results.append(result.presentValue.amount)
            return results
        
        # Run benchmark
        pvs = benchmark(price_all_bonds)
        
        # Verify we got 1000 results
        assert len(pvs) == 1000
        # Verify all PVs are non-zero
        assert all(pv != 0.0 for pv in pvs)
    
    def test_bench_price_with_metrics_100_bonds(self, benchmark):
        """Benchmark: Price 100 bonds with full metrics (slower but more realistic)."""
        market = create_market_data()
        bonds = create_bond_portfolio(100)
        registry = create_standard_registry()
        
        # Request common metrics
        metrics = ["clean_price", "accrued", "duration_mod", "dv01", "theta"]
        
        def price_with_metrics():
            results = []
            for bond in bonds:
                result = registry.priceBondWithMetrics(bond, "discounting", market, metrics)
                results.append({
                    "pv": result.presentValue.amount,
                    "clean_price": result.metric("clean_price"),
                    "dv01": result.metric("dv01"),
                })
            return results
        
        # Run benchmark
        results = benchmark(price_with_metrics)
        
        # Verify we got 100 results with metrics
        assert len(results) == 100
        assert all(r["pv"] != 0.0 for r in results)
        assert all(r["clean_price"] is not None for r in results)
    
    def test_bench_bond_construction(self, benchmark):
        """Benchmark: Bond construction overhead."""
        notional = Money.fromCode(1_000_000, "USD")
        issue_date = date(2024, 1, 1)
        maturity_date = date(2029, 1, 1)
        
        def construct_bonds():
            bonds = []
            for i in range(1000):
                bond_id = f"BOND{i:04d}"
                bond = Bond.fixedSemiannual(
                    bond_id,
                    notional,
                    0.05,
                    issue_date,
                    maturity_date,
                    "USD.OIS",
                )
                bonds.append(bond)
            return bonds
        
        # Run benchmark
        bonds = benchmark(construct_bonds)
        assert len(bonds) == 1000
    
    def test_bench_single_bond_pricing(self, benchmark):
        """Benchmark: Single bond pricing (baseline for understanding overhead)."""
        market = create_market_data()
        notional = Money.fromCode(1_000_000, "USD")
        issue_date = date(2024, 1, 1)
        maturity_date = date(2029, 1, 1)
        
        bond = Bond.fixedSemiannual(
            "BOND0001",
            notional,
            0.05,
            issue_date,
            maturity_date,
            "USD.OIS",
        )
        registry = create_standard_registry()
        
        def price_bond():
            result = registry.priceBond(bond, "discounting", market)
            return result.presentValue.amount
        
        # Run benchmark
        pv = benchmark(price_bond)
        assert pv != 0.0


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--benchmark-only"])
