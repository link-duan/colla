//! Private WebAssembly bindings for `colla`.
//!
//! Generated names and error payloads in this crate are an implementation
//! detail of the handwritten `colla-ot` facade.

mod change_input;

use change_input::{decode_change_input, ChangeInputError};
use colla::{
    apply, compose, invert, transform_pair, ApplyError, AttrChange, AttrPatch, AttrValue, Attrs,
    Change, ChangeKind, CodecError, ComposeError, InputLimits, IntChange, InvertError, ListOp,
    MapEntryChange, Path, PathSeg, RichContent, RichText, RichTextOp, TextOp, TieBreak,
    TransformError, Utf16PositionError, Value, ValueError, ValueKind, ValueType,
};
use serde_json::{json, Value as JsonValue};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ValueHandle {
    value: Value,
}

#[wasm_bindgen]
impl ValueHandle {
    #[wasm_bindgen(js_name = null)]
    pub fn null_value() -> Self {
        Self {
            value: Value::null(),
        }
    }

    #[wasm_bindgen(js_name = bool)]
    pub fn bool_value(value: bool) -> Self {
        Self {
            value: Value::bool(value),
        }
    }

    #[wasm_bindgen(js_name = int)]
    pub fn int_value(value: i64) -> Self {
        Self {
            value: Value::int(value),
        }
    }

    #[wasm_bindgen(js_name = float)]
    pub fn float_value(value: f64) -> Result<Self, JsValue> {
        Value::float(value)
            .map(|value| Self { value })
            .map_err(|error| wasm_error("invalid_value", "value_from_js", error.to_string()))
    }

    #[wasm_bindgen(js_name = string)]
    pub fn string_value(value: &str) -> Self {
        Self {
            value: Value::string(value),
        }
    }

    #[wasm_bindgen(js_name = decode)]
    pub fn decode(bytes: &[u8], limits: &str) -> Result<Self, JsValue> {
        Value::decode_with_limits(bytes, &parse_limits(limits, "value_decode")?)
            .map(|value| Self { value })
            .map_err(|error| codec_error(error, "value_decode"))
    }

    pub fn encode(&self) -> Vec<u8> {
        self.value.encode()
    }

    pub fn kind(&self, path: &str) -> Result<String, JsValue> {
        let path = parse_path(path, "value_kind")?;
        Ok(value_kind_name(value_at_path(&self.value, &path, "value_kind")?.kind()).into())
    }

    pub fn has(&self, path: &str) -> Result<bool, JsValue> {
        let path = parse_path(path, "value_has")?;
        Ok(self.value.get(&path).is_some())
    }

    #[wasm_bindgen(js_name = getBytes)]
    pub fn get_bytes(&self, path: &str) -> Result<Vec<u8>, JsValue> {
        let path = parse_path(path, "value_get")?;
        Ok(value_at_path(&self.value, &path, "value_get")?.encode())
    }

    #[wasm_bindgen(js_name = boolValue)]
    pub fn bool_data(&self) -> Result<bool, JsValue> {
        match self.value.kind() {
            ValueKind::Bool(value) => Ok(*value),
            actual => Err(type_error("bool", actual)),
        }
    }

    #[wasm_bindgen(js_name = intValue)]
    pub fn int_data(&self) -> Result<i64, JsValue> {
        self.value
            .as_int()
            .ok_or_else(|| type_error("int", self.value.kind()))
    }

    #[wasm_bindgen(js_name = floatValue)]
    pub fn float_data(&self) -> Result<f64, JsValue> {
        match self.value.kind() {
            ValueKind::Float(value) => Ok(value.get()),
            actual => Err(type_error("float", actual)),
        }
    }

    #[wasm_bindgen(js_name = stringValue)]
    pub fn string_data(&self) -> Result<String, JsValue> {
        match self.value.kind() {
            ValueKind::String(value) => Ok(value.to_string()),
            actual => Err(type_error("string", actual)),
        }
    }

