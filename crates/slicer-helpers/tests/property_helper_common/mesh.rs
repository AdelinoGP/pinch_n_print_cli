//! Construction-first bounded triangle scenes and the local value tree.
//!
//! Every scene is built from exact eighth-millimetre grid steps, so vertex
//! positions are finite and exactly comparable. The non-shrinkable half of a
//! scene is [`SceneKind`] (the contact mode and therefore the component count);
//! the shrinkable half is [`SceneParams`] (a part-origin offset and a part
//! height, both plain grid sizes). [`SplitSceneValueTree`] only ever moves those
//! two parameters toward simpler values, and it re-validates every candidate
//! before exposing it, so a shrink step can never lose the contact-mode or
//! component witness the plan requires.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{Reason, TestRunner};
use slicer_ir::{IndexedTriangleSet, Point3};
use slicer_sdk::test_support::property_testing::oracles::validate_indexed_mesh;

use super::bounds::HelperLimits;

/// Grid steps per millimetre. Every coordinate is an integer count of these.
const EIGHTHS_PER_MM: i32 = 8;
/// X distance between adjacent scene parts, in eighths (2 mm).
///
/// A part's local X extent is at most one millimetre and the largest origin
/// offset is half a millimetre, so parts placed at this stride can never
/// overlap and their positions can never coincide.
const PART_STRIDE_EIGHTHS: i32 = 16;
/// Largest part-origin offset the shrinkable parameter may name, in eighths.
const MAX_OFFSET_EIGHTHS: i32 = 4;
/// Smallest admissible tetrahedron height or triangle apex, in eighths (0.5 mm).
const MIN_HEIGHT_EIGHTHS: i32 = 4;
/// Largest admissible tetrahedron height or triangle apex, in eighths (1.5 mm).
const MAX_HEIGHT_EIGHTHS: i32 = 12;
/// Candidate budget for one `simplify` call.
///
/// This record's registry profile has no `max_shrink_candidates` key, so the
/// budget is a documented local constant instead: at most one offset candidate
/// plus one per halving step of the height, well inside the budget.
const MAX_SHRINK_ATTEMPTS: u32 = 16;

/// Exact vertex position identity: the three IEEE-754 bit patterns.
///
/// Comparing positions bit-for-bit makes the oracle's face identity exact
/// rather than tolerance-based. The builders never emit negative zero, so bit
/// equality and value equality agree on every constructed scene.
pub type Position = [u32; 3];

/// Exact identity of one vertex position.
#[must_use]
pub fn position_bits(vertex: Point3) -> Position {
    [vertex.x.to_bits(), vertex.y.to_bits(), vertex.z.to_bits()]
}

/// Canonical oriented triangle: the lexicographically smallest cyclic rotation.
///
/// Rotation preserves winding; reversal is never applied, so a face and its
/// orientation-flipped twin stay distinguishable.
#[must_use]
pub fn canonical_face(positions: [Position; 3]) -> [Position; 3] {
    let rotations = [
        [positions[0], positions[1], positions[2]],
        [positions[1], positions[2], positions[0]],
        [positions[2], positions[0], positions[1]],
    ];
    rotations
        .into_iter()
        .min()
        .expect("three rotations always exist")
}

/// Number of complete triangles in `mesh`.
#[must_use]
pub fn face_count(mesh: &IndexedTriangleSet) -> usize {
    mesh.indices.len() / 3
}

/// Every face id of `mesh`, in ascending order.
#[must_use]
pub fn face_ids(mesh: &IndexedTriangleSet) -> Vec<usize> {
    (0..face_count(mesh)).collect()
}

/// Raw index triple of one face.
///
/// Precondition: `face < face_count(mesh)`. Callers that cannot prove that must
/// use [`face_positions`], which reports the out-of-range case.
#[must_use]
pub fn triangle_indices(mesh: &IndexedTriangleSet, face: usize) -> [u32; 3] {
    let base = face * 3;
    [
        mesh.indices[base],
        mesh.indices[base + 1],
        mesh.indices[base + 2],
    ]
}

/// The three directed edges of one face, in winding order.
///
/// Precondition: `face < face_count(mesh)`.
#[must_use]
pub fn directed_edges(mesh: &IndexedTriangleSet, face: usize) -> [(u32, u32); 3] {
    let [a, b, c] = triangle_indices(mesh, face);
    [(a, b), (b, c), (c, a)]
}

