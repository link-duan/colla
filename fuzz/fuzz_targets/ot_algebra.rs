#![no_main]

//! Structured OT algebra fuzzing.
//!
//! Instead of feeding raw bytes to the decoders, this target interprets the
//! input as an `arbitrary` stream that builds a valid base [`Value`] and one or
//! more [`Change`]s that are compatible with it by construction. It then drives
//! the untrusted bytes through the semantic core and asserts the OT laws:
//!
//! - canonical codec round-trips for the generated value and changes;
//! - the invert law: `apply(apply(v, a), invert(a, v)) == v`;
//! - the compose law: `apply(v, compose(a, b)) == apply(apply(v, a), b)`;
//! - TP1 convergence for two concurrent changes over the same base.
//!
//! Assertions on the deeper laws are gated on the relevant `apply` succeeding,
//! so a rejected precondition simply ends the run instead of failing it. That
//! keeps every failure a genuine engine invariant violation.

use arbitrary::{Arbitrary, Unstructured};
use colla::codec::{decode_change, decode_value, encode_change, encode_value};
use colla::{
    apply, compose, invert, transform_pair, Change, IntChange, ListChange, ListOp, MapChange,
    MapEntryChange, TextChange, TextOp, TieBreak, Value, ValueKind,
};
use libfuzzer_sys::fuzz_target;

const MAX_DEPTH: u32 = 3;
const MAX_SEQ: usize = 6;
const MAX_SEGMENTS: usize = 3;

fn gen_value(u: &mut Unstructured, depth: u32) -> arbitrary::Result<Value> {
    let leaf_only = depth == 0 || u.is_empty();
    let variants: u32 = if leaf_only { 6 } else { 8 };
    Ok(match u.int_in_range(0..=variants - 1)? {
        0 => Value::null(),
        1 => Value::bool(bool::arbitrary(u)?),
        2 => Value::int(i64::arbitrary(u)?),
        3 => Value::float(finite_f64(u)?).unwrap_or_else(|_| Value::float(0.0).unwrap()),
        4 => Value::string(gen_string(u)?),
        5 => Value::text(gen_string(u)?),
        6 => {
            let len = u.int_in_range(0..=MAX_SEQ)?;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(gen_value(u, depth - 1)?);
            }
            Value::list(items)
        }
        _ => {
            let len = u.int_in_range(0..=MAX_SEQ)?;
            let mut entries = Vec::with_capacity(len);
            let mut keys = std::collections::BTreeSet::new();
            for _ in 0..len {
                let key = gen_key(u)?;
                if keys.insert(key.clone()) {
                    entries.push((key, gen_value(u, depth - 1)?));
                }
            }
            Value::map(entries).expect("unique keys by construction")
        }
    })
}

fn finite_f64(u: &mut Unstructured) -> arbitrary::Result<f64> {
    let raw = f64::arbitrary(u)?;
    Ok(if raw.is_finite() { raw } else { 0.0 })
}

fn gen_string(u: &mut Unstructured) -> arbitrary::Result<String> {
    let len = u.int_in_range(0..=6)?;
    let mut out = String::new();
    for _ in 0..len {
        let ch = match u.int_in_range(0..=3)? {
            0 => 'a',
            1 => 'b',
            2 => 'c',
            _ => '😭',
        };
        out.push(ch);
    }
    Ok(out)
}

fn gen_key(u: &mut Unstructured) -> arbitrary::Result<String> {
    Ok(match u.int_in_range(0..=3)? {
        0 => "k0",
        1 => "k1",
        2 => "k2",
        _ => "k3",
    }
    .to_string())
}

fn gen_change(u: &mut Unstructured, value: &Value, depth: u32) -> arbitrary::Result<Change> {
    if u.is_empty() {
        return Ok(Change::noop());
    }
    // Reserve a slice of the choice space for the structure-agnostic changes so
    // every value kind can still be replaced wholesale or left untouched.
    match u.int_in_range(0..=9)? {
        0 => return Ok(Change::noop()),
        1 => return Ok(Change::replace(gen_value(u, depth.min(2))?)),
        _ => {}
    }

    Ok(match value.kind() {
        ValueKind::Int(_) => {
            let delta = i64::arbitrary(u)?;
            // Bound the delta so the checked add mostly stays in range; genuine
            // overflow is still surfaced by gating the apply below.
            Change::from(IntChange::Add(delta % 1024))
        }
        ValueKind::Text(text) => gen_text_change(u, text.len())?,
        ValueKind::List(list) => gen_list_change(u, list.as_slice(), depth)?,
        ValueKind::Map(map) => gen_map_change(u, map, depth)?,
        // Null, Bool, Float, String and RichText have no in-place operation used
        // here other than a whole-value replacement, already covered above.
        _ => Change::noop(),
    })
}

