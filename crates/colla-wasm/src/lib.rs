//! Private WebAssembly bindings for `colla`.
//!
//! Generated names and error payloads in this crate are an implementation
//! detail of the handwritten `colla-ot` facade.

use colla::{
    apply, compose, invert, transform_pair, ApplyError, AttrChange, AttrPatch, AttrValue, Attrs,
    BuildError, Change, ChangeBuilder, ChangeKind, CodecError, ComposeError, InputLimits,
    IntChange, InvertError, ListOp, MapEntryChange, Path, PathSeg, RichContent, RichText,
    RichTextOp, TextOp, TieBreak, TransformError, Utf16PositionError, Value, ValueKind, ValueType,
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
    pub fn replace(&mut self, path: &str, value: &ValueHandle) -> Result<(), JsValue> {
        let path = parse_path(path, "builder_replace")?;
        self.builder_mut()?
            .replace(&path, value.value.clone())
            .map(|_| ())
            .map_err(|error| build_error(error, "builder_replace"))
    }

    #[wasm_bindgen(js_name = mapSet)]
    pub fn map_set(&mut self, path: &str, key: &str, value: &ValueHandle) -> Result<(), JsValue> {
        let path = parse_path(path, "map_set")?;
        self.builder_mut()?
            .map_set(&path, key, value.value.clone())
            .map(|_| ())
            .map_err(|error| build_error(error, "map_set"))
    }

    #[wasm_bindgen(js_name = mapDelete)]
    pub fn map_delete(&mut self, path: &str, key: &str) -> Result<(), JsValue> {
        let path = parse_path(path, "map_delete")?;
        self.builder_mut()?
            .map_delete(&path, key)
            .map(|_| ())
            .map_err(|error| build_error(error, "map_delete"))
    }

    #[wasm_bindgen(js_name = listInsert)]
    pub fn list_insert(
        &mut self,
        path: &str,
        index: usize,
        values: &ValueHandle,
    ) -> Result<(), JsValue> {
        let path = parse_path(path, "list_insert")?;
        let ValueKind::List(values) = values.value.kind() else {
            return Err(type_error("list", values.value.kind()));
        };
        self.builder_mut()?
            .list_insert(&path, index, values.as_slice().iter().cloned())
            .map(|_| ())
            .map_err(|error| build_error(error, "list_insert"))
    }

    #[wasm_bindgen(js_name = listSet)]
    pub fn list_set(
        &mut self,
        path: &str,
        index: usize,
        value: &ValueHandle,
    ) -> Result<(), JsValue> {
        let path = parse_path(path, "list_set")?;
        self.builder_mut()?
            .list_set(&path, index, value.value.clone())
            .map(|_| ())
            .map_err(|error| build_error(error, "list_set"))
    }

    #[wasm_bindgen(js_name = listDelete)]
    pub fn list_delete(&mut self, path: &str, from: usize, to: usize) -> Result<(), JsValue> {
        let path = parse_path(path, "list_delete")?;
        let len = to.checked_sub(from).ok_or_else(|| {
            wasm_error_details(
                "invalid_argument",
                "list_delete",
                json!({ "argument": "range", "reason": "from must not exceed to" }),
            )
        })?;
        self.builder_mut()?
            .list_delete(&path, from, len)
            .map(|_| ())
            .map_err(|error| build_error(error, "list_delete"))
    }

    #[wasm_bindgen(js_name = currentKind)]
    pub fn current_kind(&self, path: &str) -> Result<String, JsValue> {
        let path = parse_path(path, "builder_scope")?;
        let builder = self.builder.as_ref().ok_or_else(|| {
            wasm_error("invalid_state", "builder_scope", "builder already consumed")
        })?;
        Ok(
            value_kind_name(value_at_path(builder.current(), &path, "builder_scope")?.kind())
                .into(),
        )
    }

    #[wasm_bindgen(js_name = textInsert)]
    pub fn text_insert(&mut self, path: &str, position: usize, value: &str) -> Result<(), JsValue> {
        let path = parse_path(path, "text_insert")?;
        let index = {
            let text = text_at_path(
                self.builder_ref("text_insert")?.current(),
                &path,
                "text_insert",
            )?;
            utf16_to_code_point(text, position, "text_insert")?
        };
        self.builder_mut()?
            .text_insert(&path, index, value)
            .map(|_| ())
            .map_err(|error| build_error(error, "text_insert"))
    }

    #[wasm_bindgen(js_name = textDelete)]
    pub fn text_delete(&mut self, path: &str, from: usize, to: usize) -> Result<(), JsValue> {
        let path = parse_path(path, "text_delete")?;
        let (from, to) = {
            let text = text_at_path(
                self.builder_ref("text_delete")?.current(),
                &path,
                "text_delete",
            )?;
            (
                utf16_to_code_point(text, from, "text_delete")?,
                utf16_to_code_point(text, to, "text_delete")?,
            )
        };
        let len = to.checked_sub(from).ok_or_else(|| {
            wasm_error_details(
                "invalid_argument",
                "text_delete",
                json!({ "argument": "range", "reason": "from must not exceed to" }),
            )
        })?;
        self.builder_mut()?
            .text_delete(&path, from, len)
            .map(|_| ())
            .map_err(|error| build_error(error, "text_delete"))
    }

    #[wasm_bindgen(js_name = textReplace)]
    pub fn text_replace(
        &mut self,
        path: &str,
        from: usize,
        to: usize,
        value: &str,
    ) -> Result<(), JsValue> {
        let path = parse_path(path, "text_replace")?;
        let (from, to) = {
            let text = text_at_path(
                self.builder_ref("text_replace")?.current(),
                &path,
                "text_replace",
            )?;
            (
                utf16_to_code_point(text, from, "text_replace")?,
                utf16_to_code_point(text, to, "text_replace")?,
            )
        };
        let len = to.checked_sub(from).ok_or_else(|| {
            wasm_error_details(
                "invalid_argument",
                "text_replace",
                json!({ "argument": "range", "reason": "from must not exceed to" }),
            )
        })?;
        self.builder_mut()?
            .text_replace(&path, from, len, value)
            .map(|_| ())
            .map_err(|error| build_error(error, "text_replace"))
    }

    #[wasm_bindgen(js_name = richTextInsertText)]
    pub fn rich_text_insert_text(
        &mut self,
        path: &str,
        position: usize,
        value: &str,
        attrs: &str,
    ) -> Result<(), JsValue> {
        let path = parse_path(path, "rich_text_insert_text")?;
        let index = {
            let rich = rich_text_at_path(
                self.builder_ref("rich_text_insert_text")?.current(),
                &path,
                "rich_text_insert_text",
            )?;
            rich_utf16_to_code_point(rich, position, "rich_text_insert_text")?
        };
        let attrs = parse_attrs(attrs, "rich_text_insert_text")?;
        self.builder_mut()?
            .rich_text_insert_text(&path, index, value, attrs)
            .map(|_| ())
            .map_err(|error| build_error(error, "rich_text_insert_text"))
    }

    #[wasm_bindgen(js_name = richTextInsertEmbed)]
    pub fn rich_text_insert_embed(
        &mut self,
        path: &str,
        position: usize,
        value: &ValueHandle,
        attrs: &str,
    ) -> Result<(), JsValue> {
        let path = parse_path(path, "rich_text_insert_embed")?;
        let index = {
            let rich = rich_text_at_path(
                self.builder_ref("rich_text_insert_embed")?.current(),
                &path,
                "rich_text_insert_embed",
            )?;
            rich_utf16_to_code_point(rich, position, "rich_text_insert_embed")?
        };
        let attrs = parse_attrs(attrs, "rich_text_insert_embed")?;
        self.builder_mut()?
            .rich_text_insert_embed(&path, index, value.value.clone(), attrs)
            .map(|_| ())
            .map_err(|error| build_error(error, "rich_text_insert_embed"))
    }

    #[wasm_bindgen(js_name = richTextDelete)]
    pub fn rich_text_delete(&mut self, path: &str, from: usize, to: usize) -> Result<(), JsValue> {
        let path = parse_path(path, "rich_text_delete")?;
        let (from, to) = {
            let rich = rich_text_at_path(
                self.builder_ref("rich_text_delete")?.current(),
                &path,
                "rich_text_delete",
            )?;
            (
                rich_utf16_to_code_point(rich, from, "rich_text_delete")?,
                rich_utf16_to_code_point(rich, to, "rich_text_delete")?,
            )
        };
        let len = to.checked_sub(from).ok_or_else(|| {
            wasm_error_details(
                "invalid_argument",
                "rich_text_delete",
                json!({ "argument": "range", "reason": "from must not exceed to" }),
            )
        })?;
        self.builder_mut()?
            .rich_text_delete(&path, from, len)
            .map(|_| ())
            .map_err(|error| build_error(error, "rich_text_delete"))
    }

    #[wasm_bindgen(js_name = richTextFormat)]
    pub fn rich_text_format(
        &mut self,
        path: &str,
        from: usize,
        to: usize,
        patch: &str,
    ) -> Result<(), JsValue> {
        let path = parse_path(path, "rich_text_format")?;
        let (from, to) = {
            let rich = rich_text_at_path(
                self.builder_ref("rich_text_format")?.current(),
                &path,
                "rich_text_format",
            )?;
            (
                rich_utf16_to_code_point(rich, from, "rich_text_format")?,
                rich_utf16_to_code_point(rich, to, "rich_text_format")?,
            )
        };
        let len = to.checked_sub(from).ok_or_else(|| {
            wasm_error_details(
                "invalid_argument",
                "rich_text_format",
                json!({ "argument": "range", "reason": "from must not exceed to" }),
            )
        })?;
        let patch = parse_attr_patch(patch, "rich_text_format")?;
        self.builder_mut()?
            .rich_text_format(&path, from, len, patch)
            .map(|_| ())
            .map_err(|error| build_error(error, "rich_text_format"))
    }

    #[wasm_bindgen(js_name = intAdd)]
    pub fn int_add(&mut self, path: &str, delta: i64) -> Result<(), JsValue> {
        let path = parse_path(path, "int_add")?;
        self.builder_mut()?
            .int_add(&path, delta)
            .map(|_| ())
            .map_err(|error| build_error(error, "int_add"))
    }

    #[wasm_bindgen(js_name = cloneForScope)]
    pub fn clone_for_scope(&self) -> Result<Self, JsValue> {
        let builder = self.builder.as_ref().ok_or_else(|| {
            wasm_error("invalid_state", "builder_scope", "builder already consumed")
        })?;
        Ok(Self {
            builder: Some(builder.clone()),
        })
    }

    #[wasm_bindgen(js_name = commitScope)]
    pub fn commit_scope(&mut self, scope: &mut BuilderHandle) -> Result<(), JsValue> {
        self.builder_mut()?;
        let next = scope
            .builder
            .take()
            .ok_or_else(|| wasm_error("invalid_state", "builder_scope", "scope already closed"))?;
        self.builder = Some(next);
        Ok(())
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
    fn builder_ref(&self, operation: &'static str) -> Result<&ChangeBuilder, JsValue> {
        self.builder
            .as_ref()
            .ok_or_else(|| wasm_error("invalid_state", operation, "builder already consumed"))
    }

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

fn build_error(error: BuildError, operation: &'static str) -> JsValue {
    match error {
        BuildError::Apply(error) => apply_error(error, operation),
        BuildError::Compose(ComposeError::Apply(error)) => apply_error(error, operation),
        BuildError::Compose(error) => wasm_error_details(
            "incompatible_change",
            operation,
            json!({ "reason": error.to_string() }),
        ),
        _ => wasm_error("invalid_argument", operation, error.to_string()),
    }
}

fn compose_error(error: ComposeError, operation: &'static str) -> JsValue {
    match error {
        ComposeError::Apply(error) => apply_error(error, operation),
        ComposeError::IncompatibleKinds { left, right } => wasm_error_details(
            "incompatible_change",
            operation,
            json!({ "reason": "kind_mismatch", "left": left, "right": right }),
        ),
        ComposeError::IncompatibleMapEntry(key) => wasm_error_details(
            "incompatible_change",
            operation,
            json!({ "reason": "map_entry_conflict", "key": key }),
        ),
        _ => wasm_error("incompatible_change", operation, error.to_string()),
    }
}

fn transform_error(error: TransformError, operation: &'static str) -> JsValue {
    match error {
        TransformError::IncompatibleKinds { left, right } => wasm_error_details(
            "incompatible_change",
            operation,
            json!({ "reason": "kind_mismatch", "left": left, "right": right }),
        ),
        TransformError::IncompatibleMapEntry(key) => wasm_error_details(
            "incompatible_change",
            operation,
            json!({ "reason": "map_entry_conflict", "key": key }),
        ),
        TransformError::LengthOverflow => wasm_error_details(
            "incompatible_change",
            operation,
            json!({ "reason": "length_overflow" }),
        ),
        _ => wasm_error("incompatible_change", operation, error.to_string()),
    }
}

fn invert_error(error: InvertError, operation: &'static str) -> JsValue {
    match error {
        InvertError::Apply(error) => apply_error(error, operation),
        _ => wasm_error("incompatible_change", operation, error.to_string()),
    }
}

fn apply_error(error: ApplyError, operation: &'static str) -> JsValue {
    match error {
        ApplyError::TypeMismatch {
            expected, actual, ..
        } => wasm_error_details(
            "type_mismatch",
            operation,
            json!({ "expected": value_type_name(expected), "actual": value_type_name(actual) }),
        ),
        ApplyError::MissingKey { key, .. } => {
            wasm_error_details("missing_key", operation, json!({ "key": key }))
        }
        ApplyError::ExistingKey { key, .. } => {
            wasm_error_details("key_already_exists", operation, json!({ "key": key }))
        }
        ApplyError::IndexOutOfBounds { index, len, .. } => wasm_error_details(
            "out_of_bounds",
            operation,
            json!({ "target": "list", "length": len, "index": index }),
        ),
        ApplyError::SequenceOutOfBounds { .. } => wasm_error_details(
            "out_of_bounds",
            operation,
            json!({ "target": "sequence", "length": 0 }),
        ),
        ApplyError::IntegerOverflow { .. } => {
            wasm_error_details("integer_overflow", operation, json!({}))
        }
        _ => wasm_error("invalid_argument", operation, error.to_string()),
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

fn parse_attrs(value: &str, operation: &'static str) -> Result<Attrs, JsValue> {
    let entries = parse_attr_entries(value, operation)?;
    let mut attrs = Vec::with_capacity(entries.len());
    for (key, _, value) in entries {
        let value = value.ok_or_else(|| {
            wasm_error("invalid_value", operation, "attribute entry cannot remove")
        })?;
        attrs.push((key, value));
    }
    Attrs::from_entries(attrs)
        .map_err(|error| wasm_error("invalid_value", operation, error.to_string()))
}

fn parse_attr_patch(value: &str, operation: &'static str) -> Result<AttrPatch, JsValue> {
    let entries = parse_attr_entries(value, operation)?;
    AttrPatch::from_entries(entries.into_iter().map(|(key, action, value)| {
        let change = if action == "remove" {
            AttrChange::Remove
        } else {
            AttrChange::Set(value.expect("set entries have a value"))
        };
        (key, change)
    }))
    .map_err(|error| wasm_error("invalid_value", operation, error.to_string()))
}

fn parse_attr_entries(
    value: &str,
    operation: &'static str,
) -> Result<Vec<(String, String, Option<AttrValue>)>, JsValue> {
    let value: JsonValue = serde_json::from_str(value)
        .map_err(|error| wasm_error("invalid_value", operation, error.to_string()))?;
    let entries = value
        .as_array()
        .ok_or_else(|| wasm_error("invalid_value", operation, "attributes must be an array"))?;
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let key = entry
            .get("key")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| wasm_error("invalid_value", operation, "attribute key is missing"))?
            .to_owned();
        let action = entry
            .get("action")
            .and_then(JsonValue::as_str)
            .unwrap_or("set")
            .to_owned();
        if action == "remove" {
            out.push((key, action, None));
            continue;
        }
        if action != "set" {
            return Err(wasm_error(
                "invalid_value",
                operation,
                "unknown attribute patch action",
            ));
        }
        let kind = entry
            .get("kind")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| wasm_error("invalid_value", operation, "attribute kind is missing"))?;
        let data = entry
            .get("value")
            .ok_or_else(|| wasm_error("invalid_value", operation, "attribute value is missing"))?;
        let value =
            match kind {
                "bool" => AttrValue::Bool(data.as_bool().ok_or_else(|| {
                    wasm_error("invalid_value", operation, "invalid Bool attribute")
                })?),
                "int" => AttrValue::Int(
                    data.as_str()
                        .and_then(|value| value.parse::<i64>().ok())
                        .ok_or_else(|| {
                            wasm_error("invalid_value", operation, "invalid Int attribute")
                        })?,
                ),
                "float" => AttrValue::float(data.as_f64().ok_or_else(|| {
                    wasm_error("invalid_value", operation, "invalid Float attribute")
                })?)
                .map_err(|error| wasm_error("invalid_value", operation, error.to_string()))?,
                "string" => AttrValue::string(data.as_str().ok_or_else(|| {
                    wasm_error("invalid_value", operation, "invalid String attribute")
                })?),
                _ => {
                    return Err(wasm_error(
                        "invalid_value",
                        operation,
                        "unknown attribute kind",
                    ))
                }
            };
        out.push((key, action, Some(value)));
    }
    Ok(out)
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

fn text_at_path<'a>(
    root: &'a Value,
    path: &Path,
    operation: &'static str,
) -> Result<&'a str, JsValue> {
    let value = value_at_path(root, path, operation)?;
    match value.kind() {
        ValueKind::Text(text) => Ok(text.as_str()),
        actual => Err(wasm_error_details(
            "type_mismatch",
            operation,
            json!({ "expected": "text", "actual": value_kind_name(actual) }),
        )),
    }
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

fn rich_text_at_path<'a>(
    root: &'a Value,
    path: &Path,
    operation: &'static str,
) -> Result<&'a RichText, JsValue> {
    let value = value_at_path(root, path, operation)?;
    match value.kind() {
        ValueKind::RichText(rich) => Ok(rich),
        actual => Err(wasm_error_details(
            "type_mismatch",
            operation,
            json!({ "expected": "richText", "actual": value_kind_name(actual) }),
        )),
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
