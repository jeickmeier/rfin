"""Colour palette, model parameters, and issuer definitions."""

from __future__ import annotations

from pptx.dml.color import RGBColor

# ── Colour palette (matches executive deck) ──────────────────────────────

DARK_BG = RGBColor(0x1A, 0x1A, 0x2E)
MID_BG = RGBColor(0x16, 0x21, 0x3E)
WHITE = RGBColor(0xFF, 0xFF, 0xFF)
LIGHT_GREY = RGBColor(0xCC, 0xCC, 0xCC)
ACCENT_BLUE = RGBColor(0x4E, 0xC9, 0xB0)
ACCENT_ORANGE = RGBColor(0xE8, 0x8D, 0x3F)
ACCENT_RED = RGBColor(0xE0, 0x4F, 0x4F)
ACCENT_GREEN = RGBColor(0x5C, 0xB8, 0x5C)
BODY_BG = RGBColor(0xF5, 0xF5, 0xFA)
TEXT_DARK = RGBColor(0x2D, 0x2D, 0x3D)
TEXT_MID = RGBColor(0x55, 0x55, 0x70)
HEADER_BLUE = RGBColor(0x2C, 0x3E, 0x6B)
TABLE_HEADER_BG = RGBColor(0x2C, 0x3E, 0x6B)
TABLE_ALT_BG = RGBColor(0xE8, 0xEB, 0xF5)
TABLE_WHITE_BG = RGBColor(0xFF, 0xFF, 0xFF)

TOTAL_SLIDES = 16

# ── Global model parameters ──────────────────────────────────────────────

RISK_FREE = 0.045
COUPON = 0.085
MATURITY = 5
NOTIONAL = 100.0
NUM_PATHS = 25_000

ISSUERS = [
    {"name": "BB+ (Solid HY)", "asset": 200, "vol": 0.20,
     "pd": 0.0020, "spread": 0.0085, "rec": 0.45, "ltv": "50%"},
    {"name": "BB\u2212 (Mid HY)", "asset": 165, "vol": 0.25,
     "pd": 0.0100, "spread": 0.0210, "rec": 0.40, "ltv": "61%"},
    {"name": "B (Weak HY)", "asset": 140, "vol": 0.30,
     "pd": 0.0250, "spread": 0.0390, "rec": 0.35, "ltv": "71%"},
    {"name": "B\u2212 (Stressed)", "asset": 125, "vol": 0.35,
     "pd": 0.0550, "spread": 0.0630, "rec": 0.30, "ltv": "80%"},
    {"name": "CCC (Deeply Stressed)", "asset": 115, "vol": 0.40,
     "pd": 0.1000, "spread": 0.1050, "rec": 0.25, "ltv": "87%"},
]
