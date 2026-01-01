"""DSL parser for scenario specification.

This module provides a simple text-based DSL for defining scenarios without
manually constructing ScenarioSpec objects. The DSL supports common operations
like curve shifts, equity shocks, and time roll-forwards.

Examples
--------
>>> from finstack.scenarios import ScenarioSpec
>>> scenario = ScenarioSpec.from_dsl('''
...     shift USD.OIS +50bp
...     shift equities -10%
...     roll forward 1m
... ''')

Syntax
------
The DSL supports the following operations:

**Curve Shifts**:
- `shift <CURVE_ID> +/-<VALUE>bp` - Parallel basis point shift
- `shift discount <CURVE_ID> +/-<VALUE>bp` - Discount curve shift
- `shift forward <CURVE_ID> +/-<VALUE>bp` - Forward curve shift
- `shift hazard <CURVE_ID> +/-<VALUE>bp` - Hazard (credit) curve shift
- `shift inflation <CURVE_ID> +/-<VALUE>bp` - Inflation curve shift

**Equity Shocks**:
- `shift equities +/-<VALUE>%` - All equities percent shock
- `shift equity <ID> +/-<VALUE>%` - Single equity percent shock

**FX Shocks**:
- `shift fx <BASE>/<QUOTE> +/-<VALUE>%` - FX rate percent shock

**Vol Surface Shocks**:
- `shift vol <ID> +/-<VALUE>%` - Vol surface parallel percent shock

**Time Operations**:
- `roll forward <VALUE><UNIT>` - Roll forward time (1d, 1w, 1m, 3m, 1y)

**Statement Operations**:
- `adjust <NODE_ID> +/-<VALUE>%` - Statement forecast percent change
- `set <NODE_ID> <VALUE>` - Statement forecast assignment

Notes
-----
- Commands are case-insensitive and whitespace-tolerant
- Multiple operations can be separated by semicolons or newlines
- Comments start with # and extend to end of line
"""

import re
from typing import List, Optional, Tuple

from finstack import Currency  # type: ignore
from finstack.scenarios import (  # type: ignore
    CurveKind,
    OperationSpec,
    ScenarioSpec,
)


class DSLParseError(Exception):
    """Error raised when DSL parsing fails."""

    def __init__(self, message: str, line: Optional[int] = None, text: Optional[str] = None):
        self.message = message
        self.line = line
        self.text = text
        super().__init__(self._format_message())

    def _format_message(self) -> str:
        msg = self.message
        if self.line is not None:
            msg = f"Line {self.line}: {msg}"
        if self.text is not None:
            msg = f"{msg}\n  > {self.text.strip()}"
        return msg


