use std::collections::BTreeMap;

use crate::attrs::{AttrChange, AttrPatch, AttrValue, Attrs};
use crate::change::{
    Change, ChangeKind, IntChange, ListChange, ListOp, MapChange, MapEntryChange, RichTextChange,
    RichTextOp, TextChange, TextOp,
};
use crate::error::CodecError;
use crate::limits::Limits;
use crate::richtext::{RichInsert, RichSpan, RichText};
use crate::value::{FiniteF64, Value, ValueKind};

pub fn encode_value(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode_value_into(value, &mut out);
    out
}

pub fn decode_value(bytes: &[u8], limits: &Limits) -> Result<Value, CodecError> {
    let mut decoder = Decoder::new(bytes, limits);
    let value = decoder.value(1)?;
    decoder.finish()?;
    Ok(value)
}

pub fn encode_change(change: &Change) -> Vec<u8> {
    let mut out = Vec::new();
    encode_change_into(change, &mut out);
    out
}

pub fn decode_change(bytes: &[u8], limits: &Limits) -> Result<Change, CodecError> {
    let mut decoder = Decoder::new(bytes, limits);
    let change = decoder.change(1)?;
    decoder.finish()?;
    if let Err(crate::ApplyError::LimitExceeded {
        name,
        actual,
        limit,
    }) = change.check_limits(limits)
    {
        return Err(CodecError::LimitExceeded {
            name,
            actual,
            limit,
        });
    }
    Ok(change)
}

fn put_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}
fn put_i64(value: i64, out: &mut Vec<u8>) {
    put_varint(((value << 1) ^ (value >> 63)) as u64, out);
}
fn put_string(value: &str, out: &mut Vec<u8>) {
    put_varint(value.len() as u64, out);
    out.extend_from_slice(value.as_bytes());
}

fn encode_value_into(value: &Value, out: &mut Vec<u8>) {
    match value.kind() {
        ValueKind::Null => out.push(0),
        ValueKind::Bool(false) => out.push(1),
        ValueKind::Bool(true) => out.push(2),
        ValueKind::Int(value) => {
            out.push(3);
            put_i64(*value, out);
        }
        ValueKind::Float(value) => {
            out.push(4);
            out.extend_from_slice(&value.get().to_le_bytes());
        }
        ValueKind::String(value) => {
            out.push(5);
            put_string(value, out);
        }
        ValueKind::Text(value) => {
            out.push(6);
            put_string(value.as_str(), out);
        }
        ValueKind::RichText(value) => {
            out.push(7);
            put_varint(value.spans().len() as u64, out);
            for span in value.spans() {
                encode_rich_insert(&span.content, out);
                encode_attrs(&span.attrs, out);
            }
        }
        ValueKind::List(value) => {
            out.push(8);
            put_varint(value.len() as u64, out);
            for item in value.as_slice() {
                encode_value_into(item, out);
            }
        }
        ValueKind::Map(value) => {
            out.push(9);
            put_varint(value.len() as u64, out);
            for (key, item) in value.iter() {
                put_string(key, out);
                encode_value_into(item, out);
            }
        }
    }
}

fn encode_attrs(attrs: &Attrs, out: &mut Vec<u8>) {
    put_varint(attrs.len() as u64, out);
    for (key, value) in attrs.iter() {
        put_string(key, out);
        encode_attr_value(value, out);
    }
}
fn encode_attr_value(value: &AttrValue, out: &mut Vec<u8>) {
    match value {
        AttrValue::Bool(false) => out.push(0),
        AttrValue::Bool(true) => out.push(1),
        AttrValue::Int(value) => {
            out.push(2);
            put_i64(*value, out);
        }
        AttrValue::Float(value) => {
            out.push(3);
            out.extend_from_slice(&value.get().to_le_bytes());
        }
        AttrValue::String(value) => {
            out.push(4);
            put_string(value, out);
        }
    }
}
fn encode_rich_insert(content: &RichInsert, out: &mut Vec<u8>) {
    match content {
        RichInsert::Text(text) => {
            out.push(0);
            put_string(text, out);
        }
        RichInsert::Embed(value) => {
            out.push(1);
            encode_value_into(value, out);
        }
    }
}