    #[wasm_bindgen(js_name = cloneHandle)]
    pub fn clone_handle(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

#[wasm_bindgen]
pub struct ChangeHandle {
    change: Change,
}

#[wasm_bindgen]
impl ChangeHandle {
    #[wasm_bindgen(js_name = fromInput)]
    pub fn from_input(bytes: &[u8], limits: &str) -> Result<Self, JsValue> {
        let limits = parse_limits(limits, "change_from_js")?;
        decode_change_input(bytes, &limits)
            .map(|change| Self { change })
            .map_err(change_input_error)
    }

    #[wasm_bindgen(js_name = decode)]
    pub fn decode(bytes: &[u8], limits: &str) -> Result<Self, JsValue> {
        Change::decode_with_limits(bytes, &parse_limits(limits, "change_decode")?)
            .map(|change| Self { change })
            .map_err(|error| codec_error(error, "change_decode"))
    }

    pub fn encode(&self) -> Vec<u8> {
        self.change.encode()
    }

    #[wasm_bindgen(js_name = cloneHandle)]
    pub fn clone_handle(&self) -> Self {
        Self {
            change: self.change.clone(),
        }
    }
}

#[wasm_bindgen(js_name = applyHandles)]
pub fn apply_handles(base: &ValueHandle, change: &ChangeHandle) -> Result<ValueHandle, JsValue> {
    apply(&base.value, &change.change)
        .map(|value| ValueHandle { value })
        .map_err(|error| apply_error(error, "apply"))
}

#[wasm_bindgen(js_name = composeHandles)]
pub fn compose_handles(
    first: &ChangeHandle,
    second: &ChangeHandle,
) -> Result<ChangeHandle, JsValue> {
    compose(&first.change, &second.change)
        .map(|change| ChangeHandle { change })
        .map_err(|error| compose_error(error, "compose"))
}

#[wasm_bindgen(js_name = invertHandle)]
pub fn invert_handle(change: &ChangeHandle, base: &ValueHandle) -> Result<ChangeHandle, JsValue> {
    invert(&change.change, &base.value)
        .map(|change| ChangeHandle { change })
        .map_err(|error| invert_error(error, "invert"))
}

#[wasm_bindgen]
pub struct TransformPairHandle {
    left: Change,
    right: Change,
}

#[wasm_bindgen]
impl TransformPairHandle {
    #[wasm_bindgen(js_name = leftHandle)]
    pub fn left_handle(&self) -> ChangeHandle {
        ChangeHandle {
            change: self.left.clone(),
        }
    }

    #[wasm_bindgen(js_name = rightHandle)]
    pub fn right_handle(&self) -> ChangeHandle {
        ChangeHandle {
            change: self.right.clone(),
        }
    }
}

#[wasm_bindgen(js_name = transformPairHandles)]
pub fn transform_pair_handles(
    left: &ChangeHandle,
    right: &ChangeHandle,
    left_first: bool,
) -> Result<TransformPairHandle, JsValue> {
    let order = if left_first {
        TieBreak::LeftFirst
    } else {
        TieBreak::RightFirst
    };
    transform_pair(&left.change, &right.change, order)
        .map(|(left, right)| TransformPairHandle { left, right })
        .map_err(|error| transform_error(error, "transform_pair"))
}

#[wasm_bindgen(js_name = inspectChangeHandle)]
pub fn inspect_change_handle(change: &ChangeHandle, base: &ValueHandle) -> Result<String, JsValue> {
    apply(&base.value, &change.change).map_err(|error| apply_error(error, "inspect_change"))?;
    let mut entries = Vec::new();
    let mut path = Path::new();
    inspect_change_at(&change.change, &base.value, &mut path, &mut entries)?;
    serde_json::to_string(&entries)
        .map_err(|error| wasm_error("invalid_value", "inspect_change", error.to_string()))
}

fn inspect_change_at(
    change: &Change,
    base: &Value,
    path: &mut Path,
    entries: &mut Vec<JsonValue>,
) -> Result<(), JsValue> {
    match change.kind() {
        ChangeKind::Noop => {}
        ChangeKind::Replace(value) => entries.push(json!({
            "type": "value.replace",
            "path": path_json_value(path),
            "valueBytes": value.encode(),
        })),
        ChangeKind::Int(IntChange::Add(delta)) => entries.push(json!({
            "type": "int.add",
            "path": path_json_value(path),
            "delta": delta.to_string(),
        })),
        ChangeKind::Map(change) => {
            let ValueKind::Map(base) = base.kind() else {
                return Err(inspect_type_error(path, ValueType::Map, base));
            };
            for (key, entry) in change.iter() {
                match entry {
                    MapEntryChange::Insert(value) => entries.push(json!({
                        "type": "map.set",
                        "path": path_json_value(path),
                        "key": key,
                        "valueBytes": value.encode(),
                    })),
                    MapEntryChange::Delete => entries.push(json!({
                        "type": "map.delete",
                        "path": path_json_value(path),
                        "key": key,
                    })),
                    MapEntryChange::Modify(child) => {
                        let child_base = base.get(key).ok_or_else(|| {
                            wasm_error_details(
                                "missing_key",
                                "inspect_change",
                                json!({ "key": key }),
                            )
                        })?;
                        if let ChangeKind::Replace(value) = child.kind() {
                            entries.push(json!({
                                "type": "map.set",
                                "path": path_json_value(path),
                                "key": key,
                                "valueBytes": value.encode(),
                            }));
                        } else {
                            path.push(PathSeg::Key(key.clone()));
                            inspect_change_at(child, child_base, path, entries)?;
                            path.pop();
                        }
                    }
                }
            }
        }
        ChangeKind::List(change) => {
            let ValueKind::List(base) = base.kind() else {
                return Err(inspect_type_error(path, ValueType::List, base));
            };
            let mut index = 0usize;
            for op in change.ops() {
                match op {
                    ListOp::Retain(len) => index += len,
                    ListOp::Insert(values) => entries.push(json!({
                        "type": "list.insert",
                        "path": path_json_value(path),
                        "index": index,
                        "valuesBytes": values.iter().map(Value::encode).collect::<Vec<_>>(),
                    })),
                    ListOp::Delete(len) => {
                        let end = index + len;
                        entries.push(json!({
                            "type": "list.delete",
                            "path": path_json_value(path),
                            "from": index,
                            "to": end,
                        }));
                        index = end;
                    }
                    ListOp::Modify(child) => {
                        let child_base = base.get(index).ok_or_else(|| {
                            wasm_error_details(
                                "out_of_bounds",
                                "inspect_change",
                                json!({ "target": "list", "length": base.len(), "index": index }),
                            )
                        })?;
                        if let ChangeKind::Replace(value) = child.kind() {
                            entries.push(json!({
                                "type": "list.set",
                                "path": path_json_value(path),
                                "index": index,
                                "valueBytes": value.encode(),
                            }));
                        } else {
                            path.push(PathSeg::Index(index));
                            inspect_change_at(child, child_base, path, entries)?;
                            path.pop();
                        }
                        index += 1;
                    }
                }
            }
        }
        ChangeKind::Text(change) => {
            let ValueKind::Text(base) = base.kind() else {
                return Err(inspect_type_error(path, ValueType::Text, base));
            };
            let mut index = 0usize;
            for op in change.ops() {
                match op {
                    TextOp::Retain(len) => index += len,
                    TextOp::Insert(text) => entries.push(json!({
                        "type": "text.insert",
                        "path": path_json_value(path),
                        "at": text_utf16_prefix(base.as_str(), index),
                        "text": text,
                    })),
                    TextOp::Delete(len) => {
                        let end = index + len;
                        entries.push(json!({
                            "type": "text.delete",
                            "path": path_json_value(path),
                            "from": text_utf16_prefix(base.as_str(), index),
                            "to": text_utf16_prefix(base.as_str(), end),
                        }));
                        index = end;
                    }
                }
            }
        }
        ChangeKind::RichText(change) => {
            let ValueKind::RichText(base) = base.kind() else {
                return Err(inspect_type_error(path, ValueType::RichText, base));
            };
            let mut index = 0usize;
            for op in change.ops() {
                match op {
                    RichTextOp::Retain { len, attrs } => {
                        let end = index + len;
                        if !attrs.is_empty() {
                            entries.push(json!({
                                "type": "richText.format",
                                "path": path_json_value(path),
                                "from": rich_utf16_prefix(base, index),
                                "to": rich_utf16_prefix(base, end),
                                "patch": attr_patch_json(attrs),
                            }));
                        }
                        index = end;
                    }
                    RichTextOp::Insert { content, attrs } => match content {
                        RichContent::Text(text) => entries.push(json!({
                            "type": "richText.insertText",
                            "path": path_json_value(path),
                            "at": rich_utf16_prefix(base, index),
                            "text": text.as_str(),
                            "attrs": attrs_json(attrs),
                        })),
                        RichContent::Embed(value) => entries.push(json!({
                            "type": "richText.insertEmbed",
                            "path": path_json_value(path),
                            "at": rich_utf16_prefix(base, index),
                            "embedBytes": value.encode(),
                            "attrs": attrs_json(attrs),
                        })),
                    },
                    RichTextOp::Delete(len) => {
                        let end = index + len;
                        entries.push(json!({
                            "type": "richText.delete",
                            "path": path_json_value(path),
                            "from": rich_utf16_prefix(base, index),
                            "to": rich_utf16_prefix(base, end),
                        }));
                        index = end;
                    }
                }
            }
        }
    }
    Ok(())
}

#[wasm_bindgen(js_name = resolveCodePointPositionHandle)]
pub fn resolve_code_point_position_handle(
    value: &ValueHandle,
    path: &str,
    position: usize,
) -> Result<usize, JsValue> {
    let path = parse_path(path, "resolve_code_point_position")?;
    let value = value_at_path(&value.value, &path, "resolve_code_point_position")?;
    match value.kind() {
        ValueKind::Text(text) => {
            utf16_to_code_point(text.as_str(), position, "resolve_code_point_position")
        }
        ValueKind::RichText(rich) => {
            rich_utf16_to_code_point(rich, position, "resolve_code_point_position")
        }
        actual => Err(wasm_error_details(
            "type_mismatch",
            "resolve_code_point_position",
            json!({ "expected": ["text", "richText"], "actual": value_kind_name(actual) }),
        )),
    }
}

#[wasm_bindgen(js_name = resolveUtf16PositionHandle)]
pub fn resolve_utf16_position_handle(
    value: &ValueHandle,
    path: &str,
    position: usize,
) -> Result<usize, JsValue> {
    let path = parse_path(path, "resolve_utf16_position")?;
    let value = value_at_path(&value.value, &path, "resolve_utf16_position")?;
    match value.kind() {
        ValueKind::Text(text) => {
            code_point_to_utf16(text.as_str(), position, "resolve_utf16_position")
        }
        ValueKind::RichText(rich) => {
            rich_code_point_to_utf16(rich, position, "resolve_utf16_position")
        }
        actual => Err(wasm_error_details(
            "type_mismatch",
            "resolve_utf16_position",
            json!({ "expected": ["text", "richText"], "actual": value_kind_name(actual) }),
        )),
    }
}

fn value_kind_name(kind: &ValueKind) -> &'static str {
    match kind {
        ValueKind::Null => "null",
        ValueKind::Bool(_) => "bool",
        ValueKind::Int(_) => "int",
        ValueKind::Float(_) => "float",
        ValueKind::String(_) => "string",
        ValueKind::Text(_) => "text",
        ValueKind::RichText(_) => "richText",
        ValueKind::List(_) => "list",
        ValueKind::Map(_) => "map",
    }
}

fn path_json_value(path: &Path) -> JsonValue {
    JsonValue::Array(
        path.segments()
            .iter()
            .map(|segment| match segment {
                PathSeg::Key(key) => json!(key),
                PathSeg::Index(index) => json!(index),
            })
            .collect(),
    )
}

fn inspect_type_error(path: &Path, expected: ValueType, actual: &Value) -> JsValue {
    wasm_error_details(
        "type_mismatch",
        "inspect_change",
        json!({
            "expected": value_type_name(expected),
            "actual": value_type_name(actual.value_type()),
            "path": path_json_value(path),
        }),
    )
}

fn text_utf16_prefix(text: &str, index: usize) -> usize {
    text.chars().take(index).map(char::len_utf16).sum()
}

fn rich_utf16_prefix(rich: &RichText, index: usize) -> usize {
    rich.code_point_to_utf16(index)
        .expect("validated RichText code point position")
}

fn attr_value_json(value: &AttrValue) -> JsonValue {
    match value {
        AttrValue::Bool(value) => json!({ "kind": "bool", "value": value }),
        AttrValue::Int(value) => json!({ "kind": "int", "value": value.to_string() }),
        AttrValue::Float(value) => json!({ "kind": "float", "value": value.get() }),
        AttrValue::String(value) => json!({ "kind": "string", "value": value.as_ref() }),
    }
}

fn attrs_json(attrs: &Attrs) -> JsonValue {
    JsonValue::Array(
        attrs
            .iter()
            .map(|(key, value)| {
                let mut entry = attr_value_json(value);
                entry["key"] = json!(key);
                entry
            })
            .collect(),
    )
}

fn attr_patch_json(patch: &AttrPatch) -> JsonValue {
    JsonValue::Array(
        patch
            .iter()
            .map(|(key, change)| match change {
                AttrChange::Set(value) => {
                    let mut entry = attr_value_json(value);
                    entry["key"] = json!(key);
                    entry["action"] = json!("set");
                    entry
                }
                AttrChange::Remove => json!({ "key": key, "action": "remove" }),
            })
            .collect(),
    )
}

fn type_error(expected: &'static str, actual: &ValueKind) -> JsValue {
    wasm_error_details(
        "type_mismatch",
        "value_to_js",
        json!({ "expected": expected, "actual": value_kind_name(actual) }),
    )
}

fn wasm_error(code: &'static str, operation: &'static str, reason: impl Into<String>) -> JsValue {
    wasm_error_details(code, operation, json!({ "reason": reason.into() }))
}

fn wasm_error_details(code: &'static str, operation: &'static str, details: JsonValue) -> JsValue {
    JsValue::from_str(
        &json!({
            "code": code,
            "operation": operation,
            "details": details,
        })
        .to_string(),
    )
}

fn codec_error(error: CodecError, operation: &'static str) -> JsValue {
    let code = error.code().as_str();
    match error {
        CodecError::LimitExceeded {
            name,
            actual,
            limit,
        } => wasm_error_details(
            code,
            operation,
            json!({ "limit": name, "actual": actual, "maximum": limit }),
        ),
        CodecError::Value(error) => wasm_error(code, operation, error.to_string()),
        other => {
            let offset = match &other {
                CodecError::UnexpectedEof { offset }
                | CodecError::UnknownTag { offset, .. }
                | CodecError::NonMinimalVarint { offset }
                | CodecError::IntegerOutOfRange { offset }
                | CodecError::InvalidUtf8 { offset }
                | CodecError::NonCanonical { offset, .. }
                | CodecError::TrailingBytes { offset } => Some(*offset),
                _ => None,
            };
            let mut details = json!({ "reason": other.to_string() });
            if let Some(offset) = offset {
                details["offset"] = json!(offset);
            }
            wasm_error_details(code, operation, details)
        }
    }
}

fn change_input_error(error: ChangeInputError) -> JsValue {
    match error {
        ChangeInputError::Encoding { offset, reason } => wasm_error_details(
            "invalid_argument",
            "change_from_js",
            json!({ "reason": reason, "offset": offset }),
        ),
        ChangeInputError::Limit {
            name,
            actual,
            maximum,
        } => wasm_error_details(
            "limit_exceeded",
            "change_from_js",
            json!({ "limit": name, "actual": actual, "maximum": maximum }),
        ),
        ChangeInputError::Value(ValueError::LengthOverflow) => wasm_error_details(
            "invalid_argument",
            "change_from_js",
            json!({ "reason": "length_overflow" }),
        ),
        ChangeInputError::Value(error) => {
            wasm_error("invalid_value", "change_from_js", error.to_string())
        }
        ChangeInputError::ValueCodec(CodecError::Value(error)) => {
            wasm_error("invalid_value", "change_from_js", error.to_string())
        }
        ChangeInputError::ValueCodec(error) => codec_error(error, "change_from_js"),
    }
}

fn compose_error(error: ComposeError, operation: &'static str) -> JsValue {
    let code = error.code().as_str();
    match error {
        ComposeError::Apply(error) => apply_error(error, operation),
        ComposeError::IncompatibleKinds { left, right } => wasm_error_details(
            code,
            operation,
            json!({ "reason": "kind_mismatch", "left": left, "right": right }),
        ),
        ComposeError::IncompatibleMapEntry(key) => wasm_error_details(
            code,
            operation,
            json!({ "reason": "map_entry_conflict", "key": key }),
        ),
        ComposeError::LengthOverflow => {
            wasm_error_details(code, operation, json!({ "reason": "length_overflow" }))
        }
        other => wasm_error(code, operation, other.to_string()),
    }
}

fn transform_error(error: TransformError, operation: &'static str) -> JsValue {
    let code = error.code().as_str();
    match error {
        TransformError::IncompatibleKinds { left, right } => wasm_error_details(
            code,
            operation,
            json!({ "reason": "kind_mismatch", "left": left, "right": right }),
        ),
        TransformError::IncompatibleMapEntry(key) => wasm_error_details(
            code,
            operation,
            json!({ "reason": "map_entry_conflict", "key": key }),
        ),
        TransformError::LengthOverflow => {
            wasm_error_details(code, operation, json!({ "reason": "length_overflow" }))
        }
        other => wasm_error(code, operation, other.to_string()),
    }
}

fn invert_error(error: InvertError, operation: &'static str) -> JsValue {
    let code = error.code().as_str();
    match error {
        InvertError::Apply(error) => apply_error(error, operation),
        InvertError::LengthOverflow => {
            wasm_error_details(code, operation, json!({ "reason": "length_overflow" }))
        }
        other => wasm_error(code, operation, other.to_string()),
    }
}

fn apply_error(error: ApplyError, operation: &'static str) -> JsValue {
    let code = error.code().as_str();
    match error {
        ApplyError::TypeMismatch {
            expected, actual, ..
        } => wasm_error_details(
            code,
            operation,
            json!({ "expected": value_type_name(expected), "actual": value_type_name(actual) }),
        ),
        ApplyError::MissingKey { key, .. } => {
            wasm_error_details(code, operation, json!({ "key": key }))
        }
        ApplyError::ExistingKey { key, .. } => {
            wasm_error_details(code, operation, json!({ "key": key }))
        }
        ApplyError::IndexOutOfBounds { index, len, .. } => wasm_error_details(
            code,
            operation,
            json!({ "target": "list", "length": len, "index": index }),
        ),
        ApplyError::SequenceOutOfBounds { .. } => wasm_error_details(
            code,
            operation,
            json!({ "target": "sequence", "length": 0 }),
        ),
        ApplyError::IntegerOverflow { .. } => wasm_error_details(code, operation, json!({})),
        other => wasm_error(code, operation, other.to_string()),
    }
}

fn value_type_name(value_type: ValueType) -> &'static str {
    match value_type {
        ValueType::Null => "null",
        ValueType::Bool => "bool",
        ValueType::Int => "int",
        ValueType::Float => "float",
        ValueType::String => "string",
        ValueType::Text => "text",
        ValueType::RichText => "richText",
        ValueType::List => "list",
        ValueType::Map => "map",
    }
}

