//! Proc-macros for the Pinch 'n Print SDK.
//!
//! This crate provides:
//! - `#[slicer_module]` — promotes `impl LayerModule for T` / `impl PrepassModule for T`
//!   / `impl FinalizationModule for T` / `impl PostpassModule for T` into a
//!   binding-schema surface that matches the documented WIT worlds under
//!   `wit/deps/<pkg>/<pkg>.wit` (docs/03, docs/05).
//! - `#[module_test]` — test wrapper with mock host setup.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemImpl, ReturnType};

// Stage/world/export table is centralised in `slicer-schema` so
// `#[slicer_module]` and `slicer-cli::cmd_new` stay in lock-step and
// drift between the macro-emitted binding and generated manifests is
// structurally impossible (docs/03, docs/05).
use slicer_schema::{StageSpec, STAGES};

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
                stage_world = stage.tier_id,
                expected_trait = stage.trait_name,
                got = trait_name,
                got_world = tier_for_trait(trait_name).unwrap_or("<unknown>"),
            );
            return syn::Error::new_spanned(&input.self_ty, msg)
                .to_compile_error()
                .into();
        }
    }

    let expanded =
        generate_slicer_module_impl(&input, &self_ty, &detected_stages, trait_ident.as_deref());
    TokenStream::from(expanded)
}

/// Returns true when the SDK trait name is one the macro knows about.
fn is_known_trait(name: &str) -> bool {
    matches!(
        name,
        "LayerModule" | "PrepassModule" | "FinalizationModule" | "PostpassModule"
    )
}

