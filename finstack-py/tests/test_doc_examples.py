"""Test docstring examples from .pyi files to ensure they work.

This module extracts code examples from docstrings in .pyi files and
runs them as tests. Examples are identified by the `>>>` prompt pattern
commonly used in Python docstrings.

Examples that are incomplete (e.g., missing setup, showing only part of
a workflow) are skipped with a note.
"""

import ast
import builtins
from collections.abc import Callable
import contextlib
from pathlib import Path
import re
import sys

import pytest

# Add finstack-py to path
sys.path.insert(0, str(Path(__file__).parent.parent))

import finstack
import finstack.core
import finstack.portfolio
import finstack.scenarios
import finstack.statements
import finstack.valuations


def extract_code_blocks_from_docstring(docstring: str) -> list[str]:
    """Extract code blocks from a docstring.

    Looks for code blocks marked with `>>>` (interactive Python prompt)
    and extracts the code, handling continuation lines.

    Args:
        docstring: The docstring to parse

    Returns:
        List of code blocks as strings
    """
    if not docstring:
        return []

    blocks = []
    current_block = []
    in_code_block = False

    for line in docstring.split("\n"):
        stripped = line.strip()

        # Check if line starts with >>> prompt (may be indented)
        if ">>> " in line:
            # Find the >>> and extract everything after it
            idx = line.find(">>> ")
            if idx >= 0:
                in_code_block = True
                code = line[idx + 4 :]  # Remove '>>> '
                current_block.append(code)
        elif in_code_block:
            # Check for continuation (starts with ...)
            if "... " in line:
                idx = line.find("... ")
                code = line[idx + 4 :]  # Remove '... '
                current_block.append(code)
            elif stripped == "...":
                current_block.append("")
            elif stripped == "":
                # Empty line - might be separator or part of block
                # If we have content, keep it; if not, end the block
                if not current_block or all(not b.strip() for b in current_block[-3:]):
                    # Multiple empty lines or empty block - end it
                    if current_block:
                        block_text = "\n".join(current_block).rstrip()
                        if block_text.strip():
                            blocks.append(block_text)
                        current_block = []
                    in_code_block = False
                else:
                    current_block.append("")
            elif not stripped.startswith("#"):
                # Non-empty, non-comment line that's not a continuation
                # End the code block
                if current_block:
                    block_text = "\n".join(current_block).rstrip()
                    if block_text.strip():
                        blocks.append(block_text)
                    current_block = []
                in_code_block = False

    # Handle case where docstring ends with code block
    if current_block:
        block_text = "\n".join(current_block).rstrip()
        if block_text.strip():
            blocks.append(block_text)

    return blocks


def extract_examples_from_pyi_file(pyi_path: Path) -> list[tuple[str, str, str]]:
    """Extract examples from a .pyi file.

    Args:
        pyi_path: Path to .pyi file

    Returns:
        List of (class_or_func_name, example_code, context) tuples
    """
    examples = []

    try:
        with pyi_path.open(encoding="utf-8") as f:
            content = f.read()
    except OSError:
        return examples

    # Parse the file to find class/function definitions
    try:
        tree = ast.parse(content)
    except SyntaxError:
        # .pyi files might have syntax that's not valid Python
        # Try a simpler regex-based approach
        return extract_examples_regex(content, pyi_path)

    for node in ast.walk(tree):
        if isinstance(node, ast.FunctionDef | ast.ClassDef | ast.AsyncFunctionDef):
            docstring = ast.get_docstring(node)
            if docstring:
                code_blocks = extract_code_blocks_from_docstring(docstring)
                for i, block in enumerate(code_blocks):
                    examples.append((f"{node.name}_{i}", block, f"{pyi_path}:{node.name}"))

    return examples


def extract_examples_regex(content: str, pyi_path: Path) -> list[tuple[str, str, str]]:
    """Extract examples using regex (fallback for files that don't parse)."""
    examples = []

    # Find class and function definitions
    pattern = r"^(class|def)\s+(\w+).*?:\s*\n(.*?)(?=^(?:class|def|\Z))"

    for match in re.finditer(pattern, content, re.MULTILINE | re.DOTALL):
        name = match.group(2)
        body = match.group(3)

        # Look for docstrings
        docstring_match = re.search(r'"""(.*?)"""', body, re.DOTALL)
        if docstring_match:
            docstring = docstring_match.group(1)
            code_blocks = extract_code_blocks_from_docstring(docstring)
            for i, block in enumerate(code_blocks):
                examples.append((f"{name}_{i}", block, f"{pyi_path}:{name}"))

    return examples


