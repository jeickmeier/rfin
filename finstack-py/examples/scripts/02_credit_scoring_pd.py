"""Credit scoring and PD calibration example.

Demonstrates the academic bankruptcy-prediction models (Altman Z family,
Ohlson O-Score, Zmijewski probit) and the Merton-Vasicek PiT / TtC
probability-of-default conversion, all exposed from the Rust
``finstack-core`` crate through the ``finstack.core.credit`` submodule.

Run:

    python finstack-py/examples/02_credit_scoring_pd.py
"""

from __future__ import annotations

import math

from finstack.core.credit import pd as credit_pd
from finstack.core.credit import scoring


def banner(title: str) -> None:
    """Print a labelled section separator."""
    print()
    print("=" * 72)
    print(f"  {title}")
    print("=" * 72)


def main() -> None:
    # -----------------------------------------------------------------
    # Sample company financials (in $ millions, except ratios)
    # -----------------------------------------------------------------
    total_assets = 1_000.0
    total_liabilities = 600.0
    current_assets = 400.0
    current_liabilities = 250.0
    working_capital = current_assets - current_liabilities  # 150
    retained_earnings = 200.0
    ebit = 120.0
    sales = 1_400.0
    market_equity = 900.0  # market cap
    book_equity = total_assets - total_liabilities  # 400
    net_income = 60.0
    funds_from_operations = 140.0

    # Pre-computed ratios
    wc_ta = working_capital / total_assets
    re_ta = retained_earnings / total_assets
    ebit_ta = ebit / total_assets
    sales_ta = sales / total_assets
    mkt_eq_liab = market_equity / total_liabilities
    bk_eq_liab = book_equity / total_liabilities
    tl_ta = total_liabilities / total_assets
    cl_ca = current_liabilities / current_assets
    ca_cl = current_assets / current_liabilities
    ni_ta = net_income / total_assets
    ffo_tl = funds_from_operations / total_liabilities

    banner("Sample company financials")
    print(f"  Total assets           : {total_assets:>10,.0f}")
    print(f"  Total liabilities      : {total_liabilities:>10,.0f}")
    print(f"  Working capital        : {working_capital:>10,.0f}")
    print(f"  Retained earnings      : {retained_earnings:>10,.0f}")
    print(f"  EBIT                   : {ebit:>10,.0f}")
    print(f"  Sales                  : {sales:>10,.0f}")
    print(f"  Market cap (equity)    : {market_equity:>10,.0f}")
    print(f"  Book equity            : {book_equity:>10,.0f}")
    print(f"  Net income             : {net_income:>10,.0f}")
    print(f"  FFO                    : {funds_from_operations:>10,.0f}")

    # -----------------------------------------------------------------
    # Altman Z-Score family
    # -----------------------------------------------------------------
    banner("Altman Z-Score family")

    z_score, z_zone, z_pd = scoring.altman_z_score(
        wc_ta, re_ta, ebit_ta, mkt_eq_liab, sales_ta
    )
    print(f"  Z-Score  (public mfg)    : {z_score:>7.3f}  zone={z_zone:<8s}  PD={z_pd:>6.2%}")

    zp_score, zp_zone, zp_pd = scoring.altman_z_prime(
        wc_ta, re_ta, ebit_ta, bk_eq_liab, sales_ta
    )
    print(f"  Z'-Score (private)       : {zp_score:>7.3f}  zone={zp_zone:<8s}  PD={zp_pd:>6.2%}")

    zdp_score, zdp_zone, zdp_pd = scoring.altman_z_double_prime(
        wc_ta, re_ta, ebit_ta, bk_eq_liab
    )
    print(f"  Z''-Score (emerging/nm)  : {zdp_score:>7.3f}  zone={zdp_zone:<8s}  PD={zdp_pd:>6.2%}")

    # -----------------------------------------------------------------
    # Ohlson O-Score (nine predictors)
    # -----------------------------------------------------------------
    banner("Ohlson O-Score (logistic)")

    o_score, o_zone, o_pd = scoring.ohlson_o_score(
        math.log(total_assets),   # log total assets (GNP-adjusted; proxy with ln)
        tl_ta,                    # total liabilities / total assets
        wc_ta,                    # working capital / total assets
        cl_ca,                    # current liab / current assets
        1.0 if total_liabilities > total_assets else 0.0,
        ni_ta,                    # ROA
        ffo_tl,                   # FFO / total liabilities
        0.0,                      # negative NI two years? 0 = no
        0.05,                     # change in net income (scaled)
    )
    print(f"  O-Score                  : {o_score:>7.3f}  zone={o_zone:<8s}  PD={o_pd:>6.2%}")

    # -----------------------------------------------------------------
    # Zmijewski probit (three predictors)
    # -----------------------------------------------------------------
    banner("Zmijewski probit")

    y_score, y_pd = scoring.zmijewski_score(ni_ta, tl_ta, ca_cl)
    print(f"  Zmijewski Y              : {y_score:>7.3f}  PD={y_pd:>6.2%}")

    # -----------------------------------------------------------------
    # PiT / TtC conversion round-trip (Merton-Vasicek ASRF)
    # -----------------------------------------------------------------
    banner("PiT / TtC conversion (Merton-Vasicek)")

    rho = 0.15           # asset correlation (Basel II corporate range)
    z_down = -1.5        # systematic downturn factor
    z_bene = +1.0        # benign / benign cycle
    ttc_pd = 0.020       # 2% long-run average PD

    pit_down = credit_pd.ttc_to_pit(ttc_pd, rho, z_down)
    pit_bene = credit_pd.ttc_to_pit(ttc_pd, rho, z_bene)
    print(f"  Input TtC PD             : {ttc_pd:>6.2%}")
    print(f"  rho = {rho:.2f}")
    print(f"  Downturn z={z_down:+.1f} -> PiT PD = {pit_down:>6.2%}  (stressed)")
    print(f"  Benign   z={z_bene:+.1f} -> PiT PD = {pit_bene:>6.2%}  (below TtC)")

    # Round-trip: PiT -> TtC -> PiT should be (approximately) identity.
    rt_ttc = credit_pd.pit_to_ttc(pit_down, rho, z_down)
    rt_pit = credit_pd.ttc_to_pit(rt_ttc, rho, z_down)
    print(f"  Round-trip: PiT({pit_down:.4%}) -> TtC({rt_ttc:.4%}) -> PiT({rt_pit:.4%})")
    assert abs(rt_ttc - ttc_pd) < 1e-10, "pit_to_ttc did not recover the original TtC PD"
    assert abs(rt_pit - pit_down) < 1e-10, "ttc_to_pit did not recover the original PiT PD"
    print("  Round-trip check passed.")

    # -----------------------------------------------------------------
    # Central tendency from historical default rates (geometric mean)
    # -----------------------------------------------------------------
    banner("Central-tendency calibration")

    history = [0.015, 0.022, 0.018, 0.030, 0.012, 0.020, 0.025]
    ct = credit_pd.central_tendency(history)
    print(f"  Observed annual rates    : {', '.join(f'{r:.2%}' for r in history)}")
    print(f"  Central tendency (GM)    : {ct:.4%}")


if __name__ == "__main__":
    main()
