#![allow(dead_code, missing_docs)]

#[path = "../src/align.rs"]
mod align;
#[path = "../src/comparator.rs"]
mod comparator;

use align::{align_seam_points, find_seam_string, LayerCandidates, SeamRef};
use comparator::{EnforcedBlockedSeamPoint, Perimeter, SeamCandidate, SeamComparator, SeamSetup};

fn candidate(x: f32, z: f32, layer_angle: f32) -> SeamCandidate {
    SeamCandidate {
        layer_angle,
        position: [x, 0.0, z],
        visibility: 0.0,
        overhang: 0.0,
        unsupported_dist: 0.0,
        embedded_distance: 0.0,
        local_ccw_angle: -0.5,
        central_enforcer: false,
        point_type: EnforcedBlockedSeamPoint::Neutral,
        flow_width: 0.4,
    }
}

fn layers_with_tracks(tracks: Vec<Option<Vec<f32>>>, layer_angle: f32) -> Vec<LayerCandidates> {
    tracks
        .into_iter()
        .enumerate()
        .map(|(layer_idx, tracks)| {
            let z = layer_idx as f32 * 0.2;
            let Some(tracks) = tracks else {
                return LayerCandidates {
                    candidates: Vec::new(),
                    perimeters: Vec::new(),
                };
            };

            let mut candidates = Vec::new();
            let mut perimeters = Vec::new();
            for x in tracks {
                let start_index = candidates.len();
                candidates.extend((0..4).map(|_| candidate(x, z, layer_angle)));
                perimeters.push(Perimeter {
                    start_index,
                    end_index: candidates.len(),
                    seam_index: start_index,
                    finalized: false,
                    final_seam_position: [0.0; 3],
                });
            }
            LayerCandidates {
                candidates,
                perimeters,
            }
        })
        .collect()
}

fn layers_with_perimeter_candidates(
    layers: Vec<Option<Vec<Vec<f32>>>>,
    layer_angle: f32,
) -> Vec<LayerCandidates> {
    layers
        .into_iter()
        .enumerate()
        .map(|(layer_idx, perimeters)| {
            let z = layer_idx as f32 * 0.2;
            let Some(perimeters) = perimeters else {
                return LayerCandidates {
                    candidates: Vec::new(),
                    perimeters: Vec::new(),
                };
            };

            let mut candidates = Vec::new();
            let mut perimeter_data = Vec::new();
            for perimeter_candidates in perimeters {
                let start_index = candidates.len();
                candidates.extend(
                    perimeter_candidates
                        .into_iter()
                        .map(|x| candidate(x, z, layer_angle)),
                );
                perimeter_data.push(Perimeter {
                    start_index,
                    end_index: candidates.len(),
                    seam_index: start_index,
                    finalized: false,
                    final_seam_position: [0.0; 3],
                });
            }
            LayerCandidates {
                candidates,
                perimeters: perimeter_data,
            }
        })
        .collect()
}

fn straight_layers(count: usize, layer_angle: f32) -> Vec<LayerCandidates> {
    layers_with_tracks((0..count).map(|_| Some(vec![0.0])).collect(), layer_angle)
}

fn assert_alignment_equal(a: &[LayerCandidates], b: &[LayerCandidates]) {
    assert_eq!(a.len(), b.len());
    for (layer_a, layer_b) in a.iter().zip(b) {
        assert_eq!(layer_a.perimeters.len(), layer_b.perimeters.len());
        for (perimeter_a, perimeter_b) in layer_a.perimeters.iter().zip(&layer_b.perimeters) {
            assert_eq!(perimeter_a.finalized, perimeter_b.finalized);
            assert_eq!(perimeter_a.seam_index, perimeter_b.seam_index);
            assert_eq!(
                perimeter_a.final_seam_position,
                perimeter_b.final_seam_position
            );
        }
    }
}

#[test]
fn alternative_start_retry_finds_longer_string() {
    let mut tracks = vec![Some(vec![100.0]), None];
    tracks.extend((0..18).map(|_| Some(vec![0.0])));
    let mut layers = layers_with_tracks(tracks, 0.0);
    let comparator = SeamComparator::new(SeamSetup::Aligned);

    align_seam_points(&mut layers, &comparator);

    assert!(!layers[0].perimeters[0].finalized);
    assert!(layers[2..]
        .iter()
        .all(|layer| layer.perimeters[0].finalized));
}

#[test]
fn bounded_continuity_anchor_bridges_gap() {
    let tracks = vec![
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
        None,
        None,
        None,
        Some(vec![1.5]),
        Some(vec![1.5]),
        Some(vec![1.5]),
        Some(vec![1.5]),
        Some(vec![1.5]),
    ];
    let mut layers = layers_with_tracks(tracks, 0.0);
    let comparator = SeamComparator::new(SeamSetup::Aligned);
    let string = find_seam_string(
        &layers,
        SeamRef {
            layer: 0,
            perimeter: 0,
            candidate: 0,
        },
        &comparator,
    );

    let mut seen_layers: Vec<_> = string.iter().map(|seam| seam.layer).collect();
    seen_layers.sort_unstable();
    assert_eq!(seen_layers, vec![0, 1, 2, 3, 4, 8, 9, 10, 11, 12]);

    align_seam_points(&mut layers, &comparator);

    assert!(layers[..5]
        .iter()
        .all(|layer| layer.perimeters[0].finalized));
    assert!(layers[5..8].iter().all(|layer| layer.perimeters.is_empty()));
    assert!(layers[8..]
        .iter()
        .all(|layer| layer.perimeters[0].finalized));
}

