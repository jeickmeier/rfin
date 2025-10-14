from __future__ import annotations

from datetime import date
from typing import Optional

from finstack.core.money import Money
from finstack.core.market_data import MarketContext

__all__ = [
    "InstrumentType",
    "ModelKey",
    "PricerKey",
    "PricerRegistry",
    "create_standard_registry",
    "MetricId",
    "MetricRegistry",
    "ValuationResult",
    "ResultsMeta",
    "CovenantReport",
    "PayReceive",
    "Bond",
    "Deposit",
    "InterestRateSwap",
    "FxSpot",
    "FxOption",
    "FxSwap",
    "Equity",
    "EquityOption",
    "ForwardRateAgreement",
    "InterestRateOption",
    "Swaption",
    "CDSPayReceive",
    "CreditDefaultSwap",
    "CDSIndex",
    "CdsOption",
    "CdsTranche",
    "RepoCollateral",
    "Repo",
    "InterestRateFuture",
    "InflationLinkedBond",
    "InflationSwap",
    "BasisSwapLeg",
    "BasisSwap",
    "Basket",
    "StructuredCredit",
]

class InstrumentType:
    BOND: "InstrumentType"
    LOAN: "InstrumentType"
    CDS: "InstrumentType"
    CDS_INDEX: "InstrumentType"
    CDS_TRANCHE: "InstrumentType"
    CDS_OPTION: "InstrumentType"
    IRS: "InstrumentType"
    CAP_FLOOR: "InstrumentType"
    SWAPTION: "InstrumentType"
    TRS: "InstrumentType"
    BASIS_SWAP: "InstrumentType"
    BASKET: "InstrumentType"
    CONVERTIBLE: "InstrumentType"
    DEPOSIT: "InstrumentType"
    EQUITY_OPTION: "InstrumentType"
    FX_OPTION: "InstrumentType"
    FX_SPOT: "InstrumentType"
    FX_SWAP: "InstrumentType"
    INFLATION_LINKED_BOND: "InstrumentType"
    INFLATION_SWAP: "InstrumentType"
    INTEREST_RATE_FUTURE: "InstrumentType"
    VARIANCE_SWAP: "InstrumentType"
    EQUITY: "InstrumentType"
    REPO: "InstrumentType"
    FRA: "InstrumentType"
    CLO: "InstrumentType"
    ABS: "InstrumentType"
    RMBS: "InstrumentType"
    CMBS: "InstrumentType"
    PRIVATE_MARKETS_FUND: "InstrumentType"
    STRUCTURED_CREDIT: "InstrumentType"
    @classmethod
    def from_name(cls, name: str) -> "InstrumentType": ...
    @property
    def name(self) -> str: ...

class ModelKey:
    DISCOUNTING: "ModelKey"
    TREE: "ModelKey"
    BLACK76: "ModelKey"
    HULL_WHITE_1F: "ModelKey"
    HAZARD_RATE: "ModelKey"
    @classmethod
    def from_name(cls, name: str) -> "ModelKey": ...
    @property
    def name(self) -> str: ...

class PricerKey:
    def __init__(self, instrument: InstrumentType | str, model: ModelKey | str) -> None: ...
    @property
    def instrument(self) -> InstrumentType: ...
    @property
    def model(self) -> ModelKey: ...

class PricerRegistry:
    def __init__(self) -> None: ...
    def price(self, instrument: object, model: ModelKey | str, market: MarketContext) -> ValuationResult: ...
    def price_with_metrics(
        self, instrument: object, model: ModelKey | str, market: MarketContext, metrics: list[MetricId | str]
    ) -> ValuationResult: ...
    def asw_forward(
        self,
        bond: object,
        market: MarketContext,
        forward_curve: str,
        float_margin_bp: float,
        dirty_price_ccy: float | None = ...,
    ) -> tuple[float, float]: ...
    def key(self, instrument: object, model: ModelKey | str) -> PricerKey: ...
    def clone(self) -> "PricerRegistry": ...

def create_standard_registry() -> PricerRegistry: ...

class MetricId:
    @classmethod
    def from_name(cls, name: str) -> "MetricId": ...
    @classmethod
    def standard_names(cls) -> list[str]: ...
    @property
    def name(self) -> str: ...

class MetricRegistry:
    def __init__(self) -> None: ...
    @classmethod
    def standard(cls) -> "MetricRegistry": ...
    def available_metrics(self) -> list[MetricId]: ...
    def metrics_for_instrument(self, instrument_type: InstrumentType | str) -> list[MetricId]: ...
    def is_applicable(self, metric: MetricId | str, instrument_type: InstrumentType | str) -> bool: ...
    def has_metric(self, metric: MetricId | str) -> bool: ...
    def clone(self) -> "MetricRegistry": ...

