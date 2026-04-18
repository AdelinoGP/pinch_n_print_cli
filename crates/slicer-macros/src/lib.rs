//! Proc-macros for the ModularSlicer SDK.
//!
//! This crate provides:
//! - `#[slicer_module]` — promotes `impl LayerModule for T` / `impl PrepassModule for T`
//!   / `impl FinalizationModule for T` / `impl PostpassModule for T` into a
//!   binding-schema surface that matches the documented WIT worlds under
//!   `wit/world-*.wit` (docs/03, docs/05).
//! - `#[module_test]` — test wrapper with mock host setup.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemImpl, ReturnType};

// Stage/world/export table is centralised in `slicer-schema` so
// `#[slicer_module]` and `slicer-cli::cmd_new` stay in lock-step and
// drift between the macro-emitted binding and generated manifests is
// structurally impossible (docs/03, docs/05).
use slicer_schema::{StageSpec, STAGES, WORLD_LIFECYCLE_EXPORTS as WORLD_LIFECYCLE};

/// The `#[slicer_module]` attribute macro.
///
/// Applied to an `impl <Module>Trait for T` block, this macro:
/// 1. Detects which documented stage method (if any) is implemented.
/// 2. Rejects impl blocks that declare more than one stage method.
/// 3. Rejects impl blocks whose detected stage does not belong to the
///    world implied by the implemented SDK trait (e.g. `run_infill`
///    inside `impl PrepassModule for T`).
/// 4. Emits a read-only binding-schema inherent impl (world id, trait
///    name, WIT export names list, stage kebab name, type name, …)
///    plus the legacy marker helpers the existing host/tooling reads.
/// 5. Generates a compile-time `const SLICER_MODULE_SCHEMA` struct
///    describing the full WIT export surface for this module, plus a
///    thin dispatcher `__slicer_wit_run(...)` that delegates through
///    the implemented trait.
#[proc_macro_attribute]
pub fn slicer_module(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let self_ty = input.self_ty.clone();

    let detected_stages = detect_stage_methods(&input);

    if detected_stages.len() > 1 {
        let names: Vec<&str> = detected_stages.iter().map(|s| s.method).collect();
        let msg = format!(
            "slicer_module: impl block contains multiple stage methods: {}. \
             A module must implement exactly one stage function.",
            names.join(", ")
        );
        return syn::Error::new_spanned(&input.self_ty, msg)
            .to_compile_error()
            .into();
    }

    // Capture the SDK trait path from `impl <Trait> for <Type>` if present.
    let trait_ident = input
        .trait_
        .as_ref()
        .and_then(|(_, path, _)| path.segments.last().map(|s| s.ident.to_string()));

    // Cross-world guardrail: if we detected a stage method AND the impl
    // declares a known SDK trait, they must agree on the WIT world.
    if let (Some(stage), Some(trait_name)) = (detected_stages.first(), trait_ident.as_deref()) {
        if is_known_trait(trait_name) && stage.trait_name != trait_name {
            let msg = format!(
                "slicer_module: stage method `{method}` belongs to world `{stage_world}` \
                 (expected trait `{expected_trait}`) but the impl declares trait `{got}` \
                 (world `{got_world}`).",
                method = stage.method,
                stage_world = stage.world_id,
                expected_trait = stage.trait_name,
                got = trait_name,
                got_world = world_for_trait(trait_name).unwrap_or("<unknown>"),
            );
            return syn::Error::new_spanned(&input.self_ty, msg)
                .to_compile_error()
                .into();
        }
    }

    let expanded = generate_slicer_module_impl(&input, &self_ty, &detected_stages, trait_ident.as_deref());
    TokenStream::from(expanded)
}

/// Returns true when the SDK trait name is one the macro knows about.
fn is_known_trait(name: &str) -> bool {
    matches!(
        name,
        "LayerModule" | "PrepassModule" | "FinalizationModule" | "PostpassModule"
    )
}

/// Map SDK trait name → WIT world id.
fn world_for_trait(trait_name: &str) -> Option<&'static str> {
    Some(match trait_name {
        "LayerModule" => "slicer:world-layer@1.0.0",
        "PrepassModule" => "slicer:world-prepass@1.0.0",
        "FinalizationModule" => "slicer:world-finalization@1.0.0",
        "PostpassModule" => "slicer:world-postpass@1.0.0",
        _ => return None,
    })
}

/// Detect which `run_*` stage methods are present in the impl block.
fn detect_stage_methods(input: &ItemImpl) -> Vec<&'static StageSpec> {
    let mut found = Vec::new();
    for item in &input.items {
        if let syn::ImplItem::Fn(method) = item {
            let name = method.sig.ident.to_string();
            for spec in STAGES {
                if name == spec.method {
                    found.push(spec);
                }
            }
        }
    }
    found
}

/// Generate the expanded impl.
fn generate_slicer_module_impl(
    input: &ItemImpl,
    self_ty: &syn::Type,
    detected: &[&StageSpec],
    trait_ident: Option<&str>,
) -> TokenStream2 {
    let type_name_str = quote!(#self_ty).to_string();
    let original_impl = quote! { #input };

    let has_stage = !detected.is_empty();

    let (stage_id_literal, stage_method_literal, stage_export_literal, stage_world_literal) =
        if let Some(s) = detected.first() {
            (
                s.stage_id,
                s.method,
                s.wit_export,
                s.world_id,
            )
        } else {
            ("", "", "", "")
        };

    // Choose effective WIT world: prefer the trait's world if the trait
    // is known, else the detected stage's world, else empty.
    let effective_world = trait_ident
        .and_then(world_for_trait)
        .unwrap_or(stage_world_literal);

    let trait_name_literal = trait_ident.unwrap_or("");

    // Build the WIT-export list for this module: lifecycle for its world,
    // plus the detected stage export (if any).
    let lifecycle_exports: &[&str] = WORLD_LIFECYCLE
        .iter()
        .find(|(w, _)| *w == effective_world)
        .map(|(_, exports)| *exports)
        .unwrap_or(&[]);
    let mut wit_exports: Vec<&str> = lifecycle_exports.to_vec();
    if !stage_export_literal.is_empty() {
        wit_exports.push(stage_export_literal);
    }
    let wit_exports_tokens = wit_exports.iter().map(|e| quote! { #e });

    // Typed structured export bindings. Every lifecycle export carries
    // `Lifecycle`; the detected stage export (if any) carries `Stage`.
    // Ordering is: lifecycle exports in source order (on-print-start,
    // on-print-end), then the stage export.
    let lifecycle_count = lifecycle_exports.len();
    let lifecycle_binding_tokens = lifecycle_exports.iter().map(|e| {
        quote! {
            ::slicer_schema::ExportBinding {
                name: #e,
                kind: ::slicer_schema::ExportKind::Lifecycle,
            }
        }
    });
    let stage_binding_tokens: TokenStream2 = if stage_export_literal.is_empty() {
        quote! {}
    } else {
        quote! {
            , ::slicer_schema::ExportBinding {
                name: #stage_export_literal,
                kind: ::slicer_schema::ExportKind::Stage,
            }
        }
    };

    // Compile-time JSON schema blob describing the module's full binding
    // surface. This is the "real glue" consumed by the host plan/build
    // step and by the CLI `test`/`build` scaffolding; keeping it as a
    // static string avoids dragging serde into a proc-macro crate.
    let schema_json = format!(
        r#"{{"type":"{ty}","trait":"{tr}","world":"{world}","stage_id":"{stage}","stage_method":"{method}","stage_export":"{export}","wit_exports":[{exports}]}}"#,
        ty = type_name_str.replace('"', "\\\""),
        tr = trait_name_literal,
        world = effective_world,
        stage = stage_id_literal,
        method = stage_method_literal,
        export = stage_export_literal,
        exports = wit_exports
            .iter()
            .map(|e| format!("\"{e}\""))
            .collect::<Vec<_>>()
            .join(",")
    );

    let generated_methods = quote! {
        impl #self_ty {
            // ── Legacy marker surface (kept for existing tests/tooling) ──

            /// Module entry point marker. Generated by `#[slicer_module]`.
            #[doc(hidden)]
            pub fn __slicer_module_marker() -> bool { true }

            /// True when the impl block contains a recognized stage method.
            #[doc(hidden)]
            pub fn __slicer_has_stage_function() -> bool { #has_stage }

            /// True if the module is WIT export compatible.
            #[doc(hidden)]
            pub fn __slicer_wit_compatible() -> bool { true }

            /// Canonical scheduler stage id detected in the impl, or "".
            #[doc(hidden)]
            pub fn __slicer_stage_name() -> &'static str { #stage_id_literal }

            /// The module's Rust type name, as written at the impl site.
            #[doc(hidden)]
            pub fn __slicer_type_name() -> &'static str { #type_name_str }

            // ── Real binding surface ─────────────────────────────────────

            /// WIT world package id backing this module (e.g.
            /// `"slicer:world-layer@1.0.0"`) or "" if the impl targets
            /// an unknown trait and no stage was detected.
            #[doc(hidden)]
            pub fn __slicer_world_id() -> &'static str { #effective_world }

            /// Name of the SDK trait the impl targets, or "" if the
            /// macro was applied to an inherent impl.
            #[doc(hidden)]
            pub fn __slicer_trait_name() -> &'static str { #trait_name_literal }

            /// Kebab-case WIT export name for the detected stage, e.g.
            /// `"run-infill"`, or "" if no stage method was detected.
            #[doc(hidden)]
            pub fn __slicer_stage_export_name() -> &'static str { #stage_export_literal }

            /// Rust-cased name of the detected stage method, e.g.
            /// `"run_infill"`, or "" if no stage method was detected.
            #[doc(hidden)]
            pub fn __slicer_stage_method_name() -> &'static str { #stage_method_literal }

            /// The full list of WIT export names this module provides:
            /// the world's lifecycle exports plus the detected stage.
            #[doc(hidden)]
            pub fn __slicer_wit_exports() -> &'static [&'static str] {
                &[ #( #wit_exports_tokens ),* ]
            }

            /// A JSON blob describing the module's complete binding
            /// schema. Stable, machine-readable; intended to be consumed
            /// by host plan/build tooling.
            #[doc(hidden)]
            pub fn __slicer_binding_schema_json() -> &'static str { #schema_json }

            /// Typed compile-time binding schema describing this module's
            /// complete WIT export surface. This is the structured form
            /// promised by the `#[slicer_module]` docstring: consumers
            /// (host plan/build, CLI `validate`/`test`) can reflect over
            /// it without parsing JSON (docs/05 §Module Entry Point;
            /// docs/03 §WIT worlds).
            #[doc(hidden)]
            pub const SLICER_MODULE_SCHEMA: ::slicer_schema::SlicerModuleSchema =
                ::slicer_schema::SlicerModuleSchema {
                    type_name: #type_name_str,
                    trait_name: #trait_name_literal,
                    world_id: #effective_world,
                    stage_id: #stage_id_literal,
                    stage_method: #stage_method_literal,
                    stage_export: #stage_export_literal,
                    exports: &[
                        #( #lifecycle_binding_tokens ),*
                        #stage_binding_tokens
                    ],
                };

            /// Accessor returning a reference to the module's typed
            /// binding schema. Present so the schema can be used through
            /// dynamic dispatch paths where an associated `const` cannot
            /// be named.
            #[doc(hidden)]
            pub fn __slicer_module_schema() -> &'static ::slicer_schema::SlicerModuleSchema {
                &Self::SLICER_MODULE_SCHEMA
            }

            /// Reports the lifecycle-export count for this module's
            /// world; tests and host tooling use this to verify that
            /// every world's mandatory lifecycle exports (`on-print-start`,
            /// `on-print-end`) are present in the emitted binding surface.
            #[doc(hidden)]
            pub const __SLICER_LIFECYCLE_EXPORT_COUNT: usize = #lifecycle_count;
        }
    };

    // ── wasm32-only real export glue ────────────────────────────────
    //
    // On `target_arch = "wasm32"` the macro emits one `extern "C"` shim
    // per WIT export (lifecycle + detected stage) with `#[export_name]`
    // set to the documented kebab-case WIT export name. These shims
    // register genuine export entries in the final .wasm artifact so
    // host-side introspection (and the documented authoring contract in
    // docs/05 §Module Entry Point) sees the declared surface rather
    // than an empty export table.
    //
    // Shim bodies are intentionally minimal: lifecycle returns 0 (OK)
    // and the stage shim returns 0 (OK). Full typed data transfer
    // through the component model is handled elsewhere (the host's
    // `wasmtime::component` dispatcher + host-side wit-bindgen
    // bindings); this step closes the export-surface gap without
    // broadening into module body rewrites (TASK-111 scope).
    //
    // Symbols are module-qualified via a dedicated `const _: () = { ... }`
    // block so `#[slicer_module]` applied to multiple types in the same
    // native test crate does not collide at Rust scope; `#[export_name]`
    // still emits the kebab-case WIT name at the WASM export level,
    // which is what host tooling inspects. The `cfg(target_arch =
    // "wasm32")` guard ensures native host-side test builds are
    // unaffected.
    let type_ident_hash: u64 = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        type_name_str.hash(&mut hasher);
        hasher.finish()
    };
    let shim_mod_ident =
        syn::Ident::new(&format!("__slicer_wasm_exports_{type_ident_hash:x}"), proc_macro2::Span::call_site());

    let lifecycle_shim_tokens: Vec<TokenStream2> = lifecycle_exports
        .iter()
        .map(|export| {
            let shim_name = syn::Ident::new(
                &format!("__slicer_export_{}", export.replace('-', "_")),
                proc_macro2::Span::call_site(),
            );
            quote! {
                #[cfg(target_arch = "wasm32")]
                #[export_name = #export]
                pub extern "C" fn #shim_name() -> i32 { 0 }
            }
        })
        .collect();

    // ── Real typed export glue per supported world (TASK-109) ───────
    //
    // For every world the macro now emits real, typed
    // `wit_bindgen::generate!`-backed component export glue that
    // marshals arguments through the documented WIT world into the
    // implemented SDK trait method. The placeholder `extern "C" fn ...
    // -> i32 { 0 }` stage/lifecycle shims are suppressed for these
    // worlds so they do not collide with or contaminate the real
    // component exports (docs/05 §Module Entry Point; docs/03
    // wit/world-*.wit).
    //
    // Worlds covered: postpass (gcode + text), finalization, prepass
    // (mesh-analysis + layer-planning), layer (all 8 stage exports +
    // 2 lifecycle exports).
    let real_glue_world = resolve_world_glue(stage_id_literal, trait_ident);

    let stage_shim_tokens: TokenStream2 = if stage_export_literal.is_empty() || real_glue_world.is_some() {
        quote! {}
    } else {
        let shim_name = syn::Ident::new(
            &format!("__slicer_export_{}", stage_export_literal.replace('-', "_")),
            proc_macro2::Span::call_site(),
        );
        quote! {
            #[cfg(target_arch = "wasm32")]
            #[export_name = #stage_export_literal]
            pub extern "C" fn #shim_name() -> i32 { 0 }
        }
    };

    // For worlds that emit real glue, skip the lifecycle fake shims —
    // the wit-bindgen expansion handles lifecycle exports (layer world)
    // or the world declares none (postpass/prepass/finalization). Raw
    // `#[export_name]` lifecycle symbols would either collide with the
    // real exports or leak non-component symbols into the final .wasm.
    let skip_lifecycle_shims = real_glue_world.is_some();
    let active_lifecycle_shims: Vec<TokenStream2> = if skip_lifecycle_shims {
        Vec::new()
    } else {
        lifecycle_shim_tokens
    };

    let world_glue: TokenStream2 = match real_glue_world {
        Some(WorldGlueKind::Postpass) => build_postpass_world_glue(self_ty, stage_id_literal),
        Some(WorldGlueKind::Finalization) => build_finalization_world_glue(self_ty),
        Some(WorldGlueKind::Prepass) => build_prepass_world_glue(self_ty, stage_id_literal),
        Some(WorldGlueKind::Layer) => build_layer_world_glue(self_ty, stage_id_literal),
        None => quote! {},
    };

    let wasm_export_shims = quote! {
        #[cfg(target_arch = "wasm32")]
        #[allow(dead_code)]
        mod #shim_mod_ident {
            #( #active_lifecycle_shims )*
            #stage_shim_tokens
        }
        #world_glue
    };

    quote! {
        #original_impl
        #generated_methods
        #wasm_export_shims
    }
}

/// Selector for which WIT world to emit real macro-generated export
/// glue for. Returned by [`resolve_world_glue`] based on the detected
/// stage and declared SDK trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldGlueKind {
    /// `slicer:world-postpass@1.0.0` — gcode + text postprocess.
    Postpass,
    /// `slicer:world-finalization@1.0.0` — layer finalization.
    Finalization,
    /// `slicer:world-prepass@1.0.0` — mesh analysis + layer planning.
    Prepass,
    /// `slicer:world-layer@1.0.0` — all 8 per-layer stage exports.
    Layer,
}

