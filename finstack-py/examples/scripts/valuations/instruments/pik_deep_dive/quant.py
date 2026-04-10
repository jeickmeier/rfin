"""Quantitative computation helpers."""

from __future__ import annotations

import math

from .constants import COUPON, MATURITY, NOTIONAL, RISK_FREE


def calibrate_hazard(spread: float, recovery: float) -> float:
    """Bisect for flat hazard rate matching a Z-spread."""
    times = [i / 2.0 for i in range(1, MATURITY * 2 + 1)]
    cpn = COUPON / 2 * NOTIONAL
    target = sum(cpn * math.exp(-(RISK_FREE + spread) * t) for t in times)
    target += NOTIONAL * math.exp(-(RISK_FREE + spread) * MATURITY)

    def _pv(h):
        pv, prev_s = 0.0, 1.0
        for t in times:
            df = math.exp(-RISK_FREE * t)
            s = math.exp(-h * t)
            pv += cpn * df * s
            pv += recovery * NOTIONAL * df * (prev_s - s)
            prev_s = s
        pv += NOTIONAL * math.exp(-RISK_FREE * MATURITY) * math.exp(
            -h * MATURITY)
        return pv

    lo, hi = 0.0, 5.0
    for _ in range(200):
        mid = (lo + hi) / 2
        if _pv(mid) > target:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def hr_bond_price(hazard: float, recovery: float, coupon_type: str = "cash",
                  coupon_rate: float = COUPON, maturity: float = MATURITY
                  ) -> float:
    """Price a bond under flat hazard rate. coupon_type: cash or pik."""
    times = [i / 2.0 for i in range(1, int(maturity * 2) + 1)]
    semi_cpn = coupon_rate / 2

    pv, prev_s = 0.0, 1.0
    notional = NOTIONAL
    for t in times:
        df = math.exp(-RISK_FREE * t)
        s = math.exp(-hazard * t)
        if coupon_type == "cash":
            pv += semi_cpn * NOTIONAL * df * s
        else:  # pik: coupon accretes
            notional *= (1 + semi_cpn)
        pv += recovery * NOTIONAL * df * (prev_s - s)
        prev_s = s
    # terminal
    df_T = math.exp(-RISK_FREE * maturity)
    s_T = math.exp(-hazard * maturity)
    pv += notional * df_T * s_T
    return pv


def price_to_zspread(price: float, coupon_rate: float = COUPON,
                     maturity: float = MATURITY) -> float:
    """Bisect for Z-spread that reproduces a given clean price."""
    times = [i / 2.0 for i in range(1, int(maturity * 2) + 1)]
    cpn = coupon_rate / 2 * NOTIONAL

    def _pv(z):
        return (sum(cpn * math.exp(-(RISK_FREE + z) * t) for t in times)
                + NOTIONAL * math.exp(-(RISK_FREE + z) * maturity))

    lo, hi = -0.5, 5.0
    for _ in range(200):
        mid = (lo + hi) / 2
        if _pv(mid) > price:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def _norm_cdf(x: float) -> float:
    """Standard normal CDF via math.erfc (no scipy needed)."""
    return 0.5 * math.erfc(-x / math.sqrt(2))


def _norm_ppf(p: float) -> float:
    """Inverse standard normal CDF via rational approximation."""
    if p <= 0:
        return -10.0
    if p >= 1:
        return 10.0
    # Beasley-Springer-Moro algorithm
    a = [0, -3.969683028665376e+01, 2.209460984245205e+02,
         -2.759285104469687e+02, 1.383577518672690e+02,
         -3.066479806614716e+01, 2.506628277459239e+00]
    b = [0, -5.447609879822406e+01, 1.615858368580409e+02,
         -1.556989798598866e+02, 6.680131188771972e+01,
         -1.328068155288572e+01]
    c = [0, -7.784894002430293e-03, -3.223964580411365e-01,
         -2.400758277161838e+00, -2.549732539343734e+00,
         4.374664141464968e+00, 2.938163982698783e+00]
    d = [0, 7.784695709041462e-03, 3.224671290700398e-01,
         2.445134137142996e+00, 3.754408661907416e+00]

    p_low, p_high = 0.02425, 1 - 0.02425
    if p < p_low:
        q = math.sqrt(-2 * math.log(p))
        return ((((c[5]*q + c[4])*q + c[3])*q + c[2])*q + c[1]) / \
               (((d[4]*q + d[3])*q + d[2])*q + d[1]*q + 1)
    elif p <= p_high:
        q = p - 0.5
        r = q * q
        return ((((a[5]*r + a[4])*r + a[3])*r + a[2])*r + a[1]) / \
               ((((b[5]*r + b[4])*r + b[3])*r + b[2])*r + b[1]*r + 1) * q
    else:
        q = math.sqrt(-2 * math.log(1 - p))
        return -((((c[5]*q + c[4])*q + c[3])*q + c[2])*q + c[1]) / \
                (((d[4]*q + d[3])*q + d[2])*q + d[1]*q + 1)


def merton_barrier(asset: float, vol: float, annual_pd: float) -> float:
    """Calibrate Merton barrier from target annual PD (terminal barrier)."""
    five_yr_pd = 1 - math.exp(-annual_pd * MATURITY)
    dd = -_norm_ppf(five_yr_pd)
    drift = (RISK_FREE - vol**2 / 2) * MATURITY
    barrier = asset * math.exp(-(dd * vol * math.sqrt(MATURITY) + drift))
    return barrier


def _dd(v: float, b: float, vol: float, t: float) -> float:
    """Distance-to-default."""
    if t <= 0 or v <= 0 or b <= 0:
        return 0.0
    return (math.log(v / b) + (RISK_FREE - vol**2/2) * t) / (vol * math.sqrt(t))