class ResultsMeta:
    @property
    def numeric_mode(self) -> str: ...
    @property
    def fx_policy_applied(self) -> Optional[str]: ...
    @property
    def rounding(self) -> dict[str, object]: ...
    def to_dict(self) -> dict[str, object]: ...

class CovenantReport:
    @property
    def covenant_type(self) -> str: ...
    @property
    def passed(self) -> bool: ...
    @property
    def actual_value(self) -> Optional[float]: ...
    @property
    def threshold(self) -> Optional[float]: ...
    @property
    def details(self) -> Optional[str]: ...

class ValuationResult:
    @property
    def instrument_id(self) -> str: ...
    @property
    def as_of(self) -> date: ...
    @property
    def value(self) -> Money: ...
    @property
    def measures(self) -> dict[str, float]: ...
    @property
    def meta(self) -> ResultsMeta: ...
    @property
    def covenants(self) -> Optional[dict[str, CovenantReport]]: ...
    def all_covenants_passed(self) -> bool: ...
    def failed_covenants(self) -> list[str]: ...
    def to_dict(self) -> dict[str, object]: ...

class PayReceive:
    PAY_FIXED: "PayReceive"
    RECEIVE_FIXED: "PayReceive"
    @classmethod
    def from_name(cls, name: str) -> "PayReceive": ...
    @property
    def name(self) -> str: ...

