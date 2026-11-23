//! Scalar and identifier strategies.

use std::ops::RangeInclusive;

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use slicer_ir::ConfigValue;

use super::{DomainError, IdTextKind, ScalarConfigDomain};

// The transformed integers are monotone from negative to positive floats,
// including both zeros, and exclude all infinities/NaNs when endpoints do.
macro_rules! finite_strategy {
    ($name:ident, $float:ty, $uint:ty, $sign:expr) => {
        pub(super) fn $name(range: RangeInclusive<$float>) -> Result<BoxedStrategy<$float>, DomainError> {
            let (min, max) = range.into_inner();
            if !min.is_finite() || !max.is_finite() || min > max {
                return Err(DomainError::InvalidBounds("finite ordered range required".into()));
            }
            let encode = |value: $float| -> $uint {
                let bits = value.to_bits();
                if bits & $sign != 0 { !bits } else { bits ^ $sign }
            };
            let decode = |key: $uint| -> $float {
                <$float>::from_bits(if key & $sign != 0 { key ^ $sign } else { !key })
            };
            // Numerically both zero signs are inside a range touching zero.
            let lower = if min == 0.0 { -0.0 } else { min };
            let upper = if max == 0.0 { 0.0 } else { max };
            let interior = (encode(lower)..=encode(upper)).prop_map(decode);
            let neighbors = interior.clone().prop_map(move |value| {
                (value.next_down().max(min), value.next_up().min(max))
            });
            let mut witnesses = vec![min, max];
            if min <= 0.0 && max >= 0.0 {
                witnesses.extend([0.0, -0.0]);
            }
            for candidate in [<$float>::from_bits(1), <$float>::from_bits($sign | 1), min, max] {
                if candidate != 0.0 && candidate.is_subnormal() && candidate >= min && candidate <= max {
                    witnesses.push(candidate);
                }
            }
            Ok(prop_oneof![
                5 => interior,
                2 => proptest::sample::select(witnesses),
                1 => neighbors.clone().prop_map(|(down, _)| down),
                1 => neighbors.prop_map(|(_, up)| up),
            ].boxed())
        }
    };
}

finite_strategy!(finite_f32, f32, u32, 0x8000_0000_u32);
finite_strategy!(finite_f64, f64, u64, 0x8000_0000_0000_0000_u64);

pub(super) fn nonfinite_f32() -> BoxedStrategy<f32> {
    proptest::sample::select(vec![
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0xffc0_0002),
        f32::from_bits(0x7f80_0001),
    ])
    .boxed()
}

pub(super) fn nonfinite_f64() -> BoxedStrategy<f64> {
    proptest::sample::select(vec![
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::from_bits(0xfff8_0000_0000_0002),
        f64::from_bits(0x7ff0_0000_0000_0001),
    ])
    .boxed()
}

// Keep boundary indices exactly representable as f64 and probes small.
const MAX_BOUNDARY_UNITS: i64 = 1_000_000;
const MAX_TEXT_BYTES: usize = 4096;

pub(super) fn rounding_boundary_mm(max_abs_units: i64) -> Result<BoxedStrategy<f32>, DomainError> {
    if !(1..=MAX_BOUNDARY_UNITS).contains(&max_abs_units) {
        return Err(DomainError::InvalidBounds(
            "boundary unit limit must be 1..=1000000".into(),
        ));
    }
    Ok((
        prop_oneof![Just(-1_i64), Just(0_i64), -max_abs_units..max_abs_units],
        0_u8..=2,
    )
        .prop_map(|(k, neighbor)| {
            // k is within the checked bound; one canonical unit is 10^-4 mm.
            let boundary = ((k as f64 + 0.5) / 10_000.0) as f32;
            match neighbor {
                0 => boundary.next_down(),
                1 => boundary,
                _ => boundary.next_up(),
            }
        })
        .boxed())
}