#[test]
fn alignment_is_deterministic() {
    let comparator = SeamComparator::new(SeamSetup::Aligned);
    let tracks = vec![
        Some(vec![100.0]),
        None,
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
        Some(vec![0.0]),
    ];
    let mut first = layers_with_tracks(tracks.clone(), 0.0);
    let mut second = layers_with_tracks(tracks, 0.0);

    align_seam_points(&mut first, &comparator);
    align_seam_points(&mut second, &comparator);

    assert_alignment_equal(&first, &second);
}

#[test]
fn alternative_start_retry_keeps_longest_string() {
    let perimeters = vec![
        Some(vec![vec![100.0, 100.0, 100.0, 100.0]]),
        None,
        Some(vec![vec![0.0, 0.0, 0.0, 0.0]]),
        Some(vec![vec![0.0, 0.0, 0.0, 0.0]]),
        Some(vec![vec![0.0, 0.0, 0.0, 0.0, 20.0, 20.0, 20.0, 20.0]]),
        Some(vec![vec![0.0, 0.0, 0.0, 0.0, 20.0, 20.0, 20.0, 20.0]]),
        Some(vec![vec![0.0, 0.0, 0.0, 0.0, 20.0, 20.0, 20.0, 20.0]]),
        Some(vec![vec![0.0, 0.0, 0.0, 0.0, 20.0, 20.0, 20.0, 20.0]]),
        Some(vec![vec![20.0, 20.0, 20.0, 20.0]]),
        Some(vec![vec![20.0, 20.0, 20.0, 20.0]]),
        Some(vec![vec![20.0, 20.0, 20.0, 20.0]]),
        Some(vec![vec![20.0, 20.0, 20.0, 20.0]]),
        Some(vec![vec![20.0, 20.0, 20.0, 20.0]]),
        Some(vec![vec![20.0, 20.0, 20.0, 20.0]]),
        None,
        None,
        None,
        None,
        None,
        None,
    ];
    let mut layers = layers_with_perimeter_candidates(perimeters, 0.0);
    layers[4].perimeters[0].seam_index = 4;
    let comparator = SeamComparator::new(SeamSetup::Aligned);
    let short = find_seam_string(
        &layers,
        SeamRef {
            layer: 2,
            perimeter: 0,
            candidate: 0,
        },
        &comparator,
    );
    let long = find_seam_string(
        &layers,
        SeamRef {
            layer: 4,
            perimeter: 0,
            candidate: 4,
        },
        &comparator,
    );

    assert_eq!(short.len(), 6);
    assert_eq!(long.len(), 10);
    assert!(long.len() > short.len());

    align_seam_points(&mut layers, &comparator);

    assert!(layers[4].perimeters[0].finalized);
    assert_eq!(layers[4].perimeters[0].seam_index, 4);
    assert!(!layers[2].perimeters[0].finalized);
}

fn weighted_string_length(layers: &[LayerCandidates], string: &[SeamRef]) -> f32 {
    string
        .windows(2)
        .map(|pair| {
            let previous = &layers[pair[0].layer].candidates[pair[0].candidate];
            let current = &layers[pair[1].layer].candidates[pair[1].candidate];
            let influence = if current.layer_angle > 2.0 * current.local_ccw_angle.abs() {
                -0.8
            } else {
                1.0
            };
            let dx = previous.position[0] - current.position[0];
            let dy = previous.position[1] - current.position[1];
            let dz = previous.position[2] - current.position[2];
            influence * (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .sum()
}

#[test]
fn canonical_curling_influence_uses_layer_angle() {
    let comparator = SeamComparator::new(SeamSetup::Aligned);
    let tracks: Vec<_> = (0..12)
        .map(|layer_idx| Some(vec![if layer_idx % 2 == 0 { 0.0 } else { 1.5 }]))
        .collect();
    let mut zero_angle = layers_with_tracks(tracks.clone(), 0.0);
    let mut high_angle = layers_with_tracks(tracks, 2.0);
    let zero_string = find_seam_string(
        &zero_angle,
        SeamRef {
            layer: 0,
            perimeter: 0,
            candidate: 0,
        },
        &comparator,
    );
    let high_string = find_seam_string(
        &high_angle,
        SeamRef {
            layer: 0,
            perimeter: 0,
            candidate: 0,
        },
        &comparator,
    );
    assert!(
        weighted_string_length(&high_angle, &high_string)
            < weighted_string_length(&zero_angle, &zero_string)
    );

    align_seam_points(&mut zero_angle, &comparator);
    align_seam_points(&mut high_angle, &comparator);
    assert_ne!(
        zero_angle[5].perimeters[0].final_seam_position,
        high_angle[5].perimeters[0].final_seam_position
    );
}
