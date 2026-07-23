#![allow(missing_docs)]

#[path = "../src/comparator.rs"]
mod comparator;

#[path = "../src/align.rs"]
mod align;

#[cfg(test)]
mod canonical_spline_tests {
    use super::align::{test_fit_cubic_bspline_value, test_solve_full_pivot_qr};

    #[test]
    fn full_rank_fit_recovers_line() {
        let design = vec![
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![1.0, 2.0],
            vec![1.0, 3.0],
        ];
        let solution = test_solve_full_pivot_qr(&design, &[2.0, 5.0, 8.0, 11.0]);
        assert!((solution[0] - 2.0).abs() < 1e-12);
        assert!((solution[1] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn rank_deficient_fit_produces_zero_not_nan() {
        let design = vec![vec![1.0, 1.0], vec![2.0, 2.0], vec![3.0, 3.0]];
        let solution = test_solve_full_pivot_qr(&design, &[2.0, 4.0, 6.0]);
        assert!(solution.iter().all(|value| value.is_finite()));
        assert!(solution[1].abs() < 1e-12);
    }

    #[test]
    fn zero_weight_observation_is_ignored() {
        let design = vec![vec![1.0, 0.0], vec![1.0, 1.0], vec![0.0, 0.0]];
        let solution = test_solve_full_pivot_qr(&design, &[4.0, 7.0, 1000.0]);
        assert!((solution[0] - 4.0).abs() < 1e-12);
        assert!((solution[1] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn single_observation_fit_returns_that_observation() {
        let actual = test_fit_cubic_bspline_value(&[0.0], &[[4.5, -2.25]], &[1.0], 1, 0.0);
        assert!((actual[0] - 4.5).abs() < 1e-6);
        assert!((actual[1] + 2.25).abs() < 1e-6);
    }

    #[test]
    fn fit_cubic_bspline_recovers_straight_line() {
        let observation_points: Vec<f32> = (0..25).map(|index| index as f32 * 0.5).collect();
        let observations: Vec<[f32; 2]> = observation_points
            .iter()
            .map(|z| [0.5 * *z + 1.0, -0.2 * *z + 3.0])
            .collect();
        let weights = vec![1.0f32; observation_points.len()];
        for &z in &observation_points {
            let expected = [0.5 * z + 1.0, -0.2 * z + 3.0];
            let actual =
                test_fit_cubic_bspline_value(&observation_points, &observations, &weights, 3, z);
            assert!(
                (actual[0] - expected[0]).abs() < 0.2,
                "x at {z}: {}",
                actual[0]
            );
            assert!(
                (actual[1] - expected[1]).abs() < 0.2,
                "y at {z}: {}",
                actual[1]
            );
        }
    }

    #[test]
    fn rank_deficient_fit_with_nan_input_does_not_propagate_nan() {
        let points = [0.0, 1.0, 2.0];
        let observations = [[1.0, 2.0], [f32::NAN, 3.0], [3.0, 4.0]];
        let weights = [1.0, 1.0, 1.0];
        for parameter in points {
            let value =
                test_fit_cubic_bspline_value(&points, &observations, &weights, 2, parameter);
            assert!(value[0].is_finite() && value[1].is_finite());
        }
    }

    #[test]
    fn solver_is_deterministic() {
        let design = vec![
            vec![1.0, 0.5, 0.0],
            vec![1.0, 1.5, 1.0],
            vec![1.0, 2.5, 4.0],
            vec![1.0, 3.5, 9.0],
        ];
        let first = test_solve_full_pivot_qr(&design, &[1.0, 2.0, 5.0, 10.0]);
        let second = test_solve_full_pivot_qr(&design, &[1.0, 2.0, 5.0, 10.0]);
        assert_eq!(first, second);
    }
}