fn parse_limits(value: &str, operation: &'static str) -> Result<InputLimits, JsValue> {
    let value: JsonValue = serde_json::from_str(value)
        .map_err(|error| wasm_error("invalid_argument", operation, error.to_string()))?;
    let field = |name: &'static str| -> Result<usize, JsValue> {
        let value = value.get(name).and_then(JsonValue::as_u64).ok_or_else(|| {
            wasm_error(
                "invalid_argument",
                operation,
                format!("invalid limit field {name}"),
            )
        })?;
        Ok(usize::try_from(value).unwrap_or(usize::MAX))
    };
    Ok(InputLimits {
        max_depth: field("maxDepth")?,
        max_value_nodes: field("maxValueNodes")?,
        max_change_nodes: field("maxChangeNodes")?,
        max_container_len: field("maxContainerLength")?,
        max_string_bytes: field("maxStringBytes")?,
        max_sequence_ops: field("maxSequenceOps")?,
        max_sequence_len: field("maxSequenceLength")?,
    })
}

fn parse_path(value: &str, operation: &'static str) -> Result<Path, JsValue> {
    let segments: JsonValue = serde_json::from_str(value)
        .map_err(|error| wasm_error("invalid_argument", operation, error.to_string()))?;
    let segments = segments
        .as_array()
        .ok_or_else(|| wasm_error("invalid_argument", operation, "path must be an array"))?;
    let mut path = Path::new();
    for segment in segments {
        match segment {
            JsonValue::String(key) => path.push(PathSeg::Key(key.clone())),
            JsonValue::Number(index) => {
                let index = index.as_u64().ok_or_else(|| {
                    wasm_error("invalid_argument", operation, "path index is out of range")
                })?;
                let index = usize::try_from(index).unwrap_or(usize::MAX);
                path.push(PathSeg::Index(index));
            }
            _ => {
                return Err(wasm_error(
                    "invalid_argument",
                    operation,
                    "path segments must be strings or indices",
                ))
            }
        }
    }
    Ok(path)
}

