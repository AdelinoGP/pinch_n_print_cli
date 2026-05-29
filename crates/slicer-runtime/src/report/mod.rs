//! Slicer report — in-process collector that turns `PipelineInstrumentation`
//! bracket events into a self-contained HTML file.
//!
//! Architecture: `slicer-runtime` owns the trait (`PipelineInstrumentation`);
//! this `report` module provides the consumer side — an
//! [`Collector`] that records timing, memory, and DAG metadata, plus an
//! [`AccountingAllocator`] that the binary can install as
//! `#[global_allocator]` to track host-side bytes per bracket scope.
//!
//! Opt-in / zero-overhead: when the binary does not install the allocator
//! or does not enable accounting via [`allocator::enable`], the
//! `AccountingAllocator` fast path is a single relaxed atomic load per
//! alloc; bracket calls into the Noop instrumentation are inlined to
//! nothing. Use a real `Collector` only when the `--report <PATH>` flag
//! is present.

pub mod allocator;
pub mod collector;
pub mod model;
pub mod render;

pub use allocator::{AccountingAllocator, MemStats};
pub use collector::Collector;
pub use model::{
    Bytes, LayerRecord, MemDelta, ModuleRecord, Nanos, ParallelismRecord, Report, SliceMeta,
    StageRecord,
};
pub use render::render_html;
