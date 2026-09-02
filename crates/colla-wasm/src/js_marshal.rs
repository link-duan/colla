//! Structured JS <-> Value/Change marshaling.
//!
//! The canonical wire format lives only in `colla`'s codec. This module builds
//! `Value`/`Change` from already-validated structured JS input (the TypeScript
//! facade rejects getters, symbol keys, non-plain objects, cycles, and unpaired
//! surrogates before calling in) and materializes a `Value` back to JS. It owns
//! kind discrimination, canonicalization (via the core constructors), and
//! receiver `InputLimits` enforcement over the raw input.

use colla::{
    AttrChange, AttrPatch, AttrValue, Attrs, Change, InputLimits, IntChange, ListChange, ListOp,
    MapChange, MapEntryChange, RichContent, RichSpan, RichText, RichTextChange, RichTextOp,
    TextChange, TextOp, Value, ValueError, ValueKind,
};
use js_sys::{Array, BigInt, Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Debug)]
pub(crate) enum InputError {
    Limit {
        name: &'static str,
        actual: usize,
        maximum: usize,
    },
    /// Domain violation of the value model (`invalid_value`).
    Value(ValueError),
    /// Value-domain violation with an explicit reason (`invalid_value`).
    InvalidValue(&'static str),
    /// Structured shape violation Rust reached despite facade validation
    /// (`invalid_argument`).
    Argument { context: String, reason: String },
}

impl From<ValueError> for InputError {
    fn from(error: ValueError) -> Self {
        Self::Value(error)
    }
}

fn argument(context: impl Into<String>, reason: impl Into<String>) -> InputError {
    InputError::Argument {
        context: context.into(),
        reason: reason.into(),
    }
}

/// Builds a canonical `Value` from validated structured JS input.
pub(crate) fn value_from_js(input: &JsValue, limits: &InputLimits) -> Result<Value, InputError> {
    Counter::new(limits).value(input, 1, "value")
}

/// Builds a canonical `Change` from validated structured JS input.
pub(crate) fn change_from_js(input: &JsValue, limits: &InputLimits) -> Result<Change, InputError> {
    Counter::new(limits).change(input, 1, "change")
}

struct Counter<'a> {
    limits: &'a InputLimits,
    value_nodes: usize,
    change_nodes: usize,
}

impl<'a> Counter<'a> {
    fn new(limits: &'a InputLimits) -> Self {
        Self {
            limits,
            value_nodes: 0,
            change_nodes: 0,
        }
    }

    fn value(&mut self, input: &JsValue, depth: usize, ctx: &str) -> Result<Value, InputError> {
        self.check_limit("depth", depth, self.limits.max_depth)?;
        self.value_nodes += 1;
        self.check_limit("value nodes", self.value_nodes, self.limits.max_value_nodes)?;

        if input.is_null() {
            return Ok(Value::null());
        }
        if let Some(flag) = input.as_bool() {
            return Ok(Value::bool(flag));
        }
        if input.is_bigint() {
            return Ok(Value::int(self.read_i64(input, ctx)?));
        }
        if let Some(number) = input.as_f64() {
            return Value::float(number).map_err(InputError::Value);
        }
        if let Some(text) = input.as_string() {
            self.check_limit("string bytes", text.len(), self.limits.max_string_bytes)?;
            return Ok(Value::string(text));
        }
        if Array::is_array(input) {
            let array: &Array = input.unchecked_ref();
            let len = array.length() as usize;
            self.check_limit("container length", len, self.limits.max_container_len)?;
            let mut values = Vec::with_capacity(len);
            for index in 0..len {
                values.push(self.value(&array.get(index as u32), depth + 1, ctx)?);
            }
            return Ok(Value::list(values));
        }
        if !input.is_object() {
            return Err(argument(ctx, "unsupported Value"));
        }
        let object: &Object = input.unchecked_ref();
        match marker(object, ctx)? {
            Some("text") => {
                let text = self.read_field_string(object, "value", ctx)?;
                self.check_limit("string bytes", text.len(), self.limits.max_string_bytes)?;
                Ok(Value::text(text))
            }
            Some("richtext") => {
                let spans = get(object, "spans", ctx)?;
                let spans: &Array = spans.unchecked_ref();
                let count = spans.length() as usize;
                self.check_limit("container length", count, self.limits.max_container_len)?;
                let mut lengths = SequenceLengths::default();
                let mut built = Vec::with_capacity(count);
                for index in 0..count {
                    let span = self.rich_span(&spans.get(index as u32), depth + 1, ctx)?;
                    lengths.insert(span.len(), self)?;
                    built.push(span);
                }
                Ok(Value::rich_text(RichText::from_spans(built)?))
            }
            Some(_) | None => {
                let keys = Object::keys(object);
                let len = keys.length() as usize;
                self.check_limit("container length", len, self.limits.max_container_len)?;
                let mut entries = Vec::with_capacity(len);
                for index in 0..len {
                    let key = keys.get(index as u32);
                    let key = key.as_string().ok_or_else(|| argument(ctx, "map key"))?;
                    self.check_limit("string bytes", key.len(), self.limits.max_string_bytes)?;
                    let child = get(object, &key, ctx)?;
                    entries.push((key, self.value(&child, depth + 1, ctx)?));
                }
                Ok(Value::map(entries)?)
            }
        }
    }