fn encode_change_into(change: &Change, out: &mut Vec<u8>) {
    match change.kind() {
        ChangeKind::Noop => out.push(0),
        ChangeKind::Replace(value) => {
            out.push(1);
            encode_value_into(value, out);
        }
        ChangeKind::Map(value) => {
            out.push(2);
            put_varint(value.len() as u64, out);
            for (key, entry) in value.iter() {
                put_string(key, out);
                match entry {
                    MapEntryChange::Insert(value) => {
                        out.push(0);
                        encode_value_into(value, out);
                    }
                    MapEntryChange::Delete => out.push(1),
                    MapEntryChange::Modify(child) => {
                        out.push(2);
                        encode_change_into(child, out);
                    }
                }
            }
        }
        ChangeKind::List(value) => {
            out.push(3);
            put_varint(value.ops().len() as u64, out);
            for op in value.ops() {
                match op {
                    ListOp::Retain(len) => {
                        out.push(0);
                        put_varint(*len as u64, out);
                    }
                    ListOp::Insert(values) => {
                        out.push(1);
                        put_varint(values.len() as u64, out);
                        for value in values {
                            encode_value_into(value, out);
                        }
                    }
                    ListOp::Delete(len) => {
                        out.push(2);
                        put_varint(*len as u64, out);
                    }
                    ListOp::Modify(child) => {
                        out.push(3);
                        encode_change_into(child, out);
                    }
                }
            }
        }
        ChangeKind::Text(value) => {
            out.push(4);
            put_varint(value.ops().len() as u64, out);
            for op in value.ops() {
                match op {
                    TextOp::Retain(len) => {
                        out.push(0);
                        put_varint(*len as u64, out);
                    }
                    TextOp::Insert(value) => {
                        out.push(1);
                        put_string(value, out);
                    }
                    TextOp::Delete(len) => {
                        out.push(2);
                        put_varint(*len as u64, out);
                    }
                }
            }
        }
        ChangeKind::RichText(value) => {
            out.push(5);
            put_varint(value.ops().len() as u64, out);
            for op in value.ops() {
                match op {
                    RichTextOp::Retain { len, attrs } => {
                        out.push(0);
                        put_varint(*len as u64, out);
                        encode_attr_patch(attrs, out);
                    }
                    RichTextOp::Insert { content, attrs } => {
                        out.push(1);
                        encode_rich_insert(content, out);
                        encode_attrs(attrs, out);
                    }
                    RichTextOp::Delete(len) => {
                        out.push(2);
                        put_varint(*len as u64, out);
                    }
                }
            }
        }
        ChangeKind::Int(IntChange::Add(delta)) => {
            out.push(6);
            put_i64(*delta, out);
        }
    }
}

