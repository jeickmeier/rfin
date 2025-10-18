"""Valuation result envelopes, metadata, and covenant report bindings."""

from typing import Dict, Optional, Any
from datetime import date
from ..core.money import Money

class CovenantReport:
    """Covenant evaluation outcome attached to a valuation result.
    
    Examples:
        >>> report = valuation_result.covenants['ltv']
        >>> report.passed
        True
    """
    
    @property
    def covenant_type(self) -> str:
        """Covenant identifier describing the check performed.
        
        Returns:
            str: Covenant label supplied by the originating configuration.
        """
        ...
    
    @property
    def passed(self) -> bool:
        """Whether the covenant passed for the evaluated scenario.
        
        Returns:
            bool: True when the covenant conditions are satisfied.
        """
        ...
    
    @property
    def actual_value(self) -> Optional[float]:
        """Observed metric value when available.
        
        Returns:
            float | None: Realized metric value used in the check.
        """
        ...
    
    @property
    def threshold(self) -> Optional[float]:
        """Required threshold for the covenant, when provided.
        
        Returns:
            float | None: Target threshold or limit for the covenant.
        """
        ...
    
    @property
    def details(self) -> Optional[str]:
        """Additional free-form details attached to the report.
        
        Returns:
            str | None: Supplemental information captured during evaluation.
        """
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class ResultsMeta:
    """Snapshot describing numeric mode, rounding context, and FX policy applied to results.
    
    Examples:
        >>> meta.numeric_mode
        'f64'
    """
    
    @property
    def numeric_mode(self) -> str:
        """Numeric engine mode used by the pricing engine (e.g., "f64").
        
        Returns:
            str: Symbol representing the numeric precision.
        """
        ...
    
    @property
    def fx_policy_applied(self) -> Optional[str]:
        """Optional FX policy key applied during result aggregation.
        
        Returns:
            str | None: FX policy identifier or None when not applied.
        """
        ...
    
    @property
    def rounding(self) -> Dict[str, Any]:
        """Rounding context snapshot as a dictionary.
        
        Returns:
            dict: Dictionary containing rounding mode and per-currency scales.
        """
        ...
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert the metadata to a Python dictionary for downstream serialization.
        
        Returns:
            dict: Serializable snapshot of metadata fields.
            
        Examples:
            >>> meta.to_dict()['numeric_mode']
            'f64'
        """
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class ValuationResult:
    """Complete valuation output including PV, measures, metadata, and covenant reports.
    
    Examples:
        >>> result.value.amount
        123.45
    """
    
    @property
    def instrument_id(self) -> str:
        """Instrument identifier used when stamping the result.
        
        Returns:
            str: Unique instrument identifier supplied at pricing time.
        """
        ...
    
    @property
    def as_of(self) -> date:
        """Valuation date associated with the pricing run.
        
        Returns:
            datetime.date: Effective market date for the valuation.
        """
        ...
    
    @property
    def value(self) -> Money:
        """Present value expressed as Money.
        
        Returns:
            Money: Present value of the instrument.
        """
        ...
    
    @property
    def measures(self) -> Dict[str, float]:
        """Dictionary of computed measures (e.g., {"dv01": 1250.0}).
        
        Returns:
            dict[str, float]: Calculated risk measures keyed by metric id.
        """
        ...
    
    @property
    def meta(self) -> ResultsMeta:
        """Metadata describing numeric mode, rounding context, and FX policy.
        
        Returns:
            ResultsMeta: Snapshot of metadata associated with the valuation.
        """
        ...
    
    @property
    def covenants(self) -> Optional[Dict[str, CovenantReport]]:
        """Covenant reports (if any) keyed by covenant identifier.
        
        Returns:
            dict[str, CovenantReport] | None: Covenant evaluations when available.
        """
        ...
    
    def all_covenants_passed(self) -> bool:
        """Convenience helper returning True when all covenants passed.
        
        Returns:
            bool: True when there are no failing covenant reports.
            
        Examples:
            >>> result.all_covenants_passed()
            True
        """
        ...
    
    def failed_covenants(self) -> List[str]:
        """List of covenant identifiers that failed (empty when all pass).
        
        Returns:
            list[str]: Identifiers for covenants that evaluated to false.
        """
        ...
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to a Python dictionary for JSON/Arrow serialization.
        
        Returns:
            dict: Serializable dictionary containing the valuation payload.
            
        Examples:
            >>> data = result.to_dict()
            >>> sorted(data.keys())
            ['as_of', 'covenants', 'instrument_id', 'measures', 'meta', 'value']
        """
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