/// Map SDK trait name → WIT world name.
///
/// Delegates to `slicer-schema`, which owns the world names. This used to
/// be a hand-copied duplicate of that table; the copies drifted apart on
/// every edit and had to be re-synced by hand.
fn tier_for_trait(trait_name: &str) -> Option<&'static str> {
    slicer_schema::tier_for_trait(trait_name)
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

    let (
        stage_id_literal,
        stage_method_literal,
        stage_export_name_literal,
        stage_export_literal,
        stage_world_literal,
    ) = if let Some(s) = detected.first() {
        let qualified_export = slicer_schema::qualified_export_for_stage_id(s.stage_id)
            .unwrap_or_else(|| s.wit_export.to_string());
        (
            s.stage_id,
            s.method,
            s.wit_export,
            qualified_export,
            s.tier_id,
        )
    } else {
        ("", "", "", String::new(), "")
    };

    // Choose effective WIT world: prefer the trait's world if the trait
    // is known, else the detected stage's world, else empty.
    let effective_tier = trait_ident
        .and_then(tier_for_trait)
        .unwrap_or(stage_world_literal);

    let trait_name_literal = trait_ident.unwrap_or("");

    let wit_exports = if stage_export_literal.is_empty() {
        Vec::new()
    } else {
        vec![stage_export_literal.clone()]
    };
    let wit_exports_tokens = wit_exports.iter().map(|e| quote! { #e });

    let native_entry_tokens = if stage_id_literal.starts_with("Layer::") {
        let body = match stage_method_literal {
            "run_infill" => quote! {
                let module = <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&req.config)?;
                let paint = req.paint.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native layer request is missing paint".to_string()))?;
                let mut output = ::slicer_sdk::builders::InfillOutputBuilder::new();
                <#self_ty as ::slicer_sdk::traits::LayerModule>::run_infill(
                    &module, req.layer_index, &req.regions, paint, &mut output, &req.config,
                )?;
                Ok(::slicer_sdk::native::NativeLayerResponse {
                      infill: Some(output), perimeters: None, support: None, slice_postprocess: None, path_optimization: None, anchored_events: None,
                })
            },
            "run_perimeters" => quote! {
                let module = <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&req.config)?;
                let paint = req.paint.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native layer request is missing paint".to_string()))?;
                let mut output = ::slicer_sdk::builders::PerimeterOutputBuilder::new();
                <#self_ty as ::slicer_sdk::traits::LayerModule>::run_perimeters(
                    &module, req.layer_index, &req.regions, paint, &mut output, &req.config,
                )?;
                Ok(::slicer_sdk::native::NativeLayerResponse {
                      infill: None, perimeters: Some(output), support: None, slice_postprocess: None, path_optimization: None, anchored_events: None,
                })
            },
            "run_wall_postprocess" => quote! {
                let module = <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&req.config)?;
                let regions = req.perimeter_regions.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native layer request is missing perimeter regions".to_string()))?;
                let mut output = ::slicer_sdk::builders::PerimeterOutputBuilder::new();
                <#self_ty as ::slicer_sdk::traits::LayerModule>::run_wall_postprocess(
                    &module, req.layer_index, regions, &mut output, &req.config,
                )?;
                Ok(::slicer_sdk::native::NativeLayerResponse {
                      infill: None, perimeters: Some(output), support: None, slice_postprocess: None, path_optimization: None, anchored_events: None,
                })
            },
            "run_infill_postprocess" => quote! {
                let module = <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&req.config)?;
                let regions = req.perimeter_regions.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native layer request is missing perimeter regions".to_string()))?;
                let prior = req.prior_infill.as_deref().unwrap_or(&[]);
                let mut output = ::slicer_sdk::builders::InfillOutputBuilder::new();
                <#self_ty as ::slicer_sdk::traits::LayerModule>::run_infill_postprocess(
                    &module, req.layer_index, regions, prior, &mut output, &req.config,
                )?;
                Ok(::slicer_sdk::native::NativeLayerResponse {
                      infill: Some(output), perimeters: None, support: None, slice_postprocess: None, path_optimization: None, anchored_events: None,
                })
            },
            "run_slice_postprocess" => quote! {
                let module = <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&req.config)?;
                let paint = req.paint.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native layer request is missing paint".to_string()))?;
                let mut output = ::slicer_sdk::builders::SlicePostprocessBuilder::new();
                <#self_ty as ::slicer_sdk::traits::LayerModule>::run_slice_postprocess(
                    &module, req.layer_index, &req.regions, paint, &mut output, &req.config,
                )?;
                Ok(::slicer_sdk::native::NativeLayerResponse {
                      infill: None, perimeters: None, support: None, slice_postprocess: Some(output), path_optimization: None, anchored_events: None,
                })
            },
            "run_support" => quote! {
                let module = <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&req.config)?;
                let paint = req.paint.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native layer request is missing paint".to_string()))?;
                let mut output = ::slicer_sdk::builders::SupportOutputBuilder::new();
                let mut collection = ::slicer_sdk::layer_collection_builder::LayerCollectionBuilder::new();
                <#self_ty as ::slicer_sdk::traits::LayerModule>::run_support(
                    &module, req.layer_index, &req.regions, paint, &mut output, &mut collection, &req.config,
                )?;
                Ok(::slicer_sdk::native::NativeLayerResponse {
                      infill: None, perimeters: None, support: Some(::slicer_sdk::native::NativeSupportOutput { output, collection }), slice_postprocess: None, path_optimization: None, anchored_events: None,
                })
            },
            "run_support_postprocess" => quote! {
                let module = <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&req.config)?;
                let mut output = ::slicer_sdk::builders::SupportOutputBuilder::new();
                <#self_ty as ::slicer_sdk::traits::LayerModule>::run_support_postprocess(
                    &module, req.layer_index, &req.regions, &mut output, &req.config,
                )?;
                Ok(::slicer_sdk::native::NativeLayerResponse {
                      infill: None, perimeters: None, support: Some(::slicer_sdk::native::NativeSupportOutput { output, collection: ::slicer_sdk::layer_collection_builder::LayerCollectionBuilder::new() }), slice_postprocess: None, path_optimization: None, anchored_events: None,
                })
            },
            "run_path_optimization" => quote! {
                let module = <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&req.config)?;
                let regions = req.perimeter_regions.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native layer request is missing perimeter regions".to_string()))?;
                let mut output = ::slicer_sdk::postpass_builders::GcodeOutputBuilder::new();
                let mut collection = ::slicer_sdk::layer_collection_builder::LayerCollectionBuilder::new();
                <#self_ty as ::slicer_sdk::traits::LayerModule>::run_path_optimization(
                    &module, req.layer_index, regions, &mut output, &mut collection, &req.config,
                )?;
                Ok(::slicer_sdk::native::NativeLayerResponse {
                     infill: None, perimeters: None, support: None, slice_postprocess: None,
                     path_optimization: Some(::slicer_sdk::native::NativePathOptimizationOutput { output, collection }),
                     anchored_events: None,
                })
            },
            "run_anchored_events" => quote! {
                let module = <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&req.config)?;
                let regions = req.perimeter_regions.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native layer request is missing perimeter regions".to_string()))?;
                let mut collection = ::slicer_sdk::layer_collection_builder::LayerCollectionBuilder::new();
                <#self_ty as ::slicer_sdk::traits::LayerModule>::run_anchored_events(
                    &module, req.layer_index, regions, &mut collection, &req.config,
                )?;
                Ok(::slicer_sdk::native::NativeLayerResponse {
                     infill: None, perimeters: None, support: None, slice_postprocess: None,
                     path_optimization: None, anchored_events: Some(collection),
                })
            },
            _ => quote! { unreachable!() },
        };
        quote! {
            #[cfg(all(not(target_arch = "wasm32"), not(test)))]
            #[doc(hidden)]
            pub fn __slicer_native_entry() -> ::slicer_sdk::native::NativeStageEntry {
                ::slicer_sdk::native::NativeStageEntry::Layer(Self::__slicer_native_layer_entry)
            }

            #[cfg(all(not(target_arch = "wasm32"), not(test)))]
            fn __slicer_native_layer_entry(
                req: &::slicer_sdk::native::NativeLayerRequest,
            ) -> ::std::result::Result<::slicer_sdk::native::NativeLayerResponse, ::slicer_sdk::error::ModuleError> {
                #body
            }
        }
    } else if stage_id_literal.starts_with("PrePass::") {
        let body = match stage_method_literal {
            "run_mesh_analysis" => quote! {
                let objects = req.object_ids.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native prepass request is missing object ids".to_string()))?;
                let module = <#self_ty as ::slicer_sdk::traits::PrepassModule>::from_config(&req.config)?;
                let mut output = ::slicer_sdk::prepass_builders::MeshAnalysisOutput::new();
                <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_mesh_analysis(&module, objects, &mut output, &req.config)?;
                Ok(::slicer_sdk::native::NativePrepassResponse { mesh_analysis: Some(output), layer_plan: None, paint_segmentation: None, seam_planning: None, support_geometry: None })
            },
            "run_layer_planning" => quote! {
                let objects = req.object_ids.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native prepass request is missing object ids".to_string()))?;
                let module = <#self_ty as ::slicer_sdk::traits::PrepassModule>::from_config(&req.config)?;
                let mut output = ::slicer_sdk::prepass_builders::LayerPlanOutput::new();
                <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_layer_planning(&module, objects, &mut output, &req.config)?;
                Ok(::slicer_sdk::native::NativePrepassResponse { mesh_analysis: None, layer_plan: Some(output), paint_segmentation: None, seam_planning: None, support_geometry: None })
            },
            "run_seam_planning" => quote! {
                let objects = req.mesh_objects.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native prepass request is missing mesh objects".to_string()))?;
                let layer_plan = req.layer_plan.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native prepass request is missing layer plan".to_string()))?;
                let regions = req.seam_regions.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native prepass request is missing seam regions".to_string()))?;
                let module = <#self_ty as ::slicer_sdk::traits::PrepassModule>::from_config(&req.config)?;
                let mut output = ::slicer_sdk::prepass_builders::SeamPlanningOutput::new();
                <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_seam_planning(&module, objects, layer_plan, &mut output, &req.config, regions)?;
                Ok(::slicer_sdk::native::NativePrepassResponse { mesh_analysis: None, layer_plan: None, paint_segmentation: None, seam_planning: Some(output), support_geometry: None })
            },
            "run_support_geometry" => quote! {
                let objects = req.mesh_objects.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native prepass request is missing mesh objects".to_string()))?;
                let layer_plan = req.layer_plan.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native prepass request is missing layer plan".to_string()))?;
                let regions = req.region_segmentation.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native prepass request is missing region segmentation".to_string()))?;
                let analysis = req.support_analysis.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native prepass request is missing support analysis".to_string()))?;
                let support = req.support_geometry.as_ref().ok_or_else(|| ::slicer_sdk::error::ModuleError::fatal(1, "native prepass request is missing support geometry".to_string()))?;
                let module = <#self_ty as ::slicer_sdk::traits::PrepassModule>::from_config(&req.config)?;
                let mut output = ::slicer_sdk::prepass_builders::SupportGeometryOutput::new();
                <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_support_geometry_with_analysis(&module, objects, layer_plan, regions, analysis, support, &mut output, &req.config)?;
                Ok(::slicer_sdk::native::NativePrepassResponse { mesh_analysis: None, layer_plan: None, paint_segmentation: None, seam_planning: None, support_geometry: Some(output) })
            },
            _ => quote! { unreachable!() },
        };
        quote! {
            #[cfg(all(not(target_arch = "wasm32"), not(test)))]
            #[doc(hidden)]
            pub fn __slicer_native_entry() -> ::slicer_sdk::native::NativeStageEntry {
                ::slicer_sdk::native::NativeStageEntry::Prepass(Self::__slicer_native_prepass_entry)
            }
            #[cfg(all(not(target_arch = "wasm32"), not(test)))]
            fn __slicer_native_prepass_entry(req: &::slicer_sdk::native::NativePrepassRequest) -> ::std::result::Result<::slicer_sdk::native::NativePrepassResponse, ::slicer_sdk::error::ModuleError> {
                #body
            }
        }
    } else if stage_id_literal.starts_with("PostPass::")
        && stage_id_literal != "PostPass::LayerFinalization"
    {
        let body = if stage_method_literal == "run_text_postprocess" {
            quote! {
                let text = match &req.input { ::slicer_sdk::native::NativePostpassInput::Text(text) => text, _ => return Err(::slicer_sdk::error::ModuleError::fatal(1, "native postpass request has the wrong input variant".to_string())) };
                let module = <#self_ty as ::slicer_sdk::traits::PostpassModule>::from_config(&req.config)?;
                let output = <#self_ty as ::slicer_sdk::traits::PostpassModule>::run_text_postprocess(&module, text, &req.config)?;
                Ok(::slicer_sdk::native::NativePostpassResponse::Text(output))
            }
        } else {
            quote! {
                let commands = match &req.input { ::slicer_sdk::native::NativePostpassInput::Gcode(commands) => commands, _ => return Err(::slicer_sdk::error::ModuleError::fatal(1, "native postpass request has the wrong input variant".to_string())) };
                let module = <#self_ty as ::slicer_sdk::traits::PostpassModule>::from_config(&req.config)?;
                let mut output = ::slicer_sdk::postpass_builders::GcodeOutputBuilder::new();
                <#self_ty as ::slicer_sdk::traits::PostpassModule>::run_gcode_postprocess(&module, commands, &mut output, &req.config)?;
                Ok(::slicer_sdk::native::NativePostpassResponse::Gcode(output.commands().to_vec()))
            }
        };
        quote! {
            #[cfg(all(not(target_arch = "wasm32"), not(test)))]
            #[doc(hidden)]
            pub fn __slicer_native_entry() -> ::slicer_sdk::native::NativeStageEntry {
                ::slicer_sdk::native::NativeStageEntry::Postpass(Self::__slicer_native_postpass_entry)
            }
            #[cfg(all(not(target_arch = "wasm32"), not(test)))]
            fn __slicer_native_postpass_entry(req: &::slicer_sdk::native::NativePostpassRequest) -> ::std::result::Result<::slicer_sdk::native::NativePostpassResponse, ::slicer_sdk::error::ModuleError> {
                #body
            }
        }
    } else if stage_id_literal == "PostPass::LayerFinalization" {
        quote! {
            #[cfg(all(not(target_arch = "wasm32"), not(test)))]
            #[doc(hidden)]
            pub fn __slicer_native_entry() -> ::slicer_sdk::native::NativeStageEntry {
                ::slicer_sdk::native::NativeStageEntry::Finalization(Self::__slicer_native_finalization_entry)
            }
            #[cfg(all(not(target_arch = "wasm32"), not(test)))]
            fn __slicer_native_finalization_entry(req: &::slicer_sdk::native::NativeFinalizationRequest) -> ::std::result::Result<::slicer_sdk::native::NativeFinalizationResponse, ::slicer_sdk::error::ModuleError> {
                let module = <#self_ty as ::slicer_sdk::traits::FinalizationModule>::from_config(&req.config)?;
                let mut output = ::slicer_sdk::traits::FinalizationOutputBuilder::new();
                <#self_ty as ::slicer_sdk::traits::FinalizationModule>::run_finalization(&module, &req.layers, &mut output, &req.config)?;
                Ok(::slicer_sdk::native::NativeFinalizationResponse { output })
            }
        }
    } else {
        quote! {}
    };

    let stage_binding_tokens: TokenStream2 = if stage_export_literal.is_empty() {
        quote! {}
    } else {
        quote! {
            ::slicer_schema::ExportBinding {
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
        r#"{{"type":"{ty}","trait":"{tr}","tier":"{tier}","stage_id":"{stage}","stage_method":"{method}","stage_export":"{export}","wit_exports":[{exports}]}}"#,
        ty = type_name_str.replace('"', "\\\""),
        tr = trait_name_literal,
        tier = effective_tier,
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
            /// `slicer_schema::TIER_LAYER`) or "" if the impl targets
            /// an unknown trait and no stage was detected.
            #[doc(hidden)]
            pub fn __slicer_tier_id() -> &'static str { #effective_tier }

            /// Name of the SDK trait the impl targets, or "" if the
            /// macro was applied to an inherent impl.
            #[doc(hidden)]
            pub fn __slicer_trait_name() -> &'static str { #trait_name_literal }

            /// Local WIT export name for the detected stage, e.g. `"run"`,
            /// or "" if no stage method was detected.
            #[doc(hidden)]
            pub fn __slicer_stage_export_name() -> &'static str { #stage_export_name_literal }

            /// Rust-cased name of the detected stage method, e.g.
            /// `"run_infill"`, or "" if no stage method was detected.
            #[doc(hidden)]
            pub fn __slicer_stage_method_name() -> &'static str { #stage_method_literal }

            /// The full list of WIT export names this module provides:
            /// the detected stage export, if any.
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
                    tier_id: #effective_tier,
                    stage_id: #stage_id_literal,
                    stage_method: #stage_method_literal,
                    stage_export: #stage_export_literal,
                    exports: &[
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

            #native_entry_tokens

        }
    };

    // ── wasm32-only real export glue ────────────────────────────────
    //
    // On `target_arch = "wasm32"` the macro emits one `extern "C"` shim
    // for a detected stage export with `#[export_name]` set to the
    // documented kebab-case WIT export name. These shims
    // register genuine export entries in the final .wasm artifact so
    // host-side introspection (and the documented authoring contract in
    // docs/05 §Module Entry Point) sees the declared surface rather
    // than an empty export table.
    //
    // Shim bodies are intentionally minimal: the stage shim returns 0
    // (OK). Full typed data transfer
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
    let shim_mod_ident = syn::Ident::new(
        &format!("__slicer_wasm_exports_{type_ident_hash:x}"),
        proc_macro2::Span::call_site(),
    );

    // ── Real typed export glue per supported world (TASK-109) ───────
    //
    // For every world the macro now emits real, typed
    // `wit_bindgen::generate!`-backed component export glue that
    // marshals arguments through the documented WIT world into the
    // implemented SDK trait method. The placeholder `extern "C" fn ...
    // -> i32 { 0 }` stage shims are suppressed for these
    // worlds so they do not collide with or contaminate the real
    // component exports (docs/05 §Module Entry Point; docs/03
    // wit/deps/<pkg>/<pkg>.wit).
    //
    // Worlds covered: postpass (gcode + text), finalization, prepass
    // (mesh-analysis + layer-planning), layer (all 8 stage exports).
    let real_glue_world = resolve_stage_glue(stage_id_literal, trait_ident);

    let stage_shim_tokens: TokenStream2 =
        if stage_export_name_literal.is_empty() || real_glue_world.is_some() {
            quote! {}
        } else {
            let shim_name = syn::Ident::new(
                &format!(
                    "__slicer_export_{}",
                    stage_export_name_literal.replace('-', "_")
                ),
                proc_macro2::Span::call_site(),
            );
            quote! {
                #[cfg(target_arch = "wasm32")]
                #[export_name = #stage_export_name_literal]
                pub extern "C" fn #shim_name() -> i32 { 0 }
            }
        };

    let world_glue: TokenStream2 = match real_glue_world {
        Some(StageGlueKind::Postpass) => {
            // Per-stage postpass split (packet 163): the dispatcher has
            // already picked gcode vs text, so route by detected stage.
            if stage_id_literal == "PostPass::TextPostProcess" {
                build_postpass_text_glue(self_ty)
            } else {
                build_postpass_gcode_glue(self_ty)
            }
        }
        Some(StageGlueKind::Finalization) => build_finalization_world_glue(self_ty),
        Some(StageGlueKind::LayerSlicePostprocess) => build_layer_slice_postprocess_glue(self_ty),
        Some(StageGlueKind::LayerPerimeters) => build_layer_perimeters_glue(self_ty),
        Some(StageGlueKind::LayerPerimetersPostprocess) => {
            build_layer_perimeters_postprocess_glue(self_ty)
        }
        Some(StageGlueKind::LayerInfill) => build_layer_infill_glue(self_ty),
        Some(StageGlueKind::LayerInfillPostprocess) => build_layer_infill_postprocess_glue(self_ty),
        Some(StageGlueKind::LayerSupport) => build_layer_support_glue(self_ty),
        Some(StageGlueKind::LayerSupportPostprocess) => {
            build_layer_support_postprocess_glue(self_ty)
        }
        Some(StageGlueKind::LayerPathOptimization) => build_layer_path_optimization_glue(self_ty),
        Some(StageGlueKind::LayerAnchoredEvents) => build_layer_anchored_events_glue(self_ty),
        Some(StageGlueKind::PrepassMeshAnalysis) => build_prepass_mesh_analysis_glue(self_ty),
        Some(StageGlueKind::PrepassLayerPlanning) => build_prepass_layer_planning_glue(self_ty),
        Some(StageGlueKind::PrepassSeamPlanning) => build_prepass_seam_planning_glue(self_ty),
        Some(StageGlueKind::PrepassSupportGeometry) => build_prepass_support_geometry_glue(self_ty),
        None => quote! {},
    };

    let wasm_export_shims = quote! {
        #[cfg(target_arch = "wasm32")]
        #[allow(dead_code)]
        mod #shim_mod_ident {
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

/// Selector for which per-stage WIT package to emit real macro-generated
/// export glue for. Postpass remains a two-stage selector because its two
/// builders predate this tier split; layer and prepass now have one variant
/// per stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StageGlueKind {
    /// Per-stage postpass (packet 163): gcode and text are split into
    /// `build_postpass_gcode_glue` / `build_postpass_text_glue`; the
    /// post-dispatch routing inside `emit_glue` picks between them.
    Postpass,
    /// `slicer:finalization-layer-finalization` — layer finalization.
    Finalization,
    LayerSlicePostprocess,
    LayerPerimeters,
    LayerPerimetersPostprocess,
    LayerInfill,
    LayerInfillPostprocess,
    LayerSupport,
    LayerSupportPostprocess,
    LayerPathOptimization,
    LayerAnchoredEvents,
    PrepassMeshAnalysis,
    PrepassLayerPlanning,
    PrepassSeamPlanning,
    PrepassSupportGeometry,
}

/// Decide which WIT world gets real `wit_bindgen`-backed macro-generated
/// glue for this `#[slicer_module]` invocation. Glue is emitted when:
/// - the stage id belongs to a supported per-stage package.
///
/// A stageless trait impl is intentionally not enough to select a package:
/// the legacy placeholder shim path is emitted for that case.
fn resolve_stage_glue(stage_id: &str, trait_ident: Option<&str>) -> Option<StageGlueKind> {
    match stage_id {
        "PostPass::TextPostProcess" | "PostPass::GCodePostProcess" => Some(StageGlueKind::Postpass),
        "PostPass::LayerFinalization" => Some(StageGlueKind::Finalization),
        "Layer::SlicePostProcess" => Some(StageGlueKind::LayerSlicePostprocess),
        "Layer::Perimeters" => Some(StageGlueKind::LayerPerimeters),
        "Layer::PerimetersPostProcess" => Some(StageGlueKind::LayerPerimetersPostprocess),
        "Layer::Infill" => Some(StageGlueKind::LayerInfill),
        "Layer::InfillPostProcess" => Some(StageGlueKind::LayerInfillPostprocess),
        "Layer::Support" => Some(StageGlueKind::LayerSupport),
        "Layer::SupportPostProcess" => Some(StageGlueKind::LayerSupportPostprocess),
        "Layer::PathOptimization" => Some(StageGlueKind::LayerPathOptimization),
        "Layer::AnchoredEvents" => Some(StageGlueKind::LayerAnchoredEvents),
        "PrePass::MeshAnalysis" => Some(StageGlueKind::PrepassMeshAnalysis),
        "PrePass::LayerPlanning" => Some(StageGlueKind::PrepassLayerPlanning),
        "PrePass::SeamPlanning" => Some(StageGlueKind::PrepassSeamPlanning),
        "PrePass::SupportGeometry" => Some(StageGlueKind::PrepassSupportGeometry),
        _ => {
            let _ = trait_ident;
            None
        }
    }
}

/// The statement every macro-generated WIT export body opens with: it installs
/// the guest profiling sink (ADR-0055).
///
/// # Why here and not somewhere shared
///
/// `slicer_core::profile`'s marks on the clipper2 primitives are inert until
/// something installs a [`slicer_sdk::profile::BridgeSink`], and the guest has
/// no other entry point — a wasm component built for `wasm32-unknown-unknown`
/// runs no constructors, so there is no "guest start" to hook. A WIT export body
/// is the first guest code that runs on every dispatch call, and *every* path
/// into a module's own code goes through one. Emitting the call at the top of
/// each `impl Guest` method is therefore the smallest thing that guarantees the
/// sink is up before any module body — and before any `polygon_ops` call —
/// executes.
///
/// It is safe to repeat: `install_guest_sink` is `OnceLock`-guarded on both the
/// prime and the install, and it compiles to nothing off `wasm32`.
///
/// Kept as one helper rather than 17 copied literals so a new world or stage
/// export picks it up by construction.
fn profile_install_stmt() -> TokenStream2 {
    quote! { ::slicer_sdk::profile::install_guest_sink(); }
}

/// Shared per-world module preamble: `wit_bindgen::generate!` expansion,
/// a `ConfigValue` `use` statement, a `__slicer_adapt_config` helper
/// and a `__slicer_error_out` helper. The `world_ident` string selects
/// the world, and `world_namespace_ident` is the Rust module path
/// produced by wit-bindgen for that world (e.g. `world_postpass`,
/// `world_layer`). Caller supplies the inline WIT and the
/// world-specific `impl Guest` body.
fn emit_world_preamble(world_name: &str, _world_namespace: &str, inline_wit: &str) -> TokenStream2 {
    // Canonical dep packages — single source of truth in slicer-schema/wit/.
    // Option A (nested-package inline): the world file is the TOP-LEVEL statement
    // header; dep packages are nested as `package slicer:X { <body> }` blocks.
    // Cross-package `use` in the world file resolves over the whole group.
    // wit-bindgen 0.57.1 UnresolvedPackageGroup::parse supports this form.
    const TYPES_WIT: &str = include_str!("../../slicer-schema/wit/deps/types.wit");
    const CONFIG_WIT: &str = include_str!("../../slicer-schema/wit/deps/config.wit");
    const IR_TYPES_WIT: &str = include_str!("../../slicer-schema/wit/deps/ir-types.wit");
    const COMMON_WIT: &str = include_str!("../../slicer-schema/wit/deps/common.wit");
    const PREPASS_TYPES_WIT: &str = include_str!("../../slicer-schema/wit/deps/prepass-types.wit");

    // Strip the statement-form `package <X>;` header from a dep WIT file,
    // returning the body for brace-wrapping into a nested package block.
    fn strip_package_decl(dep_wit: &str) -> &str {
        for (i, c) in dep_wit.char_indices() {
            if c == '\n' {
                continue;
            }
            let rest = &dep_wit[i..];
            if rest.starts_with("package ") {
                let line_end = rest.find('\n').map(|p| i + p + 1).unwrap_or(dep_wit.len());
                return dep_wit[line_end..].trim_start();
            }
            break;
        }
        dep_wit
    }

    // Extract package name (without version) for brace-wrapping: e.g.
    // "package slicer:types;" → "slicer:types".
    fn extract_dep_pkg_name(dep_wit: &str) -> &str {
        for (i, c) in dep_wit.char_indices() {
            if c == '\n' {
                continue;
            }
            let rest = &dep_wit[i..];
            if rest.starts_with("package ") {
                let line_end = rest.find('\n').map(|p| p).unwrap_or(rest.len());
                let decl = rest[..line_end].trim();
                // decl is "package slicer:types;" → strip prefix/suffix
                let inner = decl
                    .trim_start_matches("package ")
                    .trim_end_matches(';')
                    .trim();
                return inner;
            }
            break;
        }
        ""
    }

    // Build nested-package dep block: `package slicer:X { <body> }`
    fn nest_dep(dep_wit: &str) -> String {
        let name = extract_dep_pkg_name(dep_wit);
        let body = strip_package_decl(dep_wit);
        format!("package {name} {{\n{body}\n}}")
    }

    // Assemble nested-package inline blob (Option A):
    // - World file is the top-level statement (begins with "package slicer:world-X@<version>;")
    // - Dep packages are nested `package slicer:X { ... }` blocks (UNVERSIONED)
    // - Cross-package `use slicer:...` in the world file resolve over the whole group
    // - ir-handles is nested unconditionally for every world: `COMMON_WIT`'s
    //   `host-services` interface (nested below, also unconditionally, into
    //   every world) itself does `use slicer:ir-handles/ir-handles.{extrusion-line}`
    //   for `generate-arachne-walls` (packet 112, Step 9A) — so every world that
    //   nests `COMMON_WIT` transitively needs the `slicer:ir-handles` package
    //   present, not just `world-layer`. Previously this was conditional
    //   per-world (`world-layer` only); that broke `world-prepass`/
    //   `world-postpass`/`world-finalization` guest builds the moment
    //   `common.wit`'s shared interface picked up the ir-handles `use` (P112
    //   Step 9B fix).
    let ir_block = format!("\n\n{}", nest_dep(IR_TYPES_WIT));

    let expanded_inline_wit = format!(
        "{}\n\n{}\n\n{}{}\n\n{}\n\n{}",
        inline_wit,
        nest_dep(TYPES_WIT),
        nest_dep(CONFIG_WIT),
        ir_block,
        nest_dep(COMMON_WIT),
        nest_dep(PREPASS_TYPES_WIT),
    );

    // With Option A, ConfigValue lives in the slicer:config package, not the world package.
    // Path: self::slicer::config::config_types::ConfigValue
    let ns_path: syn::Path = syn::parse_str("self::slicer::config::config_types::ConfigValue")
        .expect("parse ConfigValue path");

    // `layer-plan-view` is declared once, in the shared unversioned
    // `slicer:prepass-types` package, so support-geometry and seam-planning
    // resolve the same generated record and need no shadow alias (ADR-0002).
    let support_geometry_type_wiring = quote! {};

    // With Option A (nested packages), wit-bindgen requires `with` entries for
    // every imported external interface — even non-resource ones — otherwise it
    // bails with `MissingWith`. Use `generate_all` to ask it to generate inline
    // code for all referenced interfaces without needing to enumerate each one.
    quote! {
        ::wit_bindgen::generate!({
            inline: #expanded_inline_wit,
            world: #world_name,
            #support_geometry_type_wiring
            generate_all,
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
                        __SlicerWitConfigValue::PercentVal(p) => ::slicer_ir::ConfigValue::Percent(p),
                        __SlicerWitConfigValue::FloatOrPercentVal(fop) => ::slicer_ir::ConfigValue::FloatOrPercent {
                            value: fop.value,
                            is_percent: fop.is_percent,
                        },
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
///
/// Per packet 163, the postpass tier is split into two per-stage packages
/// (`slicer:postpass-gcode-postprocess@1.0.0` and
/// `slicer:postpass-text-postprocess@1.0.0`), each with its own world. The
/// `PostPass::GCodePostProcess` glue: binds the
/// `slicer:postpass-gcode-postprocess/gcode-postprocess-module` world and
/// routes into the user's `PostpassModule::run_gcode_postprocess`.
fn build_postpass_gcode_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/postpass-gcode-postprocess/postpass-gcode-postprocess.wit"
    );

    let preamble = emit_world_preamble("gcode-postprocess-module", "gcode_postprocess", wit_inline);
    let profile_install = profile_install_stmt();

    let gcode_arm = quote! {
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::PostpassModule>::from_config(&ir_config) {
            Ok(m) => m,
            Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_commands: ::std::vec::Vec<::slicer_sdk::postpass_types::GcodeCommand> =
            commands.iter().map(__slicer_adapt_postpass_command).collect();
        let mut sdk_builder = ::slicer_sdk::postpass_builders::GcodeOutputBuilder::new();
        let out = <#self_ty as ::slicer_sdk::traits::PostpassModule>::run_gcode_postprocess(
            &module, &sdk_commands, &mut sdk_builder, &ir_config,
        );
        match out {
            Ok(()) => {
                __slicer_drain_postpass_gcode(&sdk_builder, &output);
                Ok(())
            }
            Err(e) => Err(__slicer_error_out(e)),
        }
    };

    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_postpass_gcode_world_export {
            use super::#self_ty;
            // Per packet 163: the postpass tier is now a per-stage package
            // (`slicer:postpass-gcode-postprocess@1.0.0`). The bindgen
            // output namespacing puts `GcodeCommand` / `GcodeMoveCmd` /
            // `GcodeOutputBuilder` / `RetractMode` under
            // `slicer::postpass_gcode_postprocess::gcode_postprocess_types`
            // rather than at the world root. Re-export the short names
            // here so the body below (which is a verbatim port of the
            // pre-163 monomorphic `world-postpass` glue) still resolves.
            use slicer::postpass_gcode_postprocess::gcode_postprocess_types::{
                GcodeCommand, GcodeFanSpeedCmd, GcodeMoveCmd, GcodeOutputBuilder,
                GcodeRetractCmd, GcodeTemperatureCmd, GcodeToolChangeCmd, RetractMode,
            };
            use slicer::types::geometry::ExtrusionRole;
            // Per packet 163: the `Guest` trait moved from the world root
            // to `exports::slicer::postpass_gcode_postprocess::gcode_postprocess::Guest`
            // (interface-grouped exports). Use the fully-qualified path on
            // the `impl` line below; AC-6 asserts on the literal text.
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;

            #preamble

            fn __slicer_wit_role_to_sdk(role: &ExtrusionRole) -> ::slicer_sdk::ir::ExtrusionRole {
                match role {
                    ExtrusionRole::OuterWall => ::slicer_sdk::ir::ExtrusionRole::OuterWall,
                    ExtrusionRole::InnerWall => ::slicer_sdk::ir::ExtrusionRole::InnerWall,
                    ExtrusionRole::ThinWall => ::slicer_sdk::ir::ExtrusionRole::ThinWall,
                    ExtrusionRole::TopSolidInfill => ::slicer_sdk::ir::ExtrusionRole::TopSolidInfill,
                    ExtrusionRole::BottomSolidInfill => ::slicer_sdk::ir::ExtrusionRole::BottomSolidInfill,
                    ExtrusionRole::SparseInfill => ::slicer_sdk::ir::ExtrusionRole::SparseInfill,
                    ExtrusionRole::SupportMaterial => ::slicer_sdk::ir::ExtrusionRole::SupportMaterial,
                    ExtrusionRole::SupportInterface => ::slicer_sdk::ir::ExtrusionRole::SupportInterface,
                    ExtrusionRole::SupportBaseInterface => ::slicer_sdk::ir::ExtrusionRole::SupportBaseInterface,
                    ExtrusionRole::Ironing => ::slicer_sdk::ir::ExtrusionRole::Ironing,
                     ExtrusionRole::BridgeInfill => ::slicer_sdk::ir::ExtrusionRole::BridgeInfill,
                     ExtrusionRole::InternalBridgeInfill => ::slicer_sdk::ir::ExtrusionRole::InternalBridgeInfill,
                    ExtrusionRole::WipeTower => ::slicer_sdk::ir::ExtrusionRole::WipeTower,
                    ExtrusionRole::Custom(s) if s == "slicer.builtin/internal-solid-infill@1" => {
                        ::slicer_sdk::ir::ExtrusionRole::InternalSolidInfill
                    }
                    ExtrusionRole::Custom(s) => ::slicer_sdk::ir::ExtrusionRole::Custom(s.clone()),
                    ExtrusionRole::GapFill => ::slicer_sdk::ir::ExtrusionRole::GapFill,
                    ExtrusionRole::RaftInfill => ::slicer_sdk::ir::ExtrusionRole::RaftInfill,
                    // Forward-compat fallback for future `#[non_exhaustive]` variants.
                    _ => ::slicer_sdk::ir::ExtrusionRole::OuterWall,
                }
            }

            fn __slicer_sdk_role_to_wit(role: &::slicer_sdk::ir::ExtrusionRole) -> ExtrusionRole {
                match role {
                    ::slicer_sdk::ir::ExtrusionRole::OuterWall => ExtrusionRole::OuterWall,
                    ::slicer_sdk::ir::ExtrusionRole::InnerWall => ExtrusionRole::InnerWall,
                    ::slicer_sdk::ir::ExtrusionRole::ThinWall => ExtrusionRole::ThinWall,
                    ::slicer_sdk::ir::ExtrusionRole::TopSolidInfill => ExtrusionRole::TopSolidInfill,
                    ::slicer_sdk::ir::ExtrusionRole::BottomSolidInfill => ExtrusionRole::BottomSolidInfill,
                    ::slicer_sdk::ir::ExtrusionRole::SparseInfill => ExtrusionRole::SparseInfill,
                    ::slicer_sdk::ir::ExtrusionRole::SupportMaterial => ExtrusionRole::SupportMaterial,
                    ::slicer_sdk::ir::ExtrusionRole::SupportInterface => ExtrusionRole::SupportInterface,
                    ::slicer_sdk::ir::ExtrusionRole::SupportBaseInterface => ExtrusionRole::SupportBaseInterface,
                    ::slicer_sdk::ir::ExtrusionRole::Ironing => ExtrusionRole::Ironing,
                     ::slicer_sdk::ir::ExtrusionRole::BridgeInfill => ExtrusionRole::BridgeInfill,
                     ::slicer_sdk::ir::ExtrusionRole::InternalBridgeInfill => ExtrusionRole::InternalBridgeInfill,
                    ::slicer_sdk::ir::ExtrusionRole::WipeTower => ExtrusionRole::WipeTower,
                    ::slicer_sdk::ir::ExtrusionRole::Custom(s) => ExtrusionRole::Custom(s.clone()),
                    ::slicer_sdk::ir::ExtrusionRole::PrimeTower => {
                        ExtrusionRole::Custom(::std::string::String::from("slicer.builtin/prime-tower@1"))
                    }
                    ::slicer_sdk::ir::ExtrusionRole::Skirt => {
                        ExtrusionRole::Custom(::std::string::String::from("slicer.builtin/skirt@1"))
                    }
                    ::slicer_sdk::ir::ExtrusionRole::Brim => {
                        ExtrusionRole::Custom(::std::string::String::from("slicer.builtin/brim@1"))
                    }
                    ::slicer_sdk::ir::ExtrusionRole::InternalSolidInfill => {
                        ExtrusionRole::Custom(::std::string::String::from(
                            "slicer.builtin/internal-solid-infill@1",
                        ))
                    }
                    ::slicer_sdk::ir::ExtrusionRole::GapFill => ExtrusionRole::GapFill,
                    ::slicer_sdk::ir::ExtrusionRole::RaftInfill => ExtrusionRole::RaftInfill,
                    // Forward-compat fallback for future `#[non_exhaustive]` variants.
                    _ => ExtrusionRole::OuterWall,
                }
            }

            fn __slicer_retract_mode_ir_to_wit(mode: &::slicer_ir::RetractMode) -> RetractMode {
                match mode {
                    ::slicer_ir::RetractMode::Gcode => RetractMode::Gcode,
                    ::slicer_ir::RetractMode::Firmware => RetractMode::Firmware,
                }
            }

            fn __slicer_retract_mode_wit_to_ir(mode: &RetractMode) -> ::slicer_ir::RetractMode {
                match mode {
                    RetractMode::Gcode => ::slicer_ir::RetractMode::Gcode,
                    RetractMode::Firmware => ::slicer_ir::RetractMode::Firmware,
                }
            }

            fn __slicer_adapt_postpass_command(command: &GcodeCommand) -> ::slicer_sdk::postpass_types::GcodeCommand {
                match command {
                    GcodeCommand::Move(cmd) => ::slicer_sdk::postpass_types::GcodeCommand::Move {
                        x: cmd.x,
                        y: cmd.y,
                        z: cmd.z,
                        e: cmd.e,
                        f: cmd.f,
                        role: __slicer_wit_role_to_sdk(&cmd.role),
                    },
                    GcodeCommand::Retract(cmd) => ::slicer_sdk::postpass_types::GcodeCommand::Retract {
                        length: cmd.length,
                        speed: cmd.speed,
                        mode: __slicer_retract_mode_wit_to_ir(&cmd.mode),
                    },
                    GcodeCommand::Unretract(cmd) => ::slicer_sdk::postpass_types::GcodeCommand::Unretract {
                        length: cmd.length,
                        speed: cmd.speed,
                        mode: __slicer_retract_mode_wit_to_ir(&cmd.mode),
                    },
                    GcodeCommand::FanSpeed(cmd) => ::slicer_sdk::postpass_types::GcodeCommand::FanSpeed {
                        value: cmd.value,
                    },
                    GcodeCommand::Temperature(cmd) => ::slicer_sdk::postpass_types::GcodeCommand::Temperature {
                        tool: cmd.tool,
                        celsius: cmd.celsius,
                        wait: cmd.wait,
                    },
                    GcodeCommand::ToolChange(cmd) => ::slicer_sdk::postpass_types::GcodeCommand::ToolChange {
                        after_entity_index: cmd.after_entity_index,
                        from: cmd.from_tool,
                        to: cmd.to_tool,
                    },
                    GcodeCommand::Comment(text) => ::slicer_sdk::postpass_types::GcodeCommand::Comment {
                        text: text.clone(),
                    },
                    GcodeCommand::Raw(text) => ::slicer_sdk::postpass_types::GcodeCommand::Raw {
                        text: text.clone(),
                    },
                }
            }

            fn __slicer_drain_postpass_gcode(
                sdk: &::slicer_sdk::postpass_builders::GcodeOutputBuilder,
                wit: &GcodeOutputBuilder,
            ) {
                for cmd in sdk.commands() {
                    match cmd {
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                            ::slicer_sdk::postpass_types::GcodeCommand::Move { x, y, z, e, f, role }
                        ) => {
                            let wit_cmd = GcodeMoveCmd {
                                x: *x,
                                y: *y,
                                z: *z,
                                e: *e,
                                f: *f,
                                role: __slicer_sdk_role_to_wit(role),
                            };
                            let _ = wit.push_move(&wit_cmd);
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                            ::slicer_sdk::postpass_types::GcodeCommand::Retract { length, speed, mode }
                        ) => {
                            let _ = wit.push_retract(*length, *speed, __slicer_retract_mode_ir_to_wit(mode));
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                            ::slicer_sdk::postpass_types::GcodeCommand::Unretract { length, speed, mode }
                        ) => {
                            let _ = wit.push_unretract(*length, *speed, __slicer_retract_mode_ir_to_wit(mode));
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                            ::slicer_sdk::postpass_types::GcodeCommand::FanSpeed { value }
                        ) => {
                            let _ = wit.push_fan_speed(*value);
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                            ::slicer_sdk::postpass_types::GcodeCommand::Temperature { tool, celsius, wait }
                        ) => {
                            let _ = wit.push_temperature(*tool, *celsius, *wait);
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                            ::slicer_sdk::postpass_types::GcodeCommand::ToolChange { after_entity_index, from, to }
                        ) => {
                            let _ = wit.push_tool_change(*after_entity_index, *from, *to);
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                            ::slicer_sdk::postpass_types::GcodeCommand::Comment { text }
                        ) => {
                            let _ = wit.push_comment(text);
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                            ::slicer_sdk::postpass_types::GcodeCommand::Raw { text }
                        ) => {
                            let _ = wit.push_raw(text);
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                            ::slicer_sdk::postpass_types::GcodeCommand::ExtrusionMode { absolute }
                        ) => {
                            let _ = wit.push_raw(&if *absolute { "M82\n".to_string() } else { "M83\n".to_string() });
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::ZHop { after_entity_index, hop_height } => {
                            let _ = wit.push_z_hop(*after_entity_index, *hop_height);
                        }
                    }
                }
            }

            struct __SlicerPostpassGcodeComponent;

            impl exports::slicer::postpass_gcode_postprocess::gcode_postprocess::Guest for __SlicerPostpassGcodeComponent {
                fn run(
                    commands: Vec<GcodeCommand>,
                    output: GcodeOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    #profile_install
                    #gcode_arm
                }
            }

            export!(__SlicerPostpassGcodeComponent);
        }
    }
}

/// `PostPass::TextPostProcess` glue: binds the
/// `slicer:postpass-text-postprocess/text-postprocess-module` world and
/// routes into the user's `PostpassModule::run_text_postprocess`.
fn build_postpass_text_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/postpass-text-postprocess/postpass-text-postprocess.wit"
    );

    let preamble = emit_world_preamble("text-postprocess-module", "text_postprocess", wit_inline);
    let profile_install = profile_install_stmt();

    let text_arm = quote! {
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::PostpassModule>::from_config(&ir_config) {
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
    };

    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_postpass_text_world_export {
            use super::#self_ty;
            // Per packet 163: the postpass tier is now a per-stage package.
            // The bindgen output namespacing puts `ConfigView` /
            // `ModuleError` under the imported dep interfaces rather than
            // at the world root. Re-import the short names so the body
            // below resolves.
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;

            #preamble

            struct __SlicerPostpassTextComponent;

            impl exports::slicer::postpass_text_postprocess::text_postprocess::Guest for __SlicerPostpassTextComponent {
                fn run(
                    gcode_text: String,
                    config: ConfigView,
                ) -> Result<String, ModuleError> {
                    #profile_install
                    #text_arm
                }
            }

            export!(__SlicerPostpassTextComponent);
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
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit"
    );

    let preamble = emit_world_preamble(
        "layer-finalization-module",
        "layer_finalization",
        wit_inline,
    );
    let profile_install = profile_install_stmt();

    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_finalization_world_export {
            // Per packet 163: the finalization tier is now a per-stage
            // package (`slicer:finalization-layer-finalization@1.0.0`).
            // The bindgen output namespacing puts the resource and
            // record types under
            // `slicer::finalization_layer_finalization::layer_finalization_types`
            // rather than at the world root. Re-export the short names
            // here so the body below (a verbatim port of the pre-163
            // monomorphic `world-finalization` glue) still resolves.
            use slicer::finalization_layer_finalization::layer_finalization_types::{
                EntityMutation, FinalizationOutputBuilder, LayerCollectionView,
                PrintEntityView, RegionKey, SortKey, SyntheticLayerData, ToolChangeView,
                ZHopView,
            };
            // Per packet 163: the `Guest` trait moved from the world root
            // to `exports::slicer::finalization_layer_finalization::layer_finalization::Guest`
            // (interface-grouped exports). Re-import the short name here
            // so the `impl Guest for __SlicerFinalizationComponent` body
            // below still resolves.
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            use slicer::types::geometry::{ExtrusionPath3d, ExtrusionRole};
            // Intentionally do NOT `use super::*;` — the user's module
            // may have imported types (e.g. `slicer_ir::Point3WithWidth`)
            // that would collide with the wit-bindgen-generated names.
            // Bring in only the user's module type.
            use super::#self_ty;

            #preamble

            // Unlike the prepass world, `wit_bindgen::generate!` does not
            // emit a flat top-level alias for the finalization world's
            // `point3-with-width`, so bring it into scope explicitly.
            // With Option A (nested-package), geometry lives in slicer:types.
            use self::slicer::types::geometry::Point3WithWidth;

            struct __SlicerFinalizationComponent;

            /// Map a wit-bindgen finalization-world `ExtrusionRole`
            /// enum value to `slicer_ir::ExtrusionRole`. The `Custom`
            /// variant carries a string tag which is passed through
            /// losslessly.
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
                    ExtrusionRole::SupportBaseInterface => ::slicer_ir::ExtrusionRole::SupportBaseInterface,
                    ExtrusionRole::Ironing => ::slicer_ir::ExtrusionRole::Ironing,
                     ExtrusionRole::BridgeInfill => ::slicer_ir::ExtrusionRole::BridgeInfill,
                     ExtrusionRole::InternalBridgeInfill => ::slicer_ir::ExtrusionRole::InternalBridgeInfill,
                    ExtrusionRole::WipeTower => ::slicer_ir::ExtrusionRole::WipeTower,
                    ExtrusionRole::Custom(s) if s == "slicer.builtin/internal-solid-infill@1" => {
                        ::slicer_ir::ExtrusionRole::InternalSolidInfill
                    }
                    ExtrusionRole::Custom(s) => ::slicer_ir::ExtrusionRole::Custom(s),
                    ExtrusionRole::GapFill => ::slicer_ir::ExtrusionRole::GapFill,
                    ExtrusionRole::RaftInfill => ::slicer_ir::ExtrusionRole::RaftInfill,
                    // Forward-compat fallback for future `#[non_exhaustive]` variants.
                    _ => ::slicer_ir::ExtrusionRole::OuterWall,
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
                    ::slicer_ir::ExtrusionRole::SupportBaseInterface => ExtrusionRole::SupportBaseInterface,
                    ::slicer_ir::ExtrusionRole::Ironing => ExtrusionRole::Ironing,
                     ::slicer_ir::ExtrusionRole::BridgeInfill => ExtrusionRole::BridgeInfill,
                     ::slicer_ir::ExtrusionRole::InternalBridgeInfill => ExtrusionRole::InternalBridgeInfill,
                    ::slicer_ir::ExtrusionRole::WipeTower => ExtrusionRole::WipeTower,
                    ::slicer_ir::ExtrusionRole::PrimeTower => {
                        ExtrusionRole::Custom(::std::string::String::from("slicer.builtin/prime-tower@1"))
                    }
                    ::slicer_ir::ExtrusionRole::Skirt => {
                        ExtrusionRole::Custom(::std::string::String::from("slicer.builtin/skirt@1"))
                    }
                    ::slicer_ir::ExtrusionRole::Brim => {
                        ExtrusionRole::Custom(::std::string::String::from("slicer.builtin/brim@1"))
                    }
                    ::slicer_ir::ExtrusionRole::InternalSolidInfill => {
                        ExtrusionRole::Custom(::std::string::String::from(
                            "slicer.builtin/internal-solid-infill@1",
                        ))
                    }
                    ::slicer_ir::ExtrusionRole::Custom(s) => ExtrusionRole::Custom(s.clone()),
                    ::slicer_ir::ExtrusionRole::GapFill => ExtrusionRole::GapFill,
                    ::slicer_ir::ExtrusionRole::RaftInfill => ExtrusionRole::RaftInfill,
                    // Forward-compat fallback for future `#[non_exhaustive]` variants.
                    _ => ExtrusionRole::OuterWall,
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
                            overhang_quartile: pt.overhang_quartile,
                            dist_to_top_mm: 0.0,
                            overhang_distance_mm: pt.overhang_distance_mm,
                        })
                        .collect(),
                    role: __slicer_role_ir_to_wit(&p.role),
                    speed_factor: p.speed_factor,
                    tool_index: p.tool_index,
                    order_lock: p.order_lock,
                }
            }

            fn __slicer_path_wit_to_ir(p: &ExtrusionPath3d) -> ::slicer_ir::ExtrusionPath3D {
                ::slicer_ir::ExtrusionPath3D {
                    points: p
                        .points
                        .iter()
                        .map(|pt| ::slicer_ir::Point3WithWidth {
                            x: pt.x,
                            y: pt.y,
                            z: pt.z,
                            width: pt.width,
                            flow_factor: pt.flow_factor,
                            overhang_quartile: pt.overhang_quartile,
                            dist_to_top_mm: 0.0,
                            overhang_distance_mm: pt.overhang_distance_mm,
                        })
                        .collect(),
                    role: __slicer_role_wit_to_ir(p.role.clone()),
                    speed_factor: p.speed_factor,
                    tool_index: p.tool_index,
                    order_lock: p.order_lock,
                }
            }

            fn __slicer_parse_region_id(raw: &str) -> Result<u64, ::std::string::String> {
                let parsed = raw.parse::<u64>().map_err(|_| {
                    format!(
                        "expected canonical decimal u64 string with no leading zeros, got '{}'",
                        raw,
                    )
                })?;
                if parsed.to_string() != raw {
                    return Err(format!(
                        "expected canonical decimal u64 string with no leading zeros, got '{}'",
                        raw,
                    ));
                }
                Ok(parsed)
            }

            impl exports::slicer::finalization_layer_finalization::layer_finalization::Guest for __SlicerFinalizationComponent {
                fn run(
                    layers: Vec<LayerCollectionView>,
                    output: FinalizationOutputBuilder,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    #profile_install
                    let ir_config = __slicer_adapt_config(&config);
                    let module = match <#self_ty as ::slicer_sdk::traits::FinalizationModule>::from_config(&ir_config) {
                        Ok(m) => m,
                        Err(e) => return Err(__slicer_error_out(e)),
                    };

                    // ── Input deep copy ────────────────────────────
                    // Build one SDK `LayerCollectionView` per incoming
                    // wit-bindgen resource handle by calling the typed
                    // accessors (`layer-index`, `z`, `entity-count`,
                    // `ordered-entities`, `tool-changes`, `z-hops`).
                    // The SDK wrapper stores a full `LayerCollectionIR`,
                    // so preserve the guest-visible completed-layer
                    // content rather than synthesizing placeholder
                    // entities.
                    let mut sdk_layers: ::std::vec::Vec<::slicer_sdk::traits::LayerCollectionView> =
                        ::std::vec::Vec::with_capacity(layers.len());
                    for wit_layer in layers.iter() {
                        let mut ordered_entities: ::std::vec::Vec<::slicer_ir::PrintEntity> =
                            ::std::vec::Vec::new();
                        for entity in wit_layer.ordered_entities().into_iter() {
                            let region_id = match __slicer_parse_region_id(&entity.region_key.region_id) {
                                Ok(region_id) => region_id,
                                Err(reason) => {
                                    return Err(ModuleError {
                                        code: 1,
                                        message: format!(
                                            "finalization input region '{}'/'{}' has invalid region-id: {}",
                                            entity.region_key.object_id,
                                            entity.region_key.region_id,
                                            reason,
                                        ),
                                        fatal: true,
                                    });
                                }
                            };

                            ordered_entities.push(::slicer_ir::PrintEntity {
                                entity_id: entity.entity_id,
                                path: __slicer_path_wit_to_ir(&entity.path),
                                role: __slicer_role_wit_to_ir(entity.role),
                                tool_index: entity.tool_index,
                                region_key: ::slicer_ir::RegionKey {
                                    global_layer_index: entity.region_key.layer_index,
                                    object_id: entity.region_key.object_id,
                                    region_id,
                                    variant_chain: Vec::new(),
                                },
                                topo_order: entity.topo_order,
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
                        let z_hops: ::std::vec::Vec<::slicer_ir::ZHop> = wit_layer
                            .z_hops()
                            .into_iter()
                            .map(|hop| ::slicer_ir::ZHop {
                                after_entity_index: hop.after_entity_index,
                                hop_height: hop.hop_height,
                            })
                            .collect();
                        let ir = ::slicer_ir::LayerCollectionIR {
                            schema_version: ::slicer_ir::CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
                            global_layer_index: wit_layer.layer_index(),
                            z: wit_layer.z(),
                            ordered_entities,
                            support_entity_identities: ::std::vec::Vec::new(),
                            tool_changes,
                            z_hops,
                            annotations: ::std::vec::Vec::new(),
                            retracts: ::std::vec::Vec::new(),
                            travel_moves: ::std::vec::Vec::new(),
                            speed_profiles: ::std::vec::Vec::new(),
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
                    // Drain ALL pushes via priority_pushes() so that explicit priorities
                    // (e.g. top-surface-ironing's priority=6000) are forwarded across the
                    // WIT boundary. entity_pushes() is NOT iterated here to avoid
                    // double-replay (all pushes, including legacy priority=0 ones, appear
                    // in priority_pushes()).
                    for (layer_index, path, tool_index, region_key, priority) in sdk_output.priority_pushes() {
                        let wit_path = __slicer_path_ir_to_wit(path);
                        let wit_region_key = RegionKey {
                            layer_index: region_key.global_layer_index,
                            object_id: region_key.object_id.clone(),
                            region_id: region_key.region_id.to_string(),
                        };
                        let _ = output.push_entity_with_priority(layer_index, &wit_path, tool_index, &wit_region_key, priority);
                    }
                    for op in sdk_output.merge_ops() {
                        match op {
                            ::slicer_sdk::traits::MergeOp::ModifyEntity { layer, entity_id, mutation } => {
                                let wit_mutation = match mutation {
                                    ::slicer_sdk::traits::EntityMutation::SetSpeedFactor(v) => EntityMutation::SetSpeedFactor(*v),
                                    ::slicer_sdk::traits::EntityMutation::SetFlowFactor(v) => EntityMutation::SetFlowFactor(*v),
                                    // Vec<f32> is not Copy: clone rather than deref.
                                    ::slicer_sdk::traits::EntityMutation::SetPointSpeedFactors(v) => EntityMutation::SetPointSpeedFactors(v.clone()),
                                    ::slicer_sdk::traits::EntityMutation::SetPathPoints(v) => EntityMutation::SetPathPoints(
                                        v.iter()
                                            .map(|pt| Point3WithWidth {
                                                x: pt.x,
                                                y: pt.y,
                                                z: pt.z,
                                                width: pt.width,
                                                flow_factor: pt.flow_factor,
                                                overhang_quartile: pt.overhang_quartile,
                                                dist_to_top_mm: pt.dist_to_top_mm,
                                                overhang_distance_mm: pt.overhang_distance_mm,
                                            })
                                            .collect(),
                                    ),
                                };
                                // Packet 189: `entity-mutation` now carries a
                                // `list<f32>` payload, so wit-bindgen generates a
                                // by-reference parameter here (it is no longer Copy).
                                let _ = output.modify_entity(*layer, *entity_id, &wit_mutation);
                            }
                            ::slicer_sdk::traits::MergeOp::SortLayer { layer, key } => {
                                let wit_key = match key {
                                    ::slicer_sdk::traits::SortKey::ByPriorityAndEntityId => SortKey::ByPriorityAndEntityId,
                                    ::slicer_sdk::traits::SortKey::ByEntityId => SortKey::ByEntityId,
                                    ::slicer_sdk::traits::SortKey::ByObjectIdThenPriority => SortKey::ByObjectIdThenPriority,
                                };
                                let _ = output.sort_layer_by(*layer, wit_key);
                            }
                            ::slicer_sdk::traits::MergeOp::InsertSynthLayer { idx, data } => {
                                let wit_paths: ::std::vec::Vec<ExtrusionPath3d> =
                                    data.paths.iter().map(__slicer_path_ir_to_wit).collect();
                                let wit_data = SyntheticLayerData { z: data.z, paths: wit_paths };
                                let _ = output.insert_synthetic_layer_after(*idx, &wit_data);
                            }
                            ::slicer_sdk::traits::MergeOp::InsertEntityAt { layer, position, path, tool_index, region_key } => {
                                let wit_path = __slicer_path_ir_to_wit(path);
                                let wit_region_key = RegionKey {
                                    layer_index: region_key.global_layer_index,
                                    object_id: region_key.object_id.clone(),
                                    region_id: region_key.region_id.to_string(),
                                };
                                let _ = output.insert_entity_at(*layer, *position, &wit_path, *tool_index, &wit_region_key);
                            }
                            ::slicer_sdk::traits::MergeOp::SetEntityOrder { layer, items } => {
                                let wit_items: ::std::vec::Vec<(u32, bool)> = items.iter().copied().collect();
                                let _ = output.set_entity_order(*layer, &wit_items);
                            }
                        }
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

fn prepass_mesh_helpers() -> TokenStream2 {
    quote! {
        fn __slicer_paint_value_from_wit(
            value: PaintValueView,
        ) -> ::slicer_sdk::prepass_types::PaintValueView {
            match value {
                PaintValueView::Flag(flag) => ::slicer_sdk::prepass_types::PaintValueView {
                    kind: ::std::string::String::from("flag"),
                    flag: Some(flag),
                    scalar: None,
                    tool_index: None,
                },
                PaintValueView::Scalar(scalar) => ::slicer_sdk::prepass_types::PaintValueView {
                    kind: ::std::string::String::from("scalar"),
                    flag: None,
                    scalar: Some(scalar),
                    tool_index: None,
                },
                PaintValueView::ToolIndex(tool_index) => ::slicer_sdk::prepass_types::PaintValueView {
                    kind: ::std::string::String::from("tool_index"),
                    flag: None,
                    scalar: None,
                    tool_index: Some(tool_index),
                },
            }
        }

        fn __slicer_paint_stroke_from_wit(
            stroke: PaintStrokeView,
        ) -> ::slicer_sdk::prepass_types::PaintStrokeView {
            let triangle_points: ::std::vec::Vec<[f32; 3]> = stroke
                .triangles
                .into_iter()
                .map(|point| [point.x, point.y, point.z])
                .collect();
            let mut triangle_chunks = triangle_points.chunks_exact(3);
            debug_assert!(
                triangle_chunks.remainder().is_empty(),
                "PaintStrokeView.triangles must contain complete triangle triplets"
            );
            ::slicer_sdk::prepass_types::PaintStrokeView {
                triangles: triangle_chunks
                    .by_ref()
                    .map(|triangle| [triangle[0], triangle[1], triangle[2]])
                    .collect(),
                semantic: stroke.semantic,
                value: __slicer_paint_value_from_wit(stroke.value),
            }
        }

        fn __slicer_paint_layer_from_wit(
            layer: PaintLayerView,
        ) -> ::slicer_sdk::prepass_types::PaintLayerView {
            ::slicer_sdk::prepass_types::PaintLayerView {
                semantic: layer.semantic,
                facet_values: layer
                    .facet_values
                    .into_iter()
                    .map(|value| value.map(__slicer_paint_value_from_wit))
                    .collect(),
                strokes: layer
                    .strokes
                    .into_iter()
                    .map(__slicer_paint_stroke_from_wit)
                    .collect(),
            }
        }

        fn __slicer_mesh_object_from_wit(
            object: MeshObjectView,
        ) -> ::slicer_sdk::prepass_types::MeshObjectView {
            ::slicer_sdk::prepass_types::MeshObjectView {
                object_id: object.object_id,
                vertices: object
                    .vertices
                    .into_iter()
                    .map(|point| [point.x, point.y, point.z])
                    .collect(),
                triangles: object
                    .triangles
                    .into_iter()
                    .map(|(a, b, c)| [a, b, c])
                    .collect(),
                paint_layers: object
                    .paint_layers
                    .into_iter()
                    .map(__slicer_paint_layer_from_wit)
                    .collect(),
            }
        }
    }
}

fn prepass_geometry_helpers() -> TokenStream2 {
    quote! {
        fn __slicer_expolygon_from_wit(
            ep: ExPolygon,
        ) -> ::slicer_ir::ExPolygon {
            ::slicer_ir::ExPolygon {
                contour: ::slicer_ir::Polygon {
                    points: ep.contour.points.iter().map(|p| ::slicer_ir::Point2 { x: p.x, y: p.y }).collect(),
                },
                holes: ep.holes.into_iter().map(|h| ::slicer_ir::Polygon {
                    points: h.points.iter().map(|p| ::slicer_ir::Point2 { x: p.x, y: p.y }).collect(),
                }).collect(),
            }
        }
    }
}

fn prepass_seam_helpers() -> TokenStream2 {
    let mesh_helpers = prepass_mesh_helpers();
    let geometry_helpers = prepass_geometry_helpers();
    quote! {
        #mesh_helpers
        #geometry_helpers

        fn __slicer_paint_semantic_from_wit(
            semantic: PaintSemantic,
        ) -> ::slicer_ir::PaintSemantic {
            match semantic {
                PaintSemantic::Material => ::slicer_ir::PaintSemantic::Material,
                PaintSemantic::FuzzySkin => ::slicer_ir::PaintSemantic::FuzzySkin,
                PaintSemantic::SupportEnforcer => ::slicer_ir::PaintSemantic::SupportEnforcer,
                PaintSemantic::SupportBlocker => ::slicer_ir::PaintSemantic::SupportBlocker,
                PaintSemantic::Custom(value) => ::slicer_ir::PaintSemantic::Custom(value),
            }
        }

        fn __slicer_paint_value_from_ir_wit(
            value: PaintValue,
        ) -> ::slicer_ir::PaintValue {
            match value {
                PaintValue::Flag(value) => ::slicer_ir::PaintValue::Flag(value),
                PaintValue::Scalar(value) => ::slicer_ir::PaintValue::Scalar(value),
                PaintValue::ToolIndex(value) => ::slicer_ir::PaintValue::ToolIndex(value),
            }
        }

        fn __slicer_seam_planning_region_from_wit(
            region: SeamPlanningRegionInput,
        ) -> ::slicer_sdk::prepass_types::SeamPlanningRegionInput {
            ::slicer_sdk::prepass_types::SeamPlanningRegionInput {
                global_layer_index: region.global_layer_index,
                object_id: region.object_id,
                region_id: region.region_id,
                variant_chain: region
                    .variant_chain
                    .into_iter()
                    .map(|(semantic, value)| {
                        (semantic, __slicer_paint_value_from_ir_wit(value))
                    })
                    .collect(),
                z: region.z,
                height: region.height,
                ex_polygons: region
                    .ex_polygons
                    .into_iter()
                    .map(__slicer_expolygon_from_wit)
                    .collect(),
                segment_annotations: region
                    .segment_annotations
                    .into_iter()
                    .map(|entry| {
                        (
                            __slicer_paint_semantic_from_wit(entry.semantic),
                            entry
                                .polygons
                                .into_iter()
                                .map(|polygon| {
                                    polygon
                                        .values
                                        .into_iter()
                                        .map(|value| value.map(__slicer_paint_value_from_ir_wit))
                                        .collect()
                                })
                                .collect(),
                        )
                    })
                    .collect(),
                scoring_width: region.scoring_width,
            }
        }
    }
}

fn build_prepass_mesh_analysis_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/prepass-mesh-analysis/prepass-mesh-analysis.wit"
    );
    let preamble = emit_world_preamble("mesh-analysis-module", "mesh_analysis", wit_inline);
    let profile_install = profile_install_stmt();
    let arm = quote! {
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::PrepassModule>::from_config(&ir_config) {
            Ok(m) => m,
            Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_objects: ::std::vec::Vec<::slicer_ir::ObjectId> = _objects.clone();
        let mut sdk_output = ::slicer_sdk::prepass_builders::MeshAnalysisOutput::new();
        let out = <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_mesh_analysis(
            &module, &sdk_objects, &mut sdk_output, &ir_config,
        );
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
            if let Err(e) = output.push_facet_annotation(__slicer_obj, __slicer_wit_ann) {
                return Err(ModuleError { code: 6, message: e, fatal: true });
            }
        }
        for (__slicer_obj, __slicer_grp) in sdk_output.surface_groups() {
            let __slicer_wit_grp = SurfaceGroupProposal {
                facet_indices: __slicer_grp.facet_indices.clone(),
                z_min: __slicer_grp.z_min,
                z_max: __slicer_grp.z_max,
                shell_count: __slicer_grp.shell_count,
            };
            if let Err(e) = output.push_surface_group(__slicer_obj, &__slicer_wit_grp) {
                return Err(ModuleError { code: 7, message: e, fatal: true });
            }
        }
        match out {
            Ok(()) => Ok(()),
            Err(e) => Err(__slicer_error_out(e)),
        }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_prepass_mesh_analysis_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            use slicer::prepass_mesh_analysis::mesh_analysis_types::{
                FacetAnnotation, FacetClass, MeshAnalysisOutput, SurfaceGroupProposal,
            };
            #preamble
            struct __SlicerPrepassMeshAnalysisComponent;
            impl exports::slicer::prepass_mesh_analysis::mesh_analysis::Guest for __SlicerPrepassMeshAnalysisComponent {
                fn run(
                    _objects: Vec<String>,
                    output: MeshAnalysisOutput,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerPrepassMeshAnalysisComponent);
        }
    }
}

fn build_prepass_layer_planning_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit"
    );
    let preamble = emit_world_preamble("layer-planning-module", "layer_planning", wit_inline);
    let profile_install = profile_install_stmt();
    let arm = quote! {
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::PrepassModule>::from_config(&ir_config) {
            Ok(m) => m,
            Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_objects: ::std::vec::Vec<::slicer_ir::ObjectId> = _objects.clone();
        let mut sdk_output = ::slicer_sdk::prepass_builders::LayerPlanOutput::new();
        let out = <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_layer_planning(
            &module, &sdk_objects, &mut sdk_output, &ir_config,
        );
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
            if let Err(e) = output.push_layer(&LayerProposal {
                z: __slicer_layer.z,
                active_regions: __slicer_wit_regions,
            }) {
                return Err(ModuleError { code: 5, message: e, fatal: true });
            }
        }
        match out {
            Ok(()) => Ok(()),
            Err(e) => Err(__slicer_error_out(e)),
        }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_prepass_layer_planning_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            use slicer::prepass_layer_planning::layer_planning_types::{
                LayerPlanOutput, LayerProposal, RegionLayerProposal,
            };
            #preamble
            struct __SlicerPrepassLayerPlanningComponent;
            impl exports::slicer::prepass_layer_planning::layer_planning::Guest for __SlicerPrepassLayerPlanningComponent {
                fn run(
                    _objects: Vec<String>,
                    output: LayerPlanOutput,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerPrepassLayerPlanningComponent);
        }
    }
}

fn build_prepass_seam_planning_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/prepass-seam-planning/prepass-seam-planning.wit"
    );
    let preamble = emit_world_preamble("seam-planning-module", "seam_planning", wit_inline);
    let profile_install = profile_install_stmt();
    let helpers = prepass_seam_helpers();
    let arm = quote! {
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::PrepassModule>::from_config(&ir_config) {
            Ok(m) => m,
            Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_objects: ::std::vec::Vec<::slicer_sdk::prepass_types::MeshObjectView> = objects
            .into_iter()
            .map(__slicer_mesh_object_from_wit)
            .collect();
        let sdk_layer_plan = ::slicer_sdk::prepass_types::LayerPlanView {
            layers: layer_plan.layers.iter().map(|e| ::slicer_sdk::prepass_types::LayerPlanViewEntry {
                global_layer_index: e.global_layer_index,
                z: e.z,
                effective_layer_height: e.effective_layer_height,
            }).collect(),
        };
        let sdk_region_input = ::slicer_sdk::prepass_types::SeamPlanningView {
            regions: region_input
                .regions()
                .into_iter()
                .map(__slicer_seam_planning_region_from_wit)
                .collect(),
        };
        let mut sdk_output = ::slicer_sdk::prepass_builders::SeamPlanningOutput::new();
        let out = <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_seam_planning(
            &module, &sdk_objects, &sdk_layer_plan, &mut sdk_output, &ir_config,
            &sdk_region_input,
        );
        for __slicer_entry in sdk_output.entries() {
            let __slicer_wit_candidates: ::std::vec::Vec<ScoredSeamCandidate> = __slicer_entry
                .scored_candidates
                .iter()
                .map(|sc| ScoredSeamCandidate {
                    position: SeamPoint3WithWidth {
                        x: sc.position.x,
                        y: sc.position.y,
                        z: sc.position.z,
                        width: sc.position.width,
                        flow_factor: sc.position.flow_factor,
                        overhang_quartile: sc.position.overhang_quartile,
                    },
                    score: sc.score,
                    reason: SeamReason { tag: sc.reason.tag.clone() },
                })
                .collect();
            let __slicer_variant_chain = match __slicer_entry.variant_chain.iter().map(|(semantic, value)| {
                let value = match value {
                    ::slicer_ir::PaintValue::Flag(v) => PaintValue::Flag(*v),
                    ::slicer_ir::PaintValue::Scalar(v) => PaintValue::Scalar(*v),
                    ::slicer_ir::PaintValue::ToolIndex(v) => PaintValue::ToolIndex(*v),
                    ::slicer_ir::PaintValue::Custom(_) => {
                        return Err(::std::string::String::from(
                            "custom paint values cannot cross the WIT boundary as variant-chain identity",
                        ));
                    }
                };
                Ok((semantic.clone(), value))
            }).collect::<::std::result::Result<::std::vec::Vec<_>, ::std::string::String>>() {
                Ok(chain) => chain,
                Err(message) => return Err(ModuleError { code: 12, message, fatal: true }),
            };
            let __slicer_wit_entry = SeamPlanEntry {
                global_layer_index: __slicer_entry.global_layer_index,
                object_id: __slicer_entry.object_id.clone(),
                region_id: __slicer_entry.region_id.clone(),
                variant_chain: __slicer_variant_chain,
                chosen_position: SeamPoint3WithWidth {
                    x: __slicer_entry.chosen_position.x,
                    y: __slicer_entry.chosen_position.y,
                    z: __slicer_entry.chosen_position.z,
                    width: __slicer_entry.chosen_position.width,
                    flow_factor: __slicer_entry.chosen_position.flow_factor,
                    overhang_quartile: __slicer_entry.chosen_position.overhang_quartile,
                },
                chosen_wall_index: __slicer_entry.chosen_wall_index,
                scored_candidates: __slicer_wit_candidates,
            };
            if let Err(e) = output.push_seam_plan(&__slicer_wit_entry) {
                return Err(ModuleError { code: 11, message: e, fatal: true });
            }
        }
        match out {
            Ok(()) => Ok(()),
            Err(e) => Err(__slicer_error_out(e)),
        }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_prepass_seam_planning_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            use slicer::ir_handles::ir_handles::{PaintSemantic, PaintValue};
            use slicer::prepass_types::prepass_types::{
                LayerPlanView, MeshObjectView, PaintLayerView, PaintStrokeView, PaintValueView,
            };
            use slicer::prepass_seam_planning::seam_planning_types::{
                ScoredSeamCandidate, SeamPlanEntry, SeamPlanningOutput,
                SeamPlanningRegionInput, SeamPlanningView, SeamReason,
            };
            use slicer::types::geometry::{SeamPoint3WithWidth, ExPolygon};
            #preamble
            #helpers
            struct __SlicerPrepassSeamPlanningComponent;
            impl exports::slicer::prepass_seam_planning::seam_planning::Guest for __SlicerPrepassSeamPlanningComponent {
                fn run(
                    objects: Vec<MeshObjectView>,
                    layer_plan: LayerPlanView,
                    output: SeamPlanningOutput,
                    config: ConfigView,
                    region_input: SeamPlanningView,
                ) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerPrepassSeamPlanningComponent);
        }
    }
}

fn build_prepass_support_geometry_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit"
    );
    let preamble = emit_world_preamble("support-geometry-module", "support_geometry", &wit_inline);
    let profile_install = profile_install_stmt();
    let helpers = {
        let mesh = prepass_mesh_helpers();
        let geometry = prepass_geometry_helpers();
        quote! { #mesh #geometry }
    };
    let arm = quote! {
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::PrepassModule>::from_config(&ir_config) {
            Ok(m) => m,
            Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_objects: ::std::vec::Vec<::slicer_sdk::prepass_types::MeshObjectView> = objects
            .into_iter()
            .map(__slicer_mesh_object_from_wit)
            .collect();
        let sdk_layer_plan = ::slicer_sdk::prepass_types::LayerPlanView {
            layers: layer_plan.layers.iter().map(|e| ::slicer_sdk::prepass_types::LayerPlanViewEntry {
                global_layer_index: e.global_layer_index,
                z: e.z,
                effective_layer_height: e.effective_layer_height,
            }).collect(),
        };
        let sdk_region_segmentation = ::slicer_sdk::prepass_types::RegionSegmentationView {
            entries: region_segmentation.entries.iter().map(|e| ::slicer_sdk::prepass_types::RegionSegmentationViewEntry {
                object_id: e.object_id.clone(),
                layer_index: e.layer_index,
                region_ids: e.region_ids.clone(),
            }).collect(),
            region_support_configs: region_segmentation.region_support_configs.iter().map(|e| ::slicer_sdk::prepass_types::RegionSupportConfig {
                object_id: e.object_id.clone(),
                layer_index: e.layer_index,
                region_id: e.region_id.clone(),
                support_family: e.support_family.clone(),
                support_type: e.support_type.clone(),
            }).collect(),
        };
        let sdk_support_geometry = ::slicer_sdk::prepass_types::SupportGeometryView {
            entries: support_geometry.entries.iter().map(|e| ::slicer_sdk::prepass_types::SupportGeometryViewEntry {
                global_support_layer_index: e.global_support_layer_index,
                object_id: e.object_id.clone(),
                region_id: e.region_id.clone(),
                outlines: e.outlines.iter().map(|ep| __slicer_expolygon_from_wit(ep.clone())).collect(),
            }).collect(),
        };
        let sdk_support_analysis = ::slicer_sdk::prepass_types::SupportAnalysisView {
            candidates: support_analysis.candidates.iter().map(|e| ::slicer_sdk::prepass_types::SupportAnalysisCandidate {
                id: e.id, geometry: e.geometry.iter().map(|ep| __slicer_expolygon_from_wit(ep.clone())).collect(),
                object_id: e.object_id.clone(), region_id: e.region_id.clone(), global_layer_index: e.global_layer_index,
                z_units: e.z_units, enforced: e.enforced, blocked: e.blocked,
            }).collect(),
            model_occupancy: support_analysis.model_occupancy.iter().map(|e| ::slicer_sdk::prepass_types::SupportAnalysisGeometryEntry {
                global_support_layer_index: e.global_support_layer_index, object_id: e.object_id.clone(), region_id: e.region_id.clone(),
                polygons: e.polygons.iter().map(|ep| __slicer_expolygon_from_wit(ep.clone())).collect(),
            }).collect(),
            termination_surfaces: support_analysis.termination_surfaces.iter().map(|e| ::slicer_sdk::prepass_types::SupportAnalysisGeometryEntry {
                global_support_layer_index: e.global_support_layer_index, object_id: e.object_id.clone(), region_id: e.region_id.clone(),
                polygons: e.polygons.iter().map(|ep| __slicer_expolygon_from_wit(ep.clone())).collect(),
            }).collect(),
            shared_settings: support_analysis.shared_settings.clone(),
            baseline_feasible_envelope: support_analysis.baseline_feasible_envelope.iter().map(|ep| __slicer_expolygon_from_wit(ep.clone())).collect(),
            family_assignments: support_analysis.family_assignments.iter().map(|e| ::slicer_sdk::prepass_types::SupportFamilyAssignment {
                object_id: e.object_id.clone(), region_id: e.region_id.clone(), family_id: e.family_id.clone(),
            }).collect(),
            support_territory: support_analysis.support_territory.iter().map(|e| ::slicer_sdk::prepass_types::SupportAnalysisGeometryEntry {
                global_support_layer_index: e.global_support_layer_index, object_id: e.object_id.clone(), region_id: e.region_id.clone(),
                polygons: e.polygons.iter().map(|ep| __slicer_expolygon_from_wit(ep.clone())).collect(),
            }).collect(),
        };
        let mut sdk_output = ::slicer_sdk::prepass_builders::SupportGeometryOutput::new();
        let out = <#self_ty as ::slicer_sdk::traits::PrepassModule>::run_support_geometry_with_analysis(
            &module, &sdk_objects, &sdk_layer_plan, &sdk_region_segmentation, &sdk_support_analysis, &sdk_support_geometry, &mut sdk_output, &ir_config,
        );
        for __slicer_entry in sdk_output.entries() {
            let __slicer_wit_entry = SupportPlanEntry {
                global_layer_index: __slicer_entry.global_layer_index,
                object_id: __slicer_entry.object_id.clone(),
                region_id: __slicer_entry.region_id.clone(),
                family_id: __slicer_entry.family_id.clone(),
                demand_ids: __slicer_entry.demand_ids.clone(),
                body_ids: __slicer_entry.body_ids.clone(),
                anchor_layer_index: __slicer_entry.anchor_layer_index,
                anchor_z: __slicer_entry.anchor_z,
                roles: __slicer_entry.roles.iter().map(|role| SupportPlanRoleRegion {
                    role: match role.role {
                        ::slicer_ir::SupportPlanRole::SupportBody => SupportPlanRole::SupportBody,
                        ::slicer_ir::SupportPlanRole::TopInterface => SupportPlanRole::TopInterface,
                        ::slicer_ir::SupportPlanRole::BaseInterface => SupportPlanRole::BaseInterface,
                        ::slicer_ir::SupportPlanRole::BottomInterface => SupportPlanRole::BottomInterface,
                        ::slicer_ir::SupportPlanRole::RaftRelated => SupportPlanRole::RaftRelated,
                    },
                    regions: role.regions.iter().map(|ep| ExPolygon {
                        contour: Polygon { points: ep.contour.points.iter().map(|p| Point2 { x: p.x, y: p.y }).collect() },
                        holes: ep.holes.iter().map(|h| Polygon { points: h.points.iter().map(|p| Point2 { x: p.x, y: p.y }).collect() }).collect(),
                    }).collect(),
                }).collect(),
                skeleton: __slicer_entry.skeleton.as_ref().map(|s| SupportPlanSkeleton {
                    points: s.points.iter().map(|p| Point3 { x: p.x, y: p.y, z: p.z }).collect(),
                    wall_counts: s.wall_counts.clone(),
                }),
                capabilities: __slicer_entry.capabilities.clone(),
                provenance: __slicer_entry.provenance.clone(),
                decline_reason: __slicer_entry.decline_reason.map(|reason| match reason {
                    ::slicer_ir::SupportPlanDeclineReason::DeclinedPolicy => SupportPlanDeclineReason::DeclinedPolicy,
                    ::slicer_ir::SupportPlanDeclineReason::NoRoute => SupportPlanDeclineReason::NoRoute,
                    ::slicer_ir::SupportPlanDeclineReason::Blocked => SupportPlanDeclineReason::Blocked,
                    ::slicer_ir::SupportPlanDeclineReason::UnsupportedMode => SupportPlanDeclineReason::UnsupportedMode,
                }),
            };
            if let Err(e) = output.push_support_plan_entry(&__slicer_wit_entry) {
                return Err(ModuleError { code: 11, message: e, fatal: true });
            }
        }
        if let Some(__slicer_raft_plan) = sdk_output.raft_plan() {
            let __slicer_wit_raft_plan = RaftPlan {
                raft_layers: __slicer_raft_plan.raft_layers,
                raft_first_layer_density: __slicer_raft_plan.raft_first_layer_density,
                base_raft_layers: __slicer_raft_plan.base_raft_layers,
                interface_raft_layers: __slicer_raft_plan.interface_raft_layers,
            };
            if let Err(e) = output.push_raft_plan(__slicer_wit_raft_plan) {
                return Err(ModuleError { code: 13, message: e, fatal: true });
            }
        }
        for __slicer_diag in sdk_output.diagnostics() {
            let __slicer_wit_severity = match __slicer_diag.severity {
                ::slicer_sdk::prepass_types::DiagnosticSeverity::Trace => SeverityLevel::Trace,
                ::slicer_sdk::prepass_types::DiagnosticSeverity::Debug => SeverityLevel::Debug,
                ::slicer_sdk::prepass_types::DiagnosticSeverity::Info => SeverityLevel::Info,
                ::slicer_sdk::prepass_types::DiagnosticSeverity::Warn => SeverityLevel::Warn,
                ::slicer_sdk::prepass_types::DiagnosticSeverity::Error => SeverityLevel::Error,
            };
            let __slicer_wit_diag = Diagnostic {
                severity: __slicer_wit_severity,
                code: __slicer_diag.code,
                layer: __slicer_diag.layer,
                object_id: __slicer_diag.object_id.clone(),
                message: __slicer_diag.message.clone(),
            };
            if let Err(e) = output.push_diagnostic(&__slicer_wit_diag) {
                return Err(ModuleError { code: 12, message: e, fatal: true });
            }
        }
        match out {
            Ok(()) => Ok(()),
            Err(e) => Err(__slicer_error_out(e)),
        }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_prepass_support_geometry_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            // `layer-plan-view` is shared: it lives once in the unversioned
            // `slicer:prepass-types` package, alongside the mesh/paint views.
            use slicer::prepass_types::prepass_types::{
                LayerPlanView, LayerPlanViewEntry, MeshObjectView, PaintLayerView,
                PaintStrokeView, PaintValueView,
            };
            use slicer::prepass_support_geometry::support_geometry_types::{
                Diagnostic, ObjectId, RaftPlan,
                RegionId, RegionSegmentationView, RegionSegmentationViewEntry,
                SeverityLevel, SupportGeometryOutput, SupportGeometryView,
                SupportGeometryViewEntry, SupportAnalysisView, SupportPlanEntry, SupportPlanRole,
                SupportPlanRoleRegion, SupportPlanSkeleton, SupportPlanDeclineReason,
            };
            use slicer::types::geometry::{ExPolygon, Point2, Point3, Point3WithWidth, Polygon};
            #preamble
            #helpers
            struct __SlicerPrepassSupportGeometryComponent;
            impl exports::slicer::prepass_support_geometry::support_geometry::Guest for __SlicerPrepassSupportGeometryComponent {
                fn run(
                    objects: Vec<MeshObjectView>,
                    layer_plan: LayerPlanView,
                    region_segmentation: RegionSegmentationView,
                    support_analysis: SupportAnalysisView,
                    support_geometry: SupportGeometryView,
                    output: SupportGeometryOutput,
                    config: ConfigView,
                ) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerPrepassSupportGeometryComponent);
        }
    }
}