fn encode_attr_patch(patch: &AttrPatch, out: &mut Vec<u8>) {
    put_varint(patch.len() as u64, out);
    for (key, change) in patch.iter() {
        put_string(key, out);
        match change {
            AttrChange::Set(value) => {
                out.push(0);
                encode_attr_value(value, out);
            }
            AttrChange::Remove => out.push(1),
        }
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    pos: usize,
    limits: &'a Limits,
    value_nodes: usize,
    change_nodes: usize,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8], limits: &'a Limits) -> Self {
        Self {
            bytes,
            pos: 0,
            limits,
            value_nodes: 0,
            change_nodes: 0,
        }
    }
    fn finish(&self) -> Result<(), CodecError> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(CodecError::TrailingBytes { offset: self.pos })
        }
    }
    fn byte(&mut self) -> Result<u8, CodecError> {
        let value = *self
            .bytes
            .get(self.pos)
            .ok_or(CodecError::UnexpectedEof { offset: self.pos })?;
        self.pos += 1;
        Ok(value)
    }
    fn exact<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        let end = self
            .pos
            .checked_add(N)
            .ok_or(CodecError::UnexpectedEof { offset: self.pos })?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(CodecError::UnexpectedEof { offset: self.pos })?;
        self.pos = end;
        Ok(slice.try_into().unwrap())
    }
    fn varint(&mut self) -> Result<u64, CodecError> {
        let start = self.pos;
        let mut value = 0u64;
        for index in 0..10 {
            let byte = self.byte()?;
            if index == 9 && byte > 1 {
                return Err(CodecError::IntegerOutOfRange { offset: start });
            }
            value |= ((byte & 0x7f) as u64) << (index * 7);
            if byte & 0x80 == 0 {
                if index > 0 && byte == 0 {
                    return Err(CodecError::NonMinimalVarint { offset: start });
                }
                return Ok(value);
            }
        }
        Err(CodecError::IntegerOutOfRange { offset: start })
    }
    fn usize(&mut self) -> Result<usize, CodecError> {
        usize::try_from(self.varint()?)
            .map_err(|_| CodecError::IntegerOutOfRange { offset: self.pos })
    }
    fn i64(&mut self) -> Result<i64, CodecError> {
        let value = self.varint()?;
        Ok(((value >> 1) as i64) ^ (-((value & 1) as i64)))
    }
    fn string(&mut self) -> Result<String, CodecError> {
        let start = self.pos;
        let len = self.usize()?;
        self.limit("string bytes", len, self.limits.max_string_bytes)?;
        let end = self
            .pos
            .checked_add(len)
            .ok_or(CodecError::UnexpectedEof { offset: self.pos })?;
        let bytes = self
            .bytes
            .get(self.pos..end)
            .ok_or(CodecError::UnexpectedEof { offset: self.pos })?;
        let value = std::str::from_utf8(bytes)
            .map_err(|_| CodecError::InvalidUtf8 { offset: start })?
            .to_owned();
        self.pos = end;
        Ok(value)
    }
    fn count(&mut self, name: &'static str, limit: usize) -> Result<usize, CodecError> {
        let value = self.usize()?;
        self.limit(name, value, limit)?;
        Ok(value)
    }
    fn limit(&self, name: &'static str, actual: usize, limit: usize) -> Result<(), CodecError> {
        if actual > limit {
            Err(CodecError::LimitExceeded {
                name,
                actual,
                limit,
            })
        } else {
            Ok(())
        }
    }
    fn depth(&self, depth: usize) -> Result<(), CodecError> {
        self.limit("depth", depth, self.limits.max_depth)
    }
    fn tag_error(&self, offset: usize, tag: u8, context: &'static str) -> CodecError {
        CodecError::UnknownTag {
            offset,
            tag,
            context,
        }
    }
    fn noncanonical(
        &self,
        offset: usize,
        context: &'static str,
        reason: &'static str,
    ) -> CodecError {
        CodecError::NonCanonical {
            offset,
            context,
            reason,
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, CodecError> {
        self.depth(depth)?;
        self.value_nodes += 1;
        self.limit("value nodes", self.value_nodes, self.limits.max_value_nodes)?;
        let offset = self.pos;
        let tag = self.byte()?;
        match tag {
            0 => Ok(Value::null()),
            1 => Ok(Value::bool(false)),
            2 => Ok(Value::bool(true)),
            3 => Ok(Value::int(self.i64()?)),
            4 => {
                let bits = u64::from_le_bytes(self.exact()?);
                let value = f64::from_bits(bits);
                if value == 0.0 && bits >> 63 == 1 {
                    return Err(self.noncanonical(offset, "Float", "negative zero"));
                }
                Ok(Value::finite_float(FiniteF64::new(value)?))
            }
            5 => Ok(Value::string(self.string()?)),
            6 => Ok(Value::text(self.string()?)),
            7 => {
                let count = self.count("container length", self.limits.max_container_len)?;
                let mut spans = Vec::with_capacity(count);
                for _ in 0..count {
                    let content = self.rich_insert(depth + 1)?;
                    let attrs = self.attrs()?;
                    spans.push(RichSpan { content, attrs });
                }
                let normalized = RichText::new(spans.clone());
                if normalized.spans() != spans.as_slice() {
                    return Err(self.noncanonical(offset, "RichText", "non-canonical spans"));
                }
                Ok(Value::rich_text(RichText::from_canonical(spans)))
            }
            8 => {
                let count = self.count("container length", self.limits.max_container_len)?;
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(self.value(depth + 1)?);
                }
                Ok(Value::list(values))
            }
            9 => {
                let count = self.count("container length", self.limits.max_container_len)?;
                let mut map = BTreeMap::new();
                let mut previous: Option<String> = None;
                for _ in 0..count {
                    let key_offset = self.pos;
                    let key = self.string()?;
                    if previous.as_ref().is_some_and(|p| p >= &key) {
                        return Err(self.noncanonical(
                            key_offset,
                            "Map",
                            "keys not strictly increasing",
                        ));
                    }
                    previous = Some(key.clone());
                    map.insert(key, self.value(depth + 1)?);
                }
                Ok(Value::from_kind(ValueKind::Map(crate::Map::from_btree(
                    map,
                ))))
            }
            _ => Err(self.tag_error(offset, tag, "Value")),
        }
    }

    fn attr_value(&mut self) -> Result<AttrValue, CodecError> {
        let offset = self.pos;
        match self.byte()? {
            0 => Ok(AttrValue::Bool(false)),
            1 => Ok(AttrValue::Bool(true)),
            2 => Ok(AttrValue::Int(self.i64()?)),
            3 => {
                let bits = u64::from_le_bytes(self.exact()?);
                let value = f64::from_bits(bits);
                if value == 0.0 && bits >> 63 == 1 {
                    return Err(self.noncanonical(offset, "AttrValue", "negative zero"));
                }
                Ok(AttrValue::Float(FiniteF64::new(value)?))
            }
            4 => Ok(AttrValue::string(self.string()?)),
            tag => Err(self.tag_error(offset, tag, "AttrValue")),
        }
    }
    fn attrs(&mut self) -> Result<Attrs, CodecError> {
        let count = self.count("container length", self.limits.max_container_len)?;
        let mut map = BTreeMap::new();
        let mut previous: Option<String> = None;
        for _ in 0..count {
            let offset = self.pos;
            let key = self.string()?;
            if previous.as_ref().is_some_and(|p| p >= &key) {
                return Err(self.noncanonical(offset, "Attrs", "keys not strictly increasing"));
            }
            previous = Some(key.clone());
            map.insert(key, self.attr_value()?);
        }
        Ok(Attrs::from_btree(map))
    }
    fn attr_patch(&mut self) -> Result<AttrPatch, CodecError> {
        let count = self.count("container length", self.limits.max_container_len)?;
        let mut map = BTreeMap::new();
        let mut previous: Option<String> = None;
        for _ in 0..count {
            let offset = self.pos;
            let key = self.string()?;
            if previous.as_ref().is_some_and(|p| p >= &key) {
                return Err(self.noncanonical(offset, "AttrPatch", "keys not strictly increasing"));
            }
            previous = Some(key.clone());
            let tag_offset = self.pos;
            let change = match self.byte()? {
                0 => AttrChange::Set(self.attr_value()?),
                1 => AttrChange::Remove,
                tag => return Err(self.tag_error(tag_offset, tag, "AttrChange")),
            };
            map.insert(key, change);
        }
        Ok(AttrPatch::from_btree(map))
    }
    fn rich_insert(&mut self, depth: usize) -> Result<RichInsert, CodecError> {
        let offset = self.pos;
        match self.byte()? {
            0 => {
                let value = self.string()?;
                if value.is_empty() {
                    Err(self.noncanonical(offset, "RichInsert", "empty text"))
                } else {
                    Ok(RichInsert::text(value))
                }
            }
            1 => Ok(RichInsert::embed(self.value(depth)?)),
            tag => Err(self.tag_error(offset, tag, "RichInsert")),
        }
    }

    fn change(&mut self, depth: usize) -> Result<Change, CodecError> {
        self.depth(depth)?;
        self.change_nodes += 1;
        self.limit(
            "change nodes",
            self.change_nodes,
            self.limits.max_change_nodes,
        )?;
        let offset = self.pos;
        let tag = self.byte()?;
        match tag {
            0 => Ok(Change::noop()),
            1 => Ok(Change::replace(self.value(depth + 1)?)),
            2 => {
                let count = self.count("container length", self.limits.max_container_len)?;
                if count == 0 {
                    return Err(self.noncanonical(offset, "MapChange", "empty change"));
                }
                let mut map = BTreeMap::new();
                let mut previous: Option<String> = None;
                for _ in 0..count {
                    let key_offset = self.pos;
                    let key = self.string()?;
                    if previous.as_ref().is_some_and(|p| p >= &key) {
                        return Err(self.noncanonical(
                            key_offset,
                            "MapChange",
                            "keys not strictly increasing",
                        ));
                    }
                    previous = Some(key.clone());
                    let entry_offset = self.pos;
                    let entry = match self.byte()? {
                        0 => MapEntryChange::Insert(self.value(depth + 1)?),
                        1 => MapEntryChange::Delete,
                        2 => {
                            let child = self.change(depth + 1)?;
                            if child.is_noop() {
                                return Err(self.noncanonical(
                                    entry_offset,
                                    "MapChange",
                                    "Modify(Noop)",
                                ));
                            }
                            MapEntryChange::Modify(child)
                        }
                        tag => return Err(self.tag_error(entry_offset, tag, "MapEntryChange")),
                    };
                    map.insert(key, entry);
                }
                Ok(Change::map(MapChange::from_btree(map)))
            }
            3 => {
                let count = self.count("sequence ops", self.limits.max_sequence_ops)?;
                if count == 0 {
                    return Err(self.noncanonical(offset, "ListChange", "empty change"));
                }
                let mut ops = Vec::with_capacity(count);
                for _ in 0..count {
                    let op_offset = self.pos;
                    ops.push(match self.byte()? {
                        0 => {
                            let n = self.usize()?;
                            if n == 0 {
                                return Err(self.noncanonical(
                                    op_offset,
                                    "ListChange",
                                    "zero retain",
                                ));
                            }
                            ListOp::Retain(n)
                        }
                        1 => {
                            let n =
                                self.count("container length", self.limits.max_container_len)?;
                            if n == 0 {
                                return Err(self.noncanonical(
                                    op_offset,
                                    "ListChange",
                                    "empty insert",
                                ));
                            }
                            let mut values = Vec::with_capacity(n);
                            for _ in 0..n {
                                values.push(self.value(depth + 1)?);
                            }
                            ListOp::Insert(values)
                        }
                        2 => {
                            let n = self.usize()?;
                            if n == 0 {
                                return Err(self.noncanonical(
                                    op_offset,
                                    "ListChange",
                                    "zero delete",
                                ));
                            }
                            ListOp::Delete(n)
                        }
                        3 => {
                            let child = self.change(depth + 1)?;
                            if child.is_noop() {
                                return Err(self.noncanonical(
                                    op_offset,
                                    "ListChange",
                                    "Modify(Noop)",
                                ));
                            }
                            ListOp::Modify(child)
                        }
                        tag => return Err(self.tag_error(op_offset, tag, "ListOp")),
                    });
                }
                let normalized = ListChange::new(ops.clone());
                if normalized.ops() != ops.as_slice() {
                    return Err(self.noncanonical(offset, "ListChange", "non-canonical ops"));
                }
                Ok(Change::list(ListChange::from_canonical(ops)))
            }
            4 => {
                let count = self.count("sequence ops", self.limits.max_sequence_ops)?;
                if count == 0 {
                    return Err(self.noncanonical(offset, "TextChange", "empty change"));
                }
                let mut ops = Vec::with_capacity(count);
                for _ in 0..count {
                    let op_offset = self.pos;
                    ops.push(match self.byte()? {
                        0 => {
                            let n = self.usize()?;
                            if n == 0 {
                                return Err(self.noncanonical(
                                    op_offset,
                                    "TextChange",
                                    "zero retain",
                                ));
                            }
                            TextOp::Retain(n)
                        }
                        1 => {
                            let value = self.string()?;
                            if value.is_empty() {
                                return Err(self.noncanonical(
                                    op_offset,
                                    "TextChange",
                                    "empty insert",
                                ));
                            }
                            TextOp::Insert(value)
                        }
                        2 => {
                            let n = self.usize()?;
                            if n == 0 {
                                return Err(self.noncanonical(
                                    op_offset,
                                    "TextChange",
                                    "zero delete",
                                ));
                            }
                            TextOp::Delete(n)
                        }
                        tag => return Err(self.tag_error(op_offset, tag, "TextOp")),
                    });
                }
                let normalized = TextChange::new(ops.clone());
                if normalized.ops() != ops.as_slice() {
                    return Err(self.noncanonical(offset, "TextChange", "non-canonical ops"));
                }
                Ok(Change::text(TextChange::from_canonical(ops)))
            }
            5 => {
                let count = self.count("sequence ops", self.limits.max_sequence_ops)?;
                if count == 0 {
                    return Err(self.noncanonical(offset, "RichTextChange", "empty change"));
                }
                let mut ops = Vec::with_capacity(count);
                for _ in 0..count {
                    let op_offset = self.pos;
                    ops.push(match self.byte()? {
                        0 => {
                            let n = self.usize()?;
                            if n == 0 {
                                return Err(self.noncanonical(
                                    op_offset,
                                    "RichTextChange",
                                    "zero retain",
                                ));
                            }
                            RichTextOp::Retain {
                                len: n,
                                attrs: self.attr_patch()?,
                            }
                        }
                        1 => RichTextOp::Insert {
                            content: self.rich_insert(depth + 1)?,
                            attrs: self.attrs()?,
                        },
                        2 => {
                            let n = self.usize()?;
                            if n == 0 {
                                return Err(self.noncanonical(
                                    op_offset,
                                    "RichTextChange",
                                    "zero delete",
                                ));
                            }
                            RichTextOp::Delete(n)
                        }
                        tag => return Err(self.tag_error(op_offset, tag, "RichTextOp")),
                    });
                }
                let normalized = RichTextChange::new(ops.clone());
                if normalized.ops() != ops.as_slice() {
                    return Err(self.noncanonical(offset, "RichTextChange", "non-canonical ops"));
                }
                Ok(Change::rich_text(RichTextChange::from_canonical(ops)))
            }
            6 => {
                let delta = self.i64()?;
                if delta == 0 {
                    return Err(self.noncanonical(offset, "IntChange", "Add(0)"));
                }
                Ok(Change::int_add(delta))
            }
            _ => Err(self.tag_error(offset, tag, "Change")),
        }
    }
}
