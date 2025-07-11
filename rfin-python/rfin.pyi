# flake8: noqa: PYI021
def generate_schedule(start, end, frequency, convention=None, calendar=None, stub=None):
    """
    Generate an inclusive date schedule between `start` and `end`.

    Args:
        start (Date): start date (inclusive)
        end (Date): end date (inclusive)
        frequency (Frequency): coupon frequency
        convention (Optional[BusDayConvention]): business-day convention (default None → unadjusted)
        calendar (Optional[Calendar]): holiday calendar used for adjustment
        stub (Optional[StubRule]): stub rule controlling how irregular periods are handled

    Returns:
        List[Date]: generated schedule (Python list of Date objects)
    """

def next_cds_date(date):
    """Return the next CDS roll date strictly after `date`."""

def next_imm(date):
    """Return the next IMM date strictly after `date`."""

def third_wednesday(month, year):
    """Return the third Wednesday of the specified `month` and `year`."""
