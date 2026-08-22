//! Golden tests for the `colla` reference implementation.
//!
//! Walks the shared, language-neutral fixtures under `golden/` and
//! checks the Rust public API against their fixed expected output, canonical
//! bytes, and stable error codes. The same fixtures drive the JavaScript facade
//! test in `packages/core/tests/golden.test.mjs`, so both sides are pinned to
//! one data source. Unknown fixture kinds are a hard error so a case can never
//! be silently skipped. RichText neutral conversion is not implemented yet.

use std::path::{Path, PathBuf};

use colla::{
    apply, compose, invert, transform_pair, Change, IntChange, ListChange, ListOp, MapChange,
    MapEntryChange, TextChange, TextOp, TieBreak, Value,
};
use serde_json::Value as Json;

#[test]
fn golden_matches() {
    let root = fixtures_dir();
    let files = fixture_files(&root);
    assert!(
        !files.is_empty(),
        "no fixtures found under {}",
        root.display()
    );

    let mut failures = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        let fixture: Json = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{}: invalid JSON: {e}", path.display()));

        // `id` and `kind` are derived from the file path, not stored in the
        // fixture: the path under `golden/` is the id, and its first segment is
        // the kind (e.g. `apply/map-modify` -> id `apply/map-modify`, kind `apply`).
        let id = fixture_id(&root, path);
        let kind = id.split('/').next().unwrap();

        if let Err(reason) = run_fixture(&id, kind, &fixture) {
            failures.push(format!("{id}: {reason}"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} golden fixture failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn fixture_id(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap()
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/")
}

/// Absolute path to the shared `golden/` fixtures directory.
fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../golden")
}

/// Every `*.json` fixture under `root`, sorted for deterministic ordering.
fn fixture_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_json(root, &mut files);
    files.sort();
    files
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read fixtures directory {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_json(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }
}

/// Runs one fixture, dispatching by `kind`. Returns `Err` on a golden fixture
/// mismatch and panics on a malformed fixture (unknown kind, missing field).
fn run_fixture(id: &str, kind: &str, fixture: &Json) -> Result<(), String> {
    match kind {
        "value-codec" => run_value_codec(fixture),
        "decode-error" => run_decode_error(fixture),
        "apply" => run_apply(fixture),
        "compose" => run_compose(fixture),
        "invert" => run_invert(fixture),
        "transform" => run_transform(fixture),
        other => panic!("{id}: unknown fixture kind: {other}"),
    }
}

// --- Fixture kinds ----------------------------------------------------------

fn run_value_codec(fixture: &Json) -> Result<(), String> {
    let value = value_from_neutral(fixture.get("value").ok_or("missing `value`")?)?;
    let expected = require_hex(fixture, "canonicalBytes")?;

    let encoded = value.encode();
    if encoded != expected {
        return Err(format!(
            "encode mismatch: expected {}, got {}",
            to_hex(&expected),
            to_hex(&encoded)
        ));
    }
    let decoded = Value::decode(&expected).map_err(|e| format!("decode failed: {e}"))?;
    if decoded != value {
        return Err("decoded value does not structurally equal the fixture value".into());
    }
    if decoded.encode() != expected {
        return Err("re-encoding the decoded value is not canonical".into());
    }
    Ok(())
}

fn run_decode_error(fixture: &Json) -> Result<(), String> {
    let target = fixture
        .get("target")
        .and_then(Json::as_str)
        .ok_or("missing `target`")?;
    let bytes = require_hex(fixture, "inputBytes")?;
    let code = match target {
        "value" => match Value::decode(&bytes) {
            Ok(_) => return Err("expected a decode error, but decoding succeeded".into()),
            Err(error) => error.code(),
        },
        "change" => match Change::decode(&bytes) {
            Ok(_) => return Err("expected a decode error, but decoding succeeded".into()),
            Err(error) => error.code(),
        },
        other => return Err(format!("unknown decode target: {other}")),
    };
    check_error_code(fixture, code.as_str())
}

fn run_apply(fixture: &Json) -> Result<(), String> {
    let snapshot = value_from_neutral(require(fixture, "snapshot")?)?;
    let change = change_from_neutral(require(fixture, "change")?)?;
    if let Some(hex) = fixture.get("changeBytes").and_then(Json::as_str) {
        let expected = decode_hex(hex)?;
        if change.encode() != expected {
            return Err(format!(
                "changeBytes mismatch: expected {hex}, got {}",
                to_hex(&change.encode())
            ));
        }
    }
    match apply(&snapshot, &change) {
        Ok(result) => {
            reject_unexpected_success(fixture)?;
            let expected = value_from_neutral(require_path(fixture, &["expect", "value"])?)?;
            if result != expected {
                return Err("apply result differs from expect.value".into());
            }
            Ok(())
        }
        Err(error) => check_error_code(fixture, error.code().as_str()),
    }
}

fn run_compose(fixture: &Json) -> Result<(), String> {
    let changes = require(fixture, "changes")?
        .as_array()
        .ok_or("`changes` must be an array")?;
    if changes.len() != 2 {
        return Err(format!(
            "compose needs exactly two changes, got {}",
            changes.len()
        ));
    }
    let first = change_from_neutral(&changes[0])?;
    let second = change_from_neutral(&changes[1])?;
    match compose(&first, &second) {
        Ok(composed) => {
            reject_unexpected_success(fixture)?;
            let expected = change_from_neutral(require_path(fixture, &["expect", "change"])?)?;
            assert_same_change("composed", &composed, &expected)?;
            if let Some(base) = fixture.get("snapshot") {
                let base = value_from_neutral(base)?;
                let via_composed =
                    apply(&base, &composed).map_err(|e| format!("apply(base, composed): {e}"))?;
                let sequential = apply(&base, &first)
                    .and_then(|mid| apply(&mid, &second))
                    .map_err(|e| format!("sequential apply: {e}"))?;
                if via_composed != sequential {
                    return Err("compose convergence check failed".into());
                }
            }
            Ok(())
        }
        Err(error) => check_error_code(fixture, error.code().as_str()),
    }
}

fn run_invert(fixture: &Json) -> Result<(), String> {
    let snapshot = value_from_neutral(require(fixture, "snapshot")?)?;
    let change = change_from_neutral(require(fixture, "change")?)?;
    let inverse = invert(&change, &snapshot).map_err(|e| format!("invert failed: {e}"))?;
    let expected = change_from_neutral(require_path(fixture, &["expect", "change"])?)?;
    assert_same_change("inverse", &inverse, &expected)?;

    let applied = apply(&snapshot, &change).map_err(|e| format!("apply(change): {e}"))?;
    let restored = apply(&applied, &inverse).map_err(|e| format!("apply(inverse): {e}"))?;
    if restored != snapshot {
        return Err("invert round-trip did not restore the snapshot".into());
    }
    Ok(())
}

fn run_transform(fixture: &Json) -> Result<(), String> {
    let change_a = change_from_neutral(require(fixture, "changeA")?)?;
    let change_b = change_from_neutral(require(fixture, "changeB")?)?;
    let tie = match require(fixture, "side")?.as_str() {
        Some("left") => TieBreak::LeftFirst,
        Some("right") => TieBreak::RightFirst,
        Some(other) => return Err(format!("unknown tie-break side: {other}")),
        None => return Err("`side` must be a string".into()),
    };
    match transform_pair(&change_a, &change_b, tie) {
        Ok((a_prime, b_prime)) => {
            reject_unexpected_success(fixture)?;
            let expected_a = change_from_neutral(require_path(fixture, &["expect", "aPrime"])?)?;
            let expected_b = change_from_neutral(require_path(fixture, &["expect", "bPrime"])?)?;
            assert_same_change("aPrime", &a_prime, &expected_a)?;
            assert_same_change("bPrime", &b_prime, &expected_b)?;
            if let Some(base) = fixture.get("base") {
                let base = value_from_neutral(base)?;
                let left_path = apply(&base, &change_a)
                    .and_then(|v| apply(&v, &b_prime))
                    .map_err(|e| format!("left convergence apply: {e}"))?;
                let right_path = apply(&base, &change_b)
                    .and_then(|v| apply(&v, &a_prime))
                    .map_err(|e| format!("right convergence apply: {e}"))?;
                if left_path != right_path {
                    return Err("transform convergence check failed".into());
                }
            }
            Ok(())
        }
        Err(error) => check_error_code(fixture, error.code().as_str()),
    }
}

// --- Neutral conversion -----------------------------------------------------

/// Converts a neutral tagged-JSON Value into a `colla::Value`.
fn value_from_neutral(json: &Json) -> Result<Value, String> {
    let (tag, body) = single_tag(json, "value")?;
    match tag {
        "null" => Ok(Value::null()),
        "bool" => Ok(Value::bool(
            body.as_bool().ok_or("`bool` needs a JSON boolean")?,
        )),
        "int" => Ok(Value::int(parse_int(body)?)),
        "float" => {
            let value = body.as_f64().ok_or("`float` needs a JSON number")?;
            Value::float(value).map_err(|e| e.to_string())
        }
        "string" => Ok(Value::string(
            body.as_str().ok_or("`string` needs a JSON string")?,
        )),
        "text" => Ok(Value::text(
            body.as_str().ok_or("`text` needs a JSON string")?,
        )),
        "list" => {
            let array = body.as_array().ok_or("`list` needs a JSON array")?;
            let mut values = Vec::with_capacity(array.len());
            for item in array {
                values.push(value_from_neutral(item)?);
            }
            Ok(Value::list(values))
        }
        "map" => {
            let object = body.as_object().ok_or("`map` needs a JSON object")?;
            let mut entries = Vec::with_capacity(object.len());
            for (key, item) in object {
                entries.push((key.clone(), value_from_neutral(item)?));
            }
            Value::map(entries).map_err(|e| e.to_string())
        }
        "richtext" => Err("`richtext` neutral conversion is not implemented yet".into()),
        other => Err(format!("unknown value tag: {other}")),
    }
}

/// Converts a neutral tagged-JSON Change into a `colla::Change`.
fn change_from_neutral(json: &Json) -> Result<Change, String> {
    let (tag, body) = single_tag(json, "change")?;
    match tag {
        "noop" => Ok(Change::noop()),
        "replace" => Ok(Change::replace(value_from_neutral(body)?)),
        "int" => {
            let add = body.get("add").ok_or("`int` change needs an `add` field")?;
            Ok(IntChange::Add(parse_int(add)?).into())
        }
        "map" => {
            let object = body.as_object().ok_or("`map` change needs a JSON object")?;
            let mut entries = Vec::with_capacity(object.len());
            for (key, entry) in object {
                entries.push((key.clone(), map_entry_from_neutral(entry)?));
            }
            Ok(MapChange::from_entries(entries)
                .map_err(|e| e.to_string())?
                .into())
        }
        "list" => {
            let array = body.as_array().ok_or("`list` change needs a JSON array")?;
            let mut ops = Vec::with_capacity(array.len());
            for op in array {
                ops.push(list_op_from_neutral(op)?);
            }
            Ok(ListChange::from_ops(ops).map_err(|e| e.to_string())?.into())
        }
        "text" => {
            let array = body.as_array().ok_or("`text` change needs a JSON array")?;
            let mut ops = Vec::with_capacity(array.len());
            for op in array {
                ops.push(text_op_from_neutral(op)?);
            }
            Ok(TextChange::from_ops(ops).map_err(|e| e.to_string())?.into())
        }
        "richtext" => Err("`richtext` neutral conversion is not implemented yet".into()),
        other => Err(format!("unknown change tag: {other}")),
    }
}

fn map_entry_from_neutral(json: &Json) -> Result<MapEntryChange, String> {
    let (tag, body) = single_tag(json, "map entry change")?;
    match tag {
        "insert" => Ok(MapEntryChange::Insert(value_from_neutral(body)?)),
        "delete" => Ok(MapEntryChange::Delete),
        "modify" => Ok(MapEntryChange::Modify(change_from_neutral(body)?)),
        other => Err(format!("unknown map entry change tag: {other}")),
    }
}

fn list_op_from_neutral(json: &Json) -> Result<ListOp, String> {
    let (tag, body) = single_tag(json, "list op")?;
    match tag {
        "retain" => Ok(ListOp::Retain(parse_len(body)?)),
        "delete" => Ok(ListOp::Delete(parse_len(body)?)),
        "insert" => {
            let array = body.as_array().ok_or("`insert` needs a JSON array")?;
            let mut values = Vec::with_capacity(array.len());
            for item in array {
                values.push(value_from_neutral(item)?);
            }
            Ok(ListOp::Insert(values))
        }
        "modify" => Ok(ListOp::Modify(change_from_neutral(body)?)),
        other => Err(format!("unknown list op tag: {other}")),
    }
}

fn text_op_from_neutral(json: &Json) -> Result<TextOp, String> {
    let (tag, body) = single_tag(json, "text op")?;
    match tag {
        "retain" => Ok(TextOp::Retain(parse_len(body)?)),
        "delete" => Ok(TextOp::Delete(parse_len(body)?)),
        "insert" => Ok(TextOp::Insert(
            body.as_str()
                .ok_or("`insert` needs a JSON string")?
                .to_owned(),
        )),
        other => Err(format!("unknown text op tag: {other}")),
    }
}

// --- Assertions & helpers ---------------------------------------------------

fn assert_same_change(label: &str, actual: &Change, expected: &Change) -> Result<(), String> {
    if actual.encode() != expected.encode() {
        return Err(format!(
            "{label} canonical bytes differ: expected {}, got {}",
            to_hex(&expected.encode()),
            to_hex(&actual.encode())
        ));
    }
    Ok(())
}

fn check_error_code(fixture: &Json, actual: &str) -> Result<(), String> {
    let expected = fixture
        .get("expectError")
        .and_then(|error| error.get("code"))
        .and_then(Json::as_str)
        .ok_or("operation failed but the fixture has no expectError.code")?;
    if actual != expected {
        return Err(format!(
            "error code mismatch: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

fn reject_unexpected_success(fixture: &Json) -> Result<(), String> {
    if fixture.get("expectError").is_some() {
        return Err("expected an error but the operation succeeded".into());
    }
    Ok(())
}

fn single_tag<'a>(json: &'a Json, what: &str) -> Result<(&'a str, &'a Json), String> {
    let object = json
        .as_object()
        .ok_or_else(|| format!("{what} must be a single-key tagged object"))?;
    if object.len() != 1 {
        return Err(format!(
            "{what} object must have exactly one tag, found {}",
            object.len()
        ));
    }
    let (tag, body) = object.iter().next().unwrap();
    Ok((tag.as_str(), body))
}

fn require<'a>(fixture: &'a Json, field: &str) -> Result<&'a Json, String> {
    fixture.get(field).ok_or(format!("missing `{field}`"))
}

fn require_path<'a>(fixture: &'a Json, path: &[&str]) -> Result<&'a Json, String> {
    let mut node = fixture;
    for key in path {
        node = node
            .get(key)
            .ok_or(format!("missing `{}`", path.join(".")))?;
    }
    Ok(node)
}

fn require_hex(fixture: &Json, field: &str) -> Result<Vec<u8>, String> {
    let hex = fixture
        .get(field)
        .and_then(Json::as_str)
        .ok_or(format!("missing `{field}`"))?;
    decode_hex(hex)
}

fn parse_int(json: &Json) -> Result<i64, String> {
    let text = json.as_str().ok_or("int needs a decimal string")?;
    text.parse::<i64>()
        .map_err(|_| format!("invalid int: {text}"))
}

fn parse_len(json: &Json) -> Result<usize, String> {
    let value = json
        .as_u64()
        .ok_or("length must be a non-negative integer")?;
    usize::try_from(value).map_err(|_| format!("length out of range: {value}"))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("hex string has an odd length".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| format!("invalid hex: {hex}")))
        .collect()
}
