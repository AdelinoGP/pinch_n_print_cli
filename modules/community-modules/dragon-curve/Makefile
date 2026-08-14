# Dragon-curve community module — build script.
#
# This module is NOT part of the Cargo workspace and is NOT discovered by
# `cargo xtask build-guests` (whose walk is hard-coded to modules/core-modules
# and crates/slicer-wasm-host/test-guests). Nothing rebuilds dragon-curve.wasm
# automatically: run `make` here by hand after changing src/, then commit the
# resulting artifact.
#
# Toolchain versions this was built and verified against:
#   wit-bindgen-cli 0.60.0
#   moon 0.1.20260807 (4da23f8) / moonc v0.10.7
#   wasm-tools 1.250.0
#
# The build is not bit-reproducible: a second clean build produces a different
# hash. That is a recorded MoonBit property, not a defect here.

WORLD   := slicer:layer-infill/infill-module
WIT     := wit
IFACE   := gen/interface/slicer/layer-infill/infill
ARTIFACT := dragon-curve.wasm

.PHONY: all bindings build component test fmt clean

all: $(ARTIFACT)

# Regenerate bindings from the WIT snapshot. This blows away gen/, interface/
# and world/, which is why no hand-written source may live in them; src/ is
# untouched.
bindings:
	-rm -rf gen interface world
	wit-bindgen moonbit \
		$(WIT)/deps/config/config.wit \
		$(WIT)/deps/types/types.wit \
		$(WIT)/deps/ir-types/ir-types.wit \
		$(WIT)/deps/common/common.wit \
		$(WIT)/layer-infill.wit \
		--out-dir . --derive-show --derive-eq --derive-error -w $(WORLD)
	# wit-bindgen forward-declares `run` in the exported interface package, so
	# the definition must be copied there - NOT into gen/. Putting it in gen/
	# yields a component that builds and then traps on dispatch.
	cp -f src/glue/main.mbt.in $(IFACE)/main.mbt
	# The generated package manifest only imports what the bindings need; the
	# glue additionally needs geometry, host-services and the pure dragon package.
	cp -f src/glue/moon.pkg.json.in $(IFACE)/moon.pkg.json

build: bindings
	moon build --target wasm --release

# Formatting rewrites source files, so it is an explicit target rather than a
# build side effect. (`moon fmt` also converts moon.mod.json -> moon.mod under
# the rr_moon_mod feature flag.)
fmt:
	moon fmt

# MoonBit emits a bare core module with no WASI imports; embed the world and
# wrap it into a component. --encoding utf16 is required: MoonBit strings are
# UTF-16 and the host transcodes at the canonical ABI boundary.
$(ARTIFACT): build
	wasm-tools component embed --encoding utf16 -w infill-module $(WIT) \
		_build/wasm/release/build/gen/gen.wasm -o embedded.wasm
	wasm-tools component new -o $(ARTIFACT) embedded.wasm
	wasm-tools component wit $(ARTIFACT) > /dev/null

component: $(ARTIFACT)

# Unit tests for the pure tiling and colour logic. The packet's acceptance
# criteria invoke individual tests from here by name via `moon test -f`.
test:
	moon test --target wasm -p slicer/layer-infill/src/dragon

clean:
	-rm -rf gen interface world _build embedded.wasm
