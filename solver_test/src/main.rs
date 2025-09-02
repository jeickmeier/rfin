use ndarray::{Array1, Array2};
use std::f64;

type F = f64;

#[derive(Debug)]
enum TestError {
    Internal,
}

type Result<T> = std::result::Result<T, TestError>;

#[derive(Clone, Debug)]
pub struct OptimizationResult {
    pub solution: Vec<F>,
    pub objective_value: F,
    pub iterations: usize,
    pub converged: bool,
    pub gradient_norm: F,
}

pub trait LeastSquaresSolver: Send + Sync {
    fn solve_least_squares<ResFunc, JacFunc>(
        &self,
        residuals: ResFunc,
        jacobian: Option<JacFunc>,
        initial_guess: &[F],
        bounds: Option<&[(F, F)]>,
    ) -> Result<OptimizationResult>
    where
        ResFunc: Fn(&[F]) -> Vec<F>,
        JacFunc: Fn(&[F]) -> Array2<F>;
}

#[derive(Clone, Debug)]
pub struct LevenbergMarquardtSolver {
    pub tolerance: F,
    pub max_iterations: usize,
    pub initial_lambda: F,
    pub lambda_factor: F,
}

impl Default for LevenbergMarquardtSolver {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            max_iterations: 100,
            initial_lambda: 0.001,
            lambda_factor: 10.0,
        }
    }
}

impl LevenbergMarquardtSolver {
    pub fn new() -> Self {
        Self::default()
    }

    fn compute_finite_diff_jacobian<ResFunc>(&self, residuals: &ResFunc, x: &[F]) -> Array2<F>
    where
        ResFunc: Fn(&[F]) -> Vec<F>,
    {
        let r_baseline = residuals(x);
        let m = r_baseline.len();
        let n = x.len();
        
        let mut jacobian = Array2::zeros((m, n));
        let h = 1e-8;

        for j in 0..n {
            let mut x_plus = x.to_vec();
            x_plus[j] += h;
            let r_plus = residuals(&x_plus);

            for i in 0..m {
                jacobian[(i, j)] = (r_plus[i] - r_baseline[i]) / h;
            }
        }

        jacobian
    }

    fn solve_linear_system(&self, a: &Array2<F>, b: &Array1<F>) -> Result<Array1<F>> {
        let n = a.nrows();
        if a.ncols() != n || b.len() != n {
            return Err(TestError::Internal);
        }

        let mut aug = Array2::zeros((n, n + 1));
        for i in 0..n {
            for j in 0..n {
                aug[(i, j)] = a[(i, j)];
            }
            aug[(i, n)] = b[i];
        }

        // Forward elimination with partial pivoting
        for k in 0..n {
            let mut max_row = k;
            for i in (k + 1)..n {
                if aug[(i, k)].abs() > aug[(max_row, k)].abs() {
                    max_row = i;
                }
            }

            if max_row != k {
                for j in 0..=n {
                    let temp = aug[(k, j)];
                    aug[(k, j)] = aug[(max_row, j)];
                    aug[(max_row, j)] = temp;
                }
            }

            if aug[(k, k)].abs() < 1e-14 {
                return Err(TestError::Internal);
            }

            for i in (k + 1)..n {
                let factor = aug[(i, k)] / aug[(k, k)];
                for j in k..=n {
                    aug[(i, j)] -= factor * aug[(k, j)];
                }
            }
        }

        let mut x = Array1::zeros(n);
        for i in (0..n).rev() {
            let mut sum = aug[(i, n)];
            for j in (i + 1)..n {
                sum -= aug[(i, j)] * x[j];
            }
            x[i] = sum / aug[(i, i)];
        }

        Ok(x)
    }
}

impl LeastSquaresSolver for LevenbergMarquardtSolver {
    fn solve_least_squares<ResFunc, JacFunc>(
        &self,
        residuals: ResFunc,
        jacobian: Option<JacFunc>,
        initial_guess: &[F],
        _bounds: Option<&[(F, F)]>,
    ) -> Result<OptimizationResult>
    where
        ResFunc: Fn(&[F]) -> Vec<F>,
        JacFunc: Fn(&[F]) -> Array2<F>,
    {
        let mut x = Array1::from_vec(initial_guess.to_vec());
        let mut lambda = self.initial_lambda;
        
        let r = residuals(x.as_slice().unwrap());
        let mut obj_val: F = r.iter().map(|ri| ri * ri).sum::<F>() / 2.0;

        for iteration in 0..self.max_iterations {
            let r_vec = residuals(x.as_slice().unwrap());
            let r_array = Array1::from_vec(r_vec.clone());
            
            let jacobian_matrix = if let Some(ref jac_fn) = jacobian {
                jac_fn(x.as_slice().unwrap())
            } else {
                self.compute_finite_diff_jacobian(&residuals, x.as_slice().unwrap())
            };

            let gradient = jacobian_matrix.t().dot(&r_array);
            let grad_norm = gradient.dot(&gradient).sqrt();
            
            if grad_norm < self.tolerance {
                return Ok(OptimizationResult {
                    solution: x.to_vec(),
                    objective_value: obj_val,
                    iterations: iteration,
                    converged: true,
                    gradient_norm: grad_norm,
                });
            }

            let jtj = jacobian_matrix.t().dot(&jacobian_matrix);
            let mut damped_jtj = jtj.clone();
            
            for i in 0..damped_jtj.nrows() {
                damped_jtj[(i, i)] += lambda;
            }
            
            let step = match self.solve_linear_system(&damped_jtj, &gradient) {
                Ok(step) => step,
                Err(_) => {
                    lambda *= self.lambda_factor * self.lambda_factor;
                    continue;
                }
            };

            let x_new = &x - &step;
            let r_new = residuals(x_new.as_slice().unwrap());
            let new_obj_val: F = r_new.iter().map(|ri| ri * ri).sum::<F>() / 2.0;

            if new_obj_val < obj_val {
                x = x_new;
                obj_val = new_obj_val;
                lambda /= self.lambda_factor;
            } else {
                lambda *= self.lambda_factor;
            }
        }

        Ok(OptimizationResult {
            solution: x.to_vec(),
            objective_value: obj_val,
            iterations: self.max_iterations,
            converged: false,
            gradient_norm: F::INFINITY,
        })
    }
}

fn main() {
    println!("Testing Levenberg-Marquardt solver...");
    
    let solver = LevenbergMarquardtSolver::new();

    // Test: Fit circle to points
    let points = vec![
        (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0),
        (0.707, 0.707), (-0.707, 0.707), (-0.707, -0.707), (0.707, -0.707),
    ];

    let residuals = |params: &[F]| -> Vec<F> {
        let (a, b, r) = (params[0], params[1], params[2]);
        points
            .iter()
            .map(|(x, y)| ((x - a).powi(2) + (y - b).powi(2)).sqrt() - r)
            .collect()
    };

    let result = solver
        .solve_least_squares(
            residuals,
            None::<fn(&[F]) -> Array2<F>>,
            &[0.1, 0.1, 0.8],
            None,
        )
        .unwrap();

    println!("Converged: {}", result.converged);
    println!("Solution: {:?}", result.solution);
    println!("Objective: {}", result.objective_value);
    println!("Iterations: {}", result.iterations);
    
    let expected = [0.0, 0.0, 1.0];
    for (i, &exp) in expected.iter().enumerate() {
        let error = (result.solution[i] - exp).abs();
        println!("Parameter {}: {} (expected {}), error: {}", i, result.solution[i], exp, error);
    }
}