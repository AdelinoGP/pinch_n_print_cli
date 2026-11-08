//! Identical differential workload runs natively and inside an actual WASM engine.
use clipper_candidate as candidate;
use clipper_original as original;

fn convert(paths: &candidate::Paths64) -> original::Paths64 {
    paths
        .iter()
        .map(|p| p.iter().map(|p| original::Point64::new(p.x, p.y)).collect())
        .collect()
}

fn exact(a: candidate::Paths64, b: original::Paths64) {
    assert_eq!(
        convert(&a),
        b,
        "ordered integer contours, not normalized area"
    );
}

pub fn run() -> u32 {
    let counts = candidate::engine::topx_contract::run_checks();
    assert_eq!(counts[1], 1);
    assert_eq!(counts[2], 13);
    let mut comparisons = 0;
    for seed in 0..48_i64 {
        let shift = seed * 13;
        let mut subjects = vec![
            candidate::make_path64(&[shift, 0, 110 + shift, 0, 110 + shift, 110, shift, 110]),
            candidate::make_path64(&[
                shift - 15,
                55,
                shift + 55,
                -15,
                shift + 125,
                55,
                shift + 55,
                125,
            ]),
            candidate::make_path64(&[
                shift + 30,
                30,
                shift + 30,
                80,
                shift + 80,
                80,
                shift + 80,
                30,
            ]), // opposite-winding hole
        ];
        if seed % 2 == 1 {
            subjects.reverse();
        }
        if seed % 3 == 0 {
            subjects[0].reverse();
        }
        let clips = vec![candidate::make_path64(&[
            shift - 30,
            17,
            shift + 140,
            17,
            shift + 90,
            93,
            shift + 20,
            93,
        ])];
        for (cf, of) in [
            (candidate::FillRule::EvenOdd, original::FillRule::EvenOdd),
            (candidate::FillRule::NonZero, original::FillRule::NonZero),
            (candidate::FillRule::Positive, original::FillRule::Positive),
            (candidate::FillRule::Negative, original::FillRule::Negative),
        ] {
            for (ct, ot) in [
                (candidate::ClipType::Union, original::ClipType::Union),
                (
                    candidate::ClipType::Intersection,
                    original::ClipType::Intersection,
                ),
                (
                    candidate::ClipType::Difference,
                    original::ClipType::Difference,
                ),
                (candidate::ClipType::Xor, original::ClipType::Xor),
            ] {
                exact(
                    candidate::boolean_op_64(ct, cf, &subjects, &clips),
                    original::boolean_op_64(ot, of, &convert(&subjects), &convert(&clips)),
                );
                comparisons += 1;
            }
        }
        for delta in [-7.0, 7.0] {
            for (cj, oj) in [
                (candidate::JoinType::Miter, original::JoinType::Miter),
                (candidate::JoinType::Round, original::JoinType::Round),
                (candidate::JoinType::Square, original::JoinType::Square),
            ] {
                exact(
                    candidate::inflate_paths_64(
                        &subjects,
                        delta,
                        cj,
                        candidate::EndType::Polygon,
                        2.0,
                        0.0,
                    ),
                    original::inflate_paths_64(
                        &convert(&subjects),
                        delta,
                        oj,
                        original::EndType::Polygon,
                        2.0,
                        0.0,
                    ),
                );
                comparisons += 1;
            }
        }
    }
    assert_eq!(comparisons, 1056);
    comparisons
}

#[no_mangle]
pub extern "C" fn t57_engine_checks() -> u32 {
    run()
}

#[test]
fn native_shared_engine_cache_and_ordered_geometry_contract() {
    assert_eq!(run(), 1056);
}
