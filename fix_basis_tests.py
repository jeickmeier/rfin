import re

# Only add .discount_curve_id where it's missing, don't change anything else
files = [
    'finstack/valuations/tests/instruments/basis_swap/test_basis_swap_edge_cases.rs',
    'finstack/valuations/tests/instruments/basis_swap/test_basis_swap_sensitivities.rs',
    'finstack/valuations/tests/instruments/basis_swap/test_basis_swap_par_spread.rs',
    'finstack/valuations/tests/instruments/basis_swap/test_basis_swap_theta.rs',
    'finstack/valuations/tests/instruments/basis_swap/test_basis_swap_metrics.rs'
]

for file_path in files:
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Find builders that don't have discount_curve_id
    pattern = r'(BasisSwap::builder\(\)[^;]+?)\.build\(\)'
    
    def add_discount_curve(match):
        builder = match.group(1)
        if '.discount_curve_id' not in builder:
            # Add discount_curve_id before build
            return builder + '\n        .discount_curve_id(CurveId::new("USD-OIS"))\n        .build()'
        return match.group(0)
    
    new_content = re.sub(pattern, add_discount_curve, content, flags=re.DOTALL)
    
    with open(file_path, 'w') as f:
        f.write(new_content)
    
    print(f"Fixed {file_path}")