/// Position triple of one face, resolved through the mesh's own vertex table.
pub fn face_positions(mesh: &IndexedTriangleSet, face: usize) -> Result<[Position; 3], String> {
    let start = face
        .checked_mul(3)
        .ok_or_else(|| format!("face {face} overflows usize"))?;
    let end = start
        .checked_add(3)
        .ok_or_else(|| format!("face {face} end overflows usize"))?;
    let slice = mesh.indices.get(start..end).ok_or_else(|| {
        format!(
            "face {face} is outside the {} indices of this mesh",
            mesh.indices.len()
        )
    })?;
    let mut positions = [position_bits(Point3::default()); 3];
    for (slot, &index) in slice.iter().enumerate() {
        let vertex = mesh.vertices.get(index as usize).ok_or_else(|| {
            format!(
                "index {index} is outside the {} vertices of this mesh",
                mesh.vertices.len()
            )
        })?;
        positions[slot] = position_bits(*vertex);
    }
    Ok(positions)
}

/// Sorted multiset of canonical oriented faces for the given face ids.
pub fn face_multiset(
    mesh: &IndexedTriangleSet,
    faces: &[usize],
) -> Result<Vec<[Position; 3]>, String> {
    let mut multiset = Vec::with_capacity(faces.len());
    for &face in faces {
        multiset.push(canonical_face(face_positions(mesh, face)?));
    }
    multiset.sort();
    Ok(multiset)
}

fn eighths(steps: i32) -> f32 {
    steps as f32 / EIGHTHS_PER_MM as f32
}

fn point(x: i32, y: i32, z: i32) -> Point3 {
    Point3 {
        x: eighths(x),
        y: eighths(y),
        z: eighths(z),
    }
}

/// Closed tetrahedron whose local X/Y extent is one millimetre.
///
/// Index layout and outward winding match `split_tdd::tetrahedron`: bottom
/// `(0,2,1)`, then `(0,1,3)`, `(1,2,3)`, `(2,0,3)`.
fn tetrahedron_part(origin_x: i32, height: i32) -> IndexedTriangleSet {
    let at = |dx: i32, dy: i32, dz: i32| point(origin_x + dx, dy, dz);
    IndexedTriangleSet {
        vertices: vec![at(0, 0, 0), at(8, 0, 0), at(4, 8, 0), at(4, 4, height)],
        indices: vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3],
    }
}

/// Closed tetrahedron mirrored onto negative X, with index 0 at the contact point.
///
/// Same face layout as [`tetrahedron_part`], so merging the two with a shared
/// index 0 produces a scene whose only common index is that one vertex.
fn mirrored_tetrahedron_part(origin_x: i32, height: i32) -> IndexedTriangleSet {
    let at = |dx: i32, dy: i32, dz: i32| point(origin_x + dx, dy, dz);
    IndexedTriangleSet {
        vertices: vec![at(0, 0, 0), at(-8, 0, 0), at(-4, 8, 0), at(-4, 4, height)],
        indices: vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3],
    }
}

/// Open single triangle: one millimetre of base and an apex `height` eighths up.
fn triangle_part(origin_x: i32, height: i32) -> IndexedTriangleSet {
    let at = |dx: i32, dy: i32| point(origin_x + dx, dy, 0);
    IndexedTriangleSet {
        vertices: vec![at(0, 0), at(8, 0), at(4, height)],
        indices: vec![0, 1, 2],
    }
}

/// Two triangles sharing vertex indices 0 and 1.
///
/// With `opposite` the second face traverses the shared edge as `1->0`, i.e. the
/// reversed directed edge that the production adjacency rule requires. Without
/// it the second face traverses `0->1` too, so index adjacency must not join
/// the pair.
fn shared_edge_part(origin_x: i32, height: i32, opposite: bool) -> IndexedTriangleSet {
    let at = |dx: i32, dy: i32| point(origin_x + dx, dy, 0);
    let mut indices = vec![0, 1, 2];
    indices.extend(if opposite {
        vec![1, 0, 3]
    } else {
        vec![0, 1, 3]
    });
    IndexedTriangleSet {
        vertices: vec![at(0, 0), at(8, 0), at(4, height), at(4, -height)],
        indices,
    }
}

