"""Metric identifiers and registry helpers for finstack valuations."""

from typing import List, Any, Union
from .common import InstrumentType

class MetricId:
    """Strongly typed metric identifier mirroring finstack_valuations::metrics::MetricId.
    
    Examples:
        >>> MetricId.from_name("pv")
        MetricId('pv')
    """
    
    @classmethod
    def from_name(cls, name: str) -> MetricId:
        """Parse a metric identifier, falling back to a custom metric when unknown.
        
        Args:
            name: Metric label such as "pv" or "dv01".
            
        Returns:
            MetricId: Identifier corresponding to name.
            
        Examples:
            >>> MetricId.from_name("dv01").name
            'dv01'
        """
        ...
    
    @property
    def name(self) -> str:
        """Snake-case name of the metric.
        
        Returns:
            str: Metric label, e.g., "pv".
        """
        ...
    
    @classmethod
    def standard_names(cls) -> List[str]:
        """List of all standard metric identifiers bundled with finstack.
        
        Returns:
            list[str]: Collection of built-in metric labels.
            
        Examples:
            >>> "pv" in MetricId.standard_names()
            True
        """
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __richcmp__(self, other: object, op: int) -> object: ...

class MetricRegistry:
    """Registry of metric calculators with applicability filtering.
    
    Examples:
        >>> registry = MetricRegistry.standard()
        >>> registry.has_metric("pv")
        True
    """
    
    def __init__(self) -> None:
        """Create an empty registry instance.
        
        Returns:
            MetricRegistry: Registry without pre-registered metrics.
            
        Examples:
            >>> custom = MetricRegistry()
            >>> custom.available_metrics()
            []
        """
        ...
    
    @classmethod
    def standard(cls) -> MetricRegistry:
        """Create a registry populated with all finstack standard metrics.
        
        Returns:
            MetricRegistry: Registry containing the default metric set.
            
        Examples:
            >>> MetricRegistry.standard().has_metric("pv")
            True
        """
        ...
    
    def available_metrics(self) -> List[MetricId]:
        """All metric identifiers currently registered.
        
        Returns:
            list[MetricId]: Wrapped metric identifiers known to the registry.
        """
        ...
    
    def metrics_for_instrument(self, instrument_type: Union[InstrumentType, str]) -> List[MetricId]:
        """Metrics applicable to the supplied instrument type.
        
        Args:
            instrument_type: Instrument type enumeration or label.
            
        Returns:
            list[MetricId]: Metrics that can be computed for the instrument.
            
        Raises:
            ValueError: If the instrument label cannot be parsed.
        """
        ...
    
    def is_applicable(
        self, 
        metric: Union[MetricId, str], 
        instrument_type: Union[InstrumentType, str]
    ) -> bool:
        """Test whether metric applies to the provided instrument type.
        
        Args:
            metric: Metric identifier or label.
            instrument_type: Instrument type enumeration or label.
            
        Returns:
            bool: True when the metric supports the instrument type.
            
        Raises:
            ValueError: If either argument cannot be parsed.
        """
        ...
    
    def has_metric(self, metric: Union[MetricId, str]) -> bool:
        """Determine whether the registry contains metric.
        
        Args:
            metric: Metric identifier or snake-case label.
            
        Returns:
            bool: True when the metric is registered.
            
        Raises:
            ValueError: If the metric cannot be parsed.
        """
        ...
    
    def clone(self) -> MetricRegistry:
        """Clone the registry for experimentation without mutating the original.
        
        Returns:
            MetricRegistry: Shallow clone of the current registry.
            
        Examples:
            >>> cloned = MetricRegistry.standard().clone()
            >>> cloned.has_metric("pv")
            True
        """
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