/// Per-stage WIT alias sets. Each per-stage `bindgen!` mod generates only
/// the types its WIT file references, so each macro-emitted `mod
/// __slicer_<stage>_world_export` must `use` only the types that the
/// per-stage WIT actually imports. The old `layer_wit_aliases()` helper
/// (carried over from the 163-era tier-world shape) referenced every
/// ir-handles type — that worked when one `world-layer.wit` imported
/// every interface, but the per-stage WITs each import only 4 ir-handles
/// types, so a single aliases block that references `PriorInfillRegion`
/// (only in `layer-infill-postprocess`) breaks the other 7 layer mods
/// at `PriorInfillRegion not found in ir_handles::ir_handles`. The shared
/// slice-input records used by the light adapters are imported only in the
/// light-stage fallback below; stage-specific records remain local to their
/// matching stage helpers.
///
/// Only the perimeters-postprocess, infill-postprocess, and
/// path-optimization bodies use `layer_glue_helpers()` (the helpers
/// carry WIT↔IR conversion code for the richer record types those
/// stages handle). The other 5 layer bodies use the lighter
/// `__slicer_adapt_*` helpers inline. So the per-stage aliases below
/// only bring in the rich type set for those 3 stages.
fn layer_per_stage_aliases(stage: &str) -> TokenStream2 {
    let ir_handles = match stage {
        "layer_perimeters_postprocess" => quote! {
            use self::slicer::ir_handles::ir_handles::{
                GcodeMoveCmd as WitGcodeMoveCmd, GcodeOutputBuilder,
                LayerCollectionBuilder, LayerIdx,
                MaterialBoundarySegment as WitMaterialBoundarySegment,
                OrderedEntityView as WitOrderedEntityView,
                PaintSemantic as WitPaintSemantic, PaintValue as WitPaintValue,
                PerimeterOutputBuilder, PerimeterRegionView,
                QuartileBand as WitQuartileBand, RetractMode as WitRetractMode,
                SeamCandidate as WitSeamCandidate, SeamPosition as WitSeamPosition,
                WallBoundaryType as WitWallBoundaryType,
                WallFeatureFlag as WitWallFeatureFlag,
                WallLoopType as WitWallLoopType, WallLoopView as WitWallLoopView,
            };
            use self::slicer::types::geometry::{
                ExPolygon as WitExPolygon, ExtrusionPath3d as WitExtrusionPath3d,
                ExtrusionRole as WitExtrusionRole, Point2 as WitPoint2, Point3 as WitPoint3,
                Point3WithWidth as WitPoint3WithWidth, Polygon as WitPolygon,
            };
        },
        "layer_infill_postprocess" => quote! {
            use self::slicer::ir_handles::ir_handles::{
                GcodeMoveCmd as WitGcodeMoveCmd,
                InfillOutputBuilder, LayerIdx, MaterialBoundarySegment as WitMaterialBoundarySegment,
                PerimeterRegionView, PriorInfillRegion,
                PaintValue as WitPaintValue, QuartileBand as WitQuartileBand,
                SeamCandidate as WitSeamCandidate, SeamPosition as WitSeamPosition,
                WallBoundaryType as WitWallBoundaryType, WallFeatureFlag as WitWallFeatureFlag,
                WallLoopType as WitWallLoopType, WallLoopView as WitWallLoopView,
            };
            use self::slicer::types::geometry::{
                ExPolygon as WitExPolygon, ExtrusionPath3d as WitExtrusionPath3d,
                ExtrusionRole as WitExtrusionRole, Point2 as WitPoint2, Point3 as WitPoint3,
                Point3WithWidth as WitPoint3WithWidth, Polygon as WitPolygon,
            };
        },
        "layer_path_optimization" => quote! {
            use self::slicer::ir_handles::ir_handles::{
                GcodeMoveCmd as WitGcodeMoveCmd, GcodeOutputBuilder,
                LayerCollectionBuilder, LayerIdx,
                MaterialBoundarySegment as WitMaterialBoundarySegment,
                OrderedEntityView as WitOrderedEntityView, PerimeterRegionView,
                PaintValue as WitPaintValue,
                QuartileBand as WitQuartileBand, RetractMode as WitRetractMode,
                SeamCandidate as WitSeamCandidate, SeamPosition as WitSeamPosition,
                SegmentAnnotationsEntry as WitSegmentAnnotationsEntry,
                SegmentAnnotationsPolygon as WitSegmentAnnotationsPolygon,
                SurfaceGroup as WitSurfaceGroup, WallBoundaryType as WitWallBoundaryType,
                WallFeatureFlag as WitWallFeatureFlag, WallLoopType as WitWallLoopType,
                WallLoopView as WitWallLoopView,
            };
            use self::slicer::types::geometry::{
                ExPolygon as WitExPolygon, ExtrusionPath3d as WitExtrusionPath3d,
                ExtrusionRole as WitExtrusionRole, Point2 as WitPoint2, Point3 as WitPoint3,
                Point3WithWidth as WitPoint3WithWidth, Polygon as WitPolygon,
            };
        },
        // 5 lighter mods (slice_postprocess, perimeters, infill, support,
        // support_postprocess) use only the WIT types their own WIT
        // references. The full ir-handles set is NOT imported here because
        // those types are not in this mod's per-stage WIT.
        _ => quote! {
            use self::slicer::ir_handles::ir_handles::{
                InfillOutputBuilder, LayerIdx, PaintRegionLayerView,
                SupportPlanEntryView as WitSupportPlanEntryView,
                SupportPlanViewRole as WitSupportPlanViewRole,
                SupportPlanViewRoleRegion as WitSupportPlanViewRoleRegion,
                SupportPlanViewSkeleton as WitSupportPlanViewSkeleton,
                SupportPlanViewDeclineReason as WitSupportPlanViewDeclineReason,
                PaintSemantic as WitPaintSemantic, PaintValue as WitPaintValue,
                PerimeterOutputBuilder, PerimeterRegionView,
                QuartileBand as WitQuartileBand,
                SegmentAnnotationsEntry as WitSegmentAnnotationsEntry,
                SegmentAnnotationsPolygon as WitSegmentAnnotationsPolygon,
                SlicePostprocessBuilder, SliceRegionView,
                SurfaceGroup as WitSurfaceGroup, SupportOutputBuilder,
            };
            use self::slicer::types::geometry::{
                ExPolygon as WitExPolygon, Point2 as WitPoint2, Point3, Polygon as WitPolygon,
            };
        },
    };
    quote! { #ir_handles }
}

/// Light helpers for the 5 lighter layer mods (slice_postprocess,
/// perimeters, infill, support, support_postprocess). The 3 heavy
/// mods (perimeters_postprocess, infill_postprocess,
/// path_optimization) use `layer_glue_helpers()` instead, which
/// includes the rich WIT↔IR conversion code for perimeter records and
/// post-processing inputs. The light helpers keep their conversion surface
/// limited to the shared slice and paint records exposed by those stages.
///
/// The light helpers provide only the adapters used by the five lighter layer
/// body sites. Stage-specific drain bodies and their WIT conversion helpers
/// are emitted by `layer_stage_helpers`.
fn layer_light_helpers() -> TokenStream2 {
    quote! {
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

        fn __slicer_wit_quartileband_to_ir(
            qb: &WitQuartileBand,
        ) -> ::slicer_ir::slice_ir::QuartileBand {
            ::slicer_ir::slice_ir::QuartileBand {
                quartile: qb.quartile,
                polygons: qb.polygons.iter().map(__slicer_wit_expolygon_to_ir).collect(),
            }
        }

        fn __slicer_wit_surfacegroup_to_ir(sg: &WitSurfaceGroup) -> ::slicer_ir::SurfaceGroup {
            ::slicer_ir::SurfaceGroup {
                id: sg.id,
                facet_indices: sg.facet_indices.clone(),
                z_min: sg.z_min,
                z_max: sg.z_max,
                area_mm2: sg.area_mm2,
                printable: sg.printable,
                shell_count: sg.shell_count,
            }
        }

        fn __slicer_wit_semantic_to_ir(s: &WitPaintSemantic) -> ::slicer_ir::PaintSemantic {
            match s {
                WitPaintSemantic::Material => ::slicer_ir::PaintSemantic::Material,
                WitPaintSemantic::FuzzySkin => ::slicer_ir::PaintSemantic::FuzzySkin,
                WitPaintSemantic::SupportEnforcer => ::slicer_ir::PaintSemantic::SupportEnforcer,
                WitPaintSemantic::SupportBlocker => ::slicer_ir::PaintSemantic::SupportBlocker,
                WitPaintSemantic::Custom(s) => ::slicer_ir::PaintSemantic::Custom(s.clone()),
            }
        }

        fn __slicer_wit_paintvalue_to_ir(v: &WitPaintValue) -> ::slicer_ir::PaintValue {
            match v {
                WitPaintValue::Flag(b) => ::slicer_ir::PaintValue::Flag(*b),
                WitPaintValue::Scalar(f) => ::slicer_ir::PaintValue::Scalar(*f),
                WitPaintValue::ToolIndex(i) => ::slicer_ir::PaintValue::ToolIndex(*i),
            }
        }

        fn __slicer_segment_annotations_to_ir(
            entries: &[WitSegmentAnnotationsEntry],
        ) -> ::std::collections::HashMap<
            ::slicer_ir::PaintSemantic,
            ::std::vec::Vec<::std::vec::Vec<::core::option::Option<::slicer_ir::PaintValue>>>,
        > {
            let mut map = ::std::collections::HashMap::new();
            for entry in entries {
                let semantic = __slicer_wit_semantic_to_ir(&entry.semantic);
                let polygons: ::std::vec::Vec<_> = entry
                    .polygons
                    .iter()
                    .map(|polygon: &WitSegmentAnnotationsPolygon| {
                        polygon
                            .values
                            .iter()
                            .map(|value| value.as_ref().map(__slicer_wit_paintvalue_to_ir))
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
                let polys: ::std::vec::Vec<::slicer_ir::ExPolygon> = r
                    .polygons()
                    .iter()
                    .map(__slicer_wit_expolygon_to_ir)
                    .collect();
                let infill: ::std::vec::Vec<::slicer_ir::ExPolygon> = r
                    .infill_areas()
                    .iter()
                    .map(__slicer_wit_expolygon_to_ir)
                    .collect();
                let segment_annotations =
                    __slicer_segment_annotations_to_ir(&r.segment_annotations());
                let region_id: ::slicer_ir::RegionId = r.region_id().parse().unwrap_or(0);
                let mut sdk_view = ::slicer_sdk::views::SliceRegionView::default();
                sdk_view.set_object_id(r.object_id());
                sdk_view.set_region_id(region_id);
                sdk_view.set_polygons(polys);
                sdk_view.set_infill_areas(infill);
                sdk_view.set_effective_layer_height(r.effective_layer_height());
                sdk_view.set_z(r.z());
                sdk_view.set_needs_support(r.needs_support());
                sdk_view.set_has_nonplanar(r.has_nonplanar());
                sdk_view.set_segment_annotations(segment_annotations);
                let variant_chain: ::std::vec::Vec<(
                    ::std::string::String,
                    ::slicer_ir::PaintValue,
                )> = r
                    .variant_chain()
                    .iter()
                    .map(|(name, value)| {
                        (name.clone(), __slicer_wit_paintvalue_to_ir(value))
                    })
                    .collect();
                sdk_view.set_variant_chain(variant_chain);
                sdk_view.set_top_shell_index(r.top_shell_index());
                sdk_view.set_bottom_shell_index(r.bottom_shell_index());
                let top_fill: ::std::vec::Vec<::slicer_ir::ExPolygon> = r
                    .top_solid_fill()
                    .iter()
                    .map(__slicer_wit_expolygon_to_ir)
                    .collect();
                let bottom_fill: ::std::vec::Vec<::slicer_ir::ExPolygon> = r
                    .bottom_solid_fill()
                    .iter()
                    .map(__slicer_wit_expolygon_to_ir)
                    .collect();
                let bridge_areas: ::std::vec::Vec<::slicer_ir::ExPolygon> = r
                    .bridge_areas()
                    .iter()
                    .map(__slicer_wit_expolygon_to_ir)
                    .collect();
                let internal_bridge_areas: ::std::vec::Vec<::slicer_ir::ExPolygon> = r
                    .internal_bridge_areas()
                    .iter()
                    .map(__slicer_wit_expolygon_to_ir)
                    .collect();
                // Ticket 19 (R1): this field was never marshalled, so every
                // guest saw an empty `internal_solid_fill` and the shell
                // shadow read as exposed top.
                let internal_solid_fill: ::std::vec::Vec<::slicer_ir::ExPolygon> = r
                    .internal_solid_fill()
                    .iter()
                    .map(__slicer_wit_expolygon_to_ir)
                    .collect();
                let sparse_infill_area: ::std::vec::Vec<::slicer_ir::ExPolygon> = r
                    .sparse_infill_area()
                    .iter()
                    .map(__slicer_wit_expolygon_to_ir)
                    .collect();
                sdk_view.set_top_solid_fill(
                    top_fill,
                );
                sdk_view.set_bottom_solid_fill(
                    bottom_fill,
                );
                sdk_view.set_is_bridge(r.is_bridge());
                sdk_view.set_bridge_areas(bridge_areas);
                sdk_view.set_internal_bridge_areas(internal_bridge_areas);
                sdk_view.set_internal_solid_fill(internal_solid_fill);
                sdk_view.set_bridge_orientation_deg(r.bridge_orientation_deg());
                sdk_view.set_sparse_infill_area(sparse_infill_area);
                sdk_view.set_held_claims(r.held_claims());
                let overhang_areas: ::std::vec::Vec<::slicer_ir::ExPolygon> = r
                    .overhang_areas()
                    .iter()
                    .map(__slicer_wit_expolygon_to_ir)
                    .collect();
                let overhang_quartile_polygons: ::std::vec::Vec<
                    ::slicer_ir::slice_ir::QuartileBand,
                > = r
                    .overhang_quartile_polygons()
                    .iter()
                    .map(__slicer_wit_quartileband_to_ir)
                    .collect();
                let prev_layer_boundary: ::std::vec::Vec<::slicer_ir::ExPolygon> = r
                    .prev_layer_boundary()
                    .iter()
                    .map(__slicer_wit_expolygon_to_ir)
                    .collect();
                sdk_view.set_overhang_areas(
                    overhang_areas,
                );
                sdk_view.set_overhang_quartile_polygons(overhang_quartile_polygons);
                sdk_view.set_prev_layer_boundary(prev_layer_boundary);
                sdk_view.set_surface_group(
                    r.surface_group()
                        .as_ref()
                        .map(__slicer_wit_surfacegroup_to_ir),
                );
                out.push(sdk_view);
            }
            out
        }

        fn __slicer_adapt_paint_layer(
            paint: &PaintRegionLayerView,
            keys: &[(::std::string::String, ::slicer_ir::RegionId)],
        ) -> ::slicer_sdk::traits::PaintRegionLayerView {
            let layer_idx = paint.layer_index() as u32;
            let sdk_paint = ::slicer_sdk::traits::PaintRegionLayerView::new(layer_idx)
                .with_support_plan(__slicer_support_plan_from_view(paint, layer_idx, keys));
            match __slicer_lightning_tree_from_view(paint, layer_idx, keys) {
                Some(ir) => sdk_paint.with_lightning_tree_ir(ir),
                None => sdk_paint,
            }
        }

        fn __slicer_lightning_tree_from_view(
            wit_paint: &PaintRegionLayerView,
            layer_idx: u32,
            keys: &[(::std::string::String, ::slicer_ir::RegionId)],
        ) -> ::std::option::Option<::std::sync::Arc<::slicer_ir::LightningTreeIR>> {
            let mut entries = ::std::vec::Vec::new();
            for (object_id, region_id) in keys.iter() {
                let region_id_str = region_id.to_string();
                let segments = wit_paint.lightning_tree_segments(object_id, &region_id_str);
                let tree_edge_segments: ::std::vec::Vec<[::slicer_ir::Point2; 2]> = segments
                    .into_iter()
                    .filter_map(|segment| {
                        let start = segment.first()?;
                        let end = segment.get(1)?;
                        Some([
                            ::slicer_ir::Point2 {
                                x: ::slicer_ir::mm_to_units(start.x),
                                y: ::slicer_ir::mm_to_units(start.y),
                            },
                            ::slicer_ir::Point2 {
                                x: ::slicer_ir::mm_to_units(end.x),
                                y: ::slicer_ir::mm_to_units(end.y),
                            },
                        ])
                    })
                    .collect();
                if tree_edge_segments.is_empty() {
                    continue;
                }
                entries.push(::slicer_ir::LightningTreeEntry {
                    object_id: object_id.clone(),
                    global_layer_index: layer_idx as i32,
                    region_id: *region_id,
                    tree_edge_segments,
                });
            }
            if entries.is_empty() {
                None
            } else {
                Some(::std::sync::Arc::new(::slicer_ir::LightningTreeIR {
                    entries,
                    ..::core::default::Default::default()
                }))
            }
        }

        fn __slicer_support_plan_from_view(
            wit_paint: &PaintRegionLayerView,
            layer_idx: u32,
            keys: &[(::std::string::String, ::slicer_ir::RegionId)],
        ) -> ::std::sync::Arc<::slicer_ir::SupportPlanIR> {
            let mut entries = ::std::vec::Vec::new();
            for (object_id, region_id) in keys.iter() {
                let region_id_str = region_id.to_string();
                for entry in wit_paint.support_plan_entries(object_id, &region_id_str) {
                    entries.push(::slicer_ir::SupportPlanEntry {
                        global_layer_index: entry.global_layer_index,
                        object_id: entry.object_id,
                        region_id: entry.region_id.parse().unwrap_or(*region_id),
                        family_id: entry.family_id,
                        demand_ids: entry.demand_ids,
                        body_ids: entry.body_ids,
                        anchor_layer_index: entry.anchor_layer_index,
                        anchor_z: entry.anchor_z,
                        roles: entry.roles.into_iter().map(|role| ::slicer_ir::SupportPlanRoleRegion {
                            role: match role.role {
                                WitSupportPlanViewRole::SupportBody => ::slicer_ir::SupportPlanRole::SupportBody,
                                WitSupportPlanViewRole::TopInterface => ::slicer_ir::SupportPlanRole::TopInterface,
                                WitSupportPlanViewRole::BaseInterface => ::slicer_ir::SupportPlanRole::BaseInterface,
                                WitSupportPlanViewRole::BottomInterface => ::slicer_ir::SupportPlanRole::BottomInterface,
                                WitSupportPlanViewRole::RaftRelated => ::slicer_ir::SupportPlanRole::RaftRelated,
                            },
                            regions: role.regions.iter().map(__slicer_wit_expolygon_to_ir).collect(),
                        }).collect(),
                        skeleton: entry.skeleton.map(|s| ::slicer_ir::SupportPlanSkeleton {
                            points: s.points.into_iter().map(|p| ::slicer_ir::Point3 { x: p.x, y: p.y, z: p.z }).collect(),
                            wall_counts: s.wall_counts,
                        }),
                        capabilities: entry.capabilities,
                        provenance: entry.provenance,
                        decline_reason: entry.decline_reason.map(|reason| match reason {
                            WitSupportPlanViewDeclineReason::DeclinedPolicy => ::slicer_ir::SupportPlanDeclineReason::DeclinedPolicy,
                            WitSupportPlanViewDeclineReason::NoRoute => ::slicer_ir::SupportPlanDeclineReason::NoRoute,
                            WitSupportPlanViewDeclineReason::Blocked => ::slicer_ir::SupportPlanDeclineReason::Blocked,
                            WitSupportPlanViewDeclineReason::UnsupportedMode => ::slicer_ir::SupportPlanDeclineReason::UnsupportedMode,
                        }),
                    });
                }
            }
            ::std::sync::Arc::new(::slicer_ir::SupportPlanIR { entries, ..::core::default::Default::default() })
        }

    }
}

fn layer_glue_helpers() -> TokenStream2 {
    quote! {
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
        fn __slicer_wit_role_to_ir(r: &WitExtrusionRole) -> ::slicer_ir::ExtrusionRole {
            match r {
                WitExtrusionRole::OuterWall => ::slicer_ir::ExtrusionRole::OuterWall,
                WitExtrusionRole::InnerWall => ::slicer_ir::ExtrusionRole::InnerWall,
                WitExtrusionRole::ThinWall => ::slicer_ir::ExtrusionRole::ThinWall,
                WitExtrusionRole::TopSolidInfill => ::slicer_ir::ExtrusionRole::TopSolidInfill,
                WitExtrusionRole::BottomSolidInfill => ::slicer_ir::ExtrusionRole::BottomSolidInfill,
                WitExtrusionRole::SparseInfill => ::slicer_ir::ExtrusionRole::SparseInfill,
                WitExtrusionRole::SupportMaterial => ::slicer_ir::ExtrusionRole::SupportMaterial,
                WitExtrusionRole::SupportInterface => ::slicer_ir::ExtrusionRole::SupportInterface,
                WitExtrusionRole::SupportBaseInterface => ::slicer_ir::ExtrusionRole::SupportBaseInterface,
                WitExtrusionRole::Ironing => ::slicer_ir::ExtrusionRole::Ironing,
                 WitExtrusionRole::BridgeInfill => ::slicer_ir::ExtrusionRole::BridgeInfill,
                 WitExtrusionRole::InternalBridgeInfill => ::slicer_ir::ExtrusionRole::InternalBridgeInfill,
                WitExtrusionRole::WipeTower => ::slicer_ir::ExtrusionRole::WipeTower,
                WitExtrusionRole::Custom(s) if s == "slicer.builtin/internal-solid-infill@1" => {
                    ::slicer_ir::ExtrusionRole::InternalSolidInfill
                }
                WitExtrusionRole::Custom(s) => ::slicer_ir::ExtrusionRole::Custom(s.clone()),
                WitExtrusionRole::GapFill => ::slicer_ir::ExtrusionRole::GapFill,
                WitExtrusionRole::RaftInfill => ::slicer_ir::ExtrusionRole::RaftInfill,
            }
        }
        fn __slicer_wit_point3w_to_ir(p: &WitPoint3WithWidth) -> ::slicer_ir::Point3WithWidth {
            ::slicer_ir::Point3WithWidth {
                x: p.x, y: p.y, z: p.z, width: p.width, flow_factor: p.flow_factor,
                overhang_quartile: p.overhang_quartile, dist_to_top_mm: 0.0,
                overhang_distance_mm: p.overhang_distance_mm,
            }
        }
        fn __slicer_wit_path_to_ir(p: &WitExtrusionPath3d) -> ::slicer_ir::ExtrusionPath3D {
            ::slicer_ir::ExtrusionPath3D {
                points: p.points.iter().map(__slicer_wit_point3w_to_ir).collect(),
                role: __slicer_wit_role_to_ir(&p.role), speed_factor: p.speed_factor,
                tool_index: p.tool_index,
                order_lock: p.order_lock,
            }
        }
        fn __slicer_wit_looptype_to_ir(lt: WitWallLoopType) -> ::slicer_ir::LoopType {
            match lt {
                WitWallLoopType::Outer => ::slicer_ir::LoopType::Outer,
                WitWallLoopType::Inner => ::slicer_ir::LoopType::Inner,
                WitWallLoopType::ThinWall => ::slicer_ir::LoopType::ThinWall,
                WitWallLoopType::NonplanarShell => ::slicer_ir::LoopType::NonPlanarShell,
                WitWallLoopType::GapFill => ::slicer_ir::LoopType::GapFill,
            }
        }
        fn __slicer_wit_paintvalue_to_ir(v: &WitPaintValue) -> ::slicer_ir::PaintValue {
            match v {
                WitPaintValue::Flag(b) => ::slicer_ir::PaintValue::Flag(*b),
                WitPaintValue::Scalar(f) => ::slicer_ir::PaintValue::Scalar(*f),
                WitPaintValue::ToolIndex(i) => ::slicer_ir::PaintValue::ToolIndex(*i),
            }
        }
        fn __slicer_wit_feature_to_ir(f: &WitWallFeatureFlag) -> ::slicer_ir::WallFeatureFlags {
            let custom = f.custom.iter()
                .map(|(k, v)| (k.clone(), __slicer_wit_paintvalue_to_ir(v)))
                .collect();
            ::slicer_ir::WallFeatureFlags {
                tool_index: f.tool_index, fuzzy_skin: f.fuzzy_skin,
                is_bridge: f.is_bridge, is_thin_wall: f.is_thin_wall,
                skip_ironing: f.skip_ironing, custom,
            }
        }
        fn __slicer_wit_material_boundary_segment_to_ir(
            seg: &WitMaterialBoundarySegment,
        ) -> ::slicer_ir::MaterialBoundarySegment {
            ::slicer_ir::MaterialBoundarySegment {
                point_range: seg.point_range_start..seg.point_range_end,
                near_tool: seg.near_tool, far_tool: seg.far_tool,
            }
        }
        fn __slicer_wit_boundarytype_to_ir(bt: &WitWallBoundaryType) -> ::slicer_ir::WallBoundaryType {
            match bt {
                WitWallBoundaryType::ExteriorSurface => ::slicer_ir::WallBoundaryType::ExteriorSurface,
                WitWallBoundaryType::Interior => ::slicer_ir::WallBoundaryType::Interior,
                WitWallBoundaryType::MaterialBoundary(segments) => {
                    ::slicer_ir::WallBoundaryType::MaterialBoundary {
                        segments: segments.iter().map(__slicer_wit_material_boundary_segment_to_ir).collect(),
                    }
                }
            }
        }
        fn __slicer_wit_wallloop_to_ir(w: &WitWallLoopView) -> ::slicer_ir::WallLoop {
            let ir_path = __slicer_wit_path_to_ir(&w.path);
            let n_pts = ir_path.points.len();
            ::slicer_ir::WallLoop {
                perimeter_index: w.perimeter_index,
                loop_type: __slicer_wit_looptype_to_ir(w.loop_type),
                path: ir_path,
                width_profile: ::slicer_ir::WidthProfile {
                    widths: (0..n_pts).map(|_| 0.4_f32).collect(),
                },
                feature_flags: w.feature_flags.iter().map(__slicer_wit_feature_to_ir).collect(),
                boundary_type: __slicer_wit_boundarytype_to_ir(&w.boundary_type),
            }
        }
        fn __slicer_adapt_seam_position(sp: WitSeamPosition) -> ::slicer_ir::SeamPosition {
            ::slicer_ir::SeamPosition {
                point: __slicer_wit_point3w_to_ir(&sp.point), wall_index: sp.wall_index,
            }
        }
        fn __slicer_adapt_seam_candidate(sc: &WitSeamCandidate) -> ::slicer_ir::SeamCandidate {
            ::slicer_ir::SeamCandidate {
                position: ::slicer_ir::Point3WithWidth {
                    x: sc.position.x, y: sc.position.y, z: sc.position.z,
                    width: 0.0, flow_factor: 1.0, overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                score: sc.score, reason: ::slicer_ir::SeamReason::Aligned,
            }
        }
        fn __slicer_adapt_perimeter_regions(
            regions: &[PerimeterRegionView],
        ) -> ::std::vec::Vec<::slicer_sdk::views::PerimeterRegionView> {
            let mut out = ::std::vec::Vec::with_capacity(regions.len());
            for r in regions.iter() {
                let walls = r.wall_loops().iter().map(__slicer_wit_wallloop_to_ir).collect();
                let infill = r.infill_areas().iter().map(__slicer_wit_expolygon_to_ir).collect();
                let region_id: ::slicer_ir::RegionId = r.region_id().parse().unwrap_or(0);
                let resolved_seam = r.resolved_seam().map(__slicer_adapt_seam_position);
                let seam_candidates = r.seam_candidates().iter().map(__slicer_adapt_seam_candidate).collect();
                let mut perimeter_view = ::slicer_sdk::views::PerimeterRegionView::default();
                perimeter_view.set_object_id(r.object_id());
                perimeter_view.set_region_id(region_id);
                perimeter_view.set_wall_loops(walls);
                perimeter_view.set_infill_areas(infill);
                perimeter_view.set_seam_candidates(seam_candidates);
                perimeter_view.set_resolved_seam(resolved_seam);
                perimeter_view.set_sparse_infill_area(r.sparse_infill_area().iter().map(__slicer_wit_expolygon_to_ir).collect());
                perimeter_view.set_top_solid_fill(r.top_solid_fill().iter().map(__slicer_wit_expolygon_to_ir).collect());
                perimeter_view.set_bottom_solid_fill(r.bottom_solid_fill().iter().map(__slicer_wit_expolygon_to_ir).collect());
                perimeter_view.set_bridge_areas(r.bridge_areas().iter().map(__slicer_wit_expolygon_to_ir).collect());
                perimeter_view.set_tool_index(r.tool_index());
                perimeter_view.set_wall_source_region_id(r.wall_source_region_id().map(|s| s.parse().unwrap_or(0)));
                out.push(perimeter_view);
            }
            out
        }
    }
}

/// Stage-specific drain bodies and the WIT conversion helpers they require.
/// Keeping them out of the general helpers prevents unrelated per-stage
/// bindgen modules from needing these types in scope.
fn layer_stage_helpers(stage: &str) -> TokenStream2 {
    let ir_role_and_path_helpers = quote! {
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
                ::slicer_ir::ExtrusionRole::SupportBaseInterface => WitExtrusionRole::SupportBaseInterface,
                ::slicer_ir::ExtrusionRole::Ironing => WitExtrusionRole::Ironing,
                 ::slicer_ir::ExtrusionRole::BridgeInfill => WitExtrusionRole::BridgeInfill,
                 ::slicer_ir::ExtrusionRole::InternalBridgeInfill => WitExtrusionRole::InternalBridgeInfill,
                ::slicer_ir::ExtrusionRole::WipeTower => WitExtrusionRole::WipeTower,
                ::slicer_ir::ExtrusionRole::Custom(s) => WitExtrusionRole::Custom(s.clone()),
                ::slicer_ir::ExtrusionRole::PrimeTower => WitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/prime-tower@1")),
                ::slicer_ir::ExtrusionRole::Skirt => WitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/skirt@1")),
                ::slicer_ir::ExtrusionRole::Brim => WitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/brim@1")),
                ::slicer_ir::ExtrusionRole::InternalSolidInfill => WitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/internal-solid-infill@1")),
                ::slicer_ir::ExtrusionRole::GapFill => WitExtrusionRole::GapFill,
                ::slicer_ir::ExtrusionRole::RaftInfill => WitExtrusionRole::RaftInfill,
                _ => WitExtrusionRole::OuterWall,
            }
        }

        fn __slicer_ir_path_to_wit(p: &::slicer_ir::ExtrusionPath3D) -> WitExtrusionPath3d {
            WitExtrusionPath3d {
                points: p.points.iter().map(|pt| WitPoint3WithWidth {
                    x: pt.x, y: pt.y, z: pt.z, width: pt.width, flow_factor: pt.flow_factor,
                    overhang_quartile: pt.overhang_quartile,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: pt.overhang_distance_mm,
                }).collect(),
                role: __slicer_ir_role_to_wit(&p.role),
                speed_factor: p.speed_factor,
                tool_index: p.tool_index,
                order_lock: p.order_lock,
            }
        }
    };

    let ir_expolygon_helpers = quote! {
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
    };

    let slice_postprocess_helpers = quote! {
        #ir_expolygon_helpers

        fn __slicer_ir_region_key_to_wit(k: &::slicer_ir::RegionKey) -> Option<WitRegionKey> {
            let variant_chain = k.variant_chain.iter().map(|(semantic, value)| {
                let value = match value {
                    ::slicer_ir::PaintValue::Flag(v) => WitPaintValue::Flag(*v),
                    ::slicer_ir::PaintValue::Scalar(v) => WitPaintValue::Scalar(*v),
                    ::slicer_ir::PaintValue::ToolIndex(v) => WitPaintValue::ToolIndex(*v),
                    ::slicer_ir::PaintValue::Custom(_) => return None,
                };
                Some((semantic.clone(), value))
            }).collect::<Option<Vec<_>>>()?;
            Some(WitRegionKey {
                variant_chain,
                layer_index: k.global_layer_index as i32,
                object_id: k.object_id.clone(),
                region_id: k.region_id.to_string(),
            })
        }
    };

    let perimeter_helpers = quote! {
        #ir_role_and_path_helpers
        #ir_expolygon_helpers

        fn __slicer_ir_looptype_to_wit(lt: &::slicer_ir::LoopType) -> WitWallLoopType {
            match lt {
                ::slicer_ir::LoopType::Outer => WitWallLoopType::Outer,
                ::slicer_ir::LoopType::Inner => WitWallLoopType::Inner,
                ::slicer_ir::LoopType::ThinWall => WitWallLoopType::ThinWall,
                ::slicer_ir::LoopType::NonPlanarShell => WitWallLoopType::NonplanarShell,
                ::slicer_ir::LoopType::GapFill => WitWallLoopType::GapFill,
                _ => WitWallLoopType::Outer,
            }
        }

        fn __slicer_ir_paintvalue_to_wit(v: &::slicer_ir::PaintValue) -> WitPaintValue {
            match v {
                ::slicer_ir::PaintValue::Flag(b) => WitPaintValue::Flag(*b),
                ::slicer_ir::PaintValue::Scalar(f) => WitPaintValue::Scalar(*f),
                ::slicer_ir::PaintValue::ToolIndex(i) => WitPaintValue::ToolIndex(*i),
                ::slicer_ir::PaintValue::Custom(_) => unreachable!("PaintValue::Custom rides on the paint-region transport (paint-value-input variant); it cannot appear in the boundary-paint read path"),
            }
        }

        fn __slicer_ir_feature_to_wit(f: &::slicer_ir::WallFeatureFlags) -> WitWallFeatureFlag {
            let mut custom_entries: ::std::vec::Vec<_> = f
                .custom
                .iter()
                .map(|(k, v)| (k.clone(), __slicer_ir_paintvalue_to_wit(v)))
                .collect();
            custom_entries.sort_by(|a, b| a.0.cmp(&b.0));
            WitWallFeatureFlag {
                tool_index: f.tool_index,
                fuzzy_skin: f.fuzzy_skin,
                is_bridge: f.is_bridge,
                is_thin_wall: f.is_thin_wall,
                skip_ironing: f.skip_ironing,
                custom: custom_entries,
            }
        }

        fn __slicer_ir_material_boundary_segment_to_wit(
            seg: &::slicer_ir::MaterialBoundarySegment,
        ) -> WitMaterialBoundarySegment {
            WitMaterialBoundarySegment {
                point_range_start: seg.point_range.start,
                point_range_end: seg.point_range.end,
                near_tool: seg.near_tool,
                far_tool: seg.far_tool,
            }
        }

        fn __slicer_ir_boundarytype_to_wit(bt: &::slicer_ir::WallBoundaryType) -> WitWallBoundaryType {
            match bt {
                ::slicer_ir::WallBoundaryType::ExteriorSurface => WitWallBoundaryType::ExteriorSurface,
                ::slicer_ir::WallBoundaryType::Interior => WitWallBoundaryType::Interior,
                ::slicer_ir::WallBoundaryType::MaterialBoundary { segments } => {
                    WitWallBoundaryType::MaterialBoundary(
                        segments.iter().map(__slicer_ir_material_boundary_segment_to_wit).collect(),
                    )
                }
            }
        }

        fn __slicer_ir_wallloop_to_wit(w: &::slicer_ir::WallLoop) -> WitWallLoopView {
            WitWallLoopView {
                perimeter_index: w.perimeter_index,
                loop_type: __slicer_ir_looptype_to_wit(&w.loop_type),
                path: __slicer_ir_path_to_wit(&w.path),
                feature_flags: w.feature_flags.iter().map(__slicer_ir_feature_to_wit).collect(),
                boundary_type: __slicer_ir_boundarytype_to_wit(&w.boundary_type),
            }
        }
    };

    let drain_slice_postprocess = quote! {
        fn __slicer_drain_slice_postprocess(
            sdk: &::slicer_sdk::builders::SlicePostprocessBuilder,
            wit: &SlicePostprocessBuilder,
        ) {
            for (key, polys) in sdk.polygon_updates() {
                let wit_polys: ::std::vec::Vec<WitExPolygon> =
                    polys.iter().map(__slicer_ir_expolygon_to_wit).collect();
                if let Some(wit_key) = __slicer_ir_region_key_to_wit(key) {
                    let _ = wit.set_polygons(&wit_key, &wit_polys);
                }
            }
            for (key, path_idx, vertex_idx, z) in sdk.path_z_updates() {
                if let Some(wit_key) = __slicer_ir_region_key_to_wit(key) {
                    let _ = wit.set_path_z(&wit_key, *path_idx, *vertex_idx, *z);
                }
            }
        }
    };

    let drain_perimeter = quote! {
        fn __slicer_drain_perimeter(
            sdk: &::slicer_sdk::builders::PerimeterOutputBuilder,
            wit: &PerimeterOutputBuilder,
        ) {
            let wall_loops = sdk.wall_loops();
            let wall_loop_origins = sdk.wall_loop_origins();
            for (i, w) in wall_loops.iter().enumerate() {
                if let Some(origin) = &wall_loop_origins[i] {
                    let _ = wit.set_current_origin(&origin.object_id, &origin.region_id.to_string());
                }
                let _ = wit.push_wall_loop(&__slicer_ir_wallloop_to_wit(w));
            }
            // Preserve each SDK set-infill-areas call's origin while draining.
            let infill_areas = sdk.infill_areas();
            let infill_areas_origins = sdk.infill_areas_origins();
            for (i, call_areas) in infill_areas.iter().enumerate() {
                let areas: ::std::vec::Vec<WitExPolygon> =
                    call_areas.iter().map(__slicer_ir_expolygon_to_wit).collect();
                if !areas.is_empty() {
                    if let Some(origin) = &infill_areas_origins[i] {
                        let _ = wit.set_current_origin(&origin.object_id, &origin.region_id.to_string());
                    }
                    let _ = wit.set_infill_areas(&areas);
                }
            }
            let seam_candidates = sdk.seam_candidates();
            let seam_candidate_origins = sdk.seam_candidate_origins();
            for (i, (pos, score)) in seam_candidates.iter().enumerate() {
                if let Some(origin) = &seam_candidate_origins[i] {
                    let _ = wit.set_current_origin(&origin.object_id, &origin.region_id.to_string());
                }
                if wit
                    .push_seam_candidate(
                        WitPoint3 { x: pos.x as f32, y: pos.y as f32, z: pos.z as f32 },
                        *score,
                    )
                    .is_err()
                {
                    ::slicer_sdk::host::log_warn(&::std::format!(
                        "seam candidate at ({}, {}, {}) rejected by host and dropped",
                        pos.x, pos.y, pos.z
                    ));
                }
            }
            let rotated_wall_loops = sdk.rotated_wall_loops();
            let rotated_wall_loop_origins = sdk.rotated_wall_loop_origins();
            for (i, (pos, wall_index, loop_)) in rotated_wall_loops.iter().enumerate() {
                if let Some(origin) = &rotated_wall_loop_origins[i] {
                    let _ = wit.set_current_origin(&origin.object_id, &origin.region_id.to_string());
                }
                let _ = wit.push_reordered_wall_loop(
                    WitPoint3WithWidth {
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                        width: pos.width,
                        flow_factor: pos.flow_factor,
                        overhang_quartile: pos.overhang_quartile,
                        dist_to_top_mm: 0.0,
                        overhang_distance_mm: pos.overhang_distance_mm,
                    },
                    *wall_index,
                    &__slicer_ir_wallloop_to_wit(loop_),
                );
            }
        }
    };

    let drain_infill = quote! {
        fn __slicer_drain_infill(
            sdk: &::slicer_sdk::builders::InfillOutputBuilder,
            wit: &InfillOutputBuilder,
        ) {
            let sparse = sdk.sparse_paths();
            let sparse_origins = sdk.sparse_path_origins();
            for (i, p) in sparse.iter().enumerate() {
                if let Some(origin) = &sparse_origins[i] {
                    let _ = wit.set_current_origin(&origin.object_id, &origin.region_id.to_string());
                }
                let _ = wit.push_sparse_path(&__slicer_ir_path_to_wit(p));
            }
            let solid = sdk.solid_paths();
            let solid_origins = sdk.solid_path_origins();
            for (i, p) in solid.iter().enumerate() {
                if let Some(origin) = &solid_origins[i] {
                    let _ = wit.set_current_origin(&origin.object_id, &origin.region_id.to_string());
                }
                let _ = wit.push_solid_path(&__slicer_ir_path_to_wit(p));
            }
            let ironing = sdk.ironing_paths();
            let ironing_origins = sdk.ironing_path_origins();
            for (i, p) in ironing.iter().enumerate() {
                if let Some(origin) = &ironing_origins[i] {
                    let _ = wit.set_current_origin(&origin.object_id, &origin.region_id.to_string());
                }
                let _ = wit.push_ironing_path(&__slicer_ir_path_to_wit(p));
            }
        }
    };

    let drain_support = quote! {
        fn __slicer_drain_support(
            sdk: &::slicer_sdk::builders::SupportOutputBuilder,
            wit: &SupportOutputBuilder,
        ) {
            let support_origins = sdk.support_path_origins();
            for (i, p) in sdk.support_paths().iter().enumerate() {
                if let Some(origin) = &support_origins[i] {
                    let _ = wit.set_current_origin(&origin.object_id, &origin.region_id.to_string());
                }
                let _ = wit.push_support_path(&__slicer_ir_path_to_wit(p));
            }
            let interface_origins = sdk.interface_path_origins();
            for (i, (p, top)) in sdk.interface_paths().iter().enumerate() {
                if let Some(origin) = &interface_origins[i] {
                    let _ = wit.set_current_origin(&origin.object_id, &origin.region_id.to_string());
                }
                let _ = wit.push_interface_path(&__slicer_ir_path_to_wit(p), *top);
            }
            let raft_origins = sdk.raft_path_origins();
            for (i, p) in sdk.raft_paths().iter().enumerate() {
                if let Some(origin) = &raft_origins[i] {
                    let _ = wit.set_current_origin(&origin.object_id, &origin.region_id.to_string());
                }
                let _ = wit.push_raft_path(&__slicer_ir_path_to_wit(p));
            }
        }
    };

    let anchored_helpers = quote! {
        use self::slicer::ir_handles::ir_handles::{
            AnchoredEntity as WitAnchoredEntity,
            AnchoredEntityProvenance as WitAnchoredEntityProvenance,
            AnchoredEventRuntimeHooks as WitAnchoredEventRuntimeHooks,
            AnchoredGeometryContract as WitAnchoredGeometryContract,
            OrderedEventCollection as WitOrderedEventCollection,
        };
        use self::slicer::types::geometry::{
            ExtrusionRole as AnchoredWitExtrusionRole,
            Point3WithWidth as AnchoredWitPoint3WithWidth,
        };

        fn __slicer_ir_anchored_role_to_wit(
            role: &::slicer_ir::ExtrusionRole,
        ) -> AnchoredWitExtrusionRole {
            match role {
                ::slicer_ir::ExtrusionRole::OuterWall => AnchoredWitExtrusionRole::OuterWall,
                ::slicer_ir::ExtrusionRole::InnerWall => AnchoredWitExtrusionRole::InnerWall,
                ::slicer_ir::ExtrusionRole::ThinWall => AnchoredWitExtrusionRole::ThinWall,
                ::slicer_ir::ExtrusionRole::TopSolidInfill => AnchoredWitExtrusionRole::TopSolidInfill,
                ::slicer_ir::ExtrusionRole::BottomSolidInfill => AnchoredWitExtrusionRole::BottomSolidInfill,
                ::slicer_ir::ExtrusionRole::InternalSolidInfill => AnchoredWitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/internal-solid-infill@1")),
                ::slicer_ir::ExtrusionRole::SparseInfill => AnchoredWitExtrusionRole::SparseInfill,
                ::slicer_ir::ExtrusionRole::RaftInfill => AnchoredWitExtrusionRole::RaftInfill,
                ::slicer_ir::ExtrusionRole::SupportMaterial => AnchoredWitExtrusionRole::SupportMaterial,
                ::slicer_ir::ExtrusionRole::SupportInterface => AnchoredWitExtrusionRole::SupportInterface,
                ::slicer_ir::ExtrusionRole::SupportBaseInterface => AnchoredWitExtrusionRole::SupportBaseInterface,
                ::slicer_ir::ExtrusionRole::WipeTower => AnchoredWitExtrusionRole::WipeTower,
                ::slicer_ir::ExtrusionRole::PrimeTower => AnchoredWitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/prime-tower@1")),
                ::slicer_ir::ExtrusionRole::Ironing => AnchoredWitExtrusionRole::Ironing,
                ::slicer_ir::ExtrusionRole::BridgeInfill => AnchoredWitExtrusionRole::BridgeInfill,
                ::slicer_ir::ExtrusionRole::InternalBridgeInfill => AnchoredWitExtrusionRole::InternalBridgeInfill,
                ::slicer_ir::ExtrusionRole::Skirt => AnchoredWitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/skirt@1")),
                ::slicer_ir::ExtrusionRole::Brim => AnchoredWitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/brim@1")),
                ::slicer_ir::ExtrusionRole::Custom(value) => AnchoredWitExtrusionRole::Custom(value.clone()),
                ::slicer_ir::ExtrusionRole::GapFill => AnchoredWitExtrusionRole::GapFill,
            }
        }

        fn __slicer_ir_anchored_collection_to_wit(
            collection: &::slicer_ir::OrderedEventCollection,
        ) -> WitOrderedEventCollection {
            WitOrderedEventCollection {
                anchor_global_layer_index: collection.anchor_global_layer_index,
                events: collection.events.iter().map(|event| WitAnchoredEntity {
                    local_id: event.local_id,
                    anchor_global_layer_index: event.anchor_global_layer_index,
                    geometry: match event.geometry {
                        ::slicer_ir::AnchoredGeometryContract::Planar { z } =>
                            WitAnchoredGeometryContract::Planar(z),
                        ::slicer_ir::AnchoredGeometryContract::ZSpanning { min_z, max_z } =>
                            WitAnchoredGeometryContract::ZSpanning((min_z, max_z)),
                    },
                    input_capabilities: event.input_capabilities.clone(),
                    output_capabilities: event.output_capabilities.clone(),
                    provenance: WitAnchoredEntityProvenance {
                        requesting_feature: event.provenance.requesting_feature.clone(),
                        source_plan_entry: event.provenance.source_plan_entry.clone(),
                    },
                    path_points: event.path_points.iter().map(|point| AnchoredWitPoint3WithWidth {
                        x: point.x,
                        y: point.y,
                        z: point.z,
                        width: point.width,
                        flow_factor: point.flow_factor,
                        overhang_quartile: point.overhang_quartile,
                        dist_to_top_mm: point.dist_to_top_mm,
                        overhang_distance_mm: point.overhang_distance_mm,
                    }).collect(),
                    role: __slicer_ir_anchored_role_to_wit(&event.role),
                }).collect(),
                runtime_hooks: WitAnchoredEventRuntimeHooks {
                    optimize_paths: collection.runtime_hooks.optimize_paths,
                    account_cooling: collection.runtime_hooks.account_cooling,
                    account_time: collection.runtime_hooks.account_time,
                },
            }
        }

        fn __slicer_drain_anchored_collection(
            sdk: &::slicer_sdk::LayerCollectionBuilder,
            wit: &LayerCollectionBuilder,
        ) -> ::std::result::Result<(), ::std::string::String> {
            if let Some(collection) = sdk.anchored_proposal() {
                wit.set_anchored_event_collection(
                    &__slicer_ir_anchored_collection_to_wit(collection),
                )?;
            }
            Ok(())
        }
    };

    match stage {
        "layer_slice_postprocess" => quote! {
            use self::slicer::ir_handles::ir_handles::{
                RegionKey as WitRegionKey,
            };
            #slice_postprocess_helpers
            #drain_slice_postprocess
        },
        "layer_perimeters" => quote! {
            use self::slicer::ir_handles::ir_handles::{
                MaterialBoundarySegment as WitMaterialBoundarySegment,
                WallBoundaryType as WitWallBoundaryType,
                WallFeatureFlag as WitWallFeatureFlag, WallLoopType as WitWallLoopType,
                WallLoopView as WitWallLoopView,
            };
            use self::slicer::types::geometry::{
                ExtrusionPath3d as WitExtrusionPath3d,
                ExtrusionRole as WitExtrusionRole, Point3 as WitPoint3,
                Point3WithWidth as WitPoint3WithWidth,
            };
            #perimeter_helpers
            #drain_perimeter
        },
        "layer_perimeters_postprocess" => quote! {
            #perimeter_helpers
            #drain_perimeter
        },
        "layer_infill" => quote! {
            use self::slicer::types::geometry::{
                ExtrusionPath3d as WitExtrusionPath3d, ExtrusionRole as WitExtrusionRole,
                Point3WithWidth as WitPoint3WithWidth,
            };
            #ir_role_and_path_helpers
            #drain_infill
        },
        "layer_infill_postprocess" => quote! {
            #ir_role_and_path_helpers
            #drain_infill
        },
        "layer_support" => quote! {
            use self::slicer::types::geometry::{
                ExtrusionPath3d as WitExtrusionPath3d, ExtrusionRole as WitExtrusionRole,
                Point3WithWidth as WitPoint3WithWidth,
            };
            #ir_role_and_path_helpers
            #drain_support
            #anchored_helpers
        },
        "layer_support_postprocess" => quote! {
            use self::slicer::types::geometry::{
                ExtrusionPath3d as WitExtrusionPath3d, ExtrusionRole as WitExtrusionRole,
                Point3WithWidth as WitPoint3WithWidth,
            };
            #ir_role_and_path_helpers
            #drain_support
        },
        "layer_anchored_events" => quote! {
            #anchored_helpers
        },
        "layer_path_optimization" => quote! {
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
                    ::slicer_ir::ExtrusionRole::SupportBaseInterface => WitExtrusionRole::SupportBaseInterface,
                    ::slicer_ir::ExtrusionRole::Ironing => WitExtrusionRole::Ironing,
                    ::slicer_ir::ExtrusionRole::BridgeInfill => WitExtrusionRole::BridgeInfill,
                    ::slicer_ir::ExtrusionRole::InternalBridgeInfill => WitExtrusionRole::InternalBridgeInfill,
                    ::slicer_ir::ExtrusionRole::WipeTower => WitExtrusionRole::WipeTower,
                    ::slicer_ir::ExtrusionRole::Custom(s) => WitExtrusionRole::Custom(s.clone()),
                    ::slicer_ir::ExtrusionRole::PrimeTower => WitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/prime-tower@1")),
                    ::slicer_ir::ExtrusionRole::Skirt => WitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/skirt@1")),
                    ::slicer_ir::ExtrusionRole::Brim => WitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/brim@1")),
                    ::slicer_ir::ExtrusionRole::InternalSolidInfill => WitExtrusionRole::Custom(::std::string::String::from("slicer.builtin/internal-solid-infill@1")),
                    ::slicer_ir::ExtrusionRole::GapFill => WitExtrusionRole::GapFill,
                    ::slicer_ir::ExtrusionRole::RaftInfill => WitExtrusionRole::RaftInfill,
                    _ => WitExtrusionRole::OuterWall,
                }
            }
            fn __slicer_retract_mode_ir_to_wit_layer(mode: &::slicer_ir::RetractMode) -> WitRetractMode {
                match mode {
                    ::slicer_ir::RetractMode::Gcode => WitRetractMode::Gcode,
                    ::slicer_ir::RetractMode::Firmware => WitRetractMode::Firmware,
                }
            }
            fn __slicer_drain_gcode(
                sdk: &::slicer_sdk::postpass_builders::GcodeOutputBuilder,
                wit: &GcodeOutputBuilder,
            ) {
                for cmd in sdk.commands() {
                    match cmd {
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(::slicer_sdk::postpass_types::GcodeCommand::Move { x, y, z, e, f, role }) => {
                            let _ = wit.push_move(&WitGcodeMoveCmd { x: *x, y: *y, z: *z, e: *e, f: *f, role: __slicer_ir_role_to_wit(role) });
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(::slicer_sdk::postpass_types::GcodeCommand::Retract { length, speed, mode }) => {
                            let _ = wit.push_retract(*length, *speed, __slicer_retract_mode_ir_to_wit_layer(mode));
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(::slicer_sdk::postpass_types::GcodeCommand::Unretract { length, speed, mode }) => {
                            let _ = wit.push_unretract(*length, *speed, __slicer_retract_mode_ir_to_wit_layer(mode));
                        }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(::slicer_sdk::postpass_types::GcodeCommand::FanSpeed { value }) => { let _ = wit.push_fan_speed(*value); }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(::slicer_sdk::postpass_types::GcodeCommand::Temperature { tool, celsius, wait }) => { let _ = wit.push_temperature(*tool, *celsius, *wait); }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(::slicer_sdk::postpass_types::GcodeCommand::ToolChange { after_entity_index, from, to }) => { let _ = wit.push_tool_change(*after_entity_index, *from, *to); }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(::slicer_sdk::postpass_types::GcodeCommand::Comment { text }) => { let _ = wit.push_comment(text); }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(::slicer_sdk::postpass_types::GcodeCommand::Raw { text }) => { let _ = wit.push_raw(text); }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::Command(::slicer_sdk::postpass_types::GcodeCommand::ExtrusionMode { absolute }) => { let _ = wit.push_raw(&if *absolute { "M82\n".to_string() } else { "M83\n".to_string() }); }
                        ::slicer_sdk::postpass_types::GcodeOutputCommand::ZHop { after_entity_index, hop_height } => { let _ = wit.push_z_hop(*after_entity_index, *hop_height); }
                    }
                }
            }
            fn __slicer_drain_layer_collection(
                sdk: &::slicer_sdk::LayerCollectionBuilder,
                wit: &LayerCollectionBuilder,
            ) {
                if let Some(items) = sdk.proposal() { let _ = wit.set_entity_order(items); }
            }
            fn __slicer_populate_layer_collection(
                wit: &LayerCollectionBuilder,
                sdk: &mut ::slicer_sdk::LayerCollectionBuilder,
            ) {
                let wit_entities: ::std::vec::Vec<WitOrderedEntityView> = wit.get_ordered_entities();
                let sdk_entities = wit_entities.into_iter().map(|e| ::slicer_sdk::OrderedEntityView {
                    original_index: e.original_index, tool_index: e.tool_index,
                    order_lock: e.order_lock,
                    region_key: ::slicer_ir::RegionKey {
                        global_layer_index: e.region_key.layer_index as u32,
                        object_id: e.region_key.object_id,
                        region_id: e.region_key.region_id.parse().unwrap_or(0),
                        variant_chain: Vec::new(),
                    },
                    role: __slicer_wit_role_to_ir(&e.role),
                    start_point: __slicer_wit_point3w_to_ir(&e.start_point),
                    end_point: __slicer_wit_point3w_to_ir(&e.end_point),
                    point_count: e.point_count,
                }).collect();
                sdk.set_ordered_entities(sdk_entities);
            }
        },
        _ => quote! {},
    }
}

fn build_layer_slice_postprocess_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/layer-slice-postprocess/layer-slice-postprocess.wit"
    );
    let preamble = emit_world_preamble("slice-postprocess-module", "slice_postprocess", wit_inline);
    let profile_install = profile_install_stmt();
    let aliases = layer_per_stage_aliases("layer_slice_postprocess");
    let helpers = layer_light_helpers();
    let stage_helpers = layer_stage_helpers("layer_slice_postprocess");
    let arm = quote! {
        let layer_index = layer_index as u32;
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&ir_config) {
            Ok(m) => m, Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_regions = __slicer_adapt_slice_regions(&regions);
        let keys: ::std::vec::Vec<(::std::string::String, ::slicer_ir::RegionId)> = sdk_regions
            .iter()
            .map(|r| (r.object_id().clone(), *r.region_id()))
            .collect();
        let sdk_paint = __slicer_adapt_paint_layer(&paint, &keys);
        let mut sdk_output = ::slicer_sdk::builders::SlicePostprocessBuilder::new();
        let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_slice_postprocess(
            &module, layer_index, &sdk_regions, &sdk_paint, &mut sdk_output, &ir_config,
        );
        __slicer_drain_slice_postprocess(&sdk_output, &output);
        match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_layer_slice_postprocess_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            #preamble
            #aliases
            #helpers #stage_helpers
            struct __SlicerLayerSlicePostprocessComponent;
            impl exports::slicer::layer_slice_postprocess::slice_postprocess::Guest for __SlicerLayerSlicePostprocessComponent {
                fn run(layer_index: i32, regions: Vec<SliceRegionView>, paint: PaintRegionLayerView, output: SlicePostprocessBuilder, config: ConfigView) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerLayerSlicePostprocessComponent);
        }
    }
}

