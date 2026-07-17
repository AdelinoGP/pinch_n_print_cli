// crates/slicer-runtime/tests/executor/main.rs
//
// Aggregator for executor-scope tests. One Cargo integration-test binary for the whole bucket;
// each test file below is mounted as a submodule. See the migration plan for the taxonomy.

#![allow(missing_docs)]

// ── OOM tripwire (AC-N1) ─────────────────────────────────────────────────────
// Guards against a single allocation >= 1 GiB (the Voronoi/emit OOM signature).
// The cumulative TOTAL_LIMIT backstop is intentionally absent: a multi-test
// bucket legitimately allocates >2 GiB in aggregate and must NOT false-trip.
// Re-entrancy is guarded via IN_HOOK so backtrace capture doesn't recurse.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

const GIB: usize = 1024 * 1024 * 1024;
const SINGLE_LIMIT: usize = GIB; // any single alloc >= 1 GiB is the OOM smoking gun

static TRIPPED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static IN_HOOK: Cell<bool> = const { Cell::new(false) };
}

struct OomGuard;

unsafe impl GlobalAlloc for OomGuard {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let in_hook = IN_HOOK.with(|f| f.get());
        if !in_hook && size >= SINGLE_LIMIT {
            if TRIPPED.swap(true, Ordering::SeqCst) {
                std::process::exit(173);
            }
            IN_HOOK.with(|f| f.set(true)); // route allocs made below straight to System
            eprintln!(
                "\n=================== OOM-GUARD TRIPPED (SINGLE) ===================\n\
                 requested SINGLE allocation = {} bytes  ({:.3} GiB)\n\
                 alignment                   = {}",
                size,
                size as f64 / GIB as f64,
                layout.align(),
            );
            let bt = std::backtrace::Backtrace::force_capture();
            eprintln!("{bt}");
            use std::io::Write as _;
            let _ = std::io::stderr().flush();
            std::process::exit(173);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GUARD: OomGuard = OomGuard;
// ── end OOM tripwire ──────────────────────────────────────────────────────────

#[path = "../common/mod.rs"]
mod common;

mod apply_commit_regression_tdd;
mod arachne_negative_spacing_fatal;
mod arachne_perimeters_simple_square;
mod cube_4color_arachne;
mod cube_4color_gcode_output_tdd;
mod cube_4color_ironing_per_painted_top_color_tdd;
mod cube_4color_paint_tdd;
mod cube_4color_phase5_tdd;
mod cube_4color_sparse_infill_per_painted_region_tdd;
mod cube_fuzzy_painted_tdd;
mod finalization_builder_insert;
mod finalization_builder_permute;
mod finalization_builder_readback;
mod finalization_live_tdd;
mod finalization_mutation_roundtrip_tdd;
mod finalization_world_deep_copy_tdd;
mod layer_executor_tdd;
mod layer_finalization_tdd;
mod layer_slice_tdd;
mod layer_world_deep_copy_tdd;
mod live_layer_support_tdd;
mod live_seam_path_tdd;
mod live_top_bottom_fill_tdd;
mod live_travel_policy_tdd;
mod macro_finalization_deep_copy_tdd;
mod paint_channel_consumer_paths_tdd;
mod paint_segmentation_skip_when_no_paint_or_no_opted_in_semantic;
mod postpass_executor_tdd;
mod prepass_execution_order_tdd;
mod prepass_executor_tdd;
mod prepass_overhang_annotation_stage_order_tdd;
mod prepass_seam_planning_macro_path_tdd;
mod prepass_slice_and_shell_tdd;
mod prepass_support_geometry_layer_plan_tdd;
mod prepass_support_geometry_tdd;
mod slicing_promotion_e2e_regression_tdd;
mod support_geometry_slice_consumption_tdd;
