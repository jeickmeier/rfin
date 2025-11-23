from datetime import date
from typing import List, Tuple, Optional

def npv(
    cash_flows: List[Tuple[date, float]],
    discount_rate: float,
    base_date: Optional[date] = None,
    day_count: Optional[str] = None,
) -> float: ...
def irr_periodic(amounts: List[float], guess: Optional[float] = None) -> float: ...
