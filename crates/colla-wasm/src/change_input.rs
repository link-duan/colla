use colla::{
    AttrChange, AttrPatch, AttrValue, Attrs, Change, CodecError, InputLimits, IntChange,
    ListChange, ListOp, MapChange, MapEntryChange, RichContent, RichTextChange, RichTextOp,
    TextChange, TextOp, Value, ValueError, ValueKind,
};

#[derive(Debug)]
pub(crate) enum ChangeInputError {
    Encoding {
        offset: usize,
        reason: &'static str,
    },
    Limit {
        name: &'static str,
        actual: usize,
        maximum: usize,
    },
    Value(ValueError),
    ValueCodec(CodecError),
}

impl From<ValueError> for ChangeInputError {
    fn from(error: ValueError) -> Self {
        Self::Value(error)
    }
}

pub(crate) fn decode_change_input(
    bytes: &[u8],
    limits: &InputLimits,
) -> Result<Change, ChangeInputError> {
    let mut decoder = Decoder {
        bytes,
        pos: 0,
        limits,
        value_nodes: 0,
        change_nodes: 0,
    };
    let change = decoder.change(1)?;
    if decoder.pos != bytes.len() {
        return Err(decoder.encoding("trailing ChangeInput bytes"));
    }
    Ok(change)
}

struct Decoder<'a> {
    bytes: &'a [u8],
    pos: usize,
    limits: &'a InputLimits,
    value_nodes: usize,
    change_nodes: usize,
}

