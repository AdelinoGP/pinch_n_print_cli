
# OrcaSlicer Configuration Reference
> Auto-generated from PrintConfig.cpp, Tab.cpp, PrintConfig.hpp
> Generation date: 2026-05-19
>
> **This is an upstream snapshot, not a registry of ModularSlicer's values.** It
> records OrcaSlicer's keys and defaults for porting reference. ModularSlicer's
> own config keys and defaults live in `docs/15_config_keys_reference.md`; where
> our default intentionally differs from upstream, the difference is listed in
> that file's generated **Deviations from OrcaSlicer** table (produced by
> `cargo xtask gen-config-docs`). Do not treat the `Default` column here as
> ModularSlicer's value.

---


## Quality

### Layer height
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "layer_height" | Layer height | coFloat | 0.2 | mm | comAdvanced | Slicing height per layer. Smaller = more accurate + more time. | ✅ |
| "initial_layer_print_height" | Initial layer height | coFloat | 0.2 | mm | comAdvanced | Height of initial layer. Thicker improves plate adhesion. | ✅  |
| "first_layer_print_sequence" | First layer print sequence | coInts | 0 | - | comSimple | Print sequence for first layer. | ❌ |
| "first_layer_sequence_choice" | First layer filament sequence | coEnum | Auto | Auto/Customize | comSimple | Auto or customized filament sequence for first layer. | ❌ |
| "other_layers_print_sequence" | Other layers print sequence | coInts | 0 | - | comAdvanced | Print sequence for layers after first. | ❌ |
| "other_layers_print_sequence_nums" | Print sequence repeat count | coInt | 0 | - | comAdvanced | Number of repeat cycles for other layers sequence. | ❌ |
| "other_layers_sequence_choice" | Other layers filament sequence | coEnum | Auto | Auto/Customize | comAdvanced | Auto or customized filament sequence for other layers. | ❌ |

### Line width
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "line_width" | Default | coFloatOrPercent | 0 (auto) | mm or % | comAdvanced | Default line width; computed over nozzle diameter if %. | ✅ |
| "initial_layer_line_width" | Initial layer | coFloatOrPercent | 0 (auto) | mm or % | comAdvanced | Line width of initial layer; computed over nozzle if %. | ❌ |
| "outer_wall_line_width" | Outer wall | coFloatOrPercent | 0 (auto) | mm or % | comAdvanced | Line width of outer wall; computed over nozzle if %. | ❌ |
| "inner_wall_line_width" | Inner wall | coFloatOrPercent | 0 (auto) | mm or % | comAdvanced | Line width of inner wall; computed over nozzle if %. | ❌ |
| "top_surface_line_width" | Top surface | coFloatOrPercent | 0 (auto) | mm or % | comAdvanced | Line width for top surfaces; computed over nozzle if %. | ❌ |
| "sparse_infill_line_width" | Sparse infill | coFloatOrPercent | 0 (auto) | mm or % | comAdvanced | Line width of sparse infill; computed over nozzle if %. | ❌ |
| "internal_solid_infill_line_width" | Internal solid infill | coFloatOrPercent | 0 (auto) | mm or % | comAdvanced | Line width of internal solid infill; computed over nozzle if %. | ❌ |
| "support_line_width" | Support | coFloatOrPercent | 0 (auto) | mm or % | comAdvanced | Line width of support; computed over nozzle diameter if %. | ❌ |

### Seam
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "seam_position" | Seam position | coEnum | aligned | nearest/aligned/aligned_back/back/random | comSimple | Start position to print each part of outer wall. | ❌ |
| "staggered_inner_seams" | Staggered inner seams | coBool | 0 | - | comAdvanced | Inner seams shifted backwards forming zigzag pattern. | ❌ |
| "seam_gap" | Seam gap | coFloatOrPercent | 10% | mm or % | comAdvanced | Loop shortened by this amount to hide seam visibility. | ❌ |
| "seam_slope_type" | Scarf joint seam (beta) | coEnum | none | none/external/all | comAdvanced | Use scarf joint to minimize seam visibility and increase strength. | ❌ |
| "seam_slope_conditional" | Conditional scarf joint | coBool | 0 | - | comAdvanced | Apply scarf joints only to smooth perimeters without sharp corners. | ❌ |
| "scarf_angle_threshold" | Conditional angle threshold | coInt | 155 | ° | comAdvanced | Threshold angle for applying conditional scarf joint seam. | ❌ |
| "scarf_overhang_threshold" | Conditional overhang threshold | coPercent | 40% | % | comAdvanced | Overhang threshold for scarf joint application. | ❌ |
| "scarf_joint_speed" | Scarf joint speed | coFloatOrPercent | 100% | mm/s or % | comAdvanced | Printing speed for scarf joints; computed over wall speed if %. | ❌ |
| "scarf_joint_flow_ratio" | Scarf joint flow ratio | coFloat | 1 | - | comDevelop | Material amount for scarf joints. | ❌ |
| "has_scarf_joint_seam" | (auto-set) Scarf joint seam flag | coBool | 0 | - | [hidden] | Auto-set flag; indicates scarf joint seam is active in model. | ❌ |
| "seam_slope_start_height" | Scarf start height | coFloatOrPercent | 0 | mm or % | comAdvanced | Start height of the scarf; relative to layer height if %. | ❌ |
| "seam_slope_entire_loop" | Scarf around entire wall | coBool | 0 | - | comAdvanced | Scarf extends to the entire length of the wall. | ❌ |
| "seam_slope_min_length" | Scarf length | coFloat | 20 | mm | comAdvanced | Length of the scarf; zero effectively disables. | ❌ |
| "seam_slope_steps" | Scarf steps | coInt | 10 | - | comAdvanced | Minimum number of segments of each scarf. | ❌ |
| "seam_slope_inner_walls" | Scarf joint for inner walls | coBool | 0 | - | comAdvanced | Use scarf joint for inner walls as well. | ❌ |
| "role_based_wipe_speed" | Role base wipe speed | coBool | 1 | - | comAdvanced | Wipe speed determined by speed of current extrusion role. | ❌ |
| "wipe_speed" | Wipe speed | coFloatOrPercent | 80% | mm/s or % | comAdvanced | Wipe speed; computed based on travel speed if %. | ❌ |
| "wipe_on_loops" | Wipe on loops | coBool | 0 | - | comAdvanced | Small inward movement before leaving loop to hide seam. | ❌ |
| "wipe_before_external_loop" | Wipe before external loop | coBool | 0 | - | comAdvanced | Deretraction performed inside from start of external perimeter. | ❌ |

### Precision
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "slice_closing_radius" | Slice gap closing radius | coFloat | 0.049 | mm | comAdvanced | Cracks smaller than 2x radius filled during mesh slicing. | ✅ |
| "resolution" | Resolution | coFloat | 0.01 | mm | comAdvanced | G-code path simplification tolerance; smaller = higher resolution. | ❌ |
| "enable_arc_fitting" | Arc fitting | coBool | 0 | - | comAdvanced | Enable G2/G3 arc moves in G-code; not recommended for Klipper. | ❌ |
| "xy_hole_compensation" | X-Y hole compensation | coFloat | 0 | mm | comAdvanced | Holes expand (+) or contract (-) in XY plane for assembly fit. | ❌ |
| "xy_contour_compensation" | X-Y contour compensation | coFloat | 0 | mm | comAdvanced | Contours expand (+) or contract (-) in XY plane for assembly fit. | ❌ |
| "elefant_foot_compensation" | Elephant foot compensation | coFloat | 0 | mm | comAdvanced | Shrink initial layer to compensate for elephant foot effect. | ❌ |
| "elefant_foot_compensation_layers" | E.F. compensation layers | coInt | 1 | layers | comAdvanced | Number of layers where compensation is active; linearly reduced. | ❌ |
| "precise_outer_wall" | Precise wall | coBool | 1 | - | comAdvanced | Improve shell precision by adjusting outer wall spacing. | ❌ |
| "precise_z_height" | Precise Z height | coBool | 0 | - | comAdvanced | Fine-tune last layers to get precise object Z height. | ❌ |
| "hole_to_polyhole" | Convert holes to polyholes | coBool | 0 | - | comAdvanced | Convert circular holes to polyholes for better dimensional fit. | ❌ |
| "hole_to_polyhole_threshold" | Polyhole detection margin | coFloatOrPercent | 0.01 | mm or % | comAdvanced | Max deviation of point to estimated circle radius for detection. | ❌ |
| "hole_to_polyhole_twisted" | Polyhole twist | coBool | 1 | - | comAdvanced | Rotate polyhole every layer. | ❌ |

### Ironing
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "ironing_type" | Ironing Type | coEnum | no ironing | no ironing/top/topmost/solid | comAdvanced | Which layers get ironed for smooth surface finish. | ❌ |
| "ironing_pattern" | Ironing Pattern | coEnum | rectilinear | rectilinear/concentric | comAdvanced | Pattern used when ironing. | ✅ |
| "ironing_flow" | Ironing flow | coPercent | 10% | % | comAdvanced | Material amount for ironing; relative to normal layer flow. | ✅ |
| "ironing_spacing" | Ironing line spacing | coFloat | 0.1 | mm | comAdvanced | Distance between ironing lines. | ✅ |
| "ironing_inset" | Ironing inset | coFloat | 0 | mm | comAdvanced | Distance to keep from edges when ironing; 0 = half nozzle diameter. | ❌ |
| "ironing_angle" | Ironing angle offset | coFloat | 0 | ° | comAdvanced | Angle offset of ironing lines from top surface. | ❌ |
| "ironing_angle_fixed" | Fixed ironing angle | coBool | 0 | - | comAdvanced | Use a fixed absolute angle for ironing. | ❌ |
| "ironing_speed" | Ironing speed | coFloat | 20 | mm/s | comAdvanced | Print speed of ironing lines. | ✅ |

### Wall generator — Shared
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "wall_generator" | Wall generator | coEnum | arachne | classic/arachne | comAdvanced | Classic = constant width; Arachne = variable extrusion width. | ✅ |

### Wall generator — Classic
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "detect_thin_wall" | Detect thin wall | coBool | 0 | - | Classic only | Detect thin walls that can't hold two widths; use single line. | ❌ |

### Wall generator — Arachne
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "wall_transition_length" | Wall transition length | coPercent | 100% | % | Arachne only | Space allotted for splitting/joining wall segments as part thins. | ❌ |
| "wall_transition_filter_deviation" | W. transitioning filter margin | coPercent | 25% | % | Arachne only | Prevents back-and-forth transitions between wall counts. | ❌ |
| "wall_transition_angle" | W. transitioning threshold angle | coFloat | 10 | ° | Arachne only | Angle threshold for creating transitions between wall counts. | ❌ |
| "wall_distribution_count" | Wall distribution count | coInt | 1 | - | Arachne only | Number of walls from center over which width variation spreads. | ❌ |
| "initial_layer_min_bead_width" | First layer minimum wall width | coPercent | 85% | % | Arachne only | Min wall width for first layer; recommended = nozzle size. | ❌ |
| "min_bead_width" | Minimum wall width | coPercent | 85% | % | Arachne only | Width replacing thin features; percentage of nozzle diameter. | ❌ |
| "min_feature_size" | Minimum feature size | coPercent | 25% | % | Arachne only | Thin features thinner than this are not printed. | ❌ |
| "min_length_factor" | Minimum wall length | coFloat | 0.5 | mm | Arachne only | Prevents short unclosed walls from being printed to save time. | ❌ |

### Walls and surfaces
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "wall_sequence" | Walls printing order | coEnum | Inner/Outer | Inner/Outer/Outer/Inner/Inner-Outer-Inner | comAdvanced | Print sequence of inner and outer walls for quality/overhangs. | ❌ |
| "is_infill_first" | Print infill first | coBool | 0 | - | comAdvanced | Print infill before walls; helps extreme overhangs but worse finish. | ❌ |
| "wall_direction" | Wall loop direction | coEnum | auto | auto/ccw/cw | comAdvanced | Direction of wall loops when viewed from top. | ❌ |
| "print_flow_ratio" | Flow ratio | coFloat | 1 | - | comAdvanced | Overall object flow multiplier x filament flow ratio. | ❌ |
| "top_solid_infill_flow_ratio" | Top surface flow ratio | coFloat | 1 | - | comAdvanced | Flow factor for top solid infill; decrease for smooth finish. | ❌ |
| "bottom_solid_infill_flow_ratio" | Bottom surface flow ratio | coFloat | 1 | - | comAdvanced | Flow factor for bottom solid infill. | ❌ |
| "set_other_flow_ratios" | Set other flow ratios | coBool | 0 | - | comAdvanced | Enable individual flow ratios for each extrusion path type. | ❌ |
| "first_layer_flow_ratio" | First layer flow ratio | coFloat | 1 | - | comAdvanced | Flow multiplier for all first layer extrusions. | ❌ |
| "outer_wall_flow_ratio" | Outer wall flow ratio | coFloat | 1 | - | comAdvanced | Flow factor for outer walls. | ❌ |
| "inner_wall_flow_ratio" | Inner wall flow ratio | coFloat | 1 | - | comAdvanced | Flow factor for inner walls. | ❌ |
| "overhang_flow_ratio" | Overhang flow ratio | coFloat | 1 | - | comAdvanced | Flow factor for overhangs. | ❌ |
| "sparse_infill_flow_ratio" | Sparse infill flow ratio | coFloat | 1 | - | comAdvanced | Flow factor for sparse infill. | ❌ |
| "internal_solid_infill_flow_ratio" | Internal solid infill flow ratio | coFloat | 1 | - | comAdvanced | Flow factor for internal solid infill. | ❌ |
| "gap_fill_flow_ratio" | Gap fill flow ratio | coFloat | 1 | - | comAdvanced | Flow factor for gap filling. | ❌ |
| "support_flow_ratio" | Support flow ratio | coFloat | 1 | - | comAdvanced | Flow factor for support material. | ❌ |
| "support_interface_flow_ratio" | Support interface flow ratio | coFloat | 1 | - | comAdvanced | Flow factor for support interface. | ❌ |
| "only_one_wall_top" | Only one wall on top surfaces | coBool | 0 | - | comAdvanced | Use only one wall on flat top surfaces for more infill space. | ❌ |
| "min_width_top_surface" | One wall threshold | coFloatOrPercent | 300% | mm or % | comAdvanced | Min width to consider surface as top layer for one-wall logic. | ❌ |
| "only_one_wall_first_layer" | Only one wall on first layer | coBool | 0 | - | comAdvanced | Use only one wall on first layer for more infill space. | ❌ |
| "reduce_crossing_wall" | Avoid crossing walls | coBool | 1 | - | comAdvanced | Detour to avoid traveling across walls to prevent blobs. | ❌ |
| "max_travel_detour_distance" | A.C.W. - Max detour length | coFloatOrPercent | 0 | mm or % | comAdvanced | Max detour distance; 0 = disabled. | ❌ |
| "small_area_infill_flow_compensation" | Small area flow comp. (beta) | coBool | 0 | - | comAdvanced | Enable flow compensation for small infill areas. | ❌ |
| "small_area_infill_flow_compensation_model" | Flow Compensation Model | coStrings | multi-line pairs | - | comAdvanced | Model of extrusion length/flow correction factor pairs. | ❌ |
| "extruder" | Extruder | coInt | 0 (inherit) | extruder index | comAdvanced | Extruder to use unless more specific settings override. | ✅ |

### Bridging
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "bridge_flow" | Bridge flow ratio | coFloat | 1 | - | comAdvanced | Flow for bridges; decrease slightly (e.g. 0.9) to improve sag. | ❌ |
| "internal_bridge_flow" | Internal bridge flow ratio | coFloat | 1 | - | comAdvanced | Flow for internal bridges (first layer over sparse infill). | ❌ |
| "bridge_density" | External bridge density | coPercent | 100% | % | comAdvanced | Density (spacing) of external bridge lines; 10-120%. | ❌ |
| "internal_bridge_density" | Internal bridge density | coPercent | 100% | % | comAdvanced | Density of internal bridge lines; 10-100%. | ❌ |
| "thick_bridges" | Thick external bridges | coBool | 0 | - | comAdvanced | If enabled, bridges more reliable but may look worse. | ❌ |
| "thick_internal_bridges" | Thick internal bridges | coBool | 1 | - | comAdvanced | If enabled, thick internal bridges are used. | ❌ |
| "enable_extra_bridge_layer" | Extra bridge layers (beta) | coEnum | disabled | disabled/external_only/internal_only/apply_to_all | comAdvanced | Generate extra bridge layer over internal and/or external bridges. | ❌ |
| "dont_filter_internal_bridges" | Filter out small internal bridges | coEnum | Filter | Filter/Limited filtering/No filtering | comAdvanced | Controls sensitivity of small internal bridge filtering. | ❌ |
| "counterbore_hole_bridging" | Bridge counterbore holes | coEnum | none | none/partiallybridge/sacrificiallayer | comAdvanced | Creates bridges for counterbore holes to avoid support. | ❌ |
| "bridge_angle" | External bridge infill direction | coFloat | 0 | ° | comAdvanced | Bridging angle override; 0 = automatic calculation. | ❌ |
| "internal_bridge_angle" | Internal bridge infill direction | coFloat | 0 | ° | comAdvanced | Internal bridging angle override; 0 = automatic. | ❌ |

### Overhangs
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "detect_overhang_wall" | Detect overhang wall | coBool | 1 | - | comAdvanced | Detect overhang % relative to line width; use different speed. | ❌ |
| "make_overhang_printable" | Make overhangs printable | coBool | 0 | - | comAdvanced | Modify geometry to print overhangs without support material. | ❌ |
| "make_overhang_printable_angle" | M.O.P. - Maximum angle | coFloat | 55 | ° | comAdvanced | Max overhang angle to allow after making printable. | ❌ |
| "make_overhang_printable_hole_size" | M.O.P. - Hole area | coFloat | 0 | mm² | comAdvanced | Max hole area filled by conical material; 0 fills all holes. | ❌ |
| "extra_perimeters_on_overhangs" | Extra perimeters on overhangs | coBool | 0 | - | comAdvanced | Additional perimeter paths over steep overhangs/unanchored bridges. | ❌ |
| "overhang_reverse" | Reverse on even | coBool | 0 | - | comAdvanced | Alternate perimeter direction on even layers over overhangs. | ❌ |
| "overhang_reverse_internal_only" | Reverse only internal perimeters | coBool | 0 | - | comAdvanced | Apply reverse logic only to internal perimeters. | ❌ |
| "overhang_reverse_threshold" | Reverse threshold | coFloatOrPercent | 50% | mm or % | comAdvanced | Min overhang length for reversal; 0 = all even layers. | ❌ |
---


## Strength

### Walls
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "wall_loops" | Wall loops | coInt | 2 | - | comSimple | Number of walls per layer; more walls = stronger part. | ✅ |
| "alternate_extra_wall" | Alternate extra wall | coBool | 0 | - | comAdvanced | Extra wall every other layer wedges infill vertically for strength. | ❌ |

### Top/bottom shells
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "top_shell_layers" | Top shell layers | coInt | 4 | layers | comSimple | Number of solid layers on top shell including surface. | ✅ |
| "top_shell_thickness" | Top shell thickness | coFloat | 0.6 | mm | comAdvanced | Min thickness for top shell; overrides layer count when needed. | ❌ |
| "top_surface_density" | Top surface density | coPercent | 100% | % | comAdvanced | Density of top surface layer; 100% = fully solid. | ❌ |
| "top_surface_pattern" | Top surface pattern | coEnum | monotonicline | monotonic/monotonicline/rectilinear/... | comSimple | Line pattern of top surface infill. | ❌ |
| "bottom_shell_layers" | Bottom shell layers | coInt | 3 | layers | comSimple | Number of solid layers on bottom shell including surface. | ✅ |
| "bottom_shell_thickness" | Bottom shell thickness | coFloat | 0 | mm | comAdvanced | Min thickness for bottom shell; 0 = disabled. | ❌ |
| "bottom_surface_density" | Bottom surface density | coPercent | 100% | % | comAdvanced | Density of bottom surface layer. | ❌ |
| "bottom_surface_pattern" | Bottom surface pattern | coEnum | monotonic | monotonic/monotonicline/rectilinear/... | comSimple | Line pattern of bottom surface infill (not bridge). | ❌ |
| "top_bottom_infill_wall_overlap" | Top/Bottom solid infill/wall overlap | coPercent | 25% | % | comAdvanced | Overlap between top/bottom solid infill and walls for bonding. | ❌ |

### Infill
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "sparse_infill_density" | Sparse infill density | coPercent | 20% | % | comSimple | Density of internal sparse infill; 100% = solid. | ✅ |
| "fill_multiline" | Fill Multiline | coInt | 1 | - | comSimple | Number of lines for infill pattern if pattern supports it. | ❌ |
| "sparse_infill_pattern" | Sparse infill pattern | coEnum | Cross Hatch | rectilinear/zigzag/grid/triangles/cubic/gyroid/... | comSimple | Line pattern for internal sparse infill. | ❌ |
| "infill_direction" | Sparse infill direction | coFloat | 45 | ° | comAdvanced | Angle controlling the start/main direction of sparse infill. | ❌ |
| "sparse_infill_rotate_template" | Sparser infill rotation template | coString | "" | ° | comAdvanced | Comma-separated per-layer rotation angles for sparse infill. | ❌ |
| "infill_anchor_max" | Max length of infill anchor | coFloatOrPercent | 20 | mm or % | comAdvanced | Max length of perimeter segment connecting two infill lines. | ❌ |
| "infill_anchor" | Sparse infill anchor length | coFloatOrPercent | 400% | mm or % | comAdvanced | Length to connect infill line to perimeter at one side. | ❌ |
| "internal_solid_infill_pattern" | Internal solid infill pattern | coEnum | monotonic | same as top_surface_pattern | comAdvanced | Line pattern of internal solid infill. | ❌ |
| "solid_infill_direction" | Solid infill direction | coFloat | 45 | ° | comAdvanced | Angle for solid infill pattern direction. | ❌ |
| "solid_infill_rotate_template" | Solid infill rotation template | coString | "" | ° | comAdvanced | Comma-separated per-layer rotation template for solid infill. | ❌ |
| "gap_fill_target" | Apply gap fill | coEnum | Nowhere | everywhere/topbottom/nowhere | comAdvanced | Controls which solid surfaces receive gap fill extrusions. | ❌ |
| "filter_out_gap_fill" | Filter out tiny gaps | coFloat | 0 | mm | comAdvanced | Don't print gap fill shorter than this threshold. | ❌ |
| "infill_wall_overlap" | Infill/Wall overlap | coPercent | 15% | % | comAdvanced | Infill enlarged to overlap wall for better bonding. | ❌ |
| "top_bottom_infill_wall_overlap" | Top/Bottom solid infill/wall overlap | coPercent | 25% | % | comAdvanced | Overlap between top/bottom solid infill and walls. | ❌ |

### Infill pattern-specific
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "skin_infill_density" | Skin infill density | coPercent | 25% | % | comAdvanced | Density of outer surface region within a certain depth range. | ❌ |
| "skeleton_infill_density" | Skeleton infill density | coPercent | 25% | % | comAdvanced | Density of remaining contour after removing surface depth. | ❌ |
| "infill_lock_depth" | Infill lock depth | coFloat | 1.0 | mm | comAdvanced | Overlapping depth between interior and skin regions. | ❌ |
| "skin_infill_depth" | Skin infill depth | coFloat | 2.0 | mm | comAdvanced | Depth of the skin region measured from outer surface. | ❌ |
| "skin_infill_line_width" | Skin line width | coFloatOrPercent | 100% | mm | comAdvanced | Line width adjustment for selected skin paths. | ❌ |
| "skeleton_infill_line_width" | Skeleton line width | coFloatOrPercent | 100% | mm | comAdvanced | Line width adjustment for selected skeleton paths. | ❌ |
| "symmetric_infill_y_axis" | Symmetric infill Y axis | coBool | 0 | - | comAdvanced | Produce symmetric infill texture about Y axis for mirrored parts. | ❌ |
| "infill_shift_step" | Infill shift step | coFloat | 0.4 | mm | comAdvanced | Slight layer displacement to create cross-textured infill pattern. | ❌ |
| "lateral_lattice_angle_1" | Lateral lattice angle 1 | coFloat | -45 | ° | comAdvanced | Angle of first lateral lattice element set in Z direction. | ❌ |
| "lateral_lattice_angle_2" | Lateral lattice angle 2 | coFloat | 45 | ° | comAdvanced | Angle of second lateral lattice element set in Z direction. | ❌ |
| "infill_overhang_angle" | Infill overhang angle | coFloat | 60 | ° | comAdvanced | Angle of infill angled lines; 60 = pure honeycomb. | ❌ |

