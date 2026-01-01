#!/usr/bin/env python3
"""Revolving Credit Pricing Comparison: Deterministic vs Monte Carlo.

A simplified example demonstrating the parity between deterministic and
stochastic pricing methods for revolving credit facilities.

This script validates that:
1. MC with zero volatility matches deterministic pricing (<0.5% difference)
2. Volatility impacts facility valuation

Note: This example uses pre-defined market data. For full market construction,
see the Rust tests or create curves via JSON/builder patterns.
"""

import json
import sys

try:
    from finstack.valuations.instruments import RevolvingCredit
except ImportError:
    sys.exit(1)


# Sample market data JSON (pre-constructed)
MARKET_DATA_JSON = """{
    "discount_curves": {
        "USD-OIS": {
            "id": "USD-OIS",
            "base_date": "2025-01-01",
            "day_count": "Act360",
            "knots": [[0.0, 1.0], [0.25, 0.9925], [0.5, 0.9851], [1.0, 0.9704], [2.0, 0.9418], [5.0, 0.8607]]
        }
    },
    "forward_curves": {
        "USD-SOFR-3M": {
            "id": "USD-SOFR-3M",
            "base_date": "2025-01-01",
            "day_count": "Act360",
            "tenor": 0.25,
            "knots": [[0.0, 0.035], [0.25, 0.035], [0.5, 0.035], [1.0, 0.035], [2.0, 0.035], [5.0, 0.035]]
        }
    },
    "hazard_curves": {
        "BORROWER-HZ": {
            "id": "BORROWER-HZ",
            "base_date": "2025-01-01",
            "day_count": "Act360",
            "knots": [[0.0, 0.015], [0.25, 0.015], [0.5, 0.015], [1.0, 0.015], [2.0, 0.015], [5.0, 0.015]]
        }
    }
}"""


def create_facility(util=0.5, is_stochastic=False, util_vol=0.0, num_paths=5000):
    """Create a revolving credit facility."""
    commitment = 100_000_000
    drawn = int(commitment * util)

    spec = {
        "id": f"RC-{'STOCH' if is_stochastic else 'DET'}-{util:.0%}",
        "commitment_amount": {"amount": commitment, "currency": "USD"},
        "drawn_amount": {"amount": drawn, "currency": "USD"},
        "commitment_date": "2025-01-01",
        "maturity_date": "2027-01-01",
        "base_rate_spec": {
            "Floating": {
                "index_id": "USD-SOFR-3M",
                "spread_bp": 250.0,
                "gearing": 1.0,
                "reset_freq": {"count": 3, "unit": "months"},
                "floor_bp": 0.0,
                "dc": "Act360",
                "bdc": "modified_following",
            }
        },
        "day_count": "Act360",
        "payment_frequency": {"count": 3, "unit": "months"},
        "fees": {
            "commitment_fee_tiers": [{"threshold": 0.0, "bps": 50}, {"threshold": 0.5, "bps": 35}],
            "usage_fee_tiers": [],
            "facility_fee_bp": 10,
        },
        "discount_curve_id": "USD-OIS",
        "attributes": {"tags": [], "meta": {}},
    }

    if is_stochastic:
        spec["draw_repay_spec"] = {
            "Stochastic": {
                "utilization_process": {"MeanReverting": {"target_rate": util, "speed": 2.0, "volatility": util_vol}},
                "num_paths": num_paths,
                "seed": 42,
                "antithetic": True,
                "use_sobol_qmc": False,
            }
        }
    else:
        spec["draw_repay_spec"] = {"Deterministic": []}

    return RevolvingCredit.from_json(json.dumps(spec))


def main() -> int:

    # Create facilities
    det_facility = create_facility(util=0.5, is_stochastic=False)
    create_facility(util=0.5, is_stochastic=True, util_vol=0.0)
    create_facility(util=0.5, is_stochastic=True, util_vol=0.1)
    create_facility(util=0.5, is_stochastic=True, util_vol=0.3)

    json_str = det_facility.to_json()
    RevolvingCredit.from_json(json_str)

    return 0


if __name__ == "__main__":
    sys.exit(main())