fn gen_text_change(u: &mut Unstructured, len: usize) -> arbitrary::Result<Change> {
    let mut ops = Vec::new();
    let mut remaining = len;
    let segments = u.int_in_range(0..=MAX_SEGMENTS)?;
    for _ in 0..segments {
        if u.is_empty() {
            break;
        }
        let retain = u.int_in_range(0..=remaining)?;
        ops.push(TextOp::Retain(retain));
        remaining -= retain;
        let delete = u.int_in_range(0..=remaining)?;
        ops.push(TextOp::Delete(delete));
        remaining -= delete;
        ops.push(TextOp::Insert(gen_string(u)?));
    }
    ops.push(TextOp::Retain(remaining));
    Ok(TextChange::from_ops(ops)
        .expect("text ops are length-consistent")
        .into())
}

fn gen_list_change(u: &mut Unstructured, items: &[Value], depth: u32) -> arbitrary::Result<Change> {
    let mut ops = Vec::new();
    let mut index = 0usize;
    let n = items.len();
    let segments = u.int_in_range(0..=MAX_SEGMENTS)?;
    for _ in 0..segments {
        if u.is_empty() {
            break;
        }
        let retain = u.int_in_range(0..=(n - index))?;
        ops.push(ListOp::Retain(retain));
        index += retain;
        match u.int_in_range(0..=2)? {
            0 => {
                let delete = u.int_in_range(0..=(n - index))?;
                ops.push(ListOp::Delete(delete));
                index += delete;
            }
            1 => {
                let count = u.int_in_range(0..=2)?;
                let mut inserted = Vec::with_capacity(count);
                for _ in 0..count {
                    inserted.push(gen_value(u, depth.saturating_sub(1))?);
                }
                ops.push(ListOp::Insert(inserted));
            }
            _ => {
                if index < n {
                    let child = gen_change(u, &items[index], depth.saturating_sub(1))?;
                    ops.push(ListOp::Modify(child));
                    index += 1;
                }
            }
        }
    }
    ops.push(ListOp::Retain(n - index));
    Ok(ListChange::from_ops(ops)
        .expect("list ops are length-consistent")
        .into())
}

fn gen_map_change(u: &mut Unstructured, map: &colla::Map, depth: u32) -> arbitrary::Result<Change> {
    let mut entries = Vec::new();
    for (key, child_value) in map.iter() {
        if u.is_empty() {
            break;
        }
        match u.int_in_range(0..=2)? {
            0 => {}
            1 => entries.push((key.clone(), MapEntryChange::Delete)),
            _ => {
                let child = gen_change(u, child_value, depth.saturating_sub(1))?;
                entries.push((key.clone(), MapEntryChange::Modify(child)));
            }
        }
    }
    if !u.is_empty() && bool::arbitrary(u)? {
        let key = gen_key(u)?;
        if map.get(&key).is_none() {
            let inserted = gen_value(u, depth.saturating_sub(1))?;
            entries.push((key, MapEntryChange::Insert(inserted)));
        }
    }
    Ok(MapChange::from_entries(entries)
        .expect("map keys are unique by construction")
        .into())
}

fn assert_codec_roundtrips(value: &Value, change: &Change) {
    let value_bytes = encode_value(value);
    assert_eq!(
        &decode_value(&value_bytes).expect("value re-decode"),
        value,
        "value codec round-trip",
    );
    let change_bytes = encode_change(change);
    assert_eq!(
        &decode_change(&change_bytes).expect("change re-decode"),
        change,
        "change codec round-trip",
    );
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    let Ok(base) = gen_value(&mut u, MAX_DEPTH) else {
        return;
    };
    let Ok(a) = gen_change(&mut u, &base, MAX_DEPTH) else {
        return;
    };
    assert_codec_roundtrips(&base, &a);

    let Ok(after_a) = apply(&base, &a) else {
        return;
    };

    // Invert law.
    if let Ok(inverse) = invert(&a, &base) {
        let restored = apply(&after_a, &inverse).expect("inverse must apply to the result");
        assert_eq!(restored, base, "apply(apply(v, a), invert(a, v)) == v");
    }

    // Compose law: a then b, sequenced over the same base.
    if let Ok(b) = gen_change(&mut u, &after_a, MAX_DEPTH) {
        if let Ok(after_b) = apply(&after_a, &b) {
            let combined = compose(&a, &b).expect("compose of two applicable changes");
            let composed = apply(&base, &combined).expect("composed change must apply to the base");
            assert_eq!(composed, after_b, "apply(v, compose(a, b)) == apply(apply(v, a), b)");
        }
    }

    // TP1 convergence for two concurrent changes over the same base.
    if let Ok(c) = gen_change(&mut u, &base, MAX_DEPTH) {
        if let Ok(after_c) = apply(&base, &c) {
            if let Ok((a_prime, c_prime)) = transform_pair(&a, &c, TieBreak::LeftFirst) {
                let left = apply(&after_a, &c_prime);
                let right = apply(&after_c, &a_prime);
                if let (Ok(left), Ok(right)) = (left, right) {
                    assert_eq!(left, right, "TP1: concurrent changes converge");
                }
            }
        }
    }
});
