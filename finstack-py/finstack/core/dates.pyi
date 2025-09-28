# flake8: noqa: PYI021
def add_months(date, months):
    """
    Add a number of calendar months to a date (preserving end-of-month logic).

    Parameters
    ----------
    date : datetime.date
        Anchor date.
    months : int
        Number of calendar months to add (negative to subtract).

    Returns
    -------
    datetime.date
        Resulting date after applying month arithmetic with EOM handling.

    Examples
    --------
    If `date` is January 31st and `months=1`, the result is February's last day.
    """

def adjust(date, convention, calendar):
    """
    Adjust a date according to a convention and calendar.

    Parameters
    ----------
    date : datetime.date
        Anchor date that may require adjustment.
    convention : BusinessDayConvention or str
        Convention controlling the adjustment.
    calendar : Calendar or str
        Calendar that defines business days.

    Returns
    -------
    datetime.date
        Adjusted business-day date.
    """

def available_calendar_codes():
    """
    Return the list of calendar codes understood by the registry.

    Returns
    -------
    list[str]
        Canonical calendar identifiers.
    """

def available_calendars():
    """
    Return all registered calendars as :class:`Calendar` instances.

    Returns
    -------
    list[Calendar]
        Collection of available calendars.
    """

def build_fiscal_periods(range, config, actuals_until=None):
    """
    Build fiscal periods using a `FiscalConfig` (e.g. ``FiscalConfig.US_FEDERAL``).

    Parameters
    ----------
    range : str
        Period range expression using fiscal period codes.
    config : FiscalConfig
        Fiscal-year configuration (country presets available as class attributes).
    actuals_until : str, optional
        Optional cutoff code marking earlier periods as actual.

    Returns
    -------
    PeriodPlan
        Ordered collection of fiscal `Period` records.
    """

def build_periods(range, actuals_until=None):
    """
    Build calendar periods from a range string.

    Parameters
    ----------
    range : str
        Period range expression, e.g. ``"2024Q1..Q4"`` or ``"2023M1..2023M12"``.
    actuals_until : str, optional
        Optional cutoff code; periods up to and including this code are marked actual.

    Returns
    -------
    PeriodPlan
        Ordered collection of `Period` records.

    Examples
    --------
    >>> plan = build_periods("2024Q1..Q4", actuals_until="2024Q2")
    >>> [p.id.code for p in plan.periods]
    ['2024Q1', '2024Q2', '2024Q3', '2024Q4']
    """

def date_to_days_since_epoch(date):
    """
    Convert a date into a day count offset from the Unix epoch (1970-01-01).

    Parameters
    ----------
    date : datetime.date
        Date to convert to an epoch day offset.

    Returns
    -------
    int
        Days since 1970-01-01 (negative for dates before the epoch).
    """

def days_in_month(year, month):
    """
    Number of days in a given month of a year.

    Parameters
    ----------
    year : int
        Calendar year.
    month : int
        Month number (1-12).

    Returns
    -------
    int
        Number of days in the month.

    Raises
    ------
    ValueError
        If `month` is not in 1..=12.
    """

def days_since_epoch_to_date(days):
    """
    Convert a day-count offset from the Unix epoch back to a date.

    Parameters
    ----------
    days : int
        Days since 1970-01-01 (negative allowed).

    Returns
    -------
    datetime.date
        Date corresponding to the epoch offset.
    """

def get_calendar(code):
    """
    Fetch a calendar by code (case-insensitive).

    Parameters
    ----------
    code : str
        Calendar identifier such as ``"usny"``.

    Returns
    -------
    Calendar
        Calendar instance resolved from the registry.
    """

def imm_option_expiry(year, month):
    """
    IMM option expiry date for a specific year and month.

    Parameters
    ----------
    year : int
        Calendar year.
    month : int
        Month (1-12).

    Returns
    -------
    datetime.date
        IMM option expiry date for the month.
    """

def is_leap_year(year):
    """
    True if the given year is a leap year.

    Parameters
    ----------
    year : int
        Calendar year.

    Returns
    -------
    bool
        `True` if `year` is a leap year, otherwise `False`.
    """

def last_day_of_month(date):
    """
    Last calendar day in the month of the provided date.

    Parameters
    ----------
    date : datetime.date
        Any date within the target month.

    Returns
    -------
    datetime.date
        Month-end date for `date`'s month.
    """

def next_cds_date(date):
    """
    Next CDS IMM date (quarterly on the 20th) after the provided date.

    Parameters
    ----------
    date : datetime.date
        Reference date.

    Returns
    -------
    datetime.date
        Next CDS IMM date.
    """

def next_equity_option_expiry(date):
    """
    Next monthly equity option expiry (third Friday) after the provided date.

    Parameters
    ----------
    date : datetime.date
        Reference date.

    Returns
    -------
    datetime.date
        Next monthly equity option expiry.
    """

def next_imm(date):
    """
    Next financial IMM date after the provided date.

    Parameters
    ----------
    date : datetime.date
        Reference date.

    Returns
    -------
    datetime.date
        The next IMM date.
    """

def next_imm_option_expiry(date):
    """
    Next financial IMM option expiry after the provided date.

    Parameters
    ----------
    date : datetime.date
        Reference date.

    Returns
    -------
    datetime.date
        Next IMM option expiry date.
    """

def third_friday(year, month):
    """
    Third Friday of the specified month/year.

    Parameters
    ----------
    year : int
        Calendar year.
    month : int
        Month (1-12).

    Returns
    -------
    datetime.date
        Third Friday date.
    """

def third_wednesday(year, month):
    """
    Third Wednesday of the specified month/year.

    Parameters
    ----------
    year : int
        Calendar year.
    month : int
        Month (1-12).

    Returns
    -------
    datetime.date
        Third Wednesday date.
    """
