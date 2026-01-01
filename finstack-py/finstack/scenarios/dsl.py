"""Domain-specific language parser for scenario specifications.

This module provides a simple DSL for defining scenario operations
in a human-readable text format.

Supported Syntax
----------------
Curve shifts:
    shift USD.OIS +50bp              # Discount curve (default)
    shift discount USD.OIS +50bp     # Explicit discount
    shift forward USD.SOFR -25bp     # Forward curve
    shift hazard ACME.5Y +100bp      # Hazard/credit curve
    shift inflation US.CPI +10bp     # Inflation curve

Equity shifts:
    shift equities -10%              # All equities
    shift equity SPY +5%             # Single equity

FX shifts:
    shift fx USD/EUR +3%             # FX pair

Volatility shifts:
    shift vol SPX_VOL +15%           # Vol surface

Time operations:
    roll forward 1d                  # Days
    roll forward 1w                  # Weeks
    roll forward 1m                  # Months
    roll forward 3m                  # Months
    roll forward 1y                  # Years

Statement operations:
    adjust revenue +10%              # Percentage adjustment
    set revenue 1000000              # Absolute value

Comments:
    # This is a comment

Multiple operations can be separated by newlines or semicolons.

Examples:
--------
    >>> from finstack.scenarios.dsl import from_dsl
    >>> scenario = from_dsl('''
    ...     shift USD.OIS +50bp
    ...     shift equities -10%
    ...     roll forward 1m
    ... ''')
"""

from __future__ import annotations

import re

from finstack import Currency

# Import the Rust types
try:
    from finstack.finstack.scenarios import CurveKind, OperationSpec, ScenarioSpec, VolSurfaceKind
except ImportError:
    try:
        from finstack.scenarios import (
            CurveKind,
            OperationSpec,
            ScenarioSpec,
            VolSurfaceKind,
        )
    except ImportError:
        # For type checking
        CurveKind = None
        OperationSpec = None
        ScenarioSpec = None
        VolSurfaceKind = None


class DSLParseError(Exception):
    """Error raised when DSL parsing fails."""

    def __init__(self, message: str, line: int | None = None) -> None:
        """Create a parse error.

        Parameters
        ----------
        message : str
            Error message.
        line : int, optional
            Line number where error occurred.
        """
        self.line = line
        if line is not None:
            super().__init__(f"Line {line}: {message}")
        else:
            super().__init__(message)