impl Decoder<'_> {
    fn change(&mut self, depth: usize) -> Result<Change, ChangeInputError> {
        self.check_depth(depth)?;
        self.change_nodes = self.change_nodes.checked_add(1).ok_or_else(|| {
            self.limit_error("change nodes", usize::MAX, self.limits.max_change_nodes)
        })?;
        self.check_limit(
            "change nodes",
            self.change_nodes,
            self.limits.max_change_nodes,
        )?;

        match self.byte()? {
            0 => Ok(Change::noop()),
            1 => Ok(Change::replace(self.value(depth + 1)?)),
            2 => {
                let count = self.count("container length", self.limits.max_container_len)?;
                let mut entries = Vec::with_capacity(count);
                for _ in 0..count {
                    let key = self.string()?;
                    let entry = match self.byte()? {
                        0 => MapEntryChange::Insert(self.value(depth + 1)?),
                        1 => MapEntryChange::Delete,
                        2 => MapEntryChange::Modify(self.change(depth + 1)?),
                        _ => return Err(self.encoding("unknown MapChange entry type")),
                    };
                    entries.push((key, entry));
                }
                Ok(MapChange::from_entries(entries)?.into())
            }
            3 => {
                let count = self.count("sequence ops", self.limits.max_sequence_ops)?;
                let mut lengths = SequenceLengths::default();
                let mut ops = Vec::with_capacity(count);
                for _ in 0..count {
                    match self.byte()? {
                        0 => {
                            let len = self.usize()?;
                            lengths.retain(len, self)?;
                            ops.push(ListOp::Retain(len));
                        }
                        1 => {
                            let count =
                                self.count("container length", self.limits.max_container_len)?;
                            lengths.insert(count, self)?;
                            let mut values = Vec::with_capacity(count);
                            for _ in 0..count {
                                values.push(self.value(depth + 1)?);
                            }
                            ops.push(ListOp::Insert(values));
                        }
                        2 => {
                            let len = self.usize()?;
                            lengths.delete(len, self)?;
                            ops.push(ListOp::Delete(len));
                        }
                        3 => {
                            lengths.retain(1, self)?;
                            ops.push(ListOp::Modify(self.change(depth + 1)?));
                        }
                        _ => return Err(self.encoding("unknown ListChange operation")),
                    }
                }
                Ok(ListChange::from_ops(ops)?.into())
            }
            4 => {
                let count = self.count("sequence ops", self.limits.max_sequence_ops)?;
                let mut lengths = SequenceLengths::default();
                let mut ops = Vec::with_capacity(count);
                for _ in 0..count {
                    match self.byte()? {
                        0 => {
                            let len = self.usize()?;
                            lengths.retain(len, self)?;
                            ops.push(TextOp::Retain(len));
                        }
                        1 => {
                            let text = self.string()?;
                            lengths.insert(text.chars().count(), self)?;
                            ops.push(TextOp::Insert(text));
                        }
                        2 => {
                            let len = self.usize()?;
                            lengths.delete(len, self)?;
                            ops.push(TextOp::Delete(len));
                        }
                        _ => return Err(self.encoding("unknown TextChange operation")),
                    }
                }
                Ok(TextChange::from_ops(ops)?.into())
            }
            5 => {
                let count = self.count("sequence ops", self.limits.max_sequence_ops)?;
                let mut lengths = SequenceLengths::default();
                let mut ops = Vec::with_capacity(count);
                for _ in 0..count {
                    match self.byte()? {
                        0 => {
                            let len = self.usize()?;
                            lengths.retain(len, self)?;
                            ops.push(RichTextOp::Retain {
                                len,
                                attrs: self.attr_patch()?,
                            });
                        }
                        1 => {
                            let content = match self.byte()? {
                                0 => RichContent::text(self.string()?),
                                1 => RichContent::embed(self.value(depth + 1)?),
                                _ => return Err(self.encoding("unknown RichText content type")),
                            };
                            lengths.insert(content.len(), self)?;
                            ops.push(RichTextOp::Insert {
                                content,
                                attrs: self.attrs()?,
                            });
                        }
                        2 => {
                            let len = self.usize()?;
                            lengths.delete(len, self)?;
                            ops.push(RichTextOp::Delete(len));
                        }
                        _ => return Err(self.encoding("unknown RichTextChange operation")),
                    }
                }
                Ok(RichTextChange::from_ops(ops)?.into())
            }
            6 => Ok(IntChange::Add(self.i64()?).into()),
            _ => Err(self.encoding("unknown ChangeInput type")),
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, ChangeInputError> {
        let bytes = self.blob()?;
        let value =
            Value::decode_with_limits(&bytes, self.limits).map_err(ChangeInputError::ValueCodec)?;
        self.count_value(&value, depth)?;
        Ok(value)
    }

    fn count_value(&mut self, value: &Value, depth: usize) -> Result<(), ChangeInputError> {
        self.check_depth(depth)?;
        self.value_nodes = self.value_nodes.checked_add(1).ok_or_else(|| {
            self.limit_error("value nodes", usize::MAX, self.limits.max_value_nodes)
        })?;
        self.check_limit("value nodes", self.value_nodes, self.limits.max_value_nodes)?;

        match value.kind() {
            ValueKind::String(value) => self.check_string(value)?,
            ValueKind::Text(value) => self.check_string(value.as_str())?,
            ValueKind::List(values) => {
                self.check_limit(
                    "container length",
                    values.len(),
                    self.limits.max_container_len,
                )?;
                for child in values.as_slice() {
                    self.count_value(child, depth + 1)?;
                }
            }
            ValueKind::Map(values) => {
                self.check_limit(
                    "container length",
                    values.len(),
                    self.limits.max_container_len,
                )?;
                for (key, child) in values.iter() {
                    self.check_string(key)?;
                    self.count_value(child, depth + 1)?;
                }
            }
            ValueKind::RichText(rich) => {
                self.check_limit(
                    "container length",
                    rich.span_count(),
                    self.limits.max_container_len,
                )?;
                for span in rich.iter_spans() {
                    self.count_attrs(span.attrs())?;
                    match span.content() {
                        RichContent::Text(text) => self.check_string(text.as_str())?,
                        RichContent::Embed(value) => self.count_value(value, depth + 1)?,
                    }
                }
            }
            ValueKind::Null | ValueKind::Bool(_) | ValueKind::Int(_) | ValueKind::Float(_) => {}
        }
        Ok(())
    }

    fn attrs(&mut self) -> Result<Attrs, ChangeInputError> {
        let count = self.count("container length", self.limits.max_container_len)?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push((self.string()?, self.attr_value()?));
        }
        Ok(Attrs::from_entries(entries)?)
    }

    fn attr_patch(&mut self) -> Result<AttrPatch, ChangeInputError> {
        let count = self.count("container length", self.limits.max_container_len)?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let key = self.string()?;
            let change = match self.byte()? {
                0 => AttrChange::Set(self.attr_value()?),
                1 => AttrChange::Remove,
                _ => return Err(self.encoding("unknown attribute patch action")),
            };
            entries.push((key, change));
        }
        Ok(AttrPatch::from_entries(entries)?)
    }

    fn attr_value(&mut self) -> Result<AttrValue, ChangeInputError> {
        match self.byte()? {
            0 => Ok(AttrValue::Bool(false)),
            1 => Ok(AttrValue::Bool(true)),
            2 => Ok(AttrValue::Int(self.i64()?)),
            3 => {
                let bytes = self.exact(8)?;
                AttrValue::float(f64::from_le_bytes(bytes.try_into().expect("eight bytes")))
                    .map_err(ChangeInputError::Value)
            }
            4 => Ok(AttrValue::string(self.string()?)),
            _ => Err(self.encoding("unknown attribute value type")),
        }
    }

    fn count_attrs(&self, attrs: &Attrs) -> Result<(), ChangeInputError> {
        self.check_limit(
            "container length",
            attrs.len(),
            self.limits.max_container_len,
        )?;
        for (key, value) in attrs.iter() {
            self.check_string(key)?;
            if let AttrValue::String(value) = value {
                self.check_string(value)?;
            }
        }
        Ok(())
    }

    fn byte(&mut self) -> Result<u8, ChangeInputError> {
        let byte = self
            .bytes
            .get(self.pos)
            .copied()
            .ok_or_else(|| self.encoding("unexpected end of ChangeInput"))?;
        self.pos += 1;
        Ok(byte)
    }

    fn exact(&mut self, len: usize) -> Result<&[u8], ChangeInputError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| self.encoding("length overflow"))?;
        let bytes = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| self.encoding("unexpected end of ChangeInput"))?;
        self.pos = end;
        Ok(bytes)
    }

    fn blob(&mut self) -> Result<Vec<u8>, ChangeInputError> {
        let len = self.usize()?;
        Ok(self.exact(len)?.to_vec())
    }

    fn string(&mut self) -> Result<String, ChangeInputError> {
        let bytes = self.blob()?;
        self.check_limit("string bytes", bytes.len(), self.limits.max_string_bytes)?;
        std::str::from_utf8(&bytes)
            .map(str::to_owned)
            .map_err(|_| self.encoding("invalid UTF-8"))
    }

    fn i64(&mut self) -> Result<i64, ChangeInputError> {
        let value = self.varint()?;
        Ok(((value >> 1) as i64) ^ -((value & 1) as i64))
    }

    fn usize(&mut self) -> Result<usize, ChangeInputError> {
        usize::try_from(self.varint()?).map_err(|_| self.encoding("integer is out of range"))
    }

    fn count(&mut self, name: &'static str, maximum: usize) -> Result<usize, ChangeInputError> {
        let count = self.usize()?;
        self.check_limit(name, count, maximum)?;
        Ok(count)
    }

    fn varint(&mut self) -> Result<u64, ChangeInputError> {
        let start = self.pos;
        let mut value = 0u64;
        for index in 0..10 {
            let byte = self.byte()?;
            if index == 9 && byte > 1 {
                return Err(self.encoding_at(start, "integer is out of range"));
            }
            value |= u64::from(byte & 0x7f) << (index * 7);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(self.encoding_at(start, "integer is out of range"))
    }

    fn check_depth(&self, depth: usize) -> Result<(), ChangeInputError> {
        self.check_limit("input depth", depth, self.limits.max_depth)
    }

    fn check_string(&self, value: &str) -> Result<(), ChangeInputError> {
        self.check_limit("string bytes", value.len(), self.limits.max_string_bytes)
    }

    fn check_limit(
        &self,
        name: &'static str,
        actual: usize,
        maximum: usize,
    ) -> Result<(), ChangeInputError> {
        if actual > maximum {
            Err(self.limit_error(name, actual, maximum))
        } else {
            Ok(())
        }
    }

    fn limit_error(&self, name: &'static str, actual: usize, maximum: usize) -> ChangeInputError {
        ChangeInputError::Limit {
            name,
            actual,
            maximum,
        }
    }

    fn encoding(&self, reason: &'static str) -> ChangeInputError {
        self.encoding_at(self.pos, reason)
    }

    fn encoding_at(&self, offset: usize, reason: &'static str) -> ChangeInputError {
        ChangeInputError::Encoding { offset, reason }
    }
}