### Advanced (Strength)
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "align_infill_direction_to_model" | Align infill direction to model | coBool | 0 | - | comAdvanced | Rotate fill direction with model orientation for strength. | ❌ |
| "extra_solid_infills" | Insert solid layers | coString | "" | - | comAdvanced | Insert solid layers at specific heights using N/N#K/list syntax. | ❌ |
| "minimum_sparse_infill_area" | Minimum sparse infill threshold | coFloat | 15 | mm² | comAdvanced | Sparse infill smaller than this is replaced by solid infill. | ❌ |
| "infill_combination" | Infill combination | coBool | 0 | - | comAdvanced | Combine sparse infill of several layers to reduce print time. | ❌ |
| "infill_combination_max_layer_height" | Infill combination max layer height | coFloatOrPercent | 100% | mm or % | comAdvanced | Max layer height for combined sparse infill; 0 or 100% = nozzle. | ❌ |
| "detect_narrow_internal_solid_infill" | Detect narrow internal solid infill | coBool | 1 | - | comAdvanced | Auto-detect narrow areas and use concentric pattern for speed. | ❌ |
| "ensure_vertical_shell_thickness" | Ensure vertical shell thickness | coEnum | All | none/critical_only/moderate/all | comAdvanced | Add solid infill near sloping surfaces to maintain shell thickness. | ❌ |
| "bridge_angle" | External bridge infill direction | coFloat | 0 | ° | comAdvanced | Bridging angle override; also accessible from Strength > Advanced. | ❌ |
| "internal_bridge_angle" | Internal bridge infill direction | coFloat | 0 | ° | comAdvanced | Internal bridging angle override; also accessible from Strength > Advanced. | ❌ |
---


## Speed

### Initial layer speed
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "initial_layer_speed" | Initial layer | coFloat | 30 | mm/s | comAdvanced | Speed of initial layer except the solid infill part. | ❌ |
| "initial_layer_infill_speed" | Initial layer infill | coFloat | 60 | mm/s | comAdvanced | Speed of solid infill part of initial layer. | ❌ |
| "initial_layer_travel_speed" | Initial layer travel speed | coFloatOrPercent | 100% | mm/s or % | comAdvanced | Travel speed on initial layer; % of travel_speed. | ❌ |
| "slow_down_layers" | Number of slow layers | coInt | 0 | layers | comAdvanced | First few layers printed slower; speed increases linearly. | ❌ |

### Other layers speed
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "outer_wall_speed" | Outer wall | coFloat | 60 | mm/s | comAdvanced | Speed of outermost visible wall; slower for better quality. | ✅ |
| "inner_wall_speed" | Inner wall | coFloat | 60 | mm/s | comAdvanced | Speed of inner wall. | ✅ |
| "small_perimeter_speed" | Small perimeters | coFloatOrPercent | 50% | mm/s or % | comAdvanced | Speed for perimeters with radius <= threshold; % of outer wall speed. | ❌ |
| "small_perimeter_threshold" | Small perimeters threshold | coFloat | 0 | mm | comAdvanced | Radius threshold for small perimeter speed; 0 = no threshold effect. | ❌ |
| "sparse_infill_speed" | Sparse infill | coFloat | 100 | mm/s | comAdvanced | Speed of internal sparse infill. | ❌ |
| "internal_solid_infill_speed" | Internal solid infill | coFloat | 100 | mm/s | comAdvanced | Speed of internal solid infill, not top or bottom surface. | ❌ |
| "top_surface_speed" | Top surface | coFloat | 100 | mm/s | comAdvanced | Speed of top surface solid infill. | ❌ |
| "gap_infill_speed" | Gap infill | coFloat | 30 | mm/s | comAdvanced | Speed of gap fill; should be slower due to irregular width. | ❌ |
| "ironing_speed" | Ironing speed | coFloat | 20 | mm/s | comAdvanced | Print speed of ironing lines. | ✅ |
| "support_speed" | Support | coFloat | 80 | mm/s | comAdvanced | Speed of support material. | ✅ |
| "support_interface_speed" | Support interface | coFloat | 80 | mm/s | comAdvanced | Speed of support interface. | ❌ |

### Overhang speed
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "enable_overhang_speed" | Slow down for overhang | coBool | 1 | - | comAdvanced | Slow printing down for different overhang degrees. | ❌ |
| "slowdown_for_curled_perimeters" | Slow down for curled perimeters | coBool | 1 | - | comAdvanced | Additional slowdown where perimeters may have curled upwards. | ❌ |
| "overhang_1_4_speed" | 10% | coFloatOrPercent | 0 | mm/s or % | comAdvanced | Speed for overhang between 10-25% line width; 0 = wall speed. |
| "overhang_2_4_speed" | 25% | coFloatOrPercent | 0 | mm/s or % | comAdvanced | Speed for overhang between 25-50% line width; 0 = wall speed. |
| "overhang_3_4_speed" | 50% | coFloatOrPercent | 0 | mm/s or % | comAdvanced | Speed for overhang between 50-75% line width; 0 = wall speed. |
| "overhang_4_4_speed" | 75% | coFloatOrPercent | 0 | mm/s or % | comAdvanced | Speed for overhang between 75-100% line width; 0 = wall speed. |
| "bridge_speed" | External | coFloat | 25 | mm/s | comAdvanced | Speed of external bridge extrusions and 100% overhangs. |
| "internal_bridge_speed" | Internal | coFloatOrPercent | 150% | mm/s or % | comAdvanced | Speed of internal bridge extrusions; % of bridge_speed. |

### Travel speed
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "travel_speed" | Travel | coFloat | 120 | mm/s | comAdvanced | Speed of travel moves (faster, no extrusion). | ❌ |
| "travel_speed_z" | Z travel | coFloat | 0 | mm/s | comDevelop | Vertical travel speed; 0 = use travel speed directly in G-code. | ❌ |

### Acceleration
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "default_acceleration" | Normal printing | coFloat | 500 | mm/s² | comAdvanced | Default acceleration for both printing and travel. | ❌ |
| "outer_wall_acceleration" | Outer wall | coFloat | 500 | mm/s² | comAdvanced | Acceleration of outer walls; lower can improve quality. | ❌ |
| "inner_wall_acceleration" | Inner wall | coFloat | 10000 | mm/s² | comAdvanced | Acceleration of inner walls. | ❌ |
| "bridge_acceleration" | Bridge | coFloatOrPercent | 50% | mm/s² or % | comAdvanced | Acceleration of bridges; % of outer wall acceleration. | ❌ |
| "sparse_infill_acceleration" | Sparse infill | coFloatOrPercent | 100% | mm/s² or % | comAdvanced | Acceleration of sparse infill; % of default acceleration. | ❌ |
| "internal_solid_infill_acceleration" | Internal solid infill | coFloatOrPercent | 100% | mm/s² or % | comAdvanced | Acceleration of internal solid infill; % of default acceleration. | ❌ |
| "initial_layer_acceleration" | Initial layer | coFloat | 300 | mm/s² | comAdvanced | Acceleration of initial layer; lower improves adhesion. | ❌ |
| "top_surface_acceleration" | Top surface | coFloat | 500 | mm/s² | comAdvanced | Acceleration of top surface infill; lower improves finish. | ❌ |
| "travel_acceleration" | Travel | coFloat | 10000 | mm/s² | comAdvanced | Acceleration of travel moves. | ❌ |
| "accel_to_decel_enable" | Enable accel_to_decel | coBool | 1 | - | comAdvanced | Klipper max_accel_to_decel auto adjustment. | ❌ |
| "accel_to_decel_factor" | accel_to_decel factor | coPercent | 50% | % | comAdvanced | Klipper max_accel_to_decel as percentage of acceleration. | ❌ |

### Jerk (XY)
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "default_junction_deviation" | Junction Deviation | coFloat | 0 | mm | comAdvanced | Marlin junction deviation; replaces traditional XY jerk. | ❌ |
| "default_jerk" | Default | coFloat | 0 | mm/s | comAdvanced | Default jerk value for speed changes. | ❌ |
| "outer_wall_jerk" | Outer wall | coFloat | 9 | mm/s | comAdvanced | Jerk of outer walls. | ❌ |
| "inner_wall_jerk" | Inner wall | coFloat | 9 | mm/s | comAdvanced | Jerk of inner walls. | ❌ |
| "infill_jerk" | Infill | coFloat | 9 | mm/s | comAdvanced | Jerk for infill. | ❌ |
| "top_surface_jerk" | Top surface | coFloat | 9 | mm/s | comAdvanced | Jerk for top surface. | ❌ |
| "initial_layer_jerk" | Initial layer | coFloat | 9 | mm/s | comAdvanced | Jerk for initial layer. | ❌ |
| "travel_jerk" | Travel | coFloat | 12 | mm/s | comAdvanced | Jerk for travel moves. | ❌ |

### Advanced (Speed)
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "max_volumetric_extrusion_rate_slope" | Extrusion rate smoothing | coFloat | 0 | mm³/s² | comAdvanced | Smooths sudden extrusion rate changes; 0 = disabled. | ❌ |
| "max_volumetric_extrusion_rate_slope_segment_length" | Smoothing segment length | coFloat | 3.0 | mm | comAdvanced | Lower = smoother transitions but larger G-code file. | ❌ |
| "extrusion_rate_smoothing_external_perimeter_only" | Apply only on external features | coBool | 0 | - | comAdvanced | Apply extrusion rate smoothing only on external perimeters and overhangs. | ❌ |
---


## Support

### Support
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "enable_support" | Enable support | coBool | 0 | - | comSimple | Enable support generation for overhangs. | ❌ |
| "support_type" | Type | coEnum | normal(auto) | normal(auto)/tree(auto)/normal(manual)/tree(manual) | comSimple | Auto or manual support generation mode. | ✅ |
| "support_style" | Style | coEnum | Default (Grid/Organic) | normal: default/grid/snug; tree: organic/tree_slim/tree_strong/tree_hybrid | comAdvanced | Style and shape of support material; enum set depends on support_type. | ❌ |
| "support_threshold_angle" | Threshold angle | coInt | 30 | ° | comSimple | Support generated for overhangs with slope below this angle. | ❌ |
| "support_threshold_overlap" | Threshold overlap | coFloatOrPercent | 50% | mm or % | comSimple | Overlap threshold when threshold angle is zero. | ❌ |
| "raft_first_layer_density" | Initial layer density | coPercent | 90% | % | comAdvanced | Density of first raft or support layer. | ❌ |
| "raft_first_layer_expansion" | Initial layer expansion | coFloat | 2.0 | mm | comAdvanced | Expand first raft/support layer to improve adhesion. | ❌ |
| "support_on_build_plate_only" | On build plate only | coBool | 0 | - | comSimple | Don't create support on model surface; only on build plate. | ❌ |
| "support_critical_regions_only" | Support critical regions only | coBool | 0 | - | comAdvanced | Only create support for critical regions like sharp tails. | ❌ |
| "support_remove_small_overhang" | Ignore small overhangs | coBool | 1 | - | comAdvanced | Ignore small overhangs that probably don't need support. | ❌ |
| "enforce_support_layers" | Enforce support layers | coInt | 0 | layers | comDevelop | Generate support for this many layers from bottom regardless. | ❌ |
| "support_object_xy_distance" | Support/object XY distance | coFloat | 0.35 | mm | comAdvanced | XY separation distance between object and its support. | ❌ |
| "support_object_first_layer_gap" | Support/object first layer gap | coFloat | 0.2 | mm | comAdvanced | XY separation at first layer between object and support. | ❌ |
| "support_angle" | Pattern angle | coFloat | 0 | ° | comAdvanced | Rotation angle of the support pattern on horizontal plane. | ✅ |
| "support_top_z_distance" | Top Z distance | coFloat | 0.2 | mm | comAdvanced | Z gap between top support interface and object. | ❌ |
| "support_bottom_z_distance" | Bottom Z distance | coFloat | 0.2 | mm | comAdvanced | Z gap between bottom support interface and object. | ❌ |
| "support_expansion" | Normal Support expansion | coFloat | 0 | mm | Normal only | Expand (+) or shrink (-) horizontal span of normal support. | ❌ |

### Raft
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "raft_layers" | Raft layers | coInt | 0 | layers | comAdvanced | Number of support layers to raise object; helps prevent warping. | ❌ |
| "raft_contact_distance" | Raft contact Z distance | coFloat | 0.1 | mm | comAdvanced | Z gap between object and raft; ignored for soluble interface. | ❌ |
| "raft_expansion" | Raft expansion | coFloat | 1.5 | mm | comAdvanced | Expand all raft layers in XY plane. | ❌ |

### Support filament
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "support_filament" | Support/raft base | coInt | 0 | extruder index | comSimple | Extruder for support base and raft; 0 = default. | ❌ |
| "support_interface_filament" | Support/raft interface | coInt | 0 | extruder index | comSimple | Extruder for support interface; 0 = default. | ❌ |
| "support_interface_not_for_body" | Avoid interface filament for base | coBool | 1 | - | comSimple | Avoid using support interface filament for support base if possible. | ❌ |

### Support ironing
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "support_ironing" | Ironing Support Interface | coBool | 0 | - | comAdvanced | Iron the support interface for smoother surface. | ❌ |
| "support_air_filtration" | Support air filtration | coBool | 1 | - | comDevelop | Enable air filtration support (M106 P3) for enclosed printers. | ❌ |
| "support_ironing_pattern" | Support Ironing Pattern | coEnum | rectilinear | rectilinear/concentric | comAdvanced | Pattern used for support ironing. | ❌ |
| "support_ironing_flow" | Support Ironing flow | coPercent | 10% | % | comAdvanced | Material amount for support ironing; relative to layer height. | ❌ |
| "support_ironing_spacing" | Support Ironing line spacing | coFloat | 0.1 | mm | comAdvanced | Distance between support ironing lines. | ❌ |

### Interface
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "support_interface_top_layers" | Top interface layers | coInt | 3 | layers | comAdvanced | Number of top interface layers. | ✅ |
| "support_interface_bottom_layers" | Bottom interface layers | coInt | 0 (Same as top) | layers | comAdvanced | Number of bottom interface layers. | ✅ |
| "support_interface_pattern" | Interface pattern | coEnum | auto | auto/rectilinear/concentric/rectilinear_interlaced/grid | comAdvanced | Line pattern of support interface. | ❌ |
| "support_interface_spacing" | Top interface spacing | coFloat | 0.5 | mm | comAdvanced | Spacing of interface lines; 0 = solid interface. | ❌ |
| "support_bottom_interface_spacing" | Bottom interface spacing | coFloat | 0.5 | mm | comAdvanced | Spacing of bottom interface lines; 0 = solid. | ❌ |
| "support_interface_speed" | Support interface | coFloat | 80 | mm/s | comAdvanced | Speed of support interface (also in Speed tab). | ❌ |
| "support_interface_loop_pattern" | Interface use loop pattern | coBool | 0 | - | comAdvanced | Cover top contact layer of supports with loops. | ❌ |

### Advanced (Support)
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "support_base_pattern" | Base pattern | coEnum | default | default/rectilinear/rectilinear-grid/honeycomb/lightning/hollow | Normal only | Line pattern of support base. | ❌ |
| "support_base_pattern_spacing" | Base pattern spacing | coFloat | 2.5 | mm | Normal only | Spacing between support lines. | ❌ |
| "bridge_no_support" | Don't support bridges | coBool | 0 | - | comAdvanced | Don't support bridge area; bridges can print without support if short. | ❌ |
| "max_bridge_length" | Max bridge length | coFloat | 10 | mm | comAdvanced | Max bridge length that doesn't need support; 0 = all supported. | ❌ |
| "independent_support_layer_height" | Independent support layer height | coBool | 1 | - | comAdvanced | Support uses independent layer height from object. | ❌ |

### Tree supports
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "tree_support_wall_count" | Support wall loops | coInt | 0 (auto) | 0..2 | Tree only | Number of support walls; 0 = auto. | ✅ |
| "tree_support_tip_diameter" | Tip Diameter | coFloat | 0.8 | mm | Organic only | Branch tip diameter for organic supports. | ❌ |
| "tree_support_branch_distance" | Tree support branch distance | coFloat | 5.0 | mm | Tree (normal) only | Distance between neighboring tree support nodes. | ✅ |
| "tree_support_branch_distance_organic" | Organic branch distance | coFloat | 1.0 | mm | Organic only | Distance between neighboring organic support nodes. | ❌ |
| "tree_support_top_rate" | Branch Density | coPercent | 30% | % | Organic only | Density of support structure for branch tips. | ❌ |
| "tree_support_branch_diameter" | Tree support branch diameter | coFloat | 5.0 | mm | Tree (normal) only | Initial diameter of tree support nodes. | ✅ |
| "tree_support_branch_diameter_organic" | Organic branch diameter | coFloat | 2.0 | mm | Organic only | Initial diameter of organic support nodes. | ❌ |
| "tree_support_branch_diameter_angle" | Branch Diameter Angle | coFloat | 5 | ° | Organic only | Taper angle of branch diameter toward bottom; 0 = uniform. | ✅ |
| "tree_support_branch_angle" | Tree support branch angle | coFloat | 40 | ° | Tree (normal) only | Max overhang angle for tree support branches. | ❌ |
| "tree_support_branch_angle_organic" | Organic branch angle | coFloat | 40 | ° | Organic only | Max overhang angle for organic support branches. | ❌ |
| "tree_support_angle_slow" | Preferred Branch Angle | coFloat | 25 | ° | Organic only | Preferred angle of branches when not avoiding model. | ❌ |
| "tree_support_auto_brim" | Auto brim width | coBool | 1 | - | Tree (normal) only | Auto-calculate brim width for tree support. | ❌ |
| "tree_support_brim_width" | Tree support brim width | coFloat | 3 | mm | Tree (normal) only | Distance from tree branch to outermost brim line. | ❌ |
| "tree_support_with_infill" | Tree support with infill | coBool | 0 | - | Tree only | Add infill inside large hollows of tree support. | ❌ |
---


## Multimaterial

### Prime tower
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "enable_prime_tower" | Enable | coBool | 0 | - | comSimple | Enable the priming/wipe tower for multi-material printing. | ❌ |
| "prime_tower_skip_points" | Skip points | coBool | 1 | - | comAdvanced | Tower wall skips start points of wipe path. | ❌ |
| "prime_tower_flat_ironing" | Flat top ironing | coBool | 0 | - | comAdvanced | Iron top surface of prime tower interface layer. | ❌ |
| "filament_tower_interface_pre_extrusion_dist" | Interface pre-extrusion distance | coFloats | 10 | mm | comAdvanced | Pre-extrusion distance for interface layer where materials meet. | ❌ |
| "filament_tower_interface_pre_extrusion_length" | Interface pre-extrusion length | coFloats | 0 | mm | comAdvanced | Pre-extrusion length for interface layer where materials meet. | ❌ |
| "filament_tower_interface_print_temp" | Interface print temperature | coInts | -1 | °C | comAdvanced | Print temp for interface layer; -1 = max recommended. | ❌ |
| "filament_tower_interface_purge_volume" | Interface purge length | coFloats | 20 | mm | comAdvanced | Purge length for interface layer where materials meet. | ❌ |
| "filament_tower_ironing_area" | Tower ironing area | coFloats | 4 | mm² | comAdvanced | Ironing area for prime tower interface layer. | ❌ |
| "enable_tower_interface_features" | Enable tower interface features | coBool | 0 | - | comAdvanced | Optimized prime tower interface behavior when materials meet. | ❌ |
| "enable_tower_interface_cooldown_during_tower" | Cool down from interface boost | coBool | 0 | - | comAdvanced | Set nozzle back to print temp at start of tower for cooling. | ❌ |
| "prime_tower_enable_framework" | Internal ribs | coBool | 0 | - | comAdvanced | Internal ribs to increase prime tower stability. | ❌ |
| "prime_tower_width" | Width | coFloat | 60 | mm | comSimple | Width of the prime tower. | ❌ |
| "prime_volume" | Prime volume | coFloat | 45 | mm³ | comSimple | Volume of material to prime extruder on tower. | ❌ |
| "prime_tower_brim_width" | Brim width | coFloat | 3 | mm | comAdvanced | Brim width of prime tower; negative = auto calculated. | ❌ |
| "prime_tower_infill_gap" | Infill gap | coPercent | 150% | % | comAdvanced | Infill gap spacing for prime tower. | ❌ |
| "wipe_tower_rotation_angle" | Wipe tower rotation angle | coFloat | 0 | ° | comAdvanced | Rotation angle of wipe tower w.r.t. X axis. | ❌ |
| "wipe_tower_bridging" | Maximal bridging distance | coFloat | 10 | mm | comAdvanced | Max distance between supports on sparse infill sections. | ❌ |
| "wipe_tower_extra_spacing" | Wipe tower purge lines spacing | coPercent | 100% | % | comAdvanced | Spacing of purge lines on the wipe tower. | ❌ |
| "wipe_tower_extra_flow" | Extra flow for purging | coPercent | 100% | % | comAdvanced | Extra flow used for purge lines on wipe tower. | ❌ |
| "wipe_tower_max_purge_speed" | Max tower print speed | coFloat | 90 | mm/s | comAdvanced | Max print speed when purging and printing tower sparse layers. | ❌ |
| "wipe_tower_wall_type" | Wall type | coEnum | rectangle | rectangle/cone/rib | comAdvanced | Wipe tower outer wall shape. | ❌ |
| "wipe_tower_cone_angle" | Stabilization cone apex angle | coFloat | 30 | ° | comAdvanced | Angle at apex of stabilization cone; larger = wider base. | ❌ |
| "wipe_tower_extra_rib_length" | Extra rib length | coFloat | 0 | mm | comAdvanced | Increase (positive) or decrease (negative) rib wall size. | ❌ |
| "wipe_tower_rib_width" | Rib width | coFloat | 8 | mm | comAdvanced | Rib width; always less than half of prime tower side. | ❌ |
| "wipe_tower_fillet_wall" | Fillet wall | coBool | 1 | - | comAdvanced | Prime tower wall will have fillet edges. | ❌ |
| "wipe_tower_no_sparse_layers" | No sparse layers (beta) | coBool | 0 | - | comAdvanced | Don't print tower on layers with no tool changes. | ❌ |
| "single_extruder_multi_material_priming" | Prime all printing extruders | coBool | 0 | - | comAdvanced | Prime all extruders at front edge at start of print. | ❌ |
| "single_extruder_multi_material" | Single Extruder Multi Material | coBool | 1 | - | comAdvanced | Use single nozzle to print multi filament. | ❌ |
| "purge_in_prime_tower" | Purge in prime tower | coBool | 1 | - | comAdvanced | Purge remaining filament into prime tower during tool change. | ❌ |
| "enable_filament_ramming" | Enable filament ramming | coBool | 1 | - | comAdvanced | Enable filament ramming during tool change sequence. | ❌ |
| "manual_filament_change" | Manual Filament Change | coBool | 0 | - | comAdvanced | Skip custom change G-code for manual multi-material printing. | ❌ |
| "wipe_tower_x" | Position X | coFloats | 15 | mm | comDevelop | X coordinate of wipe tower left front corner. | ✅ |
| "wipe_tower_y" | Position Y | coFloats | 220 | mm | comDevelop | Y coordinate of wipe tower left front corner. | ✅ |