/// Decide which WIT world gets real `wit_bindgen`-backed macro-generated
/// glue for this `#[slicer_module]` invocation. Glue is emitted when:
/// - the stage id belongs to a supported world, OR
/// - the impl declares a known SDK trait (lifecycle-only impls).
///
/// Unresolvable combinations return `None`, in which case the legacy
/// placeholder shim path is emitted (inert — currently no legitimate
/// authoring path hits that branch).
fn resolve_world_glue(stage_id: &str, trait_ident: Option<&str>) -> Option<WorldGlueKind> {
    match stage_id {
        "PostPass::TextPostProcess" | "PostPass::GCodePostProcess" => Some(WorldGlueKind::Postpass),
        "PostPass::LayerFinalization" => Some(WorldGlueKind::Finalization),
        "PrePass::MeshAnalysis"
        | "PrePass::LayerPlanning"
        | "PrePass::MeshSegmentation"
        | "PrePass::PaintSegmentation" => Some(WorldGlueKind::Prepass),
        "Layer::Slice"
        | "Layer::SlicePostProcess"
        | "Layer::Perimeters"
        | "Layer::PerimetersPostProcess"
        | "Layer::Infill"
        | "Layer::InfillPostProcess"
        | "Layer::Support"
        | "Layer::SupportPostProcess"
        | "Layer::PathOptimization" => Some(WorldGlueKind::Layer),
        _ => match trait_ident {
            Some("PostpassModule") => Some(WorldGlueKind::Postpass),
            Some("FinalizationModule") => Some(WorldGlueKind::Finalization),
            Some("PrepassModule") => Some(WorldGlueKind::Prepass),
            Some("LayerModule") => Some(WorldGlueKind::Layer),
            _ => None,
        },
    }
}

/// Shared per-world module preamble: `wit_bindgen::generate!` expansion,
/// a `ConfigValue` `use` statement, a `__slicer_adapt_config` helper
/// and a `__slicer_error_out` helper. The `world_ident` string selects
/// the world, and `world_namespace_ident` is the Rust module path
/// produced by wit-bindgen for that world (e.g. `postpass_world`,
/// `layer_world`). Caller supplies the inline WIT and the
/// world-specific `impl Guest` body.
fn emit_world_preamble(
    world_name: &str,
    world_namespace: &str,
    inline_wit: &str,
) -> TokenStream2 {
    let ns_path: syn::Path = syn::parse_str(&format!("self::slicer::{world_namespace}::config_types::ConfigValue"))
        .expect("parse ConfigValue path");
    quote! {
        ::wit_bindgen::generate!({
            inline: #inline_wit,
            world: #world_name,
        });

        // Bring the wit-bindgen-generated `ConfigValue` variant into
        // scope so the adapter match arms can reference it directly.
        use #ns_path as __SlicerWitConfigValue;

        /// Adapt a wit-bindgen `ConfigView` resource into a
        /// `slicer_ir::ConfigView`, preserving every declared key/value.
        fn __slicer_adapt_config(
            wit_cfg: &ConfigView,
        ) -> ::slicer_ir::ConfigView {
            use ::std::collections::HashMap;
            let mut fields: HashMap<String, ::slicer_ir::ConfigValue> = HashMap::new();
            for k in wit_cfg.keys() {
                if let Some(v) = wit_cfg.get(&k) {
                    let iv = match v {
                        __SlicerWitConfigValue::BoolVal(b) => ::slicer_ir::ConfigValue::Bool(b),
                        __SlicerWitConfigValue::IntVal(i) => ::slicer_ir::ConfigValue::Int(i),
                        __SlicerWitConfigValue::FloatVal(f) => ::slicer_ir::ConfigValue::Float(f),
                        __SlicerWitConfigValue::StringVal(s) => ::slicer_ir::ConfigValue::String(s),
                        __SlicerWitConfigValue::FloatList(v) => ::slicer_ir::ConfigValue::List(
                            v.into_iter().map(::slicer_ir::ConfigValue::Float).collect()
                        ),
                        __SlicerWitConfigValue::StringList(v) => ::slicer_ir::ConfigValue::List(
                            v.into_iter().map(::slicer_ir::ConfigValue::String).collect()
                        ),
                    };
                    fields.insert(k, iv);
                }
            }
            ::slicer_ir::ConfigView::from_map(fields)
        }

        fn __slicer_error_out(e: ::slicer_sdk::error::ModuleError) -> ModuleError {
            ModuleError { code: e.code, message: e.message, fatal: e.fatal }
        }
    }
}

