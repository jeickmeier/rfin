/**
 * Structured credit instruments fixture data.
 */

import { DiscountCurveData, DateData, DEFAULT_VALUATION_DATE } from './market-data';

// Hazard curve for structured credit
export interface StructuredCreditHazardData {
  id: string;
  baseDate: DateData;
  tenors: number[];
  hazardRates: number[];
  recoveryRate: number;
  dayCount: string;
}

// Structured credit deal definition (passed as JSON string)
export interface StructuredCreditDealData {
  name: string;
  type: 'CLO' | 'ABS' | 'RMBS' | 'CMBS';
  json: string;
  totalSize: number;
  trancheCount: number;
  description: string;
  poolWal: number;
  poolWac: number;
}

export interface StructuredCreditExampleProps {
  valuationDate?: DateData;
  discountCurve?: DiscountCurveData;
  hazardCurve?: StructuredCreditHazardData;
  deals?: StructuredCreditDealData[];
}

// Default discount curve for structured credit
export const DEFAULT_SC_DISCOUNT_CURVE: DiscountCurveData = {
  id: 'USD-OIS',
  baseDate: DEFAULT_VALUATION_DATE,
  tenors: [0, 1, 3, 5, 7, 10],
  discountFactors: [1, 0.995, 0.98, 0.96, 0.935, 0.905],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

// Default hazard curve
export const DEFAULT_SC_HAZARD_CURVE: StructuredCreditHazardData = {
  id: 'POOL-HZD',
  baseDate: DEFAULT_VALUATION_DATE,
  tenors: [0, 3, 5, 7],
  hazardRates: [0.008, 0.012, 0.015, 0.018],
  recoveryRate: 0.4,
  dayCount: 'act_365f',
};

// CLO Deal JSON
export const CLO_DEAL_JSON = {
  id: 'clo_2024_1',
  deal_type: 'CLO',
  discount_curve_id: 'USD-OIS',
  payment_calendar_id: 'nyse',
  closing_date: '2024-01-02',
  first_payment_date: '2024-04-15',
  reinvestment_end_date: '2027-01-15',
  legal_maturity: '2031-01-15',
  payment_frequency: { Months: 3 },
  deal_metadata: { manager: 'CLO_Manager', servicer: 'CLO_Servicer' },
  attributes: { tags: [], meta: {} },
  pool: {
    id: 'clo_pool_2024_1',
    deal_type: 'CLO',
    assets: [
      {
        id: 'loan_001',
        asset_type: { type: 'FirstLienLoan', industry: 'Technology' },
        balance: { amount: 50_000_000, currency: 'USD' },
        rate: 0.078,
        spread_bps: 450,
        index_id: null,
        maturity: '2030-01-15',
        credit_quality: 'B',
        industry: 'Technology',
        obligor_id: 'BORROWER_001',
        is_defaulted: false,
        recovery_amount: null,
        purchase_price: { amount: 50_000_000, currency: 'USD' },
        acquisition_date: '2024-01-02',
        day_count: 'Act360',
      },
      {
        id: 'loan_002',
        asset_type: { type: 'FirstLienLoan', industry: 'Healthcare' },
        balance: { amount: 75_000_000, currency: 'USD' },
        rate: 0.072,
        spread_bps: 400,
        index_id: null,
        maturity: '2029-06-15',
        credit_quality: 'BB',
        industry: 'Healthcare',
        obligor_id: 'BORROWER_002',
        is_defaulted: false,
        recovery_amount: null,
        purchase_price: { amount: 75_000_000, currency: 'USD' },
        acquisition_date: '2024-01-02',
        day_count: 'Act360',
      },
      {
        id: 'loan_003',
        asset_type: { type: 'FirstLienLoan', industry: 'Manufacturing' },
        balance: { amount: 100_000_000, currency: 'USD' },
        rate: 0.075,
        spread_bps: 425,
        index_id: null,
        maturity: '2030-12-15',
        credit_quality: 'B',
        industry: 'Manufacturing',
        obligor_id: 'BORROWER_003',
        is_defaulted: false,
        recovery_amount: null,
        purchase_price: { amount: 100_000_000, currency: 'USD' },
        acquisition_date: '2024-01-02',
        day_count: 'Act360',
      },
      {
        id: 'loan_004',
        asset_type: { type: 'SecondLienLoan', industry: 'Retail' },
        balance: { amount: 80_000_000, currency: 'USD' },
        rate: 0.08,
        spread_bps: 475,
        index_id: null,
        maturity: '2028-09-15',
        credit_quality: 'B',
        industry: 'Retail',
        obligor_id: 'BORROWER_004',
        is_defaulted: false,
        recovery_amount: null,
        purchase_price: { amount: 80_000_000, currency: 'USD' },
        acquisition_date: '2024-01-02',
        day_count: 'Act360',
      },
      {
        id: 'loan_005',
        asset_type: { type: 'FirstLienLoan', industry: 'Energy' },
        balance: { amount: 95_000_000, currency: 'USD' },
        rate: 0.076,
        spread_bps: 450,
        index_id: null,
        maturity: '2031-03-15',
        credit_quality: 'BB',
        industry: 'Energy',
        obligor_id: 'BORROWER_005',
        is_defaulted: false,
        recovery_amount: null,
        purchase_price: { amount: 95_000_000, currency: 'USD' },
        acquisition_date: '2024-01-02',
        day_count: 'Act360',
      },
    ],
    cumulative_defaults: { amount: 0, currency: 'USD' },
    cumulative_recoveries: { amount: 0, currency: 'USD' },
    cumulative_prepayments: { amount: 0, currency: 'USD' },
    reinvestment_period: {
      end_date: '2027-01-15',
      is_active: true,
      criteria: {
        max_price: 100,
        min_yield: 0,
        maintain_credit_quality: true,
        maintain_wal: true,
        apply_eligibility_criteria: true,
      },
    },
    collection_account: { amount: 0, currency: 'USD' },
    reserve_account: { amount: 0, currency: 'USD' },
    excess_spread_account: { amount: 0, currency: 'USD' },
  },
  tranches: {
    total_size: { amount: 400_000_000, currency: 'USD' },
    tranches: [
      {
        id: 'clo_2024_1_class_a',
        original_balance: { amount: 280_000_000, currency: 'USD' },
        current_balance: { amount: 280_000_000, currency: 'USD' },
        coupon: { Fixed: { rate: 0.045 } },
        seniority: 'Senior',
        attachment_point: 0,
        detachment_point: 0.7,
        rating: 'AAA',
        credit_enhancement: {
          subordination: { amount: 0, currency: 'USD' },
          overcollateralization: { amount: 0, currency: 'USD' },
          reserve_account: { amount: 0, currency: 'USD' },
          excess_spread: 0,
          cash_trap_active: false,
        },
        payment_frequency: { Months: 3 },
        day_count: 'Act360',
        deferred_interest: { amount: 0, currency: 'USD' },
        is_revolving: false,
        can_reinvest: true,
        legal_maturity: '2031-01-15',
        payment_priority: 1,
        attributes: { tags: [], meta: {} },
      },
      {
        id: 'clo_2024_1_class_b',
        original_balance: { amount: 60_000_000, currency: 'USD' },
        current_balance: { amount: 60_000_000, currency: 'USD' },
        coupon: { Fixed: { rate: 0.065 } },
        seniority: 'Mezzanine',
        attachment_point: 0.7,
        detachment_point: 0.85,
        rating: 'AA',
        credit_enhancement: {
          subordination: { amount: 0, currency: 'USD' },
          overcollateralization: { amount: 0, currency: 'USD' },
          reserve_account: { amount: 0, currency: 'USD' },
          excess_spread: 0,
          cash_trap_active: false,
        },
        payment_frequency: { Months: 3 },
        day_count: 'Act360',
        deferred_interest: { amount: 0, currency: 'USD' },
        is_revolving: false,
        can_reinvest: true,
        legal_maturity: '2031-01-15',
        payment_priority: 2,
        attributes: { tags: [], meta: {} },
      },
      {
        id: 'clo_2024_1_class_c',
        original_balance: { amount: 40_000_000, currency: 'USD' },
        current_balance: { amount: 40_000_000, currency: 'USD' },
        coupon: { Fixed: { rate: 0.095 } },
        seniority: 'Subordinated',
        attachment_point: 0.85,
        detachment_point: 0.95,
        rating: 'BBB',
        credit_enhancement: {
          subordination: { amount: 0, currency: 'USD' },
          overcollateralization: { amount: 0, currency: 'USD' },
          reserve_account: { amount: 0, currency: 'USD' },
          excess_spread: 0,
          cash_trap_active: false,
        },
        payment_frequency: { Months: 3 },
        day_count: 'Act360',
        deferred_interest: { amount: 0, currency: 'USD' },
        is_revolving: false,
        can_reinvest: true,
        legal_maturity: '2031-01-15',
        payment_priority: 3,
        attributes: { tags: [], meta: {} },
      },
      {
        id: 'clo_2024_1_equity',
        original_balance: { amount: 20_000_000, currency: 'USD' },
        current_balance: { amount: 20_000_000, currency: 'USD' },
        coupon: { Fixed: { rate: 0 } },
        seniority: 'Equity',
        attachment_point: 0.95,
        detachment_point: 1,
        rating: 'NR',
        credit_enhancement: {
          subordination: { amount: 0, currency: 'USD' },
          overcollateralization: { amount: 0, currency: 'USD' },
          reserve_account: { amount: 0, currency: 'USD' },
          excess_spread: 0,
          cash_trap_active: false,
        },
        payment_frequency: { Months: 3 },
        day_count: 'Act360',
        deferred_interest: { amount: 0, currency: 'USD' },
        is_revolving: false,
        can_reinvest: true,
        legal_maturity: '2031-01-15',
        payment_priority: 4,
        attributes: { tags: [], meta: {} },
      },
    ],
  },
  market_conditions: {
    refi_rate: 0.04,
    original_rate: null,
    hpa: null,
    unemployment: null,
    seasonal_factor: null,
    custom_factors: {},
  },
  credit_factors: {
    credit_score: null,
    dti: null,
    ltv: null,
    delinquency_days: 0,
    unemployment_rate: null,
    custom_factors: {},
  },
};

// Default deals (simplified to just CLO for now)
export const DEFAULT_SC_DEALS: StructuredCreditDealData[] = [
  {
    name: 'CLO 2024-1',
    type: 'CLO',
    json: JSON.stringify(CLO_DEAL_JSON),
    totalSize: 400_000_000,
    trancheCount: 4,
    description: 'Senior secured leveraged loans with 4-tranche structure',
    poolWal: 5.5,
    poolWac: 7.6,
  },
];

// Complete props bundle
export const DEFAULT_STRUCTURED_CREDIT_PROPS: StructuredCreditExampleProps = {
  valuationDate: DEFAULT_VALUATION_DATE,
  discountCurve: DEFAULT_SC_DISCOUNT_CURVE,
  hazardCurve: DEFAULT_SC_HAZARD_CURVE,
  deals: DEFAULT_SC_DEALS,
};
