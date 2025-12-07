/**
 * Dates showcase fixture data.
 */

import { DateData } from './market-data';

// Date construction example props
export interface DateConstructionProps {
  exampleDate?: DateData;
  weekdaysToAdd?: number;
}

// Date utilities example props
export interface DateUtilitiesProps {
  baseDate?: DateData;
  monthsToAdd?: number[];
}

// Calendar example props
export interface CalendarExampleProps {
  saturdayDate?: DateData;
  holidayDate?: DateData;
  calendarCodes?: string[];
}

// Day count example props
export interface DayCountExampleProps {
  startDate?: DateData;
  endDate?: DateData;
  calendarCode?: string;
}

// Schedule builder example props
export interface ScheduleBuilderExampleProps {
  startDate?: DateData;
  endDate?: DateData;
  cdsStartDate?: DateData;
  cdsEndDate?: DateData;
  calendarCode?: string;
}

// Period plans example props
export interface PeriodPlansExampleProps {
  calendarPeriodRange?: string;
  calendarActualsThrough?: string;
  fiscalPeriodRange?: string;
}

// IMM dates example props
export interface IMMDatesExampleProps {
  referenceDate?: DateData;
  immYears?: number[];
  immMonths?: number[];
}

// Frequency example props - no data needed, just demonstrates API

// Default values for date construction
export const DEFAULT_DATE_CONSTRUCTION: DateConstructionProps = {
  exampleDate: { year: 2024, month: 9, day: 30 },
  weekdaysToAdd: 5,
};

// Default values for date utilities
export const DEFAULT_DATE_UTILITIES: DateUtilitiesProps = {
  baseDate: { year: 2025, month: 1, day: 31 },
  monthsToAdd: [1, 3],
};

// Default values for calendar example
export const DEFAULT_CALENDAR_EXAMPLE: CalendarExampleProps = {
  saturdayDate: { year: 2025, month: 1, day: 4 },
  holidayDate: { year: 2024, month: 12, day: 25 },
  calendarCodes: ['usny', 'gblo'],
};

// Default values for day count example
export const DEFAULT_DAY_COUNT_EXAMPLE: DayCountExampleProps = {
  startDate: { year: 2024, month: 1, day: 15 },
  endDate: { year: 2024, month: 7, day: 15 },
  calendarCode: 'usny',
};

// Default values for schedule builder example
export const DEFAULT_SCHEDULE_BUILDER: ScheduleBuilderExampleProps = {
  startDate: { year: 2024, month: 1, day: 15 },
  endDate: { year: 2024, month: 12, day: 15 },
  cdsStartDate: { year: 2024, month: 3, day: 20 },
  cdsEndDate: { year: 2029, month: 3, day: 20 },
  calendarCode: 'usny',
};

// Default values for period plans example
export const DEFAULT_PERIOD_PLANS: PeriodPlansExampleProps = {
  calendarPeriodRange: '2024Q1..Q4',
  calendarActualsThrough: '2024Q2',
  fiscalPeriodRange: '2024Q1..2025Q2',
};

// Default values for IMM dates example
export const DEFAULT_IMM_DATES: IMMDatesExampleProps = {
  referenceDate: { year: 2024, month: 9, day: 30 },
  immYears: [2025],
  immMonths: [3, 6, 9, 12],
};

// Export all defaults
export const DEFAULT_DATES_SHOWCASE_PROPS = {
  dateConstruction: DEFAULT_DATE_CONSTRUCTION,
  dateUtilities: DEFAULT_DATE_UTILITIES,
  calendar: DEFAULT_CALENDAR_EXAMPLE,
  dayCount: DEFAULT_DAY_COUNT_EXAMPLE,
  scheduleBuilder: DEFAULT_SCHEDULE_BUILDER,
  periodPlans: DEFAULT_PERIOD_PLANS,
  immDates: DEFAULT_IMM_DATES,
};