fn build_layer_perimeters_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline =
        include_str!("../../slicer-schema/wit/deps/layer-perimeters/layer-perimeters.wit");
    let preamble = emit_world_preamble("perimeters-module", "perimeters", wit_inline);
    let profile_install = profile_install_stmt();
    let aliases = layer_per_stage_aliases("layer_perimeters");
    let helpers = layer_light_helpers();
    let stage_helpers = layer_stage_helpers("layer_perimeters");
    let arm = quote! {
        let layer_index = layer_index as u32;
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&ir_config) {
            Ok(m) => m, Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_regions = __slicer_adapt_slice_regions(&regions);
        let keys: ::std::vec::Vec<(::std::string::String, ::slicer_ir::RegionId)> = sdk_regions
            .iter()
            .map(|r| (r.object_id().clone(), *r.region_id()))
            .collect();
        let sdk_paint = __slicer_adapt_paint_layer(&paint, &keys);
        let mut sdk_output = ::slicer_sdk::builders::PerimeterOutputBuilder::new();
        let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_perimeters(
            &module, layer_index, &sdk_regions, &sdk_paint, &mut sdk_output, &ir_config,
        );
        __slicer_drain_perimeter(&sdk_output, &output);
        match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_layer_perimeters_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            #preamble #aliases #helpers #stage_helpers
            struct __SlicerLayerPerimetersComponent;
            impl exports::slicer::layer_perimeters::perimeters::Guest for __SlicerLayerPerimetersComponent {
                fn run(layer_index: i32, regions: Vec<SliceRegionView>, paint: PaintRegionLayerView, output: PerimeterOutputBuilder, config: ConfigView) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerLayerPerimetersComponent);
        }
    }
}

