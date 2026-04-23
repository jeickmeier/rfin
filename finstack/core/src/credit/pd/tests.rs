//! Tests for PD calibration, term structure, and master scale.

#[cfg(test)]
mod calibration_tests {
    use crate::credit::pd::{pit_to_ttc, ttc_to_pit, PdCalibrationError, PdCycleParams};

    /// PiT/TtC round-trip: converting TtC -> PiT -> TtC should recover the original.
    #[test]
    fn round_trip_consistency() {
        let params = PdCycleParams {
            asset_correlation: 0.20,
            cycle_index: -1.5,
        };
        let pd_ttc = 0.02;
        let pd_pit = ttc_to_pit(pd_ttc, &params).unwrap();
        let recovered = pit_to_ttc(pd_pit, &params).unwrap();
        assert!(
            (recovered - pd_ttc).abs() < 1e-10,
            "Round-trip failed: original={}, recovered={}",
            pd_ttc,
            recovered
        );
    }

    /// z = 0 with round-trip: ttc -> pit -> ttc recovers original at z=0.
    ///
    /// Note: z=0 does NOT imply PiT == TtC in the single-factor model
    /// (that only holds when rho=0). But the round-trip property holds
    /// for any z value.
    #[test]
    fn neutral_cycle_round_trip() {
        let params = PdCycleParams {
            asset_correlation: 0.20,
            cycle_index: 0.0,
        };
        let pd_ttc = 0.03;
        let pd_pit = ttc_to_pit(pd_ttc, &params).unwrap();
        let recovered = pit_to_ttc(pd_pit, &params).unwrap();
        assert!(
            (recovered - pd_ttc).abs() < 1e-10,
            "z=0 round-trip failed: original={}, recovered={}",
            pd_ttc,
            recovered
        );
    }

    /// z < 0 (downturn) => PiT > TtC.
    #[test]
    fn downturn_increases_pd() {
        let params = PdCycleParams {
            asset_correlation: 0.20,
            cycle_index: -2.0,
        };
        let pd_ttc = 0.02;
        let pd_pit = ttc_to_pit(pd_ttc, &params).unwrap();
        assert!(
            pd_pit > pd_ttc,
            "Downturn should increase PD: pit={}, ttc={}",
            pd_pit,
            pd_ttc
        );
    }

    /// z > 0 (benign) => PiT < TtC.
    #[test]
    fn benign_decreases_pd() {
        let params = PdCycleParams {
            asset_correlation: 0.20,
            cycle_index: 1.5,
        };
        let pd_ttc = 0.05;
        let pd_pit = ttc_to_pit(pd_ttc, &params).unwrap();
        assert!(
            pd_pit < pd_ttc,
            "Benign conditions should decrease PD: pit={}, ttc={}",
            pd_pit,
            pd_ttc
        );
    }

    /// PD output is always in (0, 1).
    #[test]
    fn output_in_valid_range() {
        let params = PdCycleParams {
            asset_correlation: 0.15,
            cycle_index: -3.0,
        };
        let pd_pit = ttc_to_pit(0.01, &params).unwrap();
        assert!(pd_pit > 0.0 && pd_pit < 1.0, "pd_pit={}", pd_pit);

        let pd_ttc = pit_to_ttc(0.99, &params).unwrap();
        assert!(pd_ttc > 0.0 && pd_ttc < 1.0, "pd_ttc={}", pd_ttc);
    }

    /// Multiple correlation values and round-trips.
    #[test]
    fn various_correlations() {
        for &rho in &[0.05, 0.12, 0.20, 0.24, 0.50, 0.90] {
            let params = PdCycleParams {
                asset_correlation: rho,
                cycle_index: -1.0,
            };
            let pd = 0.05;
            let pit = ttc_to_pit(pd, &params).unwrap();
            let recovered = pit_to_ttc(pit, &params).unwrap();
            assert!(
                (recovered - pd).abs() < 1e-8,
                "rho={}: original={}, recovered={}",
                rho,
                pd,
                recovered
            );
        }
    }

    /// Reject PD outside (0, 1).
    #[test]
    fn reject_invalid_pd() {
        let params = PdCycleParams {
            asset_correlation: 0.20,
            cycle_index: 0.0,
        };
        assert!(matches!(
            ttc_to_pit(0.0, &params),
            Err(PdCalibrationError::PdOutOfRange { .. })
        ));
        assert!(matches!(
            ttc_to_pit(1.0, &params),
            Err(PdCalibrationError::PdOutOfRange { .. })
        ));
        assert!(matches!(
            ttc_to_pit(-0.5, &params),
            Err(PdCalibrationError::PdOutOfRange { .. })
        ));
        assert!(matches!(
            pit_to_ttc(1.5, &params),
            Err(PdCalibrationError::PdOutOfRange { .. })
        ));
    }

