"""Tests for ModelKey enum variants and string parsing."""

from finstack.valuations.common import ModelKey
import pytest


class TestModelKeyConstruction:
    """Test that all ModelKey enum variants can be constructed."""

    def test_discounting(self) -> None:
        """Test DISCOUNTING model key."""
        assert ModelKey.DISCOUNTING.name == "discounting"

    def test_tree(self) -> None:
        """Test TREE model key."""
        assert ModelKey.TREE.name == "tree"

    def test_black76(self) -> None:
        """Test BLACK76 model key."""
        assert ModelKey.BLACK76.name == "black76"

    def test_hull_white_1f(self) -> None:
        """Test HULL_WHITE_1F model key."""
        assert ModelKey.HULL_WHITE_1F.name == "hull_white_1f"

    def test_hazard_rate(self) -> None:
        """Test HAZARD_RATE model key."""
        assert ModelKey.HAZARD_RATE.name == "hazard_rate"

    def test_normal(self) -> None:
        """Test NORMAL model key."""
        assert ModelKey.NORMAL.name == "normal"

    def test_monte_carlo_gbm(self) -> None:
        """Test MONTE_CARLO_GBM model key."""
        assert ModelKey.MONTE_CARLO_GBM.name == "monte_carlo_gbm"

    def test_monte_carlo_heston(self) -> None:
        """Test MONTE_CARLO_HESTON model key."""
        assert ModelKey.MONTE_CARLO_HESTON.name == "monte_carlo_heston"

    def test_monte_carlo_hull_white_1f(self) -> None:
        """Test MONTE_CARLO_HULL_WHITE_1F model key."""
        assert ModelKey.MONTE_CARLO_HULL_WHITE_1F.name == "monte_carlo_hull_white_1f"

    def test_barrier_bs_continuous(self) -> None:
        """Test BARRIER_BS_CONTINUOUS model key."""
        assert ModelKey.BARRIER_BS_CONTINUOUS.name == "barrier_bs_continuous"

    def test_asian_geometric_bs(self) -> None:
        """Test ASIAN_GEOMETRIC_BS model key."""
        assert ModelKey.ASIAN_GEOMETRIC_BS.name == "asian_geometric_bs"

    def test_asian_turnbull_wakeman(self) -> None:
        """Test ASIAN_TURNBULL_WAKEMAN model key."""
        assert ModelKey.ASIAN_TURNBULL_WAKEMAN.name == "asian_turnbull_wakeman"

    def test_lookback_bs_continuous(self) -> None:
        """Test LOOKBACK_BS_CONTINUOUS model key."""
        assert ModelKey.LOOKBACK_BS_CONTINUOUS.name == "lookback_bs_continuous"

    def test_quanto_bs(self) -> None:
        """Test QUANTO_BS model key."""
        assert ModelKey.QUANTO_BS.name == "quanto_bs"

    def test_fx_barrier_bs_continuous(self) -> None:
        """Test FX_BARRIER_BS_CONTINUOUS model key."""
        assert ModelKey.FX_BARRIER_BS_CONTINUOUS.name == "fx_barrier_bs_continuous"

    def test_heston_fourier(self) -> None:
        """Test HESTON_FOURIER model key."""
        assert ModelKey.HESTON_FOURIER.name == "heston_fourier"


