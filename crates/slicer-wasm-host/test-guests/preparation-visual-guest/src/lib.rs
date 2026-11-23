//! Test-only same-source native/component projection witness, not a community module.
//!
//! The private `probe` codec is exactly four ASCII alphanumeric bytes. The
//! default token is `PRB1`; ordinary dispatch must decode the required piece
//! before emitting `PREPARATION-VISUAL-PROBE` and `PREPARATION-VISUAL-TOKEN:PRB1`.
//! These bytes are never an input to diagnostic geometry or host rendering.

use slicer_ir::{ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::error::ModuleError;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::preparation::{
    LayerPreparation, PreparationInputError, PreparationLayer, PreparationProjectionError,
    PreparationProjectionGeometry as Geometry, PreparationProjectionOutput,
    PreparationProjectionPrimitive as Primitive, PreparationProjectionScene as Scene,
    PreparedPlanOutput, PreparedPlanView, PrintPreparationInput,
};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

pub struct PreparationVisualGuest;

fn input_error(error: PreparationInputError) -> ModuleError {
    ModuleError::fatal(57502, format!("visual fixture input: {error:?}"))
}

fn projection_error(error: PreparationProjectionError) -> ModuleError {
    ModuleError::fatal(57504, format!("visual fixture projection: {error:?}"))
}

fn decode_probe(bytes: &[u8]) -> Result<&str, ModuleError> {
    if bytes.len() != 4 || !bytes.iter().all(u8::is_ascii_alphanumeric) {
        return Err(ModuleError::fatal(57503, "malformed visual probe"));
    }
    std::str::from_utf8(bytes).map_err(|_| ModuleError::fatal(57503, "visual probe must be ASCII"))
}

fn encode_probe(token: &str) -> Result<[u8; 4], ModuleError> {
    let bytes = decode_probe(token.as_bytes())?.as_bytes();
    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

// All coordinates below are literal integer units (100 nm), not millimeters.
fn points(vertices: &[(i64, i64)]) -> Vec<Point2> {
    vertices.iter().map(|&(x, y)| Point2 { x, y }).collect()
}

fn polygon(contour: &[(i64, i64)], holes: &[&[(i64, i64)]]) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: points(contour),
        },
        holes: holes
            .iter()
            .map(|vertices| Polygon {
                points: points(vertices),
            })
            .collect(),
    }
}

fn primitive(id: &str, layer: u32, class: &str, label: &str, geometry: Geometry) -> Primitive {
    Primitive {
        // exhaustive: fixture pins every diagnostic identity and geometry field
        id: id.into(),
        global_layer_index: layer,
        class: class.into(),
        label: Some(label.into()),
        geometry,
    }
}

fn valid_scene(layers: &[PreparationLayer]) -> Scene {
    let layer = layers[0].index;
    let mut scene = Scene {
        scene_id: "final".into(),
        view: "probe_xy".into(),
        classes: vec!["anchors".into(), "domains".into(), "paths".into()],
        primitives: vec![
            primitive(
                "anchor_a",
                layer,
                "anchors",
                "anchor",
                Geometry::Point(Point2 {
                    x: 10_000,
                    y: 20_000,
                }),
            ),
            primitive(
                "path_a",
                layer,
                "paths",
                "route",
                Geometry::Polyline(points(&[(0, 0), (20_000, 0)])),
            ),
            primitive(
                "domain_a",
                layer,
                "domains",
                "area",
                Geometry::Polygon {
                    polygon: polygon(
                        &[(0, 0), (40_000, 0), (40_000, 40_000), (0, 40_000)],
                        &[&[
                            (10_000, 10_000),
                            (10_000, 30_000),
                            (30_000, 30_000),
                            (30_000, 10_000),
                        ]],
                    ),
                    filled: true,
                },
            ),
            primitive(
                "outline_a",
                layer,
                "domains",
                "outline",
                Geometry::Polygon {
                    polygon: polygon(
                        &[(60_000, 0), (80_000, 0), (80_000, 20_000), (60_000, 20_000)],
                        &[],
                    ),
                    filled: false,
                },
            ),
        ],
    };
    // Use an actual second scheduled layer, including when it is not rendered.
    // Its remote geometry/class controls whole-scene framing and palette ranks.
    if let Some(remote_layer) = layers.get(1) {
        scene.classes.push("remote".into());
        scene.primitives.push(primitive(
            "remote_a",
            remote_layer.index,
            "remote",
            "remote",
            Geometry::Point(Point2 {
                x: 2_000_000,
                y: 1_500_000,
            }),
        ));
    }
    scene
}

fn malformed_scene(mode: &str, scene: &mut Scene, layers: &[PreparationLayer]) {
    match mode {
        "undeclared_view" => scene.view = "undeclared_xy".into(),
        "undeclared_class" => scene.primitives[0].class = "undeclared_class".into(),
        "unknown_layer" => {
            scene.primitives[0].global_layer_index = (0..=u32::MAX)
                .find(|index| layers.iter().all(|layer| layer.index != *index))
                .expect("fixture schedule cannot exhaust every u32 index");
        }
        "invalid_line" | "caught_error" | "module_error" | "trap" => {
            // Consecutive repeated vertices cannot form an open polyline.
            scene.primitives[1].geometry = Geometry::Polyline(points(&[(0, 0), (0, 0)]));
        }
        "invalid_ring" => {
            // Rings must not repeat their terminal vertex.
            scene.primitives[2].geometry = Geometry::Polygon {
                polygon: polygon(&[(0, 0), (40_000, 0), (0, 40_000), (0, 0)], &[]),
                filled: true,
            };
        }
        "invalid_hole" => {
            // The clockwise hole touches the otherwise valid CCW contour.
            scene.primitives[2].geometry = Geometry::Polygon {
                polygon: polygon(
                    &[(0, 0), (40_000, 0), (40_000, 40_000), (0, 40_000)],
                    &[&[(0, 10_000), (0, 30_000), (20_000, 30_000), (20_000, 10_000)]],
                ),
                filled: true,
            };
        }
        "invalid_coordinate" => {
            // Valid integer coordinates individually; checked ring predicates
            // overflow i128. No fabricated NaN or arbitrary coordinate limit.
            scene.primitives[2].geometry = Geometry::Polygon {
                polygon: polygon(
                    &[
                        (i64::MIN, i64::MIN),
                        (i64::MAX, i64::MIN),
                        (i64::MAX, i64::MAX),
                        (i64::MIN, i64::MAX),
                    ],
                    &[],
                ),
                filled: true,
            };
        }
        _ => unreachable!("not a malformed-scene mode"),
    }
}