### Filament for Features
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "wall_filament" | Walls | coInt | 1 | extruder index | comAdvanced | Filament to print walls. | ❌ |
| "sparse_infill_filament" | Infill | coInt | 1 | extruder index | comAdvanced | Filament to print internal sparse infill. | ❌ |
| "solid_infill_filament" | Solid infill | coInt | 1 | extruder index | comAdvanced | Filament to print solid infill. | ❌ |
| "wipe_tower_filament" | Wipe tower | coInt | 0 | extruder index | comAdvanced | Extruder for wipe tower perimeter; 0 = available (non-soluble preferred). | ❌ |
| "filament_map" | Filament map to extruder | coInts | {1} | extruder index | comDevelop | Maps each filament slot to a physical extruder. | ❌ |
| "filament_map_mode" | Filament mapping mode | coEnum | Auto For Flush | Auto For Flush/Auto For Match/Manual/Default | comAdvanced | Mode for mapping filaments to extruders. | ❌ |

### Ooze prevention
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "ooze_prevention" | Enable | coBool | 0 | - | comAdvanced | Drop temperature of inactive extruders to prevent oozing. | ❌ |
| "standby_temperature_delta" | Temperature variation | coInt | -5 | ∆°C | comAdvanced | Temperature change when extruder is not active. | ❌ |
| "preheat_time" | Preheat time | coFloat | 30 | s | comAdvanced | Time to preheat next tool before tool change. | ❌ |
| "preheat_steps" | Preheat steps | coInt | 1 | - | comDevelop | Number of preheat commands; only useful for Prusa XL. | ❌ |

### Flush options
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "flush_into_infill" | Flush into objects' infill | coBool | 0 | - | comAdvanced | Purge after filament change inside object infill to reduce waste. | ❌ |
| "flush_into_objects" | Flush into this object | coBool | 0 | - | comAdvanced | Purge nozzle into this object after filament change. | ❌ |
| "flush_into_support" | Flush into objects' support | coBool | 1 | - | comAdvanced | Purge after filament change inside object support. | ❌ |
| "filament_flush_temp" | Flush temperature | coInts | 0 | °C | comAdvanced | Temperature when flushing; 0 = upper bound of recommended temp. | ❌ |
| "filament_flush_volumetric_speed" | Flush volumetric speed | coFloats | 0 | mm³/s | comAdvanced | Volumetric speed when flushing; 0 = max volumetric speed. | ❌ |
| "flush_multiplier" | Flush multiplier | coFloats | 0.3 | - | comAdvanced | Multiplier for flushing volumes in the table. | ❌ |
| "flush_volumes_matrix" | Purging volumes | coFloats | matrix of 0,280,... | - | comAdvanced | Matrix of purge volumes required between tool pairs. | ❌ |
| "flush_volumes_vector" | Purging volumes load/unload | coFloats | 140,140,... | - | comAdvanced | Required volumes to change from/to each tool. | ❌ |
| "wiping_volumes_extruders" | Purging volumes per extruder | coFloats | 70,70,... | - | comAdvanced | Tool change purge volumes per extruder. | ❌ |
| "nozzle_flush_dataset" | Nozzle flush dataset | coInts | 0 | - | comAdvanced | Nozzle flush dataset. | ❌ |

### Multimaterial advanced
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "interlocking_beam" | Use beam interlocking | coBool | 0 | - | comAdvanced | Interlocking beam structure at filament boundaries for adhesion. | ❌ |
| "interface_shells" | Interface shells | coBool | 0 | - | comAdvanced | Solid shells between adjacent materials/volumes. | ❌ |
| "mmu_segmented_region_max_width" | Max width of segmented region | coFloat | 0 | mm | comAdvanced | Max width of a segmented region; 0 = disabled. | ❌ |
| "mmu_segmented_region_interlocking_depth" | Interlocking depth of segmented region | coFloat | 0 | mm | comAdvanced | Interlocking depth for segmented regions; 0 = disabled. | ❌ |
| "interlocking_beam_width" | Interlocking beam width | coFloat | 0.8 | mm | comAdvanced | Width of interlocking structure beams. | ❌ |
| "interlocking_orientation" | Interlocking direction | coFloat | 22.5 | ° | comAdvanced | Orientation of interlock beams. | ❌ |
| "interlocking_beam_layer_count" | Interlocking beam layers | coInt | 2 | layers | comAdvanced | Height of beams measured in number of layers. | ❌ |
| "interlocking_depth" | Interlocking depth | coInt | 2 | cells | comAdvanced | Distance from filament boundary for interlocking; in cells. | ❌ |
| "interlocking_boundary_avoidance" | Interlocking boundary avoidance | coInt | 2 | cells | comAdvanced | Distance from model outside where no interlocking; in cells. | ❌ |
| "support_object_skip_flush" | Skip flush | coBool | 0 | - | comAdvanced | Skip flushing for support objects. | ❌ |
---


## Calibration

### Flow / Pressure advance calibration
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "calib_flowrate_topinfill_special_order" | Flowrate calib. top infill special order | coBool | 0 | - | comDevelop | Modified top infill order for flow rate calibration models. | ❌ |

