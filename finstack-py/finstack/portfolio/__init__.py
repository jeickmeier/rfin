"""Portfolio module wrapper with Python-level compatibility helpers."""

from __future__ import annotations

import contextlib
import sys as _sys
import types as _types

from finstack import finstack as _finstack

_rust_portfolio = _finstack.portfolio

for _name in dir(_rust_portfolio):
    if _name.startswith("_"):
        continue
    _attr = getattr(_rust_portfolio, _name)
    globals()[_name] = _attr
    if isinstance(_attr, _types.ModuleType):
        _sys.modules[f"{__name__}.{_name}"] = _attr


_entity_container = globals().get("types")
_Entity = globals().get("Entity") if _entity_container is None else getattr(_entity_container, "Entity", None)

if _Entity is not None:

    def _entity_factory(entity_id: str, name: str | None = None) -> object:
        entity = _Entity(entity_id)
        if name is not None:
            with contextlib.suppress(Exception):
                entity = entity.with_name(name)
        return entity

    if hasattr(_Entity, "dummy"):
        _entity_factory.dummy = _Entity.dummy
    _entity_factory.__doc__ = _Entity.__doc__
    if _entity_container is not None:
        _entity_container.Entity = _entity_factory
    globals()["Entity"] = _entity_factory


_valuation_container = globals().get("valuation")
if _valuation_container is None:
    PortfolioValuation = globals().get("PortfolioValuation")
else:
    PortfolioValuation = getattr(_valuation_container, "PortfolioValuation", None)

if PortfolioValuation is not None:
    if not hasattr(PortfolioValuation, "total"):
        PortfolioValuation.total = property(lambda self: self.total_base_ccy)

    if not hasattr(PortfolioValuation, "entities"):
        PortfolioValuation.entities = property(lambda self: self.by_entity)

    if not hasattr(PortfolioValuation, "positions"):

        def _positions(self: object) -> list[object]:
            try:
                values = self.position_values
            except AttributeError:
                return []
            if isinstance(values, dict):
                return list(values.values())
            try:
                return list(values.values())
            except (AttributeError, TypeError):
                return list(values)

        PortfolioValuation.positions = property(_positions)


_builder_container = globals().get("builder")
if _builder_container is None:
    PortfolioBuilder = globals().get("PortfolioBuilder")
else:
    PortfolioBuilder = getattr(_builder_container, "PortfolioBuilder", None)

if PortfolioBuilder is not None:
    _build = PortfolioBuilder.build

    def _build_compat(self: object) -> object:
        try:
            return _build(self)
        except Exception as exc:
            msg = str(exc)
            lower = msg.lower()
            if "valid" not in lower and "error" not in lower:
                raise type(exc)(f"Validation error: {msg}") from exc
            raise

    PortfolioBuilder.build = _build_compat


if "aggregate_by_attribute" in globals():
    _agg_by_attribute = globals()["aggregate_by_attribute"]

    def aggregate_by_attribute(*args: object) -> object:
        """Aggregate positions by attribute tag."""
        if len(args) == 4:
            valuation, positions, key, _base_ccy = args
            totals: dict[str, object] = {}
            for position in positions:
                tags = getattr(position, "tags", None) or {}
                tag_value = tags.get(key)
                if tag_value is None:
                    continue
                pos_id = getattr(position, "position_id", None) or getattr(position, "id", None)
                pos_val = None
                if pos_id is not None and hasattr(valuation, "position_values"):
                    pos_val = valuation.position_values.get(pos_id)
                if pos_val is None and pos_id is not None and hasattr(valuation, "get_position_value"):
                    pos_val = valuation.get_position_value(pos_id)
                if pos_val is None:
                    continue
                value = getattr(pos_val, "value_base", None) or getattr(pos_val, "value_native", None)
                if value is None:
                    continue
                if tag_value in totals:
                    totals[tag_value] = totals[tag_value] + value
                else:
                    totals[tag_value] = value
            return totals
        return _agg_by_attribute(*args)

    globals()["aggregate_by_attribute"] = aggregate_by_attribute