fn value_at_path<'a>(
    root: &'a Value,
    path: &Path,
    operation: &'static str,
) -> Result<&'a Value, JsValue> {
    let mut current = root;
    for segment in path.segments() {
        current = match (current.kind(), segment) {
            (ValueKind::Map(map), PathSeg::Key(key)) => map.get(key).ok_or_else(|| {
                wasm_error_details("missing_key", operation, json!({ "key": key }))
            })?,
            (ValueKind::List(list), PathSeg::Index(index)) => {
                list.get(*index).ok_or_else(|| {
                    wasm_error_details(
                        "out_of_bounds",
                        operation,
                        json!({ "target": "list", "length": list.len(), "index": index }),
                    )
                })?
            }
            (actual, PathSeg::Key(_)) => {
                return Err(wasm_error_details(
                    "type_mismatch",
                    operation,
                    json!({ "expected": "map", "actual": value_kind_name(actual) }),
                ))
            }
            (actual, PathSeg::Index(_)) => {
                return Err(wasm_error_details(
                    "type_mismatch",
                    operation,
                    json!({ "expected": "list", "actual": value_kind_name(actual) }),
                ))
            }
        };
    }
    Ok(current)
}

fn utf16_to_code_point(
    text: &str,
    position: usize,
    operation: &'static str,
) -> Result<usize, JsValue> {
    let mut utf16 = 0usize;
    for (code_point, character) in text.chars().enumerate() {
        if position == utf16 {
            return Ok(code_point);
        }
        let next = utf16 + character.len_utf16();
        if position < next {
            return Err(wasm_error_details(
                "invalid_utf16_boundary",
                operation,
                json!({ "position": position }),
            ));
        }
        utf16 = next;
    }
    if position == utf16 {
        Ok(text.chars().count())
    } else {
        Err(wasm_error_details(
            "out_of_bounds",
            operation,
            json!({ "target": "text", "length": utf16, "index": position }),
        ))
    }
}

