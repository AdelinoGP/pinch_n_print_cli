//! Pure synthesis of `LayerCollectionIR` rows from a committed layer-event
//! stream (packet 239a, TASK-403).
//!
//! [`synthesize_anchored_rows`] is a **pure** function over an already-ordered
//! [`CommittedLayerEvent`] sequence: no I/O, no global state, no parallelism,
//! and no dependence on hash iteration order. Running it twice on equal input
//! yields byte-identical output.
//!
//! # The merge rule
//!
//! The walk mirrors canonical `GCode::collect_layers_to_print` (`GCode.cpp`).
//! Two independent indices walk object rows and anchored planes. Each
//! iteration speculatively takes one candidate from each non-exhausted side;
//! `print_z_min` is the lower of the two candidate Z values. A side is
//! *un-consumed* (put back) iff `its_z > print_z_min + EPSILON` — a strict `>`,
//! so the effective merge test is `<=`. When both sides are taken they merge
//! into ONE row; otherwise the lower side emits a solo row and the higher side
//! retries on the next iteration.
//!
//! # Invariants
//!
//! * **Epsilon.** The merge threshold is
//!   [`slicer_ir::AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`], the
//!   same constant that governs the on-grid/off-grid routing partition in
//!   `crate::layer_executor::route_of`, so the two can never disagree. No
//!   numeric epsilon appears in this module.
//! * **Units.** A declared planar Z is already canonical i64 units
//!   (1 unit = 100 nm); `LayerCollectionIR::z` is `f32` millimetres. The
//!   conversion happens once at each boundary via [`slicer_ir::mm_to_units`] /
//!   [`slicer_ir::units_to_mm`]; every comparison is in i64 unit space.
//! * **Merge direction.** On merge the anchored entities are appended into the
//!   OBJECT row; the object row's `z` and `global_layer_index` win.
//! * **Upper anchor.** A solo synthesized row adopts the index of the `Model`
//!   row that immediately FOLLOWS it in ascending Z, per
//!   `docs/adr/0059-support-families-and-anchored-entities.md` ("anchored to
//!   the upper global layer, executes in ascending Z before that layer's
//!   ordinary model event"). With no upper `Model` row it adopts the last
//!   `Model` row's index. Adjacent rows sharing an index is intended.
//! * **`ZSpanning`.** A `ZSpanning` entity never gets a row of its own: its
//!   paths go as one contiguous block into its anchor layer's ordinary `Model`
//!   row, at that layer's normal position. If that anchor row is absent from
//!   the committed stream there is no position the ADR's rule can name, so
//!   synthesis FAILS rather than dropping the paths — see
//!   [`LayerExecutionError::AnchoredGeometry`]. Silently discarding them would
//!   lose support geometry with no diagnostic anywhere in the pipeline.
//! * **Schema version.** Synthesized rows read
//!   [`slicer_ir::CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`] from the live
//!   constant; no version literal is written down here.

use slicer_ir::{AnchoredEntity, AnchoredGeometryContract, LayerCollectionIR};

use crate::layer_executor::{
    anchored_entity_to_print_entity, CommittedLayerEvent, LayerExecutionError,
};

/// Merge threshold, in canonical units, shared with the routing partition.
const MERGE_EPSILON_UNITS: i64 = AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS;

/// One distinct declared off-grid plane, with the entities that sit on it.
struct AnchoredPlane {
    /// Declared plane height in canonical units.
    z_units: i64,
    /// Entities on this plane, in deterministic `(z, local_id)` order.
    events: Vec<AnchoredEntity>,
}

/// Declared Z used for deterministic ordering of an anchored entity.
fn ordering_z(entity: &AnchoredEntity) -> i64 {
    match entity.geometry {
        AnchoredGeometryContract::Planar { z } => z,
        AnchoredGeometryContract::ZSpanning { min_z, .. } => min_z,
    }
}

/// Append `entities` to `row`'s `ordered_entities`, continuing its topo order.
fn append_entities(row: &mut LayerCollectionIR, entities: &[AnchoredEntity]) {
    for entity in entities {
        let topo_order = row.ordered_entities.len() as u32;
        row.ordered_entities
            .push(anchored_entity_to_print_entity(entity, topo_order));
    }
}