#[derive(Default)]
struct SequenceLengths {
    input: usize,
    output: usize,
}

impl SequenceLengths {
    fn retain(&mut self, len: usize, decoder: &Decoder<'_>) -> Result<(), ChangeInputError> {
        self.input = self.add(self.input, len, decoder)?;
        self.output = self.add(self.output, len, decoder)?;
        Ok(())
    }

    fn insert(&mut self, len: usize, decoder: &Decoder<'_>) -> Result<(), ChangeInputError> {
        self.output = self.add(self.output, len, decoder)?;
        Ok(())
    }

    fn delete(&mut self, len: usize, decoder: &Decoder<'_>) -> Result<(), ChangeInputError> {
        self.input = self.add(self.input, len, decoder)?;
        Ok(())
    }

    fn add(
        &self,
        current: usize,
        amount: usize,
        decoder: &Decoder<'_>,
    ) -> Result<usize, ChangeInputError> {
        let total = current.checked_add(amount).ok_or_else(|| {
            decoder.limit_error(
                "sequence length",
                usize::MAX,
                decoder.limits.max_sequence_len,
            )
        })?;
        decoder.check_limit("sequence length", total, decoder.limits.max_sequence_len)?;
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use colla::ChangeKind;

    fn put_varint(mut value: u64, bytes: &mut Vec<u8>) {
        while value >= 0x80 {
            bytes.push(value as u8 | 0x80);
            value >>= 7;
        }
        bytes.push(value as u8);
    }

    fn put_string(value: &str, bytes: &mut Vec<u8>) {
        put_varint(value.len() as u64, bytes);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn put_value(value: &Value, bytes: &mut Vec<u8>) {
        let encoded = value.encode();
        put_varint(encoded.len() as u64, bytes);
        bytes.extend_from_slice(&encoded);
    }

    #[test]
    fn decodes_noop() {
        let change = decode_change_input(&[0], &InputLimits::default()).unwrap();
        assert!(change.is_noop());
    }

    #[test]
    fn normalizes_raw_text_operations() {
        let mut bytes = vec![4];
        put_varint(5, &mut bytes);
        bytes.push(0);
        put_varint(0, &mut bytes);
        bytes.push(1);
        put_string("a", &mut bytes);
        bytes.push(1);
        put_string("b", &mut bytes);
        bytes.push(2);
        put_varint(0, &mut bytes);
        bytes.push(0);
        put_varint(9, &mut bytes);

        let change = decode_change_input(&bytes, &InputLimits::default()).unwrap();
        let ChangeKind::Text(change) = change.kind() else {
            panic!("expected Text change");
        };
        assert_eq!(change.ops(), &[TextOp::Insert("ab".into())]);
    }

    #[test]
    fn applies_sequence_limits_before_normalization() {
        let mut bytes = vec![4];
        put_varint(2, &mut bytes);
        bytes.push(0);
        put_varint(2, &mut bytes);
        bytes.push(2);
        put_varint(2, &mut bytes);
        let limits = InputLimits {
            max_sequence_len: 3,
            ..InputLimits::default()
        };

        assert!(matches!(
            decode_change_input(&bytes, &limits),
            Err(ChangeInputError::Limit {
                name: "sequence length",
                actual: 4,
                maximum: 3,
            })
        ));
    }

    #[test]
    fn rejects_duplicate_map_keys() {
        let mut bytes = vec![2];
        put_varint(2, &mut bytes);
        put_string("key", &mut bytes);
        bytes.push(1);
        put_string("key", &mut bytes);
        bytes.push(1);

        assert!(matches!(
            decode_change_input(&bytes, &InputLimits::default()),
            Err(ChangeInputError::Value(ValueError::DuplicateKey(key))) if key == "key"
        ));
    }

    #[test]
    fn rejects_malformed_and_trailing_payloads() {
        assert!(matches!(
            decode_change_input(&[], &InputLimits::default()),
            Err(ChangeInputError::Encoding { .. })
        ));
        assert!(matches!(
            decode_change_input(&[0, 0], &InputLimits::default()),
            Err(ChangeInputError::Encoding {
                reason: "trailing ChangeInput bytes",
                ..
            })
        ));
    }

    #[test]
    fn counts_embedded_value_nodes_across_payloads() {
        let mut bytes = vec![3];
        put_varint(1, &mut bytes);
        bytes.push(1);
        put_varint(2, &mut bytes);
        put_value(&Value::null(), &mut bytes);
        put_value(&Value::null(), &mut bytes);
        let limits = InputLimits {
            max_value_nodes: 1,
            ..InputLimits::default()
        };

        assert!(matches!(
            decode_change_input(&bytes, &limits),
            Err(ChangeInputError::Limit {
                name: "value nodes",
                actual: 2,
                maximum: 1,
            })
        ));
    }

    #[test]
    fn counts_embedded_value_depth_inside_change_tree() {
        let mut bytes = vec![1];
        put_value(&Value::list(vec![Value::null()]), &mut bytes);
        let limits = InputLimits {
            max_depth: 2,
            ..InputLimits::default()
        };

        assert!(matches!(
            decode_change_input(&bytes, &limits),
            Err(ChangeInputError::Limit {
                name: "input depth",
                actual: 3,
                maximum: 2,
            })
        ));
    }
}