/// Concatenate `second` onto `first`, optionally reusing one of `first`'s
/// vertices for one of `second`'s.
///
/// `share = Some((first_index, second_index))` makes `second`'s `second_index`
/// refer to `first`'s `first_index` instead of a fresh slot, which is how a
/// vertex-only contact is built from two otherwise independent solids.
pub fn merge(
    first: &IndexedTriangleSet,
    second: &IndexedTriangleSet,
    share: Option<(u32, u32)>,
) -> IndexedTriangleSet {
    if let Some((first_index, second_index)) = share {
        assert!(
            (first_index as usize) < first.vertices.len()
                && (second_index as usize) < second.vertices.len(),
            "merge share must name existing vertices"
        );
    }
    let mut vertices = first.vertices.clone();
    let mut indices = first.indices.clone();
    let mut remap = Vec::with_capacity(second.vertices.len());
    for (index, vertex) in second.vertices.iter().enumerate() {
        if share.is_some_and(|(_, second_index)| second_index as usize == index) {
            remap.push(share.expect("share checked above").0);
        } else {
            remap.push(vertices.len() as u32);
            vertices.push(*vertex);
        }
    }
    indices.extend(second.indices.iter().map(|&index| remap[index as usize]));
    IndexedTriangleSet { vertices, indices }
}

fn concatenate(parts: &[IndexedTriangleSet]) -> IndexedTriangleSet {
    parts.iter().skip(1).fold(
        parts.first().cloned().unwrap_or_default(),
        |accumulated, part| merge(&accumulated, part, None),
    )
}

/// Non-shrinkable scene plan: the contact mode, and with it the component count.
///
/// Shrinking may move or resize a scene; it may never change which kind it is,
/// so the named contact witness survives every shrink step by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneKind {
    /// No triangles at all.
    Empty,
    /// One isolated open triangle.
    SingleTriangle,
    /// One closed tetrahedron.
    Tetrahedron,
    /// Two spatially disjoint closed tetrahedra.
    TwoDisjointSolids,
    /// Three spatially disjoint closed tetrahedra.
    ThreeDisjointSolids,
    /// Two triangles sharing a directed edge with opposite winding, which joins.
    OppositeEdgePair,
    /// Two triangles sharing a directed edge with the same winding, which does not join.
    SameDirectionEdgePair,
    /// Two closed tetrahedra sharing exactly one vertex index, which does not join.
    VertexOnlyContact,
    /// One closed tetrahedron plus one isolated single-face fragment.
    SolidWithFragment,
}

/// Every plan kind, in a fixed order, for the deterministic witness pass.
pub const EVERY_KIND: [SceneKind; 9] = [
    SceneKind::Empty,
    SceneKind::SingleTriangle,
    SceneKind::Tetrahedron,
    SceneKind::TwoDisjointSolids,
    SceneKind::ThreeDisjointSolids,
    SceneKind::OppositeEdgePair,
    SceneKind::SameDirectionEdgePair,
    SceneKind::VertexOnlyContact,
    SceneKind::SolidWithFragment,
];

impl SceneKind {
    /// Components this contact structure must produce.
    #[must_use]
    pub fn expected_components(self) -> usize {
        match self {
            SceneKind::Empty => 0,
            SceneKind::SingleTriangle | SceneKind::Tetrahedron | SceneKind::OppositeEdgePair => 1,
            SceneKind::TwoDisjointSolids
            | SceneKind::SameDirectionEdgePair
            | SceneKind::VertexOnlyContact
            | SceneKind::SolidWithFragment => 2,
            SceneKind::ThreeDisjointSolids => 3,
        }
    }

    /// Triangle count every scene of this kind carries.
    #[must_use]
    pub fn expected_faces(self) -> usize {
        match self {
            SceneKind::Empty => 0,
            SceneKind::SingleTriangle => 1,
            SceneKind::Tetrahedron => 4,
            SceneKind::TwoDisjointSolids | SceneKind::VertexOnlyContact => 8,
            SceneKind::ThreeDisjointSolids => 12,
            SceneKind::OppositeEdgePair | SceneKind::SameDirectionEdgePair => 2,
            SceneKind::SolidWithFragment => 5,
        }
    }