    /// Reject correlation outside (0, 1).
    #[test]
    fn reject_invalid_correlation() {
        let bad_params = PdCycleParams {
            asset_correlation: 0.0,
            cycle_index: 0.0,
        };
        assert!(matches!(
            ttc_to_pit(0.05, &bad_params),
            Err(PdCalibrationError::InvalidCorrelation { .. })
        ));

        let bad_params2 = PdCycleParams {
            asset_correlation: 1.0,
            cycle_index: 0.0,
        };
        assert!(matches!(
            ttc_to_pit(0.05, &bad_params2),
            Err(PdCalibrationError::InvalidCorrelation { .. })
        ));
    }
}

#[cfg(test)]
mod central_tendency_tests {
    use crate::credit::pd::{central_tendency, PdCalibrationError};

    #[test]
    fn single_year() {
        let result = central_tendency(&[0.03]).unwrap();
        assert!((result - 0.03).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean() {
        // Geometric mean of [0.01, 0.04] = sqrt(0.01 * 0.04) = sqrt(0.0004) = 0.02
        let result = central_tendency(&[0.01, 0.04]).unwrap();
        assert!(
            (result - 0.02).abs() < 1e-10,
            "expected=0.02, got={}",
            result
        );
    }

    #[test]
    fn zero_rate_is_rejected() {
        assert!(matches!(
            central_tendency(&[0.0, 0.0, 0.0]),
            Err(PdCalibrationError::ZeroAnnualDefaultRate)
        ));
    }

    #[test]
    fn empty_input() {
        assert!(matches!(
            central_tendency(&[]),
            Err(PdCalibrationError::EmptyInput)
        ));
    }

    #[test]
    fn out_of_range() {
        assert!(matches!(
            central_tendency(&[0.5, 1.5]),
            Err(PdCalibrationError::ValueOutOfRange { .. })
        ));
        assert!(matches!(
            central_tendency(&[-0.1, 0.5]),
            Err(PdCalibrationError::ValueOutOfRange { .. })
        ));
    }
}

#[cfg(test)]
mod term_structure_tests {
    use crate::credit::pd::{PdCalibrationError, PdTermStructureBuilder};

    #[test]
    fn basic_construction_and_interpolation() {
        let ts = PdTermStructureBuilder::new()
            .with_cumulative_pds(&[(1.0, 0.002), (3.0, 0.008), (5.0, 0.018)])
            .build()
            .unwrap();

        // At grid points
        assert!((ts.cumulative_pd(1.0) - 0.002).abs() < 1e-10);
        assert!((ts.cumulative_pd(3.0) - 0.008).abs() < 1e-10);
        assert!((ts.cumulative_pd(5.0) - 0.018).abs() < 1e-10);

        // Interpolated: should be between neighbors
        let pd_2y = ts.cumulative_pd(2.0);
        assert!(pd_2y > 0.002 && pd_2y < 0.008, "pd_2y={}", pd_2y);

        let pd_4y = ts.cumulative_pd(4.0);
        assert!(pd_4y > 0.008 && pd_4y < 0.018, "pd_4y={}", pd_4y);
    }

    #[test]
    fn monotonicity_at_t_zero() {
        let ts = PdTermStructureBuilder::new()
            .with_cumulative_pds(&[(1.0, 0.01)])
            .build()
            .unwrap();

        assert_eq!(ts.cumulative_pd(0.0), 0.0);
        assert!(ts.cumulative_pd(0.5) > 0.0);
        assert!(ts.cumulative_pd(0.5) < 0.01);
    }

    #[test]
    fn extrapolation_beyond_last_tenor() {
        let ts = PdTermStructureBuilder::new()
            .with_cumulative_pds(&[(1.0, 0.01), (5.0, 0.05)])
            .build()
            .unwrap();

        let pd_10 = ts.cumulative_pd(10.0);
        assert!(pd_10 > 0.05, "pd_10={}", pd_10);
        assert!(pd_10 < 1.0, "pd_10={}", pd_10);
    }

    #[test]
    fn marginal_pd() {
        let ts = PdTermStructureBuilder::new()
            .with_cumulative_pds(&[(1.0, 0.01), (2.0, 0.025), (5.0, 0.06)])
            .build()
            .unwrap();

        let marginal = ts.marginal_pd(1.0, 2.0);
        // S(1) = 0.99, S(2) = 0.975 => marginal = (0.99-0.975)/0.99 ~ 0.01515
        assert!(marginal > 0.0, "marginal={}", marginal);
        assert!(marginal < 1.0, "marginal={}", marginal);
    }

    #[test]
    fn hazard_rate_positive() {
        let ts = PdTermStructureBuilder::new()
            .with_cumulative_pds(&[(1.0, 0.01), (5.0, 0.05)])
            .build()
            .unwrap();

        assert!(ts.hazard_rate(0.5) > 0.0);
        assert!(ts.hazard_rate(3.0) > 0.0);
        assert!(ts.hazard_rate(7.0) > 0.0);
    }

    #[test]
    fn monotonicity_enforcement() {
        // Provide non-monotonic data; builder should fix it
        let ts = PdTermStructureBuilder::new()
            .with_cumulative_pds(&[(1.0, 0.05), (2.0, 0.03), (3.0, 0.08)])
            .build()
            .unwrap();

        let pds = ts.cumulative_pds();
        for i in 1..pds.len() {
            assert!(
                pds[i] >= pds[i - 1],
                "Non-monotonic: pds[{}]={} < pds[{}]={}",
                i,
                pds[i],
                i - 1,
                pds[i - 1]
            );
        }
    }

    #[test]
    fn empty_builder_fails() {
        assert!(matches!(
            PdTermStructureBuilder::new().build(),
            Err(PdCalibrationError::EmptyTermStructure)
        ));
    }

    #[test]
    fn invalid_tenor_fails() {
        assert!(matches!(
            PdTermStructureBuilder::new()
                .with_cumulative_pds(&[(0.0, 0.01)])
                .build(),
            Err(PdCalibrationError::InvalidTenor { .. })
        ));
        assert!(matches!(
            PdTermStructureBuilder::new()
                .with_cumulative_pds(&[(-1.0, 0.01)])
                .build(),
            Err(PdCalibrationError::InvalidTenor { .. })
        ));
    }

    #[test]
    fn accessors() {
        let ts = PdTermStructureBuilder::new()
            .with_cumulative_pds(&[(1.0, 0.01), (3.0, 0.03)])
            .build()
            .unwrap();

        assert_eq!(ts.tenors(), &[1.0, 3.0]);
        assert_eq!(ts.cumulative_pds(), &[0.01, 0.03]);
    }
}

#[cfg(test)]
mod term_structure_from_matrix_tests {
    use crate::credit::migration::{RatingScale, TransitionMatrix};
    use crate::credit::pd::PdTermStructureBuilder;

    /// Simple 3-state matrix: AAA can default, BBB can default, D absorbing.
    #[test]
    fn from_transition_matrix_basic() {
        let scale =
            RatingScale::custom(vec!["AAA".to_string(), "BBB".to_string(), "D".to_string()])
                .unwrap();
        // AAA: 95% stay, 4% -> BBB, 1% -> D
        // BBB: 5% -> AAA, 90% stay, 5% -> D
        // D:   absorbing
        #[rustfmt::skip]
        let data = &[
            0.95, 0.04, 0.01,
            0.05, 0.90, 0.05,
            0.00, 0.00, 1.00,
        ];
        let tm = TransitionMatrix::new(scale, data, 1.0).unwrap();

        let ts = PdTermStructureBuilder::new()
            .from_transition_matrix(&tm, "AAA", &[1.0, 2.0, 5.0])
            .unwrap()
            .build()
            .unwrap();

        // 1-year PD for AAA should be 0.01
        assert!(
            (ts.cumulative_pd(1.0) - 0.01).abs() < 1e-10,
            "1y pd={}",
            ts.cumulative_pd(1.0)
        );
        // Multi-year PD should increase
        assert!(ts.cumulative_pd(2.0) > ts.cumulative_pd(1.0));
        assert!(ts.cumulative_pd(5.0) > ts.cumulative_pd(2.0));
    }
}

#[cfg(test)]
mod master_scale_tests {
    use crate::credit::pd::{MasterScale, MasterScaleGrade, PdCalibrationError};

    #[test]
    fn sp_empirical_mapping() {
        let scale = MasterScale::sp_empirical();
        assert_eq!(scale.n_grades(), 8);

        // AAA: PD <= 0.0001
        let aaa = scale.map_pd(0.00005);
        assert_eq!(aaa.grade, "AAA");
        assert_eq!(aaa.grade_index, 0);

        // BBB: PD <= 0.005
        let bbb = scale.map_pd(0.0015);
        assert_eq!(bbb.grade, "BBB");
        assert_eq!(bbb.grade_index, 3);

        // B: PD <= 0.07
        let b = scale.map_pd(0.05);
        assert_eq!(b.grade, "B");
        assert_eq!(b.grade_index, 5);

        // CC/C: PD > 0.25
        let ccc_plus = scale.map_pd(0.30);
        assert_eq!(ccc_plus.grade, "CC/C");
        assert_eq!(ccc_plus.grade_index, 7);
    }

    #[test]
    fn moodys_empirical_mapping() {
        let scale = MasterScale::moodys_empirical();
        assert_eq!(scale.n_grades(), 8);

        let baa = scale.map_pd(0.003);
        assert_eq!(baa.grade, "Baa");
    }

    #[test]
    fn pd_exceeds_all_grades() {
        let scale = MasterScale::sp_empirical();
        let result = scale.map_pd(1.5);
        assert_eq!(result.grade, "CC/C");
        assert_eq!(result.grade_index, 7);
    }

    #[test]
    fn pd_at_boundary() {
        let scale = MasterScale::sp_empirical();
        // Exactly at AAA upper boundary (0.0001)
        let result = scale.map_pd(0.0001);
        assert_eq!(result.grade, "AAA");

        // Just above AAA boundary
        let result = scale.map_pd(0.00011);
        assert_eq!(result.grade, "AA");
    }

    #[test]
    fn custom_scale() {
        let grades = vec![
            MasterScaleGrade {
                label: "Good".to_owned(),
                upper_pd: 0.01,
                central_pd: 0.005,
            },
            MasterScaleGrade {
                label: "Medium".to_owned(),
                upper_pd: 0.10,
                central_pd: 0.05,
            },
            MasterScaleGrade {
                label: "Bad".to_owned(),
                upper_pd: 1.0,
                central_pd: 0.50,
            },
        ];
        let scale = MasterScale::new(grades).unwrap();
        assert_eq!(scale.n_grades(), 3);
        assert_eq!(scale.map_pd(0.005).grade, "Good");
        assert_eq!(scale.map_pd(0.05).grade, "Medium");
        assert_eq!(scale.map_pd(0.80).grade, "Bad");
    }

    #[test]
    fn empty_grades_fails() {
        assert!(matches!(
            MasterScale::new(vec![]),
            Err(PdCalibrationError::EmptyInput)
        ));
    }

    #[test]
    fn unsorted_grades_fails() {
        let grades = vec![
            MasterScaleGrade {
                label: "B".to_owned(),
                upper_pd: 0.10,
                central_pd: 0.05,
            },
            MasterScaleGrade {
                label: "A".to_owned(),
                upper_pd: 0.01,
                central_pd: 0.005,
            },
        ];
        assert!(matches!(
            MasterScale::new(grades),
            Err(PdCalibrationError::GradesNotSorted)
        ));
    }

    #[test]
    fn map_score_uses_implied_pd() {
        use crate::credit::scoring::{altman_z_score, AltmanZScoreInput};

        let input = AltmanZScoreInput {
            working_capital_to_total_assets: 0.10,
            retained_earnings_to_total_assets: 0.20,
            ebit_to_total_assets: 0.15,
            market_equity_to_total_liabilities: 1.50,
            sales_to_total_assets: 1.80,
        };
        let scoring_result = altman_z_score(&input).unwrap();
        let scale = MasterScale::sp_empirical();
        let mapped = scale.map_score(&scoring_result);
        // The implied PD from a safe Z-score (Z~3.595) maps based on the
        // empirical PD mapping. Verify that map_score actually uses implied_pd.
        assert_eq!(mapped.input_pd, scoring_result.implied_pd);
        // Safe zone has low PD, should not be in the worst grades
        assert!(
            mapped.grade_index < scale.n_grades() - 1,
            "grade={}",
            mapped.grade
        );
    }

    #[test]
    fn grades_accessor() {
        let scale = MasterScale::sp_empirical();
        let grades = scale.grades();
        assert_eq!(grades.len(), 8);
        assert_eq!(grades[0].label, "AAA");
        assert_eq!(grades[7].label, "CC/C");
    }
}
