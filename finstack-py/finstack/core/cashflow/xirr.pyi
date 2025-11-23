from datetime import date
from typing import List, Tuple, Optional

def xirr(cash_flows: List[Tuple[date, float]], guess: Optional[float] = None) -> float: ...