    /// Whether the merged mesh must satisfy the closed edge rule.
    ///
    /// A lone triangle, an edge-sharing pair and a solid-plus-fragment scene are
    /// open by construction; the two closed solids, their three-solid variant and
    /// the vertex-only contact (two closed tetrahedra meeting at one index) are
    /// closed even after merging.
    #[must_use]
    pub fn is_closed(self) -> bool {
        matches!(
            self,
            SceneKind::Tetrahedron
                | SceneKind::TwoDisjointSolids
                | SceneKind::ThreeDisjointSolids
                | SceneKind::VertexOnlyContact
        )
    }
}

/// The shrinkable half of a scene, in exact eighth-millimetre grid steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneParams {
    /// Uniform X offset added to every part origin, in eighths (`0..=4`).
    pub offset_eighths: i32,
    /// Tetrahedron height or triangle apex, in eighths (`4..=12`).
    pub height_eighths: i32,
}

impl SceneParams {
    /// Nonminimal witness parameters: widest offset and tallest parts.
    pub const WITNESS: Self = Self {
        offset_eighths: MAX_OFFSET_EIGHTHS,
        height_eighths: MAX_HEIGHT_EIGHTHS,
    };

    /// Simplest admissible parameters.
    pub const MINIMAL: Self = Self {
        offset_eighths: 0,
        height_eighths: MIN_HEIGHT_EIGHTHS,
    };

    /// Documented simplification weight.
    ///
    /// A shrink candidate is permitted only when this strictly decreases, so
    /// `simplify` can never enlarge a scene or trade one parameter for another.
    #[must_use]
    pub fn simplification_weight(self) -> i32 {
        self.offset_eighths + (self.height_eighths - MIN_HEIGHT_EIGHTHS)
    }
}

/// One standalone sub-mesh, validated against the shared mesh predicate on its own.
#[derive(Debug, Clone)]
pub struct MeshPart {
    /// The sub-mesh.
    pub mesh: IndexedTriangleSet,
    /// Whether this sub-mesh must satisfy the closed edge rule.
    pub closed: bool,
}

/// One constructed scene together with the plan and parameters that produced it.
#[derive(Debug, Clone)]
pub struct SplitScene {
    /// Non-shrinkable contact plan.
    pub kind: SceneKind,
    /// Shrinkable parameters.
    pub params: SceneParams,
    /// Standalone sub-meshes, for per-part validation.
    pub parts: Vec<MeshPart>,
    /// The merged scene the production SUT receives.
    pub mesh: IndexedTriangleSet,
}

/// Build the scene named by `kind` at `params`.
#[must_use]
pub fn make_scene(kind: SceneKind, params: SceneParams) -> SplitScene {
    let origin = params.offset_eighths;
    let height = params.height_eighths;
    let solid = |index: i32| tetrahedron_part(index * PART_STRIDE_EIGHTHS + origin, height);
    let (parts, mesh) = match kind {
        SceneKind::Empty => (Vec::new(), IndexedTriangleSet::default()),
        SceneKind::SingleTriangle => {
            let part = triangle_part(origin, height);
            (
                vec![MeshPart {
                    mesh: part.clone(),
                    closed: false,
                }],
                part,
            )
        }
        SceneKind::Tetrahedron => {
            let part = solid(0);
            (
                vec![MeshPart {
                    mesh: part.clone(),
                    closed: true,
                }],
                part,
            )
        }
        SceneKind::TwoDisjointSolids | SceneKind::ThreeDisjointSolids => {
            let count = if kind == SceneKind::TwoDisjointSolids {
                2
            } else {
                3
            };
            let solids: Vec<IndexedTriangleSet> = (0..count).map(solid).collect();
            (
                solids
                    .iter()
                    .cloned()
                    .map(|mesh| MeshPart { mesh, closed: true })
                    .collect(),
                concatenate(&solids),
            )
        }
        SceneKind::OppositeEdgePair => {
            let part = shared_edge_part(origin, height, true);
            (
                vec![MeshPart {
                    mesh: part.clone(),
                    closed: false,
                }],
                part,
            )
        }
        SceneKind::SameDirectionEdgePair => {
            let part = shared_edge_part(origin, height, false);
            (
                vec![MeshPart {
                    mesh: part.clone(),
                    closed: false,
                }],
                part,
            )
        }
        SceneKind::VertexOnlyContact => {
            let first = solid(0);
            let second = mirrored_tetrahedron_part(origin, height);
            (
                vec![
                    MeshPart {
                        mesh: first.clone(),
                        closed: true,
                    },
                    MeshPart {
                        mesh: second.clone(),
                        closed: true,
                    },
                ],
                merge(&first, &second, Some((0, 0))),
            )
        }
        SceneKind::SolidWithFragment => {
            let first = solid(0);
            let second = triangle_part(PART_STRIDE_EIGHTHS + origin, height);
            (
                vec![
                    MeshPart {
                        mesh: first.clone(),
                        closed: true,
                    },
                    MeshPart {
                        mesh: second.clone(),
                        closed: false,
                    },
                ],
                concatenate(&[first, second]),
            )
        }
    };
    SplitScene {
        kind,
        params,
        parts,
        mesh,
    }
}