class TestModelKeyFromName:
    """Test parsing ModelKey from string names."""

    def test_from_name_discounting(self) -> None:
        """Test parsing 'discounting' string."""
        key = ModelKey.from_name("discounting")
        assert key == ModelKey.DISCOUNTING
        assert key.name == "discounting"

    def test_from_name_tree(self) -> None:
        """Test parsing 'tree' string."""
        key = ModelKey.from_name("tree")
        assert key == ModelKey.TREE

    def test_from_name_black76(self) -> None:
        """Test parsing 'black76' string."""
        key = ModelKey.from_name("black76")
        assert key == ModelKey.BLACK76

    def test_from_name_black76_alias(self) -> None:
        """Test parsing 'black' alias for black76."""
        key = ModelKey.from_name("black")
        assert key == ModelKey.BLACK76

    def test_from_name_hull_white_1f(self) -> None:
        """Test parsing 'hull_white_1f' string."""
        key = ModelKey.from_name("hull_white_1f")
        assert key == ModelKey.HULL_WHITE_1F

    def test_from_name_hull_white_1f_alias(self) -> None:
        """Test parsing 'hw1f' alias for hull_white_1f."""
        key = ModelKey.from_name("hw1f")
        assert key == ModelKey.HULL_WHITE_1F

    def test_from_name_normal(self) -> None:
        """Test parsing 'normal' string."""
        key = ModelKey.from_name("normal")
        assert key == ModelKey.NORMAL

    def test_from_name_normal_alias(self) -> None:
        """Test parsing 'bachelier' alias for normal."""
        key = ModelKey.from_name("bachelier")
        assert key == ModelKey.NORMAL

    def test_from_name_monte_carlo_gbm(self) -> None:
        """Test parsing 'monte_carlo_gbm' string."""
        key = ModelKey.from_name("monte_carlo_gbm")
        assert key == ModelKey.MONTE_CARLO_GBM

    def test_from_name_monte_carlo_gbm_alias(self) -> None:
        """Test parsing 'mc_gbm' alias."""
        key = ModelKey.from_name("mc_gbm")
        assert key == ModelKey.MONTE_CARLO_GBM

    def test_from_name_monte_carlo_heston(self) -> None:
        """Test parsing 'monte_carlo_heston' string."""
        key = ModelKey.from_name("monte_carlo_heston")
        assert key == ModelKey.MONTE_CARLO_HESTON

    def test_from_name_monte_carlo_heston_alias(self) -> None:
        """Test parsing 'mc_heston' alias."""
        key = ModelKey.from_name("mc_heston")
        assert key == ModelKey.MONTE_CARLO_HESTON

    def test_from_name_barrier_bs_continuous(self) -> None:
        """Test parsing 'barrier_bs_continuous' string."""
        key = ModelKey.from_name("barrier_bs_continuous")
        assert key == ModelKey.BARRIER_BS_CONTINUOUS

    def test_from_name_asian_geometric_bs(self) -> None:
        """Test parsing 'asian_geometric_bs' string."""
        key = ModelKey.from_name("asian_geometric_bs")
        assert key == ModelKey.ASIAN_GEOMETRIC_BS

    def test_from_name_asian_turnbull_wakeman(self) -> None:
        """Test parsing 'asian_turnbull_wakeman' string."""
        key = ModelKey.from_name("asian_turnbull_wakeman")
        assert key == ModelKey.ASIAN_TURNBULL_WAKEMAN

    def test_from_name_lookback_bs_continuous(self) -> None:
        """Test parsing 'lookback_bs_continuous' string."""
        key = ModelKey.from_name("lookback_bs_continuous")
        assert key == ModelKey.LOOKBACK_BS_CONTINUOUS

    def test_from_name_quanto_bs(self) -> None:
        """Test parsing 'quanto_bs' string."""
        key = ModelKey.from_name("quanto_bs")
        assert key == ModelKey.QUANTO_BS

    def test_from_name_fx_barrier_bs_continuous(self) -> None:
        """Test parsing 'fx_barrier_bs_continuous' string."""
        key = ModelKey.from_name("fx_barrier_bs_continuous")
        assert key == ModelKey.FX_BARRIER_BS_CONTINUOUS

    def test_from_name_heston_fourier(self) -> None:
        """Test parsing 'heston_fourier' string."""
        key = ModelKey.from_name("heston_fourier")
        assert key == ModelKey.HESTON_FOURIER

    def test_from_name_heston_fourier_alias(self) -> None:
        """Test parsing 'heston_analytical' alias."""
        key = ModelKey.from_name("heston_analytical")
        assert key == ModelKey.HESTON_FOURIER

    def test_from_name_invalid(self) -> None:
        """Test that invalid model name raises ValueError."""
        with pytest.raises(ValueError, match="Unknown model key"):
            ModelKey.from_name("invalid_model_name")


class TestModelKeyEquality:
    """Test equality and hashing for ModelKey."""

    def test_equality(self) -> None:
        """Test that same model keys are equal."""
        key1 = ModelKey.BLACK76
        key2 = ModelKey.from_name("black76")
        assert key1 == key2

    def test_inequality(self) -> None:
        """Test that different model keys are not equal."""
        key1 = ModelKey.BLACK76
        key2 = ModelKey.NORMAL
        assert key1 != key2

    def test_hash(self) -> None:
        """Test that model keys can be used in sets."""
        keys = {ModelKey.BLACK76, ModelKey.NORMAL, ModelKey.BLACK76}
        assert len(keys) == 2
        assert ModelKey.BLACK76 in keys
        assert ModelKey.NORMAL in keys


class TestModelKeyRepr:
    """Test string representations of ModelKey."""

    def test_repr(self) -> None:
        """Test __repr__ output."""
        key = ModelKey.BLACK76
        assert repr(key) == "ModelKey('black76')"

    def test_str(self) -> None:
        """Test __str__ output."""
        key = ModelKey.BLACK76
        assert str(key) == "black76"


class TestModelKeyWithPricerKey:
    """Test ModelKey usage with PricerKey."""

    def test_pricer_key_construction(self) -> None:
        """Test that ModelKey can be used to construct PricerKey."""
        from finstack.valuations.common import InstrumentType, PricerKey

        key = PricerKey(InstrumentType.EQUITY_OPTION, ModelKey.BLACK76)
        assert key.model == ModelKey.BLACK76
        assert key.instrument == InstrumentType.EQUITY_OPTION

    def test_pricer_key_with_string_model(self) -> None:
        """Test that string model names work with PricerKey."""
        from finstack.valuations.common import InstrumentType, PricerKey

        key = PricerKey(InstrumentType.ASIAN_OPTION, "asian_geometric_bs")
        assert key.model == ModelKey.ASIAN_GEOMETRIC_BS
        assert key.instrument == InstrumentType.ASIAN_OPTION


class TestAllModelKeysExist:
    """Test that all expected model keys are accessible."""

    def test_all_model_keys_accessible(self) -> None:
        """Verify all 16 model key variants are accessible."""
        expected_keys = [
            "DISCOUNTING",
            "TREE",
            "BLACK76",
            "HULL_WHITE_1F",
            "HAZARD_RATE",
            "NORMAL",
            "MONTE_CARLO_GBM",
            "MONTE_CARLO_HESTON",
            "MONTE_CARLO_HULL_WHITE_1F",
            "BARRIER_BS_CONTINUOUS",
            "ASIAN_GEOMETRIC_BS",
            "ASIAN_TURNBULL_WAKEMAN",
            "LOOKBACK_BS_CONTINUOUS",
            "QUANTO_BS",
            "FX_BARRIER_BS_CONTINUOUS",
            "HESTON_FOURIER",
        ]

        for key_name in expected_keys:
            assert hasattr(ModelKey, key_name), f"ModelKey.{key_name} not found"
            key = getattr(ModelKey, key_name)
            assert key.name == key_name.lower()
