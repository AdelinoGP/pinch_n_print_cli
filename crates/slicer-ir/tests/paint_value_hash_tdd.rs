//! TDD tests verifying that `PaintValue` hashes consistently.
//!
//! These tests exist because `PaintValue::Scalar` wraps `f32` which cannot
//! use `#[derive(Hash)]` — a manual impl hashes via `f.to_bits()`.  The
//! test acts as a regression gate ensuring no one accidentally reverts to a
//! float-equality-based impl.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use slicer_ir::slice_ir::PaintValue;

fn hash_of(v: &PaintValue) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

#[test]
fn paint_value_hash_scalar_consistent() {
    let a = PaintValue::Scalar(1.5);
    let b = PaintValue::Scalar(1.5);
    assert_eq!(
        hash_of(&a),
        hash_of(&b),
        "two equal Scalar(1.5) values must produce the same hash"
    );
}

#[test]
fn paint_value_hash_custom_consistent() {
    let a = PaintValue::Custom("x".into());
    let b = PaintValue::Custom("x".into());
    assert_eq!(
        hash_of(&a),
        hash_of(&b),
        "two equal Custom(\"x\") values must produce the same hash"
    );
}

#[test]
fn paint_value_hash_scalar_equals_self() {
    let v = PaintValue::Scalar(2.5);
    assert_eq!(v, v.clone(), "Scalar must equal itself via PartialEq");
    assert_eq!(
        hash_of(&v),
        hash_of(&v.clone()),
        "hash must match equal values"
    );
}

#[test]
fn paint_value_hash_different_variants_differ() {
    let scalar = PaintValue::Scalar(1.0);
    let flag = PaintValue::Flag(true);
    // Hashes of different variants should differ (not guaranteed by contract,
    // but true for DefaultHasher on discriminant-first hashing).
    assert_ne!(
        hash_of(&scalar),
        hash_of(&flag),
        "distinct variants should produce distinct hashes"
    );
}
