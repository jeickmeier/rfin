"""Parity-style tests for factor-model Python exports."""

from finstack.portfolio import (
    FactorModelBuilder,
    FactorModelConfig,
    MarketMapping,
    MatchingConfig,
    factor_model as factor_model_module,
)


def test_factor_model_symbols_are_reexported_from_portfolio() -> None:
    assert FactorModelBuilder is factor_model_module.FactorModelBuilder
    assert FactorModelConfig is factor_model_module.FactorModelConfig
    assert MarketMapping is factor_model_module.MarketMapping
    assert MatchingConfig is factor_model_module.MatchingConfig