fn build_layer_perimeters_postprocess_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!("../../slicer-schema/wit/deps/layer-perimeters-postprocess/layer-perimeters-postprocess.wit");
    let preamble = emit_world_preamble(
        "perimeters-postprocess-module",
        "perimeters_postprocess",
        wit_inline,
    );
    let profile_install = profile_install_stmt();
    let aliases = layer_per_stage_aliases("layer_perimeters_postprocess");
    let helpers = layer_glue_helpers();
    let stage_helpers = layer_stage_helpers("layer_perimeters_postprocess");
    let arm = quote! {
        let layer_index = layer_index as u32;
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&ir_config) {
            Ok(m) => m, Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_regions = __slicer_adapt_perimeter_regions(&regions);
        let mut sdk_output = ::slicer_sdk::builders::PerimeterOutputBuilder::new();
        let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_wall_postprocess(
            &module, layer_index, &sdk_regions, &mut sdk_output, &ir_config,
        );
        __slicer_drain_perimeter(&sdk_output, &output);
        match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_layer_perimeters_postprocess_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            #preamble #aliases #helpers #stage_helpers
            struct __SlicerLayerPerimetersPostprocessComponent;
            impl exports::slicer::layer_perimeters_postprocess::perimeters_postprocess::Guest for __SlicerLayerPerimetersPostprocessComponent {
                fn run(layer_index: i32, regions: Vec<PerimeterRegionView>, output: PerimeterOutputBuilder, config: ConfigView) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerLayerPerimetersPostprocessComponent);
        }
    }
}

