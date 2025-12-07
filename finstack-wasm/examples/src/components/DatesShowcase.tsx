import React, { useEffect, useState } from 'react';
import {
  FsDate,
  BusinessDayConvention,
  DayCount,
  DayCountContext,
  Frequency,
  ScheduleBuilder,
  StubKind,
  buildPeriods,
  buildFiscalPeriods,
  FiscalConfig,
  addMonths,
  lastDayOfMonth,
  daysInMonth,
  isLeapYear,
  dateToDaysSinceEpoch,
  daysSinceEpochToDate,
  adjust,
  availableCalendarCodes,
  getCalendar,
  nextImm,
  nextCdsDate,
  nextEquityOptionExpiry,
  thirdFriday,
  thirdWednesday,
} from 'finstack-wasm';
import {
  DateConstructionProps,
  DateUtilitiesProps,
  CalendarExampleProps,
  DayCountExampleProps,
  ScheduleBuilderExampleProps,
  PeriodPlansExampleProps,
  IMMDatesExampleProps,
  DEFAULT_DATE_CONSTRUCTION,
  DEFAULT_DATE_UTILITIES,
  DEFAULT_CALENDAR_EXAMPLE,
  DEFAULT_DAY_COUNT_EXAMPLE,
  DEFAULT_SCHEDULE_BUILDER,
  DEFAULT_PERIOD_PLANS,
  DEFAULT_IMM_DATES,
} from './data/dates-showcase';

type RequiredDateConstructionProps = Required<DateConstructionProps>;
type RequiredDateUtilitiesProps = Required<DateUtilitiesProps>;
type RequiredCalendarExampleProps = Required<CalendarExampleProps>;
type RequiredDayCountExampleProps = Required<DayCountExampleProps>;
type RequiredScheduleBuilderExampleProps = Required<ScheduleBuilderExampleProps>;
type RequiredPeriodPlansExampleProps = Required<PeriodPlansExampleProps>;
type RequiredIMMDatesExampleProps = Required<IMMDatesExampleProps>;

const toIso = (date: FsDate) => {
  const month = String(date.month).padStart(2, '0');
  const day = String(date.day).padStart(2, '0');
  return `${date.year}-${month}-${day}`;
};

// 1. Date Construction & Manipulation Example
export const DateConstructionExample: React.FC<DateConstructionProps> = (props) => {
  const defaults = DEFAULT_DATE_CONSTRUCTION as RequiredDateConstructionProps;
  const { exampleDate = defaults.exampleDate, weekdaysToAdd = defaults.weekdaysToAdd } = props;

  const [data, setData] = useState<{ [key: string]: string | number | boolean }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const date = new FsDate(exampleDate.year, exampleDate.month, exampleDate.day);
        const results: { [key: string]: string | number | boolean } = {
          Date: toIso(date),
          Year: date.year,
          Month: date.month,
          Day: date.day,
          Quarter: date.quarter(),
          'Is Weekend': date.isWeekend(),
          'Fiscal Year': date.fiscalYear(),
          [`Add ${weekdaysToAdd} weekdays`]: toIso(date.addWeekdays(weekdaysToAdd)),
        };

        if (!cancelled) {
          setData(results);
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [exampleDate, weekdaysToAdd]);

  if (error) return <p className="error">{error}</p>;
  if (Object.keys(data).length === 0) return <p>Loading...</p>;

  return (
    <section className="example-section">
      <h2>Date Construction & Properties</h2>
      <dl className="data-list">
        {Object.entries(data).map(([key, value]) => (
          <React.Fragment key={key}>
            <dt>{key}</dt>
            <dd>{String(value)}</dd>
          </React.Fragment>
        ))}
      </dl>
    </section>
  );
};