class DSLParser:
    """Parser for scenario DSL text.

    This class converts DSL text into OperationSpec objects that can be
    used to construct a ScenarioSpec.

    Parameters
    ----------
    text : str
        DSL text to parse.

    Attributes
    ----------
    operations : list[OperationSpec]
        Parsed operations.
    """

    def __init__(self, text: str):
        self.text = text
        self.operations: List[OperationSpec] = []
        self._parse()

    def _parse(self) -> None:
        """Parse DSL text into operations."""
        lines = self._preprocess(self.text)
        for line_num, line in enumerate(lines, 1):
            if not line:
                continue
            try:
                ops = self._parse_line(line)
                self.operations.extend(ops)
            except Exception as e:
                raise DSLParseError(str(e), line_num, line) from e

    def _preprocess(self, text: str) -> List[str]:
        """Preprocess text: remove comments, split on semicolons."""
        lines = []
        for line in text.split('\n'):
            # Remove comments
            line = line.split('#')[0].strip()
            if not line:
                continue
            # Split on semicolons
            for subline in line.split(';'):
                subline = subline.strip()
                if subline:
                    lines.append(subline)
        return lines

    def _parse_line(self, line: str) -> List[OperationSpec]:
        """Parse a single line into operations."""
        # Normalize whitespace
        line = ' '.join(line.split())

        # Match operation type
        if re.match(r'^shift\s+', line, re.I):
            return [self._parse_shift(line)]
        elif re.match(r'^roll\s+forward\s+', line, re.I):
            return [self._parse_roll_forward(line)]
        elif re.match(r'^adjust\s+', line, re.I):
            return [self._parse_adjust(line)]
        elif re.match(r'^set\s+', line, re.I):
            return [self._parse_set(line)]
        else:
            raise ValueError(f"Unknown operation: {line}")

    def _parse_shift(self, line: str) -> OperationSpec:
        """Parse shift operations."""
        # Curve shifts: shift [curve_kind] <CURVE_ID> +/-<VALUE>bp
        # Example: shift discount USD.OIS +50bp
        m = re.match(
            r'^shift\s+(discount|forward|hazard|inflation)\s+(\S+)\s+([-+]?\d+(?:\.\d+)?)bp$',
            line,
            re.I,
        )
        if m:
            curve_kind_str, curve_id, bp_str = m.groups()
            curve_kind = self._parse_curve_kind(curve_kind_str)
            bp = float(bp_str)
            return OperationSpec.curve_parallel_bp(curve_kind, curve_id, bp)

        # Default curve shift: shift <CURVE_ID> +/-<VALUE>bp
        # Example: shift USD.OIS +50bp
        m = re.match(r'^shift\s+(\S+)\s+([-+]?\d+(?:\.\d+)?)bp$', line, re.I)
        if m:
            curve_id, bp_str = m.groups()
            bp = float(bp_str)
            # Default to discount curve
            return OperationSpec.curve_parallel_bp(CurveKind.Discount, curve_id, bp)

        # Equity shifts: shift equities +/-<VALUE>%
        m = re.match(r'^shift\s+equities\s+([-+]?\d+(?:\.\d+)?)%$', line, re.I)
        if m:
            pct = float(m.group(1))
            return OperationSpec.equity_price_pct([], pct)

        # Single equity shift: shift equity <ID> +/-<VALUE>%
        m = re.match(r'^shift\s+equity\s+(\S+)\s+([-+]?\d+(?:\.\d+)?)%$', line, re.I)
        if m:
            equity_id, pct_str = m.groups()
            pct = float(pct_str)
            return OperationSpec.equity_price_pct([equity_id], pct)

        # FX shift: shift fx <BASE>/<QUOTE> +/-<VALUE>%
        m = re.match(r'^shift\s+fx\s+([A-Z]{3})/([A-Z]{3})\s+([-+]?\d+(?:\.\d+)?)%$', line, re.I)
        if m:
            base_str, quote_str, pct_str = m.groups()
            base = Currency.from_code(base_str.upper())
            quote = Currency.from_code(quote_str.upper())
            pct = float(pct_str)
            return OperationSpec.market_fx_pct(base, quote, pct)

        # Vol surface shift: shift vol <ID> +/-<VALUE>%
        m = re.match(r'^shift\s+vol\s+(\S+)\s+([-+]?\d+(?:\.\d+)?)%$', line, re.I)
        if m:
            vol_id, pct_str = m.groups()
            pct = float(pct_str)
            # Default to equity vol surface
            from finstack.scenarios import VolSurfaceKind  # type: ignore

            return OperationSpec.vol_surface_parallel_pct(
                VolSurfaceKind.Equity, vol_id, pct
            )

        raise ValueError(f"Invalid shift syntax: {line}")

    def _parse_roll_forward(self, line: str) -> OperationSpec:
        """Parse roll forward operations."""
        # roll forward <VALUE><UNIT>
        # Example: roll forward 1m
        m = re.match(r'^roll\s+forward\s+(\d+)([dwmy])$', line, re.I)
        if m:
            value_str, unit = m.groups()
            value = int(value_str)

            # Convert to period string
            unit_map = {'d': 'd', 'w': 'w', 'm': 'm', 'y': 'y'}
            period = f"{value}{unit_map[unit.lower()]}"

            return OperationSpec.time_roll_forward(period)

        raise ValueError(f"Invalid roll forward syntax: {line}")

    def _parse_adjust(self, line: str) -> OperationSpec:
        """Parse statement forecast percent adjustments."""
        # adjust <NODE_ID> +/-<VALUE>%
        # Example: adjust revenue +10%
        m = re.match(r'^adjust\s+(\S+)\s+([-+]?\d+(?:\.\d+)?)%$', line, re.I)
        if m:
            node_id, pct_str = m.groups()
            pct = float(pct_str)
            return OperationSpec.stmt_forecast_percent(node_id, None, pct)

        raise ValueError(f"Invalid adjust syntax: {line}")

    def _parse_set(self, line: str) -> OperationSpec:
        """Parse statement forecast assignments."""
        # set <NODE_ID> <VALUE>
        # Example: set revenue 1000000
        m = re.match(r'^set\s+(\S+)\s+([-+]?\d+(?:\.\d+)?)$', line, re.I)
        if m:
            node_id, value_str = m.groups()
            value = float(value_str)
            return OperationSpec.stmt_forecast_assign(node_id, None, value)

        raise ValueError(f"Invalid set syntax: {line}")

    def _parse_curve_kind(self, kind_str: str) -> CurveKind:
        """Parse curve kind from string."""
        kind_map = {
            'discount': CurveKind.Discount,
            'forward': CurveKind.Forward,
            'hazard': CurveKind.Hazard,
            'inflation': CurveKind.Inflation,
        }
        kind = kind_map.get(kind_str.lower())
        if kind is None:
            raise ValueError(f"Unknown curve kind: {kind_str}")
        return kind