fn build_layer_infill_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!("../../slicer-schema/wit/deps/layer-infill/layer-infill.wit");
    let preamble = emit_world_preamble("infill-module", "infill", wit_inline);
    let profile_install = profile_install_stmt();
    let aliases = layer_per_stage_aliases("layer_infill");
    let helpers = layer_light_helpers();
    let stage_helpers = layer_stage_helpers("layer_infill");
    let arm = quote! {
        let layer_index = layer_index as u32;
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&ir_config) {
            Ok(m) => m, Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_regions = __slicer_adapt_slice_regions(&regions);
        let keys: ::std::vec::Vec<(::std::string::String, ::slicer_ir::RegionId)> = sdk_regions
            .iter()
            .map(|r| (r.object_id().clone(), *r.region_id()))
            .collect();
        let sdk_paint = __slicer_adapt_paint_layer(&paint, &keys);
        let mut sdk_output = ::slicer_sdk::builders::InfillOutputBuilder::new();
        let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_infill(
            &module, layer_index, &sdk_regions, &sdk_paint, &mut sdk_output, &ir_config,
        );
        __slicer_drain_infill(&sdk_output, &output);
        match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_layer_infill_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            #preamble #aliases #helpers #stage_helpers
            struct __SlicerLayerInfillComponent;
            impl exports::slicer::layer_infill::infill::Guest for __SlicerLayerInfillComponent {
                fn run(layer_index: i32, regions: Vec<SliceRegionView>, paint: PaintRegionLayerView, output: InfillOutputBuilder, config: ConfigView) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerLayerInfillComponent);
        }
    }
}