// 2. Date Utilities Example
export const DateUtilitiesExample: React.FC<DateUtilitiesProps> = (props) => {
  const defaults = DEFAULT_DATE_UTILITIES as RequiredDateUtilitiesProps;
  const { baseDate = defaults.baseDate, monthsToAdd = defaults.monthsToAdd } = props;

  const [data, setData] = useState<{ [key: string]: string | number | boolean }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const base = new FsDate(baseDate.year, baseDate.month, baseDate.day);
        const epochDays = dateToDaysSinceEpoch(base);

        const results: { [key: string]: string | number | boolean } = {
          'Base Date': toIso(base),
        };

        // Add months dynamically
        for (const months of monthsToAdd) {
          results[`Add ${months} month${months > 1 ? 's' : ''}`] = toIso(addMonths(base, months));
        }

        Object.assign(results, {
          'Last day of month': toIso(lastDayOfMonth(base)),
          [`Days in ${base.month === 1 ? 'Jan' : 'Feb'} ${base.year}`]: daysInMonth(
            base.year,
            base.month
          ),
          'Days in Feb 2024': daysInMonth(2024, 2),
          'Days in Feb 2025': daysInMonth(2025, 2),
          'Is 2024 leap year': isLeapYear(2024),
          'Is 2025 leap year': isLeapYear(2025),
          'Days since epoch': epochDays,
          'Epoch round-trip': toIso(daysSinceEpochToDate(epochDays)),
        });

        if (!cancelled) {
          setData(results);
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [baseDate, monthsToAdd]);

  if (error) return <p className="error">{error}</p>;
  if (Object.keys(data).length === 0) return <p>Loading...</p>;

  return (
    <section className="example-section">
      <h2>Date Utilities</h2>
      <dl className="data-list">
        {Object.entries(data).map(([key, value]) => (
          <React.Fragment key={key}>
            <dt>{key}</dt>
            <dd>{String(value)}</dd>
          </React.Fragment>
        ))}
      </dl>
    </section>
  );
};

// 3. Calendar & Business Day Adjustments
export const CalendarExample: React.FC<CalendarExampleProps> = (props) => {
  const defaults = DEFAULT_CALENDAR_EXAMPLE as RequiredCalendarExampleProps;
  const {
    saturdayDate = defaults.saturdayDate,
    holidayDate = defaults.holidayDate,
    calendarCodes = defaults.calendarCodes,
  } = props;

  const [data, setData] = useState<{ [key: string]: string | boolean }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const codes = availableCalendarCodes();
        const calendars = calendarCodes.map((code) => getCalendar(code));
        const [primaryCal, secondaryCal] = calendars;

        const saturday = new FsDate(saturdayDate.year, saturdayDate.month, saturdayDate.day);
        const holiday = new FsDate(holidayDate.year, holidayDate.month, holidayDate.day);

        const results: { [key: string]: string | boolean } = {
          'Available Calendars': codes.length.toString(),
          'Sample Calendars': codes.slice(0, 5).join(', '),
        };

        if (primaryCal) {
          results[`${calendarCodes[0].toUpperCase()} Name`] = primaryCal.name;
          results[`${calendarCodes[0].toUpperCase()} Ignores Weekends`] = primaryCal.ignoreWeekends;
        }
        if (secondaryCal) {
          results[`${calendarCodes[1].toUpperCase()} Name`] = secondaryCal.name;
        }

        results['Saturday is weekend'] = saturday.isWeekend();
        if (primaryCal) {
          results[`Saturday is ${calendarCodes[0].toUpperCase()} business day`] =
            primaryCal.isBusinessDay(saturday);
          results[`Holiday is ${calendarCodes[0].toUpperCase()} holiday`] =
            primaryCal.isHoliday(holiday);
          results['Adjusted (Following)'] = toIso(
            adjust(saturday, BusinessDayConvention.Following, primaryCal)
          );
          results['Adjusted (Preceding)'] = toIso(
            adjust(saturday, BusinessDayConvention.Preceding, primaryCal)
          );
          results['Adjusted (Modified Following)'] = toIso(
            adjust(saturday, BusinessDayConvention.ModifiedFollowing, primaryCal)
          );
        }

        if (!cancelled) {
          setData(results);
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [saturdayDate, holidayDate, calendarCodes]);

  if (error) return <p className="error">{error}</p>;
  if (Object.keys(data).length === 0) return <p>Loading...</p>;

  return (
    <section className="example-section">
      <h2>Calendars & Business Day Adjustments</h2>
      <dl className="data-list">
        {Object.entries(data).map(([key, value]) => (
          <React.Fragment key={key}>
            <dt>{key}</dt>
            <dd>{String(value)}</dd>
          </React.Fragment>
        ))}
      </dl>
    </section>
  );
};