/// Build a solo synthesized row for one off-grid plane.
fn solo_row(plane: &AnchoredPlane, global_layer_index: u32) -> LayerCollectionIR {
    let mut row = LayerCollectionIR {
        schema_version: slicer_ir::CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
        global_layer_index,
        z: slicer_ir::units_to_mm(plane.z_units),
        ordered_entities: Vec::new(),
        support_entity_identities: Vec::new(),
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
        speed_profiles: Vec::new(),
    };
    append_entities(&mut row, &plane.events);
    row
}

/// Lower the committed event stream into the final ascending row sequence.
///
/// `committed` must already be in global-layer execution order (the order
/// `crate::layer_executor::execute_per_layer_with_committed_anchored_events`
/// returns). The result is the row sequence handed to the G-code emitter.
///
/// See the module docs for the merge rule and the full invariant list.
///
/// # Errors
///
/// Returns [`LayerExecutionError::AnchoredGeometry`] when a `ZSpanning` entity
/// names an `anchor_global_layer_index` that no `Model` row in `committed`
/// carries. ADR-0059 places such an entity at its anchor layer's normal
/// position; with no anchor row that position does not exist, and emitting the
/// remaining rows anyway would drop the entity's paths without a diagnostic.
pub fn synthesize_anchored_rows(
    committed: Vec<CommittedLayerEvent>,
) -> Result<Vec<LayerCollectionIR>, LayerExecutionError> {
    let mut object_rows: Vec<LayerCollectionIR> = Vec::new();
    let mut collections = Vec::new();
    for event in committed {
        match event {
            CommittedLayerEvent::Anchored(collection) => collections.push(collection),
            CommittedLayerEvent::Model(row) => object_rows.push(row),
        }
    }

    // ── Route `ZSpanning` entities into their anchor layer's ordinary row ────
    // One contiguous block per collection, at that layer's normal position.
    for collection in &collections {
        let mut spanning: Vec<AnchoredEntity> = collection
            .events
            .iter()
            .filter(|event| matches!(event.geometry, AnchoredGeometryContract::ZSpanning { .. }))
            .cloned()
            .collect();
        if spanning.is_empty() {
            continue;
        }
        spanning.sort_by_key(|event| (ordering_z(event), event.local_id));
        let anchor = collection.anchor_global_layer_index;
        let Some(row) = object_rows
            .iter_mut()
            .find(|row| row.global_layer_index == anchor)
        else {
            // No anchor row: ADR-0059's "at that layer's normal position" has
            // no referent. Fail loudly — dropping the block here would delete
            // support geometry silently, and the pipeline has no other place
            // where a Z-spanning entity could still reach G-code.
            return Err(LayerExecutionError::AnchoredGeometry {
                local_id: spanning[0].local_id,
                message: format!(
                    "z-spanning entity anchors to global layer {anchor}, \
                     which has no committed model row"
                ),
            });
        };
        append_entities(row, &spanning);
    }

    // ── Group planar entities into one plane per distinct declared Z ─────────
    let mut planes: Vec<(i64, usize, AnchoredPlane)> = Vec::new();
    for (ordinal, collection) in collections.iter().enumerate() {
        let mut planar: Vec<AnchoredEntity> = collection
            .events
            .iter()
            .filter(|event| matches!(event.geometry, AnchoredGeometryContract::Planar { .. }))
            .cloned()
            .collect();
        planar.sort_by_key(|event| (ordering_z(event), event.local_id));

        let mut grouped: Vec<AnchoredPlane> = Vec::new();
        for event in planar {
            let z_units = ordering_z(&event);
            match grouped.last_mut() {
                Some(plane) if plane.z_units == z_units => plane.events.push(event),
                _ => grouped.push(AnchoredPlane {
                    z_units,
                    events: vec![event],
                }),
            }
        }
        planes.extend(
            grouped
                .into_iter()
                .map(|plane| (plane.z_units, ordinal, plane)),
        );
    }
    // Stable sort on an explicit total key: no hash order, no float compare.
    planes.sort_by_key(|(z_units, ordinal, _)| (*z_units, *ordinal));

    // ── Coalesce equal-Z planes ACROSS collections ──────────────────────────
    // Grouping above is per collection, so two collections declaring the same
    // off-grid Z would otherwise reach the merge walk as two planes and emit
    // two adjacent rows at the same Z. A run is opened at the first plane and
    // a successor joins iff it is within `MERGE_EPSILON_UNITS` of the RUN
    // ANCHOR — never of its predecessor, which would let a chain of small
    // steps drift arbitrarily far. The run keeps the anchor's Z (the lowest,
    // the list being sorted ascending) and concatenates entities in run order.
    let mut coalesced: Vec<AnchoredPlane> = Vec::with_capacity(planes.len());
    for (z_units, _, plane) in planes {
        match coalesced.last_mut() {
            Some(run) if (z_units - run.z_units).abs() <= MERGE_EPSILON_UNITS => {
                run.events.extend(plane.events);
            }
            _ => coalesced.push(plane),
        }
    }
    let planes = coalesced;

    // ── Canonical two-index merge walk ──────────────────────────────────────
    let last_model_index = object_rows.last().map(|row| row.global_layer_index);
    let mut out: Vec<LayerCollectionIR> = Vec::with_capacity(object_rows.len() + planes.len());
    let mut object_index = 0usize;
    let mut plane_index = 0usize;

    while object_index < object_rows.len() || plane_index < planes.len() {
        let object_z = object_rows
            .get(object_index)
            .map(|row| slicer_ir::mm_to_units(row.z));
        let plane_z = planes.get(plane_index).map(|plane| plane.z_units);

        let print_z_min = match (object_z, plane_z) {
            (Some(object), Some(plane)) => object.min(plane),
            (Some(object), None) => object,
            (None, Some(plane)) => plane,
            (None, None) => unreachable!("loop condition guarantees one side remains"),
        };

        // A side is un-consumed iff its Z is strictly above the tolerance band,
        // so the effective merge test is `<=`. Ties at exactly the boundary
        // merge, and the object row supplies `z` / `global_layer_index`.
        let take_object = object_z.is_some_and(|z| z <= print_z_min + MERGE_EPSILON_UNITS);
        let take_plane = plane_z.is_some_and(|z| z <= print_z_min + MERGE_EPSILON_UNITS);

        match (take_object, take_plane) {
            (true, take_plane) => {
                let mut row = std::mem::take(&mut object_rows[object_index]);
                object_index += 1;
                if take_plane {
                    append_entities(&mut row, &planes[plane_index].events);
                    plane_index += 1;
                }
                out.push(row);
            }
            (false, true) => {
                // The plane is strictly below the next object row: it is the
                // lower side and emits a solo row, adopting the UPPER global
                // layer's index (the `Model` row that follows it).
                let upper_index = object_rows
                    .get(object_index)
                    .map(|row| row.global_layer_index)
                    .or(last_model_index)
                    .unwrap_or(0);
                out.push(solo_row(&planes[plane_index], upper_index));
                plane_index += 1;
            }
            (false, false) => unreachable!("the side holding `print_z_min` is always taken"),
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{
        AnchoredEntityProvenance, AnchoredEventRuntimeHooks, ExtrusionRole, OrderedEventCollection,
        Point3WithWidth,
    };

    const ANCHOR: u32 = 1;

    /// Synthesis that is expected to succeed. Tests asserting the error path
    /// call [`synthesize_anchored_rows`] directly.
    fn synth(committed: Vec<CommittedLayerEvent>) -> Vec<LayerCollectionIR> {
        synthesize_anchored_rows(committed).expect("row synthesis must succeed for this fixture")
    }

    fn planar_entity(local_id: u64, z_units: i64) -> AnchoredEntity {
        // exhaustive: no Default impl for AnchoredEntity; fixture pins every field
        AnchoredEntity {
            local_id,
            anchor_global_layer_index: ANCHOR,
            geometry: AnchoredGeometryContract::Planar { z: z_units },
            input_capabilities: Vec::new(),
            output_capabilities: Vec::new(),
            provenance: AnchoredEntityProvenance {
                requesting_feature: "same-z-support".to_string(),
                source_plan_entry: "same-z-support".to_string(),
            },
            path_points: vec![Point3WithWidth {
                x: 1.0,
                y: 1.0,
                z: slicer_ir::units_to_mm(z_units),
                width: 0.45,
                flow_factor: 1.0,
                ..Default::default()
            }],
            role: ExtrusionRole::SupportMaterial,
        }
    }

    fn collection(events: Vec<AnchoredEntity>) -> OrderedEventCollection {
        // exhaustive: collection fixture pins every field
        OrderedEventCollection {
            anchor_global_layer_index: ANCHOR,
            events,
            runtime_hooks: AnchoredEventRuntimeHooks::default(),
        }
    }

    fn model_row(global_layer_index: u32, z_units: i64) -> LayerCollectionIR {
        LayerCollectionIR {
            global_layer_index,
            z: slicer_ir::units_to_mm(z_units),
            ..Default::default()
        }
    }

    /// A declared plane inside the merge epsilon of an object row merges into
    /// that row: one row out, carrying the anchored entity, at the object
    /// row's own Z and index.
    #[test]
    fn merge_within_epsilon_produces_one_row() {
        let object_z = slicer_ir::mm_to_units(0.4);
        let plane_z = object_z + MERGE_EPSILON_UNITS;

        let rows = synth(vec![
            CommittedLayerEvent::Anchored(collection(vec![planar_entity(42, plane_z)])),
            CommittedLayerEvent::Model(model_row(ANCHOR, object_z)),
        ]);

        assert_eq!(
            rows.len(),
            1,
            "a plane within {MERGE_EPSILON_UNITS} units of the object row must merge into it, \
             got {} rows at {:?} units",
            rows.len(),
            rows.iter()
                .map(|row| slicer_ir::mm_to_units(row.z))
                .collect::<Vec<_>>()
        );
        assert_eq!(slicer_ir::mm_to_units(rows[0].z), object_z);
        assert_eq!(rows[0].global_layer_index, ANCHOR);
        assert_eq!(
            rows[0]
                .ordered_entities
                .iter()
                .map(|entity| entity.entity_id)
                .collect::<Vec<_>>(),
            vec![42],
            "the merged row must carry the anchored entity"
        );
        assert_eq!(
            rows[0].schema_version,
            slicer_ir::CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION
        );
    }

    /// A declared plane beyond the merge epsilon and below the object row emits
    /// its own solo row first, adopting the UPPER (following) layer's index.
    #[test]
    fn beyond_epsilon_lower_z_emits_solo_row() {
        let object_z = slicer_ir::mm_to_units(0.4);
        let plane_z = object_z - (MERGE_EPSILON_UNITS + 1);

        let rows = synth(vec![
            CommittedLayerEvent::Anchored(collection(vec![planar_entity(7, plane_z)])),
            CommittedLayerEvent::Model(model_row(ANCHOR, object_z)),
        ]);

        assert_eq!(
            rows.len(),
            2,
            "a plane {} units below the object row is beyond the {MERGE_EPSILON_UNITS}-unit \
             merge epsilon and must emit its own row",
            MERGE_EPSILON_UNITS + 1
        );
        assert_eq!(slicer_ir::mm_to_units(rows[0].z), plane_z);
        assert_eq!(
            rows[0].global_layer_index, ANCHOR,
            "the solo row adopts the UPPER global layer's index (ADR-0059)"
        );
        assert_eq!(
            rows[0]
                .ordered_entities
                .iter()
                .map(|entity| entity.entity_id)
                .collect::<Vec<_>>(),
            vec![7]
        );
        assert_eq!(slicer_ir::mm_to_units(rows[1].z), object_z);
        assert!(
            rows[1].ordered_entities.is_empty(),
            "the object row must not also carry the off-grid entity"
        );
        assert_eq!(
            rows[0].schema_version,
            slicer_ir::CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION
        );
    }

    /// A `ZSpanning` entity gets no row of its own: it lands in its anchor
    /// layer's ordinary model row.
    #[test]
    fn z_spanning_entity_lands_in_its_anchor_row() {
        let object_z = slicer_ir::mm_to_units(0.4);
        let mut entity = planar_entity(9, object_z);
        entity.geometry = AnchoredGeometryContract::ZSpanning {
            min_z: 0,
            max_z: object_z,
        };

        let rows = synth(vec![
            CommittedLayerEvent::Anchored(collection(vec![entity])),
            CommittedLayerEvent::Model(model_row(ANCHOR, object_z)),
        ]);

        assert_eq!(rows.len(), 1, "a ZSpanning entity must not get its own row");
        assert_eq!(
            rows[0]
                .ordered_entities
                .iter()
                .map(|entity| entity.entity_id)
                .collect::<Vec<_>>(),
            vec![9]
        );
    }

    /// A `ZSpanning` entity whose anchor layer has no committed `Model` row has
    /// no ADR-0059 position to execute at. Synthesis must FAIL rather than drop
    /// the block: silently discarding it would delete support geometry with no
    /// diagnostic anywhere downstream.
    #[test]
    fn z_spanning_entity_with_no_anchor_row_is_an_error_not_a_silent_drop() {
        let object_z = slicer_ir::mm_to_units(0.4);
        let mut entity = planar_entity(77, object_z);
        entity.geometry = AnchoredGeometryContract::ZSpanning {
            min_z: 0,
            max_z: object_z,
        };

        // The only model row carries index ANCHOR + 5, so nothing matches the
        // collection's `anchor_global_layer_index`.
        let result = synthesize_anchored_rows(vec![
            CommittedLayerEvent::Anchored(collection(vec![entity])),
            CommittedLayerEvent::Model(model_row(ANCHOR + 5, object_z)),
        ]);

        match result {
            Err(LayerExecutionError::AnchoredGeometry { local_id, message }) => {
                assert_eq!(local_id, 77, "the error must name the offending entity");
                assert!(
                    message.contains(&ANCHOR.to_string()),
                    "the message must name the missing anchor layer, got {message:?}"
                );
            }
            other => panic!(
                "an unanchored ZSpanning entity must surface as \
                 LayerExecutionError::AnchoredGeometry, got {other:?}"
            ),
        }
    }

    /// A plane above every object row has no upper `Model` row, so it falls
    /// back to the LAST model row's index — the only anchor available.
    #[test]
    fn plane_above_every_object_row_adopts_the_last_model_index() {
        let object_z = slicer_ir::mm_to_units(0.4);
        let plane_z = object_z + 10 * (MERGE_EPSILON_UNITS + 1);
        const TOP_INDEX: u32 = 4;

        let rows = synth(vec![
            CommittedLayerEvent::Anchored(collection(vec![planar_entity(5, plane_z)])),
            CommittedLayerEvent::Model(model_row(TOP_INDEX, object_z)),
        ]);

        assert_eq!(rows.len(), 2, "the plane is beyond the epsilon: two rows");
        assert_eq!(slicer_ir::mm_to_units(rows[0].z), object_z);
        assert_eq!(slicer_ir::mm_to_units(rows[1].z), plane_z);
        assert_eq!(
            rows[1].global_layer_index, TOP_INDEX,
            "with no upper Model row the solo row adopts the LAST model index"
        );
        assert_eq!(
            rows[1]
                .ordered_entities
                .iter()
                .map(|entity| entity.entity_id)
                .collect::<Vec<_>>(),
            vec![5]
        );
    }

    /// Planes are grouped per collection, so two SEPARATE collections declaring
    /// the same (or within-epsilon) off-grid Z must still coalesce into ONE
    /// synthesized row — AC-5's "no duplicate Z". Beyond the epsilon they stay
    /// two rows.
    #[test]
    fn planes_within_epsilon_across_collections_merge_into_one_row() {
        let object_z = slicer_ir::mm_to_units(0.4);
        let plane_z = object_z - 10 * (MERGE_EPSILON_UNITS + 1);

        // Case A: two collections at the SAME declared Z.
        let rows = synth(vec![
            CommittedLayerEvent::Anchored(collection(vec![planar_entity(1, plane_z)])),
            CommittedLayerEvent::Anchored(collection(vec![planar_entity(2, plane_z)])),
            CommittedLayerEvent::Model(model_row(ANCHOR, object_z)),
        ]);
        assert_eq!(
            rows.len(),
            2,
            "two collections at the same off-grid Z must coalesce into ONE synthesized row \
             plus the object row, got Zs {:?}",
            rows.iter()
                .map(|row| slicer_ir::mm_to_units(row.z))
                .collect::<Vec<_>>()
        );
        assert_eq!(slicer_ir::mm_to_units(rows[0].z), plane_z);
        assert_eq!(
            rows[0]
                .ordered_entities
                .iter()
                .map(|entity| entity.entity_id)
                .collect::<Vec<_>>(),
            vec![1, 2],
            "the coalesced row carries both entities in (z, collection ordinal) order"
        );

        // Case B: Zs differing by exactly `MERGE_EPSILON_UNITS` still coalesce,
        // and the coalesced row keeps the RUN ANCHOR's (lower) Z.
        let rows = synth(vec![
            CommittedLayerEvent::Anchored(collection(vec![planar_entity(1, plane_z)])),
            CommittedLayerEvent::Anchored(collection(vec![planar_entity(
                2,
                plane_z + MERGE_EPSILON_UNITS,
            )])),
            CommittedLayerEvent::Model(model_row(ANCHOR, object_z)),
        ]);
        assert_eq!(
            rows.len(),
            2,
            "planes exactly {MERGE_EPSILON_UNITS} units apart must coalesce, got Zs {:?}",
            rows.iter()
                .map(|row| slicer_ir::mm_to_units(row.z))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            slicer_ir::mm_to_units(rows[0].z),
            plane_z,
            "the coalesced plane keeps the run anchor's Z"
        );
        assert_eq!(
            rows[0]
                .ordered_entities
                .iter()
                .map(|entity| entity.entity_id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );

        // Case C: beyond the epsilon the two collections stay two rows.
        let rows = synth(vec![
            CommittedLayerEvent::Anchored(collection(vec![planar_entity(1, plane_z)])),
            CommittedLayerEvent::Anchored(collection(vec![planar_entity(
                2,
                plane_z + MERGE_EPSILON_UNITS + 1,
            )])),
            CommittedLayerEvent::Model(model_row(ANCHOR, object_z)),
        ]);
        assert_eq!(
            rows.len(),
            3,
            "planes {} units apart are beyond the merge epsilon and stay separate rows",
            MERGE_EPSILON_UNITS + 1
        );
        assert_eq!(slicer_ir::mm_to_units(rows[0].z), plane_z);
        assert_eq!(
            slicer_ir::mm_to_units(rows[1].z),
            plane_z + MERGE_EPSILON_UNITS + 1
        );
    }

    /// Synthesis is deterministic: equal input yields byte-equal output.
    #[test]
    fn synthesis_is_deterministic() {
        let object_z = slicer_ir::mm_to_units(0.4);
        let plane_z = object_z - (MERGE_EPSILON_UNITS + 1);
        let build = || {
            vec![
                CommittedLayerEvent::Anchored(collection(vec![
                    planar_entity(2, plane_z),
                    planar_entity(1, plane_z),
                ])),
                CommittedLayerEvent::Model(model_row(ANCHOR, object_z)),
            ]
        };

        assert_eq!(synth(build()), synth(build()));
        let rows = synth(build());
        assert_eq!(
            rows[0]
                .ordered_entities
                .iter()
                .map(|entity| entity.entity_id)
                .collect::<Vec<_>>(),
            vec![1, 2],
            "entities on one plane order by (z, local_id)"
        );
    }
}