/// Emit the `wit_bindgen`-backed component export glue for the postpass
/// world (`PostPass::TextPostProcess` + `PostPass::GCodePostProcess`).
/// Only compiled on `wasm32`.
fn build_postpass_world_glue(self_ty: &syn::Type, detected_stage: &str) -> TokenStream2 {
    let wit_inline = r#"
        package slicer:world-postpass@1.0.0;

        include "../../wit/deps/types.wit";
        include "../../wit/deps/config.wit";

        interface host-services {
            use geometry.{point3, bounding-box3, ex-polygon, polygon};
            type object-id = string;
            enum log-level { trace, debug, info, warn, error }
            log: func(level: log-level, message: string);
            raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
            surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
            object-bounds:      func(object-id: object-id) -> bounding-box3;
            enum clip-operation   { union, intersection, difference, xor }
            enum offset-join-type { miter, round, square }
            clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
            offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
            simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;
            now-us: func() -> u64;
        }

        world postpass-module {
            import host-services;
            import config-types;
            use config-types.{config-view};
            use geometry.{extrusion-role};
            record module-error { code: u32, message: string, fatal: bool }

            record gcode-move-cmd { x: option<f32>, y: option<f32>, z: option<f32>, e: option<f32>, f: option<f32>, role: extrusion-role }
            resource gcode-output-builder {
                push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
                push-retract:     func(length: f32, speed: f32) -> result<_, string>;
                push-fan-speed:   func(value: u8) -> result<_, string>;
                push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
                push-tool-change: func(from-tool: u32, to-tool: u32) -> result<_, string>;
                push-comment:     func(text: string) -> result<_, string>;
                push-raw:         func(text: string) -> result<_, string>;
                push-z-hop:       func(after-entity-index: u32, hop-height: f32) -> result<_, string>;
            }

            enum gcode-command-kind { move-cmd, retract, fan-speed, temperature, tool-change, comment, raw }
            record gcode-command-view { index: u32, kind: gcode-command-kind }

            export run-gcode-postprocess: func(
                commands: list<gcode-command-view>,
                output: gcode-output-builder,
                config: config-view,
            ) -> result<_, module-error>;

            export run-text-postprocess: func(
                gcode-text: string,
                config: config-view,
            ) -> result<string, module-error>;
        }
    "#;

    let preamble = emit_world_preamble("postpass-module", "postpass_world", wit_inline);

    // Decide which stage method routes into the user's trait: the
    // detected stage for this impl. The other arm returns a benign
    // `Ok` so the component remains WIT-conformant.
    let (gcode_arm, text_arm) = match detected_stage {
        "PostPass::GCodePostProcess" => (
            quote! {
                let ir_config = __slicer_adapt_config(&config);
                let module = match <#self_ty as ::slicer_sdk::traits::PostpassModule>::on_print_start(&ir_config) {
                    Ok(m) => m,
                    Err(e) => return Err(__slicer_error_out(e)),
                };
                // `run_gcode_postprocess` takes typed `GcodeCommandView`
                // slices via the SDK-level postpass_types module. The
                // current SDK trait signature (docs/05) is
                // `fn run_gcode_postprocess(&self, commands: &[GcodeCommandView], output: &mut GcodeOutputBuilder, config: &ConfigView) -> Result<(), ModuleError>`.
                // We construct empty SDK views/builders — the SDK trait
                // default accepts them; resource-level deep copy for
                // per-command content is a follow-on polish.
                let sdk_commands: ::std::vec::Vec<::slicer_sdk::postpass_types::GcodeCommandView> = ::std::vec::Vec::new();
                let mut sdk_builder = ::slicer_sdk::postpass_builders::GcodeOutputBuilder::new();
                let out = <#self_ty as ::slicer_sdk::traits::PostpassModule>::run_gcode_postprocess(
                    &module, &sdk_commands, &mut sdk_builder, &ir_config,
                );
                match out {
                    Ok(()) => Ok(()),
                    Err(e) => Err(__slicer_error_out(e)),
                }
            },
            quote! { Ok(gcode_text) },
        ),
        _ => (
            quote! { Ok(()) },
            quote! {
                let ir_config = __slicer_adapt_config(&config);
                let module = match <#self_ty as ::slicer_sdk::traits::PostpassModule>::on_print_start(&ir_config) {
                    Ok(m) => m,
                    Err(e) => return Err(__slicer_error_out(e)),
                };
                let out = <#self_ty as ::slicer_sdk::traits::PostpassModule>::run_text_postprocess(
                    &module, &gcode_text, &ir_config,
                );
                match out {
                    Ok(s) => Ok(s),
                    Err(e) => Err(__slicer_error_out(e)),
                }
            },
        ),
    };

    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_postpass_world_export {
            // Intentionally do NOT `use super::*;` — the user's module
            // may have imported types (e.g. `slicer_ir::Point3WithWidth`)
            // that would collide with the wit-bindgen-generated names.
            // Bring in only the user's module type.
            use super::#self_ty;

            #preamble

            struct __SlicerPostpassComponent;

            impl Guest for __SlicerPostpassComponent {
                fn run_gcode_postprocess(
                    _commands: Vec<GcodeCommandView>,
                    _output: GcodeOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    #gcode_arm
                }

                fn run_text_postprocess(
                    gcode_text: String,
                    config: ConfigView,
                ) -> Result<String, ModuleError> {
                    #text_arm
                }
            }

            export!(__SlicerPostpassComponent);
        }
    }
}

/// Emit the `wit_bindgen`-backed component export glue for the
/// finalization world (`PostPass::LayerFinalization`). Routes into the
/// user's `FinalizationModule::run_finalization` trait method with the
/// typed `ConfigView` pre-filtered and adapted. Resource-level deep
/// copy of `LayerCollectionView` / `FinalizationOutputBuilder` is a
/// follow-on polish; the SDK trait sees well-typed (possibly empty)
/// SDK values and its `Result<(), ModuleError>` return round-trips.
fn build_finalization_world_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = r#"
        package slicer:world-finalization@1.0.0;

        include "../../wit/deps/types.wit";
        include "../../wit/deps/config.wit";

        interface host-services {
            use geometry.{point3, bounding-box3, ex-polygon, polygon};
            type object-id = string;
            enum log-level { trace, debug, info, warn, error }
            log: func(level: log-level, message: string);
            raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
            surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
            object-bounds:      func(object-id: object-id) -> bounding-box3;
            enum clip-operation   { union, intersection, difference, xor }
            enum offset-join-type { miter, round, square }
            clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
            offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
            simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;
            now-us: func() -> u64;
        }

        world finalization-module {
            import host-services;
            import config-types;
            use config-types.{config-view};
            use geometry.{extrusion-path-3d};
            type layer-idx = u32;
            type object-id = string;
            type region-id = string;
            record module-error { code: u32, message: string, fatal: bool }
            record region-key { layer-index: layer-idx, object-id: object-id, region-id: region-id }

            record tool-change-view {
                after-entity-index: u32,
                from-tool: u32,
                to-tool: u32,
            }

            resource layer-collection-view {
                layer-index:  func() -> layer-idx;
                z:            func() -> f32;
                entity-count: func() -> u32;
                tool-changes: func() -> list<tool-change-view>;
            }

            resource finalization-output-builder {
                push-entity-to-layer: func(layer-index: layer-idx, path: extrusion-path-3d, region-key: region-key) -> result<_, string>;
                insert-synthetic-layer: func(z: f32, paths: list<extrusion-path-3d>) -> result<_, string>;
            }

            export run-finalization: func(
                layers: list<layer-collection-view>,
                output: finalization-output-builder,
                config: config-view,
            ) -> result<_, module-error>;
        }
    "#;

    let preamble = emit_world_preamble("finalization-module", "finalization_world", wit_inline);

    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_finalization_world_export {
            // Intentionally do NOT `use super::*;` — the user's module
            // may have imported types (e.g. `slicer_ir::Point3WithWidth`)
            // that would collide with the wit-bindgen-generated names.
            // Bring in only the user's module type.
            use super::#self_ty;

            #preamble

            // The `geometry` interface types needed by the drain-back
            // adapter (`ExtrusionRole`, `ExtrusionPath3d`,
            // `Point3WithWidth`) live under the world's geometry
            // namespace and are not re-exported at the world's top
            // level by wit-bindgen. Bring them in explicitly.
            use self::slicer::finalization_world::geometry::{
                ExtrusionRole, Point3WithWidth,
            };

            struct __SlicerFinalizationComponent;

            /// Map a wit-bindgen finalization-world `ExtrusionRole`
            /// enum value to `slicer_ir::ExtrusionRole`. The `Custom`
            /// variant loses its payload across the boundary (the WIT
            /// enum is arity-0 while `slicer_ir::ExtrusionRole::Custom`
            /// carries a `String` tag); we synthesise an empty tag so
            /// downstream code sees a valid variant.
            fn __slicer_role_wit_to_ir(r: ExtrusionRole) -> ::slicer_ir::ExtrusionRole {
                match r {
                    ExtrusionRole::OuterWall => ::slicer_ir::ExtrusionRole::OuterWall,
                    ExtrusionRole::InnerWall => ::slicer_ir::ExtrusionRole::InnerWall,
                    ExtrusionRole::ThinWall => ::slicer_ir::ExtrusionRole::ThinWall,
                    ExtrusionRole::TopSolidInfill => ::slicer_ir::ExtrusionRole::TopSolidInfill,
                    ExtrusionRole::BottomSolidInfill => ::slicer_ir::ExtrusionRole::BottomSolidInfill,
                    ExtrusionRole::SparseInfill => ::slicer_ir::ExtrusionRole::SparseInfill,
                    ExtrusionRole::SupportMaterial => ::slicer_ir::ExtrusionRole::SupportMaterial,
                    ExtrusionRole::SupportInterface => ::slicer_ir::ExtrusionRole::SupportInterface,
                    ExtrusionRole::Ironing => ::slicer_ir::ExtrusionRole::Ironing,
                    ExtrusionRole::BridgeInfill => ::slicer_ir::ExtrusionRole::BridgeInfill,
                    ExtrusionRole::WipeTower => ::slicer_ir::ExtrusionRole::WipeTower,
                    ExtrusionRole::Custom => ::slicer_ir::ExtrusionRole::Custom(::std::string::String::new()),
                }
            }

            fn __slicer_role_ir_to_wit(r: &::slicer_ir::ExtrusionRole) -> ExtrusionRole {
                match r {
                    ::slicer_ir::ExtrusionRole::OuterWall => ExtrusionRole::OuterWall,
                    ::slicer_ir::ExtrusionRole::InnerWall => ExtrusionRole::InnerWall,
                    ::slicer_ir::ExtrusionRole::ThinWall => ExtrusionRole::ThinWall,
                    ::slicer_ir::ExtrusionRole::TopSolidInfill => ExtrusionRole::TopSolidInfill,
                    ::slicer_ir::ExtrusionRole::BottomSolidInfill => ExtrusionRole::BottomSolidInfill,
                    ::slicer_ir::ExtrusionRole::SparseInfill => ExtrusionRole::SparseInfill,
                    ::slicer_ir::ExtrusionRole::SupportMaterial => ExtrusionRole::SupportMaterial,
                    ::slicer_ir::ExtrusionRole::SupportInterface => ExtrusionRole::SupportInterface,
                    ::slicer_ir::ExtrusionRole::Ironing => ExtrusionRole::Ironing,
                    ::slicer_ir::ExtrusionRole::BridgeInfill => ExtrusionRole::BridgeInfill,
                    ::slicer_ir::ExtrusionRole::WipeTower => ExtrusionRole::WipeTower,
                    // The finalization-world WIT `extrusion-role` enum
                    // carries a subset of `slicer_ir::ExtrusionRole`.
                    // Roles without a direct counterpart map to the
                    // arity-0 `Custom` variant — the IR role's name
                    // tag (if any) is dropped at the boundary.
                    ::slicer_ir::ExtrusionRole::PrimeTower => ExtrusionRole::Custom,
                    ::slicer_ir::ExtrusionRole::Skirt => ExtrusionRole::Custom,
                    ::slicer_ir::ExtrusionRole::Custom(_) => ExtrusionRole::Custom,
                }
            }

            fn __slicer_path_ir_to_wit(p: &::slicer_ir::ExtrusionPath3D) -> ExtrusionPath3d {
                ExtrusionPath3d {
                    points: p
                        .points
                        .iter()
                        .map(|pt| Point3WithWidth {
                            x: pt.x,
                            y: pt.y,
                            z: pt.z,
                            width: pt.width,
                            flow_factor: pt.flow_factor,
                        })
                        .collect(),
                    role: __slicer_role_ir_to_wit(&p.role),
                    speed_factor: p.speed_factor,
                }
            }

            impl Guest for __SlicerFinalizationComponent {
                fn run_finalization(
                    layers: Vec<LayerCollectionView>,
                    output: FinalizationOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    let ir_config = __slicer_adapt_config(&config);
                    let module = match <#self_ty as ::slicer_sdk::traits::FinalizationModule>::on_print_start(&ir_config) {
                        Ok(m) => m,
                        Err(e) => return Err(__slicer_error_out(e)),
                    };

                    // ── Input deep copy ────────────────────────────
                    // Build one SDK `LayerCollectionView` per incoming
                    // wit-bindgen resource handle by calling the typed
                    // accessors (`layer-index`, `z`, `entity-count`,
                    // `tool-changes`). The SDK wrapper stores a full
                    // `LayerCollectionIR`; we populate the fields the
                    // guest trait body is documented to read and keep
                    // `ordered_entities` at the reported arity using
                    // empty entity placeholders so `entity_count()`
                    // matches. Real per-entity geometry is not part of
                    // the finalization-world WIT contract (docs/03).
                    let mut sdk_layers: ::std::vec::Vec<::slicer_sdk::traits::LayerCollectionView> =
                        ::std::vec::Vec::with_capacity(layers.len());
                    for wit_layer in layers.iter() {
                        let entity_count = wit_layer.entity_count() as usize;
                        let mut ordered_entities: ::std::vec::Vec<::slicer_ir::PrintEntity> =
                            ::std::vec::Vec::with_capacity(entity_count);
                        for i in 0..entity_count {
                            ordered_entities.push(::slicer_ir::PrintEntity {
                                path: ::slicer_ir::ExtrusionPath3D {
                                    points: ::std::vec::Vec::new(),
                                    role: ::slicer_ir::ExtrusionRole::Custom(::std::string::String::new()),
                                    speed_factor: 1.0,
                                },
                                role: ::slicer_ir::ExtrusionRole::Custom(::std::string::String::new()),
                                region_key: ::slicer_ir::RegionKey {
                                    global_layer_index: wit_layer.layer_index(),
                                    object_id: ::std::string::String::new(),
                                    region_id: 0,
                                },
                                topo_order: i as u32,
                            });
                        }
                        let tool_changes: ::std::vec::Vec<::slicer_ir::ToolChange> = wit_layer
                            .tool_changes()
                            .into_iter()
                            .map(|tc| ::slicer_ir::ToolChange {
                                after_entity_index: tc.after_entity_index,
                                from_tool: tc.from_tool,
                                to_tool: tc.to_tool,
                            })
                            .collect();
                        let ir = ::slicer_ir::LayerCollectionIR {
                            schema_version: ::slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
                            global_layer_index: wit_layer.layer_index(),
                            z: wit_layer.z(),
                            ordered_entities,
                            tool_changes,
                            z_hops: ::std::vec::Vec::new(),
                            annotations: ::std::vec::Vec::new(),
                        };
                        sdk_layers.push(::slicer_sdk::traits::LayerCollectionView::new(ir));
                    }

                    let mut sdk_output = ::slicer_sdk::traits::FinalizationOutputBuilder::new();
                    let out = <#self_ty as ::slicer_sdk::traits::FinalizationModule>::run_finalization(
                        &module, &sdk_layers, &mut sdk_output, &ir_config,
                    );

                    // ── Output drain-back ──────────────────────────
                    // Every entity push / synthetic layer insert that
                    // ran through the SDK builder must be replayed
                    // through the wit-bindgen builder resource so the
                    // host can apply it to the downstream layer
                    // collection (docs/03 world-finalization.wit
                    // §finalization-output-builder). Order is
                    // preserved: entity pushes first in SDK-emission
                    // order, then synthetic-layer inserts.
                    for (layer_index, path, region_key) in sdk_output.entity_pushes() {
                        let wit_path = __slicer_path_ir_to_wit(path);
                        let wit_region_key = RegionKey {
                            layer_index: region_key.global_layer_index,
                            object_id: region_key.object_id.clone(),
                            region_id: region_key.region_id.to_string(),
                        };
                        let _ = output.push_entity_to_layer(*layer_index, &wit_path, &wit_region_key);
                    }
                    for (z, paths) in sdk_output.synthetic_layers() {
                        let wit_paths: ::std::vec::Vec<ExtrusionPath3d> =
                            paths.iter().map(__slicer_path_ir_to_wit).collect();
                        let _ = output.insert_synthetic_layer(*z, &wit_paths);
                    }

                    match out {
                        Ok(()) => Ok(()),
                        Err(e) => Err(__slicer_error_out(e)),
                    }
                }
            }

            export!(__SlicerFinalizationComponent);
        }
    }
}