fn build_layer_infill_postprocess_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/layer-infill-postprocess/layer-infill-postprocess.wit"
    );
    let preamble = emit_world_preamble(
        "infill-postprocess-module",
        "infill_postprocess",
        wit_inline,
    );
    let profile_install = profile_install_stmt();
    let aliases = layer_per_stage_aliases("layer_infill_postprocess");
    let helpers = layer_glue_helpers();
    let stage_helpers = layer_stage_helpers("layer_infill_postprocess");
    let arm = quote! {
        let layer_index = layer_index as u32;
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&ir_config) {
            Ok(m) => m, Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_regions = __slicer_adapt_perimeter_regions(&regions);
        let sdk_prior_infill: ::std::vec::Vec<::slicer_ir::InfillRegion> = prior_infill.iter().map(|r| ::slicer_ir::InfillRegion {
            object_id: r.object_id.clone(), region_id: r.region_id.parse().unwrap_or(0),
            sparse_infill: r.sparse_infill.iter().map(__slicer_wit_path_to_ir).collect(),
            solid_infill: r.solid_infill.iter().map(__slicer_wit_path_to_ir).collect(),
            ironing: r.ironing.iter().map(__slicer_wit_path_to_ir).collect(),
            internal_bridge_infill: ::std::vec::Vec::new(),
        }).collect();
        let mut sdk_output = ::slicer_sdk::builders::InfillOutputBuilder::new();
        let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_infill_postprocess(
            &module, layer_index, &sdk_regions, &sdk_prior_infill, &mut sdk_output, &ir_config,
        );
        __slicer_drain_infill(&sdk_output, &output);
        match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_layer_infill_postprocess_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            #preamble #aliases #helpers #stage_helpers
            struct __SlicerLayerInfillPostprocessComponent;
            impl exports::slicer::layer_infill_postprocess::infill_postprocess::Guest for __SlicerLayerInfillPostprocessComponent {
                fn run(layer_index: i32, regions: Vec<PerimeterRegionView>, prior_infill: Vec<PriorInfillRegion>, output: InfillOutputBuilder, config: ConfigView) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerLayerInfillPostprocessComponent);
        }
    }
}