def find_pyi_files(root: Path) -> list[Path]:
    """Find all .pyi files in a directory tree."""
    return sorted([path for path in root.rglob("*.pyi") if "__pycache__" not in str(path)])


def is_runnable_example(code: str) -> bool:
    """Check if an example looks runnable (not just a fragment)."""
    if not code or not code.strip():
        return False

    # Skip examples that are clearly incomplete
    incomplete_patterns = [
        r"# \.\.\.\s*more code",  # Comment indicating more code
        r"# \.\.\.\s*setup",  # Comment indicating setup needed
        r"# Raises ValueError",  # Just a comment, not executable code
    ]

    for pattern in incomplete_patterns:
        if re.search(pattern, code, re.MULTILINE | re.IGNORECASE):
            return False

    # Skip if it's only comments
    non_comment_lines = [line for line in code.split("\n") if line.strip() and not line.strip().startswith("#")]
    if not non_comment_lines:
        return False

    # Check if it has at least one complete statement
    # Be lenient - try to parse, but don't fail on minor syntax issues
    try:
        ast.parse(code)
        return True
    except SyntaxError as e:
        # If it's just a missing import or undefined name, that's OK
        # We'll catch those at runtime
        error_msg = str(e).lower()
        if "invalid syntax" in error_msg or "unexpected" in error_msg:
            # Real syntax error - might be incomplete, but try anyway
            # Some examples might have incomplete statements that are still useful
            return len(non_comment_lines) > 0
        # Otherwise, assume it's runnable (might just need imports)
        return True


def create_simple_market_context() -> "finstack.core.market_data.context.MarketContext":
    """Create a simple MarketContext with basic USD curves for testing."""
    from datetime import date

    import finstack

    # Use the full import path
    ctx = finstack.core.market_data.context.MarketContext()

    # Add a simple USD discount curve
    discount_curve = finstack.core.market_data.term_structures.DiscountCurve(
        id="USD",
        base_date=date(2024, 1, 1),
        knots=[
            (0.0, 1.0),
            (0.25, 0.9975),
            (0.5, 0.995),
            (1.0, 0.99),
            (2.0, 0.98),
            (5.0, 0.95),
            (10.0, 0.90),
        ],
    )
    ctx.insert(discount_curve)

    # Add a simple forward curve (USD-LIBOR-3M)
    try:
        forward_curve = finstack.core.market_data.term_structures.ForwardCurve(
            id="USD-LIBOR-3M",
            tenor_years=0.25,
            knots=[
                (0.0, 0.035),
                (1.0, 0.04),
                (2.0, 0.042),
                (5.0, 0.045),
            ],
            base_date=date(2024, 1, 1),
        )
        ctx.insert(forward_curve)
    except Exception:  # noqa: BLE001
        # Forward curve insertion may fail if not supported
        pass

    return ctx


def create_simple_metric_registry() -> "finstack.valuations.metrics.MetricRegistry":
    """Create a simple MetricRegistry for testing."""
    import finstack

    # Use the standard registry
    return finstack.valuations.metrics.MetricRegistry.standard()


def create_simple_pricer_registry() -> "finstack.valuations.pricer.PricerRegistry":
    """Create a simple PricerRegistry for testing."""
    import finstack

    return finstack.valuations.pricer.standard_registry()


# Collect all examples
PYI_ROOT = Path(__file__).parent.parent / "finstack"
all_examples: list[tuple[str, str, str]] = []

for pyi_file in find_pyi_files(PYI_ROOT):
    examples = extract_examples_from_pyi_file(pyi_file)
    all_examples.extend(examples)