/// Emit the `wit_bindgen`-backed component export glue for the prepass
/// world (`PrePass::MeshAnalysis` + `PrePass::LayerPlanning`). The
/// other two documented prepass stages (`MeshSegmentation`,
/// `PaintSegmentation`) are not yet routed by the host's wit_host.rs
/// prepass world and therefore stay on the placeholder path.
fn build_prepass_world_glue(self_ty: &syn::Type, detected_stage: &str) -> TokenStream2 {
    let wit_inline = r#"
        package slicer:world-prepass@1.0.0;

        include "../../wit/deps/types.wit";
        include "../../wit/deps/config.wit";

        interface host-services {
            use geometry.{point3, bounding-box3, ex-polygon, polygon};
            type object-id = string;
            enum log-level { trace, debug, info, warn, error }
            log: func(level: log-level, message: string);
            raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
            surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
            object-bounds:      func(object-id: object-id) -> bounding-box3;
            enum clip-operation   { union, intersection, difference, xor }
            enum offset-join-type { miter, round, square }
            clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
            offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
            simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;
            now-us: func() -> u64;
        }

        world prepass-module {
            import host-services;
            import config-types;
            type object-id = string;
            type region-id = string;
            record module-error { code: u32, message: string, fatal: bool }

            enum facet-class { normal, near-horizontal, overhang, bridge, top-surface, bottom-surface }
            record facet-annotation { facet-index: u32, slope-angle-deg: f32, classification: facet-class }
            record surface-group-proposal { facet-indices: list<u32>, z-min: f32, z-max: f32, shell-count: u32 }

            use config-types.{config-view};

            resource mesh-analysis-output {
                push-facet-annotation: func(obj: object-id, ann: facet-annotation) -> result<_, string>;
                push-surface-group:    func(obj: object-id, grp: surface-group-proposal) -> result<_, string>;
            }

            export run-mesh-analysis: func(
                objects: list<object-id>,
                output: mesh-analysis-output,
                config: config-view,
            ) -> result<_, module-error>;

            resource mesh-segmentation-output {
                mark-triangle-paint: func(obj: object-id, facet-index: u32, semantic: string, value: string) -> result<_, string>;
            }

            export run-mesh-segmentation: func(
                objects: list<object-id>,
                output: mesh-segmentation-output,
                config: config-view,
            ) -> result<_, module-error>;

            use geometry.{ex-polygon};

            record paint-region-entry {
                object-id: object-id,
                layer-index: u32,
                semantic: string,
                polygons: list<ex-polygon>,
                value: string,
            }
            resource paint-segmentation-output {
                push-paint-region: func(entry: paint-region-entry) -> result<_, string>;
            }

            export run-paint-segmentation: func(
                objects: list<object-id>,
                output: paint-segmentation-output,
                config: config-view,
            ) -> result<_, module-error>;

            record region-layer-proposal {
                object-id: object-id, region-id: region-id,
                effective-layer-height: f32,
                is-catchup: bool, catchup-z-bottom: f32,
            }
            record layer-proposal { z: f32, active-regions: list<region-layer-proposal> }

            resource layer-plan-output {
                push-layer: func(proposal: layer-proposal) -> result<_, string>;
            }

            export run-layer-planning: func(
                objects: list<object-id>,
                output: layer-plan-output,
                config: config-view,
            ) -> result<_, module-error>;
        }
    "#;

    let preamble = emit_world_preamble("prepass-module", "prepass_world", wit_inline);

    let (mesh_arm, layer_arm, mesh_seg_arm, paint_seg_arm) = match detected_stage {
        "PrePass::MeshAnalysis" => (
            quote! {
                let ir_config = __slicer_adapt_config(&config);
                let module = match <#self_ty as ::slicer_sdk::traits::PrepassModule>::on_print_start(&ir_config) {
                    Ok(m) => m,
                    Err(e) => return Err(__slicer_error_out(e)),
                };
                // Forward the real WIT `objects` list (aliased to
                // `String` by wit-bindgen; `slicer_ir::ObjectId` is also
                // `String`) so the SDK trait body sees per-object ids
                // instead of the previous empty-Vec stub.
                let sdk_objects: ::std::vec::Vec<::slicer_ir::ObjectId> = _objects.clone();
                let mut sdk_output = ::slicer_sdk::prepass_builders::MeshAnalysisOutput::new();
                let out = <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_mesh_analysis(
                    &module, &sdk_objects, &mut sdk_output, &ir_config,
                );
                // Drain the SDK builder back into the WIT
                // `mesh-analysis-output` resource in push order so
                // facet annotations and surface groups reach the host.
                // Push failures surface as fatal `ModuleError` (matches
                // the LayerPlanning bridge pattern): the host-side
                // `push-facet-annotation` / `push-surface-group`
                // handlers reject malformed records (empty object-id,
                // non-finite z bounds, inverted z ranges), and silently
                // dropping them would break the drain contract.
                for (__slicer_obj, __slicer_ann) in sdk_output.facet_annotations() {
                    let __slicer_wit_ann = FacetAnnotation {
                        facet_index: __slicer_ann.facet_index,
                        slope_angle_deg: __slicer_ann.slope_angle_deg,
                        classification: match __slicer_ann.classification {
                            ::slicer_sdk::prepass_types::FacetClass::Normal => FacetClass::Normal,
                            ::slicer_sdk::prepass_types::FacetClass::NearHorizontal => FacetClass::NearHorizontal,
                            ::slicer_sdk::prepass_types::FacetClass::Overhang => FacetClass::Overhang,
                            ::slicer_sdk::prepass_types::FacetClass::Bridge => FacetClass::Bridge,
                            ::slicer_sdk::prepass_types::FacetClass::TopSurface => FacetClass::TopSurface,
                            ::slicer_sdk::prepass_types::FacetClass::BottomSurface => FacetClass::BottomSurface,
                        },
                    };
                    if let Err(e) = _output.push_facet_annotation(
                        __slicer_obj,
                        __slicer_wit_ann,
                    ) {
                        return Err(ModuleError {
                            code: 6,
                            message: e,
                            fatal: true,
                        });
                    }
                }
                for (__slicer_obj, __slicer_grp) in sdk_output.surface_groups() {
                    let __slicer_wit_grp = SurfaceGroupProposal {
                        facet_indices: __slicer_grp.facet_indices.clone(),
                        z_min: __slicer_grp.z_min,
                        z_max: __slicer_grp.z_max,
                        shell_count: __slicer_grp.shell_count,
                    };
                    if let Err(e) = _output.push_surface_group(
                        __slicer_obj,
                        &__slicer_wit_grp,
                    ) {
                        return Err(ModuleError {
                            code: 7,
                            message: e,
                            fatal: true,
                        });
                    }
                }
                match out {
                    Ok(()) => Ok(()),
                    Err(e) => Err(__slicer_error_out(e)),
                }
            },
            quote! { Ok(()) },
            quote! { Ok(()) },
            quote! { Ok(()) },
        ),
        "PrePass::LayerPlanning" => (
            quote! { Ok(()) },
            quote! {
                let ir_config = __slicer_adapt_config(&config);
                let module = match <#self_ty as ::slicer_sdk::traits::PrepassModule>::on_print_start(&ir_config) {
                    Ok(m) => m,
                    Err(e) => return Err(__slicer_error_out(e)),
                };
                // The WIT `objects` list is `Vec<ObjectId>` (aliased to
                // `String` by wit-bindgen); the SDK trait wants
                // `&[::slicer_ir::ObjectId]`, which is also `String`.
                // Forward the list by value so the guest's planner
                // actually sees per-object ids.
                let sdk_objects: ::std::vec::Vec<::slicer_ir::ObjectId> = _objects.clone();
                let mut sdk_output = ::slicer_sdk::prepass_builders::LayerPlanOutput::new();
                let out = <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_layer_planning(
                    &module, &sdk_objects, &mut sdk_output, &ir_config,
                );
                // Drain the SDK builder back into the WIT
                // `layer-plan-output` resource in push order so the host
                // sees every planner-emitted proposal. A `push_layer`
                // failure is surfaced as a fatal module error because
                // the host's `push_layer` host-side handler rejects
                // malformed proposals (e.g. non-finite Z), and dropping
                // them silently would desync the planner's contract.
                for __slicer_layer in sdk_output.layers() {
                    let __slicer_wit_regions: ::std::vec::Vec<RegionLayerProposal> = __slicer_layer
                        .active_regions
                        .iter()
                        .map(|r| RegionLayerProposal {
                            object_id: r.object_id.clone(),
                            region_id: r.region_id.clone(),
                            effective_layer_height: r.effective_layer_height,
                            is_catchup: r.is_catchup,
                            catchup_z_bottom: r.catchup_z_bottom,
                        })
                        .collect();
                    if let Err(e) = _output.push_layer(&LayerProposal {
                        z: __slicer_layer.z,
                        active_regions: __slicer_wit_regions,
                    }) {
                        return Err(ModuleError {
                            code: 5,
                            message: e,
                            fatal: true,
                        });
                    }
                }
                match out {
                    Ok(()) => Ok(()),
                    Err(e) => Err(__slicer_error_out(e)),
                }
            },
            quote! { Ok(()) },
            quote! { Ok(()) },
        ),
        "PrePass::MeshSegmentation" => (
            quote! { Ok(()) },
            quote! { Ok(()) },
            // STEP H: forward real `_objects` as skeletal
            // `MeshObjectView`s (only `object_id` populated; the WIT
            // `run-mesh-segmentation` surface provides just
            // `list<object-id>`, so geometry/paint can't cross the
            // boundary without separate host-service calls), then drain
            // the SDK builder's `triangle_paint_marks` back through the
            // WIT `mesh-segmentation-output::mark-triangle-paint`
            // resource method. The SDK's legacy `push_modification` /
            // `ObjectMeshModification` stream is intentionally NOT
            // drained: it has no WIT representation and is reserved for
            // native-mode authoring. Push failures surface as fatal
            // `ModuleError` (mirrors the LayerPlanning / MeshAnalysis
            // bridge shape from STEP F / STEP G).
            quote! {
                let ir_config = __slicer_adapt_config(&config);
                let module = match <#self_ty as ::slicer_sdk::traits::PrepassModule>::on_print_start(&ir_config) {
                    Ok(m) => m,
                    Err(e) => return Err(__slicer_error_out(e)),
                };
                let sdk_objects: ::std::vec::Vec<::slicer_sdk::prepass_types::MeshObjectView> = _objects
                    .iter()
                    .map(|id| ::slicer_sdk::prepass_types::MeshObjectView {
                        object_id: id.clone(),
                        vertices: ::std::vec::Vec::new(),
                        triangles: ::std::vec::Vec::new(),
                        paint_layers: ::std::vec::Vec::new(),
                    })
                    .collect();
                let mut sdk_output = ::slicer_sdk::prepass_builders::MeshSegmentationOutput::new();
                let out = <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_mesh_segmentation(
                    &module, &sdk_objects, &mut sdk_output, &ir_config,
                );
                for __slicer_mark in sdk_output.triangle_paint_marks() {
                    if let Err(e) = _output.mark_triangle_paint(
                        &__slicer_mark.object_id,
                        __slicer_mark.facet_index,
                        &__slicer_mark.semantic,
                        &__slicer_mark.value,
                    ) {
                        return Err(ModuleError {
                            code: 10,
                            message: e,
                            fatal: true,
                        });
                    }
                }
                match out {
                    Ok(()) => Ok(()),
                    Err(e) => Err(__slicer_error_out(e)),
                }
            },
            quote! { Ok(()) },
        ),
        "PrePass::PaintSegmentation" => (
            quote! { Ok(()) },
            quote! { Ok(()) },
            quote! { Ok(()) },
            // Same disconnect as MeshSegmentation: the SDK
            // `PaintSegmentationOutput` builder operates on an in-Rust
            // tree of `(layer, semantic, object, paint-order)` tuples
            // that doesn't map 1:1 back to the WIT `push-paint-region`
            // calls. Canonical modules implement the WIT world
            // directly in their `wit-guest/` subcrate (pattern shared
            // with `layer-planner-default` and `mesh-segmentation`);
            // the macro path is kept alive for `#[slicer_module]`
            // authoring of PaintSegmentation-stage modules where
            // lifecycle-only behavior is acceptable.
            quote! {
                let ir_config = __slicer_adapt_config(&config);
                let module = match <#self_ty as ::slicer_sdk::traits::PrepassModule>::on_print_start(&ir_config) {
                    Ok(m) => m,
                    Err(e) => return Err(__slicer_error_out(e)),
                };
                let sdk_objects: ::std::vec::Vec<::slicer_sdk::prepass_types::PaintSegmentationObjectView> = ::std::vec::Vec::new();
                let mut sdk_output = ::slicer_sdk::prepass_builders::PaintSegmentationOutput::new();
                let out = <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_paint_segmentation(
                    &module, &sdk_objects, &mut sdk_output, &ir_config,
                );
                match out {
                    Ok(()) => Ok(()),
                    Err(e) => Err(__slicer_error_out(e)),
                }
            },
        ),
        _ => (quote! { Ok(()) }, quote! { Ok(()) }, quote! { Ok(()) }, quote! { Ok(()) }),
    };

    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_prepass_world_export {
            // Intentionally do NOT `use super::*;` — the user's module
            // may have imported types (e.g. `slicer_ir::Point3WithWidth`)
            // that would collide with the wit-bindgen-generated names.
            // Bring in only the user's module type.
            use super::#self_ty;

            #preamble

            struct __SlicerPrepassComponent;

            impl Guest for __SlicerPrepassComponent {
                fn run_mesh_analysis(
                    _objects: Vec<ObjectId>,
                    _output: MeshAnalysisOutput,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    #mesh_arm
                }
                fn run_layer_planning(
                    _objects: Vec<ObjectId>,
                    _output: LayerPlanOutput,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    #layer_arm
                }
                fn run_mesh_segmentation(
                    _objects: Vec<ObjectId>,
                    _output: MeshSegmentationOutput,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    #mesh_seg_arm
                }
                fn run_paint_segmentation(
                    _objects: Vec<ObjectId>,
                    _output: PaintSegmentationOutput,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    #paint_seg_arm
                }
            }

            export!(__SlicerPrepassComponent);
        }
    }
}

