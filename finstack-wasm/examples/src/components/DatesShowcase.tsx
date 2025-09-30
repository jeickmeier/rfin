import React, { useEffect, useState } from "react";
import {
  Date as FsDate,
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
} from "finstack-wasm";

const toIso = (date: FsDate) => {
  const month = String(date.month).padStart(2, "0");
  const day = String(date.day).padStart(2, "0");
  return `${date.year}-${month}-${day}`;
};

// 1. Date Construction & Manipulation Example
export const DateConstructionExample: React.FC = () => {
  const [data, setData] = useState<{ [key: string]: string | number | boolean }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const date = new FsDate(2024, 9, 30); // September 30, 2024
        const results: { [key: string]: string | number | boolean } = {
          "Date": toIso(date),
          "Year": date.year,
          "Month": date.month,
          "Day": date.day,
          "Quarter": date.quarter(),
          "Is Weekend": date.isWeekend(),
          "Fiscal Year": date.fiscalYear(),
          "Add 5 weekdays": toIso(date.addWeekdays(5)),
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
export const DateUtilitiesExample: React.FC = () => {
  const [data, setData] = useState<{ [key: string]: string | number | boolean }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const base = new FsDate(2025, 1, 31); // January 31, 2025
        const epochDays = dateToDaysSinceEpoch(base);

        const results: { [key: string]: string | number | boolean } = {
          "Base Date": toIso(base),
          "Add 1 month": toIso(addMonths(base, 1)),
          "Add 3 months": toIso(addMonths(base, 3)),
          "Last day of month": toIso(lastDayOfMonth(base)),
          "Days in Jan 2025": daysInMonth(2025, 1),
          "Days in Feb 2024": daysInMonth(2024, 2),
          "Days in Feb 2025": daysInMonth(2025, 2),
          "Is 2024 leap year": isLeapYear(2024),
          "Is 2025 leap year": isLeapYear(2025),
          "Days since epoch": epochDays,
          "Epoch round-trip": toIso(daysSinceEpochToDate(epochDays)),
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
export const CalendarExample: React.FC = () => {
  const [data, setData] = useState<{ [key: string]: string | boolean }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const codes = availableCalendarCodes();
        const usny = getCalendar("usny");
        const gblo = getCalendar("gblo");

        const saturday = new FsDate(2025, 1, 4); // Saturday
        const christmas = new FsDate(2024, 12, 25); // Christmas

        const results: { [key: string]: string | boolean } = {
          "Available Calendars": codes.length.toString(),
          "Sample Calendars": codes.slice(0, 5).join(", "),
          "US NY Name": usny.name,
          "US NY Ignores Weekends": usny.ignoreWeekends,
          "GB LO Name": gblo.name,
          "Saturday is weekend": saturday.isWeekend(),
          "Saturday is US business day": usny.isBusinessDay(saturday),
          "Christmas is US holiday": usny.isHoliday(christmas),
          "Adjusted (Following)": toIso(adjust(saturday, BusinessDayConvention.Following, usny)),
          "Adjusted (Preceding)": toIso(adjust(saturday, BusinessDayConvention.Preceding, usny)),
          "Adjusted (Modified Following)": toIso(
            adjust(saturday, BusinessDayConvention.ModifiedFollowing, usny)
          ),
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
export const DayCountExample: React.FC = () => {
  const [data, setData] = useState<{ [key: string]: string }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const start = new FsDate(2024, 1, 15);
        const end = new FsDate(2024, 7, 15);
        const calendar = getCalendar("usny");

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
          "Start Date": toIso(start),
          "End Date": toIso(end),
          "Act/360": act360.yearFraction(start, end, null).toFixed(6),
          "Act/365F": act365f.yearFraction(start, end, null).toFixed(6),
          "30/360": thirty360.yearFraction(start, end, null).toFixed(6),
          "Act/Act (ISDA)": actAct.yearFraction(start, end, null).toFixed(6),
          "Act/Act (ISMA)": actActIsma.yearFraction(start, end, ctxIsma).toFixed(6),
          "BUS/252": bus252.yearFraction(start, end, ctxBus).toFixed(6),
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
export const ScheduleBuilderExample: React.FC = () => {
  const [schedules, setSchedules] = useState<{ [key: string]: string[] }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const start = new FsDate(2024, 1, 15);
        const end = new FsDate(2024, 12, 15);
        const calendar = getCalendar("usny");

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
        const cdsStart = new FsDate(2024, 3, 20); // Standard CDS date
        const cdsEnd = new FsDate(2029, 3, 20); // 5-year CDS
        const cdsSchedule = new ScheduleBuilder(cdsStart, cdsEnd)
          .frequency(Frequency.quarterly())
          .cdsImm()
          .adjustWith(BusinessDayConvention.Following, calendar)
          .build();

        const results = {
          "Monthly (Modified Following)": monthly.toArray().map((d) => toIso(d as FsDate)),
          "Quarterly (Short Back)": quarterly.toArray().map((d) => toIso(d as FsDate)),
          "Semi-Annual": semiAnnual.toArray().map((d) => toIso(d as FsDate)),
          "CDS IMM (5Y)": cdsSchedule.toArray().map((d) => toIso(d as FsDate)),
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
  }, []);

  if (error) return <p className="error">{error}</p>;
  if (Object.keys(schedules).length === 0) return <p>Loading...</p>;

  return (
    <section className="example-section">
      <h2>Schedule Builder</h2>
      {Object.entries(schedules).map(([title, dates]) => (
        <div key={title} style={{ marginBottom: "2rem" }}>
          <h3 style={{ fontSize: "1.2rem", marginBottom: "0.5rem" }}>{title}</h3>
          <div style={{ display: "flex", flexWrap: "wrap", gap: "0.5rem" }}>
            {dates.map((date, idx) => (
              <span
                key={idx}
                style={{
                  padding: "0.25rem 0.5rem",
                  backgroundColor: "rgba(100, 108, 255, 0.1)",
                  borderRadius: "4px",
                  fontSize: "0.9rem",
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
export const PeriodPlansExample: React.FC = () => {
  const [data, setData] = useState<{
    calendar: Array<{ id: string; start: string; end: string; actual: boolean }>;
    fiscal: Array<{ id: string; start: string; end: string }>;
  }>({ calendar: [], fiscal: [] });
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        // Calendar periods with actuals until Q2
        const calendarPlan = buildPeriods("2024Q1..Q4", "2024Q2");
        const calendarPeriods = calendarPlan.toArray().map((p) => ({
          id: p.id.code,
          start: toIso(p.start),
          end: toIso(p.end),
          actual: p.isActual,
        }));

        // Fiscal periods (US Federal: Oct 1 - Sep 30)
        const fiscalPlan = buildFiscalPeriods("2024Q1..2025Q2", FiscalConfig.usFederal(), null);
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
  }, []);

  if (error) return <p className="error">{error}</p>;
  if (data.calendar.length === 0) return <p>Loading...</p>;

  return (
    <section className="example-section">
      <h2>Period Plans</h2>

      <h3 style={{ fontSize: "1.2rem", marginTop: "1rem", marginBottom: "0.5rem" }}>
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
              <td>{row.actual ? "yes" : "no"}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3 style={{ fontSize: "1.2rem", marginTop: "2rem", marginBottom: "0.5rem" }}>
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
export const IMMDatesExample: React.FC = () => {
  const [data, setData] = useState<{ [key: string]: string }>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const today = new FsDate(2024, 9, 30);

        const results: { [key: string]: string } = {
          "Reference Date": toIso(today),
          "Next IMM": toIso(nextImm(today)),
          "Next CDS Date": toIso(nextCdsDate(today)),
          "Next Equity Option Expiry": toIso(nextEquityOptionExpiry(today)),
          "Third Friday Mar 2025": toIso(thirdFriday(2025, 3)),
          "Third Wednesday Mar 2025": toIso(thirdWednesday(2025, 3)),
          "Third Friday Jun 2025": toIso(thirdFriday(2025, 6)),
          "Third Friday Sep 2025": toIso(thirdFriday(2025, 9)),
          "Third Friday Dec 2025": toIso(thirdFriday(2025, 12)),
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

// 8. Frequency Examples
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
          "Annual": `${annual.months} months`,
          "Semi-Annual": `${semiAnnual.months} months`,
          "Quarterly": `${quarterly.months} months`,
          "Monthly": `${monthly.months} months`,
          "Bi-Monthly": `${biMonthly.months} months`,
          "Weekly": `${weekly.days} days`,
          "Bi-Weekly": `${biWeekly.days} days`,
          "Daily": `${daily.days} days`,
          "Custom (3 months)": `${customMonths.months} months`,
          "Custom (91 days)": `${customDays.days} days`,
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