fn build_layer_support_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!("../../slicer-schema/wit/deps/layer-support/layer-support.wit");
    let preamble = emit_world_preamble("support-module", "support", wit_inline);
    let profile_install = profile_install_stmt();
    let aliases = layer_per_stage_aliases("layer_support");
    let helpers = layer_light_helpers();
    let stage_helpers = layer_stage_helpers("layer_support");
    let arm = quote! {
        let layer_index = layer_index as u32;
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&ir_config) {
            Ok(m) => m, Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_regions = __slicer_adapt_slice_regions(&regions);
        let keys: ::std::vec::Vec<(::std::string::String, ::slicer_ir::RegionId)> = sdk_regions
            .iter()
            .map(|r| (r.object_id().clone(), *r.region_id()))
            .collect();
        let sdk_paint = __slicer_adapt_paint_layer(&paint, &keys);
        let sdk_paint = sdk_paint.with_slice_ir(::std::sync::Arc::new(::slicer_ir::SliceIR {
            schema_version: ::slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: layer_index,
            z: sdk_regions.first().map(|r| r.z()).unwrap_or(0.0),
            regions: sdk_regions.iter().map(|r| ::slicer_ir::SlicedRegion {
                object_id: r.object_id().clone(), region_id: *r.region_id(),
                polygons: r.polygons().to_vec(), segment_annotations: r.segment_annotations().clone(),
                ..::core::default::Default::default()
            }).collect(),
        }));
        let mut sdk_output = ::slicer_sdk::builders::SupportOutputBuilder::new();
        let mut sdk_collection = ::slicer_sdk::LayerCollectionBuilder::new();
        let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_support(
            &module, layer_index, &sdk_regions, &sdk_paint, &mut sdk_output, &mut sdk_collection, &ir_config,
        );
        match out {
            Ok(()) => {
                __slicer_drain_support(&sdk_output, &output);
                if let Err(e) = __slicer_drain_anchored_collection(&sdk_collection, &collection) {
                    return Err(__slicer_error_out(::slicer_sdk::error::ModuleError::fatal(1, e)));
                }
                Ok(())
            }
            Err(e) => Err(__slicer_error_out(e)),
        }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_layer_support_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            #preamble #aliases #helpers #stage_helpers
            use exports::slicer::layer_support::support::LayerCollectionBuilder;
            struct __SlicerLayerSupportComponent;
            impl exports::slicer::layer_support::support::Guest for __SlicerLayerSupportComponent {
                fn run(layer_index: i32, regions: Vec<SliceRegionView>, paint: PaintRegionLayerView, output: SupportOutputBuilder, collection: LayerCollectionBuilder, config: ConfigView) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerLayerSupportComponent);
        }
    }
}

fn anchored_events_wit_preamble() -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/layer-anchored-events/layer-anchored-events.wit"
    );
    emit_world_preamble("anchored-events-module", "anchored_events", wit_inline)
}

fn build_layer_anchored_events_glue(self_ty: &syn::Type) -> TokenStream2 {
    let preamble = anchored_events_wit_preamble();
    let profile_install = profile_install_stmt();
    let aliases = layer_per_stage_aliases("layer_anchored_events");
    let helpers = layer_light_helpers();
    let stage_helpers = layer_stage_helpers("layer_anchored_events");
    let arm = quote! {
        let layer_index = layer_index as u32;
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&ir_config) {
            Ok(m) => m, Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_regions = __slicer_adapt_slice_regions(&regions);
        let mut sdk_collection = ::slicer_sdk::LayerCollectionBuilder::new();
        let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_anchored_events(
            &module, layer_index, &sdk_regions, &mut sdk_collection, &ir_config,
        );
        match out {
            Ok(()) => {
                if let Err(e) = __slicer_drain_anchored_collection(&sdk_collection, &collection) {
                    return Err(__slicer_error_out(::slicer_sdk::error::ModuleError::fatal(1, e)));
                }
                Ok(())
            }
            Err(e) => Err(__slicer_error_out(e)),
        }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_layer_anchored_events_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            #preamble #aliases #helpers #stage_helpers
            use exports::slicer::layer_anchored_events::anchored_events::LayerCollectionBuilder;
            struct __SlicerLayerAnchoredEventsComponent;
            impl exports::slicer::layer_anchored_events::anchored_events::Guest for __SlicerLayerAnchoredEventsComponent {
                fn run(layer_index: i32, regions: Vec<SliceRegionView>, collection: LayerCollectionBuilder, config: ConfigView) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerLayerAnchoredEventsComponent);
        }
    }
}

fn build_layer_support_postprocess_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/layer-support-postprocess/layer-support-postprocess.wit"
    );
    let preamble = emit_world_preamble(
        "support-postprocess-module",
        "support_postprocess",
        wit_inline,
    );
    let profile_install = profile_install_stmt();
    let aliases = layer_per_stage_aliases("layer_support_postprocess");
    let helpers = layer_light_helpers();
    let stage_helpers = layer_stage_helpers("layer_support_postprocess");
    let arm = quote! {
        let layer_index = layer_index as u32;
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&ir_config) {
            Ok(m) => m, Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_regions = __slicer_adapt_slice_regions(&regions);
        let mut sdk_output = ::slicer_sdk::builders::SupportOutputBuilder::new();
        let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_support_postprocess(
            &module, layer_index, &sdk_regions, &mut sdk_output, &ir_config,
        );
        __slicer_drain_support(&sdk_output, &output);
        match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_layer_support_postprocess_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            #preamble #aliases #helpers #stage_helpers
            struct __SlicerLayerSupportPostprocessComponent;
            impl exports::slicer::layer_support_postprocess::support_postprocess::Guest for __SlicerLayerSupportPostprocessComponent {
                fn run(layer_index: i32, regions: Vec<SliceRegionView>, output: SupportOutputBuilder, config: ConfigView) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerLayerSupportPostprocessComponent);
        }
    }
}

fn build_layer_path_optimization_glue(self_ty: &syn::Type) -> TokenStream2 {
    let wit_inline = include_str!(
        "../../slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit"
    );
    let preamble = emit_world_preamble("path-optimization-module", "path_optimization", wit_inline);
    let profile_install = profile_install_stmt();
    let aliases = layer_per_stage_aliases("layer_path_optimization");
    let helpers = layer_glue_helpers();
    let stage_helpers = layer_stage_helpers("layer_path_optimization");
    let arm = quote! {
        let layer_index = layer_index as u32;
        let ir_config = __slicer_adapt_config(&config);
        let module = match <#self_ty as ::slicer_sdk::traits::LayerModule>::from_config(&ir_config) {
            Ok(m) => m, Err(e) => return Err(__slicer_error_out(e)),
        };
        let sdk_regions = __slicer_adapt_perimeter_regions(&regions);
        let mut sdk_output = ::slicer_sdk::postpass_builders::GcodeOutputBuilder::new();
        let mut sdk_collection = ::slicer_sdk::LayerCollectionBuilder::new();
        __slicer_populate_layer_collection(&collection, &mut sdk_collection);
        let out = <#self_ty as ::slicer_sdk::traits::LayerModule>::run_path_optimization(
            &module, layer_index, &sdk_regions, &mut sdk_output, &mut sdk_collection, &ir_config,
        );
        __slicer_drain_gcode(&sdk_output, &output);
        __slicer_drain_layer_collection(&sdk_collection, &collection);
        match out { Ok(()) => Ok(()), Err(e) => Err(__slicer_error_out(e)) }
    };
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __slicer_layer_path_optimization_world_export {
            use super::#self_ty;
            use slicer::common::module_errors::ModuleError;
            use slicer::config::config_types::ConfigView;
            #preamble #aliases #helpers #stage_helpers
            struct __SlicerLayerPathOptimizationComponent;
            impl exports::slicer::layer_path_optimization::path_optimization::Guest for __SlicerLayerPathOptimizationComponent {
                fn run(layer_index: i32, regions: Vec<PerimeterRegionView>, output: GcodeOutputBuilder, collection: LayerCollectionBuilder, config: ConfigView) -> Result<(), ModuleError> {
                    #profile_install
                    #arm
                }
            }
            export!(__SlicerLayerPathOptimizationComponent);
        }
    }
}

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
                        ::slicer_sdk::test_support::mock_host_teardown();
                    }
                }

                ::slicer_sdk::test_support::reset_global_state();
                ::slicer_sdk::test_support::install_panic_handler();
                ::slicer_sdk::test_support::mock_host_setup();

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
                        ::slicer_sdk::test_support::mock_host_teardown();
                    }
                }

                ::slicer_sdk::test_support::reset_global_state();
                ::slicer_sdk::test_support::install_panic_handler();
                ::slicer_sdk::test_support::mock_host_setup();

                let _guard = __SlicerTestGuard;

                #fn_block
            }
        }
    }
}