/// Shared undirected edges between two faces, and whether each is traversed in
/// opposite directions.
fn shared_edges(mesh: &IndexedTriangleSet, first: usize, second: usize) -> Vec<((u32, u32), bool)> {
    let mut shared = Vec::new();
    for &(a0, a1) in &directed_edges(mesh, first) {
        for &(b0, b1) in &directed_edges(mesh, second) {
            if (a0 == b0 && a1 == b1) || (a0 == b1 && a1 == b0) {
                shared.push(((a0.min(a1), a0.max(a1)), a0 == b1 && a1 == b0));
            }
        }
    }
    shared
}

/// Prove the plan's contact witness is structurally present in the merged mesh.
fn validate_contact_witness(scene: &SplitScene) -> Result<(), String> {
    let mesh = &scene.mesh;
    let kind = scene.kind;
    let faces = face_count(mesh);
    if faces != kind.expected_faces() {
        return Err(format!(
            "{kind:?} must carry {} faces, found {faces}",
            kind.expected_faces()
        ));
    }
    match kind {
        SceneKind::Empty => Ok(()),
        SceneKind::SingleTriangle | SceneKind::Tetrahedron => {
            if scene.parts.len() != 1 {
                return Err(format!(
                    "{kind:?} is a single sub-mesh, found {} parts",
                    scene.parts.len()
                ));
            }
            Ok(())
        }
        SceneKind::TwoDisjointSolids | SceneKind::ThreeDisjointSolids => {
            let expected = kind.expected_components();
            if scene.parts.len() != expected {
                return Err(format!(
                    "{kind:?} needs {expected} separate closed solids, found {}",
                    scene.parts.len()
                ));
            }
            Ok(())
        }
        SceneKind::OppositeEdgePair => {
            let shared = shared_edges(mesh, 0, 1);
            if shared.len() != 1 || !shared[0].1 {
                return Err(format!(
                    "an opposite-edge pair must share exactly one reversed directed edge, found {shared:?}"
                ));
            }
            Ok(())
        }
        SceneKind::SameDirectionEdgePair => {
            let shared = shared_edges(mesh, 0, 1);
            if shared.len() != 1 || shared[0].1 {
                return Err(format!(
                    "a same-direction pair must share exactly one unreversed directed edge, found {shared:?}"
                ));
            }
            Ok(())
        }
        SceneKind::VertexOnlyContact => {
            let mut first = BTreeSet::new();
            let mut second = BTreeSet::new();
            for face in 0..4 {
                first.extend(triangle_indices(mesh, face));
            }
            for face in 4..8 {
                second.extend(triangle_indices(mesh, face));
            }
            if first.intersection(&second).count() != 1 {
                return Err("a vertex-only contact must share exactly one vertex index".to_string());
            }
            for left in 0..4 {
                for right in 4..8 {
                    if !shared_edges(mesh, left, right).is_empty() {
                        return Err(format!(
                            "faces {left} and {right} share an edge, so the contact is not vertex-only"
                        ));
                    }
                }
            }
            Ok(())
        }
        SceneKind::SolidWithFragment => {
            if scene.parts.len() != 2
                || face_count(&scene.parts[0].mesh) != 4
                || face_count(&scene.parts[1].mesh) != 1
                || !scene.parts[0].closed
                || scene.parts[1].closed
            {
                return Err(
                    "a solid-with-fragment scene needs one closed four-face solid and one open one-face fragment"
                        .to_string(),
                );
            }
            Ok(())
        }
    }
}

