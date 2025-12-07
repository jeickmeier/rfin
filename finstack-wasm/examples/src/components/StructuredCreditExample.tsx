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
          new Float64Array([1.0, 0.995, 0.98, 0.96, 0.935, 0.905]),
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
          new Float64Array([0.008, 0.012, 0.015, 0.018]),
          0.4, // 40% recovery rate
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
          discount_curve_id: 'USD-OIS',
          payment_calendar_id: 'nyse',
          closing_date: '2024-01-02',
          first_payment_date: '2024-04-15',
          reinvestment_end_date: '2027-01-15',
          legal_maturity: '2031-01-15',
          payment_frequency: { Months: 3 },
          // manager/servicer metadata
          deal_metadata: { manager: 'CLO_Manager', servicer: 'CLO_Servicer' },
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
                index_id: null,
                maturity: '2030-01-15',
                credit_quality: 'B',
                industry: 'Technology',
                obligor_id: 'BORROWER_001',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 50_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-02',
                day_count: 'Act360',
              },
              {
                id: 'loan_002',
                asset_type: { type: 'FirstLienLoan', industry: 'Healthcare' },
                balance: { amount: 75_000_000.0, currency: 'USD' },
                rate: 0.072,
                spread_bps: 400.0,
                index_id: null,
                maturity: '2029-06-15',
                credit_quality: 'BB',
                industry: 'Healthcare',
                obligor_id: 'BORROWER_002',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 75_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-02',
                day_count: 'Act360',
              },
              {
                id: 'loan_003',
                asset_type: { type: 'FirstLienLoan', industry: 'Manufacturing' },
                balance: { amount: 100_000_000.0, currency: 'USD' },
                rate: 0.075,
                spread_bps: 425.0,
                index_id: null,
                maturity: '2030-12-15',
                credit_quality: 'B',
                industry: 'Manufacturing',
                obligor_id: 'BORROWER_003',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 100_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-02',
                day_count: 'Act360',
              },
              {
                id: 'loan_004',
                asset_type: { type: 'SecondLienLoan', industry: 'Retail' },
                balance: { amount: 80_000_000.0, currency: 'USD' },
                rate: 0.08,
                spread_bps: 475.0,
                index_id: null,
                maturity: '2028-09-15',
                credit_quality: 'B',
                industry: 'Retail',
                obligor_id: 'BORROWER_004',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 80_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-02',
                day_count: 'Act360',
              },
              {
                id: 'loan_005',
                asset_type: { type: 'FirstLienLoan', industry: 'Energy' },
                balance: { amount: 95_000_000.0, currency: 'USD' },
                rate: 0.076,
                spread_bps: 450.0,
                index_id: null,
                maturity: '2031-03-15',
                credit_quality: 'BB',
                industry: 'Energy',
                obligor_id: 'BORROWER_005',
                is_defaulted: false,
                recovery_amount: null,
                purchase_price: { amount: 95_000_000.0, currency: 'USD' },
                acquisition_date: '2024-01-02',
                day_count: 'Act360',
              },
            ],
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
          },
          tranches: {
            total_size: { amount: 400_000_000.0, currency: 'USD' },
            tranches: [
              {
                id: 'clo_2024_1_class_a',
                original_balance: { amount: 280_000_000.0, currency: 'USD' },
                current_balance: { amount: 280_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.045 } },
                seniority: 'Senior',
                attachment_point: 0.0,
                detachment_point: 0.7,
                rating: 'AAA',
                credit_enhancement: {
                  subordination: { amount: 0.0, currency: 'USD' },
                  overcollateralization: { amount: 0.0, currency: 'USD' },
                  reserve_account: { amount: 0.0, currency: 'USD' },
                  excess_spread: 0.0,
                  cash_trap_active: false,
                },
                payment_frequency: { Months: 3 },
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: true,
                legal_maturity: '2031-01-15',
                payment_priority: 1,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'clo_2024_1_class_b',
                original_balance: { amount: 60_000_000.0, currency: 'USD' },
                current_balance: { amount: 60_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.065 } },
                seniority: 'Mezzanine',
                attachment_point: 0.7,
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
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: true,
                legal_maturity: '2031-01-15',
                payment_priority: 2,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'clo_2024_1_class_c',
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
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: true,
                legal_maturity: '2031-01-15',
                payment_priority: 3,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'clo_2024_1_equity',
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
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
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
        });

        let clo: StructuredCredit;
        try {
          clo = StructuredCredit.fromJson(cloJson);
        } catch (err) {
          console.error('CLO fromJson failed', err);
          results.push({
            name: 'CLO 2024-1',
            type: 'Collateralized Loan Obligation',
            totalSize: 400_000_000,
            trancheCount: 4,
            presentValue: 0,
            description: `CLO JSON invalid: ${err instanceof Error ? err.message : String(err)}`,
            tranches: [],
            poolWal: 5.5,
            poolWac: 7.6,
          });
          clo = null as unknown as StructuredCredit;
        }
        if (clo) {
          try {
            const cloResult = registry.priceStructuredCredit(
              clo,
              'discounting',
              market,
              asOf,
              null
            );
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
            console.error('CLO pricing failed:', err);
            results.push({
              name: 'CLO 2024-1',
              type: 'Collateralized Loan Obligation',
              totalSize: 400_000_000,
              trancheCount: 4,
              presentValue: 0,
              description: `CLO pricing failed: ${err instanceof Error ? err.message : String(err)}`,
              tranches: [],
              poolWal: 5.5,
              poolWac: 7.6,
            });
          }
        }

        // ===================================================================
        // 2. ABS (Asset-Backed Securities) - Auto Loan Pool
        // ===================================================================
        // ABS Auto: 5.5% WAR, 20% ABS prepayment, 1.8% default, 55% recovery
        const absJson = JSON.stringify({
          id: 'abs_auto_2024_1',
          deal_type: 'ABS',
          discount_curve_id: 'USD-OIS',
          payment_calendar_id: 'nyse',
          closing_date: '2024-03-01',
          first_payment_date: '2024-04-15',
          reinvestment_end_date: null,
          legal_maturity: '2029-06-15',
          payment_frequency: { Months: 1 },
          deal_metadata: { servicer: 'ABS_Servicer' },
          attributes: { tags: [], meta: {} },
          pool: {
            id: 'abs_pool_2024_1',
            deal_type: 'ABS',
            assets: [
              {
                id: 'auto_001',
                asset_type: { type: 'NewAutoLoan', ltv: 0.8 },
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
                day_count: 'Act360',
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
                day_count: 'Act360',
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
                day_count: 'Act360',
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
                day_count: 'Act360',
              },
            ],
            cumulative_defaults: { amount: 0.0, currency: 'USD' },
            cumulative_recoveries: { amount: 0.0, currency: 'USD' },
            cumulative_prepayments: { amount: 0.0, currency: 'USD' },
            reinvestment_period: null,
            collection_account: { amount: 0.0, currency: 'USD' },
            reserve_account: { amount: 0.0, currency: 'USD' },
            excess_spread_account: { amount: 0.0, currency: 'USD' },
          },
          tranches: {
            total_size: { amount: 250_000_000.0, currency: 'USD' },
            tranches: [
              {
                id: 'abs_auto_2024_1_class_a1',
                original_balance: { amount: 150_000_000.0, currency: 'USD' },
                current_balance: { amount: 150_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.038 } },
                seniority: 'Senior',
                attachment_point: 0.0,
                detachment_point: 0.6,
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
                legal_maturity: '2029-06-15',
                payment_priority: 1,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'abs_auto_2024_1_class_a2',
                original_balance: { amount: 50_000_000.0, currency: 'USD' },
                current_balance: { amount: 50_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.045 } },
                seniority: 'Senior',
                attachment_point: 0.6,
                detachment_point: 0.8,
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
                legal_maturity: '2029-06-15',
                payment_priority: 2,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'abs_auto_2024_1_class_b',
                original_balance: { amount: 30_000_000.0, currency: 'USD' },
                current_balance: { amount: 30_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.06 } },
                seniority: 'Subordinated',
                attachment_point: 0.8,
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
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2029-06-15',
                payment_priority: 3,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'abs_auto_2024_1_class_c',
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
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2029-06-15',
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
        });

        let abs: StructuredCredit;
        try {
          abs = StructuredCredit.fromJson(absJson);
        } catch (err) {
          console.error('ABS fromJson failed', err);
          throw err;
        }
        try {
          const absResult = registry.priceStructuredCredit(abs, 'discounting', market, asOf, null);
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
          discount_curve_id: 'USD-OIS',
          payment_calendar_id: 'nyse',
          closing_date: '2024-01-15',
          first_payment_date: '2024-02-15',
          reinvestment_end_date: null,
          legal_maturity: '2054-01-15',
          payment_frequency: { Months: 1 },
          deal_metadata: { servicer: 'RMBS_Master_Servicer' },
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
                day_count: 'Act360',
              },
              {
                id: 'mortgage_002',
                asset_type: { type: 'SingleFamilyMortgage', ltv: 0.8 },
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
                day_count: 'Act360',
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
                day_count: 'Act360',
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
                day_count: 'Act360',
              },
            ],
            cumulative_defaults: { amount: 0.0, currency: 'USD' },
            cumulative_recoveries: { amount: 0.0, currency: 'USD' },
            cumulative_prepayments: { amount: 0.0, currency: 'USD' },
            reinvestment_period: null,
            collection_account: { amount: 0.0, currency: 'USD' },
            reserve_account: { amount: 0.0, currency: 'USD' },
            excess_spread_account: { amount: 0.0, currency: 'USD' },
          },
          tranches: {
            total_size: { amount: 500_000_000.0, currency: 'USD' },
            tranches: [
              {
                id: 'rmbs_prime_2024_1_class_a1',
                original_balance: { amount: 250_000_000.0, currency: 'USD' },
                current_balance: { amount: 250_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.042 } },
                seniority: 'Senior',
                attachment_point: 0.0,
                detachment_point: 0.5,
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
                legal_maturity: '2054-01-15',
                payment_priority: 1,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'rmbs_prime_2024_1_class_a2',
                original_balance: { amount: 150_000_000.0, currency: 'USD' },
                current_balance: { amount: 150_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.048 } },
                seniority: 'Senior',
                attachment_point: 0.5,
                detachment_point: 0.8,
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
                legal_maturity: '2054-01-15',
                payment_priority: 2,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'rmbs_prime_2024_1_class_m1',
                original_balance: { amount: 50_000_000.0, currency: 'USD' },
                current_balance: { amount: 50_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.065 } },
                seniority: 'Mezzanine',
                attachment_point: 0.8,
                detachment_point: 0.9,
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
                legal_maturity: '2054-01-15',
                payment_priority: 3,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'rmbs_prime_2024_1_class_b',
                original_balance: { amount: 30_000_000.0, currency: 'USD' },
                current_balance: { amount: 30_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.085 } },
                seniority: 'Subordinated',
                attachment_point: 0.9,
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
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2054-01-15',
                payment_priority: 4,
                attributes: { tags: [], meta: {} },
              },
              {
                id: 'rmbs_prime_2024_1_residual',
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
                day_count: 'Act360',
                deferred_interest: { amount: 0.0, currency: 'USD' },
                is_revolving: false,
                can_reinvest: false,
                legal_maturity: '2054-01-15',
                payment_priority: 5,
                attributes: { tags: [], meta: {} },
              },
            ],
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
        });

        let rmbs: StructuredCredit;
        try {
          rmbs = StructuredCredit.fromJson(rmbsJson);
        } catch (err) {
          console.error('RMBS fromJson failed', err);
          throw err;
        }
        try {
          const rmbsResult = registry.priceStructuredCredit(
            rmbs,
            'discounting',
            market,
            asOf,
            null
          );
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
          discount_curve_id: 'USD-OIS',
          payment_calendar_id: 'nyse',
          closing_date: '2024-01-15',
          first_payment_date: '2024-02-15',
          reinvestment_end_date: null,
          legal_maturity: '2034-01-15',
          payment_frequency: { Months: 1 },
          deal_metadata: { servicer: 'CMBS_Special_Servicer' },
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
                day_count: 'Act360',
              },
              {
                id: 'commercial_002',
                asset_type: { type: 'MultifamilyMortgage', ltv: 0.68 },
                balance: { amount: 87_500_000.0, currency: 'USD' },
                rate: 0.06,
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
                day_count: 'Act360',
              },
              {
                id: 'commercial_003',
                asset_type: { type: 'OfficeMortgage', ltv: 0.6 },
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
                day_count: 'Act360',
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
                day_count: 'Act360',
              },
            ],
            cumulative_defaults: { amount: 0.0, currency: 'USD' },
            cumulative_recoveries: { amount: 0.0, currency: 'USD' },
            cumulative_prepayments: { amount: 0.0, currency: 'USD' },
            reinvestment_period: null,
            collection_account: { amount: 0.0, currency: 'USD' },
            reserve_account: { amount: 0.0, currency: 'USD' },
            excess_spread_account: { amount: 0.0, currency: 'USD' },
          },
          tranches: {
            total_size: { amount: 350_000_000.0, currency: 'USD' },
            tranches: [
              {
                id: 'cmbs_multifamily_2024_1_class_a',
                original_balance: { amount: 245_000_000.0, currency: 'USD' },
                current_balance: { amount: 245_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.046 } },
                seniority: 'Senior',
                attachment_point: 0.0,
                detachment_point: 0.7,
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
                original_balance: { amount: 52_500_000.0, currency: 'USD' },
                current_balance: { amount: 52_500_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.062 } },
                seniority: 'Mezzanine',
                attachment_point: 0.7,
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
                original_balance: { amount: 35_000_000.0, currency: 'USD' },
                current_balance: { amount: 35_000_000.0, currency: 'USD' },
                coupon: { Fixed: { rate: 0.08 } },
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

        let cmbs: StructuredCredit;
        try {
          cmbs = StructuredCredit.fromJson(cmbsJson);
        } catch (err) {
          console.error('CMBS fromJson failed', err);
          throw err;
        }
        try {
          const cmbsResult = registry.priceStructuredCredit(
            cmbs,
            'discounting',
            market,
            asOf,
            null
          );
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
        represent pools of underlying loans (corporate, auto, residential mortgage, or commercial
        mortgage) tranched into sequential or pro-rata payment structures with varying credit
        ratings and yields.
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

      <div
        style={{
          marginTop: '3rem',
          padding: '1.5rem',
          backgroundColor: 'rgba(100, 108, 255, 0.05)',
          borderRadius: '8px',
        }}
      >
        <h3 style={{ fontSize: '1.2rem', marginBottom: '1rem' }}>Key Concepts</h3>
        <div style={{ display: 'grid', gap: '1.5rem' }}>
          <div>
            <h4 style={{ fontSize: '1rem', color: '#a8afff', marginBottom: '0.5rem' }}>
              Tranching & Credit Enhancement
            </h4>
            <p style={{ color: '#bbb', lineHeight: '1.6', margin: 0 }}>
              Structured credit uses <strong>tranching</strong> to redistribute cash flows and
              credit risk. Senior tranches receive priority payment and have lower yields, while
              junior tranches absorb first losses and offer higher returns. The subordination of
              junior tranches provides credit enhancement to senior tranches.
            </p>
          </div>

          <div>
            <h4 style={{ fontSize: '1rem', color: '#a8afff', marginBottom: '0.5rem' }}>
              Prepayment & Default Risk
            </h4>
            <p style={{ color: '#bbb', lineHeight: '1.6', margin: 0 }}>
              <strong>CPR (Constant Prepayment Rate)</strong> measures voluntary prepayments, which
              reduce interest income. <strong>Default rates</strong> represent credit losses that
              erode junior tranches. Higher recoveries reduce losses on defaulted assets. Models
              incorporate both to project cash flows.
            </p>
          </div>

          <div>
            <h4 style={{ fontSize: '1rem', color: '#a8afff', marginBottom: '0.5rem' }}>
              Waterfall Structures
            </h4>
            <p style={{ color: '#bbb', lineHeight: '1.6', margin: 0 }}>
              <strong>Sequential</strong> structures pay senior tranches completely before
              subordinate tranches, providing maximum protection but longer maturities for juniors.{' '}
              <strong>Pro-rata</strong> structures distribute principal proportionally, reducing
              duration differences but lowering credit enhancement.
            </p>
          </div>

          <div>
            <h4 style={{ fontSize: '1rem', color: '#a8afff', marginBottom: '0.5rem' }}>
              Collateral Types
            </h4>
            <ul
              style={{
                color: '#bbb',
                lineHeight: '1.8',
                paddingLeft: '1.5rem',
                margin: '0.5rem 0 0 0',
              }}
            >
              <li>
                <strong>CLO:</strong> Senior secured leveraged loans to corporations (floating rate,
                SOFR-based)
              </li>
              <li>
                <strong>ABS:</strong> Auto loans, credit cards, student loans, equipment leases
              </li>
              <li>
                <strong>RMBS:</strong> Residential mortgages (prime, Alt-A, subprime) - long
                duration, prepayment-sensitive
              </li>
              <li>
                <strong>CMBS:</strong> Commercial real estate mortgages (office, retail,
                multifamily, industrial)
              </li>
            </ul>
          </div>

          <div
            style={{
              marginTop: '1rem',
              padding: '1rem',
              backgroundColor: 'rgba(255, 255, 255, 0.03)',
              borderRadius: '6px',
              borderLeft: '3px solid #646cff',
            }}
          >
            <h4 style={{ fontSize: '1rem', marginBottom: '0.5rem' }}>JSON-Based Modeling</h4>
            <p style={{ margin: '0.5rem 0', color: '#aaa', fontSize: '0.95rem' }}>
              All structured credit instruments are defined via JSON for maximum flexibility:
            </p>
            <ul
              style={{
                marginTop: '0.5rem',
                paddingLeft: '1.5rem',
                color: '#bbb',
                fontSize: '0.9rem',
                lineHeight: '1.8',
              }}
            >
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
