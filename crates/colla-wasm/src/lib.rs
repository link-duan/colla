//! Private WebAssembly bindings for `colla`.
//!
//! Generated names and error payloads in this crate are an implementation
//! detail of the handwritten `@colla/core` facade.

use colla::{
    apply, Change, ChangeBuilder, CodecError, InputLimits, Path, PathSeg, Value, ValueKind,
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

    #[wasm_bindgen(js_name = decodeTrusted)]
    pub fn decode_trusted(bytes: &[u8]) -> Result<Self, JsValue> {
        Value::decode_with_limits(bytes, &trusted_limits())
            .map(|value| Self { value })
            .map_err(|error| codec_error(error, "value_from_js"))
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

    #[wasm_bindgen(js_name = change)]
    pub fn change_builder(&self) -> BuilderHandle {
        BuilderHandle {
            builder: Some(self.value.change()),
        }
    }
}

#[wasm_bindgen]
pub struct ChangeHandle {
    change: Change,
}

#[wasm_bindgen]
impl ChangeHandle {
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

#[wasm_bindgen]
pub struct BuilderHandle {
    builder: Option<ChangeBuilder>,
}

#[wasm_bindgen]
impl BuilderHandle {
    #[wasm_bindgen(js_name = replaceRoot)]
    pub fn replace_root(&mut self, value: &ValueHandle) -> Result<(), JsValue> {
        self.builder_mut()?
            .replace(&Path::new(), value.value.clone())
            .map(|_| ())
            .map_err(|error| wasm_error("invalid_argument", "builder_replace", error.to_string()))
    }

    pub fn build(&mut self) -> Result<ChangeHandle, JsValue> {
        let builder = self.builder.take().ok_or_else(|| {
            wasm_error("invalid_state", "builder_build", "builder already consumed")
        })?;
        Ok(ChangeHandle {
            change: builder.build(),
        })
    }
}

impl BuilderHandle {
    fn builder_mut(&mut self) -> Result<&mut ChangeBuilder, JsValue> {
        self.builder.as_mut().ok_or_else(|| {
            wasm_error(
                "invalid_state",
                "builder_replace",
                "builder already consumed",
            )
        })
    }
}

#[wasm_bindgen(js_name = applyHandles)]
pub fn apply_handles(base: &ValueHandle, change: &ChangeHandle) -> Result<ValueHandle, JsValue> {
    apply(&base.value, &change.change)
        .map(|value| ValueHandle { value })
        .map_err(|error| wasm_error("incompatible_change", "apply", error.to_string()))
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
    match error {
        CodecError::LimitExceeded {
            name,
            actual,
            limit,
        } => wasm_error_details(
            "limit_exceeded",
            operation,
            json!({ "limit": name, "actual": actual, "maximum": limit }),
        ),
        other => {
            let offset = match &other {
                CodecError::UnexpectedEof { offset }
                | CodecError::UnknownTag { offset, .. }
                | CodecError::NonMinimalVarint { offset }
                | CodecError::IntegerOutOfRange { offset }
                | CodecError::InvalidUtf8 { offset }
                | CodecError::NonCanonical { offset, .. }
                | CodecError::TrailingBytes { offset } => Some(*offset),
                CodecError::LimitExceeded { .. } | CodecError::Value(_) => None,
                _ => None,
            };
            let mut details = json!({ "reason": other.to_string() });
            if let Some(offset) = offset {
                details["offset"] = json!(offset);
            }
            wasm_error_details("invalid_encoding", operation, details)
        }
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

fn trusted_limits() -> InputLimits {
    InputLimits {
        max_depth: usize::MAX,
        max_value_nodes: usize::MAX,
        max_change_nodes: usize::MAX,
        max_container_len: usize::MAX,
        max_string_bytes: usize::MAX,
        max_sequence_ops: usize::MAX,
        max_sequence_len: usize::MAX,
    }
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