/// Checked preconditions every generated and shrink-exposed scene must satisfy.
///
/// These are the domain's own requirements, not production claims: finite and
/// globally distinct vertex positions, valid in-range indices, distinct oriented
/// faces (so the oracle's position-to-face lookup stays injective), the plan's
/// contact witness, and the profile's component/face/coordinate ceilings.
pub fn validate_scene(scene: &SplitScene, limits: &HelperLimits) -> Result<(), String> {
    let params = scene.params;
    if !(0..=MAX_OFFSET_EIGHTHS).contains(&params.offset_eighths) {
        return Err(format!(
            "offset {} eighths is outside 0..={MAX_OFFSET_EIGHTHS}",
            params.offset_eighths
        ));
    }
    if !(MIN_HEIGHT_EIGHTHS..=MAX_HEIGHT_EIGHTHS).contains(&params.height_eighths) {
        return Err(format!(
            "height {} eighths is outside {MIN_HEIGHT_EIGHTHS}..={MAX_HEIGHT_EIGHTHS}",
            params.height_eighths
        ));
    }
    for (index, part) in scene.parts.iter().enumerate() {
        validate_indexed_mesh(&part.mesh, part.closed)
            .map_err(|error| format!("sub-mesh {index} is invalid: {error}"))?;
    }
    validate_indexed_mesh(&scene.mesh, scene.kind.is_closed())
        .map_err(|error| format!("merged mesh is invalid: {error}"))?;
    if scene
        .mesh
        .vertices
        .iter()
        .any(|vertex| !vertex.x.is_finite() || !vertex.y.is_finite() || !vertex.z.is_finite())
    {
        return Err("every vertex coordinate must be finite".to_string());
    }
    let mut positions: Vec<Position> = scene
        .mesh
        .vertices
        .iter()
        .copied()
        .map(position_bits)
        .collect();
    positions.sort_unstable();
    if positions.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("vertex positions must be globally distinct".to_string());
    }
    let mut signatures = Vec::with_capacity(face_count(&scene.mesh));
    for face in 0..face_count(&scene.mesh) {
        signatures.push(canonical_face(face_positions(&scene.mesh, face)?));
    }
    signatures.sort();
    if signatures.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("two faces share the same oriented position triple".to_string());
    }
    validate_contact_witness(scene)?;
    if scene.kind.expected_components() as u32 > limits.max_components {
        return Err(format!(
            "{:?} needs {} components but max_components is {}",
            scene.kind,
            scene.kind.expected_components(),
            limits.max_components
        ));
    }
    if face_count(&scene.mesh) > limits.max_faces {
        return Err(format!(
            "{:?} carries {} faces but max_faces is {}",
            scene.kind,
            face_count(&scene.mesh),
            limits.max_faces
        ));
    }
    let max_abs_mm = scene
        .mesh
        .vertices
        .iter()
        .map(|vertex| vertex.x.abs().max(vertex.y.abs()).max(vertex.z.abs()))
        .fold(0.0_f32, f32::max);
    if max_abs_mm > limits.max_abs_mm {
        return Err(format!(
            "{:?} reaches {max_abs_mm} mm but max_abs_mm is {}",
            scene.kind, limits.max_abs_mm
        ));
    }
    Ok(())
}

/// Strategy for bounded split scenes whose tree only ever shrinks offsets and sizes.
#[derive(Debug, Clone)]
pub struct SplitSceneStrategy {
    limits: HelperLimits,
    plan: BoxedStrategy<SceneKind>,
    params: BoxedStrategy<SceneParams>,
}

/// Construct the `H-HELPERS-SPLIT` scene strategy.
///
/// The plan sampler is explicitly `no_shrink`: the contact mode and component
/// count are not shrinkable values, they are the witness the property protects.
#[must_use]
pub fn split_scene_strategy(limits: &HelperLimits) -> SplitSceneStrategy {
    let plan = proptest::sample::select(EVERY_KIND.to_vec())
        .no_shrink()
        .boxed();
    let params = (
        0..=MAX_OFFSET_EIGHTHS,
        MIN_HEIGHT_EIGHTHS..=MAX_HEIGHT_EIGHTHS,
    )
        .prop_map(|(offset_eighths, height_eighths)| SceneParams {
            offset_eighths,
            height_eighths,
        })
        .boxed();
    SplitSceneStrategy {
        limits: *limits,
        plan,
        params,
    }
}

