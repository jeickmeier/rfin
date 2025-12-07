from finstack.statements import ModelBuilder, Evaluator
from finstack import dates
PeriodId = dates.PeriodId
from finstack.statements.adjustments import (
    Adjustment,
    NormalizationConfig,
    NormalizationEngine,
)

def main():
    print("Setting up model...")
    # 1. Create a simple model with Revenue and EBITDA
    builder = ModelBuilder.new("private_credit_deal")
    builder.periods("2025Q1..Q4", None)
    
    # Revenue: 1000, 1100, 1200, 1300
    builder.value_scalar("Revenue", {
        PeriodId.quarter(2025, 1): 1000.0,
        PeriodId.quarter(2025, 2): 1100.0,
        PeriodId.quarter(2025, 3): 1200.0,
        PeriodId.quarter(2025, 4): 1300.0,
    })
    
    # EBITDA: 20% margin (200, 220, 240, 260)
    builder.compute("EBITDA", "Revenue * 0.2")
    
    model = builder.build()
    
    # 2. Evaluate the model
    print("Evaluating base model...")
    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)
    
    # 3. Define Adjustments
    print("Defining adjustments...")
    
    # Adjustment 1: Owner's Compensation (Fixed add-back)
    owners_comp = Adjustment.fixed(
        "owners_comp",
        "Owner's Compensation",
        {
            "2025Q1": 50.0,
            "2025Q2": 50.0,
            "2025Q3": 50.0,
            "2025Q4": 50.0,
        }
    )
    
    # Adjustment 2: Synergies (Capped at 20% of EBITDA)
    # Raw synergies: 100 per quarter
    # EBITDA Q1=200 -> Cap=40. Adjusted=40.
    synergies = Adjustment.fixed(
        "synergies",
        "Synergies",
        {
            "2025Q1": 100.0,
            "2025Q2": 100.0,
            "2025Q3": 100.0,
            "2025Q4": 100.0,
        }
    ).with_cap("EBITDA", 0.20)
    
    # Adjustment 3: One-time Legal Fees (Percentage of Revenue, e.g. 1%)
    legal_fees = Adjustment.percentage(
        "legal",
        "Legal Fees",
        "Revenue",
        0.01
    )
    
    # 4. Configure Normalization
    config = NormalizationConfig("EBITDA")
    config.add_adjustment(owners_comp)
    config.add_adjustment(synergies)
    config.add_adjustment(legal_fees)
    
    # 5. Run Normalization
    print("Running normalization...")
    normalization_results = NormalizationEngine.normalize(results, config)
    
    # 6. Print Audit Trail
    print("\n--- EBITDA Normalization Audit Trail ---")
    for res in normalization_results:
        print(f"\nPeriod: {res.period}")
        print(f"  Unadjusted EBITDA: {res.base_value:.2f}")
        
        for adj in res.adjustments:
            cap_info = f" (Capped from {adj.raw_amount:.2f})" if adj.is_capped else ""
            print(f"  + {adj.name}: {adj.capped_amount:.2f}{cap_info}")
            
        print(f"  = Adjusted EBITDA: {res.final_value:.2f}")

    # 7. Merge back into Results
    print("\nMerging Adjusted EBITDA back into results...")
    NormalizationEngine.merge_into_results(results, normalization_results, "Adjusted EBITDA")
    
    # Verify it's there
    period_q1 = PeriodId.quarter(2025, 1)
    adj_ebitda = results.get("Adjusted EBITDA", period_q1)
    print(f"Adjusted EBITDA in Results (2025Q1): {adj_ebitda}")

if __name__ == "__main__":
    main()