class DSLParser:
    """Parser for scenario DSL.

    Parses a DSL string into a list of OperationSpec objects.

    Parameters
    ----------
    dsl_text : str
        The DSL text to parse.

    Attributes:
    ----------
    operations : list of OperationSpec
        The parsed operations.

    Examples:
    --------
        >>> parser = DSLParser("shift USD.OIS +50bp")
        >>> len(parser.operations)
        1
    """

    # Regex patterns
    _BP_PATTERN = re.compile(r"([+-]?\d+(?:\.\d+)?)\s*bp", re.IGNORECASE)
    _PCT_PATTERN = re.compile(r"([+-]?\d+(?:\.\d+)?)\s*%", re.IGNORECASE)
    _NUMBER_PATTERN = re.compile(r"([+-]?\d+(?:\.\d+)?)")
    _TENOR_PATTERN = re.compile(r"(\d+)\s*([dwmy])", re.IGNORECASE)
    _FX_PAIR_PATTERN = re.compile(r"([A-Z]{3})/([A-Z]{3})", re.IGNORECASE)

    def __init__(self, dsl_text: str) -> None:
        """Parse the DSL text.

        Parameters
        ----------
        dsl_text : str
            The DSL text to parse.
        """
        self.operations: list[OperationSpec] = []
        self._parse(dsl_text)

    def _parse(self, dsl_text: str) -> None:
        """Parse the DSL text into operations."""
        # Split by semicolons and newlines
        lines = dsl_text.replace(";", "\n").split("\n")

        for line_num, raw_line in enumerate(lines, 1):
            line = raw_line
            # Remove comments
            if "#" in line:
                line = line[: line.index("#")]

            # Strip whitespace
            line = line.strip()

            # Skip empty lines
            if not line:
                continue

            # Parse the line
            try:
                op = self._parse_line(line)
                if op is not None:
                    self.operations.append(op)
            except DSLParseError as e:
                raise DSLParseError(str(e), line_num) from e
            except Exception as e:
                raise DSLParseError(str(e), line_num) from e

    def _parse_line(self, line: str) -> OperationSpec | None:
        """Parse a single line into an operation."""
        tokens = line.lower().split()
        if not tokens:
            return None

        command = tokens[0]

        if command == "shift":
            return self._parse_shift(line, tokens[1:])
        elif command == "roll":
            return self._parse_roll(line, tokens[1:])
        elif command == "adjust":
            return self._parse_adjust(line, tokens[1:])
        elif command == "set":
            return self._parse_set(line, tokens[1:])
        else:
            raise DSLParseError(f"Unknown operation: {command}")

    def _parse_shift(self, original_line: str, tokens: list[str]) -> OperationSpec | None:
        """Parse a shift operation."""
        if not tokens:
            raise DSLParseError("Invalid shift syntax: missing arguments")

        # Check for curve kind prefix
        # Map common names to CurveKind enum values
        curve_kinds = {
            "discount": CurveKind.Discount,
            "forward": CurveKind.Forecast,  # 'forward' in DSL maps to Forecast curve kind
            "hazard": CurveKind.ParCDS,  # 'hazard' in DSL maps to ParCDS curve kind
            "inflation": CurveKind.Inflation,
        }

        first_part = tokens[0]

        # Handle "shift equities" or "shift equity"
        if first_part in ("equities", "equity"):
            return self._parse_equity_shift(original_line, tokens[1:], first_part)

        # Handle "shift fx"
        if first_part == "fx":
            return self._parse_fx_shift(original_line, tokens[1:])

        # Handle "shift vol"
        if first_part in ("vol", "volatility"):
            return self._parse_vol_shift(original_line, tokens[1:])

        # Handle curve shifts
        curve_kind = CurveKind.Discount  # Default
        remaining_tokens = tokens

        if first_part in curve_kinds:
            curve_kind = curve_kinds[first_part]
            remaining_tokens = tokens[1:]

        if len(remaining_tokens) < 2:
            raise DSLParseError(f"Invalid shift syntax: expected curve_id and value in '{original_line}'")

        curve_id = remaining_tokens[0].upper()
        value_str = " ".join(remaining_tokens[1:])

        # Parse basis points
        bp_match = self._BP_PATTERN.search(value_str)
        if bp_match:
            bp_value = float(bp_match.group(1))
            return OperationSpec.curve_parallel_bp(curve_kind, curve_id, bp_value)

        raise DSLParseError(f"Invalid shift syntax: could not parse value in '{original_line}'")

    def _parse_equity_shift(self, original_line: str, tokens: list[str], shift_type: str) -> OperationSpec:
        """Parse an equity shift operation."""
        if not tokens:
            raise DSLParseError(f"Invalid shift syntax: missing value in '{original_line}'")

        # Check if there's a symbol first (for single equity)
        if shift_type == "equity" and len(tokens) >= 2:
            symbol = tokens[0].upper()
            value_str = " ".join(tokens[1:])
        else:
            symbol = None
            value_str = " ".join(tokens)

        # Parse percentage
        pct_match = self._PCT_PATTERN.search(value_str)
        if pct_match:
            pct_value = float(pct_match.group(1))
            ids = [symbol] if symbol else ["*"]
            return OperationSpec.equity_price_pct(ids, pct_value)

        raise DSLParseError(f"Invalid shift syntax: could not parse percentage in '{original_line}'")

    def _parse_fx_shift(self, original_line: str, tokens: list[str]) -> OperationSpec:
        """Parse an FX shift operation."""
        if len(tokens) < 2:
            raise DSLParseError(f"Invalid shift syntax: expected FX pair and value in '{original_line}'")

        pair_str = tokens[0].upper()
        value_str = " ".join(tokens[1:])

        # Parse FX pair
        fx_match = self._FX_PAIR_PATTERN.match(pair_str)
        if not fx_match:
            raise DSLParseError(f"Invalid shift syntax: invalid FX pair format in '{original_line}'")

        base_ccy = fx_match.group(1)
        quote_ccy = fx_match.group(2)

        # Parse percentage
        pct_match = self._PCT_PATTERN.search(value_str)
        if pct_match:
            pct_value = float(pct_match.group(1))
            base = Currency(base_ccy) if Currency is not None else base_ccy
            quote = Currency(quote_ccy) if Currency is not None else quote_ccy
            return OperationSpec.market_fx_pct(base, quote, pct_value)

        raise DSLParseError(f"Invalid shift syntax: could not parse percentage in '{original_line}'")

    def _parse_vol_shift(self, original_line: str, tokens: list[str]) -> OperationSpec:
        """Parse a volatility surface shift operation."""
        if len(tokens) < 2:
            raise DSLParseError(f"Invalid shift syntax: expected surface_id and value in '{original_line}'")

        surface_id = tokens[0].upper()
        value_str = " ".join(tokens[1:])

        # Parse percentage
        pct_match = self._PCT_PATTERN.search(value_str)
        if pct_match:
            pct_value = float(pct_match.group(1))
            return OperationSpec.vol_surface_parallel_pct(VolSurfaceKind.Equity, surface_id, pct_value)

        raise DSLParseError(f"Invalid shift syntax: could not parse percentage in '{original_line}'")

    def _parse_roll(self, original_line: str, tokens: list[str]) -> OperationSpec | None:
        """Parse a roll forward operation."""
        if len(tokens) < 2 or tokens[0] != "forward":
            raise DSLParseError(f"Invalid roll forward syntax: expected 'roll forward <tenor>' in '{original_line}'")

        tenor_str = tokens[1]
        tenor_match = self._TENOR_PATTERN.match(tenor_str)

        if not tenor_match:
            raise DSLParseError(f"Invalid roll forward syntax: invalid tenor format '{tenor_str}'")

        return OperationSpec.time_roll_forward(tenor_str, True, None)

    def _parse_adjust(self, original_line: str, tokens: list[str]) -> OperationSpec | None:
        """Parse a statement adjustment operation."""
        if len(tokens) < 2:
            raise DSLParseError(f"Invalid adjust syntax: expected metric and value in '{original_line}'")

        metric = tokens[0]
        value_str = " ".join(tokens[1:])

        # Parse percentage
        pct_match = self._PCT_PATTERN.search(value_str)
        if pct_match:
            pct_value = float(pct_match.group(1))
            return OperationSpec.stmt_forecast_percent(metric, pct_value)

        raise DSLParseError(f"Invalid adjust syntax: could not parse percentage in '{original_line}'")

    def _parse_set(self, original_line: str, tokens: list[str]) -> OperationSpec | None:
        """Parse a statement set operation."""
        if len(tokens) < 2:
            raise DSLParseError(f"Invalid set syntax: expected metric and value in '{original_line}'")

        metric = tokens[0]
        value_str = " ".join(tokens[1:])

        # Parse number
        num_match = self._NUMBER_PATTERN.search(value_str)
        if num_match:
            value = float(num_match.group(1))
            return OperationSpec.stmt_forecast_assign(metric, value)

        raise DSLParseError(f"Invalid set syntax: could not parse value in '{original_line}'")


def from_dsl(
    dsl_text: str,
    scenario_id: str = "dsl_scenario",
    name: str | None = None,
    description: str | None = None,
    priority: int = 0,
) -> ScenarioSpec:
    """Create a scenario from DSL text.

    Parameters
    ----------
    dsl_text : str
        The DSL text defining operations.
    scenario_id : str, default "dsl_scenario"
        Unique identifier for the scenario.
    name : str, optional
        Human-readable name.
    description : str, optional
        Detailed description.
    priority : int, default 0
        Execution priority.

    Returns:
    -------
    ScenarioSpec
        The created scenario specification.

    Examples:
    --------
        >>> scenario = from_dsl("shift USD.OIS +50bp")
        >>> scenario.id()
        'dsl_scenario'
    """
    parser = DSLParser(dsl_text)
    return ScenarioSpec(
        scenario_id,  # First positional argument is 'id'
        parser.operations,
        name=name,
        description=description,
        priority=priority,
    )


__all__ = ["DSLParseError", "DSLParser", "from_dsl"]