def from_dsl(text: str, scenario_id: str = "dsl_scenario", **kwargs) -> ScenarioSpec:
    """Parse DSL text into a ScenarioSpec.

    Parameters
    ----------
    text : str
        DSL text to parse.
    scenario_id : str, optional
        Scenario identifier (default: "dsl_scenario").
    **kwargs
        Additional arguments passed to ScenarioSpec constructor (name, description, priority).

    Returns
    -------
    ScenarioSpec
        Parsed scenario specification.

    Raises
    ------
    DSLParseError
        If parsing fails.

    Examples
    --------
    >>> scenario = from_dsl('''
    ...     shift USD.OIS +50bp
    ...     shift equities -10%
    ...     roll forward 1m
    ... ''', scenario_id="stress_test", name="Q1 Stress")
    """
    parser = DSLParser(text)
    return ScenarioSpec(scenario_id, parser.operations, **kwargs)


# Monkey-patch ScenarioSpec to add from_dsl class method
_original_scenario_spec = ScenarioSpec


def _add_from_dsl_to_spec():
    """Add from_dsl class method to ScenarioSpec."""

    @classmethod
    def from_dsl_method(cls, text: str, scenario_id: str = "dsl_scenario", **kwargs):
        """Parse DSL text into a ScenarioSpec.

        This is a convenience method that wraps the standalone from_dsl function.

        Parameters
        ----------
        text : str
            DSL text to parse.
        scenario_id : str, optional
            Scenario identifier (default: "dsl_scenario").
        **kwargs
            Additional arguments (name, description, priority).

        Returns
        -------
        ScenarioSpec
            Parsed scenario specification.

        Raises
        ------
        DSLParseError
            If parsing fails.

        Examples
        --------
        >>> scenario = ScenarioSpec.from_dsl('''
        ...     shift USD.OIS +50bp
        ...     shift equities -10%
        ... ''')
        """
        return from_dsl(text, scenario_id, **kwargs)

    # Add method to class
    ScenarioSpec.from_dsl = from_dsl_method  # type: ignore


# Apply monkey-patch when module is imported
_add_from_dsl_to_spec()
