//! Host-owned, strategy-neutral support analysis.

use std::sync::Arc;

use slicer_ir::mm_to_units;
use slicer_ir::slice_ir::{SupportAnalysisIR, SupportCandidate, SupportCandidateSource};

use crate::blackboard::Blackboard;

/// Build conservative candidates without propagating support bodies.
pub fn commit_support_analysis_builtin(
    blackboard: &mut Blackboard,
    enable_support: bool,
) -> Result<(), crate::BlackboardError> {
    let mut ir = SupportAnalysisIR::default();
    if enable_support {
        if let (Some(slices), Some(plan)) = (blackboard.slice_ir(), blackboard.layer_plan()) {
            let mut id = 0_u64;
            for slice in slices.iter() {
                let z = plan
                    .global_layers
                    .get(slice.global_layer_index as usize)
                    .map_or(0.0, |layer| layer.z);
                for region in &slice.regions {
                    if region.polygons.is_empty() {
                        continue;
                    }
                    let source = SupportCandidateSource {
                        object_id: region.object_id.clone(),
                        region_id: region.region_id,
                        global_layer_index: slice.global_layer_index,
                        z_units: mm_to_units(z),
                    };
                    ir.candidates.push(SupportCandidate {
                        id,
                        geometry: region.polygons.clone(),
                        source,
                        enforced: false,
                        blocked: false,
                    });
                    id += 1;
                }
            }
            ir.candidates.sort_by_key(|candidate| {
                (
                    candidate.source.global_layer_index,
                    candidate.source.object_id.clone(),
                    candidate.source.region_id,
                    candidate.id,
                )
            });
            for candidate in &ir.candidates {
                ir.family_assignments
                    .entry((
                        candidate.source.object_id.clone(),
                        candidate.source.region_id,
                    ))
                    .or_insert_with(|| "traditional".to_string());
            }
        }
    }
    blackboard.commit_support_analysis(Arc::new(ir))
}
