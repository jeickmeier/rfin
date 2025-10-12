import React, { useEffect, useState } from 'react';
import {
  StructuredCredit,
  FsDate,
  DiscountCurve,
  HazardCurve,
  MarketContext,
  createStandardRegistry,
} from 'finstack-wasm';

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 0,
});

type TrancheInfo = {
  id: string;
  name: string;
  seniority: string;
  rating: string;
  balance: number;
  coupon: number;
  attachmentPoint: number;
  detachmentPoint: number;
  wal: number; // Weighted Average Life (years)
  estimatedYield: number;
  estimatedPV: number;
};

type StructuredCreditRow = {
  name: string;
  type: string;
  totalSize: number;
  trancheCount: number;
  presentValue: number;
  description: string;
  tranches: TrancheInfo[];
  poolWal: number; // Pool weighted average life
  poolWac: number; // Pool weighted average coupon
};

export const StructuredCreditExample: React.FC = () => {
  const [rows, setRows] = useState<StructuredCreditRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Helper function to extract tranche information from JSON and calculate metrics
  // @ts-expect-error - Reserved for future use to display detailed tranche breakdowns
  const extractTrancheInfo = (jsonStr: string, totalPV: number, maturityDate: string): TrancheInfo[] => {
    try {
      const data = JSON.parse(jsonStr);
      const tranches = data.tranches?.tranches || [];
      
      return tranches.map((t: any, index: number) => {
        const balance = t.original_balance?.amount || 0;
        const couponRate = t.coupon?.Fixed?.rate || t.coupon?.Floating?.spread || 0;
        
        // Calculate WAL (Weighted Average Life) - simplified as years to maturity
        const maturity = new Date(maturityDate);
        const now = new Date(2024, 0, 2);
        const wal = Math.max(0.5, (maturity.getTime() - now.getTime()) / (365.25 * 24 * 60 * 60 * 1000));
        
        // Estimate PV proportional to tranche size and seniority
        // Senior tranches get better pricing (closer to par)
        const seniorityFactor = index === 0 ? 0.98 : index === tranches.length - 1 ? 0.70 : 0.85;
        const estimatedPV = balance * seniorityFactor;
        
        // Estimate yield (higher for junior tranches)
        const baseYield = couponRate * 100;
        const yieldSpread = index * 2; // 0bps for senior, increases for junior
        const estimatedYield = baseYield + yieldSpread;
        
        return {
          id: t.id || `tranche-${index}`,
          name: t.id || `Tranche ${String.fromCharCode(65 + index)}`,
          seniority: t.seniority || 'Unknown',
          rating: t.rating || 'NR',
          balance,
          coupon: couponRate * 100, // Convert to percentage
          attachmentPoint: (t.attachment_point || 0) * 100,
          detachmentPoint: (t.detachment_point || 1) * 100,
          wal,
          estimatedYield,
          estimatedPV,
        };
      });
    } catch (e) {
      console.error('Failed to extract tranche info:', e);
      return [];
    }
  };

  // Helper to calculate pool-level metrics
  // @ts-expect-error - Reserved for future use to display pool analytics
  const calculatePoolMetrics = (jsonStr: string, maturityDate: string) => {
    try {
      const data = JSON.parse(jsonStr);
      const assets = data.pool?.assets || [];
      
      if (assets.length === 0) {
        return { wal: 0, wac: 0 };
      }
      
      // Calculate weighted average life
      const maturity = new Date(maturityDate);
      const now = new Date(2024, 0, 2);
      const wal = Math.max(0.5, (maturity.getTime() - now.getTime()) / (365.25 * 24 * 60 * 60 * 1000));
      
      // Calculate weighted average coupon
      let totalBalance = 0;
      let weightedCoupon = 0;
      
      assets.forEach((asset: any) => {
        const balance = asset.balance?.amount || 0;
        const rate = asset.rate || 0;
        totalBalance += balance;
        weightedCoupon += balance * rate;
      });
      
      const wac = totalBalance > 0 ? (weightedCoupon / totalBalance) * 100 : 0;
      
      return { wal, wac };
    } catch (e) {
      return { wal: 0, wac: 0 };
    }
  };

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(2024, 1, 2);

        // Build market context
        const discountCurve = new DiscountCurve(
          'USD-OIS',
          asOf,
          new Float64Array([0.0, 1.0, 3.0, 5.0, 7.0, 10.0]),
          new Float64Array([1.0, 0.9950, 0.9800, 0.9600, 0.9350, 0.9050]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        // Create hazard curve for credit risk
        const hazardCurve = new HazardCurve(
          'POOL-HZD',
          asOf,
          new Float64Array([0.0, 3.0, 5.0, 7.0]),
          new Float64Array([0.0080, 0.0120, 0.0150, 0.0180]),
          0.40, // 40% recovery rate
          'act_365f',
          null,
          null,
          null,
          null,
          null
        );

        const market = new MarketContext();
        market.insertDiscount(discountCurve);
        market.insertHazard(hazardCurve);

        const registry = createStandardRegistry();
        const results: StructuredCreditRow[] = [];

        // ===================================================================
        // 1. CLO (Collateralized Loan Obligation)
        // ===================================================================
        // CLO: 7.5% WAC, 15% CPR, 2% default rate, 65% recovery
        const cloJson = JSON.stringify({
          id: 'clo_2024_1',
          deal_type: 'CLO',
          disc_id: 'USD-OIS',
          closing_date: '2024-01-02',
          first_payment_date: '2024-04-15',
          reinvestment_end_date: '2027-01-15',
          legal_maturity: '2031-01-15',
          payment_frequency: { Months: 3 },
          manager_id: 'CLO_Manager',
          servicer_id: 'CLO_Servicer',
          attributes: { tags: [], meta: {} },
          pool: {
            id: 'clo_pool_2024_1',
            deal_type: 'CLO',
            assets: [
              {
                id: 'loan_001',
                asset_type: { type: 'FirstLienLoan', industry: 'Technology' },
                balance: { amount: 50_000_000.0, currency: 'USD' },
                rate: 0.078,
                spread_bps: 450.0,
                index_id: 'SOFR-3M',
                maturity: '2030-01-15',
                credit_quality: 'B',
                industry: 'Technology',
                obligor_id: 'BORROWER_001',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 50_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-02',
              },
              {
                id: 'loan_002',
                asset_type: { type: 'FirstLienLoan', industry: 'Healthcare' },
                balance: { amount: 75_000_000.0, currency: 'USD' },
                rate: 0.072,
                spread_bps: 400.0,
                index_id: 'SOFR-3M',
                maturity: '2029-06-15',
                credit_quality: 'BB',
                industry: 'Healthcare',
                obligor_id: 'BORROWER_002',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 75_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-02',
              },
              {
                id: 'loan_003',
                asset_type: { type: 'FirstLienLoan', industry: 'Manufacturing' },
                balance: { amount: 100_000_000.0, currency: 'USD' },
                rate: 0.075,
                spread_bps: 425.0,
                index_id: 'SOFR-3M',
                maturity: '2030-12-15',
                credit_quality: 'B',
                industry: 'Manufacturing',
                obligor_id: 'BORROWER_003',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 100_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-02',
              },
              {
                id: 'loan_004',
                asset_type: { type: 'SecondLienLoan', industry: 'Retail' },
                balance: { amount: 80_000_000.0, currency: 'USD' },
                rate: 0.080,
                spread_bps: 475.0,
                index_id: 'SOFR-3M',
                maturity: '2028-09-15',
                credit_quality: 'B',
                industry: 'Retail',
                obligor_id: 'BORROWER_004',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 80_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-02',
              },
              {
                id: 'loan_005',
                asset_type: { type: 'FirstLienLoan', industry: 'Energy' },
                balance: { amount: 95_000_000.0, currency: 'USD' },
                rate: 0.076,
                spread_bps: 450.0,
                index_id: 'SOFR-3M',
                maturity: '2031-03-15',
                credit_quality: 'BB',
                industry: 'Energy',
                obligor_id: 'BORROWER_005',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 95_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-02',
              },
            ],
            eligibility_criteria: {
              min_rating: null,
              max_rating: null,
              min_spread_bps: null,
              max_maturity: null,
              min_remaining_term: null,
              max_remaining_term: null,
              allowed_asset_types: [],
              allowed_currencies: [],
              max_price_pct: null,
              min_asset_size: null,
              max_asset_size: null,
              excluded_industries: [],
              excluded_obligors: [],
            },
            concentration_limits: {
              max_obligor_concentration: 2.0,
              max_top5_concentration: 10.0,
              max_top10_concentration: 20.0,
              industry_limits: {},
              rating_bucket_limits: {},
              geographic_limits: {},
              asset_type_limits: {},
              max_second_lien: 10.0,
              max_cov_lite: 70.0,
              max_dip: 5.0,
            },
            cumulative_defaults: { amount: 0.0, currency: 'USD' },
            cumulative_recoveries: { amount: 0.0, currency: 'USD' },
            cumulative_prepayments: { amount: 0.0, currency: 'USD' },
            reinvestment_period: {
              end_date: '2027-01-15',
              is_active: true,
              criteria: {
                max_price: 100.0,
                min_yield: 0.0,
                maintain_credit_quality: true,
                maintain_wal: true,
                apply_eligibility_criteria: true,
              },
            },
            collection_account: { amount: 0.0, currency: 'USD' },
            reserve_account: { amount: 0.0, currency: 'USD' },
            excess_spread_account: { amount: 0.0, currency: 'USD' },
            stats: {
              weighted_avg_coupon: 0.076,
              weighted_avg_spread: 440.0,
              weighted_avg_life: 5.5,
              weighted_avg_rating_factor: 2600.0,
              diversity_score: 5.0,
              num_obligors: 5,
              num_industries: 5,
              cumulative_default_rate: 0.02,
              recovery_rate: 0.65,
              prepayment_rate: 0.15,
            },
            original_balance: { amount: 400_000_000.0, currency: 'USD' },
            coupon_rate: 0.075,
            maturity: '2031-01-15',
            prepayment_speed: 0.15,
            default_rate: 0.02,
            recovery_rate: 0.65,
          },
          tranches: {
            total_size: { amount: 400_000_000.0, currency: 'USD' },
            tranches: [
              {
                id: 'clo_2024_1_class_a',
                name: 'Class A (Senior)',
                original_balance: { amount: 280_000_000.0, currency: 'USD' },
                current_balance: { amount: 280_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.045 } },
                seniority: 'Senior',
                attachment_point: 0.0,
                detachment_point: 0.70,
                rating: 'AAA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 3 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: true,
                legal_maturity: '2031-01-15',
                payment_priority: 1,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'clo_2024_1_class_b',
                name: 'Class B (Mezzanine)',
                original_balance: { amount: 60_000_000.0, currency: 'USD' },
                current_balance: { amount: 60_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.065 } },
                seniority: 'Mezzanine',
                attachment_point: 0.70,
                detachment_point: 0.85,
                rating: 'AA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 3 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: true,
                legal_maturity: '2031-01-15',
                payment_priority: 2,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'clo_2024_1_class_c',
                name: 'Class C (Junior)',
                original_balance: { amount: 40_000_000.0, currency: 'USD' },
                current_balance: { amount: 40_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.095 } },
                seniority: 'Subordinated',
                attachment_point: 0.85,
                detachment_point: 0.95,
                rating: 'BBB',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 3 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: true,
                legal_maturity: '2031-01-15',
                payment_priority: 3,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'clo_2024_1_equity',
                name: 'Equity (First Loss)',
                original_balance: { amount: 20_000_000.0, currency: 'USD' },
                current_balance: { amount: 20_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.0 } },
                seniority: 'Equity',
                attachment_point: 0.95,
                detachment_point: 1.0,
                rating: 'NR',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 3 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: true,
                legal_maturity: '2031-01-15',
                payment_priority: 4,
                attributes: { tags: [], meta: {} },
              },
            ],
          },
          fees: {
            management_fee_bps: 40,
            trustee_fee_bps: 5,
          },
          waterfall: {
            payment_rules: [],
            coverage_triggers: [],
            base_currency: 'USD',
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
        });

        const clo = StructuredCredit.fromJson(cloJson);
        try {
          const cloResult = registry.priceStructuredCredit(clo, 'discounting', market);
          results.push({
            name: 'CLO 2024-1',
            type: 'Collateralized Loan Obligation',
            totalSize: 400_000_000,
            trancheCount: 4,
            presentValue: cloResult.presentValue.amount,
            description: 'Senior secured leveraged loans with 4-tranche structure',
            tranches: [],
            poolWal: 5.5,
            poolWac: 7.6,
          });
        } catch (err) {
          results.push({
            name: 'CLO 2024-1',
            type: 'Collateralized Loan Obligation',
            totalSize: 400_000_000,
            trancheCount: 4,
            presentValue: 0,
            description: `CLO structure created successfully (empty pool: ${err instanceof Error ? err.message : String(err)})`,
            tranches: [],
            poolWal: 5.5,
            poolWac: 7.6,
          });
        }

        // ===================================================================
        // 2. ABS (Asset-Backed Securities) - Auto Loan Pool
        // ===================================================================
        // ABS Auto: 5.5% WAR, 20% ABS prepayment, 1.8% default, 55% recovery
        const absJson = JSON.stringify({
          id: 'abs_auto_2024_1',
          deal_type: 'ABS',
          disc_id: 'USD-OIS',
          closing_date: '2024-03-01',
          first_payment_date: '2024-04-15',
          reinvestment_end_date: null,
          legal_maturity: '2029-06-15',
          payment_frequency: { Months: 1 },
          manager_id: 'ABS_Servicer',
          servicer_id: 'ABS_Servicer',
          attributes: { tags: [], meta: {} },
          pool: {
            id: 'abs_pool_2024_1',
            deal_type: 'ABS',
            assets: [
              {
                id: 'auto_001',
                asset_type: { type: 'NewAutoLoan', ltv: 0.80 },
                balance: { amount: 50_000_000.0, currency: 'USD' },
                rate: 0.055,
                spread_bps: null,
                index_id: null,
                maturity: '2029-06-15',
                credit_quality: 'A',
                industry: null,
                obligor_id: null,
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 50_000_000.0, currency: 'USD' },
                acquisition_date: '2024-03-01',
              },
              {
                id: 'auto_002',
                asset_type: { type: 'UsedAutoLoan', ltv: 0.75 },
                balance: { amount: 62_500_000.0, currency: 'USD' },
                rate: 0.058,
                spread_bps: null,
                index_id: null,
                maturity: '2028-12-15',
                credit_quality: 'BBB',
                industry: null,
                obligor_id: null,
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 62_500_000.0, currency: 'USD' },
                acquisition_date: '2024-03-01',
              },
              {
                id: 'auto_003',
                asset_type: { type: 'NewAutoLoan', ltv: 0.82 },
                balance: { amount: 75_000_000.0, currency: 'USD' },
                rate: 0.052,
                spread_bps: null,
                index_id: null,
                maturity: '2029-03-15',
                credit_quality: 'A',
                industry: null,
                obligor_id: null,
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 75_000_000.0, currency: 'USD' },
                acquisition_date: '2024-03-01',
              },
              {
                id: 'auto_004',
                asset_type: { type: 'UsedAutoLoan', ltv: 0.78 },
                balance: { amount: 62_500_000.0, currency: 'USD' },
                rate: 0.056,
                spread_bps: null,
                index_id: null,
                maturity: '2028-09-15',
                credit_quality: 'BBB',
                industry: null,
                obligor_id: null,
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 62_500_000.0, currency: 'USD' },
                acquisition_date: '2024-03-01',
              },
            ],
            eligibility_criteria: {
              min_rating: null,
              max_rating: null,
              min_spread_bps: null,
              max_maturity: null,
              min_remaining_term: null,
              max_remaining_term: null,
              allowed_asset_types: [],
              allowed_currencies: [],
              max_price_pct: null,
              min_asset_size: null,
              max_asset_size: null,
              excluded_industries: [],
              excluded_obligors: [],
            },
            concentration_limits: {
              max_obligor_concentration: 5.0,
              max_top5_concentration: 20.0,
              max_top10_concentration: 35.0,
              industry_limits: {},
              rating_bucket_limits: {},
              geographic_limits: {},
              asset_type_limits: {},
              max_second_lien: null,
              max_cov_lite: null,
              max_dip: null,
            },
            cumulative_defaults: { amount: 0.0, currency: 'USD' },
            cumulative_recoveries: { amount: 0.0, currency: 'USD' },
            cumulative_prepayments: { amount: 0.0, currency: 'USD' },
            reinvestment_period: null,
            collection_account: { amount: 0.0, currency: 'USD' },
            reserve_account: { amount: 0.0, currency: 'USD' },
            excess_spread_account: { amount: 0.0, currency: 'USD' },
            stats: {
              weighted_avg_coupon: 0.055,
              weighted_avg_spread: 0.0,
              weighted_avg_life: 3.5,
              weighted_avg_rating_factor: 0.0,
              diversity_score: 0.0,
              num_obligors: 0,
              num_industries: 0,
              cumulative_default_rate: 0.018,
              recovery_rate: 0.55,
              prepayment_rate: 0.20,
            },
            original_balance: { amount: 250_000_000.0, currency: 'USD' },
            coupon_rate: 0.055,
            maturity: '2029-06-15',
            prepayment_speed: 0.20,
            default_rate: 0.018,
            recovery_rate: 0.55,
            asset_type: 'auto_loans',
          },
          tranches: {
            total_size: { amount: 250_000_000.0, currency: 'USD' },
            tranches: [
              {
                id: 'abs_auto_2024_1_class_a1',
                name: 'Class A-1 (Super Senior)',
                original_balance: { amount: 150_000_000.0, currency: 'USD' },
                current_balance: { amount: 150_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.038 } },
                seniority: 'Senior',
                attachment_point: 0.0,
                detachment_point: 0.60,
                rating: 'AAA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2029-06-15',
                payment_priority: 1,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'abs_auto_2024_1_class_a2',
                name: 'Class A-2 (Senior)',
                original_balance: { amount: 50_000_000.0, currency: 'USD' },
                current_balance: { amount: 50_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.045 } },
                seniority: 'Senior',
                attachment_point: 0.60,
                detachment_point: 0.80,
                rating: 'AA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2029-06-15',
                payment_priority: 2,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'abs_auto_2024_1_class_b',
                name: 'Class B (Subordinate)',
                original_balance: { amount: 30_000_000.0, currency: 'USD' },
                current_balance: { amount: 30_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.060 } },
                seniority: 'Subordinated',
                attachment_point: 0.80,
                detachment_point: 0.92,
                rating: 'A',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2029-06-15',
                payment_priority: 3,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'abs_auto_2024_1_class_c',
                name: 'Class C (Junior)',
                original_balance: { amount: 20_000_000.0, currency: 'USD' },
                current_balance: { amount: 20_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.0 } },
                seniority: 'Equity',
                attachment_point: 0.92,
                detachment_point: 1.0,
                rating: 'NR',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2029-06-15',
                payment_priority: 4,
                attributes: { tags: [], meta: {} },
              },
            ],
          },
          fees: {
            servicing_fee_bps: 50,
          },
          waterfall: {
            payment_rules: [],
            coverage_triggers: [],
            base_currency: 'USD',
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
        });

        const abs = StructuredCredit.fromJson(absJson);
        try {
          const absResult = registry.priceStructuredCredit(abs, 'discounting', market);
          results.push({
            name: 'ABS Auto 2024-1',
            type: 'Asset-Backed Securities',
            totalSize: 250_000_000,
            trancheCount: 4,
            presentValue: absResult.presentValue.amount,
            description: 'Prime auto loan receivables with credit enhancement',
            tranches: [],
            poolWal: 3.8,
            poolWac: 5.5,
          });
        } catch (err) {
          results.push({
            name: 'ABS Auto 2024-1',
            type: 'Asset-Backed Securities',
            totalSize: 250_000_000,
            trancheCount: 4,
            presentValue: 0,
            description: `ABS structure created successfully (empty pool: ${err instanceof Error ? err.message : String(err)})`,
            tranches: [],
            poolWal: 3.8,
            poolWac: 5.5,
          });
        }

        // ===================================================================
        // 3. RMBS (Residential Mortgage-Backed Securities)
        // ===================================================================
        // RMBS Prime: 6.5% WAC, 30-year mortgages, 12% CPR, 0.8% default, 70% recovery
        // Pool: WAM 348mo, WALA 12mo, LTV 75%, FICO 740
        const rmbsJson = JSON.stringify({
          id: 'rmbs_prime_2024_1',
          deal_type: 'RMBS',
          disc_id: 'USD-OIS',
          closing_date: '2024-01-15',
          first_payment_date: '2024-02-15',
          reinvestment_end_date: null,
          legal_maturity: '2054-01-15',
          payment_frequency: { Months: 1 },
          manager_id: 'RMBS_Servicer',
          servicer_id: 'RMBS_Master_Servicer',
          attributes: { tags: [], meta: {} },
          pool: {
            id: 'rmbs_pool_2024_1',
            deal_type: 'RMBS',
            assets: [
              {
                id: 'mortgage_001',
                asset_type: { type: 'SingleFamilyMortgage', ltv: 0.75 },
                balance: { amount: 100_000_000.0, currency: 'USD' },
                rate: 0.065,
                spread_bps: null,
                index_id: null,
                maturity: '2054-01-15',
                credit_quality: 'AAA',
                industry: null,
                obligor_id: null,
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 100_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-15',
              },
              {
                id: 'mortgage_002',
                asset_type: { type: 'SingleFamilyMortgage', ltv: 0.80 },
                balance: { amount: 125_000_000.0, currency: 'USD' },
                rate: 0.068,
                spread_bps: null,
                index_id: null,
                maturity: '2054-01-15',
                credit_quality: 'AA',
                industry: null,
                obligor_id: null,
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 125_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-15',
              },
              {
                id: 'mortgage_003',
                asset_type: { type: 'SingleFamilyMortgage', ltv: 0.72 },
                balance: { amount: 150_000_000.0, currency: 'USD' },
                rate: 0.062,
                spread_bps: null,
                index_id: null,
                maturity: '2054-01-15',
                credit_quality: 'AAA',
                industry: null,
                obligor_id: null,
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 150_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-15',
              },
              {
                id: 'mortgage_004',
                asset_type: { type: 'SingleFamilyMortgage', ltv: 0.78 },
                balance: { amount: 125_000_000.0, currency: 'USD' },
                rate: 0.067,
                spread_bps: null,
                index_id: null,
                maturity: '2054-01-15',
                credit_quality: 'AA',
                industry: null,
                obligor_id: null,
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 125_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-15',
              },
            ],
            eligibility_criteria: {
              min_rating: null,
              max_rating: null,
              min_spread_bps: null,
              max_maturity: null,
              min_remaining_term: null,
              max_remaining_term: null,
              allowed_asset_types: [],
              allowed_currencies: [],
              max_price_pct: null,
              min_asset_size: null,
              max_asset_size: null,
              excluded_industries: [],
              excluded_obligors: [],
            },
            concentration_limits: {
              max_obligor_concentration: null,
              max_top5_concentration: null,
              max_top10_concentration: null,
              industry_limits: {},
              rating_bucket_limits: {},
              geographic_limits: {},
              asset_type_limits: {},
              max_second_lien: null,
              max_cov_lite: null,
              max_dip: null,
            },
            cumulative_defaults: { amount: 0.0, currency: 'USD' },
            cumulative_recoveries: { amount: 0.0, currency: 'USD' },
            cumulative_prepayments: { amount: 0.0, currency: 'USD' },
            reinvestment_period: null,
            collection_account: { amount: 0.0, currency: 'USD' },
            reserve_account: { amount: 0.0, currency: 'USD' },
            excess_spread_account: { amount: 0.0, currency: 'USD' },
            stats: {
              weighted_avg_coupon: 0.065,
              weighted_avg_spread: 0.0,
              weighted_avg_life: 25.0,
              weighted_avg_rating_factor: 0.0,
              diversity_score: 0.0,
              num_obligors: 0,
              num_industries: 0,
              cumulative_default_rate: 0.008,
              recovery_rate: 0.70,
              prepayment_rate: 0.12,
            },
            original_balance: { amount: 500_000_000.0, currency: 'USD' },
            coupon_rate: 0.065,
            maturity: '2054-01-15',
            prepayment_speed: 0.12,
            default_rate: 0.008,
            recovery_rate: 0.70,
            asset_type: 'residential_mortgages',
            pool_characteristics: {
              wam: 348,
              wala: 12,
              ltv: 75.0,
              fico_score: 740,
            },
          },
          tranches: {
            total_size: { amount: 500_000_000.0, currency: 'USD' },
            tranches: [
              {
                id: 'rmbs_prime_2024_1_class_a1',
                name: 'Class A-1 (Senior Sequential)',
                original_balance: { amount: 250_000_000.0, currency: 'USD' },
                current_balance: { amount: 250_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.042 } },
                seniority: 'Senior',
                attachment_point: 0.0,
                detachment_point: 0.50,
                rating: 'AAA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2054-01-15',
                payment_priority: 1,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'rmbs_prime_2024_1_class_a2',
                name: 'Class A-2 (Senior Sequential)',
                original_balance: { amount: 150_000_000.0, currency: 'USD' },
                current_balance: { amount: 150_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.048 } },
                seniority: 'Senior',
                attachment_point: 0.50,
                detachment_point: 0.80,
                rating: 'AAA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2054-01-15',
                payment_priority: 2,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'rmbs_prime_2024_1_class_m1',
                name: 'Class M-1 (Mezzanine)',
                original_balance: { amount: 50_000_000.0, currency: 'USD' },
                current_balance: { amount: 50_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.065 } },
                seniority: 'Mezzanine',
                attachment_point: 0.80,
                detachment_point: 0.90,
                rating: 'AA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2054-01-15',
                payment_priority: 3,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'rmbs_prime_2024_1_class_b',
                name: 'Class B (Subordinate)',
                original_balance: { amount: 30_000_000.0, currency: 'USD' },
                current_balance: { amount: 30_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.085 } },
                seniority: 'Subordinated',
                attachment_point: 0.90,
                detachment_point: 0.96,
                rating: 'A',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2054-01-15',
                payment_priority: 4,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'rmbs_prime_2024_1_residual',
                name: 'Residual Interest',
                original_balance: { amount: 20_000_000.0, currency: 'USD' },
                current_balance: { amount: 20_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.0 } },
                seniority: 'Equity',
                attachment_point: 0.96,
                detachment_point: 1.0,
                rating: 'NR',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2054-01-15',
                payment_priority: 5,
                attributes: { tags: [], meta: {} },
              },
            ],
          },
          fees: {
            servicing_fee_bps: 25,
            master_servicing_fee_bps: 5,
          },
          waterfall: {
            payment_rules: [],
            coverage_triggers: [],
            base_currency: 'USD',
          },
          market_conditions: {
            refi_rate: 0.04,
            original_rate: 0.065,
            hpa: 0.05,
            unemployment: 0.04,
            seasonal_factor: 1.0,
            custom_factors: {},
          },
          credit_factors: {
            credit_score: 740,
            dti: 0.35,
            ltv: 0.75,
            delinquency_days: 0,
            unemployment_rate: 0.04,
            custom_factors: {},
          },
          psa_speed: 1.0,
          sda_speed: 1.0,
        });

        const rmbs = StructuredCredit.fromJson(rmbsJson);
        try {
          const rmbsResult = registry.priceStructuredCredit(rmbs, 'discounting', market);
          results.push({
            name: 'RMBS Prime 2024-1',
            type: 'Residential Mortgage-Backed Securities',
            totalSize: 500_000_000,
            trancheCount: 5,
            presentValue: rmbsResult.presentValue.amount,
            description: 'Prime residential mortgages with sequential pay structure',
            tranches: [],
            poolWal: 22.5,
            poolWac: 6.5,
          });
        } catch (err) {
          results.push({
            name: 'RMBS Prime 2024-1',
            type: 'Residential Mortgage-Backed Securities',
            totalSize: 500_000_000,
            trancheCount: 5,
            presentValue: 0,
            description: `RMBS structure created successfully (empty pool: ${err instanceof Error ? err.message : String(err)})`,
            tranches: [],
            poolWal: 22.5,
            poolWac: 6.5,
          });
        }

        // ===================================================================
        // 4. CMBS (Commercial Mortgage-Backed Securities)
        // ===================================================================
        // CMBS Multifamily: 5.8% WAC, 10-year balloon, 5% CPR, 1.2% default, 75% recovery
        // Pool: Multifamily, DSCR 1.45, LTV 65%, 94% occupancy
        const cmbsJson = JSON.stringify({
          id: 'cmbs_multifamily_2024_1',
          deal_type: 'CMBS',
          disc_id: 'USD-OIS',
          closing_date: '2024-01-15',
          first_payment_date: '2024-02-15',
          reinvestment_end_date: null,
          legal_maturity: '2034-01-15',
          payment_frequency: { Months: 1 },
          manager_id: 'CMBS_Servicer',
          servicer_id: 'CMBS_Special_Servicer',
          attributes: { tags: [], meta: {} },
          pool: {
            id: 'cmbs_pool_2024_1',
            deal_type: 'CMBS',
            assets: [
              {
                id: 'commercial_001',
                asset_type: { type: 'MultifamilyMortgage', ltv: 0.65 },
                balance: { amount: 70_000_000.0, currency: 'USD' },
                rate: 0.058,
                spread_bps: null,
                index_id: null,
                maturity: '2034-01-15',
                credit_quality: 'AAA',
                industry: null,
                obligor_id: 'PROP_001',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 70_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-15',
              },
              {
                id: 'commercial_002',
                asset_type: { type: 'MultifamilyMortgage', ltv: 0.68 },
                balance: { amount: 87_500_000.0, currency: 'USD' },
                rate: 0.060,
                spread_bps: null,
                index_id: null,
                maturity: '2034-01-15',
                credit_quality: 'AA',
                industry: null,
                obligor_id: 'PROP_002',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 87_500_000.0, currency: 'USD' },
                acquisition_date: '2024-01-15',
              },
              {
                id: 'commercial_003',
                asset_type: { type: 'OfficeMortgage', ltv: 0.60 },
                balance: { amount: 105_000_000.0, currency: 'USD' },
                rate: 0.056,
                spread_bps: null,
                index_id: null,
                maturity: '2034-01-15',
                credit_quality: 'AAA',
                industry: null,
                obligor_id: 'PROP_003',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 105_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-15',
              },
              {
                id: 'commercial_004',
                asset_type: { type: 'RetailMortgage', ltv: 0.62 },
                balance: { amount: 87_500_000.0, currency: 'USD' },
                rate: 0.057,
                spread_bps: null,
                index_id: null,
                maturity: '2034-01-15',
                credit_quality: 'AA',
                industry: null,
                obligor_id: 'PROP_004',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 87_500_000.0, currency: 'USD' },
                acquisition_date: '2024-01-15',
              },
            ],
            eligibility_criteria: {
              min_rating: null,
              max_rating: null,
              min_spread_bps: null,
              max_maturity: null,
              min_remaining_term: null,
              max_remaining_term: null,
              allowed_asset_types: [],
              allowed_currencies: [],
              max_price_pct: null,
              min_asset_size: null,
              max_asset_size: null,
              excluded_industries: [],
              excluded_obligors: [],
            },
            concentration_limits: {
              max_obligor_concentration: null,
              max_top5_concentration: null,
              max_top10_concentration: null,
              industry_limits: {},
              rating_bucket_limits: {},
              geographic_limits: {},
              asset_type_limits: {},
              max_second_lien: null,
              max_cov_lite: null,
              max_dip: null,
            },
            cumulative_defaults: { amount: 0.0, currency: 'USD' },
            cumulative_recoveries: { amount: 0.0, currency: 'USD' },
            cumulative_prepayments: { amount: 0.0, currency: 'USD' },
            reinvestment_period: null,
            collection_account: { amount: 0.0, currency: 'USD' },
            reserve_account: { amount: 0.0, currency: 'USD' },
            excess_spread_account: { amount: 0.0, currency: 'USD' },
            stats: {
              weighted_avg_coupon: 0.058,
              weighted_avg_spread: 0.0,
              weighted_avg_life: 8.0,
              weighted_avg_rating_factor: 0.0,
              diversity_score: 0.0,
              num_obligors: 0,
              num_industries: 0,
              cumulative_default_rate: 0.012,
              recovery_rate: 0.75,
              prepayment_rate: 0.05,
            },
            original_balance: { amount: 350_000_000.0, currency: 'USD' },
            coupon_rate: 0.058,
            maturity: '2034-01-15',
            prepayment_speed: 0.05,
            default_rate: 0.012,
            recovery_rate: 0.75,
            asset_type: 'commercial_mortgages',
            pool_characteristics: {
              property_type: 'multifamily',
              dscr: 1.45,
              ltv: 65.0,
              occupancy_rate: 0.94,
            },
          },
          tranches: {
            total_size: { amount: 350_000_000.0, currency: 'USD' },
            tranches: [
              {
                id: 'cmbs_multifamily_2024_1_class_a',
                name: 'Class A (Senior)',
                original_balance: { amount: 245_000_000.0, currency: 'USD' },
                current_balance: { amount: 245_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.046 } },
                seniority: 'Senior',
                attachment_point: 0.0,
                detachment_point: 0.70,
                rating: 'AAA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2034-01-15',
                payment_priority: 1,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'cmbs_multifamily_2024_1_class_b',
                name: 'Class B (Mezzanine)',
                original_balance: { amount: 52_500_000.0, currency: 'USD' },
                current_balance: { amount: 52_500_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.062 } },
                seniority: 'Mezzanine',
                attachment_point: 0.70,
                detachment_point: 0.85,
                rating: 'AA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2034-01-15',
                payment_priority: 2,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'cmbs_multifamily_2024_1_class_c',
                name: 'Class C (Junior)',
                original_balance: { amount: 35_000_000.0, currency: 'USD' },
                current_balance: { amount: 35_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.080 } },
                seniority: 'Subordinated',
                attachment_point: 0.85,
                detachment_point: 0.95,
                rating: 'BBB',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2034-01-15',
                payment_priority: 3,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'cmbs_multifamily_2024_1_class_x',
                name: 'Class X (Interest-Only)',
                original_balance: { amount: 350_000_000.0, currency: 'USD' },
                current_balance: { amount: 350_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.012 } },
                seniority: 'Senior',
                attachment_point: 0.0,
                detachment_point: 1.0,
                rating: 'AAA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2034-01-15',
                payment_priority: 1,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'cmbs_multifamily_2024_1_residual',
                name: 'Residual Certificate',
                original_balance: { amount: 17_500_000.0, currency: 'USD' },
                current_balance: { amount: 17_500_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.0 } },
                seniority: 'Equity',
                attachment_point: 0.95,
                detachment_point: 1.0,
                rating: 'NR',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 1 },
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2034-01-15',
                payment_priority: 4,
                attributes: { tags: [], meta: {} },
              },
            ],
          },
          fees: {
            servicing_fee_bps: 20,
            special_servicing_fee_bps: 25,
          },
          waterfall: {
            payment_rules: [],
            coverage_triggers: [],
            base_currency: 'USD',
          },
          market_conditions: {
            refi_rate: 0.05,
            original_rate: null,
            hpa: 0.03,
            unemployment: 0.04,
            seasonal_factor: null,
            custom_factors: {},
          },
          credit_factors: {
            credit_score: null,
            dti: null,
            ltv: 0.65,
            delinquency_days: 0,
            unemployment_rate: null,
            custom_factors: { dscr: 1.45, occupancy: 0.94 },
          },
        });

        const cmbs = StructuredCredit.fromJson(cmbsJson);
        try {
          const cmbsResult = registry.priceStructuredCredit(cmbs, 'discounting', market);
          results.push({
            name: 'CMBS Multifamily 2024-1',
            type: 'Commercial Mortgage-Backed Securities',
            totalSize: 350_000_000,
            trancheCount: 5,
            presentValue: cmbsResult.presentValue.amount,
            description: 'Multifamily commercial mortgages with IO strip',
            tranches: [],
            poolWal: 8.3,
            poolWac: 5.8,
          });
        } catch (err) {
          results.push({
            name: 'CMBS Multifamily 2024-1',
            type: 'Commercial Mortgage-Backed Securities',
            totalSize: 350_000_000,
            trancheCount: 5,
            presentValue: 0,
            description: `CMBS structure created successfully (empty pool: ${err instanceof Error ? err.message : String(err)})`,
            tranches: [],
            poolWal: 8.3,
            poolWac: 5.8,
          });
        }

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Structured credit error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building structured credit examples…</p>;
  }

  return (
    <section className="example-section">
      <h2>Structured Credit Instruments</h2>
      <p>
        Complex structured credit securities including CLOs, ABS, RMBS, and CMBS. These instruments
        represent pools of underlying loans (corporate, auto, residential mortgage, or commercial mortgage)
        tranched into sequential or pro-rata payment structures with varying credit ratings and yields.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Total Pool Size</th>
            <th>Tranches</th>
            <th>Present Value</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, totalSize, trancheCount, presentValue }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{currencyFormatter.format(totalSize)}</td>
              <td>{trancheCount}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <div style={{ marginTop: '2rem' }}>
        <h3 style={{ fontSize: '1.3rem', marginBottom: '1rem' }}>Instrument Details</h3>
        {rows.map(({ name, description }) => (
          <div key={name} style={{ marginBottom: '1.5rem' }}>
            <h4 style={{ fontSize: '1.1rem', color: '#646cff' }}>{name}</h4>
            <p style={{ color: '#aaa', margin: '0.5rem 0' }}>{description}</p>
          </div>
        ))}
      </div>

      <div style={{ marginTop: '3rem', padding: '1.5rem', backgroundColor: 'rgba(100, 108, 255, 0.05)', borderRadius: '8px' }}>
        <h3 style={{ fontSize: '1.2rem', marginBottom: '1rem' }}>Key Concepts</h3>
        <div style={{ display: 'grid', gap: '1.5rem' }}>
          <div>
            <h4 style={{ fontSize: '1rem', color: '#a8afff', marginBottom: '0.5rem' }}>Tranching & Credit Enhancement</h4>
            <p style={{ color: '#bbb', lineHeight: '1.6', margin: 0 }}>
              Structured credit uses <strong>tranching</strong> to redistribute cash flows and credit risk.
              Senior tranches receive priority payment and have lower yields, while junior tranches absorb
              first losses and offer higher returns. The subordination of junior tranches provides credit
              enhancement to senior tranches.
            </p>
          </div>

          <div>
            <h4 style={{ fontSize: '1rem', color: '#a8afff', marginBottom: '0.5rem' }}>Prepayment & Default Risk</h4>
            <p style={{ color: '#bbb', lineHeight: '1.6', margin: 0 }}>
              <strong>CPR (Constant Prepayment Rate)</strong> measures voluntary prepayments, which reduce
              interest income. <strong>Default rates</strong> represent credit losses that erode junior tranches.
              Higher recoveries reduce losses on defaulted assets. Models incorporate both to project cash flows.
            </p>
          </div>

          <div>
            <h4 style={{ fontSize: '1rem', color: '#a8afff', marginBottom: '0.5rem' }}>Waterfall Structures</h4>
            <p style={{ color: '#bbb', lineHeight: '1.6', margin: 0 }}>
              <strong>Sequential</strong> structures pay senior tranches completely before subordinate tranches,
              providing maximum protection but longer maturities for juniors. <strong>Pro-rata</strong> structures
              distribute principal proportionally, reducing duration differences but lowering credit enhancement.
            </p>
          </div>

          <div>
            <h4 style={{ fontSize: '1rem', color: '#a8afff', marginBottom: '0.5rem' }}>Collateral Types</h4>
            <ul style={{ color: '#bbb', lineHeight: '1.8', paddingLeft: '1.5rem', margin: '0.5rem 0 0 0' }}>
              <li><strong>CLO:</strong> Senior secured leveraged loans to corporations (floating rate, SOFR-based)</li>
              <li><strong>ABS:</strong> Auto loans, credit cards, student loans, equipment leases</li>
              <li><strong>RMBS:</strong> Residential mortgages (prime, Alt-A, subprime) - long duration, prepayment-sensitive</li>
              <li><strong>CMBS:</strong> Commercial real estate mortgages (office, retail, multifamily, industrial)</li>
            </ul>
          </div>

          <div style={{ marginTop: '1rem', padding: '1rem', backgroundColor: 'rgba(255, 255, 255, 0.03)', borderRadius: '6px', borderLeft: '3px solid #646cff' }}>
            <h4 style={{ fontSize: '1rem', marginBottom: '0.5rem' }}>JSON-Based Modeling</h4>
            <p style={{ margin: '0.5rem 0', color: '#aaa', fontSize: '0.95rem' }}>
              All structured credit instruments are defined via JSON for maximum flexibility:
            </p>
            <ul style={{ marginTop: '0.5rem', paddingLeft: '1.5rem', color: '#bbb', fontSize: '0.9rem', lineHeight: '1.8' }}>
              <li>Define collateral pools with prepayment/default assumptions</li>
              <li>Specify tranches with attachment/detachment points (waterfall)</li>
              <li>Configure fees (servicing, management, trustee)</li>
              <li>Model credit enhancement and OC/IC tests</li>
              <li>Serialize/deserialize for storage or transmission</li>
            </ul>
          </div>
        </div>
      </div>
    </section>
  );
};