// 4. Day Count Conventions
export const DayCountExample: React.FC<DayCountExampleProps> = (props) => {
  const defaults = DEFAULT_DAY_COUNT_EXAMPLE as RequiredDayCountExampleProps;
  const {
    startDate = defaults.startDate,
    endDate = defaults.endDate,
    calendarCode = defaults.calendarCode,
  } = props;

  const [data, setData] = useState<{ [key: string]: string }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const start = new FsDate(startDate.year, startDate.month, startDate.day);
        const end = new FsDate(endDate.year, endDate.month, endDate.day);
        const calendar = getCalendar(calendarCode);

        // Different day count conventions
        const act360 = DayCount.act360();
        const act365f = DayCount.act365f();
        const thirty360 = DayCount.thirty360();
        const actAct = DayCount.actAct();
        const actActIsma = DayCount.actActIsma();
        const bus252 = DayCount.bus252();

        // Context with calendar and frequency for ISMA
        const ctxIsma = new DayCountContext();
        ctxIsma.setCalendar(calendar);
        ctxIsma.setFrequency(Frequency.semiAnnual());

        // Context for BUS/252
        const ctxBus = new DayCountContext();
        ctxBus.setCalendar(calendar);

        const results: { [key: string]: string } = {
          'Start Date': toIso(start),
          'End Date': toIso(end),
          'Act/360': act360.yearFraction(start, end, null).toFixed(6),
          'Act/365F': act365f.yearFraction(start, end, null).toFixed(6),
          '30/360': thirty360.yearFraction(start, end, null).toFixed(6),
          'Act/Act (ISDA)': actAct.yearFraction(start, end, null).toFixed(6),
          'Act/Act (ISMA)': actActIsma.yearFraction(start, end, ctxIsma).toFixed(6),
          'BUS/252': bus252.yearFraction(start, end, ctxBus).toFixed(6),
        };

        if (!cancelled) {
          setData(results);
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [startDate, endDate, calendarCode]);

  if (error) return <p className="error">{error}</p>;
  if (Object.keys(data).length === 0) return <p>Loading...</p>;

  return (
    <section className="example-section">
      <h2>Day Count Conventions</h2>
      <dl className="data-list">
        {Object.entries(data).map(([key, value]) => (
          <React.Fragment key={key}>
            <dt>{key}</dt>
            <dd>{value}</dd>
          </React.Fragment>
        ))}
      </dl>
    </section>
  );
};

// 5. Schedule Builder Examples
export const ScheduleBuilderExample: React.FC<ScheduleBuilderExampleProps> = (props) => {
  const defaults = DEFAULT_SCHEDULE_BUILDER as RequiredScheduleBuilderExampleProps;
  const {
    startDate = defaults.startDate,
    endDate = defaults.endDate,
    cdsStartDate = defaults.cdsStartDate,
    cdsEndDate = defaults.cdsEndDate,
    calendarCode = defaults.calendarCode,
  } = props;

  const [schedules, setSchedules] = useState<{ [key: string]: string[] }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const start = new FsDate(startDate.year, startDate.month, startDate.day);
        const end = new FsDate(endDate.year, endDate.month, endDate.day);
        const calendar = getCalendar(calendarCode);

        // Monthly schedule with modified following and EOM
        const monthly = new ScheduleBuilder(start, end)
          .frequency(Frequency.monthly())
          .stubRule(StubKind.none())
          .adjustWith(BusinessDayConvention.ModifiedFollowing, calendar)
          .endOfMonth(false)
          .build();

        // Quarterly schedule
        const quarterly = new ScheduleBuilder(start, end)
          .frequency(Frequency.quarterly())
          .stubRule(StubKind.shortBack())
          .adjustWith(BusinessDayConvention.Following, calendar)
          .build();

        // Semi-annual schedule
        const semiAnnual = new ScheduleBuilder(start, end)
          .frequency(Frequency.semiAnnual())
          .stubRule(StubKind.none())
          .adjustWith(BusinessDayConvention.ModifiedFollowing, calendar)
          .build();

        // CDS IMM schedule
        const cdsStart = new FsDate(cdsStartDate.year, cdsStartDate.month, cdsStartDate.day);
        const cdsEnd = new FsDate(cdsEndDate.year, cdsEndDate.month, cdsEndDate.day);
        const cdsSchedule = new ScheduleBuilder(cdsStart, cdsEnd)
          .frequency(Frequency.quarterly())
          .cdsImm()
          .adjustWith(BusinessDayConvention.Following, calendar)
          .build();

        const results = {
          'Monthly (Modified Following)': monthly.toArray().map((d) => toIso(d as FsDate)),
          'Quarterly (Short Back)': quarterly.toArray().map((d) => toIso(d as FsDate)),
          'Semi-Annual': semiAnnual.toArray().map((d) => toIso(d as FsDate)),
          'CDS IMM (5Y)': cdsSchedule.toArray().map((d) => toIso(d as FsDate)),
        };

        if (!cancelled) {
          setSchedules(results);
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [startDate, endDate, cdsStartDate, cdsEndDate, calendarCode]);

  if (error) return <p className="error">{error}</p>;
  if (Object.keys(schedules).length === 0) return <p>Loading...</p>;

  return (
    <section className="example-section">
      <h2>Schedule Builder</h2>
      {Object.entries(schedules).map(([title, dates]) => (
        <div key={title} style={{ marginBottom: '2rem' }}>
          <h3 style={{ fontSize: '1.2rem', marginBottom: '0.5rem' }}>{title}</h3>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem' }}>
            {dates.map((date, idx) => (
              <span
                key={idx}
                style={{
                  padding: '0.25rem 0.5rem',
                  backgroundColor: 'rgba(100, 108, 255, 0.1)',
                  borderRadius: '4px',
                  fontSize: '0.9rem',
                }}
              >
                {date}
              </span>
            ))}
          </div>
        </div>
      ))}
    </section>
  );
};

