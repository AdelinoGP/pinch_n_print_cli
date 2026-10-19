//! Single source of truth for the controlled-build (packet 254) artifact
//! namespaces shared by the `build-guests`, `test`, and `dist` entry points.
//! A `--accelerated` flag fans out across all three files; these constants keep
//! the isolated target/cache/staging directories from desyncing.

/// Isolated target/cache namespace for accelerated (controlled-compiler) guest
/// artifacts: `target/guests-accelerated/`, never `target/guests/`.
pub const ACCELERATED_GUEST_NAMESPACE: &str = "guests-accelerated";
/// Staging namespace for accelerated dist output: `target/dist-accelerated/`.
pub const ACCELERATED_DIST_NAMESPACE: &str = "dist-accelerated";
/// Isolated host target namespace for accelerated dist builds.
pub const ACCELERATED_HOST_TARGET_NAMESPACE: &str = "dist-host-accelerated";
/// Isolated fingerprint sidecar namespace for accelerated freshness metadata.
pub const ACCELERATED_FINGERPRINT_NAMESPACE: &str = "guest-fingerprints-accelerated";
/// Test-support feature that must never reach dist production artifacts.
pub const TEST_SUPPORT_FEATURE: &str = "perimeter-spatial-test-support";
