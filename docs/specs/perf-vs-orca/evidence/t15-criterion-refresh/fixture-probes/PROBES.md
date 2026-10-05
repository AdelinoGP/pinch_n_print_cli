# t15 fixture probes — do the mesh_ops fixtures exercise repair/decimate?

Run 2026-09-23 with the same-release-profile `target/release/pnp_cli.exe`
(`mesh import|repair|decimate` call the identical `slicer-helpers`
functions the bench calls: `repair` (`crates/slicer-helpers/src/repair.rs`),
`decimate` (`crates/slicer-helpers/src/decimate.rs`)).

## Inputs

```
$ pnp_cli mesh import --input crates/slicer-helpers/tests/resources/cube.step --output cube.stl --no-repair
target/t15-fixture-probe/i-cube.stl -> 12 triangles
$ pnp_cli mesh import --input .../assembly.step --output assembly.stl --no-repair --merge-components
target/t15-fixture-probe/i-assembly.stl -> 24 triangles
```

## repair on the bench fixture (cube.step): three phases find nothing

```
$ pnp_cli mesh repair --input cube.stl --output cube-rep.stl --stats
{"components":1,"degenerate_removed":0,"event":"done","faces_reoriented":0,"open_edges_closed":0,"operation":"repair","warnings":[]}
```

## repair on a fixture that needs repair (resources/regression_wedge.stl, 80 tri, 5 components)

```
$ pnp_cli mesh repair --input resources/regression_wedge.stl --output wedge-rep.stl --stats
{"detail":{"count":5},"event":"warning","kind":"multiple_components","operation":"repair"}
{"components":5,"degenerate_removed":0,"event":"done","faces_reoriented":12,"open_edges_closed":56,"operation":"repair","warnings":["multiple_components"]}
```

## decimate sweep on the bench fixture: error budget is the gate

```
$ for e in 0.01 0.05 0.1 0.2 0.3 0.4 0.5 0.6; do pnp_cli mesh decimate --input cube.stl --output e.stl --target-ratio 0.5 --max-error \$e --stats; done
max_error=0.01  {"achieved_error":0.0,"event":"done","final_triangle_count":12,"operation":"decimate","original_triangle_count":12,"target_reached":false}
max_error=0.05  {"achieved_error":0.0,"event":"done","final_triangle_count":12,"operation":"decimate","original_triangle_count":12,"target_reached":false}
max_error=0.1   {"achieved_error":0.0,"event":"done","final_triangle_count":12,"operation":"decimate","original_triangle_count":12,"target_reached":false}
max_error=0.2   {"achieved_error":0.0,"event":"done","final_triangle_count":12,"operation":"decimate","original_triangle_count":12,"target_reached":false}
max_error=0.3   {"achieved_error":0.0,"event":"done","final_triangle_count":12,"operation":"decimate","original_triangle_count":12,"target_reached":false}
max_error=0.4   {"achieved_error":0.0,"event":"done","final_triangle_count":12,"operation":"decimate","original_triangle_count":12,"target_reached":false}
max_error=0.5   {"achieved_error":0.44721367955207825,"event":"done","final_triangle_count":6,"operation":"decimate","original_triangle_count":12,"target_reached":true}
max_error=0.6   {"achieved_error":0.44721367955207825,"event":"done","final_triangle_count":6,"operation":"decimate","original_triangle_count":12,"target_reached":true}
```

## decimate on a dense mesh (resources/calicat.stl, 876 tri) at the bench's default budget

```
$ pnp_cli mesh decimate --input resources/calicat.stl --output calicat-dec.stl --target-ratio 0.5 --stats
{"achieved_error":0.0001603284472366795,"event":"done","final_triangle_count":438,"operation":"decimate","original_triangle_count":876,"target_reached":true}
```

Reading: `repair/cube` measures a scan that returns the mesh unchanged
(`degenerate_removed: 0, faces_reoriented: 0, open_edges_closed: 0`), and
`decimate/cube_default` measures a simplification attempt that the default
`max_error = 0.01` budget rejects (12 -> 12; the budget must exceed 0.4 to
collapse this box to 6). Both numbers are reproducible real work on a
fixture that cannot exhibit a successful repair/simplification. See
FINDINGS.md "What the read-back-from-disk discipline changed" #2 and #3.
