//! Private WebAssembly bindings for `colla`.
//!
//! Generated names and error payloads in this crate are an implementation
//! detail of the handwritten `@colla/core` facade.

use colla::{apply, Change, ChangeBuilder, Path, Value, ValueKind};
use serde_json::json;
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
    pub fn decode(bytes: &[u8]) -> Result<Self, JsValue> {
        Value::decode(bytes)
            .map(|value| Self { value })
            .map_err(|error| wasm_error("invalid_encoding", "value_decode", error.to_string()))
    }

    pub fn encode(&self) -> Vec<u8> {
        self.value.encode()
    }

    pub fn kind(&self) -> String {
        value_kind_name(self.value.kind()).into()
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
    pub fn decode(bytes: &[u8]) -> Result<Self, JsValue> {
        Change::decode(bytes)
            .map(|change| Self { change })
            .map_err(|error| wasm_error("invalid_encoding", "change_decode", error.to_string()))
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
    wasm_error(
        "type_mismatch",
        "value_to_js",
        format!("expected {expected}, got {}", value_kind_name(actual)),
    )
}

fn wasm_error(code: &'static str, operation: &'static str, reason: impl Into<String>) -> JsValue {
    JsValue::from_str(
        &json!({
            "code": code,
            "operation": operation,
            "details": { "reason": reason.into() },
        })
        .to_string(),
    )
}