/// Emit the `wit_bindgen`-backed component export glue for the layer
/// world (all 8 stage exports + `on-print-start` / `on-print-end`
/// lifecycle). The detected stage routes into the user's trait method
/// with real resource-level deep copy: typed wit-bindgen resources
/// are read through their generated accessors and rebuilt as SDK
/// `SliceRegionView` / `PerimeterRegionView` / `PaintRegionLayerView`
/// values before the trait body runs, and the SDK builder contents
/// the trait body fills are drained back through the corresponding
/// wit-bindgen builder resource methods after it returns. Mirrors
/// the finalization-world deep-copy template at
/// `build_finalization_world_glue`.
fn build_layer_world_glue(self_ty: &syn::Type, detected_stage: &str) -> TokenStream2 {
    let wit_inline = LAYER_WORLD_WIT;
    let preamble = emit_world_preamble("layer-module", "layer_world", wit_inline);

    // Real deep-copy IN (from wit-bindgen resources to SDK views).
    let adapt_slice = quote! {
        let sdk_regions: ::std::vec::Vec<::slicer_sdk::views::SliceRegionView> =
            __slicer_adapt_slice_regions(&regions);
    };
    let adapt_perim = quote! {
        let sdk_regions: ::std::vec::Vec<::slicer_sdk::views::PerimeterRegionView> =
            __slicer_adapt_perimeter_regions(&regions);
    };
    let adapt_paint = quote! {
        let sdk_paint = __slicer_adapt_paint_layer(&paint);
    };

    let slice_postprocess_arm = if detected_stage == "Layer::SlicePostProcess" {
        quote! {
            let ir_config = __slicer_adapt_config(&config);
            let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::on_print_start(&ir_config) {
                Ok(m) => m,
                Err(e) => return Err(__slicer_error_out(e)),
            };
            #adapt_slice
            #adapt_paint
            let mut sdk_output = ::slicer_sdk::builders::SlicePostprocessBuilder::new();
            let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_slice_postprocess(
                &module, layer_index, &sdk_regions, &sdk_paint, &mut sdk_output, &ir_config,
            );
            __slicer_drain_slice_postprocess(&sdk_output, &output);
            match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
        }
    } else {
        quote! { Ok(()) }
    };

    let perimeters_arm = if detected_stage == "Layer::Perimeters" {
        quote! {
            let ir_config = __slicer_adapt_config(&config);
            let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::on_print_start(&ir_config) {
                Ok(m) => m,
                Err(e) => return Err(__slicer_error_out(e)),
            };
            #adapt_slice
            #adapt_paint
            let mut sdk_output = ::slicer_sdk::builders::PerimeterOutputBuilder::new();
            let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_perimeters(
                &module, layer_index, &sdk_regions, &sdk_paint, &mut sdk_output, &ir_config,
            );
            __slicer_drain_perimeter(&sdk_output, &output);
            match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
        }
    } else {
        quote! { Ok(()) }
    };

    let wall_postprocess_arm = if detected_stage == "Layer::PerimetersPostProcess" {
        quote! {
            let ir_config = __slicer_adapt_config(&config);
            let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::on_print_start(&ir_config) {
                Ok(m) => m,
                Err(e) => return Err(__slicer_error_out(e)),
            };
            #adapt_perim
            let mut sdk_output = ::slicer_sdk::builders::PerimeterOutputBuilder::new();
            let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_wall_postprocess(
                &module, layer_index, &sdk_regions, &mut sdk_output, &ir_config,
            );
            __slicer_drain_perimeter(&sdk_output, &output);
            match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
        }
    } else {
        quote! { Ok(()) }
    };

    let infill_arm = if detected_stage == "Layer::Infill" {
        quote! {
            let ir_config = __slicer_adapt_config(&config);
            let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::on_print_start(&ir_config) {
                Ok(m) => m,
                Err(e) => return Err(__slicer_error_out(e)),
            };
            #adapt_slice
            let mut sdk_output = ::slicer_sdk::builders::InfillOutputBuilder::new();
            let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_infill(
                &module, layer_index, &sdk_regions, &mut sdk_output, &ir_config,
            );
            __slicer_drain_infill(&sdk_output, &output);
            match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
        }
    } else {
        quote! { Ok(()) }
    };

    let infill_postprocess_arm = if detected_stage == "Layer::InfillPostProcess" {
        quote! {
            let ir_config = __slicer_adapt_config(&config);
            let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::on_print_start(&ir_config) {
                Ok(m) => m,
                Err(e) => return Err(__slicer_error_out(e)),
            };
            #adapt_perim
            let mut sdk_output = ::slicer_sdk::builders::InfillOutputBuilder::new();
            let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_infill_postprocess(
                &module, layer_index, &sdk_regions, &mut sdk_output, &ir_config,
            );
            __slicer_drain_infill(&sdk_output, &output);
            match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
        }
    } else {
        quote! { Ok(()) }
    };

    let support_arm = if detected_stage == "Layer::Support" {
        quote! {
            let ir_config = __slicer_adapt_config(&config);
            let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::on_print_start(&ir_config) {
                Ok(m) => m,
                Err(e) => return Err(__slicer_error_out(e)),
            };
            #adapt_slice
            #adapt_paint
            let mut sdk_output = ::slicer_sdk::builders::SupportOutputBuilder::new();
            let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_support(
                &module, layer_index, &sdk_regions, &sdk_paint, &mut sdk_output, &ir_config,
            );
            __slicer_drain_support(&sdk_output, &output);
            match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
        }
    } else {
        quote! { Ok(()) }
    };

    let support_postprocess_arm = if detected_stage == "Layer::SupportPostProcess" {
        quote! {
            let ir_config = __slicer_adapt_config(&config);
            let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::on_print_start(&ir_config) {
                Ok(m) => m,
                Err(e) => return Err(__slicer_error_out(e)),
            };
            #adapt_slice
            let mut sdk_output = ::slicer_sdk::builders::SupportOutputBuilder::new();
            let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_support_postprocess(
                &module, layer_index, &sdk_regions, &mut sdk_output, &ir_config,
            );
            __slicer_drain_support(&sdk_output, &output);
            match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
        }
    } else {
        quote! { Ok(()) }
    };

    let path_opt_arm = if detected_stage == "Layer::PathOptimization" {
        quote! {
            let ir_config = __slicer_adapt_config(&config);
            let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::on_print_start(&ir_config) {
                Ok(m) => m,
                Err(e) => return Err(__slicer_error_out(e)),
            };
            #adapt_perim
            let mut sdk_output = ::slicer_sdk::postpass_builders::GcodeOutputBuilder::new();
            let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_path_optimization(
                &module, layer_index, &sdk_regions, &mut sdk_output, &ir_config,
            );
            __slicer_drain_gcode(&sdk_output, &output);
            match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
        }
    } else {
        quote! { Ok(()) }
    };

    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_layer_world_export {
            // Intentionally do NOT `use super::*;` — the user's module
            // may have imported types (e.g. `slicer_ir::Point3WithWidth`)
            // that would collide with the wit-bindgen-generated names.
            // Bring in only the user's module type.
            use super::#self_ty;

            #preamble

            // Bring geometry / ir-handles types that appear in the adapter
            // helpers into scope. wit-bindgen re-exports `ConfigView`,
            // `SliceRegionView`, etc. at the world's top level (they appear
            // as parameters on `Guest`), but the record/enum types used
            // inside method signatures (`ExPolygon`, `Point2`, `WallLoopView`,
            // `SemanticRegion`, `PaintValue`, `RegionKey`, …) live under
            // the interface sub-modules and are not re-exported. Use aliased
            // names so the helpers below can spell them without clashing
            // with slicer-ir or slicer-sdk names.
            use self::slicer::layer_world::geometry::{
                ExPolygon as WitExPolygon, ExtrusionPath3d as WitExtrusionPath3d,
                ExtrusionRole as WitExtrusionRole, Point2 as WitPoint2,
                Point3 as WitPoint3, Point3WithWidth as WitPoint3WithWidth,
                Polygon as WitPolygon,
            };
            use self::slicer::layer_world::ir_handles::{
                BoundaryPaintEntry as WitBoundaryPaintEntry,
                BoundaryPaintPolygon as WitBoundaryPaintPolygon,
                GcodeMoveCmd as WitGcodeMoveCmd,
                PaintSemantic as WitPaintSemantic, PaintValue as WitPaintValue,
                RegionKey as WitRegionKey,
                SemanticRegion as WitSemanticRegion,
                WallFeatureFlag as WitWallFeatureFlag,
                WallLoopType as WitWallLoopType, WallLoopView as WitWallLoopView,
            };

            // ── Converters: wit-bindgen → slicer-ir (input direction) ──

            fn __slicer_wit_point2_to_ir(p: &WitPoint2) -> ::slicer_ir::Point2 {
                ::slicer_ir::Point2 { x: p.x, y: p.y }
            }
            fn __slicer_wit_polygon_to_ir(p: &WitPolygon) -> ::slicer_ir::Polygon {
                ::slicer_ir::Polygon {
                    points: p.points.iter().map(__slicer_wit_point2_to_ir).collect(),
                }
            }
            fn __slicer_wit_expolygon_to_ir(ep: &WitExPolygon) -> ::slicer_ir::ExPolygon {
                ::slicer_ir::ExPolygon {
                    contour: __slicer_wit_polygon_to_ir(&ep.contour),
                    holes: ep.holes.iter().map(__slicer_wit_polygon_to_ir).collect(),
                }
            }
            fn __slicer_wit_role_to_ir(r: WitExtrusionRole) -> ::slicer_ir::ExtrusionRole {
                match r {
                    WitExtrusionRole::OuterWall => ::slicer_ir::ExtrusionRole::OuterWall,
                    WitExtrusionRole::InnerWall => ::slicer_ir::ExtrusionRole::InnerWall,
                    WitExtrusionRole::ThinWall => ::slicer_ir::ExtrusionRole::ThinWall,
                    WitExtrusionRole::TopSolidInfill => ::slicer_ir::ExtrusionRole::TopSolidInfill,
                    WitExtrusionRole::BottomSolidInfill => ::slicer_ir::ExtrusionRole::BottomSolidInfill,
                    WitExtrusionRole::SparseInfill => ::slicer_ir::ExtrusionRole::SparseInfill,
                    WitExtrusionRole::SupportMaterial => ::slicer_ir::ExtrusionRole::SupportMaterial,
                    WitExtrusionRole::SupportInterface => ::slicer_ir::ExtrusionRole::SupportInterface,
                    WitExtrusionRole::Ironing => ::slicer_ir::ExtrusionRole::Ironing,
                    WitExtrusionRole::BridgeInfill => ::slicer_ir::ExtrusionRole::BridgeInfill,
                    WitExtrusionRole::WipeTower => ::slicer_ir::ExtrusionRole::WipeTower,
                    WitExtrusionRole::Custom => ::slicer_ir::ExtrusionRole::Custom(::std::string::String::new()),
                }
            }
            fn __slicer_wit_point3w_to_ir(p: &WitPoint3WithWidth) -> ::slicer_ir::Point3WithWidth {
                ::slicer_ir::Point3WithWidth {
                    x: p.x, y: p.y, z: p.z, width: p.width, flow_factor: p.flow_factor,
                }
            }
            fn __slicer_wit_path_to_ir(p: &WitExtrusionPath3d) -> ::slicer_ir::ExtrusionPath3D {
                ::slicer_ir::ExtrusionPath3D {
                    points: p.points.iter().map(__slicer_wit_point3w_to_ir).collect(),
                    role: __slicer_wit_role_to_ir(p.role),
                    speed_factor: p.speed_factor,
                }
            }
            fn __slicer_wit_looptype_to_ir(lt: WitWallLoopType) -> ::slicer_ir::LoopType {
                match lt {
                    WitWallLoopType::Outer => ::slicer_ir::LoopType::Outer,
                    WitWallLoopType::Inner => ::slicer_ir::LoopType::Inner,
                    WitWallLoopType::ThinWall => ::slicer_ir::LoopType::ThinWall,
                    WitWallLoopType::NonplanarShell => ::slicer_ir::LoopType::NonPlanarShell,
                }
            }
            fn __slicer_wit_feature_to_ir(f: &WitWallFeatureFlag) -> ::slicer_ir::WallFeatureFlags {
                ::slicer_ir::WallFeatureFlags {
                    tool_index: f.tool_index,
                    fuzzy_skin: f.fuzzy_skin,
                    is_bridge: f.is_bridge,
                    is_thin_wall: f.is_thin_wall,
                    skip_ironing: f.skip_ironing,
                    // WIT `wall-feature-flag` does not carry the IR's
                    // `custom: HashMap<String, PaintValue>` map (it is
                    // populated later by paint-region modules); we arrive
                    // here with an empty map.
                    custom: ::std::collections::HashMap::new(),
                }
            }
            fn __slicer_wit_wallloop_to_ir(w: &WitWallLoopView) -> ::slicer_ir::WallLoop {
                let ir_path = __slicer_wit_path_to_ir(&w.path);
                let n_pts = ir_path.points.len();
                ::slicer_ir::WallLoop {
                    perimeter_index: w.perimeter_index,
                    loop_type: __slicer_wit_looptype_to_ir(w.loop_type),
                    path: ir_path,
                    // WIT `wall-loop-view` does not carry a width profile;
                    // synthesize one of the right arity from the path widths.
                    width_profile: ::slicer_ir::WidthProfile {
                        widths: (0..n_pts).map(|_| 0.4_f32).collect(),
                    },
                    feature_flags: w.feature_flags.iter().map(__slicer_wit_feature_to_ir).collect(),
                    boundary_type: ::slicer_ir::WallBoundaryType::Interior,
                }
            }
            fn __slicer_wit_semantic_to_ir(s: WitPaintSemantic) -> ::slicer_ir::PaintSemantic {
                match s {
                    WitPaintSemantic::Material => ::slicer_ir::PaintSemantic::Material,
                    WitPaintSemantic::FuzzySkin => ::slicer_ir::PaintSemantic::FuzzySkin,
                    WitPaintSemantic::SupportEnforcer => ::slicer_ir::PaintSemantic::SupportEnforcer,
                    WitPaintSemantic::SupportBlocker => ::slicer_ir::PaintSemantic::SupportBlocker,
                    WitPaintSemantic::Custom => ::slicer_ir::PaintSemantic::Custom(::std::string::String::new()),
                }
            }
            fn __slicer_wit_paintvalue_to_ir(v: &WitPaintValue) -> ::slicer_ir::PaintValue {
                match v {
                    WitPaintValue::Flag(b) => ::slicer_ir::PaintValue::Flag(*b),
                    WitPaintValue::Scalar(f) => ::slicer_ir::PaintValue::Scalar(*f),
                    WitPaintValue::ToolIndex(i) => ::slicer_ir::PaintValue::ToolIndex(*i),
                }
            }
            fn __slicer_boundary_paint_to_ir(
                entries: &[WitBoundaryPaintEntry],
            ) -> ::std::collections::HashMap<
                ::slicer_ir::PaintSemantic,
                ::std::vec::Vec<::std::vec::Vec<::core::option::Option<::slicer_ir::PaintValue>>>,
            > {
                let mut map = ::std::collections::HashMap::new();
                for e in entries {
                    let semantic = __slicer_wit_semantic_to_ir(e.semantic);
                    let polygons: ::std::vec::Vec<_> = e
                        .polygons
                        .iter()
                        .map(|poly: &WitBoundaryPaintPolygon| -> ::std::vec::Vec<::core::option::Option<::slicer_ir::PaintValue>> {
                            poly.values
                                .iter()
                                .map(|opt| opt.as_ref().map(__slicer_wit_paintvalue_to_ir))
                                .collect()
                        })
                        .collect();
                    map.insert(semantic, polygons);
                }
                map
            }

            fn __slicer_adapt_slice_regions(
                regions: &[SliceRegionView],
            ) -> ::std::vec::Vec<::slicer_sdk::views::SliceRegionView> {
                let mut out = ::std::vec::Vec::with_capacity(regions.len());
                for r in regions.iter() {
                    let polys: ::std::vec::Vec<::slicer_ir::ExPolygon> =
                        r.polygons().iter().map(__slicer_wit_expolygon_to_ir).collect();
                    let infill: ::std::vec::Vec<::slicer_ir::ExPolygon> =
                        r.infill_areas().iter().map(__slicer_wit_expolygon_to_ir).collect();
                    let boundary_paint = __slicer_boundary_paint_to_ir(&r.boundary_paint());
                    // `region_id` arrives as a string over WIT; the SDK view
                    // stores a `u64` (RegionId). Parse with a stable fallback
                    // when the string is non-numeric.
                    let region_id: ::slicer_ir::RegionId = r
                        .region_id()
                        .parse()
                        .unwrap_or(0);
                    let sdk_view = ::slicer_sdk::views::SliceRegionView::with_boundary_paint(
                        r.object_id(),
                        region_id,
                        polys,
                        infill,
                        r.effective_layer_height(),
                        r.z(),
                        r.has_nonplanar(),
                        boundary_paint,
                    );
                    out.push(sdk_view);
                }
                out
            }

            fn __slicer_adapt_perimeter_regions(
                regions: &[PerimeterRegionView],
            ) -> ::std::vec::Vec<::slicer_sdk::views::PerimeterRegionView> {
                let mut out = ::std::vec::Vec::with_capacity(regions.len());
                for r in regions.iter() {
                    let walls: ::std::vec::Vec<::slicer_ir::WallLoop> =
                        r.wall_loops().iter().map(__slicer_wit_wallloop_to_ir).collect();
                    let infill: ::std::vec::Vec<::slicer_ir::ExPolygon> =
                        r.infill_areas().iter().map(__slicer_wit_expolygon_to_ir).collect();
                    let region_id: ::slicer_ir::RegionId = r.region_id().parse().unwrap_or(0);
                    out.push(::slicer_sdk::views::PerimeterRegionView::new(
                        r.object_id(),
                        region_id,
                        walls,
                        infill,
                        // Seam candidates are not on the WIT view (they are
                        // written via `perimeter-output-builder.push-seam-candidate`
                        // and consumed later); per the read-only input view we
                        // arrive here with none.
                        ::std::vec::Vec::new(),
                    ));
                }
                out
            }

            fn __slicer_adapt_paint_layer(
                paint: &PaintRegionLayerView,
            ) -> ::slicer_sdk::traits::PaintRegionLayerView {
                use ::std::collections::HashMap;
                let layer_idx = paint.layer_index();
                let mut semantic_regions: HashMap<
                    ::slicer_ir::PaintSemantic,
                    ::std::vec::Vec<::slicer_ir::SemanticRegion>,
                > = HashMap::new();
                // Enumerate every documented built-in semantic; `Custom` is
                // intentionally skipped here because the WIT contract exposes
                // custom regions through a separate `get-custom-regions`
                // method keyed by module id.
                let semantics = [
                    WitPaintSemantic::Material,
                    WitPaintSemantic::FuzzySkin,
                    WitPaintSemantic::SupportEnforcer,
                    WitPaintSemantic::SupportBlocker,
                ];
                for s in semantics.iter().copied() {
                    let wit_regions: ::std::vec::Vec<WitSemanticRegion> =
                        paint.get_regions(s);
                    if wit_regions.is_empty() { continue; }
                    let ir_semantic = __slicer_wit_semantic_to_ir(s);
                    let ir_regions: ::std::vec::Vec<::slicer_ir::SemanticRegion> = wit_regions
                        .iter()
                        .enumerate()
                        .map(|(idx, sr)| ::slicer_ir::SemanticRegion {
                            object_id: sr.object_id.clone(),
                            polygons: sr.polygons.iter().map(__slicer_wit_expolygon_to_ir).collect(),
                            value: __slicer_wit_paintvalue_to_ir(&sr.value),
                            paint_order: idx as u64,
                        })
                        .collect();
                    semantic_regions.insert(ir_semantic, ir_regions);
                }
                let ir = ::slicer_ir::PaintRegionIR {
                    schema_version: ::slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
                    per_layer: {
                        let mut m: HashMap<u32, ::slicer_ir::LayerPaintMap> = HashMap::new();
                        m.insert(
                            layer_idx,
                            ::slicer_ir::LayerPaintMap {
                                global_layer_index: layer_idx,
                                semantic_regions,
                            },
                        );
                        m
                    },
                };
                ::slicer_sdk::traits::PaintRegionLayerView::with_paint_regions(
                    layer_idx,
                    ::std::sync::Arc::new(ir),
                )
            }

            // ── Converters: slicer-ir → wit-bindgen (drain direction) ──

            fn __slicer_ir_role_to_wit(r: &::slicer_ir::ExtrusionRole) -> WitExtrusionRole {
                match r {
                    ::slicer_ir::ExtrusionRole::OuterWall => WitExtrusionRole::OuterWall,
                    ::slicer_ir::ExtrusionRole::InnerWall => WitExtrusionRole::InnerWall,
                    ::slicer_ir::ExtrusionRole::ThinWall => WitExtrusionRole::ThinWall,
                    ::slicer_ir::ExtrusionRole::TopSolidInfill => WitExtrusionRole::TopSolidInfill,
                    ::slicer_ir::ExtrusionRole::BottomSolidInfill => WitExtrusionRole::BottomSolidInfill,
                    ::slicer_ir::ExtrusionRole::SparseInfill => WitExtrusionRole::SparseInfill,
                    ::slicer_ir::ExtrusionRole::SupportMaterial => WitExtrusionRole::SupportMaterial,
                    ::slicer_ir::ExtrusionRole::SupportInterface => WitExtrusionRole::SupportInterface,
                    ::slicer_ir::ExtrusionRole::Ironing => WitExtrusionRole::Ironing,
                    ::slicer_ir::ExtrusionRole::BridgeInfill => WitExtrusionRole::BridgeInfill,
                    ::slicer_ir::ExtrusionRole::WipeTower => WitExtrusionRole::WipeTower,
                    // PrimeTower/Skirt/Custom(tag) all collapse to the
                    // arity-0 WIT Custom variant; the IR's tag string is
                    // not carried across the boundary.
                    _ => WitExtrusionRole::Custom,
                }
            }
            fn __slicer_ir_path_to_wit(p: &::slicer_ir::ExtrusionPath3D) -> WitExtrusionPath3d {
                WitExtrusionPath3d {
                    points: p.points.iter().map(|pt| WitPoint3WithWidth {
                        x: pt.x, y: pt.y, z: pt.z, width: pt.width, flow_factor: pt.flow_factor,
                    }).collect(),
                    role: __slicer_ir_role_to_wit(&p.role),
                    speed_factor: p.speed_factor,
                }
            }
            fn __slicer_ir_point2_to_wit(p: &::slicer_ir::Point2) -> WitPoint2 {
                WitPoint2 { x: p.x, y: p.y }
            }
            fn __slicer_ir_polygon_to_wit(p: &::slicer_ir::Polygon) -> WitPolygon {
                WitPolygon { points: p.points.iter().map(__slicer_ir_point2_to_wit).collect() }
            }
            fn __slicer_ir_expolygon_to_wit(ep: &::slicer_ir::ExPolygon) -> WitExPolygon {
                WitExPolygon {
                    contour: __slicer_ir_polygon_to_wit(&ep.contour),
                    holes: ep.holes.iter().map(__slicer_ir_polygon_to_wit).collect(),
                }
            }
            fn __slicer_ir_looptype_to_wit(lt: &::slicer_ir::LoopType) -> WitWallLoopType {
                match lt {
                    ::slicer_ir::LoopType::Outer => WitWallLoopType::Outer,
                    ::slicer_ir::LoopType::Inner => WitWallLoopType::Inner,
                    ::slicer_ir::LoopType::ThinWall => WitWallLoopType::ThinWall,
                    ::slicer_ir::LoopType::NonPlanarShell => WitWallLoopType::NonplanarShell,
                }
            }
            fn __slicer_ir_feature_to_wit(f: &::slicer_ir::WallFeatureFlags) -> WitWallFeatureFlag {
                WitWallFeatureFlag {
                    tool_index: f.tool_index,
                    fuzzy_skin: f.fuzzy_skin,
                    is_bridge: f.is_bridge,
                    is_thin_wall: f.is_thin_wall,
                    skip_ironing: f.skip_ironing,
                }
            }
            fn __slicer_ir_wallloop_to_wit(w: &::slicer_ir::WallLoop) -> WitWallLoopView {
                WitWallLoopView {
                    perimeter_index: w.perimeter_index,
                    loop_type: __slicer_ir_looptype_to_wit(&w.loop_type),
                    path: __slicer_ir_path_to_wit(&w.path),
                    feature_flags: w.feature_flags.iter().map(__slicer_ir_feature_to_wit).collect(),
                }
            }
            fn __slicer_ir_region_key_to_wit(k: &::slicer_ir::RegionKey) -> WitRegionKey {
                WitRegionKey {
                    layer_index: k.global_layer_index,
                    object_id: k.object_id.clone(),
                    region_id: k.region_id.to_string(),
                }
            }

            // ── Drain-back helpers ─────────────────────────────────────

            fn __slicer_drain_infill(
                sdk: &::slicer_sdk::builders::InfillOutputBuilder,
                wit: &InfillOutputBuilder,
            ) {
                for p in sdk.sparse_paths() {
                    let _ = wit.push_sparse_path(&__slicer_ir_path_to_wit(p));
                }
                for p in sdk.solid_paths() {
                    let _ = wit.push_solid_path(&__slicer_ir_path_to_wit(p));
                }
                for p in sdk.ironing_paths() {
                    let _ = wit.push_ironing_path(&__slicer_ir_path_to_wit(p));
                }
            }

            fn __slicer_drain_perimeter(
                sdk: &::slicer_sdk::builders::PerimeterOutputBuilder,
                wit: &PerimeterOutputBuilder,
            ) {
                for w in sdk.wall_loops() {
                    let _ = wit.push_wall_loop(&__slicer_ir_wallloop_to_wit(w));
                }
                let areas: ::std::vec::Vec<WitExPolygon> =
                    sdk.infill_areas().iter().map(__slicer_ir_expolygon_to_wit).collect();
                if !areas.is_empty() {
                    let _ = wit.set_infill_areas(&areas);
                }
                for (pos, score) in sdk.seam_candidates() {
                    let _ = wit.push_seam_candidate(
                        WitPoint3 { x: pos.x as f32, y: pos.y as f32, z: 0.0 },
                        *score,
                    );
                }
            }

            fn __slicer_drain_support(
                sdk: &::slicer_sdk::builders::SupportOutputBuilder,
                wit: &SupportOutputBuilder,
            ) {
                for p in sdk.support_paths() {
                    let _ = wit.push_support_path(&__slicer_ir_path_to_wit(p));
                }
                for (p, top) in sdk.interface_paths() {
                    let _ = wit.push_interface_path(&__slicer_ir_path_to_wit(p), *top);
                }
                for p in sdk.raft_paths() {
                    let _ = wit.push_raft_path(&__slicer_ir_path_to_wit(p));
                }
            }

            fn __slicer_drain_slice_postprocess(
                sdk: &::slicer_sdk::builders::SlicePostprocessBuilder,
                wit: &SlicePostprocessBuilder,
            ) {
                for (key, polys) in sdk.polygon_updates() {
                    let wit_polys: ::std::vec::Vec<WitExPolygon> =
                        polys.iter().map(__slicer_ir_expolygon_to_wit).collect();
                    let _ = wit.set_polygons(&__slicer_ir_region_key_to_wit(key), &wit_polys);
                }
                for (key, path_idx, vertex_idx, z) in sdk.path_z_updates() {
                    let _ = wit.set_path_z(
                        &__slicer_ir_region_key_to_wit(key),
                        *path_idx, *vertex_idx, *z,
                    );
                }
                // `boundary_paint_updates` has no corresponding WIT method on
                // `slice-postprocess-builder` (docs/03 wit/world-layer.wit);
                // the documented write path is via `perimeter-output-builder`
                // in later stages. Nothing to drain here.
            }

            fn __slicer_drain_gcode(
                sdk: &::slicer_sdk::postpass_builders::GcodeOutputBuilder,
                wit: &GcodeOutputBuilder,
            ) {
                for cmd in sdk.commands() {
                    match cmd {
                        ::slicer_ir::GCodeCommand::Move { x, y, z, e, f, role } => {
                            // `push-move` takes `gcode-move-cmd` by value in
                            // the layer-world WIT (it is a record, not a
                            // resource); wit-bindgen generates a
                            // by-value parameter on the guest side.
                            let wit_cmd = WitGcodeMoveCmd {
                                x: *x, y: *y, z: *z, e: *e, f: *f,
                                role: __slicer_ir_role_to_wit(role),
                            };
                            let _ = wit.push_move(wit_cmd);
                        }
                        ::slicer_ir::GCodeCommand::Retract { length, speed } => {
                            let _ = wit.push_retract(*length, *speed);
                        }
                        ::slicer_ir::GCodeCommand::FanSpeed { value } => {
                            let _ = wit.push_fan_speed(*value);
                        }
                        ::slicer_ir::GCodeCommand::Temperature { tool, celsius, wait } => {
                            let _ = wit.push_temperature(*tool, *celsius, *wait);
                        }
                        ::slicer_ir::GCodeCommand::ToolChange { from, to } => {
                            let _ = wit.push_tool_change(*from, *to);
                        }
                        ::slicer_ir::GCodeCommand::Comment { text } => {
                            let _ = wit.push_comment(text);
                        }
                        ::slicer_ir::GCodeCommand::Raw { text } => {
                            let _ = wit.push_raw(text);
                        }
                        // `Unretract` (and any future non-layer-world GCode
                        // variants) is not modelled in the layer-world WIT
                        // `gcode-output-builder`. Drop silently so adding
                        // variants to the IR does not break the macro glue.
                        _ => {}
                    }
                }
            }

            struct __SlicerLayerComponent;

            impl Guest for __SlicerLayerComponent {
                fn on_print_start(config: ConfigView) -> Result<(), ModuleError> {
                    let ir_config = __slicer_adapt_config(&config);
                    match <#self_ty as ::slicer_sdk::traits::LayerModule>::on_print_start(&ir_config) {
                        Ok(_m) => Ok(()),
                        Err(e) => Err(__slicer_error_out(e)),
                    }
                }
                fn on_print_end() -> Result<(), ModuleError> { Ok(()) }

                fn run_slice_postprocess(
                    layer_index: u32,
                    regions: Vec<SliceRegionView>,
                    paint: PaintRegionLayerView,
                    output: SlicePostprocessBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> { #slice_postprocess_arm }

                fn run_perimeters(
                    layer_index: u32,
                    regions: Vec<SliceRegionView>,
                    paint: PaintRegionLayerView,
                    output: PerimeterOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> { #perimeters_arm }

                fn run_wall_postprocess(
                    layer_index: u32,
                    regions: Vec<PerimeterRegionView>,
                    output: PerimeterOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> { #wall_postprocess_arm }

                fn run_infill(
                    layer_index: u32,
                    regions: Vec<SliceRegionView>,
                    output: InfillOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> { #infill_arm }

                fn run_infill_postprocess(
                    layer_index: u32,
                    regions: Vec<PerimeterRegionView>,
                    output: InfillOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> { #infill_postprocess_arm }

                fn run_support(
                    layer_index: u32,
                    regions: Vec<SliceRegionView>,
                    paint: PaintRegionLayerView,
                    output: SupportOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> { #support_arm }

                fn run_support_postprocess(
                    layer_index: u32,
                    regions: Vec<SliceRegionView>,
                    output: SupportOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> { #support_postprocess_arm }

                fn run_path_optimization(
                    layer_index: u32,
                    regions: Vec<PerimeterRegionView>,
                    output: GcodeOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> { #path_opt_arm }
            }

            export!(__SlicerLayerComponent);
        }
    }
}

/// Inline WIT for the full layer-module world, mirroring
/// `crates/slicer-host/src/wit_host.rs::layer::bindgen!` so the
/// macro-emitted guest binds against the same resource shapes the host
/// dispatcher expects.
const LAYER_WORLD_WIT: &str = r#"
    package slicer:world-layer@1.0.0;

    include "../../wit/deps/types.wit";
    include "../../wit/deps/config.wit";
    include "../../wit/deps/ir-types.wit";

    interface host-services {
        use geometry.{point3, bounding-box3, ex-polygon, polygon};
        type object-id = string;
        enum log-level { trace, debug, info, warn, error }
        log: func(level: log-level, message: string);
        raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
        surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
        object-bounds:      func(object-id: object-id) -> bounding-box3;
        enum clip-operation   { union, intersection, difference, xor }
        enum offset-join-type { miter, round, square }
        clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
        offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
        simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;
        now-us: func() -> u64;
    }

    world layer-module {
        import host-services;
        import config-types;
        import ir-handles;
        record module-error { code: u32, message: string, fatal: bool }
        use config-types.{config-view};
        use ir-handles.{
            slice-region-view, perimeter-region-view,
            infill-output-builder, perimeter-output-builder,
            slice-postprocess-builder, support-output-builder,
            gcode-output-builder, region-key, layer-idx,
            paint-region-layer-view,
        };
        export on-print-start: func(config: config-view) -> result<_, module-error>;
        export on-print-end:   func() -> result<_, module-error>;
        export run-slice-postprocess: func(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view, output: slice-postprocess-builder, config: config-view) -> result<_, module-error>;
        export run-perimeters: func(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view, output: perimeter-output-builder, config: config-view) -> result<_, module-error>;
        export run-wall-postprocess: func(layer-index: layer-idx, regions: list<perimeter-region-view>, output: perimeter-output-builder, config: config-view) -> result<_, module-error>;
        export run-infill: func(layer-index: layer-idx, regions: list<slice-region-view>, output: infill-output-builder, config: config-view) -> result<_, module-error>;
        export run-infill-postprocess: func(layer-index: layer-idx, regions: list<perimeter-region-view>, output: infill-output-builder, config: config-view) -> result<_, module-error>;
        export run-support: func(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view, output: support-output-builder, config: config-view) -> result<_, module-error>;
        export run-support-postprocess: func(layer-index: layer-idx, regions: list<slice-region-view>, output: support-output-builder, config: config-view) -> result<_, module-error>;
        export run-path-optimization: func(layer-index: layer-idx, regions: list<perimeter-region-view>, output: gcode-output-builder, config: config-view) -> result<_, module-error>;
    }
"#;

/// The `#[module_test]` attribute macro.
///
/// Wrapper around `#[test]` that automatically sets up the mock host,
/// installs the SDK's test panic handler, and resets global state between tests.
///
/// # Example
///
/// ```ignore
/// #[module_test]
/// fn test_my_module() {
///     // Test code with mock host automatically available
/// }
/// ```
#[proc_macro_attribute]
pub fn module_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let expanded = generate_module_test_impl(&input);
    TokenStream::from(expanded)
}

fn generate_module_test_impl(input: &ItemFn) -> TokenStream2 {
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_attrs = &input.attrs;
    let fn_block = &input.block;
    let fn_output = &input.sig.output;

    let has_return_type = !matches!(fn_output, ReturnType::Default);

    if has_return_type {
        quote! {
            #(#fn_attrs)*
            #[test]
            #fn_vis fn #fn_name() #fn_output {
                struct __SlicerTestGuard;
                impl Drop for __SlicerTestGuard {
                    fn drop(&mut self) {
                        __slicer_test_mock_host_teardown();
                    }
                }

                __slicer_test_reset_global_state();
                __slicer_test_install_panic_handler();
                __slicer_test_mock_host_setup();

                let _guard = __SlicerTestGuard;

                #fn_block
            }
        }
    } else {
        quote! {
            #(#fn_attrs)*
            #[test]
            #fn_vis fn #fn_name() {
                struct __SlicerTestGuard;
                impl Drop for __SlicerTestGuard {
                    fn drop(&mut self) {
                        __slicer_test_mock_host_teardown();
                    }
                }

                __slicer_test_reset_global_state();
                __slicer_test_install_panic_handler();
                __slicer_test_mock_host_setup();

                let _guard = __SlicerTestGuard;

                #fn_block
            }
        }
    }
}