impl Strategy for SplitSceneStrategy {
    type Tree = SplitSceneValueTree;
    type Value = SplitScene;

    fn new_tree(&self, runner: &mut TestRunner) -> Result<Self::Tree, Reason> {
        let kind = self.plan.new_tree(runner)?.current();
        let params = self.params.new_tree(runner)?.current();
        let scene = make_scene(kind, params);
        validate_scene(&scene, &self.limits).map_err(|error| {
            Reason::from(format!(
                "constructed an invalid split scene ({kind:?} {params:?}): {error}"
            ))
        })?;
        Ok(SplitSceneValueTree {
            limits: self.limits,
            kind,
            current: params,
            scene,
            history: Vec::new(),
            rejected: Vec::new(),
        })
    }
}

/// The local value tree: `current`, witness-preserving `simplify`, `complicate`.
///
/// Every candidate is rebuilt and re-validated before it is exposed, so an
/// exposed value can never violate the scene preconditions.
#[derive(Debug)]
pub struct SplitSceneValueTree {
    limits: HelperLimits,
    kind: SceneKind,
    current: SceneParams,
    scene: SplitScene,
    history: Vec<SceneParams>,
    rejected: Vec<(SceneParams, SceneParams)>,
}

impl SplitSceneValueTree {
    /// Build a tree around explicit parameters.
    ///
    /// # Panics
    ///
    /// Panics when `params` does not produce a valid scene under `limits`, so a
    /// control cannot start from an out-of-domain probe.
    #[must_use]
    pub fn with_params(limits: HelperLimits, kind: SceneKind, params: SceneParams) -> Self {
        let scene = make_scene(kind, params);
        validate_scene(&scene, &limits).unwrap_or_else(|error| {
            panic!("with_params got an invalid scene ({kind:?} {params:?}): {error}")
        });
        Self {
            limits,
            kind,
            current: params,
            scene,
            history: Vec::new(),
            rejected: Vec::new(),
        }
    }

    /// Try one candidate, bounded by the per-call attempt budget.
    ///
    /// `None` means "this candidate was already rejected or is not simpler";
    /// `Some(false)` means the attempt budget is exhausted.
    fn consider(&mut self, candidate: SceneParams, attempts: &mut u32) -> Option<bool> {
        if *attempts >= MAX_SHRINK_ATTEMPTS {
            return Some(false);
        }
        *attempts += 1;
        if self
            .rejected
            .iter()
            .any(|(from, to)| *from == self.current && *to == candidate)
        {
            return None;
        }
        if candidate.simplification_weight() >= self.current.simplification_weight() {
            return None;
        }
        let scene = make_scene(self.kind, candidate);
        if validate_scene(&scene, &self.limits).is_err() {
            return None;
        }
        self.history
            .push(std::mem::replace(&mut self.current, candidate));
        self.scene = scene;
        Some(true)
    }
}

impl ValueTree for SplitSceneValueTree {
    type Value = SplitScene;

    fn current(&self) -> SplitScene {
        self.scene.clone()
    }

    fn simplify(&mut self) -> bool {
        let mut attempts = 0;
        if self.current.offset_eighths > 0 {
            let candidate = SceneParams {
                offset_eighths: 0,
                ..self.current
            };
            if let Some(result) = self.consider(candidate, &mut attempts) {
                return result;
            }
        }
        let mut reduced = self.current;
        while reduced.height_eighths > MIN_HEIGHT_EIGHTHS {
            reduced.height_eighths = (reduced.height_eighths + MIN_HEIGHT_EIGHTHS) / 2;
            if let Some(result) = self.consider(reduced, &mut attempts) {
                return result;
            }
        }
        false
    }

    fn complicate(&mut self) -> bool {
        if let Some(previous) = self.history.pop() {
            let rejected = std::mem::replace(&mut self.current, previous);
            self.scene = make_scene(self.kind, previous);
            self.rejected.push((self.current, rejected));
            true
        } else {
            false
        }
    }
}