    fn rich_span(
        &mut self,
        span: &JsValue,
        depth: usize,
        ctx: &str,
    ) -> Result<RichSpan, InputError> {
        let object: &Object = span.unchecked_ref();
        let attrs = self.attrs(&get(object, "attrs", ctx)?, ctx)?;
        match marker(object, ctx)? {
            Some("text") => {
                let text = self.read_field_string(object, "text", ctx)?;
                self.check_limit("string bytes", text.len(), self.limits.max_string_bytes)?;
                Ok(RichSpan::text(text, attrs))
            }
            Some("embed") => {
                let value = self.value(&get(object, "value", ctx)?, depth, ctx)?;
                Ok(RichSpan::embed(value, attrs))
            }
            _ => Err(argument(ctx, "rich span type")),
        }
    }

    fn attrs(&self, input: &JsValue, ctx: &str) -> Result<Attrs, InputError> {
        if input.is_undefined() || input.is_null() {
            return Ok(Attrs::new());
        }
        let object: &Object = input.unchecked_ref();
        let keys = Object::keys(object);
        let len = keys.length() as usize;
        self.check_limit("container length", len, self.limits.max_container_len)?;
        let mut entries = Vec::with_capacity(len);
        for index in 0..len {
            let key = keys.get(index as u32);
            let key = key.as_string().ok_or_else(|| argument(ctx, "attr key"))?;
            self.check_limit("string bytes", key.len(), self.limits.max_string_bytes)?;
            entries.push((key.clone(), self.attr_value(&get(object, &key, ctx)?, ctx)?));
        }
        Ok(Attrs::from_entries(entries)?)
    }

    fn attr_value(&self, input: &JsValue, ctx: &str) -> Result<AttrValue, InputError> {
        if let Some(flag) = input.as_bool() {
            return Ok(AttrValue::Bool(flag));
        }
        if input.is_bigint() {
            return Ok(AttrValue::Int(self.read_i64(input, ctx)?));
        }
        if let Some(number) = input.as_f64() {
            return AttrValue::float(number).map_err(InputError::Value);
        }
        if let Some(text) = input.as_string() {
            self.check_limit("string bytes", text.len(), self.limits.max_string_bytes)?;
            return Ok(AttrValue::string(text));
        }
        Err(argument(ctx, "unsupported attribute value"))
    }