// 6. Period Plans (Calendar & Fiscal)
export const PeriodPlansExample: React.FC<PeriodPlansExampleProps> = (props) => {
  const defaults = DEFAULT_PERIOD_PLANS as RequiredPeriodPlansExampleProps;
  const {
    calendarPeriodRange = defaults.calendarPeriodRange,
    calendarActualsThrough = defaults.calendarActualsThrough,
    fiscalPeriodRange = defaults.fiscalPeriodRange,
  } = props;

  const [data, setData] = useState<{
    calendar: Array<{ id: string; start: string; end: string; actual: boolean }>;
    fiscal: Array<{ id: string; start: string; end: string }>;
  }>({ calendar: [], fiscal: [] });
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        // Calendar periods with actuals
        const calendarPlan = buildPeriods(calendarPeriodRange, calendarActualsThrough);
        const calendarPeriods = calendarPlan.toArray().map((p) => ({
          id: p.id.code,
          start: toIso(p.start),
          end: toIso(p.end),
          actual: p.isActual,
        }));

        // Fiscal periods (US Federal: Oct 1 - Sep 30)
        const fiscalPlan = buildFiscalPeriods(fiscalPeriodRange, FiscalConfig.usFederal(), null);
        const fiscalPeriods = fiscalPlan.toArray().map((p) => ({
          id: p.id.code,
          start: toIso(p.start),
          end: toIso(p.end),
        }));

        if (!cancelled) {
          setData({
            calendar: calendarPeriods,
            fiscal: fiscalPeriods,
          });
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [calendarPeriodRange, calendarActualsThrough, fiscalPeriodRange]);

  if (error) return <p className="error">{error}</p>;
  if (data.calendar.length === 0) return <p>Loading...</p>;

  return (
    <section className="example-section">
      <h2>Period Plans</h2>

      <h3 style={{ fontSize: '1.2rem', marginTop: '1rem', marginBottom: '0.5rem' }}>
        Calendar Periods (with Actuals)
      </h3>
      <table>
        <thead>
          <tr>
            <th>Period</th>
            <th>Start</th>
            <th>End</th>
            <th>Actual?</th>
          </tr>
        </thead>
        <tbody>
          {data.calendar.map((row) => (
            <tr key={row.id}>
              <td>{row.id}</td>
              <td>{row.start}</td>
              <td>{row.end}</td>
              <td>{row.actual ? 'yes' : 'no'}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3 style={{ fontSize: '1.2rem', marginTop: '2rem', marginBottom: '0.5rem' }}>
        Fiscal Periods (US Federal)
      </h3>
      <table>
        <thead>
          <tr>
            <th>Period</th>
            <th>Start</th>
            <th>End</th>
          </tr>
        </thead>
        <tbody>
          {data.fiscal.map((row) => (
            <tr key={row.id}>
              <td>{row.id}</td>
              <td>{row.start}</td>
              <td>{row.end}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};

// 7. IMM Dates & Option Expiries
export const IMMDatesExample: React.FC<IMMDatesExampleProps> = (props) => {
  const defaults = DEFAULT_IMM_DATES as RequiredIMMDatesExampleProps;
  const {
    referenceDate = defaults.referenceDate,
    immYears = defaults.immYears,
    immMonths = defaults.immMonths,
  } = props;

  const [data, setData] = useState<{ [key: string]: string }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const today = new FsDate(referenceDate.year, referenceDate.month, referenceDate.day);

        const results: { [key: string]: string } = {
          'Reference Date': toIso(today),
          'Next IMM': toIso(nextImm(today)),
          'Next CDS Date': toIso(nextCdsDate(today)),
          'Next Equity Option Expiry': toIso(nextEquityOptionExpiry(today)),
        };

        // Add third Friday/Wednesday for specified years and months
        for (const year of immYears) {
          for (const month of immMonths) {
            const monthNames = [
              '',
              'Jan',
              'Feb',
              'Mar',
              'Apr',
              'May',
              'Jun',
              'Jul',
              'Aug',
              'Sep',
              'Oct',
              'Nov',
              'Dec',
            ];
            results[`Third Friday ${monthNames[month]} ${year}`] = toIso(thirdFriday(year, month));
          }
        }

        // Add third Wednesday for first year/month
        if (immYears.length > 0 && immMonths.length > 0) {
          const monthNames = [
            '',
            'Jan',
            'Feb',
            'Mar',
            'Apr',
            'May',
            'Jun',
            'Jul',
            'Aug',
            'Sep',
            'Oct',
            'Nov',
            'Dec',
          ];
          results[`Third Wednesday ${monthNames[immMonths[0]]} ${immYears[0]}`] = toIso(
            thirdWednesday(immYears[0], immMonths[0])
          );
        }

        if (!cancelled) {
          setData(results);
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [referenceDate, immYears, immMonths]);

  if (error) return <p className="error">{error}</p>;
  if (Object.keys(data).length === 0) return <p>Loading...</p>;

  return (
    <section className="example-section">
      <h2>IMM Dates & Option Expiries</h2>
      <dl className="data-list">
        {Object.entries(data).map(([key, value]) => (
          <React.Fragment key={key}>
            <dt>{key}</dt>
            <dd>{value}</dd>
          </React.Fragment>
        ))}
      </dl>
    </section>
  );
};

// 8. Frequency Examples - no data needed
export const FrequencyExample: React.FC = () => {
  const [data, setData] = useState<{ [key: string]: string }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const annual = Frequency.annual();
        const semiAnnual = Frequency.semiAnnual();
        const quarterly = Frequency.quarterly();
        const monthly = Frequency.monthly();
        const biMonthly = Frequency.biMonthly();
        const weekly = Frequency.weekly();
        const biWeekly = Frequency.biWeekly();
        const daily = Frequency.daily();

        const customMonths = Frequency.fromMonths(3);
        const customDays = Frequency.fromDays(91);

        const results: { [key: string]: string } = {
          Annual: `${annual.months} months`,
          'Semi-Annual': `${semiAnnual.months} months`,
          Quarterly: `${quarterly.months} months`,
          Monthly: `${monthly.months} months`,
          'Bi-Monthly': `${biMonthly.months} months`,
          Weekly: `${weekly.days} days`,
          'Bi-Weekly': `${biWeekly.days} days`,
          Daily: `${daily.days} days`,
          'Custom (3 months)': `${customMonths.months} months`,
          'Custom (91 days)': `${customDays.days} days`,
        };

        if (!cancelled) {
          setData(results);
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) return <p className="error">{error}</p>;
  if (Object.keys(data).length === 0) return <p>Loading...</p>;

  return (
    <section className="example-section">
      <h2>Frequency Conventions</h2>
      <dl className="data-list">
        {Object.entries(data).map(([key, value]) => (
          <React.Fragment key={key}>
            <dt>{key}</dt>
            <dd>{value}</dd>
          </React.Fragment>
        ))}
      </dl>
    </section>
  );
};