pub(super) fn scalar_config_values(
    domain: ScalarConfigDomain,
) -> Result<BoxedStrategy<ConfigValue>, DomainError> {
    Ok(match domain {
        ScalarConfigDomain::Bool => any::<bool>().prop_map(ConfigValue::Bool).boxed(),
        ScalarConfigDomain::Int { min, max } => {
            if min > max {
                return Err(DomainError::InvalidBounds("reversed integer range".into()));
            }
            (min..=max).prop_map(ConfigValue::Int).boxed()
        }
        ScalarConfigDomain::Float { min, max } => {
            finite_f64(min..=max)?.prop_map(ConfigValue::Float).boxed()
        }
        ScalarConfigDomain::Percent { min, max } => finite_f64(min..=max)?
            .prop_map(ConfigValue::Percent)
            .boxed(),
        ScalarConfigDomain::FloatOrPercent {
            min,
            max,
            is_percent,
        } => finite_f64(min..=max)?
            .prop_map(move |value| ConfigValue::FloatOrPercent { value, is_percent })
            .boxed(),
        ScalarConfigDomain::StringChoices(choices) => {
            if choices.is_empty() {
                return Err(DomainError::InvalidInput(
                    "string choices must not be empty".into(),
                ));
            }
            proptest::sample::select(choices)
                .prop_map(ConfigValue::String)
                .boxed()
        }
    })
}

pub(super) fn id_text(
    kind: IdTextKind,
    max_bytes: usize,
) -> Result<BoxedStrategy<String>, DomainError> {
    if max_bytes > MAX_TEXT_BYTES {
        return Err(DomainError::InvalidBounds(
            "identifier byte limit exceeds 4096".into(),
        ));
    }
    match kind {
        IdTextKind::UuidSyntax => {
            if max_bytes < 36 {
                return Err(DomainError::InvalidBounds(
                    "UUID syntax needs 36 bytes".into(),
                ));
            }
            let arbitrary = proptest::collection::vec(0_u8..16, 32).prop_map(|digits| {
                let mut text = String::with_capacity(36);
                for (index, digit) in digits.into_iter().enumerate() {
                    if [8, 12, 16, 20].contains(&index) {
                        text.push('-');
                    }
                    text.push(char::from(b"0123456789abcdef"[usize::from(digit)]));
                }
                text
            });
            Ok(prop_oneof![
                4 => arbitrary,
                1 => Just("00000000-0000-0000-0000-000000000000".to_string()),
                1 => Just("ffffffff-ffff-ffff-ffff-ffffffffffff".to_string()),
            ]
            .boxed())
        }
        IdTextKind::OtherText => {
            let ascii = proptest::collection::vec(b'0'..=b'z', 0..=max_bytes)
                .prop_map(|bytes| bytes.into_iter().map(char::from).collect::<String>());
            let unicode =
                proptest::collection::vec(any::<char>(), 0..=max_bytes).prop_map(move |chars| {
                    let mut text = String::new();
                    for ch in chars {
                        if text.len() + ch.len_utf8() > max_bytes {
                            break;
                        }
                        text.push(ch);
                    }
                    text
                });
            let generated = prop_oneof![ascii, unicode].prop_map(|mut text| {
                let bytes = text.as_bytes();
                let uuid = bytes.len() == 36
                    && bytes.iter().enumerate().all(|(index, byte)| {
                        if [8, 13, 18, 23].contains(&index) {
                            *byte == b'-'
                        } else {
                            byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
                        }
                    });
                if uuid {
                    text.replace_range(..1, "g");
                }
                text
            });
            let mut witnesses = vec![String::new()];
            if max_bytes >= 1 {
                witnesses.push("!".to_string());
            }
            if max_bytes >= 2 {
                witnesses.push("é".to_string());
            }
            if max_bytes >= 36 {
                witnesses.push("FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF".to_string());
                witnesses.push("00000000_0000-0000-0000-000000000000".to_string());
            }
            Ok(prop_oneof![4 => generated, 1 => proptest::sample::select(witnesses)].boxed())
        }
    }
}
