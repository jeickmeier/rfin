"""Credit risk models: academic scoring, PD calibration, and LGD / EAD.

Bindings for ``finstack_core::credit``. Each submodule mirrors the Rust
module of the same name and is registered at runtime in ``sys.modules``
so that ``from finstack.core.credit import scoring`` (or ``pd``, ``lgd``)
works transparently.
"""

from __future__ import annotations

__all__ = ["scoring", "pd", "lgd"]

class scoring:
    """Academic credit scoring: Altman Z-Score family, Ohlson O-Score, Zmijewski."""

    @staticmethod
    def altman_z_score(
        working_capital_to_ta: float,
        retained_earnings_to_ta: float,
        ebit_to_ta: float,
        market_equity_to_book_liab: float,
        sales_to_ta: float,
    ) -> tuple[float, str, float]:
        """Original Altman Z-Score (1968) for publicly traded manufacturers.

        Returns ``(score, zone, implied_pd)`` where ``zone`` is one of
        ``"safe"``, ``"grey"``, ``"distress"``.
        """
        ...

    @staticmethod
    def altman_z_prime(
        working_capital_to_ta: float,
        retained_earnings_to_ta: float,
        ebit_to_ta: float,
        book_equity_to_book_liab: float,
        sales_to_ta: float,
    ) -> tuple[float, str, float]:
        """Altman Z'-Score for private firms. Returns ``(score, zone, implied_pd)``."""
        ...

    @staticmethod
    def altman_z_double_prime(
        working_capital_to_ta: float,
        retained_earnings_to_ta: float,
        ebit_to_ta: float,
        book_equity_to_book_liab: float,
    ) -> tuple[float, str, float]:
        """Altman Z''-Score for non-manufacturing / emerging markets.

        Returns ``(score, zone, implied_pd)``.
        """
        ...

    @staticmethod
    def ohlson_o_score(
        log_total_assets_adjusted: float,
        total_liab_to_ta: float,
        working_capital_to_ta: float,
        current_liab_to_current_assets: float,
        liab_exceed_assets: float,
        net_income_to_ta: float,
        ffo_to_total_liab: float,
        negative_ni_two_years: float,
        net_income_change: float,
    ) -> tuple[float, str, float]:
        """Ohlson O-Score (1980) logistic bankruptcy model.

        Returns ``(score, zone, implied_pd)``.
        """
        ...

    @staticmethod
    def zmijewski_score(
        roa: float,
        debt_ratio: float,
        current_ratio: float,
    ) -> tuple[float, float]:
        """Zmijewski (1984) probit bankruptcy score.

        Returns ``(score, implied_pd)``.
        """
        ...

class pd:
    """Probability of default: PiT/TtC conversion and central-tendency calibration."""

    @staticmethod
    def pit_to_ttc(pit_pd: float, asset_correlation: float, cycle_index: float) -> float:
        r"""Convert a Point-in-Time PD to Through-the-Cycle via Merton-Vasicek.

        ``PD_TtC = Phi( Phi^{-1}(PD_PiT) * sqrt(1 - rho) + sqrt(rho) * z )``.
        """
        ...

    @staticmethod
    def ttc_to_pit(ttc_pd: float, asset_correlation: float, cycle_index: float) -> float:
        r"""Convert a Through-the-Cycle PD to Point-in-Time via Merton-Vasicek.

        ``PD_PiT = Phi( (Phi^{-1}(PD_TtC) - sqrt(rho) * z) / sqrt(1 - rho) )``.
        """
        ...

    @staticmethod
    def central_tendency(annual_default_rates: list[float]) -> float:
        """Geometric-mean long-run PD from annual default rates (regulatory TtC)."""
        ...

class lgd:
    """Loss-given-default: seniority recovery, workout LGD, downturn adjustments, EAD."""

    @staticmethod
    def seniority_recovery_stats(
        seniority: str,
        rating_agency: str | None = None,
    ) -> dict[str, float]:
        """Historical recovery moments for a seniority class.

        If ``rating_agency`` is omitted, the Rust credit-assumptions registry
        default seniority calibration is used.

        Returns a dict with keys ``{"mean", "std", "alpha", "beta"}``.
        """
        ...

    @staticmethod
    def beta_recovery_sample(
        mean: float,
        std: float,
        n_samples: int,
        seed: int,
    ) -> list[float]:
        """Sample ``n_samples`` recoveries from Beta(alpha, beta) via PCG64."""
        ...

    @staticmethod
    def beta_recovery_quantile(mean: float, std: float, q: float) -> float:
        """Quantile ``q`` of a Beta recovery distribution parameterized by (mean, std)."""
        ...

    @staticmethod
    def workout_lgd(
        ead: float,
        collateral: list[tuple[str, float, float]],
        direct_cost_pct: float,
        indirect_cost_pct: float,
        time_to_resolution_years: float,
        discount_rate: float,
    ) -> tuple[float, float]:
        """Workout LGD from collateral waterfall, costs, and discounting.

        Returns ``(net_recovery, lgd)`` with ``lgd`` clamped to ``[0, 1]``.
        """
        ...

    @staticmethod
    def downturn_lgd_frye_jacobs(
        base_lgd: float,
        asset_correlation: float,
        stress_quantile: float,
    ) -> float:
        """Frye-Jacobs (2012) downturn LGD adjustment, clamped to ``[0, 1]``."""
        ...

    @staticmethod
    def downturn_lgd_regulatory_floor(
        base_lgd: float,
        add_on: float,
        floor: float,
    ) -> float:
        """Regulatory-floor downturn LGD: ``max(base + add_on, floor)`` clamped to ``[0, 1]``."""
        ...

    @staticmethod
    def ead_term_loan(principal: float) -> float:
        """Exposure at default for a fully drawn term loan (equal to principal)."""
        ...

    @staticmethod
    def ead_revolver(drawn: float, undrawn: float, ccf: float) -> float:
        """Exposure at default for a revolver: ``drawn + undrawn * ccf``."""
        ...