    fn change(&mut self, input: &JsValue, depth: usize, ctx: &str) -> Result<Change, InputError> {
        self.check_limit("depth", depth, self.limits.max_depth)?;
        self.change_nodes += 1;
        self.check_limit(
            "change nodes",
            self.change_nodes,
            self.limits.max_change_nodes,
        )?;
        let object: &Object = input.unchecked_ref();
        let kind = self.read_field_string(object, "type", ctx)?;
        match kind.as_str() {
            "noop" => Ok(Change::noop()),
            "replace" => Ok(Change::replace(self.value(
                &get(object, "value", ctx)?,
                depth + 1,
                ctx,
            )?)),
            "int" => Ok(IntChange::Add(self.read_i64(&get(object, "delta", ctx)?, ctx)?).into()),
            "map" => {
                let items = array_field(object, "entries", ctx)?;
                let count = items.length() as usize;
                self.check_limit("container length", count, self.limits.max_container_len)?;
                let mut entries = Vec::with_capacity(count);
                for index in 0..count {
                    let item = items.get(index as u32);
                    let item: &Object = item.unchecked_ref();
                    let key = self.read_field_string(item, "key", ctx)?;
                    let action = self.read_field_string(item, "type", ctx)?;
                    let entry = match action.as_str() {
                        "insert" => MapEntryChange::Insert(self.value(
                            &get(item, "value", ctx)?,
                            depth + 1,
                            ctx,
                        )?),
                        "delete" => MapEntryChange::Delete,
                        "modify" => MapEntryChange::Modify(self.change(
                            &get(item, "change", ctx)?,
                            depth + 1,
                            ctx,
                        )?),
                        other => return Err(argument(ctx, format!("map entry type {other}"))),
                    };
                    entries.push((key, entry));
                }
                Ok(MapChange::from_entries(entries)?.into())
            }
            "list" => {
                let items = array_field(object, "ops", ctx)?;
                let count = items.length() as usize;
                self.check_limit("sequence ops", count, self.limits.max_sequence_ops)?;
                let mut lengths = SequenceLengths::default();
                let mut ops = Vec::with_capacity(count);
                for index in 0..count {
                    let op = items.get(index as u32);
                    let op: &Object = op.unchecked_ref();
                    match self.read_field_string(op, "type", ctx)?.as_str() {
                        "retain" => {
                            let len = self.read_usize(&get(op, "length", ctx)?, ctx)?;
                            lengths.retain(len, self)?;
                            ops.push(ListOp::Retain(len));
                        }
                        "insert" => {
                            let values = array_field(op, "values", ctx)?;
                            let vlen = values.length() as usize;
                            self.check_limit(
                                "container length",
                                vlen,
                                self.limits.max_container_len,
                            )?;
                            lengths.insert(vlen, self)?;
                            let mut built = Vec::with_capacity(vlen);
                            for vi in 0..vlen {
                                built.push(self.value(&values.get(vi as u32), depth + 1, ctx)?);
                            }
                            ops.push(ListOp::Insert(built));
                        }
                        "delete" => {
                            let len = self.read_usize(&get(op, "length", ctx)?, ctx)?;
                            lengths.delete(len, self)?;
                            ops.push(ListOp::Delete(len));
                        }
                        "modify" => {
                            lengths.retain(1, self)?;
                            ops.push(ListOp::Modify(self.change(
                                &get(op, "change", ctx)?,
                                depth + 1,
                                ctx,
                            )?));
                        }
                        other => return Err(argument(ctx, format!("list op type {other}"))),
                    }
                }
                Ok(ListChange::from_ops(ops)?.into())
            }
            "text" => {
                let items = array_field(object, "ops", ctx)?;
                let count = items.length() as usize;
                self.check_limit("sequence ops", count, self.limits.max_sequence_ops)?;
                let mut lengths = SequenceLengths::default();
                let mut ops = Vec::with_capacity(count);
                for index in 0..count {
                    let op = items.get(index as u32);
                    let op: &Object = op.unchecked_ref();
                    match self.read_field_string(op, "type", ctx)?.as_str() {
                        "retain" => {
                            let len = self.read_usize(&get(op, "length", ctx)?, ctx)?;
                            lengths.retain(len, self)?;
                            ops.push(TextOp::Retain(len));
                        }
                        "insert" => {
                            let text = self.read_field_string(op, "text", ctx)?;
                            self.check_limit(
                                "string bytes",
                                text.len(),
                                self.limits.max_string_bytes,
                            )?;
                            lengths.insert(text.chars().count(), self)?;
                            ops.push(TextOp::Insert(text));
                        }
                        "delete" => {
                            let len = self.read_usize(&get(op, "length", ctx)?, ctx)?;
                            lengths.delete(len, self)?;
                            ops.push(TextOp::Delete(len));
                        }
                        other => return Err(argument(ctx, format!("text op type {other}"))),
                    }
                }
                Ok(TextChange::from_ops(ops)?.into())
            }
            "richtext" => {
                let items = array_field(object, "ops", ctx)?;
                let count = items.length() as usize;
                self.check_limit("sequence ops", count, self.limits.max_sequence_ops)?;
                let mut lengths = SequenceLengths::default();
                let mut ops = Vec::with_capacity(count);
                for index in 0..count {
                    let op = items.get(index as u32);
                    let op: &Object = op.unchecked_ref();
                    match self.read_field_string(op, "type", ctx)?.as_str() {
                        "retain" => {
                            let len = self.read_usize(&get(op, "length", ctx)?, ctx)?;
                            lengths.retain(len, self)?;
                            let patch = self.attr_patch(&get(op, "patch", ctx)?, ctx)?;
                            ops.push(RichTextOp::Retain { len, attrs: patch });
                        }
                        "insert" => {
                            let content_js = get(op, "content", ctx)?;
                            let content_obj: &Object = content_js.unchecked_ref();
                            let content_attrs =
                                self.attrs(&get(content_obj, "attrs", ctx)?, ctx)?;
                            let content = match marker(content_obj, ctx)? {
                                Some("text") => {
                                    let text = self.read_field_string(content_obj, "text", ctx)?;
                                    self.check_limit(
                                        "string bytes",
                                        text.len(),
                                        self.limits.max_string_bytes,
                                    )?;
                                    RichContent::text(text)
                                }
                                Some("embed") => RichContent::embed(self.value(
                                    &get(content_obj, "value", ctx)?,
                                    depth + 1,
                                    ctx,
                                )?),
                                _ => return Err(argument(ctx, "rich content type")),
                            };
                            lengths.insert(content.len(), self)?;
                            ops.push(RichTextOp::Insert {
                                content,
                                attrs: content_attrs,
                            });
                        }
                        "delete" => {
                            let len = self.read_usize(&get(op, "length", ctx)?, ctx)?;
                            lengths.delete(len, self)?;
                            ops.push(RichTextOp::Delete(len));
                        }
                        other => return Err(argument(ctx, format!("rich text op type {other}"))),
                    }
                }
                Ok(RichTextChange::from_ops(ops)?.into())
            }
            other => Err(argument(ctx, format!("change type {other}"))),
        }
    }