> **Note:** Pressure advance options (`enable_pressure_advance`, `pressure_advance`, `adaptive_pressure_advance`, etc.) are documented under [Extruder / Nozzle → Pressure advance](#pressure-advance). These are the primary calibration controls for extrusion tuning.

---

## Others

### Skirt
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "skirt_loops" | Skirt loops | coInt | 1 | - | comSimple | Number of skirt loops; 0 = disabling skirt. | ✅ |
| "skirt_type" | Skirt type | coEnum | combined | combined/perobject | comAdvanced | Combined = single skirt; Per object = individual. | ❌ |
| "min_skirt_length" | Skirt minimum extrusion length | coFloat | 0 | mm | comAdvanced | Min filament extrusion length for skirt; 0 = disabled feature. | ❌ |
| "skirt_distance" | Skirt distance | coFloat | 2 | mm | comAdvanced | Distance from skirt to brim or object. | ✅ |
| "skirt_start_angle" | Skirt start point | coFloat | -135 | ° | comAdvanced | Angle from object center to skirt start point; 0 = rightmost. | ❌ |
| "skirt_speed" | Skirt speed | coFloat | 50 | mm/s | comAdvanced | Speed of skirt; 0 means use default layer extrusion speed. | ❌ |
| "skirt_height" | Skirt height | coInt | 1 | layers | comSimple | Number of skirt layers; usually only one. | ✅ |
| "draft_shield" | Draft shield | coEnum | disabled | disabled/enabled | comAdvanced | Tall skirt to protect print from drafts; enabled = max object height. | ❌ |
| "single_loop_draft_shield" | Single loop after first layer | coBool | 0 | - | comAdvanced | Limits draft shield to one wall after first layer. | ❌ |

### Brim
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "brim_type" | Brim type | coEnum | auto_brim | auto/brim_ears/painted/outer_only/inner_only/outer_and_inner/no_brim | comSimple | Controls brim generation: auto, ears, painted, etc. | ❌ |
| "brim_width" | Brim width | coFloat | 0 | mm | comSimple | Distance from model to outermost brim line. | ✅ |
| "brim_object_gap" | Brim-object gap | coFloat | 0 | mm | comAdvanced | Gap between innermost brim line and object for easy removal. | ❌ |
| "brim_use_efc_outline" | Brim follows compensated outline | coBool | 0 | - | comAdvanced | Align brim with first-layer perimeter after EFC. | ❌ |
| "brim_ears_max_angle" | Brim ear max angle | coFloat | 125 | ° | comAdvanced | Max angle for brim ear to appear; 0 = none; ~180 = all but straight. | ❌ |
| "brim_ears_detection_length" | Brim ear detection radius | coFloat | 1 | mm | comAdvanced | Decimation deviation for detecting sharp angles; 0 = deactivate. | ❌ |
| "brim_ears" | Brim ears | coBool | 0 | - | comAdvanced | Only draw brim over the sharp edges of the model. | ❌ |

### Special mode
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "slicing_mode" | Slicing Mode | coEnum | regular | regular/even_odd/close_holes | comAdvanced | Even-odd for 3DLabPrint models; Close holes to fill all holes. | ❌ |
| "print_sequence" | Print sequence | coEnum | by layer | by layer/by object | comSimple | Print layer by layer or object by object. | ❌ |
| "print_order" | Intra-layer order | coEnum | default | default/as_obj_list | comAdvanced | Print order within a single layer. | ❌ |
| "spiral_mode" | Spiral vase | coBool | 0 | - | comSimple | Spiralize Z moves; single-walled print with solid bottom, no seam. | ❌ |
| "spiral_mode_smooth" | Smooth Spiral | coBool | 0 | - | comSimple | Smooth XY moves too; no visible seam even on non-vertical walls. | ❌ |
| "spiral_mode_max_xy_smoothing" | Max XY Smoothing | coFloatOrPercent | 200% | mm or % | comAdvanced | Max XY point movement for smooth spiral; % of nozzle diameter. | ❌ |
| "spiral_starting_flow_ratio" | Spiral starting flow ratio | coFloat | 0 | - | comAdvanced | Starting flow ratio when transitioning from bottom to spiral. | ❌ |
| "spiral_finishing_flow_ratio" | Spiral finishing flow ratio | coFloat | 0 | - | comAdvanced | Finishing flow ratio when ending the spiral. | ❌ |
| "timelapse_type" | Timelapse | coEnum | Traditional | Traditional/Smooth | comSimple | Timelapse video recording mode: traditional or smooth. | ❌ |
| "enable_timelapse" | Enable timelapse for print | coBool | 0 | - | comSimple | Mark slicing as using timelapse recording. | ❌ |
| "enable_wrapping_detection" | Enable clumping detection | coBool | 0 | - | comAdvanced | Enable detection of filament clumping during print. | ❌ |
| "wrapping_detection_layers" | Clumping detection layers | coInt | 20 | layers | comDevelop | Number of layers for clumping detection. | ❌ |
| "wrapping_exclude_area" | Probing exclude area | coPoints | {} | - | comAdvanced | Bed area to exclude from clumping detection probing. | ❌ |

### Fuzzy Skin
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "fuzzy_skin" | Fuzzy Skin | coEnum | Disabled | none/external/all/allwalls/disabled_fuzzy | comSimple | Random jitter on walls for rough surface texture. | ✅ |
| "fuzzy_skin_mode" | Fuzzy skin generator mode | coEnum | displacement | displacement/extrusion/combined | comSimple | Mode for generating fuzzy skin pattern. | ❌ |
| "fuzzy_skin_noise_type" | Fuzzy skin noise type | coEnum | classic | classic/perlin/billow/ridgedmulti/voronoi | comSimple | Noise type for fuzzy skin texture generation. | ❌ |
| "fuzzy_skin_point_distance" | Fuzzy skin point distance | coFloat | 0.3 | mm | comSimple | Average distance between random jitter points on line segments. | ❌ |
| "fuzzy_skin_thickness" | Fuzzy skin thickness | coFloat | 0.2 | mm | comSimple | Width of jitter; should be below outer wall line width. | ❌ |
| "fuzzy_skin_scale" | Fuzzy skin feature size | coFloat | 1.0 | mm | comAdvanced | Base size of coherent noise features; higher = larger features. | ❌ |
| "fuzzy_skin_octaves" | Fuzzy Skin Noise Octaves | coInt | 4 | - | comAdvanced | Number of coherent noise octaves; higher = more detail. | ❌ |
| "fuzzy_skin_persistence" | Fuzzy skin noise persistence | coFloat | 0.5 | - | comAdvanced | Decay rate for higher noise octaves; lower = smoother. | ❌ |
| "fuzzy_skin_first_layer" | Apply fuzzy skin to first layer | coBool | 0 | - | comSimple | Whether to apply fuzzy skin effect on first layer. | ❌ |

### G-code output
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "reduce_infill_retraction" | Reduce infill retraction | coBool | 0 | - | comAdvanced | Skip retraction when travel is within infill areas. | ❌ |
| "gcode_add_line_number" | Add line number | coBool | 0 | - | comDevelop | Add line number (Nx) at beginning of each G-code line. | ❌ |
| "gcode_comments" | Verbose G-code | coBool | 0 | - | comAdvanced | Commented G-code file with descriptive text for each line. | ❌ |
| "gcode_label_objects" | Label objects | coBool | 1 | - | comAdvanced | Add comments labeling print moves with object for CancelObject. | ❌ |
| "exclude_object" | Exclude objects | coBool | 0 | - | comAdvanced | Add EXCLUDE OBJECT command in G-code. | ❌ |
| "filename_format" | Filename format | coString | {input_filename_base}_... | - | comAdvanced | Template for project file name when exporting. | ❌ |
| "gcode_flavor" | G-code flavor | coEnum | Marlin(legacy) | marlin/klipper/reprapfirmware/marlin2 | comAdvanced | G-code compatibility type for printer firmware. | ❌ |
| "use_relative_e_distances" | Use relative E distances | coBool | 1 | - | comAdvanced | Relative extrusion mode; recommended for label_objects and wipe tower. | ✅ |

### Post-processing Scripts
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "post_process" | Post-processing Scripts | coStrings | "" | - | comAdvanced | Absolute paths to custom scripts for processing output G-code. | ❌ |

### Notes
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "notes" | Configuration notes | coString | "" | - | comAdvanced | Personal notes added to G-code header comments. | ❌ |
---


## Cooling
These options typically appear in the Filament tab.

| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "enable_overhang_bridge_fan" | Force cooling for overhangs and bridges | coBools | 1 | - | comSimple | Adjust fan speed specifically for overhangs and bridges. | ❌ |
| "overhang_fan_speed" | Overhangs and external bridges fan speed | coInts | 100 | % | comAdvanced | Fan speed when printing bridges/overhangs exceeding threshold. | ✅ |
| "overhang_fan_threshold" | Overhang cooling activation threshold | coEnums | 50% | 0%/10%/25%/50%/75%/95% | comAdvanced | Overhang % threshold to activate overhang fan speed. | ❌ |
| "internal_bridge_fan_speed" | Internal bridges fan speed | coInts | -1 | % | comAdvanced | Fan speed for internal bridges; -1 = use overhang fan settings. | ❌ |
| "ironing_fan_speed" | Ironing fan speed | coInts | -1 | % | comAdvanced | Fan speed when ironing; -1 = disabled. | ❌ |
| "full_fan_speed_layer" | Full fan speed at layer | coInts | 0 | layer | comAdvanced | Layer at which fan reaches maximum speed. | ❌ |
| "close_fan_the_first_x_layers" | No cooling for the first | coInts | 1 | layers | comSimple | Turn off all cooling fans for first few layers for adhesion. | ❌ |
| "slow_down_for_layer_cooling" | Slow printing down for better cooling | coBools | 1 | - | comSimple | Slow speed to meet minimum layer time for cooling. | ✅ |
| "dont_slow_down_outer_wall" | Don't slow down outer walls | coBools | 0 | - | comSimple | Ensure external perimeters aren't slowed for min layer time. | ❌ |
| "slow_down_layer_time" | Layer time | coFloats | 5 | s | comSimple | Slow down when estimated layer time is shorter than this. | ✅ |
| "slow_down_min_speed" | Min print speed | coFloats | 10 | mm/s | comAdvanced | Minimum print speed when slowing down for cooling. | ✅ |
| "fan_max_speed" | Fan speed (max) | coFloats | 100 | % | comSimple | Maximum part cooling fan speed. | ❌ |
| "fan_min_speed" | Fan speed (min) | coFloats | 20 | % | comSimple | Minimum part cooling fan speed. | ❌ |
| "fan_cooling_layer_time" | Layer time (fan activation) | coFloats | 60 | s | comSimple | Fan enabled when estimated layer time is shorter than this. | ❌ |
| "auxiliary_fan" | Auxiliary part cooling fan | coBool | 0 | - | comAdvanced | Enable if machine has auxiliary part cooling fan (M106 P2). | ❌ |
| "additional_cooling_fan_speed" | Fan speed (auxiliary) | coInts | 0 | % | comSimple | Speed of auxiliary fan during printing. | ❌ |
| "reduce_fan_stop_start_freq" | Keep fan always on | coBools | 0 | - | comSimple | Fan never stops; runs at min speed to reduce on/off cycles. | ❌ |
| "fan_speedup_time" | Fan speed-up time | coFloat | 0 | s | comAdvanced | Start fan this many seconds earlier than target; 0 = disabled. | ❌ |
| "fan_speedup_overhangs" | Only overhangs | coBool | 1 | - | comAdvanced | Only apply fan speed-up delay for overhang cooling. | ❌ |
| "fan_kickstart" | Fan kick-start time | coFloat | 0 | s | comAdvanced | Max fan speed for this duration before reducing to target. | ❌ |
| "support_material_interface_fan_speed" | Support interface fan speed | coInts | -1 | % | comAdvanced | Fan speed for support interfaces; -1 = disabled. | ❌ |
| "during_print_exhaust_fan_speed" | Exhaust fan speed (printing) | coInts | 60 | % | comSimple | Speed of exhaust fan during printing. | ❌ |
| "complete_print_exhaust_fan_speed" | Exhaust fan speed (after print) | coInts | 80 | % | comSimple | Speed of exhaust fan after printing completes. | ❌ |
| "activate_air_filtration" | Activate air filtration | coBools | 0 | - | comSimple | Activate air filtration (M106 P3). | ❌ |
| "max_layer_height" | Max layer height | coFloats | 0 | mm | comAdvanced | Highest printable layer height for extruder. | ❌ |
| "min_layer_height" | Min layer height | coFloats | 0.07 | mm | comAdvanced | Lowest printable layer height for extruder. | ❌ |
| "activate_chamber_temp_control" | Activate chamber temperature control | coBools | 0 | - | comSimple | Enable automated chamber temperature control (M191/M141). | ❌ |
---


## Filament
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "filament_type" | Type | coStrings | PLA | - | comSimple | Material type of filament from shared material database. | ❌ |
| "filament_density" | Density | coFloats | 0 | g/cm³ | comAdvanced | Filament density for statistics only. | ❌ |
| "filament_diameter" | Diameter | coFloats | 1.75 | mm | comAdvanced | Filament diameter used for extrusion calculation; must be accurate. | ❌ |
| "filament_flow_ratio" | Flow ratio | coFloats | 1 | - | comAdvanced | Proportional extrusion flow adjustment; recommended range 0.95-1.05. | ❌ |
| "filament_cost" | Price | coFloats | 0 | money/kg | comAdvanced | Filament price for statistics only. | ❌ |
| "filament_colour" | Color | coStrings | #F2754E | - | comAdvanced | Visual color on UI only. | ❌ |
| "default_filament_colour" | Default color | coStrings | "" | - | comAdvanced | Default filament color; right-click to reset. | ❌ |
| "filament_vendor" | Vendor | coStrings | (Undefined) | - | comAdvanced | Vendor of filament for display only. | ❌ |
| "filament_notes" | Filament notes | coStrings | "" | - | comAdvanced | Personal notes regarding the filament. | ❌ |
| "filament_settings_id" | Settings ID | coStrings | "" | - | comAdvanced | Filament settings identifier. | ❌ |
| "filament_ids" | Filament IDs | coStrings | "" | - | comAdvanced | Filament IDs for system matching. | ❌ |
| "filament_soluble" | Soluble material | coBools | 0 | - | comAdvanced | Soluble material commonly used for support and interface. | ❌ |
| "filament_is_support" | Support material | coBools | 0 | - | comAdvanced | Support material for supports and support interfaces. | ❌ |
| "filament_printable" | Filament printable | coInts | 3 | bits | comDevelop | Bitmask: 0 = cannot support, 1 = can support left, 2 = right. | ❌ |
| "filament_multi_colour" | Multi colour | coStrings | "" | - | comAdvanced | Multi colour data. | ❌ |
| "filament_colour_type" | Colour type | coStrings | 1 | - | comAdvanced | 0 = gradient color; 1 = default (single or multi color). | ❌ |
| "filament_adaptive_volumetric_speed" | Adaptive volumetric speed | coBools | 0 | - | comAdvanced | Limit flow by fitted max flow from line width/layer height. | ❌ |
| "filament_max_volumetric_speed" | Max volumetric speed | coFloats | 2 | mm³/s | comAdvanced | Max melt volume per second; printing speed limited by this. | ❌ |
| "volumetric_speed_coefficients" | Max volumetric speed coefficients | coStrings | "" | - | comAdvanced | Multinomial coefficients for max volumetric speed. | ❌ |
| "filament_shrink" | Shrinkage (XY) | coPercents | 100% | % | comAdvanced | Shrinkage percentage after cooling; part scaled in XY to compensate. | ❌ |
| "filament_shrinkage_compensation_z" | Shrinkage (Z) | coPercents | 100% | % | comAdvanced | Shrinkage percentage in Z direction. | ❌ |
| "filament_loading_speed" | Loading speed | coFloats | 28 | mm/s | comAdvanced | Speed used for loading filament on wipe tower. | ❌ |
| "filament_loading_speed_start" | Loading speed at start | coFloats | 3 | mm/s | comAdvanced | Speed at very beginning of loading phase. | ❌ |
| "filament_unloading_speed" | Unloading speed | coFloats | 90 | mm/s | comAdvanced | Speed for unloading filament on wipe tower. | ❌ |
| "filament_unloading_speed_start" | Unloading speed at start | coFloats | 100 | mm/s | comAdvanced | Speed for unloading tip immediately after ramming. | ❌ |
| "filament_cooling_moves" | Number of cooling moves | coInts | 4 | - | comAdvanced | Number of back-and-forth cooling moves in cooling tubes. | ❌ |
| "filament_cooling_initial_speed" | Speed of first cooling move | coFloats | 2.2 | mm/s | comAdvanced | Starting speed for gradually accelerating cooling moves. | ❌ |
| "filament_cooling_final_speed" | Speed of last cooling move | coFloats | 3.4 | mm/s | comAdvanced | Final speed for gradually accelerating cooling moves. | ❌ |
| "filament_ramming_parameters" | Ramming parameters | coStrings | "120 100 ..." | - | comAdvanced | Ramming specific parameters edited by RammingDialog. | ❌ |
| "filament_multitool_ramming" | Enable ramming for multi-tool | coBools | 0 | - | comAdvanced | Perform ramming when using multi-tool printer setup. | ❌ |
| "filament_multitool_ramming_volume" | Multi-tool ramming volume | coFloats | 10 | mm³ | comAdvanced | Volume to ram before tool change. | ❌ |
| "filament_multitool_ramming_flow" | Multi-tool ramming flow | coFloats | 10 | mm³/s | comAdvanced | Flow used for ramming before tool change. | ❌ |
| "filament_stamping_loading_speed" | Stamping loading speed | coFloats | 0 | mm/s | comAdvanced | Speed used for stamping. | ❌ |
| "filament_stamping_distance" | Stamping distance | coFloats | 0 | mm | comAdvanced | Stamping movement length measured from cooling tube center. | ❌ |
| "filament_toolchange_delay" | Delay after unloading | coFloats | 0 | s | comAdvanced | Time to wait after filament is unloaded for reliable changes. | ❌ |
| "filament_change_length" | Filament ramming length | coFloats | 10 | mm | comAdvanced | Extrude this length from original extruder to minimize oozing. | ❌ |
| "filament_minimal_purge_on_wipe_tower" | Minimal purge on wipe tower | coFloats | 15 | mm³ | comAdvanced | Minimum prime volume on wipe tower before infill/object purging. | ❌ |
| "filament_adhesiveness_category" | Adhesiveness Category | coInts | 0 | - | comDevelop | Filament category for adhesion. | ❌ |
| "temperature_vitrification" | Softening temperature | coInts | 100 | °C | comSimple | Material softening temp; bed >= this means open door/remove glass. | ❌ |
| "filament_ironing_flow" | Ironing flow (filament override) | coPercents | nil | % | comAdvanced | Per-filament override for ironing flow. | ❌ |
| "filament_ironing_spacing" | Ironing line spacing (filament override) | coFloats | nil | mm | comAdvanced | Per-filament override for ironing spacing. | ❌ |
| "filament_ironing_inset" | Ironing inset (filament override) | coFloats | nil | mm | comAdvanced | Per-filament override for ironing inset. | ❌ |
| "filament_ironing_speed" | Ironing speed (filament override) | coFloats | nil | mm/s | comAdvanced | Per-filament override for ironing speed. | ❌ |

### Temperature (Nozzle)
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "nozzle_temperature" | Nozzle temperature (other layers) | coInts | 200 | °C | comSimple | Nozzle temperature for layers after the initial one. | ❌ |
| "nozzle_temperature_initial_layer" | Initial layer nozzle temperature | coInts | 200 | °C | comSimple | Nozzle temperature for printing the initial layer. | ✅ |
| "nozzle_temperature_range_low" | Min nozzle temperature | coInts | 190 | °C | comAdvanced | Minimum recommended nozzle temperature for this filament. | ❌ |
| "nozzle_temperature_range_high" | Max nozzle temperature | coInts | 240 | °C | comAdvanced | Maximum recommended nozzle temperature for this filament. | ❌ |
| "chamber_temperature" | Chamber temperature | coInts | 0 | °C | comSimple | Chamber temp for high-temp materials; 0 = disabled. | ❌ |
| "idle_temperature" | Idle temperature | coInts | 0 | °C | comAdvanced | Nozzle temp when tool unused; only when ooze prevention active. | ❌ |

### Bed temperature
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "cool_plate_temp" | Cool Plate temperature (other layers) | coInts | 35 | °C | comAdvanced | Bed temp for Cool Plate. | ❌ |
| "cool_plate_temp_initial_layer" | Cool Plate temperature (initial layer) | coInts | 35 | °C | comAdvanced | Initial layer bed temp for Cool Plate. | ❌ |
| "textured_cool_plate_temp" | Textured Cool Plate temp (other layers) | coInts | 40 | °C | comAdvanced | Bed temp for Textured Cool Plate. | ❌ |
| "textured_cool_plate_temp_initial_layer" | Textured Cool Plate temp (initial) | coInts | 40 | °C | comAdvanced | Initial layer bed temp for Textured Cool Plate. | ❌ |
| "eng_plate_temp" | Engineering Plate temp (other layers) | coInts | 45 | °C | comAdvanced | Bed temp for Engineering Plate. | ❌ |
| "eng_plate_temp_initial_layer" | Engineering Plate temp (initial) | coInts | 45 | °C | comAdvanced | Initial layer bed temp for Engineering Plate. | ❌ |
| "hot_plate_temp" | High Temp Plate temp (other layers) | coInts | 45 | °C | comAdvanced | Bed temp for High Temp Plate. | ❌ |
| "hot_plate_temp_initial_layer" | High Temp Plate temp (initial) | coInts | 45 | °C | comAdvanced | Initial layer bed temp for High Temp Plate. | ❌ |
| "textured_plate_temp" | Textured PEI Plate temp (other layers) | coInts | 45 | °C | comAdvanced | Bed temp for Textured PEI Plate. | ❌ |
| "textured_plate_temp_initial_layer" | Textured PEI Plate temp (initial) | coInts | 45 | °C | comAdvanced | Initial layer bed temp for Textured PEI Plate. | ❌ |
| "supertack_plate_temp" | SuperTack Plate temp (other layers) | coInts | 35 | °C | comAdvanced | Bed temp for Cool Plate SuperTack. | ❌ |
| "supertack_plate_temp_initial_layer" | SuperTack Plate temp (initial) | coInts | 35 | °C | comAdvanced | Initial layer bed temp for Cool Plate SuperTack. | ❌ |
| "curr_bed_type" | Bed type | coEnum | Cool Plate | 6 bed types | comSimple | Current bed type for temperature selection. | ❌ |
| "default_bed_type" | Default bed type | coString | "" | - | comAdvanced | Default bed type for printer profile; not shown in UI. | ❌ |
| "bed_temperature_formula" | Bed temperature type | coEnum | By Highest Temp | by_first_filament/by_highest_temp | comAdvanced | Determines bed temp from first or highest filament temp. | ❌ |
| "support_multi_bed_types" | Support multi bed types | coBool | 0 | - | comSimple | Enable multiple bed types for this printer. | ❌ |
| "support_chamber_temp_control" | Support chamber temp control | coBool | 1 | - | comDevelop | Enable M141 chamber temp control G-code. | ❌ |
---


## Extruder / Nozzle

### Retraction
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "retraction_length" | Length | coFloats | 0.8 | mm | comSimple | Amount of filament pulled back to avoid ooze during travel. | ❌ |
| "retraction_speed" | Retraction Speed | coFloats | 30 | mm/s | comAdvanced | Speed for retracting filament from the nozzle. | ❌ |
| "deretraction_speed" | De-retraction Speed | coFloats | 0 | mm/s | comAdvanced | Speed for reloading filament; 0 = same as retraction speed. | ❌ |
| "retract_restart_extra" | Extra length on restart | coFloats | 0 | mm | comAdvanced | Additional filament pushed after travel move compensation. | ❌ |
| "retract_restart_extra_toolchange" | Extra length on restart (toolchange) | coFloats | 0 | mm | comAdvanced | Additional filament after tool change compensation. | ❌ |
| "retraction_minimum_travel" | Travel distance threshold | coFloats | 2 | mm | comAdvanced | Only trigger retraction for travel longer than this. | ❌ |
| "retract_before_wipe" | Retract amount before wipe | coPercents | 100% | % | comAdvanced | Fast retraction length before wipe; % of retraction length. | ❌ |
| "retract_when_changing_layer" | Retract when change layer | coBools | 0 | - | comAdvanced | Force a retraction when changing layer. | ❌ |
| "z_hop" | Z-hop height | coFloats | 0.4 | mm | comSimple | Nozzle lift during retraction; prevents hitting the print. | ❌ |
| "z_hop_types" | Z-hop type | coEnums | Slope | Auto/Normal/Slope/Spiral | comAdvanced | Type of Z-hop movement. | ❌ |
| "retract_lift_above" | Z-hop lower boundary | coFloats | 0 | mm | comAdvanced | Z-hop only active when Z position is above this value. | ❌ |
| "retract_lift_below" | Z-hop upper boundary | coFloats | 0 | mm | comAdvanced | Z-hop only active when Z position is below this value. | ❌ |
| "retract_lift_enforce" | On surfaces | coEnums | All Surfaces | All/Top Only/Bottom Only/Top and Bottom | comAdvanced | Enforce Z-hop behavior on specified surfaces. | ❌ |
| "travel_slope" | Traveling angle | coFloats | 3 | ° | comAdvanced | Angle for Slope/Spiral Z-hop; 90 = Normal Lift. | ❌ |
| "use_firmware_retraction" | Use firmware retraction | coBool | 0 | - | comAdvanced | Use G10/G11 for firmware-handled retraction (Marlin). | ❌ |
| "wipe" | Wipe while retracting | coBools | 0 | - | comAdvanced | Move nozzle along last extrusion path when retracting. | ✅ |
| "wipe_distance" | Wipe Distance | coFloats | 1 | mm | comAdvanced | Length of wipe movement along last path. | ❌ |
| "long_retractions_when_cut" | Long retraction when cut (beta) | coBools | 0 | - | comDevelop | Retract longer during changes to minimize purge. | ❌ |
| "retraction_distances_when_cut" | Retraction distance when cut | coFloats | 18 | mm | comDevelop | Retraction length before cutting off during filament change. | ❌ |
| "long_retractions_when_ec" | Long retraction when extruder change | coBools | 0 | - | comAdvanced | Long retraction on extruder change. | ❌ |
| "retraction_distances_when_ec" | Retraction distance when EC | coFloats | 10 | mm | comAdvanced | Retraction length when extruder change. | ❌ |
| "retract_length_toolchange" | Length (toolchange) | coFloats | 10 | mm | comAdvanced | Retraction length triggered before tool change. | ❌ |
| "z_offset" | Z offset | coFloat | 0 | mm | comAdvanced | Compensation added/subtracted from all Z coordinates. | ❌ |

### Nozzle
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "nozzle_diameter" | Nozzle diameter | coFloats | 0.4 | mm | comAdvanced | Diameter of nozzle. | ❌ |
| "nozzle_type" | Nozzle type | coEnums | undefine | undefine/hardened_steel/stainless_steel/tungsten_carbide/brass | comAdvanced | Metallic material of nozzle for abrasive resistance. | ❌ |
| "nozzle_hrc" | Nozzle HRC | coInt | 0 | HRC | comDevelop | Nozzle hardness; 0 = no checking during slicing. | ❌ |
| "nozzle_height" | Nozzle height | coFloat | 2.5 | mm | comDevelop | Height of nozzle tip. | ❌ |
| "nozzle_volume" | Nozzle volume | coFloats | 0 | mm³ | comAdvanced | Volume between cutter and end of nozzle. | ❌ |
| "nozzle_volume_type" | Nozzle Volume Type | coEnums | Standard | Standard/High Flow | comSimple | Nozzle flow capacity type. | ❌ |
| "default_nozzle_volume_type" | Default Nozzle Volume Type | coEnums | Standard | Standard/High Flow | comDevelop | Default nozzle volume type for extruders in this printer. | ❌ |
| "required_nozzle_HRC" | Required nozzle HRC | coInts | 0 | HRC | comDevelop | Minimum nozzle HRC required for filament; 0 = no check. | ❌ |

### Pressure advance
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "enable_pressure_advance" | Enable pressure advance | coBools | 0 | - | comAdvanced | Enable PA; auto calibration result overwritten once enabled. | ❌ |
| "pressure_advance" | Pressure advance | coFloats | 0.02 | - | comAdvanced | PA value for Klipper (pressure advance) or Marlin (linear advance). | ❌ |
| "adaptive_pressure_advance" | Enable adaptive PA (beta) | coBools | 0 | - | comAdvanced | Model PA based on print conditions for optimal value per feature. | ❌ |
| "adaptive_pressure_advance_model" | Adaptive PA measurements (beta) | coStrings | "0,0,0\n0,0,0" | - | comAdvanced | PA values, volumetric flow speeds, accelerations per line. | ❌ |
| "adaptive_pressure_advance_overhangs" | Adaptive PA for overhangs (beta) | coBools | 0 | - | comAdvanced | Enable adaptive PA for overhangs and flow changes. | ❌ |
| "adaptive_pressure_advance_bridges" | Pressure advance for bridges | coFloats | 0.0 | - | comAdvanced | PA value for bridges; 0 = disabled. | ❌ |

### MMU Hardware
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "cooling_tube_retraction" | Cooling tube position | coFloat | 91.5 | mm | comAdvanced | Distance of cooling tube center-point from extruder tip. | ❌ |
| "cooling_tube_length" | Cooling tube length | coFloat | 5 | mm | comAdvanced | Length of cooling tube for cooling move space limit. | ❌ |
| "high_current_on_filament_swap" | High current on filament swap | coBool | 0 | - | comAdvanced | Increase extruder motor current during filament exchange. | ❌ |
| "parking_pos_retraction" | Filament parking position | coFloat | 92 | mm | comAdvanced | Distance from extruder tip to filament parking position. | ❌ |
| "extra_loading_move" | Extra loading distance | coFloat | -2 | mm | comAdvanced | Extra loading beyond parking position; negative = shorter load. | ❌ |
| "grab_length" | Grab length | coFloats | 0 | mm | comDevelop | Grab length for filament handling. | ❌ |
| "start_end_points" | Start end points | coPoints | (30,-3),(54,245) | - | comDevelop | Start and end points from cutter area to garbage can. | ❌ |

### Extruder geometry / mapping
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "extruder_colour" | Extruder Color | coStrings | "" | - | comAdvanced | Visual color on UI only. | ❌ |
| "extruder_offset" | Extruder offset | coPoints | (0,0) | mm | comAdvanced | Displacement of each extruder relative to first one. | ❌ |
| "extruder_type" | Type | coEnums | Direct Drive | Direct Drive/Bowden | comAdvanced | Extruder type for pressure advance default value. | ❌ |
| "extruder_variant_list" | Extruder variant list | coStrings | Direct Drive Standard | - | comDevelop | List of extruder variants. | ❌ |
| "extruder_ams_count" | Extruder AMS count | coStrings | {} | - | comDevelop | AMS counts per extruder. | ❌ |
| "printer_extruder_id" | Printer extruder id | coInts | 1 | - | comDevelop | Extruder ID in printer. | ❌ |
| "printer_extruder_variant" | Printer's extruder variant | coStrings | Direct Drive Standard | - | comDevelop | Printer's extruder variant. | ❌ |
| "master_extruder_id" | Master extruder id | coInt | 1 | - | comDevelop | Default extruder to place filament. | ❌ |
| "print_extruder_id" | Print extruder id | coInts | 1 | - | comDevelop | Extruder ID for printing. | ❌ |
| "print_extruder_variant" | Print's extruder variant | coStrings | Direct Drive Standard | - | comDevelop | Print extruder variant. | ❌ |
| "physical_extruder_map" | Physical extruder map | coInts | 0 | - | comDevelop | Map logical extruder to physical extruder. | ❌ |
| "filament_extruder_variant" | Filament's extruder variant | coStrings | Direct Drive Standard | - | comDevelop | Filament extruder variant. | ❌ |
| "filament_self_index" | Filament self index | coInts | 1 | - | comDevelop | Filament self index. | ❌ |
---


## Printer / Machine

### Print volume
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "printable_area" | Printable area | coPoints | (0,0),(200,0),(200,200),(0,200) | - | comAdvanced | Shape of printable area defined by polygon points. | ❌ |
| "printable_height" | Printable height | coFloat | 100 | mm | comSimple | Maximum printable height limited by printer mechanism. | ❌ |
| "extruder_printable_area" | Extruder printable area | coPointsGroups | {} | - | comAdvanced | Per-extruder printable area override. | ❌ |
| "extruder_printable_height" | Extruder printable height | coFloats | 0 | mm | comAdvanced | Per-extruder max printable height. | ❌ |
| "bed_exclude_area" | Bed exclude area | coPoints | (0,0) | - | comAdvanced | Unprintable area in XY plane (e.g. filament cut area). | ❌ |
| "bed_custom_texture" | Bed custom texture | coString | "" | - | comAdvanced | Custom bed texture image. | ❌ |
| "bed_custom_model" | Bed custom model | coString | "" | - | comAdvanced | Custom bed model STL file. | ❌ |
| "extruder_clearance_height_to_rod" | Height to rod | coFloat | 40 | mm | comAdvanced | Distance of nozzle tip to lower rod; used for by-object collision. | ❌ |
| "extruder_clearance_height_to_lid" | Height to lid | coFloat | 120 | mm | comAdvanced | Distance of nozzle tip to lid; used for by-object collision. | ❌ |
| "extruder_clearance_radius" | Radius | coFloat | 40 | mm | comAdvanced | Clearance radius around extruder for by-object printing. | ❌ |
| "preferred_orientation" | Preferred orientation | coFloat | 0 | ° | comAdvanced | Auto-orient STL files on Z axis upon import. | ❌ |
| "best_object_pos" | Best object position | coPoint | (0.5,0.5) | - | comAdvanced | Best auto-arranging position in range [0,1] w.r.t. bed. | ❌ |

### Printer identity
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "printer_technology" | Printer technology | coEnum | FFF | FFF/SLA | comSimple | Printer type: FFF (FDM) or SLA (resin). | ❌ |
| "printer_model" | Printer type | coString | "" | - | comSimple | Type/model of the printer. | ❌ |
| "printer_variant" | Printer variant | coString | "" | - | comSimple | Variant name (e.g., differentiated by nozzle diameter). | ❌ |
| "printer_structure" | Printer structure | coEnum | undefine | undefine/corexy/i3/hbot/delta | comDevelop | Physical arrangement of printer components. | ❌ |
| "printer_notes" | Printer notes | coString | "" | - | comAdvanced | Personal notes added to G-code header. | ❌ |
| "gcode_flavor" | G-code flavor | coEnum | Marlin(legacy) | marlin/klipper/reprapfirmware/marlin2 | comAdvanced | Firmware G-code compatibility type. | ❌ |
| "host_type" | Host Type | coEnum | Octo/Klipper | octoprint/duet/flashair/astrobox/repetier/... | comAdvanced | Print host type for G-code upload. | ❌ |
| "print_host" | Hostname, IP or URL | coString | "" | - | comAdvanced | Printer host address for G-code upload. | ❌ |
| "print_host_webui" | Device UI | coString | "" | - | comAdvanced | Device UI URL if different from print_host. | ❌ |
| "printhost_apikey" | API Key / Password | coString | "" | - | comAdvanced | API key or password for print host authentication. | ❌ |
| "printhost_port" | Printer | coString | "" | - | comAdvanced | Name of the printer on the host. | ❌ |
| "printhost_cafile" | HTTPS CA File | coString | "" | - | comAdvanced | Custom CA certificate file for HTTPS connections. | ❌ |
| "printhost_user" | User | coString | "" | - | comAdvanced | Print host login username. | ❌ |
| "printhost_password" | Password | coString | "" | - | comAdvanced | Print host login password. | ❌ |
| "printhost_authorization_type" | Authorization Type | coEnum | API key | API key/HTTP digest | comAdvanced | Authorization type for print host. | ❌ |
| "printhost_ssl_ignore_revoke" | Ignore HTTPS cert revocation | coBool | 0 | - | comAdvanced | Ignore HTTPS certificate revocation checks. | ❌ |
| "bbl_use_printhost" | Use 3rd-party print host | coBool | 0 | - | comAdvanced | Allow controlling BBL printer through 3rd party hosts. | ❌ |
| "allow_mix_temp" | Allow mixed filament temperatures | coBool | 0 | - | comAdvanced | Allow filaments with different nozzle temps to print together. | ❌ |
| "allow_multicolor_oneplate" | Allow multi-color on one plate | coBool | 1 | - | comAdvanced | Arrange allows multiple colors on one plate when enabled. | ❌ |
| "printer_agent" | Printer Agent | coString | "" | - | comAdvanced | Network agent implementation for printer communication. | ❌ |
| "pellet_modded_printer" | Pellet Modded Printer | coBool | 0 | - | comSimple | Enable if printer uses pellets instead of filaments. | ❌ |
| "pellet_flow_coefficient" | Pellet flow coefficient | coFloats | 0.4157 | - | comAdvanced | Empirically derived coefficient for pellet volume to filament diameter. | ❌ |
| "printer_settings_id" | Printer settings ID | coString | "" | - | comAdvanced | Printer preset identifier. | ❌ |
| "default_filament_profile" | Default filament profile | coStrings | "" | - | comAdvanced | Default filament profile when switching to this machine. | ❌ |
| "default_print_profile" | Default process profile | coString | "" | - | comAdvanced | Default print profile when switching to this machine. | ❌ |
| "upward_compatible_machine" | Upward compatible machine | coStrings | "" | - | comAdvanced | Machine models upward-compatible with this profile. | ❌ |

### Motion limits
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "machine_max_speed_x / y / z / e" | Maximum speed X/Y/Z/E | coFloats | 500/500/12/120 | mm/s | comSimple | Maximum axis speed (M203). | ❌ |
| "machine_max_acceleration_x / y / z / e" | Maximum acceleration X/Y/Z/E | coFloats | 1000/1000/500/5000 | mm/s² | comSimple | Maximum axis acceleration (M201). | ❌ |
| "machine_max_jerk_x / y / z / e" | Maximum jerk X/Y/Z/E | coFloats | 10/10/0.2/2.5 | mm/s | comSimple | Maximum axis jerk (M205). | ❌ |
| "machine_max_junction_deviation" | Maximum Junction Deviation | coFloats | 0.01 | mm | comAdvanced | Maximum junction deviation (M205 J). | ❌ |
| "machine_min_extruding_rate" | Min speed for extruding | coFloats | 0 | mm/s | comDevelop | Minimum speed for extruding (M205 S). | ❌ |
| "machine_min_travel_rate" | Min travel speed | coFloats | 0 | mm/s | comDevelop | Minimum travel speed (M205 T). | ❌ |
| "machine_max_acceleration_extruding" | Max acceleration extruding | coFloats | 1500 | mm/s² | comSimple | Maximum acceleration for extruding (M204 P). | ❌ |
| "machine_max_acceleration_retracting" | Max acceleration retracting | coFloats | 1500 | mm/s² | comSimple | Maximum acceleration for retracting (M204 R). | ❌ |
| "machine_max_acceleration_travel" | Max acceleration travel | coFloats | 0 | mm/s² | comAdvanced | Maximum acceleration for travel (M204 T; Marlin 2 only). | ❌ |

### Bed mesh
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "bed_mesh_min" | Bed mesh min | coPoint | (-99999,-99999) | mm | comAdvanced | Minimum point for allowed bed mesh area. | ❌ |
| "bed_mesh_max" | Bed mesh max | coPoint | (99999,99999) | mm | comAdvanced | Maximum point for allowed bed mesh area. | ❌ |
| "bed_mesh_probe_distance" | Probe point distance | coPoint | (50,50) | mm | comAdvanced | Preferred distance between probe points (grid size). | ❌ |
| "adaptive_bed_mesh_margin" | Mesh margin | coFloat | 0 | mm | comAdvanced | Additional expansion distance for adaptive bed mesh area. | ❌ |

### Resonance
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "resonance_avoidance" | Resonance avoidance | coBool | 0 | - | comAdvanced | Reduce outer wall speed to avoid printer resonance zone. | ❌ |
| "min_resonance_avoidance_speed" | Min speed (resonance) | coFloat | 70 | mm/s | comAdvanced | Minimum speed for resonance avoidance. | ❌ |
| "max_resonance_avoidance_speed" | Max speed (resonance) | coFloat | 120 | mm/s | comAdvanced | Maximum speed for resonance avoidance. | ❌ |

### Power / recovery
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "enable_power_loss_recovery" | Power Loss Recovery | coEnum | Printer configuration | printer_config/enable/disable | comAdvanced | Control power loss recovery G-code emission. | ❌ |
| "scan_first_layer" | Scan first layer | coBool | 0 | - | comAdvanced | Enable camera to check quality of first layer. | ❌ |
| "bbl_calib_mark_logo" | Show auto-calibration marks | coBool | 1 | - | comAdvanced | Show auto-calibration marks on print. | ❌ |
| "head_wrap_detect_zone" | Head wrap detect zone | coPoints | {} | - | comDevelop | Detection area for head wrap. | ❌ |
| "disable_m73" | Disable M73 progress | coBool | 0 | - | comAdvanced | Disable generating M73 set remaining print time. | ❌ |
| "silent_mode" | Supports silent mode | coBool | 0 | - | comDevelop | Whether machine supports silent mode with lower acceleration. | ❌ |
| "emit_machine_limits_to_gcode" | Emit limits to G-code | coBool | 1 | - | comAdvanced | Emit machine limits to G-code; ignored for Klipper. | ❌ |

### Timing
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "machine_tool_change_time" | Tool change time | coFloat | 0 | s | comAdvanced | Time taken to switch tools; for statistics. | ❌ |
| "machine_load_filament_time" | Filament load time | coFloat | 0 | s | comAdvanced | Time to load filament when switching; for statistics. | ❌ |
| "machine_unload_filament_time" | Filament unload time | coFloat | 0 | s | comAdvanced | Time to unload filament when switching; for statistics. | ❌ |
| "time_cost" | Printer cost per hour | coFloat | 0 | money/h | comAdvanced | Cost per hour of printing for statistics. | ❌ |
---


## SLA Printing

### Exposure
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "exposure_time" | Exposure time | coFloat | 10 | s | comSimple | Layer exposure time for SLA printing. | ❌ |
| "initial_exposure_time" | Initial exposure time | coFloat | 15 | s | comSimple | First layer exposure time for better adhesion. | ❌ |
| "max_exposure_time" | Max exposure time | coFloat | 100 | s | comAdvanced | Maximum exposure time limit. | ❌ |
| "min_exposure_time" | Min exposure time | coFloat | 0 | s | comAdvanced | Minimum exposure time limit. | ❌ |
| "max_initial_exposure_time" | Max initial exposure time | coFloat | 150 | s | comAdvanced | Maximum initial exposure time limit. | ❌ |
| "min_initial_exposure_time" | Min initial exposure time | coFloat | 0 | s | comAdvanced | Minimum initial exposure time limit. | ❌ |

### Display
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "display_width" | Display width | coFloat | 120 | mm | comAdvanced | LCD display width. | ❌ |
| "display_height" | Display height | coFloat | 68 | mm | comAdvanced | LCD display height. | ❌ |
| "display_pixels_x" | Display pixels X | coInt | 2560 | px | comAdvanced | Horizontal display resolution. | ❌ |
| "display_pixels_y" | Display pixels Y | coInt | 1440 | px | comAdvanced | Vertical display resolution. | ❌ |
| "display_orientation" | Display orientation | coEnum | portrait | landscape/portrait | comAdvanced | LCD display orientation. | ❌ |
| "display_mirror_x" | Mirror X | coBool | 1 | - | comAdvanced | Mirror display along X axis. | ❌ |
| "display_mirror_y" | Mirror Y | coBool | 0 | - | comAdvanced | Mirror display along Y axis. | ❌ |
| "fast_tilt_time" | Fast tilt time | coFloat | 5 | s | comAdvanced | Fast peel speed duration. | ❌ |
| "slow_tilt_time" | Slow tilt time | coFloat | 8 | s | comAdvanced | Slow peel speed duration. | ❌ |
| "elefant_foot_min_width" | Min elephant foot width | coFloat | 0.2 | mm | comAdvanced | SLA elephant foot minimum compensation width. | ❌ |

### Material (SLA)
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "material_type" | Material type | coString | Tough | - | comSimple | SLA resin type: Tough, Flexible, Casting, Dental, Heat-resistant. | ❌ |
| "material_vendor" | Material vendor | coString | "" | - | comAdvanced | SLA resin vendor. | ❌ |
| "material_colour" | Material colour | coString | #29B2B2 | - | comAdvanced | SLA resin colour. | ❌ |
| "material_density" | Material density | coFloat | 1.0 | g/cm³ | comAdvanced | SLA resin density. | ❌ |
| "material_print_speed" | Material print speed | coEnum | fast | slow/fast | comAdvanced | SLA material print speed mode. | ❌ |
| "material_correction" | Material correction | coFloats | (1,1,1) | - | comAdvanced | XYZ material correction factors. | ❌ |
| "material_correction_x" | Material correction X | coFloat | 1 | - | comAdvanced | X material correction factor. | ❌ |
| "material_correction_y" | Material correction Y | coFloat | 1 | - | comAdvanced | Y material correction factor. | ❌ |
| "material_correction_z" | Material correction Z | coFloat | 1 | - | comAdvanced | Z material correction factor. | ❌ |
| "relative_correction" | Relative correction | coFloats | (1,1) | - | comAdvanced | XY relative correction. | ❌ |
| "relative_correction_x" | Relative correction X | coFloat | 1 | - | comAdvanced | X relative correction factor. | ❌ |
| "relative_correction_y" | Relative correction Y | coFloat | 1 | - | comAdvanced | Y relative correction factor. | ❌ |
| "relative_correction_z" | Relative correction Z | coFloat | 1 | - | comAdvanced | Z relative correction factor. | ❌ |
| "absolute_correction" | Absolute correction | coFloat | 0 | - | comAdvanced | Absolute correction offset. | ❌ |
| "initial_layer_height" | Initial layer height (SLA) | coFloat | 0.3 | mm | comSimple | SLA first layer height. | ❌ |
| "bottle_volume" | Bottle volume | coFloat | 1000 | ml | comAdvanced | Resin bottle volume. | ❌ |
| "bottle_weight" | Bottle weight | coFloat | 1.0 | kg | comAdvanced | Resin bottle weight. | ❌ |
| "bottle_cost" | Bottle cost | coFloat | 0 | money | comAdvanced | Resin bottle cost. | ❌ |
| "gamma_correction" | Gamma correction | coFloat | 1.0 | - | comAdvanced | Display gamma correction value. | ❌ |

### Support (SLA)
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "supports_enable" | Enable supports | coBool | 1 | - | comSimple | Enable SLA support generation. | ❌ |
| "support_buildplate_only" | On build plate only | coBool | 0 | - | comSimple | Only generate supports from build plate, not on model. | ❌ |
| "support_critical_angle" | Critical angle | coFloat | 45 | ° | comAdvanced | Overhang angle threshold for support generation. | ❌ |
| "support_base_diameter" | Base diameter | coFloat | 4 | mm | comAdvanced | Support base diameter. | ❌ |
| "support_base_height" | Base height | coFloat | 1 | mm | comAdvanced | Support base height. | ❌ |
| "support_base_safety_distance" | Base safety distance | coFloat | 1 | mm | comAdvanced | Distance between support base and model. | ❌ |
| "support_head_front_diameter" | Head front diameter | coFloat | 0.4 | mm | comAdvanced | Support tip front diameter. | ❌ |
| "support_head_penetration" | Head penetration | coFloat | 0.2 | mm | comAdvanced | Depth of support head penetration into model. | ❌ |
| "support_head_width" | Head width | coFloat | 1.0 | mm | comAdvanced | Support head width. | ❌ |
| "support_max_bridge_length" | Max bridge length | coFloat | 15.0 | mm | comAdvanced | Maximum unsupported bridge length. | ❌ |
| "support_max_bridges_on_pillar" | Max bridges on pillar | coInt | 3 | - | comAdvanced | Maximum number of bridges on a single pillar. | ❌ |
| "support_max_pillar_link_distance" | Max pillar link distance | coFloat | 10 | mm | comAdvanced | Max distance for linking pillars; 0 = no linking. | ❌ |
| "support_object_elevation" | Object elevation | coFloat | 5 | mm | comAdvanced | Elevation of object above build plate. | ❌ |
| "support_pillar_connection_mode" | Pillar connection mode | coEnum | dynamic | zigzag/cross/dynamic | comAdvanced | Method for connecting pillars. | ❌ |
| "support_pillar_diameter" | Pillar diameter | coFloat | 1.0 | mm | comSimple | Diameter of support pillars. | ❌ |
| "support_pillar_widening_factor" | Pillar widening factor | coFloat | 0 | - | comAdvanced | Widening factor for pillar base. | ❌ |
| "support_points_density_relative" | Points density | coInt | 100 | - | comAdvanced | Relative density of support points. | ❌ |
| "support_points_minimal_distance" | Points minimal distance | coFloat | 1 | mm | comAdvanced | Minimum distance between support points. | ❌ |
| "support_small_pillar_diameter_percent" | Small pillar diameter % | coPercent | 50% | % | comAdvanced | Diameter of small pillars as percentage of normal. | ❌ |

### Hollowing (SLA)
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "hollowing_enable" | Enable hollowing | coBool | 0 | - | comSimple | Enable SLA model hollowing. | ❌ |
| "hollowing_closing_distance" | Closing distance | coFloat | 2.0 | mm | comAdvanced | Minimum distance to close a hole during hollowing. | ❌ |
| "hollowing_min_thickness" | Min thickness | coFloat | 3.0 | mm | comSimple | Minimum remaining wall thickness after hollowing. | ❌ |
| "hollowing_quality" | Quality | coFloat | 0.5 | - | comAdvanced | Hollowing detail quality level. | ❌ |

### Pad (SLA)
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "pad_enable" | Enable pad | coBool | 1 | - | comSimple | Enable SLA pad (base raft) generation. | ❌ |
| "pad_around_object" | Pad around object | coBool | 0 | - | comSimple | Generate pad around the object only. | ❌ |
| "pad_around_object_everywhere" | Pad everywhere | coBool | 0 | - | comSimple | Generate pad under the entire object. | ❌ |
| "pad_brim_size" | Brim size | coFloat | 1.6 | mm | comAdvanced | Pad brim dimension. | ❌ |
| "pad_max_merge_distance" | Max merge distance | coFloat | 50 | mm | comAdvanced | Maximum distance to merge separate pads. | ❌ |
| "pad_object_connector_penetration" | Connector penetration | coFloat | 0.3 | mm | comAdvanced | Depth of pad connector penetration into object. | ❌ |
| "pad_object_connector_stride" | Connector stride | coFloat | 10 | mm | comAdvanced | Spacing between pad connectors. | ❌ |
| "pad_object_connector_width" | Connector width | coFloat | 0.5 | mm | comAdvanced | Width of pad-object connector. | ❌ |
| "pad_object_gap" | Pad-object gap | coFloat | 1 | mm | comAdvanced | Gap between pad and object. | ❌ |
| "pad_wall_height" | Wall height | coFloat | 0 | mm | comAdvanced | Pad wall height. | ❌ |
| "pad_wall_slope" | Wall slope | coFloat | 90 | ° | comAdvanced | Pad wall slope angle. | ❌ |
| "pad_wall_thickness" | Wall thickness | coFloat | 2 | mm | comSimple | Pad wall thickness. | ❌ |

### Faded layers (SLA)
| Internal key | UI label | Type | Default | Units/Enum | Applies-to mode | Description | In Codebase |
|---|---|---|---|---|---|---|---|
| "faded_layers" | Faded layers | coInt | 10 | 3..20 | comAdvanced | Number of transition layers for exposure fading. | ❌ |
| "area_fill" | Area fill | coFloat | 50 | - | comAdvanced | Fill area for SLA structures. | ❌ |
---


## Cross-Reference Index (alphabetical)

| "absolute_correction" | SLA Printing | Material (SLA) |
| "accel_to_decel_enable" | Speed | Acceleration |
| "accel_to_decel_factor" | Speed | Acceleration |
| "activate_air_filtration" | Cooling | General |
| "activate_chamber_temp_control" | Cooling | General |
| "adaptive_bed_mesh_margin" | Printer/Machine | Bed mesh |
| "adaptive_pressure_advance" | Extruder/Nozzle | Pressure advance |
| "adaptive_pressure_advance_bridges" | Extruder/Nozzle | Pressure advance |
| "adaptive_pressure_advance_model" | Extruder/Nozzle | Pressure advance |
| "adaptive_pressure_advance_overhangs" | Extruder/Nozzle | Pressure advance |
| "additional_cooling_fan_speed" | Cooling | General |
| "align_infill_direction_to_model" | Strength | Advanced |
| "allow_mix_temp" | Printer/Machine | Printer identity |
| "allow_multicolor_oneplate" | Printer/Machine | Printer identity |
| "alternate_extra_wall" | Strength | Walls |
| "area_fill" | SLA Printing | Faded layers |
| "auxiliary_fan" | Cooling | General |
| "bbl_calib_mark_logo" | Printer/Machine | Power/recovery |
| "bbl_use_printhost" | Printer/Machine | Printer identity |
| "bed_custom_model" | Printer/Machine | Print volume |
| "bed_custom_texture" | Printer/Machine | Print volume |
| "bed_exclude_area" | Printer/Machine | Print volume |
| "bed_mesh_max" | Printer/Machine | Bed mesh |
| "bed_mesh_min" | Printer/Machine | Bed mesh |
| "bed_mesh_probe_distance" | Printer/Machine | Bed mesh |
| "bed_temperature_formula" | Filament | Bed temperature |
| "before_layer_change_gcode" | Output/G-code | G-code macros |
| "best_object_pos" | Printer/Machine | Print volume |
| "bottle_cost" | SLA Printing | Material (SLA) |
| "bottle_volume" | SLA Printing | Material (SLA) |
| "bottle_weight" | SLA Printing | Material (SLA) |
| "bottom_shell_layers" | Strength | Top/bottom shells |
| "bottom_shell_thickness" | Strength | Top/bottom shells |
| "bottom_solid_infill_flow_ratio" | Quality | Walls and surfaces |
| "bottom_surface_density" | Strength | Top/bottom shells |
| "bottom_surface_pattern" | Strength | Top/bottom shells |
| "bridge_acceleration" | Speed | Acceleration |
| "bridge_angle" | Quality | Bridging |
| "bridge_density" | Quality | Bridging |
| "bridge_flow" | Quality | Bridging |
| "bridge_no_support" | Support | Advanced |
| "bridge_speed" | Speed | Overhang speed |
| "brim_ears" | Others | Brim |
| "brim_ears_detection_length" | Others | Brim |
| "brim_ears_max_angle" | Others | Brim |
| "brim_object_gap" | Others | Brim |
| "brim_type" | Others | Brim |
| "brim_use_efc_outline" | Others | Brim |
| "brim_width" | Others | Brim |
| "calib_flowrate_topinfill_special_order" | Calibration | Flow/Pressure advance |
| "chamber_temperature" | Filament | Temperature |
| "change_extrusion_role_gcode" | Output/G-code | G-code macros |
| "change_filament_gcode" | Output/G-code | G-code macros |
| "close_fan_the_first_x_layers" | Cooling | General |
| "compatible_machine_expression_group" | Output/G-code | Metadata |
| "compatible_printers" | Output/G-code | Metadata |
| "compatible_printers_condition" | Output/G-code | Metadata |
| "compatible_prints" | Output/G-code | Metadata |
| "compatible_prints_condition" | Output/G-code | Metadata |
| "compatible_process_expression_group" | Output/G-code | Metadata |
| "complete_print_exhaust_fan_speed" | Cooling | General |
| "cool_plate_temp" | Filament | Bed temperature |
| "cool_plate_temp_initial_layer" | Filament | Bed temperature |
| "cooling_tube_length" | Extruder/Nozzle | MMU Hardware |
| "cooling_tube_retraction" | Extruder/Nozzle | MMU Hardware |
| "counterbore_hole_bridging" | Quality | Bridging |
| "curr_bed_type" | Filament | Bed temperature |
| "default_acceleration" | Speed | Acceleration |
| "default_bed_type" | Filament | Bed temperature |
| "default_filament_colour" | Filament | General |
| "default_filament_profile" | Printer/Machine | Printer identity |
| "default_jerk" | Speed | Jerk (XY) |
| "default_junction_deviation" | Speed | Jerk (XY) |
| "default_nozzle_volume_type" | Extruder/Nozzle | Nozzle |
| "default_print_profile" | Printer/Machine | Printer identity |
| "deretraction_speed" | Extruder/Nozzle | Retraction |
| "detect_narrow_internal_solid_infill" | Strength | Advanced |
| "detect_overhang_wall" | Quality | Overhangs |
| "detect_thin_wall" | Quality | Wall generator — Classic |
| "different_settings_to_system" | Output/G-code | Metadata |
| "disable_m73" | Printer/Machine | Power/recovery |
| "display_height" | SLA Printing | Display |
| "display_mirror_x" | SLA Printing | Display |
| "display_mirror_y" | SLA Printing | Display |
| "display_orientation" | SLA Printing | Display |
| "display_pixels_x" | SLA Printing | Display |
| "display_pixels_y" | SLA Printing | Display |
| "display_width" | SLA Printing | Display |
| "dont_filter_internal_bridges" | Quality | Bridging |
| "dont_slow_down_outer_wall" | Cooling | General |
| "draft_shield" | Others | Skirt |
| "during_print_exhaust_fan_speed" | Cooling | General |
| "elefant_foot_compensation" | Quality | Precision |
| "elefant_foot_compensation_layers" | Quality | Precision |
| "elefant_foot_min_width" | SLA Printing | Display |
| "emit_machine_limits_to_gcode" | Printer/Machine | Power/recovery |
| "enable_arc_fitting" | Quality | Precision |
| "enable_extra_bridge_layer" | Quality | Bridging |
| "enable_filament_ramming" | Multimaterial | Prime tower |
| "enable_overhang_bridge_fan" | Cooling | General |
| "enable_overhang_speed" | Speed | Overhang speed |
| "enable_power_loss_recovery" | Printer/Machine | Power/recovery |
| "enable_pressure_advance" | Extruder/Nozzle | Pressure advance |
| "enable_prime_tower" | Multimaterial | Prime tower |
| "enable_support" | Support | Support |
| "enable_timelapse" | Others | Special mode |
| "enable_tower_interface_cooldown_during_tower" | Multimaterial | Prime tower |
| "enable_tower_interface_features" | Multimaterial | Prime tower |
| "enable_wrapping_detection" | Others | Special mode |
| "enforce_support_layers" | Support | Support |
| "eng_plate_temp" | Filament | Bed temperature |
| "eng_plate_temp_initial_layer" | Filament | Bed temperature |
| "ensure_vertical_shell_thickness" | Strength | Advanced |
| "exclude_object" | Others | G-code output |
| "exposure_time" | SLA Printing | Exposure |
| "extra_loading_move" | Extruder/Nozzle | MMU Hardware |
| "extra_perimeters_on_overhangs" | Quality | Overhangs |
| "extra_solid_infills" | Strength | Advanced |
| "extruder" | Quality | Walls and surfaces |
| "extruder_ams_count" | Extruder/Nozzle | Extruder geometry |
| "extruder_clearance_height_to_lid" | Printer/Machine | Print volume |
| "extruder_clearance_height_to_rod" | Printer/Machine | Print volume |
| "extruder_clearance_radius" | Printer/Machine | Print volume |
| "extruder_colour" | Extruder/Nozzle | Extruder geometry |
| "extruder_offset" | Extruder/Nozzle | Extruder geometry |
| "extruder_printable_area" | Printer/Machine | Print volume |
| "extruder_printable_height" | Printer/Machine | Print volume |
| "extruder_type" | Extruder/Nozzle | Extruder geometry |
| "extruder_variant_list" | Extruder/Nozzle | Extruder geometry |
| "extrusion_rate_smoothing_external_perimeter_only" | Speed | Advanced |
| "faded_layers" | SLA Printing | Faded layers |
| "fan_cooling_layer_time" | Cooling | General |
| "fan_kickstart" | Cooling | General |
| "fan_max_speed" | Cooling | General |
| "fan_min_speed" | Cooling | General |
| "fan_speedup_overhangs" | Cooling | General |
| "fan_speedup_time" | Cooling | General |
| "fast_tilt_time" | SLA Printing | Display |
| "filament_adaptive_volumetric_speed" | Filament | General |
| "filament_adhesiveness_category" | Filament | General |
| "filament_change_length" | Filament | General |
| "filament_colour" | Filament | General |
| "filament_colour_type" | Filament | General |
| "filament_cooling_final_speed" | Filament | General |
| "filament_cooling_initial_speed" | Filament | General |
| "filament_cooling_moves" | Filament | General |
| "filament_cost" | Filament | General |
| "filament_density" | Filament | General |
| "filament_diameter" | Filament | General |
| "filament_end_gcode" | Output/G-code | G-code macros |
| "filament_extruder_variant" | Extruder/Nozzle | Extruder geometry |
| "filament_flow_ratio" | Filament | General |
| "filament_flush_temp" | Multimaterial | Flush options |
| "filament_flush_volumetric_speed" | Multimaterial | Flush options |
| "filament_ids" | Filament | General |
| "filament_ironing_flow" | Filament | General |
| "filament_ironing_inset" | Filament | General |
| "filament_ironing_spacing" | Filament | General |
| "filament_ironing_speed" | Filament | General |
| "filament_is_support" | Filament | General |
| "filament_loading_speed" | Filament | General |
| "filament_loading_speed_start" | Filament | General |
| "filament_map" | Multimaterial | Filament for Features |
| "filament_map_mode" | Multimaterial | Filament for Features |
| "filament_max_volumetric_speed" | Filament | General |
| "filament_minimal_purge_on_wipe_tower" | Filament | General |
| "filament_multi_colour" | Filament | General |
| "filament_multitool_ramming" | Filament | General |
| "filament_multitool_ramming_flow" | Filament | General |
| "filament_multitool_ramming_volume" | Filament | General |
| "filament_notes" | Output/G-code | Metadata |
| "filament_printable" | Filament | General |
| "filament_ramming_parameters" | Filament | General |
| "filament_self_index" | Extruder/Nozzle | Extruder geometry |
| "filament_settings_id" | Filament | General |
| "filament_shrink" | Filament | General |
| "filament_shrinkage_compensation_z" | Filament | General |
| "filament_soluble" | Filament | General |
| "filament_stamping_distance" | Filament | General |
| "filament_stamping_loading_speed" | Filament | General |
| "filament_start_gcode" | Output/G-code | G-code macros |
| "filament_toolchange_delay" | Filament | General |
| "filament_tower_interface_pre_extrusion_dist" | Multimaterial | Prime tower |
| "filament_tower_interface_pre_extrusion_length" | Multimaterial | Prime tower |
| "filament_tower_interface_print_temp" | Multimaterial | Prime tower |
| "filament_tower_interface_purge_volume" | Multimaterial | Prime tower |
| "filament_tower_ironing_area" | Multimaterial | Prime tower |
| "filament_type" | Filament | General |
| "filament_unloading_speed" | Filament | General |
| "filament_unloading_speed_start" | Filament | General |
| "filament_vendor" | Filament | General |
| "file_start_gcode" | Output/G-code | G-code macros |
| "filename_format" | Others | G-code output |
| "fill_multiline" | Strength | Infill |
| "filter_out_gap_fill" | Strength | Infill |
| "first_layer_flow_ratio" | Quality | Walls and surfaces |
| "first_layer_print_sequence" | Quality | Layer height |
| "first_layer_sequence_choice" | Quality | Layer height |
| "flush_into_infill" | Multimaterial | Flush options |
| "flush_into_objects" | Multimaterial | Flush options |
| "flush_into_support" | Multimaterial | Flush options |
| "flush_multiplier" | Multimaterial | Flush options |
| "flush_volumes_matrix" | Multimaterial | Flush options |
| "flush_volumes_vector" | Multimaterial | Flush options |
| "full_fan_speed_layer" | Cooling | General |
| "fuzzy_skin" | Others | Fuzzy Skin |
| "fuzzy_skin_first_layer" | Others | Fuzzy Skin |
| "fuzzy_skin_mode" | Others | Fuzzy Skin |
| "fuzzy_skin_noise_type" | Others | Fuzzy Skin |
| "fuzzy_skin_octaves" | Others | Fuzzy Skin |
| "fuzzy_skin_persistence" | Others | Fuzzy Skin |
| "fuzzy_skin_point_distance" | Others | Fuzzy Skin |
| "fuzzy_skin_scale" | Others | Fuzzy Skin |
| "fuzzy_skin_thickness" | Others | Fuzzy Skin |
| "gamma_correction" | SLA Printing | Material (SLA) |
| "gap_fill_flow_ratio" | Quality | Walls and surfaces |
| "gap_fill_target" | Strength | Infill |
| "gap_infill_speed" | Speed | Other layers speed |
| "gcode_add_line_number" | Others | G-code output |
| "gcode_comments" | Others | G-code output |
| "gcode_flavor" | Printer/Machine | Printer identity |
| "gcode_label_objects" | Others | G-code output |
| "grab_length" | Extruder/Nozzle | MMU Hardware |
| "has_scarf_joint_seam" | Quality | Seam |
| "head_wrap_detect_zone" | Printer/Machine | Power/recovery |
| "high_current_on_filament_swap" | Extruder/Nozzle | MMU Hardware |
| "hole_to_polyhole" | Quality | Precision |
| "hole_to_polyhole_threshold" | Quality | Precision |
| "hole_to_polyhole_twisted" | Quality | Precision |
| "hollowing_closing_distance" | SLA Printing | Hollowing |
| "hollowing_enable" | SLA Printing | Hollowing |
| "hollowing_min_thickness" | SLA Printing | Hollowing |
| "hollowing_quality" | SLA Printing | Hollowing |
| "host_type" | Printer/Machine | Printer identity |
| "hot_plate_temp" | Filament | Bed temperature |
| "hot_plate_temp_initial_layer" | Filament | Bed temperature |
| "idle_temperature" | Filament | Temperature |
| "independent_support_layer_height" | Support | Advanced |
| "infill_anchor" | Strength | Infill |
| "infill_anchor_max" | Strength | Infill |
| "infill_combination" | Strength | Advanced |
| "infill_combination_max_layer_height" | Strength | Advanced |
| "infill_direction" | Strength | Infill |
| "infill_jerk" | Speed | Jerk (XY) |
| "infill_lock_depth" | Strength | Infill pattern-specific |
| "infill_overhang_angle" | Strength | Infill pattern-specific |
| "infill_shift_step" | Strength | Infill pattern-specific |
| "infill_wall_overlap" | Strength | Infill |
| "inherits" | Output/G-code | Metadata |
| "inherits_group" | Output/G-code | Metadata |
| "initial_exposure_time" | SLA Printing | Exposure |
| "initial_layer_acceleration" | Speed | Acceleration |
| "initial_layer_height" | SLA Printing | Material (SLA) |
| "initial_layer_infill_speed" | Speed | Initial layer speed |
| "initial_layer_jerk" | Speed | Jerk (XY) |
| "initial_layer_line_width" | Quality | Line width |
| "initial_layer_min_bead_width" | Quality | Wall generator |
| "initial_layer_print_height" | Quality | Layer height |
| "initial_layer_speed" | Speed | Initial layer speed |
| "initial_layer_travel_speed" | Speed | Initial layer speed |
| "inner_wall_acceleration" | Speed | Acceleration |
| "inner_wall_flow_ratio" | Quality | Walls and surfaces |
| "inner_wall_jerk" | Speed | Jerk (XY) |
| "inner_wall_line_width" | Quality | Line width |
| "inner_wall_speed" | Speed | Other layers speed |
| "interface_shells" | Multimaterial | Advanced |
| "interlocking_beam" | Multimaterial | Advanced |
| "interlocking_beam_layer_count" | Multimaterial | Advanced |
| "interlocking_beam_width" | Multimaterial | Advanced |
| "interlocking_boundary_avoidance" | Multimaterial | Advanced |
| "interlocking_depth" | Multimaterial | Advanced |
| "interlocking_orientation" | Multimaterial | Advanced |
| "internal_bridge_angle" | Quality | Bridging |
| "internal_bridge_density" | Quality | Bridging |
| "internal_bridge_fan_speed" | Cooling | General |
| "internal_bridge_flow" | Quality | Bridging |
| "internal_bridge_speed" | Speed | Overhang speed |
| "internal_solid_infill_acceleration" | Speed | Acceleration |
| "internal_solid_infill_flow_ratio" | Quality | Walls and surfaces |
| "internal_solid_infill_line_width" | Quality | Line width |
| "internal_solid_infill_pattern" | Strength | Infill |
| "internal_solid_infill_speed" | Speed | Other layers speed |
| "ironing_angle" | Quality | Ironing |
| "ironing_angle_fixed" | Quality | Ironing |
| "ironing_fan_speed" | Cooling | General |
| "ironing_flow" | Quality | Ironing |
| "ironing_inset" | Quality | Ironing |
| "ironing_pattern" | Quality | Ironing |
| "ironing_spacing" | Quality | Ironing |
| "ironing_speed" | Speed | Other layers speed |
| "ironing_type" | Quality | Ironing |
| "is_infill_first" | Quality | Walls and surfaces |
| "lateral_lattice_angle_1" | Strength | Infill pattern-specific |
| "lateral_lattice_angle_2" | Strength | Infill pattern-specific |
| "layer_change_gcode" | Output/G-code | G-code macros |
| "layer_height" | Quality | Layer height |
| "line_width" | Quality | Line width |
| "long_retractions_when_cut" | Extruder/Nozzle | Retraction |
| "long_retractions_when_ec" | Extruder/Nozzle | Retraction |
| "machine_end_gcode" | Output/G-code | G-code macros |
| "machine_load_filament_time" | Printer/Machine | Timing |
| "machine_max_acceleration_e" | Printer/Machine | Motion limits |
| "machine_max_acceleration_extruding" | Printer/Machine | Motion limits |
| "machine_max_acceleration_retracting" | Printer/Machine | Motion limits |
| "machine_max_acceleration_travel" | Printer/Machine | Motion limits |
| "machine_max_acceleration_x" | Printer/Machine | Motion limits |
| "machine_max_acceleration_y" | Printer/Machine | Motion limits |
| "machine_max_acceleration_z" | Printer/Machine | Motion limits |
| "machine_max_jerk_e" | Printer/Machine | Motion limits |
| "machine_max_jerk_x" | Printer/Machine | Motion limits |
| "machine_max_jerk_y" | Printer/Machine | Motion limits |
| "machine_max_jerk_z" | Printer/Machine | Motion limits |
| "machine_max_junction_deviation" | Printer/Machine | Motion limits |
| "machine_max_speed_e" | Printer/Machine | Motion limits |
| "machine_max_speed_x" | Printer/Machine | Motion limits |
| "machine_max_speed_y" | Printer/Machine | Motion limits |
| "machine_max_speed_z" | Printer/Machine | Motion limits |
| "machine_min_extruding_rate" | Printer/Machine | Motion limits |
| "machine_min_travel_rate" | Printer/Machine | Motion limits |
| "machine_pause_gcode" | Output/G-code | G-code macros |
| "machine_start_gcode" | Output/G-code | G-code macros |
| "machine_tool_change_time" | Printer/Machine | Timing |
| "machine_unload_filament_time" | Printer/Machine | Timing |
| "make_overhang_printable" | Quality | Overhangs |
| "make_overhang_printable_angle" | Quality | Overhangs |
| "make_overhang_printable_hole_size" | Quality | Overhangs |
| "manual_filament_change" | Multimaterial | Prime tower |
| "master_extruder_id" | Extruder/Nozzle | Extruder geometry |
| "material_colour" | SLA Printing | Material (SLA) |
| "material_correction" | SLA Printing | Material (SLA) |
| "material_correction_x" | SLA Printing | Material (SLA) |
| "material_correction_y" | SLA Printing | Material (SLA) |
| "material_correction_z" | SLA Printing | Material (SLA) |
| "material_density" | SLA Printing | Material (SLA) |
| "material_print_speed" | SLA Printing | Material (SLA) |
| "material_type" | SLA Printing | Material (SLA) |
| "material_vendor" | SLA Printing | Material (SLA) |
| "max_bridge_length" | Support | Advanced |
| "max_exposure_time" | SLA Printing | Exposure |
| "max_initial_exposure_time" | SLA Printing | Exposure |
| "max_layer_height" | Cooling | General |
| "max_resonance_avoidance_speed" | Printer/Machine | Resonance |
| "max_travel_detour_distance" | Quality | Walls and surfaces |
| "max_volumetric_extrusion_rate_slope" | Speed | Advanced |
| "max_volumetric_extrusion_rate_slope_segment_length" | Speed | Advanced |
| "min_bead_width" | Quality | Wall generator |
| "min_exposure_time" | SLA Printing | Exposure |
| "min_feature_size" | Quality | Wall generator |
| "min_initial_exposure_time" | SLA Printing | Exposure |
| "min_layer_height" | Cooling | General |
| "min_length_factor" | Quality | Wall generator |
| "min_resonance_avoidance_speed" | Printer/Machine | Resonance |
| "min_skirt_length" | Others | Skirt |
| "min_width_top_surface" | Quality | Walls and surfaces |
| "minimum_sparse_infill_area" | Strength | Advanced |
| "mmu_segmented_region_interlocking_depth" | Multimaterial | Advanced |
| "mmu_segmented_region_max_width" | Multimaterial | Advanced |
| "notes" | Output/G-code | Metadata |
| "nozzle_diameter" | Extruder/Nozzle | Nozzle |
| "nozzle_flush_dataset" | Multimaterial | Flush options |
| "nozzle_height" | Extruder/Nozzle | Nozzle |
| "nozzle_hrc" | Extruder/Nozzle | Nozzle |
| "nozzle_temperature" | Filament | Temperature |
| "nozzle_temperature_initial_layer" | Filament | Temperature |
| "nozzle_temperature_range_high" | Filament | Temperature |
| "nozzle_temperature_range_low" | Filament | Temperature |
| "nozzle_type" | Extruder/Nozzle | Nozzle |
| "nozzle_volume" | Extruder/Nozzle | Nozzle |
| "nozzle_volume_type" | Extruder/Nozzle | Nozzle |
| "only_one_wall_first_layer" | Quality | Walls and surfaces |
| "only_one_wall_top" | Quality | Walls and surfaces |
| "ooze_prevention" | Multimaterial | Ooze prevention |
| "other_layers_print_sequence" | Quality | Layer height |
| "other_layers_print_sequence_nums" | Quality | Layer height |
| "other_layers_sequence_choice" | Quality | Layer height |
| "outer_wall_acceleration" | Speed | Acceleration |
| "outer_wall_flow_ratio" | Quality | Walls and surfaces |
| "outer_wall_jerk" | Speed | Jerk (XY) |
| "outer_wall_line_width" | Quality | Line width |
| "outer_wall_speed" | Speed | Other layers speed |
| "overhang_1_4_speed" | Speed | Overhang speed |
| "overhang_2_4_speed" | Speed | Overhang speed |
| "overhang_3_4_speed" | Speed | Overhang speed |
| "overhang_4_4_speed" | Speed | Overhang speed |
| "overhang_fan_speed" | Cooling | General |
| "overhang_fan_threshold" | Cooling | General |
| "overhang_flow_ratio" | Quality | Walls and surfaces |
| "overhang_reverse" | Quality | Overhangs |
| "overhang_reverse_internal_only" | Quality | Overhangs |
| "overhang_reverse_threshold" | Quality | Overhangs |
| "pad_around_object" | SLA Printing | Pad |
| "pad_around_object_everywhere" | SLA Printing | Pad |
| "pad_brim_size" | SLA Printing | Pad |
| "pad_enable" | SLA Printing | Pad |
| "pad_max_merge_distance" | SLA Printing | Pad |
| "pad_object_connector_penetration" | SLA Printing | Pad |
| "pad_object_connector_stride" | SLA Printing | Pad |
| "pad_object_connector_width" | SLA Printing | Pad |
| "pad_object_gap" | SLA Printing | Pad |
| "pad_wall_height" | SLA Printing | Pad |
| "pad_wall_slope" | SLA Printing | Pad |
| "pad_wall_thickness" | SLA Printing | Pad |
| "parking_pos_retraction" | Extruder/Nozzle | MMU Hardware |
| "pellet_flow_coefficient" | Filament | General |
| "pellet_modded_printer" | Printer/Machine | Printer identity |
| "physical_extruder_map" | Extruder/Nozzle | Extruder geometry |
| "post_process" | Others | Post-processing |
| "precise_outer_wall" | Quality | Precision |
| "precise_z_height" | Quality | Precision |
| "preferred_orientation" | Printer/Machine | Print volume |
| "preheat_steps" | Multimaterial | Ooze prevention |
| "preheat_time" | Multimaterial | Ooze prevention |
| "pressure_advance" | Extruder/Nozzle | Pressure advance |
| "prime_tower_brim_width" | Multimaterial | Prime tower |
| "prime_tower_enable_framework" | Multimaterial | Prime tower |
| "prime_tower_flat_ironing" | Multimaterial | Prime tower |
| "prime_tower_infill_gap" | Multimaterial | Prime tower |
| "prime_tower_skip_points" | Multimaterial | Prime tower |
| "prime_tower_width" | Multimaterial | Prime tower |
| "prime_volume" | Multimaterial | Prime tower |
| "print_compatible_printers" | Output/G-code | Metadata |
| "print_extruder_id" | Extruder/Nozzle | Extruder geometry |
| "print_extruder_variant" | Extruder/Nozzle | Extruder geometry |
| "print_flow_ratio" | Quality | Walls and surfaces |
| "print_host" | Printer/Machine | Printer identity |
| "print_host_webui" | Printer/Machine | Printer identity |
| "print_order" | Others | Special mode |
| "print_sequence" | Others | Special mode |
| "print_settings_id" | Output/G-code | Metadata |
| "printable_area" | Printer/Machine | Print volume |
| "printable_height" | Printer/Machine | Print volume |
| "printer_agent" | Printer/Machine | Printer identity |
| "printer_extruder_id" | Extruder/Nozzle | Extruder geometry |
| "printer_extruder_variant" | Extruder/Nozzle | Extruder geometry |
| "printer_model" | Printer/Machine | Printer identity |
| "printer_notes" | Output/G-code | Metadata |
| "printer_settings_id" | Printer/Machine | Printer identity |
| "printer_structure" | Printer/Machine | Printer identity |
| "printer_technology" | Printer/Machine | Printer identity |
| "printer_variant" | Printer/Machine | Printer identity |
| "printhost_apikey" | Printer/Machine | Printer identity |
| "printhost_authorization_type" | Printer/Machine | Printer identity |
| "printhost_cafile" | Printer/Machine | Printer identity |
| "printhost_password" | Printer/Machine | Printer identity |
| "printhost_port" | Printer/Machine | Printer identity |
| "printhost_ssl_ignore_revoke" | Printer/Machine | Printer identity |
| "printhost_user" | Printer/Machine | Printer identity |
| "printing_by_object_gcode" | Output/G-code | G-code macros |
| "purge_in_prime_tower" | Multimaterial | Prime tower |
| "raft_contact_distance" | Support | Raft |
| "raft_expansion" | Support | Raft |
| "raft_first_layer_density" | Support | Support |
| "raft_first_layer_expansion" | Support | Support |
| "raft_layers" | Support | Raft |
| "reduce_crossing_wall" | Quality | Walls and surfaces |
| "reduce_fan_stop_start_freq" | Cooling | General |
| "reduce_infill_retraction" | Others | G-code output |
| "relative_correction" | SLA Printing | Material (SLA) |
| "relative_correction_x" | SLA Printing | Material (SLA) |
| "relative_correction_y" | SLA Printing | Material (SLA) |
| "relative_correction_z" | SLA Printing | Material (SLA) |
| "required_nozzle_HRC" | Extruder/Nozzle | Nozzle |
| "resolution" | Quality | Precision |
| "resonance_avoidance" | Printer/Machine | Resonance |
| "retract_before_wipe" | Extruder/Nozzle | Retraction |
| "retract_length_toolchange" | Extruder/Nozzle | Retraction |
| "retract_lift_above" | Extruder/Nozzle | Retraction |
| "retract_lift_below" | Extruder/Nozzle | Retraction |
| "retract_lift_enforce" | Extruder/Nozzle | Retraction |
| "retract_restart_extra" | Extruder/Nozzle | Retraction |
| "retract_restart_extra_toolchange" | Extruder/Nozzle | Retraction |
| "retract_when_changing_layer" | Extruder/Nozzle | Retraction |
| "retraction_distances_when_cut" | Extruder/Nozzle | Retraction |
| "retraction_distances_when_ec" | Extruder/Nozzle | Retraction |
| "retraction_length" | Extruder/Nozzle | Retraction |
| "retraction_minimum_travel" | Extruder/Nozzle | Retraction |
| "retraction_speed" | Extruder/Nozzle | Retraction |
| "role_based_wipe_speed" | Quality | Seam |
| "scan_first_layer" | Printer/Machine | Power/recovery |
| "scarf_angle_threshold" | Quality | Seam |
| "scarf_joint_flow_ratio" | Quality | Seam |
| "scarf_joint_speed" | Quality | Seam |
| "scarf_overhang_threshold" | Quality | Seam |
| "seam_gap" | Quality | Seam |
| "seam_position" | Quality | Seam |
| "seam_slope_conditional" | Quality | Seam |
| "seam_slope_entire_loop" | Quality | Seam |
| "seam_slope_inner_walls" | Quality | Seam |
| "seam_slope_min_length" | Quality | Seam |
| "seam_slope_start_height" | Quality | Seam |
| "seam_slope_steps" | Quality | Seam |
| "seam_slope_type" | Quality | Seam |
| "set_other_flow_ratios" | Quality | Walls and surfaces |
| "silent_mode" | Printer/Machine | Power/recovery |
| "single_extruder_multi_material" | Multimaterial | Prime tower |
| "single_extruder_multi_material_priming" | Multimaterial | Prime tower |
| "single_loop_draft_shield" | Others | Skirt |
| "skeleton_infill_density" | Strength | Infill pattern-specific |
| "skeleton_infill_line_width" | Strength | Infill pattern-specific |
| "skin_infill_density" | Strength | Infill pattern-specific |
| "skin_infill_depth" | Strength | Infill pattern-specific |
| "skin_infill_line_width" | Strength | Infill pattern-specific |
| "skirt_distance" | Others | Skirt |
| "skirt_height" | Others | Skirt |
| "skirt_loops" | Others | Skirt |
| "skirt_speed" | Others | Skirt |
| "skirt_start_angle" | Others | Skirt |
| "skirt_type" | Others | Skirt |
| "slice_closing_radius" | Quality | Precision |
| "slicing_mode" | Others | Special mode |
| "slow_down_for_layer_cooling" | Cooling | General |
| "slow_down_layer_time" | Cooling | General |
| "slow_down_layers" | Speed | Initial layer speed |
| "slow_down_min_speed" | Cooling | General |
| "slow_tilt_time" | SLA Printing | Display |
| "slowdown_for_curled_perimeters" | Speed | Overhang speed |
| "small_area_infill_flow_compensation" | Quality | Walls and surfaces |
| "small_area_infill_flow_compensation_model" | Quality | Walls and surfaces |
| "small_perimeter_speed" | Speed | Other layers speed |
| "small_perimeter_threshold" | Speed | Other layers speed |
| "solid_infill_direction" | Strength | Infill |
| "solid_infill_filament" | Multimaterial | Filament for Features |
| "solid_infill_rotate_template" | Strength | Infill |
| "sparse_infill_acceleration" | Speed | Acceleration |
| "sparse_infill_density" | Strength | Infill |
| "sparse_infill_filament" | Multimaterial | Filament for Features |
| "sparse_infill_flow_ratio" | Quality | Walls and surfaces |
| "sparse_infill_line_width" | Quality | Line width |
| "sparse_infill_pattern" | Strength | Infill |
| "sparse_infill_rotate_template" | Strength | Infill |
| "sparse_infill_speed" | Speed | Other layers speed |
| "spiral_finishing_flow_ratio" | Others | Special mode |
| "spiral_mode" | Others | Special mode |
| "spiral_mode_max_xy_smoothing" | Others | Special mode |
| "spiral_mode_smooth" | Others | Special mode |
| "spiral_starting_flow_ratio" | Others | Special mode |
| "staggered_inner_seams" | Quality | Seam |
| "standby_temperature_delta" | Multimaterial | Ooze prevention |
| "start_end_points" | Extruder/Nozzle | MMU Hardware |
| "supertack_plate_temp" | Filament | Bed temperature |
| "supertack_plate_temp_initial_layer" | Filament | Bed temperature |
| "support_air_filtration" | Support | Support |
| "support_angle" | Support | Support |
| "support_base_diameter" | SLA Printing | Support (SLA) |
| "support_base_height" | SLA Printing | Support (SLA) |
| "support_base_pattern" | Support | Advanced |
| "support_base_pattern_spacing" | Support | Advanced |
| "support_base_safety_distance" | SLA Printing | Support (SLA) |
| "support_bottom_interface_spacing" | Support | Advanced |
| "support_bottom_z_distance" | Support | Support |
| "support_buildplate_only" | SLA Printing | Support (SLA) |
| "support_chamber_temp_control" | Filament | Bed temperature |
| "support_critical_angle" | SLA Printing | Support (SLA) |
| "support_critical_regions_only" | Support | Support |
| "support_expansion" | Support | Support |
| "support_filament" | Support | Support filament |
| "support_flow_ratio" | Quality | Walls and surfaces |
| "support_head_front_diameter" | SLA Printing | Support (SLA) |
| "support_head_penetration" | SLA Printing | Support (SLA) |
| "support_head_width" | SLA Printing | Support (SLA) |
| "support_interface_bottom_layers" | Support | Advanced |
| "support_interface_filament" | Support | Support filament |
| "support_interface_flow_ratio" | Quality | Walls and surfaces |
| "support_interface_loop_pattern" | Support | Advanced |
| "support_interface_not_for_body" | Support | Support filament |
| "support_interface_pattern" | Support | Advanced |
| "support_interface_spacing" | Support | Advanced |
| "support_interface_speed" | Support | Advanced |
| "support_interface_top_layers" | Support | Advanced |
| "support_ironing" | Support | Support ironing |
| "support_ironing_flow" | Support | Support ironing |
| "support_ironing_pattern" | Support | Support ironing |
| "support_ironing_spacing" | Support | Support ironing |
| "support_line_width" | Quality | Line width |
| "support_material_interface_fan_speed" | Cooling | General |
| "support_max_bridge_length" | SLA Printing | Support (SLA) |
| "support_max_bridges_on_pillar" | SLA Printing | Support (SLA) |
| "support_max_pillar_link_distance" | SLA Printing | Support (SLA) |
| "support_multi_bed_types" | Filament | Bed temperature |
| "support_object_elevation" | SLA Printing | Support (SLA) |
| "support_object_first_layer_gap" | Support | Support |
| "support_object_skip_flush" | Multimaterial | Advanced |
| "support_object_xy_distance" | Support | Support |
| "support_on_build_plate_only" | Support | Support |
| "support_pillar_connection_mode" | SLA Printing | Support (SLA) |
| "support_pillar_diameter" | SLA Printing | Support (SLA) |
| "support_pillar_widening_factor" | SLA Printing | Support (SLA) |
| "support_points_density_relative" | SLA Printing | Support (SLA) |
| "support_points_minimal_distance" | SLA Printing | Support (SLA) |
| "support_remove_small_overhang" | Support | Support |
| "support_small_pillar_diameter_percent" | SLA Printing | Support (SLA) |
| "support_speed" | Speed | Other layers speed |
| "support_style" | Support | Support |
| "support_threshold_angle" | Support | Support |
| "support_threshold_overlap" | Support | Support |
| "support_top_z_distance" | Support | Support |
| "support_type" | Support | Support |
| "supports_enable" | SLA Printing | Support (SLA) |
| "symmetric_infill_y_axis" | Strength | Infill pattern-specific |
| "temperature_vitrification" | Filament | General |
| "template_custom_gcode" | Output/G-code | G-code macros |
| "textured_cool_plate_temp" | Filament | Bed temperature |
| "textured_cool_plate_temp_initial_layer" | Filament | Bed temperature |
| "textured_plate_temp" | Filament | Bed temperature |
| "textured_plate_temp_initial_layer" | Filament | Bed temperature |
| "thick_bridges" | Quality | Bridging |
| "thick_internal_bridges" | Quality | Bridging |
| "thumbnails" | Output/G-code | File output |
| "thumbnails_format" | Output/G-code | File output |
| "time_cost" | Printer/Machine | Timing |
| "time_lapse_gcode" | Output/G-code | G-code macros |
| "timelapse_type" | Others | Special mode |
| "top_bottom_infill_wall_overlap" | Strength | Top/bottom shells |
| "top_shell_layers" | Strength | Top/bottom shells |
| "top_shell_thickness" | Strength | Top/bottom shells |
| "top_solid_infill_flow_ratio" | Quality | Walls and surfaces |
| "top_surface_acceleration" | Speed | Acceleration |
| "top_surface_density" | Strength | Top/bottom shells |
| "top_surface_jerk" | Speed | Jerk (XY) |
| "top_surface_line_width" | Quality | Line width |
| "top_surface_pattern" | Strength | Top/bottom shells |
| "top_surface_speed" | Speed | Other layers speed |
| "travel_acceleration" | Speed | Acceleration |
| "travel_jerk" | Speed | Jerk (XY) |
| "travel_slope" | Extruder/Nozzle | Retraction |
| "travel_speed" | Speed | Travel speed |
| "travel_speed_z" | Speed | Travel speed |
| "tree_support_angle_slow" | Support | Tree supports |
| "tree_support_auto_brim" | Support | Tree supports |
| "tree_support_branch_angle" | Support | Tree supports |
| "tree_support_branch_angle_organic" | Support | Tree supports |
| "tree_support_branch_diameter" | Support | Tree supports |
| "tree_support_branch_diameter_angle" | Support | Tree supports |
| "tree_support_branch_diameter_organic" | Support | Tree supports |
| "tree_support_branch_distance" | Support | Tree supports |
| "tree_support_branch_distance_organic" | Support | Tree supports |
| "tree_support_brim_width" | Support | Tree supports |
| "tree_support_tip_diameter" | Support | Tree supports |
| "tree_support_top_rate" | Support | Tree supports |
| "tree_support_wall_count" | Support | Advanced |
| "tree_support_with_infill" | Support | Tree supports |
| "upward_compatible_machine" | Printer/Machine | Printer identity |
| "use_firmware_retraction" | Extruder/Nozzle | Retraction |
| "use_relative_e_distances" | Others | G-code output |
| "volumetric_speed_coefficients" | Filament | General |
| "wall_direction" | Quality | Walls and surfaces |
| "wall_distribution_count" | Quality | Wall generator |
| "wall_filament" | Multimaterial | Filament for Features |
| "wall_generator" | Quality | Wall generator |
| "wall_loops" | Strength | Walls |
| "wall_sequence" | Quality | Walls and surfaces |
| "wall_transition_angle" | Quality | Wall generator |
| "wall_transition_filter_deviation" | Quality | Wall generator |
| "wall_transition_length" | Quality | Wall generator |
| "wipe" | Extruder/Nozzle | Retraction |
| "wipe_before_external_loop" | Quality | Seam |
| "wipe_distance" | Extruder/Nozzle | Retraction |
| "wipe_on_loops" | Quality | Seam |
| "wipe_speed" | Quality | Seam |
| "wipe_tower_bridging" | Multimaterial | Prime tower |
| "wipe_tower_cone_angle" | Multimaterial | Prime tower |
| "wipe_tower_extra_flow" | Multimaterial | Prime tower |
| "wipe_tower_extra_rib_length" | Multimaterial | Prime tower |
| "wipe_tower_extra_spacing" | Multimaterial | Prime tower |
| "wipe_tower_filament" | Multimaterial | Filament for Features |
| "wipe_tower_fillet_wall" | Multimaterial | Prime tower |
| "wipe_tower_max_purge_speed" | Multimaterial | Prime tower |
| "wipe_tower_no_sparse_layers" | Multimaterial | Prime tower |
| "wipe_tower_rib_width" | Multimaterial | Prime tower |
| "wipe_tower_rotation_angle" | Multimaterial | Prime tower |
| "wipe_tower_wall_type" | Multimaterial | Prime tower |
| "wipe_tower_x" | Multimaterial | Prime tower |
| "wipe_tower_y" | Multimaterial | Prime tower |
| "wiping_volumes_extruders" | Multimaterial | Flush options |
| "wrapping_detection_gcode" | Output/G-code | G-code macros |
| "wrapping_detection_layers" | Others | Special mode |
| "wrapping_exclude_area" | Others | Special mode |
| "xy_contour_compensation" | Quality | Precision |
| "xy_hole_compensation" | Quality | Precision |
| "z_hop" | Extruder/Nozzle | Retraction |
| "z_hop_types" | Extruder/Nozzle | Retraction |
| "z_offset" | Extruder/Nozzle | Retraction |

---

| Internal key | Feature | Sub-feature |

| "absolute_correction" | SLA Printing | Material (SLA) |
| "accel_to_decel_enable" | Speed | Acceleration |
| "accel_to_decel_factor" | Speed | Acceleration |
| "activate_air_filtration" | Cooling | General |
| "activate_chamber_temp_control" | Cooling | General |
| "adaptive_bed_mesh_margin" | Printer/Machine | Bed mesh |
| "adaptive_pressure_advance" | Extruder/Nozzle | Pressure advance |
| "adaptive_pressure_advance_bridges" | Extruder/Nozzle | Pressure advance |
| "adaptive_pressure_advance_model" | Extruder/Nozzle | Pressure advance |
| "adaptive_pressure_advance_overhangs" | Extruder/Nozzle | Pressure advance |
| "additional_cooling_fan_speed" | Cooling | General |
| "align_infill_direction_to_model" | Strength | Advanced |
| "allow_mix_temp" | Printer/Machine | Printer identity |
| "allow_multicolor_oneplate" | Printer/Machine | Printer identity |
| "alternate_extra_wall" | Strength | Walls |
| "area_fill" | SLA Printing | Faded layers |
| "auxiliary_fan" | Cooling | General |
| "bbl_calib_mark_logo" | Printer/Machine | Power/recovery |
| "bbl_use_printhost" | Printer/Machine | Printer identity |
| "bed_custom_model" | Printer/Machine | Print volume |
| "bed_custom_texture" | Printer/Machine | Print volume |
| "bed_exclude_area" | Printer/Machine | Print volume |
| "bed_mesh_max" | Printer/Machine | Bed mesh |
| "bed_mesh_min" | Printer/Machine | Bed mesh |
| "bed_mesh_probe_distance" | Printer/Machine | Bed mesh |
| "bed_temperature_formula" | Filament | Bed temperature |
| "before_layer_change_gcode" | Output/G-code | G-code macros |
| "best_object_pos" | Printer/Machine | Print volume |
| "bottle_cost" | SLA Printing | Material (SLA) |
| "bottle_volume" | SLA Printing | Material (SLA) |
| "bottle_weight" | SLA Printing | Material (SLA) |
| "bottom_shell_layers" | Strength | Top/bottom shells |
| "bottom_shell_thickness" | Strength | Top/bottom shells |
| "bottom_solid_infill_flow_ratio" | Quality | Walls and surfaces |
| "bottom_surface_density" | Strength | Top/bottom shells |
| "bottom_surface_pattern" | Strength | Top/bottom shells |
| "bridge_acceleration" | Speed | Acceleration |
| "bridge_angle" | Quality | Bridging |
| "bridge_density" | Quality | Bridging |
| "bridge_flow" | Quality | Bridging |
| "bridge_no_support" | Support | Advanced |
| "bridge_speed" | Speed | Overhang speed |
| "brim_ears" | Others | Brim |
| "brim_ears_detection_length" | Others | Brim |
| "brim_ears_max_angle" | Others | Brim |
| "brim_object_gap" | Others | Brim |
| "brim_type" | Others | Brim |
| "brim_use_efc_outline" | Others | Brim |
| "brim_width" | Others | Brim |
| "calib_flowrate_topinfill_special_order" | Calibration | Flow/Pressure advance |
| "chamber_temperature" | Filament | Temperature |
| "change_extrusion_role_gcode" | Output/G-code | G-code macros |
| "change_filament_gcode" | Output/G-code | G-code macros |
| "close_fan_the_first_x_layers" | Cooling | General |
| "compatible_machine_expression_group" | Output/G-code | Metadata |
| "compatible_printers" | Output/G-code | Metadata |
| "compatible_printers_condition" | Output/G-code | Metadata |
| "compatible_prints" | Output/G-code | Metadata |
| "compatible_prints_condition" | Output/G-code | Metadata |
| "compatible_process_expression_group" | Output/G-code | Metadata |
| "complete_print_exhaust_fan_speed" | Cooling | General |
| "cool_plate_temp" | Filament | Bed temperature |
| "cool_plate_temp_initial_layer" | Filament | Bed temperature |
| "cooling_tube_length" | Extruder/Nozzle | MMU Hardware |
| "cooling_tube_retraction" | Extruder/Nozzle | MMU Hardware |
| "counterbore_hole_bridging" | Quality | Bridging |
| "curr_bed_type" | Filament | Bed temperature |
| "default_acceleration" | Speed | Acceleration |
| "default_bed_type" | Filament | Bed temperature |
| "default_filament_colour" | Filament | General |
| "default_filament_profile" | Printer/Machine | Printer identity |
| "default_jerk" | Speed | Jerk (XY) |
| "default_junction_deviation" | Speed | Jerk (XY) |
| "default_nozzle_volume_type" | Extruder/Nozzle | Nozzle |
| "default_print_profile" | Printer/Machine | Printer identity |
| "deretraction_speed" | Extruder/Nozzle | Retraction |
| "detect_narrow_internal_solid_infill" | Strength | Advanced |
| "detect_overhang_wall" | Quality | Overhangs |
| "detect_thin_wall" | Quality | Wall generator — Classic |
| "different_settings_to_system" | Output/G-code | Metadata |
| "disable_m73" | Printer/Machine | Power/recovery |
| "display_height" | SLA Printing | Display |
| "display_mirror_x" | SLA Printing | Display |
| "display_mirror_y" | SLA Printing | Display |
| "display_orientation" | SLA Printing | Display |
| "display_pixels_x" | SLA Printing | Display |
| "display_pixels_y" | SLA Printing | Display |
| "display_width" | SLA Printing | Display |
| "dont_filter_internal_bridges" | Quality | Bridging |
| "dont_slow_down_outer_wall" | Cooling | General |
| "draft_shield" | Others | Skirt |
| "during_print_exhaust_fan_speed" | Cooling | General |
| "elefant_foot_compensation" | Quality | Precision |
| "elefant_foot_compensation_layers" | Quality | Precision |
| "elefant_foot_min_width" | SLA Printing | Display |
| "emit_machine_limits_to_gcode" | Printer/Machine | Power/recovery |
| "enable_arc_fitting" | Quality | Precision |
| "enable_extra_bridge_layer" | Quality | Bridging |
| "enable_filament_ramming" | Multimaterial | Prime tower |
| "enable_overhang_bridge_fan" | Cooling | General |
| "enable_overhang_speed" | Speed | Overhang speed |
| "enable_power_loss_recovery" | Printer/Machine | Power/recovery |
| "enable_pressure_advance" | Extruder/Nozzle | Pressure advance |
| "enable_prime_tower" | Multimaterial | Prime tower |
| "enable_support" | Support | Support |
| "enable_timelapse" | Others | Special mode |
| "enable_tower_interface_cooldown_during_tower" | Multimaterial | Prime tower |
| "enable_tower_interface_features" | Multimaterial | Prime tower |
| "enable_wrapping_detection" | Others | Special mode |
| "enforce_support_layers" | Support | Support |
| "eng_plate_temp" | Filament | Bed temperature |
| "eng_plate_temp_initial_layer" | Filament | Bed temperature |
| "ensure_vertical_shell_thickness" | Strength | Advanced |
| "exclude_object" | Others | G-code output |
| "exposure_time" | SLA Printing | Exposure |
| "extra_loading_move" | Extruder/Nozzle | MMU Hardware |
| "extra_perimeters_on_overhangs" | Quality | Overhangs |
| "extra_solid_infills" | Strength | Advanced |
| "extruder" | Quality | Walls and surfaces |
| "extruder_ams_count" | Extruder/Nozzle | Extruder geometry |
| "extruder_clearance_height_to_lid" | Printer/Machine | Print volume |
| "extruder_clearance_height_to_rod" | Printer/Machine | Print volume |
| "extruder_clearance_radius" | Printer/Machine | Print volume |
| "extruder_colour" | Extruder/Nozzle | Extruder geometry |
| "extruder_offset" | Extruder/Nozzle | Extruder geometry |
| "extruder_printable_area" | Printer/Machine | Print volume |
| "extruder_printable_height" | Printer/Machine | Print volume |
| "extruder_type" | Extruder/Nozzle | Extruder geometry |
| "extruder_variant_list" | Extruder/Nozzle | Extruder geometry |
| "extrusion_rate_smoothing_external_perimeter_only" | Speed | Advanced |
| "faded_layers" | SLA Printing | Faded layers |
| "fan_cooling_layer_time" | Cooling | General |
| "fan_kickstart" | Cooling | General |
| "fan_max_speed" | Cooling | General |
| "fan_min_speed" | Cooling | General |
| "fan_speedup_overhangs" | Cooling | General |
| "fan_speedup_time" | Cooling | General |
| "fast_tilt_time" | SLA Printing | Display |
| "filament_adaptive_volumetric_speed" | Filament | General |
| "filament_adhesiveness_category" | Filament | General |
| "filament_change_length" | Filament | General |
| "filament_colour" | Filament | General |
| "filament_colour_type" | Filament | General |
| "filament_cooling_final_speed" | Filament | General |
| "filament_cooling_initial_speed" | Filament | General |
| "filament_cooling_moves" | Filament | General |
| "filament_cost" | Filament | General |
| "filament_density" | Filament | General |
| "filament_diameter" | Filament | General |
| "filament_end_gcode" | Output/G-code | G-code macros |
| "filament_extruder_variant" | Extruder/Nozzle | Extruder geometry |
| "filament_flow_ratio" | Filament | General |
| "filament_flush_temp" | Multimaterial | Flush options |
| "filament_flush_volumetric_speed" | Multimaterial | Flush options |
| "filament_ids" | Filament | General |
| "filament_ironing_flow" | Filament | General |
| "filament_ironing_inset" | Filament | General |
| "filament_ironing_spacing" | Filament | General |
| "filament_ironing_speed" | Filament | General |
| "filament_is_support" | Filament | General |
| "filament_loading_speed" | Filament | General |
| "filament_loading_speed_start" | Filament | General |
| "filament_map" | Multimaterial | Filament for Features |
| "filament_map_mode" | Multimaterial | Filament for Features |
| "filament_max_volumetric_speed" | Filament | General |
| "filament_minimal_purge_on_wipe_tower" | Filament | General |
| "filament_multi_colour" | Filament | General |
| "filament_multitool_ramming" | Filament | General |
| "filament_multitool_ramming_flow" | Filament | General |
| "filament_multitool_ramming_volume" | Filament | General |
| "filament_notes" | Output/G-code | Metadata |
| "filament_printable" | Filament | General |
| "filament_ramming_parameters" | Filament | General |
| "filament_self_index" | Extruder/Nozzle | Extruder geometry |
| "filament_settings_id" | Filament | General |
| "filament_shrink" | Filament | General |
| "filament_shrinkage_compensation_z" | Filament | General |
| "filament_soluble" | Filament | General |
| "filament_stamping_distance" | Filament | General |
| "filament_stamping_loading_speed" | Filament | General |
| "filament_start_gcode" | Output/G-code | G-code macros |
| "filament_toolchange_delay" | Filament | General |
| "filament_tower_interface_pre_extrusion_dist" | Multimaterial | Prime tower |
| "filament_tower_interface_pre_extrusion_length" | Multimaterial | Prime tower |
| "filament_tower_interface_print_temp" | Multimaterial | Prime tower |
| "filament_tower_interface_purge_volume" | Multimaterial | Prime tower |
| "filament_tower_ironing_area" | Multimaterial | Prime tower |
| "filament_type" | Filament | General |
| "filament_unloading_speed" | Filament | General |
| "filament_unloading_speed_start" | Filament | General |
| "filament_vendor" | Filament | General |
| "file_start_gcode" | Output/G-code | G-code macros |
| "filename_format" | Others | G-code output |
| "fill_multiline" | Strength | Infill |
| "filter_out_gap_fill" | Strength | Infill |
| "first_layer_flow_ratio" | Quality | Walls and surfaces |
| "first_layer_print_sequence" | Quality | Layer height |
| "first_layer_sequence_choice" | Quality | Layer height |
| "flush_into_infill" | Multimaterial | Flush options |
| "flush_into_objects" | Multimaterial | Flush options |
| "flush_into_support" | Multimaterial | Flush options |
| "flush_multiplier" | Multimaterial | Flush options |
| "flush_volumes_matrix" | Multimaterial | Flush options |
| "flush_volumes_vector" | Multimaterial | Flush options |
| "full_fan_speed_layer" | Cooling | General |
| "fuzzy_skin" | Others | Fuzzy Skin |
| "fuzzy_skin_first_layer" | Others | Fuzzy Skin |
| "fuzzy_skin_mode" | Others | Fuzzy Skin |
| "fuzzy_skin_noise_type" | Others | Fuzzy Skin |
| "fuzzy_skin_octaves" | Others | Fuzzy Skin |
| "fuzzy_skin_persistence" | Others | Fuzzy Skin |
| "fuzzy_skin_point_distance" | Others | Fuzzy Skin |
| "fuzzy_skin_scale" | Others | Fuzzy Skin |
| "fuzzy_skin_thickness" | Others | Fuzzy Skin |
| "gamma_correction" | SLA Printing | Material (SLA) |
| "gap_fill_flow_ratio" | Quality | Walls and surfaces |
| "gap_fill_target" | Strength | Infill |
| "gap_infill_speed" | Speed | Other layers speed |
| "gcode_add_line_number" | Others | G-code output |
| "gcode_comments" | Others | G-code output |
| "gcode_flavor" | Printer/Machine | Printer identity |
| "gcode_label_objects" | Others | G-code output |
| "grab_length" | Extruder/Nozzle | MMU Hardware |
| "has_scarf_joint_seam" | Quality | Seam |
| "head_wrap_detect_zone" | Printer/Machine | Power/recovery |
| "high_current_on_filament_swap" | Extruder/Nozzle | MMU Hardware |
| "hole_to_polyhole" | Quality | Precision |
| "hole_to_polyhole_threshold" | Quality | Precision |
| "hole_to_polyhole_twisted" | Quality | Precision |
| "hollowing_closing_distance" | SLA Printing | Hollowing |
| "hollowing_enable" | SLA Printing | Hollowing |
| "hollowing_min_thickness" | SLA Printing | Hollowing |
| "hollowing_quality" | SLA Printing | Hollowing |
| "host_type" | Printer/Machine | Printer identity |
| "hot_plate_temp" | Filament | Bed temperature |
| "hot_plate_temp_initial_layer" | Filament | Bed temperature |
| "idle_temperature" | Filament | Temperature |
| "independent_support_layer_height" | Support | Advanced |
| "infill_anchor" | Strength | Infill |
| "infill_anchor_max" | Strength | Infill |
| "infill_combination" | Strength | Advanced |
| "infill_combination_max_layer_height" | Strength | Advanced |
| "infill_direction" | Strength | Infill |
| "infill_jerk" | Speed | Jerk (XY) |
| "infill_lock_depth" | Strength | Infill pattern-specific |
| "infill_overhang_angle" | Strength | Infill pattern-specific |
| "infill_shift_step" | Strength | Infill pattern-specific |
| "infill_wall_overlap" | Strength | Infill |
| "inherits" | Output/G-code | Metadata |
| "inherits_group" | Output/G-code | Metadata |
| "initial_exposure_time" | SLA Printing | Exposure |
| "initial_layer_acceleration" | Speed | Acceleration |
| "initial_layer_height" | SLA Printing | Material (SLA) |
| "initial_layer_infill_speed" | Speed | Initial layer speed |
| "initial_layer_jerk" | Speed | Jerk (XY) |
| "initial_layer_line_width" | Quality | Line width |
| "initial_layer_min_bead_width" | Quality | Wall generator |
| "initial_layer_print_height" | Quality | Layer height |
| "initial_layer_speed" | Speed | Initial layer speed |
| "initial_layer_travel_speed" | Speed | Initial layer speed |
| "inner_wall_acceleration" | Speed | Acceleration |
| "inner_wall_flow_ratio" | Quality | Walls and surfaces |
| "inner_wall_jerk" | Speed | Jerk (XY) |
| "inner_wall_line_width" | Quality | Line width |
| "inner_wall_speed" | Speed | Other layers speed |
| "interface_shells" | Multimaterial | Advanced |
| "interlocking_beam" | Multimaterial | Advanced |
| "interlocking_beam_layer_count" | Multimaterial | Advanced |
| "interlocking_beam_width" | Multimaterial | Advanced |
| "interlocking_boundary_avoidance" | Multimaterial | Advanced |
| "interlocking_depth" | Multimaterial | Advanced |
| "interlocking_orientation" | Multimaterial | Advanced |
| "internal_bridge_angle" | Quality | Bridging |
| "internal_bridge_density" | Quality | Bridging |
| "internal_bridge_fan_speed" | Cooling | General |
| "internal_bridge_flow" | Quality | Bridging |
| "internal_bridge_speed" | Speed | Overhang speed |
| "internal_solid_infill_acceleration" | Speed | Acceleration |
| "internal_solid_infill_flow_ratio" | Quality | Walls and surfaces |
| "internal_solid_infill_line_width" | Quality | Line width |
| "internal_solid_infill_pattern" | Strength | Infill |
| "internal_solid_infill_speed" | Speed | Other layers speed |
| "ironing_angle" | Quality | Ironing |
| "ironing_angle_fixed" | Quality | Ironing |
| "ironing_fan_speed" | Cooling | General |
| "ironing_flow" | Quality | Ironing |
| "ironing_inset" | Quality | Ironing |
| "ironing_pattern" | Quality | Ironing |
| "ironing_spacing" | Quality | Ironing |
| "ironing_speed" | Speed | Other layers speed |
| "ironing_type" | Quality | Ironing |
| "is_infill_first" | Quality | Walls and surfaces |
| "lateral_lattice_angle_1" | Strength | Infill pattern-specific |
| "lateral_lattice_angle_2" | Strength | Infill pattern-specific |
| "layer_change_gcode" | Output/G-code | G-code macros |
| "layer_height" | Quality | Layer height |
| "line_width" | Quality | Line width |
| "long_retractions_when_cut" | Extruder/Nozzle | Retraction |
| "long_retractions_when_ec" | Extruder/Nozzle | Retraction |
| "machine_end_gcode" | Output/G-code | G-code macros |
| "machine_load_filament_time" | Printer/Machine | Timing |
| "machine_max_acceleration_e" | Printer/Machine | Motion limits |
| "machine_max_acceleration_extruding" | Printer/Machine | Motion limits |
| "machine_max_acceleration_retracting" | Printer/Machine | Motion limits |
| "machine_max_acceleration_travel" | Printer/Machine | Motion limits |
| "machine_max_acceleration_x" | Printer/Machine | Motion limits |
| "machine_max_acceleration_y" | Printer/Machine | Motion limits |
| "machine_max_acceleration_z" | Printer/Machine | Motion limits |
| "machine_max_jerk_e" | Printer/Machine | Motion limits |
| "machine_max_jerk_x" | Printer/Machine | Motion limits |
| "machine_max_jerk_y" | Printer/Machine | Motion limits |
| "machine_max_jerk_z" | Printer/Machine | Motion limits |
| "machine_max_junction_deviation" | Printer/Machine | Motion limits |
| "machine_max_speed_e" | Printer/Machine | Motion limits |
| "machine_max_speed_x" | Printer/Machine | Motion limits |
| "machine_max_speed_y" | Printer/Machine | Motion limits |
| "machine_max_speed_z" | Printer/Machine | Motion limits |
| "machine_min_extruding_rate" | Printer/Machine | Motion limits |
| "machine_min_travel_rate" | Printer/Machine | Motion limits |
| "machine_pause_gcode" | Output/G-code | G-code macros |
| "machine_start_gcode" | Output/G-code | G-code macros |
| "machine_tool_change_time" | Printer/Machine | Timing |
| "machine_unload_filament_time" | Printer/Machine | Timing |
| "make_overhang_printable" | Quality | Overhangs |
| "make_overhang_printable_angle" | Quality | Overhangs |
| "make_overhang_printable_hole_size" | Quality | Overhangs |
| "manual_filament_change" | Multimaterial | Prime tower |
| "master_extruder_id" | Extruder/Nozzle | Extruder geometry |
| "material_colour" | SLA Printing | Material (SLA) |
| "material_correction" | SLA Printing | Material (SLA) |
| "material_correction_x" | SLA Printing | Material (SLA) |
| "material_correction_y" | SLA Printing | Material (SLA) |
| "material_correction_z" | SLA Printing | Material (SLA) |
| "material_density" | SLA Printing | Material (SLA) |
| "material_print_speed" | SLA Printing | Material (SLA) |
| "material_type" | SLA Printing | Material (SLA) |
| "material_vendor" | SLA Printing | Material (SLA) |
| "max_bridge_length" | Support | Advanced |
| "max_exposure_time" | SLA Printing | Exposure |
| "max_initial_exposure_time" | SLA Printing | Exposure |
| "max_layer_height" | Cooling | General |
| "max_resonance_avoidance_speed" | Printer/Machine | Resonance |
| "max_travel_detour_distance" | Quality | Walls and surfaces |
| "max_volumetric_extrusion_rate_slope" | Speed | Advanced |
| "max_volumetric_extrusion_rate_slope_segment_length" | Speed | Advanced |
| "min_bead_width" | Quality | Wall generator |
| "min_exposure_time" | SLA Printing | Exposure |
| "min_feature_size" | Quality | Wall generator |
| "min_initial_exposure_time" | SLA Printing | Exposure |
| "min_layer_height" | Cooling | General |
| "min_length_factor" | Quality | Wall generator |
| "min_resonance_avoidance_speed" | Printer/Machine | Resonance |
| "min_skirt_length" | Others | Skirt |
| "min_width_top_surface" | Quality | Walls and surfaces |
| "minimum_sparse_infill_area" | Strength | Advanced |
| "mmu_segmented_region_interlocking_depth" | Multimaterial | Advanced |
| "mmu_segmented_region_max_width" | Multimaterial | Advanced |
| "notes" | Output/G-code | Metadata |
| "nozzle_diameter" | Extruder/Nozzle | Nozzle |
| "nozzle_flush_dataset" | Multimaterial | Flush options |
| "nozzle_height" | Extruder/Nozzle | Nozzle |
| "nozzle_hrc" | Extruder/Nozzle | Nozzle |
| "nozzle_temperature" | Filament | Temperature |
| "nozzle_temperature_initial_layer" | Filament | Temperature |
| "nozzle_temperature_range_high" | Filament | Temperature |
| "nozzle_temperature_range_low" | Filament | Temperature |
| "nozzle_type" | Extruder/Nozzle | Nozzle |
| "nozzle_volume" | Extruder/Nozzle | Nozzle |
| "nozzle_volume_type" | Extruder/Nozzle | Nozzle |
| "only_one_wall_first_layer" | Quality | Walls and surfaces |
| "only_one_wall_top" | Quality | Walls and surfaces |
| "ooze_prevention" | Multimaterial | Ooze prevention |
| "other_layers_print_sequence" | Quality | Layer height |
| "other_layers_print_sequence_nums" | Quality | Layer height |
| "other_layers_sequence_choice" | Quality | Layer height |
| "outer_wall_acceleration" | Speed | Acceleration |
| "outer_wall_flow_ratio" | Quality | Walls and surfaces |
| "outer_wall_jerk" | Speed | Jerk (XY) |
| "outer_wall_line_width" | Quality | Line width |
| "outer_wall_speed" | Speed | Other layers speed |
| "overhang_1_4_speed" | Speed | Overhang speed |
| "overhang_2_4_speed" | Speed | Overhang speed |
| "overhang_3_4_speed" | Speed | Overhang speed |
| "overhang_4_4_speed" | Speed | Overhang speed |
| "overhang_fan_speed" | Cooling | General |
| "overhang_fan_threshold" | Cooling | General |
| "overhang_flow_ratio" | Quality | Walls and surfaces |
| "overhang_reverse" | Quality | Overhangs |
| "overhang_reverse_internal_only" | Quality | Overhangs |
| "overhang_reverse_threshold" | Quality | Overhangs |
| "pad_around_object" | SLA Printing | Pad |
| "pad_around_object_everywhere" | SLA Printing | Pad |
| "pad_brim_size" | SLA Printing | Pad |
| "pad_enable" | SLA Printing | Pad |
| "pad_max_merge_distance" | SLA Printing | Pad |
| "pad_object_connector_penetration" | SLA Printing | Pad |
| "pad_object_connector_stride" | SLA Printing | Pad |
| "pad_object_connector_width" | SLA Printing | Pad |
| "pad_object_gap" | SLA Printing | Pad |
| "pad_wall_height" | SLA Printing | Pad |
| "pad_wall_slope" | SLA Printing | Pad |
| "pad_wall_thickness" | SLA Printing | Pad |
| "parking_pos_retraction" | Extruder/Nozzle | MMU Hardware |
| "pellet_flow_coefficient" | Filament | General |
| "pellet_modded_printer" | Printer/Machine | Printer identity |
| "physical_extruder_map" | Extruder/Nozzle | Extruder geometry |
| "post_process" | Others | Post-processing |
| "precise_outer_wall" | Quality | Precision |
| "precise_z_height" | Quality | Precision |
| "preferred_orientation" | Printer/Machine | Print volume |
| "preheat_steps" | Multimaterial | Ooze prevention |
| "preheat_time" | Multimaterial | Ooze prevention |
| "pressure_advance" | Extruder/Nozzle | Pressure advance |
| "prime_tower_brim_width" | Multimaterial | Prime tower |
| "prime_tower_enable_framework" | Multimaterial | Prime tower |
| "prime_tower_flat_ironing" | Multimaterial | Prime tower |
| "prime_tower_infill_gap" | Multimaterial | Prime tower |
| "prime_tower_skip_points" | Multimaterial | Prime tower |
| "prime_tower_width" | Multimaterial | Prime tower |
| "prime_volume" | Multimaterial | Prime tower |
| "print_compatible_printers" | Output/G-code | Metadata |
| "print_extruder_id" | Extruder/Nozzle | Extruder geometry |
| "print_extruder_variant" | Extruder/Nozzle | Extruder geometry |
| "print_flow_ratio" | Quality | Walls and surfaces |
| "print_host" | Printer/Machine | Printer identity |
| "print_host_webui" | Printer/Machine | Printer identity |
| "print_order" | Others | Special mode |
| "print_sequence" | Others | Special mode |
| "print_settings_id" | Output/G-code | Metadata |
| "printable_area" | Printer/Machine | Print volume |
| "printable_height" | Printer/Machine | Print volume |
| "printer_agent" | Printer/Machine | Printer identity |
| "printer_extruder_id" | Extruder/Nozzle | Extruder geometry |
| "printer_extruder_variant" | Extruder/Nozzle | Extruder geometry |
| "printer_model" | Printer/Machine | Printer identity |
| "printer_notes" | Output/G-code | Metadata |
| "printer_settings_id" | Printer/Machine | Printer identity |
| "printer_structure" | Printer/Machine | Printer identity |
| "printer_technology" | Printer/Machine | Printer identity |
| "printer_variant" | Printer/Machine | Printer identity |
| "printhost_apikey" | Printer/Machine | Printer identity |
| "printhost_authorization_type" | Printer/Machine | Printer identity |
| "printhost_cafile" | Printer/Machine | Printer identity |
| "printhost_password" | Printer/Machine | Printer identity |
| "printhost_port" | Printer/Machine | Printer identity |
| "printhost_ssl_ignore_revoke" | Printer/Machine | Printer identity |
| "printhost_user" | Printer/Machine | Printer identity |
| "printing_by_object_gcode" | Output/G-code | G-code macros |
| "purge_in_prime_tower" | Multimaterial | Prime tower |
| "raft_contact_distance" | Support | Raft |
| "raft_expansion" | Support | Raft |
| "raft_first_layer_density" | Support | Support |
| "raft_first_layer_expansion" | Support | Support |
| "raft_layers" | Support | Raft |
| "reduce_crossing_wall" | Quality | Walls and surfaces |
| "reduce_fan_stop_start_freq" | Cooling | General |
| "reduce_infill_retraction" | Others | G-code output |
| "relative_correction" | SLA Printing | Material (SLA) |
| "relative_correction_x" | SLA Printing | Material (SLA) |
| "relative_correction_y" | SLA Printing | Material (SLA) |
| "relative_correction_z" | SLA Printing | Material (SLA) |
| "required_nozzle_HRC" | Extruder/Nozzle | Nozzle |
| "resolution" | Quality | Precision |
| "resonance_avoidance" | Printer/Machine | Resonance |
| "retract_before_wipe" | Extruder/Nozzle | Retraction |
| "retract_length_toolchange" | Extruder/Nozzle | Retraction |
| "retract_lift_above" | Extruder/Nozzle | Retraction |
| "retract_lift_below" | Extruder/Nozzle | Retraction |
| "retract_lift_enforce" | Extruder/Nozzle | Retraction |
| "retract_restart_extra" | Extruder/Nozzle | Retraction |
| "retract_restart_extra_toolchange" | Extruder/Nozzle | Retraction |
| "retract_when_changing_layer" | Extruder/Nozzle | Retraction |
| "retraction_distances_when_cut" | Extruder/Nozzle | Retraction |
| "retraction_distances_when_ec" | Extruder/Nozzle | Retraction |
| "retraction_length" | Extruder/Nozzle | Retraction |
| "retraction_minimum_travel" | Extruder/Nozzle | Retraction |
| "retraction_speed" | Extruder/Nozzle | Retraction |
| "role_based_wipe_speed" | Quality | Seam |
| "scan_first_layer" | Printer/Machine | Power/recovery |
| "scarf_angle_threshold" | Quality | Seam |
| "scarf_joint_flow_ratio" | Quality | Seam |
| "scarf_joint_speed" | Quality | Seam |
| "scarf_overhang_threshold" | Quality | Seam |
| "seam_gap" | Quality | Seam |
| "seam_position" | Quality | Seam |
| "seam_slope_conditional" | Quality | Seam |
| "seam_slope_entire_loop" | Quality | Seam |
| "seam_slope_inner_walls" | Quality | Seam |
| "seam_slope_min_length" | Quality | Seam |
| "seam_slope_start_height" | Quality | Seam |
| "seam_slope_steps" | Quality | Seam |
| "seam_slope_type" | Quality | Seam |
| "set_other_flow_ratios" | Quality | Walls and surfaces |
| "silent_mode" | Printer/Machine | Power/recovery |
| "single_extruder_multi_material" | Multimaterial | Prime tower |
| "single_extruder_multi_material_priming" | Multimaterial | Prime tower |
| "single_loop_draft_shield" | Others | Skirt |
| "skeleton_infill_density" | Strength | Infill pattern-specific |
| "skeleton_infill_line_width" | Strength | Infill pattern-specific |
| "skin_infill_density" | Strength | Infill pattern-specific |
| "skin_infill_depth" | Strength | Infill pattern-specific |
| "skin_infill_line_width" | Strength | Infill pattern-specific |
| "skirt_distance" | Others | Skirt |
| "skirt_height" | Others | Skirt |
| "skirt_loops" | Others | Skirt |
| "skirt_speed" | Others | Skirt |
| "skirt_start_angle" | Others | Skirt |
| "skirt_type" | Others | Skirt |
| "slice_closing_radius" | Quality | Precision |
| "slicing_mode" | Others | Special mode |
| "slow_down_for_layer_cooling" | Cooling | General |
| "slow_down_layer_time" | Cooling | General |
| "slow_down_layers" | Speed | Initial layer speed |
| "slow_down_min_speed" | Cooling | General |
| "slow_tilt_time" | SLA Printing | Display |
| "slowdown_for_curled_perimeters" | Speed | Overhang speed |
| "small_area_infill_flow_compensation" | Quality | Walls and surfaces |
| "small_area_infill_flow_compensation_model" | Quality | Walls and surfaces |
| "small_perimeter_speed" | Speed | Other layers speed |
| "small_perimeter_threshold" | Speed | Other layers speed |
| "solid_infill_direction" | Strength | Infill |
| "solid_infill_filament" | Multimaterial | Filament for Features |
| "solid_infill_rotate_template" | Strength | Infill |
| "sparse_infill_acceleration" | Speed | Acceleration |
| "sparse_infill_density" | Strength | Infill |
| "sparse_infill_filament" | Multimaterial | Filament for Features |
| "sparse_infill_flow_ratio" | Quality | Walls and surfaces |
| "sparse_infill_line_width" | Quality | Line width |
| "sparse_infill_pattern" | Strength | Infill |
| "sparse_infill_rotate_template" | Strength | Infill |
| "sparse_infill_speed" | Speed | Other layers speed |
| "spiral_finishing_flow_ratio" | Others | Special mode |
| "spiral_mode" | Others | Special mode |
| "spiral_mode_max_xy_smoothing" | Others | Special mode |
| "spiral_mode_smooth" | Others | Special mode |
| "spiral_starting_flow_ratio" | Others | Special mode |
| "staggered_inner_seams" | Quality | Seam |
| "standby_temperature_delta" | Multimaterial | Ooze prevention |
| "start_end_points" | Extruder/Nozzle | MMU Hardware |
| "supertack_plate_temp" | Filament | Bed temperature |
| "supertack_plate_temp_initial_layer" | Filament | Bed temperature |
| "support_air_filtration" | Support | Support |
| "support_angle" | Support | Support |
| "support_base_diameter" | SLA Printing | Support (SLA) |
| "support_base_height" | SLA Printing | Support (SLA) |
| "support_base_pattern" | Support | Advanced |
| "support_base_pattern_spacing" | Support | Advanced |
| "support_base_safety_distance" | SLA Printing | Support (SLA) |
| "support_bottom_interface_spacing" | Support | Advanced |
| "support_bottom_z_distance" | Support | Support |
| "support_buildplate_only" | SLA Printing | Support (SLA) |
| "support_chamber_temp_control" | Filament | Bed temperature |
| "support_critical_angle" | SLA Printing | Support (SLA) |
| "support_critical_regions_only" | Support | Support |
| "support_expansion" | Support | Support |
| "support_filament" | Support | Support filament |
| "support_flow_ratio" | Quality | Walls and surfaces |
| "support_head_front_diameter" | SLA Printing | Support (SLA) |
| "support_head_penetration" | SLA Printing | Support (SLA) |
| "support_head_width" | SLA Printing | Support (SLA) |
| "support_interface_bottom_layers" | Support | Advanced |
| "support_interface_filament" | Support | Support filament |
| "support_interface_flow_ratio" | Quality | Walls and surfaces |
| "support_interface_loop_pattern" | Support | Advanced |
| "support_interface_not_for_body" | Support | Support filament |
| "support_interface_pattern" | Support | Advanced |
| "support_interface_spacing" | Support | Advanced |
| "support_interface_speed" | Support | Advanced |
| "support_interface_top_layers" | Support | Advanced |
| "support_ironing" | Support | Support ironing |
| "support_ironing_flow" | Support | Support ironing |
| "support_ironing_pattern" | Support | Support ironing |
| "support_ironing_spacing" | Support | Support ironing |
| "support_line_width" | Quality | Line width |
| "support_material_interface_fan_speed" | Cooling | General |
| "support_max_bridge_length" | SLA Printing | Support (SLA) |
| "support_max_bridges_on_pillar" | SLA Printing | Support (SLA) |
| "support_max_pillar_link_distance" | SLA Printing | Support (SLA) |
| "support_multi_bed_types" | Filament | Bed temperature |
| "support_object_elevation" | SLA Printing | Support (SLA) |
| "support_object_first_layer_gap" | Support | Support |
| "support_object_skip_flush" | Multimaterial | Advanced |
| "support_object_xy_distance" | Support | Support |
| "support_on_build_plate_only" | Support | Support |
| "support_pillar_connection_mode" | SLA Printing | Support (SLA) |
| "support_pillar_diameter" | SLA Printing | Support (SLA) |
| "support_pillar_widening_factor" | SLA Printing | Support (SLA) |
| "support_points_density_relative" | SLA Printing | Support (SLA) |
| "support_points_minimal_distance" | SLA Printing | Support (SLA) |
| "support_remove_small_overhang" | Support | Support |
| "support_small_pillar_diameter_percent" | SLA Printing | Support (SLA) |
| "support_speed" | Speed | Other layers speed |
| "support_style" | Support | Support |
| "support_threshold_angle" | Support | Support |
| "support_threshold_overlap" | Support | Support |
| "support_top_z_distance" | Support | Support |
| "support_type" | Support | Support |
| "supports_enable" | SLA Printing | Support (SLA) |
| "symmetric_infill_y_axis" | Strength | Infill pattern-specific |
| "temperature_vitrification" | Filament | General |
| "template_custom_gcode" | Output/G-code | G-code macros |
| "textured_cool_plate_temp" | Filament | Bed temperature |
| "textured_cool_plate_temp_initial_layer" | Filament | Bed temperature |
| "textured_plate_temp" | Filament | Bed temperature |
| "textured_plate_temp_initial_layer" | Filament | Bed temperature |
| "thick_bridges" | Quality | Bridging |
| "thick_internal_bridges" | Quality | Bridging |
| "thumbnails" | Output/G-code | File output |
| "thumbnails_format" | Output/G-code | File output |
| "time_cost" | Printer/Machine | Timing |
| "time_lapse_gcode" | Output/G-code | G-code macros |
| "timelapse_type" | Others | Special mode |
| "top_bottom_infill_wall_overlap" | Strength | Top/bottom shells |
| "top_shell_layers" | Strength | Top/bottom shells |
| "top_shell_thickness" | Strength | Top/bottom shells |
| "top_solid_infill_flow_ratio" | Quality | Walls and surfaces |
| "top_surface_acceleration" | Speed | Acceleration |
| "top_surface_density" | Strength | Top/bottom shells |
| "top_surface_jerk" | Speed | Jerk (XY) |
| "top_surface_line_width" | Quality | Line width |
| "top_surface_pattern" | Strength | Top/bottom shells |
| "top_surface_speed" | Speed | Other layers speed |
| "travel_acceleration" | Speed | Acceleration |
| "travel_jerk" | Speed | Jerk (XY) |
| "travel_slope" | Extruder/Nozzle | Retraction |
| "travel_speed" | Speed | Travel speed |
| "travel_speed_z" | Speed | Travel speed |
| "tree_support_angle_slow" | Support | Tree supports |
| "tree_support_auto_brim" | Support | Tree supports |
| "tree_support_branch_angle" | Support | Tree supports |
| "tree_support_branch_angle_organic" | Support | Tree supports |
| "tree_support_branch_diameter" | Support | Tree supports |
| "tree_support_branch_diameter_angle" | Support | Tree supports |
| "tree_support_branch_diameter_organic" | Support | Tree supports |
| "tree_support_branch_distance" | Support | Tree supports |
| "tree_support_branch_distance_organic" | Support | Tree supports |
| "tree_support_brim_width" | Support | Tree supports |
| "tree_support_tip_diameter" | Support | Tree supports |
| "tree_support_top_rate" | Support | Tree supports |
| "tree_support_wall_count" | Support | Advanced |
| "tree_support_with_infill" | Support | Tree supports |
| "upward_compatible_machine" | Printer/Machine | Printer identity |
| "use_firmware_retraction" | Extruder/Nozzle | Retraction |
| "use_relative_e_distances" | Others | G-code output |
| "volumetric_speed_coefficients" | Filament | General |
| "wall_direction" | Quality | Walls and surfaces |
| "wall_distribution_count" | Quality | Wall generator |
| "wall_filament" | Multimaterial | Filament for Features |
| "wall_generator" | Quality | Wall generator |
| "wall_loops" | Strength | Walls |
| "wall_sequence" | Quality | Walls and surfaces |
| "wall_transition_angle" | Quality | Wall generator |
| "wall_transition_filter_deviation" | Quality | Wall generator |
| "wall_transition_length" | Quality | Wall generator |
| "wipe" | Extruder/Nozzle | Retraction |
| "wipe_before_external_loop" | Quality | Seam |
| "wipe_distance" | Extruder/Nozzle | Retraction |
| "wipe_on_loops" | Quality | Seam |
| "wipe_speed" | Quality | Seam |
| "wipe_tower_bridging" | Multimaterial | Prime tower |
| "wipe_tower_cone_angle" | Multimaterial | Prime tower |
| "wipe_tower_extra_flow" | Multimaterial | Prime tower |
| "wipe_tower_extra_rib_length" | Multimaterial | Prime tower |
| "wipe_tower_extra_spacing" | Multimaterial | Prime tower |
| "wipe_tower_filament" | Multimaterial | Filament for Features |
| "wipe_tower_fillet_wall" | Multimaterial | Prime tower |
| "wipe_tower_max_purge_speed" | Multimaterial | Prime tower |
| "wipe_tower_no_sparse_layers" | Multimaterial | Prime tower |
| "wipe_tower_rib_width" | Multimaterial | Prime tower |
| "wipe_tower_rotation_angle" | Multimaterial | Prime tower |
| "wipe_tower_wall_type" | Multimaterial | Prime tower |
| "wipe_tower_x" | Multimaterial | Prime tower |
| "wipe_tower_y" | Multimaterial | Prime tower |
| "wiping_volumes_extruders" | Multimaterial | Flush options |
| "wrapping_detection_gcode" | Output/G-code | G-code macros |
| "wrapping_detection_layers" | Others | Special mode |
| "wrapping_exclude_area" | Others | Special mode |
| "xy_contour_compensation" | Quality | Precision |
| "xy_hole_compensation" | Quality | Precision |
| "z_hop" | Extruder/Nozzle | Retraction |
| "z_hop_types" | Extruder/Nozzle | Retraction |
| "z_offset" | Extruder/Nozzle | Retraction |
---

## Mode-gating map
### GUI visibility modes
| Mode | Description | Key count | Typical options |
|---|---|---|---|
| comSimple | Basic/simple mode | ~30 | wall_loops, top_shell_layers, enable_support, skirt_loops, spiral_mode |
| comAdvanced | Advanced mode (default) | ~500+ | most print settings: speeds, accelerations, line widths |
| comDevelop | Developer mode | ~40 | scarf_joint_flow_ratio, gcode_add_line_number, preheat_steps, travel_speed_z |
| comExpert | Expert mode | 0 | reserved for future use |

### Enum-driven option gating
| Enum key | Enum value | Gated keys count | Gated keys |
|---|---|---|---|
| wall_generator | arachne | 8 | wall_transition_length, wall_transition_filter_deviation, wall_transition_angle, wall_distribution_count, min_bead_width, initial_layer_min_bead_width, min_feature_size, min_length_factor |
| wall_generator | classic | 1 | detect_thin_wall |
| support_type | normal | 3 | support_expansion, support_base_pattern, support_base_pattern_spacing |
| support_type | tree (normal style) | 5 | tree_support_branch_angle, tree_support_branch_distance, tree_support_branch_diameter, tree_support_auto_brim, tree_support_brim_width |
| support_type | tree (organic style) | 7 | tree_support_branch_angle_organic, tree_support_branch_distance_organic, tree_support_branch_diameter_organic, tree_support_tip_diameter, tree_support_top_rate, tree_support_branch_diameter_angle, tree_support_angle_slow |
| seam_slope_type | != none | 7 | seam_slope_conditional, seam_slope_start_height, seam_slope_entire_loop, seam_slope_min_length, seam_slope_steps, seam_slope_inner_walls, scarf_joint_speed |
| ironing_type | != no ironing | 7 | ironing_pattern, ironing_flow, ironing_spacing, ironing_angle, ironing_inset, ironing_angle_fixed, ironing_speed |
| fuzzy_skin | != disabled | 5 | fuzzy_skin_mode, fuzzy_skin_noise_type, fuzzy_skin_point_distance, fuzzy_skin_thickness, fuzzy_skin_first_layer |
| spiral_mode | true | 3 | spiral_mode_smooth, spiral_starting_flow_ratio, spiral_finishing_flow_ratio |

---


## Coverage report
Total configuration options documented: 682+ (in cross-reference index)
Total unique `def.add` keys in PrintConfig.cpp: 814 (includes CLI-only, runtime, and internal keys)
Total user-facing config options actively documented in feature tables: ~740

Options are organized by their UI tab/group as defined in TabPrint::build() in src/slic3r/GUI/Tab.cpp.
Every option includes: internal key, UI label, type, default value, units/enum values, mode/gating, and description.
Descriptions are rewritten from def->tooltip (≤15 words, not verbatim).

### Unclassified keys (167 keys from source not in main tables)
Most missing keys are CLI-only (`export_*`, `load_*`, `slice`, `help`), 
runtime/computed (`extruded_volume`, `print_time`, `layer_num`, `used_filament`), 
internal config management (`preset_names`, `inherits`, `compatible_*`), 
commented-out/legacy (`adaptive_layer_height`), 
or UI action commands (`arrange`, `center`, `cut`, `rotate`, `scale`, `split`).
These are noted in the cross-reference index but not given full feature tables.

### Mode-gating status
- Wall generator: properly split into Classic / Arachne sub-sections
- Support type: properly split into Normal / Tree / Organic sub-sections
- Support interface: separated into its own sub-section
- Brim type: gating noted in Applies-to column
- Seam slope: gating noted (seam_slope_type != None)
- Ironing: gating noted (ironing_type != NoIroning)
- Fuzzy skin noise: gating noted per noise type
- Spiral mode: gating noted (spiral_mode ON)