fn code_point_to_utf16(
    text: &str,
    position: usize,
    operation: &'static str,
) -> Result<usize, JsValue> {
    let mut utf16 = 0usize;
    for (code_point, character) in text.chars().enumerate() {
        if code_point == position {
            return Ok(utf16);
        }
        utf16 += character.len_utf16();
    }
    let length = text.chars().count();
    if position == length {
        Ok(utf16)
    } else {
        Err(wasm_error_details(
            "out_of_bounds",
            operation,
            json!({ "target": "text", "length": length, "index": position }),
        ))
    }
}

fn rich_utf16_to_code_point(
    rich: &RichText,
    position: usize,
    operation: &'static str,
) -> Result<usize, JsValue> {
    rich.utf16_to_code_point(position)
        .map_err(|error| rich_position_error(error, operation))
}

fn rich_code_point_to_utf16(
    rich: &RichText,
    position: usize,
    operation: &'static str,
) -> Result<usize, JsValue> {
    rich.code_point_to_utf16(position)
        .map_err(|error| rich_position_error(error, operation))
}

fn rich_position_error(error: Utf16PositionError, operation: &'static str) -> JsValue {
    match error {
        Utf16PositionError::InvalidUtf16Boundary { position } => wasm_error_details(
            "invalid_utf16_boundary",
            operation,
            json!({ "position": position }),
        ),
        Utf16PositionError::CodePointOutOfBounds { position, len }
        | Utf16PositionError::Utf16OutOfBounds { position, len } => wasm_error_details(
            "out_of_bounds",
            operation,
            json!({ "target": "richText", "length": len, "index": position }),
        ),
        other => wasm_error("invalid_argument", operation, other.to_string()),
    }
}