    fn attr_patch(&self, input: &JsValue, ctx: &str) -> Result<AttrPatch, InputError> {
        if input.is_undefined() || input.is_null() {
            return Ok(AttrPatch::from_entries(Vec::<(String, AttrChange)>::new())?);
        }
        let object: &Object = input.unchecked_ref();
        let keys = Object::keys(object);
        let len = keys.length() as usize;
        self.check_limit("container length", len, self.limits.max_container_len)?;
        let mut entries = Vec::with_capacity(len);
        for index in 0..len {
            let key = keys.get(index as u32);
            let key = key
                .as_string()
                .ok_or_else(|| argument(ctx, "attr patch key"))?;
            self.check_limit("string bytes", key.len(), self.limits.max_string_bytes)?;
            let action = get(object, &key, ctx)?;
            let action: &Object = action.unchecked_ref();
            let change = match self.read_field_string(action, "type", ctx)?.as_str() {
                "set" => AttrChange::Set(self.attr_value(&get(action, "value", ctx)?, ctx)?),
                "remove" => AttrChange::Remove,
                other => return Err(argument(ctx, format!("attr patch action {other}"))),
            };
            entries.push((key, change));
        }
        Ok(AttrPatch::from_entries(entries)?)
    }

    fn read_i64(&self, input: &JsValue, ctx: &str) -> Result<i64, InputError> {
        if !input.is_bigint() {
            return Err(argument(ctx, "expected a bigint"));
        }
        let big: BigInt = input.clone().unchecked_into();
        let text: String = big
            .to_string(10)
            .map_err(|_| argument(ctx, "expected a bigint"))?
            .into();
        text.parse::<i64>()
            .map_err(|_| InputError::InvalidValue("integer is outside the signed 64-bit range"))
    }

    fn read_usize(&self, input: &JsValue, ctx: &str) -> Result<usize, InputError> {
        let number = input
            .as_f64()
            .ok_or_else(|| argument(ctx, "expected a length"))?;
        if number < 0.0 || number.fract() != 0.0 || number > usize::MAX as f64 {
            return Err(argument(ctx, "invalid length"));
        }
        Ok(number as usize)
    }

    fn read_field_string(
        &self,
        object: &Object,
        field: &str,
        ctx: &str,
    ) -> Result<String, InputError> {
        get(object, field, ctx)?
            .as_string()
            .ok_or_else(|| argument(ctx, format!("expected string field {field}")))
    }

    fn check_limit(
        &self,
        name: &'static str,
        actual: usize,
        maximum: usize,
    ) -> Result<(), InputError> {
        if actual > maximum {
            Err(InputError::Limit {
                name,
                actual,
                maximum,
            })
        } else {
            Ok(())
        }
    }
}

fn get(object: &Object, key: &str, ctx: &str) -> Result<JsValue, InputError> {
    Reflect::get(object, &JsValue::from_str(key))
        .map_err(|_| argument(ctx, format!("missing {key}")))
}

fn array_field(object: &Object, key: &str, ctx: &str) -> Result<Array, InputError> {
    let value = get(object, key, ctx)?;
    if !Array::is_array(&value) {
        return Err(argument(ctx, format!("expected array field {key}")));
    }
    Ok(value.unchecked_into())
}

fn marker(object: &Object, ctx: &str) -> Result<Option<&'static str>, InputError> {
    let value =
        Reflect::get(object, &JsValue::from_str("type")).map_err(|_| argument(ctx, "type"))?;
    match value.as_string().as_deref() {
        Some("text") => Ok(Some("text")),
        Some("richtext") => Ok(Some("richtext")),
        Some("embed") => Ok(Some("embed")),
        Some(_) => Ok(Some("")),
        None => Ok(None),
    }
}