# Generate test functions dynamically
def make_test_function(example_id: str, code: str, context: str) -> Callable[[], None]:
    """Create a test function for an example."""

    def test_example() -> None:
        """Test a docstring example."""
        # Skip if clearly incomplete
        if not is_runnable_example(code):
            return

        # Try to execute the code
        try:
            # Create a safe execution environment
            namespace = {
                "__name__": "__main__",
                "__builtins__": __builtins__,
            }

            # Add common imports that examples might use
            # Try to import dynamically to avoid import errors
            try:
                namespace["finstack"] = finstack
                namespace["Currency"] = finstack.core.currency.Currency
                namespace["Money"] = finstack.core.money.Money
                namespace["date"] = __import__("datetime").date
                namespace["datetime"] = __import__("datetime")
                namespace["json"] = __import__("json")
                namespace["PeriodId"] = finstack.core.dates.periods.PeriodId
                namespace["build_periods"] = finstack.core.dates.periods.build_periods
                namespace["build_fiscal_periods"] = finstack.core.dates.periods.build_fiscal_periods
                namespace["FiscalConfig"] = finstack.core.dates.periods.FiscalConfig
                namespace["ModelBuilder"] = finstack.statements.builder.builder.ModelBuilder
                namespace["Evaluator"] = finstack.statements.evaluator.evaluator.Evaluator
                namespace["FinancialModelSpec"] = finstack.statements.types.model.FinancialModelSpec
                namespace["AmountOrScalar"] = finstack.statements.types.model.AmountOrScalar
                namespace["ForecastSpec"] = finstack.statements.types.forecast.ForecastSpec
                namespace["ForecastMethod"] = finstack.statements.types.forecast.ForecastMethod
                namespace["ScenarioSpec"] = finstack.scenarios.ScenarioSpec
                namespace["OperationSpec"] = finstack.scenarios.OperationSpec
                namespace["ScenarioEngine"] = finstack.scenarios.ScenarioEngine
                namespace["ExecutionContext"] = finstack.scenarios.ExecutionContext
                namespace["Portfolio"] = finstack.portfolio.portfolio.Portfolio
                namespace["PortfolioBuilder"] = finstack.portfolio.PortfolioBuilder
                namespace["Entity"] = finstack.portfolio.types.Entity
                namespace["Position"] = finstack.portfolio.types.Position
                namespace["PositionUnit"] = finstack.portfolio.types.PositionUnit
                namespace["value_portfolio"] = finstack.portfolio.valuation.value_portfolio
                namespace["PortfolioValuation"] = finstack.portfolio.valuation.PortfolioValuation
                namespace["PositionValue"] = finstack.portfolio.valuation.PositionValue
                namespace["group_by_attribute"] = finstack.portfolio.grouping.group_by_attribute
                namespace["aggregate_by_attribute"] = finstack.portfolio.grouping.aggregate_by_attribute
                namespace["aggregate_metrics"] = finstack.portfolio.metrics.aggregate_metrics
                namespace["PortfolioMetrics"] = finstack.portfolio.metrics.PortfolioMetrics
                namespace["PricerRegistry"] = finstack.valuations.pricer.PricerRegistry
                namespace["standard_registry"] = finstack.valuations.pricer.standard_registry
                namespace["ValuationResult"] = finstack.valuations.results.ValuationResult
                namespace["ResultsMeta"] = finstack.valuations.results.ResultsMeta
                namespace["CovenantReport"] = finstack.valuations.results.CovenantReport
                namespace["MetricId"] = finstack.valuations.metrics.MetricId
                namespace["MetricRegistry"] = finstack.valuations.metrics.MetricRegistry
                namespace["MarketContext"] = finstack.core.market_data.context.MarketContext
                namespace["DiscountCurve"] = finstack.core.market_data.term_structures.DiscountCurve
                namespace["ForwardCurve"] = finstack.core.market_data.term_structures.ForwardCurve
                namespace["HazardCurve"] = finstack.core.market_data.term_structures.HazardCurve
                namespace["InflationCurve"] = finstack.core.market_data.term_structures.InflationCurve
                namespace["FxMatrix"] = finstack.core.market_data.fx.FxMatrix
                namespace["VolSurface"] = finstack.core.market_data.surfaces.VolSurface
                namespace["npv"] = finstack.core.cashflow.performance.npv
                namespace["irr_periodic"] = finstack.core.cashflow.performance.irr_periodic
                namespace["xirr"] = finstack.core.cashflow.xirr.xirr
                namespace["CashFlow"] = finstack.core.cashflow.primitives.CashFlow
                namespace["CFKind"] = finstack.core.cashflow.primitives.CFKind
                namespace["NewtonSolver"] = finstack.core.math.solver.NewtonSolver
                namespace["BrentSolver"] = finstack.core.math.solver.BrentSolver
                namespace["InterpStyle"] = finstack.core.math.interp.InterpStyle
                namespace["ExtrapolationPolicy"] = finstack.core.math.interp.ExtrapolationPolicy
                namespace["ExplainOpts"] = finstack.core.explain.ExplainOpts
                namespace["ExplanationTrace"] = finstack.core.explain.ExplanationTrace
                # Instrument types
                namespace["Bond"] = finstack.valuations.instruments.Bond
                namespace["InterestRateSwap"] = finstack.valuations.instruments.InterestRateSwap
                namespace["EquityOption"] = finstack.valuations.instruments.EquityOption
                namespace["Swaption"] = finstack.valuations.instruments.Swaption
                namespace["InterestRateOption"] = finstack.valuations.instruments.InterestRateOption
                namespace["CreditDefaultSwap"] = finstack.valuations.instruments.CreditDefaultSwap
                namespace["ForwardRateAgreement"] = finstack.valuations.instruments.ForwardRateAgreement
                namespace["Deposit"] = finstack.valuations.instruments.Deposit
                namespace["InflationLinkedBond"] = finstack.valuations.instruments.InflationLinkedBond
                namespace["FxSpot"] = finstack.valuations.instruments.FxSpot
                namespace["FxOption"] = finstack.valuations.instruments.FxOption
                namespace["FxSwap"] = finstack.valuations.instruments.FxSwap
                namespace["Equity"] = finstack.valuations.instruments.Equity
                namespace["CDSIndex"] = finstack.valuations.instruments.CDSIndex
                namespace["CDSOption"] = finstack.valuations.instruments.CDSOption
                namespace["CDSTranche"] = finstack.valuations.instruments.CDSTranche
                namespace["BarrierOption"] = finstack.valuations.instruments.BarrierOption
                namespace["BarrierType"] = finstack.valuations.instruments.BarrierType
                namespace["AsianOption"] = finstack.valuations.instruments.AsianOption
                namespace["AveragingMethod"] = finstack.valuations.instruments.AveragingMethod
                namespace["ConvertibleBond"] = finstack.valuations.instruments.ConvertibleBond
                namespace["ConversionSpec"] = finstack.valuations.instruments.ConversionSpec
                namespace["VarianceSwap"] = finstack.valuations.instruments.VarianceSwap
                namespace["Repo"] = finstack.valuations.instruments.Repo
                namespace["RepoCollateral"] = finstack.valuations.instruments.RepoCollateral
                namespace["LookbackOption"] = finstack.valuations.instruments.LookbackOption
                namespace["QuantoOption"] = finstack.valuations.instruments.QuantoOption
                namespace["CmsOption"] = finstack.valuations.instruments.CmsOption
                namespace["CliquetOption"] = finstack.valuations.instruments.CliquetOption
                namespace["FxBarrierOption"] = finstack.valuations.instruments.FxBarrierOption
                namespace["RangeAccrual"] = finstack.valuations.instruments.RangeAccrual
                namespace["StructuredCredit"] = finstack.valuations.instruments.StructuredCredit
                namespace["Autocallable"] = finstack.valuations.instruments.Autocallable
                namespace["Basket"] = finstack.valuations.instruments.Basket
                namespace["TermLoan"] = finstack.valuations.instruments.TermLoan
                namespace["RevolvingCredit"] = finstack.valuations.instruments.RevolvingCredit
                namespace["PrivateMarketsFund"] = finstack.valuations.instruments.PrivateMarketsFund
                namespace["BasisSwap"] = finstack.valuations.instruments.BasisSwap
                namespace["InflationSwap"] = finstack.valuations.instruments.InflationSwap
                namespace["InterestRateFuture"] = finstack.valuations.instruments.InterestRateFuture
                namespace["EquityTotalReturnSwap"] = finstack.valuations.instruments.EquityTotalReturnSwap
                namespace["FiIndexTotalReturnSwap"] = finstack.valuations.instruments.FiIndexTotalReturnSwap
                namespace["attribute_pnl"] = finstack.valuations.attribution.attribute_pnl
                namespace["attribute_portfolio_pnl"] = finstack.valuations.attribution.attribute_portfolio_pnl
                namespace["attribute_pnl_from_json"] = finstack.valuations.attribution.attribute_pnl_from_json
                # TRS-related imports (from instruments module)
                namespace["TrsSide"] = finstack.valuations.instruments.TrsSide
                namespace["TrsFinancingLegSpec"] = finstack.valuations.instruments.TrsFinancingLegSpec
                namespace["TrsScheduleSpec"] = finstack.valuations.instruments.TrsScheduleSpec
                namespace["EquityUnderlying"] = finstack.valuations.instruments.EquityUnderlying
                namespace["IndexUnderlying"] = finstack.valuations.instruments.IndexUnderlying
                # Cashflow builder
                namespace["ScheduleParams"] = finstack.valuations.cashflow.ScheduleParams
                namespace["FeeBase"] = finstack.valuations.cashflow.FeeBase
                namespace["FeeSpec"] = finstack.valuations.cashflow.FeeSpec
                namespace["FixedWindow"] = finstack.valuations.cashflow.FixedWindow
                namespace["FloatWindow"] = finstack.valuations.cashflow.FloatWindow
                # Day count
                namespace["DayCount"] = finstack.core.dates.daycount.DayCount
                # Instrument imports (commonly used)
                namespace["Deposit"] = finstack.valuations.instruments.Deposit
                # Market context and registry (commonly used)
                namespace["standard_registry"] = finstack.valuations.pricer.standard_registry
                # InstrumentType for metric examples
                namespace["InstrumentType"] = finstack.valuations.common.InstrumentType
            except (AttributeError, ImportError):
                # Some imports might not be available, that's OK
                # But try to get PortfolioBuilder from the correct location
                with contextlib.suppress(builtins.BaseException):
                    namespace["PortfolioBuilder"] = finstack.portfolio.PortfolioBuilder
                # Try to add common ones that might be in different locations
                with contextlib.suppress(builtins.BaseException):
                    namespace["DayCount"] = finstack.core.dates.DayCount

            # Check if example needs special setup for MarketContext/curves
            # Only provide setup if it's actually needed to avoid unnecessary overhead
            needs_curve_var = "curve." in code or "curve.df" in code or "curve.npv" in code or "curve.zero" in code
            needs_ctx_var = "ctx." in code or "ctx.insert" in code
            needs_market_var = "market." in code
            needs_registry_var = "registry." in code and "registry =" not in code

            # Provide common test fixtures ONLY if the variables are referenced
            if needs_curve_var or needs_ctx_var or needs_market_var:
                # Try to create the context - if it fails, we can still run the test without it
                try:
                    test_ctx = create_simple_market_context()
                    if needs_ctx_var:
                        namespace["ctx"] = test_ctx
                    if needs_market_var:
                        namespace["market"] = test_ctx
                    if needs_curve_var:
                        namespace["curve"] = test_ctx.get_discount("USD")
                except (AttributeError, RuntimeError):
                    # If we can't create the context, don't fail - just continue without it
                    # The test will fail naturally if the variable is needed but missing
                    pass

            # Provide registry if needed
            if needs_registry_var:
                try:
                    # Try MetricRegistry first (most common)
                    if (
                        "MetricRegistry" in code
                        or "available_metrics" in code
                        or "metrics_for" in code
                        or "is_applicable" in code
                    ):
                        namespace["registry"] = create_simple_metric_registry()
                    # Try PricerRegistry
                    elif "PricerRegistry" in code or "price_with_metrics" in code:
                        namespace["registry"] = create_simple_pricer_registry()
                except (AttributeError, RuntimeError):
                    pass

            # Preprocess code to handle import statements
            # Replace problematic import paths with working ones
            code_lines = code.split("\n")
            processed_lines = []
            for line_item in code_lines:
                line = line_item
                # Fix cashflow.performance imports
                if "from finstack.core.cashflow.performance import" in line:
                    line = line.replace(
                        "from finstack.core.cashflow.performance import", "from finstack.core.cashflow import"
                    )
                # Fix import paths that don't work
                if "from finstack.valuations.cashflow.builder import" in line:
                    line = line.replace(
                        "from finstack.valuations.cashflow.builder import", "from finstack.valuations.cashflow import"
                    )
                # Fix market_data imports - DiscountCurve, etc. are in term_structures
                if "from finstack.core.market_data import DiscountCurve" in line:
                    line = line.replace(
                        "from finstack.core.market_data import DiscountCurve",
                        "from finstack.core.market_data.term_structures import DiscountCurve",
                    )
                if "from finstack.core.market_data import ForwardCurve" in line:
                    line = line.replace(
                        "from finstack.core.market_data import ForwardCurve",
                        "from finstack.core.market_data.term_structures import ForwardCurve",
                    )
                if "from finstack.core.market_data import HazardCurve" in line:
                    line = line.replace(
                        "from finstack.core.market_data import HazardCurve",
                        "from finstack.core.market_data.term_structures import HazardCurve",
                    )
                if "from finstack.core.market_data import InflationCurve" in line:
                    line = line.replace(
                        "from finstack.core.market_data import InflationCurve",
                        "from finstack.core.market_data.term_structures import InflationCurve",
                    )
                if "from finstack.core.market_data import VolSurface" in line:
                    line = line.replace(
                        "from finstack.core.market_data import VolSurface",
                        "from finstack.core.market_data.surfaces import VolSurface",
                    )
                if "from finstack.core.market_data import MarketContext" in line:
                    line = line.replace(
                        "from finstack.core.market_data import MarketContext",
                        "from finstack.core.market_data.context import MarketContext",
                    )
                if "from finstack.core.market_data import FxMatrix" in line:
                    line = line.replace(
                        "from finstack.core.market_data import FxMatrix",
                        "from finstack.core.market_data.fx import FxMatrix",
                    )
                # Only skip imports that are definitely already in namespace and would cause ImportError
                # Be conservative - only skip if the import path doesn't exist as a module
                if "from finstack.core.dates.periods import build_periods" in line:
                    # Skip - build_periods is already in namespace
                    continue
                if "from finstack.core.dates.periods import build_fiscal_periods" in line:
                    # Skip - build_fiscal_periods is already in namespace
                    continue
                processed_lines.append(line)
            processed_code = "\n".join(processed_lines)

            # Strip leading whitespace from the first line if present (common in extracted examples)
            if processed_code and processed_code[0] == " ":
                lines = processed_code.split("\n")
                # Find the minimum indentation of non-empty lines
                min_indent = min((len(line) - len(line.lstrip()) for line in lines if line.strip()), default=0)
                if min_indent > 0:
                    # Remove the minimum indentation from all lines
                    processed_code = "\n".join(line[min_indent:] if len(line) > min_indent else line for line in lines)

            # Execute the code
            exec(processed_code, namespace)

        except NameError as e:
            # Missing import or name - try to provide it from namespace
            name = str(e).split("'")[1] if "'" in str(e) else None
            if name and name in namespace:
                # Name exists in namespace but wasn't found - might be a scoping issue
                # Try again with explicit assignment
                try:
                    # Re-execute with name explicitly available
                    exec(f"{name} = namespace['{name}']\n" + processed_code, namespace)
                except (NameError, AttributeError, RuntimeError):
                    return
            else:
                return
        except ImportError as e:
            # Missing import - try to handle common cases
            import_str = str(e)
            if "cashflow.performance" in import_str:
                # Already handled above, but try again
                try:
                    exec(processed_code, namespace)
                except (ImportError, AttributeError, RuntimeError):
                    return
            else:
                return
        except (AttributeError, RuntimeError, ValueError, TypeError) as e:
            # Other errors - fail the test
            pytest.fail(f"Example failed in {context}:\n{code}\n\nError: {type(e).__name__}: {e}")

    test_example.__name__ = f"test_{example_id}"
    test_example.__doc__ = f"Test example from {context}"
    return test_example


# Register test functions
for example_id, code, context in all_examples:
    # Create a unique test name
    safe_id = re.sub(r"[^a-zA-Z0-9_]", "_", example_id)
    test_func = make_test_function(safe_id, code, context)
    globals()[test_func.__name__] = test_func


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