fn expect_rejection(
    sink: &PreparationProjectionOutput,
    scene: &Scene,
    expected: PreparationProjectionError,
) {
    let actual = sink.publish(scene).expect_err("invalid scene was accepted");
    // Log the real typed resource result, never a preselected error string.
    slicer_sdk::host::log_info(&format!("PREPARATION-VISUAL-PUBLISH-ERROR:{actual:?}"));
    assert_eq!(actual, expected, "wrong typed projection rejection");
}

impl LayerPreparation for PreparationVisualGuest {
    fn prepare_print(
        input: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError> {
        slicer_sdk::host::log_info("PREPARATION-VISUAL-PREPARE");
        let layers = input.layers(0, u64::MAX).map_err(input_error)?;
        if layers.is_empty() {
            return Err(ModuleError::fatal(57502, "visual fixture needs a schedule"));
        }
        let mut selected_config = None;
        for layer in &layers {
            for target in input
                .selected_targets(layer.index, 0, u64::MAX)
                .map_err(input_error)?
            {
                if target.invoke && selected_config.is_none() {
                    selected_config = Some(target.config);
                }
            }
        }
        let config = selected_config
            .ok_or_else(|| ModuleError::fatal(57502, "visual fixture needs a selected target"))?;
        let mode = config
            .get_string("preparation_visual_mode")
            .unwrap_or("valid");
        match mode {
            "valid" | "empty" | "missing" | "duplicate" | "undeclared_view"
            | "undeclared_class" | "unknown_layer" | "invalid_line" | "invalid_ring"
            | "invalid_hole" | "invalid_coordinate" | "caught_error" | "module_error" | "trap" => {}
            _ => return Err(ModuleError::fatal(57502, "unknown preparation_visual_mode")),
        }
        let probe = encode_probe(
            config
                .get_string("preparation_visual_token")
                .unwrap_or("PRB1"),
        )?;
        output.put("probe", &probe)?;

        if mode == "missing" {
            // MissingScene must also be detected when projection() is not called.
            return Ok(());
        }
        if let Some(sink) = output.projection().map_err(projection_error)? {
            let requested = sink.requested_views().map_err(projection_error)?;
            if requested != ["probe_xy"] {
                return Err(ModuleError::fatal(
                    57504,
                    "unexpected requested visual views",
                ));
            }
            // No diagnostic collections/geometry are created on the None path.
            let mut scene = valid_scene(&layers);
            match mode {
                "valid" => sink.publish(&scene).map_err(projection_error)?,
                "empty" => {
                    scene.primitives.clear();
                    sink.publish(&scene).map_err(projection_error)?;
                }
                "duplicate" => {
                    sink.publish(&scene).map_err(projection_error)?;
                    expect_rejection(&sink, &scene, PreparationProjectionError::DuplicateScene);
                }
                _ => {
                    malformed_scene(mode, &mut scene, &layers);
                    let expected = match mode {
                        "undeclared_view" => PreparationProjectionError::UndeclaredView,
                        "undeclared_class" | "unknown_layer" => {
                            PreparationProjectionError::InvalidReference
                        }
                        "invalid_coordinate" => PreparationProjectionError::InvalidCoordinate,
                        _ => PreparationProjectionError::InvalidShape,
                    };
                    expect_rejection(&sink, &scene, expected);
                    if mode == "caught_error" {
                        // Catching the first error cannot make later writes valid.
                        expect_rejection(
                            &sink,
                            &valid_scene(&layers),
                            PreparationProjectionError::PoisonedOutput,
                        );
                    }
                }
            }
        }
        // Original module failures/traps must win over any caught sink poison;
        // both also fail when diagnostics were not requested.
        match mode {
            "module_error" => Err(ModuleError::non_fatal(57551, "visual fixture module error")),
            "trap" => panic!("visual fixture preparation trap"),
            _ => Ok(()),
        }
    }
}

#[slicer_module(preparation)]
impl LayerModule for PreparationVisualGuest {
    fn from_config(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_path_optimization(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        output: &mut GcodeOutputBuilder,
        _collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let piece = PreparedPlanView::current()?.require("probe")?;
        // Reading a fifth byte rejects oversized as well as short private pieces.
        let bytes = piece.read(0, 5)?;
        let token = decode_probe(&bytes)?;
        output
            .push_comment("PREPARATION-VISUAL-PROBE".into())
            .map_err(|message| ModuleError::fatal(57505, message))?;
        output
            .push_comment(format!("PREPARATION-VISUAL-TOKEN:{token}"))
            .map_err(|message| ModuleError::fatal(57505, message))
    }
}