#[derive(Default)]
struct SequenceLengths {
    input: usize,
    output: usize,
}

impl SequenceLengths {
    fn retain(&mut self, len: usize, counter: &Counter<'_>) -> Result<(), InputError> {
        self.input = self.add(self.input, len, counter)?;
        self.output = self.add(self.output, len, counter)?;
        Ok(())
    }

    fn insert(&mut self, len: usize, counter: &Counter<'_>) -> Result<(), InputError> {
        self.output = self.add(self.output, len, counter)?;
        Ok(())
    }

    fn delete(&mut self, len: usize, counter: &Counter<'_>) -> Result<(), InputError> {
        self.input = self.add(self.input, len, counter)?;
        Ok(())
    }

    fn add(
        &self,
        current: usize,
        amount: usize,
        counter: &Counter<'_>,
    ) -> Result<usize, InputError> {
        let total = current.checked_add(amount).ok_or(InputError::Limit {
            name: "sequence length",
            actual: usize::MAX,
            maximum: counter.limits.max_sequence_len,
        })?;
        counter.check_limit("sequence length", total, counter.limits.max_sequence_len)?;
        Ok(total)
    }
}

/// Materializes a canonical `Value` into structured JS output.
///
/// Maps become frozen null-prototype objects (so `__proto__` is a data key and
/// `deepStrictEqual` against `Object.create(null)` holds); lists become frozen
/// arrays; tag objects (`text`/`richtext`/spans) are frozen normal objects.
pub(crate) fn value_to_js(value: &Value) -> JsValue {
    match value.kind() {
        ValueKind::Null => JsValue::NULL,
        ValueKind::Bool(flag) => JsValue::from_bool(*flag),
        ValueKind::Int(number) => JsValue::from(*number),
        ValueKind::Float(number) => JsValue::from_f64(number.get()),
        ValueKind::String(text) => JsValue::from_str(text),
        ValueKind::Text(text) => {
            let object = Object::new();
            set(&object, "type", &JsValue::from_str("text"));
            set(&object, "value", &JsValue::from_str(text.as_str()));
            freeze(object)
        }
        ValueKind::List(values) => {
            let array = Array::new();
            for item in values.as_slice() {
                array.push(&value_to_js(item));
            }
            let object: &Object = array.unchecked_ref();
            Object::freeze(object);
            array.into()
        }
        ValueKind::Map(map) => {
            let object = null_proto();
            for (key, item) in map.iter() {
                set(&object, key, &value_to_js(item));
            }
            freeze(object)
        }
        ValueKind::RichText(rich) => {
            let spans = Array::new();
            for span in rich.iter_spans() {
                let object = Object::new();
                match span.content() {
                    RichContent::Text(chunk) => {
                        set(&object, "type", &JsValue::from_str("text"));
                        set(&object, "text", &JsValue::from_str(chunk.as_str()));
                    }
                    RichContent::Embed(value) => {
                        set(&object, "type", &JsValue::from_str("embed"));
                        set(&object, "value", &value_to_js(value));
                    }
                }
                if let Some(attrs) = attrs_to_js(span.attrs()) {
                    set(&object, "attrs", &attrs);
                }
                spans.push(&freeze(object));
            }
            let spans_object: &Object = spans.unchecked_ref();
            Object::freeze(spans_object);
            let object = Object::new();
            set(&object, "type", &JsValue::from_str("richtext"));
            set(&object, "spans", &spans);
            freeze(object)
        }
    }
}

fn attrs_to_js(attrs: &Attrs) -> Option<JsValue> {
    if attrs.is_empty() {
        return None;
    }
    let object = null_proto();
    for (key, value) in attrs.iter() {
        let js = match value {
            AttrValue::Bool(flag) => JsValue::from_bool(*flag),
            AttrValue::Int(number) => JsValue::from(*number),
            AttrValue::Float(number) => JsValue::from_f64(number.get()),
            AttrValue::String(text) => JsValue::from_str(text),
        };
        set(&object, key, &js);
    }
    Some(freeze(object))
}

fn null_proto() -> Object {
    let object = Object::new();
    Object::set_prototype_of(&object, JsValue::NULL.unchecked_ref());
    object
}

fn freeze(object: Object) -> JsValue {
    Object::freeze(&object);
    object.into()
}

fn set(object: &Object, key: &str, value: &JsValue) {
    let _ = Reflect::set(object, &JsValue::from_str(key), value);
}