class Bond:
    @classmethod
    def fixed_semiannual(
        cls,
        instrument_id: str,
        notional: Money,
        coupon_rate: float,
        issue: date,
        maturity: date,
        discount_curve: str,
    ) -> "Bond": ...
    @classmethod
    def treasury(
        cls,
        instrument_id: str,
        notional: Money,
        coupon_rate: float,
        issue: date,
        maturity: date,
    ) -> "Bond": ...
    @classmethod
    def zero_coupon(
        cls,
        instrument_id: str,
        notional: Money,
        issue: date,
        maturity: date,
        discount_curve: str,
    ) -> "Bond": ...
    @classmethod
    def floating(
        cls,
        instrument_id: str,
        notional: Money,
        issue: date,
        maturity: date,
        discount_curve: str,
        forward_curve: str,
        margin_bp: float,
    ) -> "Bond": ...
    @classmethod
    def from_cashflows(
        cls,
        instrument_id: str,
        schedule: CashFlowSchedule,
        discount_curve: str,
        quoted_clean: Optional[float] = ...,
        forward_curve: Optional[str] = ...,
        float_margin_bp: Optional[float] = ...,
        float_gearing: Optional[float] = ...,
        float_reset_lag_days: Optional[int] = ...,
    ) -> "Bond": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def coupon(self) -> float: ...
    @property
    def issue(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def hazard_curve(self) -> Optional[str]: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class Deposit:
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        start: date,
        end: date,
        day_count: object,
        discount_curve: str,
        quote_rate: Optional[float] = ...,
    ) -> None: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def start(self) -> date: ...
    @property
    def end(self) -> date: ...
    @property
    def day_count(self) -> object: ...
    @property
    def quote_rate(self) -> Optional[float]: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class InterestRateSwap:
    @classmethod
    def usd_pay_fixed(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        start: date,
        end: date,
    ) -> "InterestRateSwap": ...
    @classmethod
    def usd_receive_fixed(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        start: date,
        end: date,
    ) -> "InterestRateSwap": ...
    @classmethod
    def usd_basis_swap(
        cls,
        instrument_id: str,
        notional: Money,
        start: date,
        end: date,
        primary_spread_bp: float,
        reference_spread_bp: float,
    ) -> "InterestRateSwap": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def side(self) -> PayReceive: ...
    @property
    def fixed_rate(self) -> float: ...
    @property
    def float_spread_bp(self) -> float: ...
    @property
    def start(self) -> date: ...
    @property
    def end(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class FxSpot:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        base_currency: Currency | str,
        quote_currency: Currency | str,
        *,
        settlement: Optional[date] = ...,
        settlement_lag_days: Optional[int] = ...,
        spot_rate: Optional[float] = ...,
        notional: Optional[Money] = ...,
        bdc: Optional[str] = ...,
        calendar: Optional[str] = ...,
    ) -> "FxSpot": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def notional(self) -> Optional[Money]: ...
    @property
    def spot_rate(self) -> Optional[float]: ...
    @property
    def settlement(self) -> Optional[date]: ...
    @property
    def settlement_lag_days(self) -> Optional[int]: ...
    @property
    def business_day_convention(self) -> str: ...
    @property
    def calendar_id(self) -> Optional[str]: ...
    @property
    def pair_name(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class FxOption:
    @classmethod
    def european_call(
        cls,
        instrument_id: str,
        base_currency: Currency | str,
        quote_currency: Currency | str,
        strike: float,
        expiry: date,
        notional: Money,
    ) -> "FxOption": ...
    @classmethod
    def european_put(
        cls,
        instrument_id: str,
        base_currency: Currency | str,
        quote_currency: Currency | str,
        strike: float,
        expiry: date,
        notional: Money,
    ) -> "FxOption": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def option_type(self) -> str: ...
    @property
    def exercise_style(self) -> str: ...
    @property
    def settlement(self) -> str: ...
    @property
    def domestic_curve(self) -> str: ...
    @property
    def foreign_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class FxSwap:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        base_currency: Currency | str,
        quote_currency: Currency | str,
        notional: Money,
        near_date: date,
        far_date: date,
        domestic_curve: str,
        foreign_curve: str,
        *,
        near_rate: Optional[float] = ...,
        far_rate: Optional[float] = ...,
    ) -> "FxSwap": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def base_notional(self) -> Money: ...
    @property
    def near_date(self) -> date: ...
    @property
    def far_date(self) -> date: ...
    @property
    def near_rate(self) -> Optional[float]: ...
    @property
    def far_rate(self) -> Optional[float]: ...
    @property
    def domestic_curve(self) -> str: ...
    @property
    def foreign_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class Equity:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        ticker: str,
        currency: Currency | str,
        *,
        shares: Optional[float] = ...,
        price: Optional[float] = ...,
    ) -> "Equity": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def shares(self) -> float: ...
    @property
    def price_quote(self) -> Optional[float]: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class EquityOption:
    @classmethod
    def european_call(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        expiry: date,
        notional: Money,
        contract_size: float = ...,
    ) -> "EquityOption": ...
    @classmethod
    def european_put(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        expiry: date,
        notional: Money,
        contract_size: float = ...,
    ) -> "EquityOption": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def strike(self) -> Money: ...
    @property
    def contract_size(self) -> float: ...
    @property
    def option_type(self) -> str: ...
    @property
    def exercise_style(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class ForwardRateAgreement:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        fixing_date: date,
        start_date: date,
        end_date: date,
        discount_curve: str,
        forward_curve: str,
        *,
        day_count: object | str = ...,
        reset_lag: int = ...,
        pay_fixed: bool = ...,
    ) -> "ForwardRateAgreement": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def fixed_rate(self) -> float: ...
    @property
    def day_count(self) -> object: ...
    @property
    def reset_lag(self) -> int: ...
    @property
    def pay_fixed(self) -> bool: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def fixing_date(self) -> date: ...
    @property
    def start_date(self) -> date: ...
    @property
    def end_date(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class InterestRateOption:
    @classmethod
    def cap(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        start_date: date,
        end_date: date,
        discount_curve: str,
        forward_curve: str,
        *,
        vol_surface: str | None = ...,
        payments_per_year: int = ...,
        day_count: object | str = ...,
    ) -> "InterestRateOption": ...
    @classmethod
    def floor(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        start_date: date,
        end_date: date,
        discount_curve: str,
        forward_curve: str,
        *,
        vol_surface: str | None = ...,
        payments_per_year: int = ...,
        day_count: object | str = ...,
    ) -> "InterestRateOption": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def start_date(self) -> date: ...
    @property
    def end_date(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class Swaption:
    @classmethod
    def payer(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        expiry: date,
        swap_start: date,
        swap_end: date,
        discount_curve: str,
        forward_curve: str,
        *,
        vol_surface: str | None = ...,
        exercise: str = ...,
        settlement: str = ...,
    ) -> "Swaption": ...
    @classmethod
    def receiver(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        expiry: date,
        swap_start: date,
        swap_end: date,
        discount_curve: str,
        forward_curve: str,
        *,
        vol_surface: str | None = ...,
        exercise: str = ...,
        settlement: str = ...,
    ) -> "Swaption": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def swap_start(self) -> date: ...
    @property
    def swap_end(self) -> date: ...
    @property
    def option_type(self) -> str: ...
    @property
    def settlement(self) -> str: ...
    @property
    def exercise(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class CDSPayReceive:
    PAY_PROTECTION: "CDSPayReceive"
    RECEIVE_PROTECTION: "CDSPayReceive"
    @classmethod
    def from_name(cls, name: str) -> "CDSPayReceive": ...
    @property
    def name(self) -> str: ...

class CreditDefaultSwap:
    @classmethod
    def buy_protection(
        cls,
        instrument_id: str,
        notional: Money,
        spread_bp: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        credit_curve: str,
        *,
        recovery_rate: float | None = ...,
        settlement_delay: int | None = ...,
    ) -> "CreditDefaultSwap": ...
    @classmethod
    def sell_protection(
        cls,
        instrument_id: str,
        notional: Money,
        spread_bp: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        credit_curve: str,
        *,
        recovery_rate: float | None = ...,
        settlement_delay: int | None = ...,
    ) -> "CreditDefaultSwap": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def side(self) -> CDSPayReceive: ...
    @property
    def notional(self) -> Money: ...
    @property
    def spread_bp(self) -> float: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_curve(self) -> str: ...
    @property
    def recovery_rate(self) -> float: ...
    @property
    def settlement_delay(self) -> int: ...
    @property
    def start_date(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class CDSIndex:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        index_name: str,
        series: int,
        version: int,
        notional: Money,
        fixed_coupon_bp: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        credit_curve: str,
        *,
        side: str = ...,
        recovery_rate: float = ...,
        index_factor: float | None = ...,
    ) -> "CDSIndex": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def index_name(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def fixed_coupon_bp(self) -> float: ...
    @property
    def side(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_curve(self) -> str: ...
    @property
    def maturity(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class CdsOption:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        strike_spread_bp: float,
        expiry: date,
        cds_maturity: date,
        discount_curve: str,
        credit_curve: str,
        vol_surface: str,
        *,
        option_type: str = ...,
        recovery_rate: float = ...,
        underlying_is_index: bool = ...,
        index_factor: float | None = ...,
        forward_adjust_bp: float = ...,
    ) -> "CdsOption": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike_spread_bp(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def cds_maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class CdsTranche:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        index_name: str,
        series: int,
        attach_pct: float,
        detach_pct: float,
        notional: Money,
        maturity: date,
        running_coupon_bp: float,
        discount_curve: str,
        credit_index_curve: str,
        *,
        side: str = ...,
        payments_per_year: int = ...,
        day_count: object | str = ...,
        business_day_convention: object | str = ...,
        calendar: str | None = ...,
        effective_date: date | None = ...,
    ) -> "CdsTranche": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def attach_pct(self) -> float: ...
    @property
    def detach_pct(self) -> float: ...
    @property
    def running_coupon_bp(self) -> float: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_index_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class RepoCollateral:
    def __init__(
        self,
        instrument_id: str,
        quantity: float,
        market_value_id: str,
        *,
        collateral_type: str = ...,
        special_security_id: str | None = ...,
        special_rate_adjust_bp: float | None = ...,
    ) -> None: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def quantity(self) -> float: ...
    @property
    def market_value_id(self) -> str: ...

class Repo:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        cash_amount: Money,
        collateral: RepoCollateral,
        repo_rate: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        *,
        repo_type: str = ...,
        haircut: float = ...,
        day_count: object | str = ...,
        business_day_convention: object | str = ...,
        calendar: str | None = ...,
        triparty: bool = ...,
    ) -> "Repo": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def cash_amount(self) -> Money: ...
    @property
    def repo_rate(self) -> float: ...
    @property
    def start_date(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class BasisSwapLeg:
    def __init__(
        self,
        forward_curve: str,
        *,
        frequency: str = ...,
        day_count: object | str = ...,
        business_day_convention: object | str = ...,
        spread: float = ...,
    ) -> None: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def spread(self) -> float: ...

class BasisSwap:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        start_date: date,
        maturity: date,
        primary_leg: BasisSwapLeg,
        reference_leg: BasisSwapLeg,
        discount_curve: str,
        *,
        calendar: str | None = ...,
        stub: str = ...,
    ) -> "BasisSwap": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class InterestRateFuture:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        quoted_price: float,
        expiry: date,
        fixing_date: date,
        period_start: date,
        period_end: date,
        discount_curve: str,
        forward_curve: str,
        *,
        position: str = ...,
        day_count: object | str = ...,
        face_value: float = ...,
        tick_size: float = ...,
        tick_value: float | None = ...,
        delivery_months: int = ...,
        convexity_adjustment: float | None = ...,
    ) -> "InterestRateFuture": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def quoted_price(self) -> float: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class InflationLinkedBond:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        real_coupon: float,
        issue: date,
        maturity: date,
        base_index: float,
        discount_curve: str,
        inflation_curve: str,
        *,
        indexation: str = ...,
        frequency: str = ...,
        day_count: object | str = ...,
        deflation_protection: str = ...,
        calendar: str | None = ...,
    ) -> "InflationLinkedBond": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def real_coupon(self) -> float: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def inflation_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class InflationSwap:
    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        inflation_index: str | None = ...,
        *,
        side: str = ...,
        day_count: object | str = ...,
        inflation_id: str | None = ...,
        lag_override: str | None = ...,
        inflation_curve: str | None = ...,
    ) -> "InflationSwap": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def fixed_rate(self) -> float: ...
    @property
    def maturity(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class Basket:
    @classmethod
    def from_json(cls, data: str | dict[str, object]) -> "Basket": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def to_json(self) -> str: ...

class StructuredCredit:
    @classmethod
    def from_json(cls, data: str | dict[str, object]) -> "StructuredCredit": ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def deal_type(self) -> str: ...
    @property
    def tranche_count(self) -> int: ...
    def to_json(self) -> str: ...
