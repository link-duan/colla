use colla::CollaError as Error;
use colla::*;
use colla::{TextChange, TextOp};
use js_sys::{Array, BigInt, Object, Reflect, Uint8Array};
use wasm_bindgen::{prelude::*, JsCast};

pub fn error(error: Error) -> JsValue {
    let out = Object::new();
    set(&out, "code", &error.code.as_str().into());
    set(&out, "operation", &error.operation.into());
    let details = Object::new();
    for (key, value) in error.details {
        set(&details, &key, &value.into());
    }
    set(&out, "details", &details);
    out.into()
}
pub fn argument(reason: &str) -> Error {
    Error::new(ErrorCode::InvalidArgument, reason)
}
pub fn set(object: &Object, key: &str, value: &JsValue) {
    Reflect::set(object, &key.into(), value).expect("fresh data object is extensible");
}
pub fn array(value: &JsValue) -> Result<Array> {
    if !Array::is_array(value) {
        return Err(argument("expected array"));
    }
    let array: Array = value.clone().unchecked_into();
    if array.length() > 1_000_000 {
        return Err(Error::new(ErrorCode::LimitExceeded, "array limit exceeded"));
    }
    Ok(array)
}
pub fn string(value: &JsValue) -> Result<String> {
    let value = value
        .as_string()
        .ok_or_else(|| argument("expected string"))?;
    if value.len() > 16 * 1024 * 1024 {
        return Err(Error::new(
            ErrorCode::LimitExceeded,
            "string limit exceeded",
        ));
    }
    Ok(value)
}
pub fn index(value: &JsValue) -> Result<usize> {
    let n = value
        .as_f64()
        .ok_or_else(|| argument("expected nonnegative integer position"))?;
    if !n.is_finite()
        || n.fract() != 0.0
        || n < 0.0
        || n > usize::MAX as f64
        || n > 9_007_199_254_740_991.0
    {
        return Err(argument("invalid position"));
    }
    Ok(n as usize)
}
pub fn signed(value: &JsValue) -> Result<i64> {
    if !value.is_bigint() {
        return Err(argument("expected bigint"));
    }
    let value: BigInt = value.clone().unchecked_into();
    value
        .to_string(10)
        .map_err(|_| argument("invalid bigint"))?
        .as_string()
        .unwrap()
        .parse()
        .map_err(|_| Error::new(ErrorCode::IntegerOverflow, "bigint is outside i64"))
}
pub fn unsigned(value: &JsValue) -> Result<u64> {
    if !value.is_bigint() {
        return Err(argument("expected u64 bigint"));
    }
    let value: BigInt = value.clone().unchecked_into();
    value
        .to_string(10)
        .map_err(|_| argument("invalid bigint"))?
        .as_string()
        .unwrap()
        .parse()
        .map_err(|_| argument("bigint is outside u64"))
}
pub fn segment(value: &JsValue) -> Result<Segment> {
    Ok(if value.is_string() {
        Segment::Key(string(value)?)
    } else {
        Segment::Index(index(value)?)
    })
}
pub fn path(value: &JsValue) -> Result<Path> {
    array(value)?.iter().map(|v| segment(&v)).collect()
}
fn path_js(path: &[Segment]) -> JsValue {
    path.iter()
        .map(|part| match part {
            Segment::Key(key) => JsValue::from(key),
            Segment::Index(n) => JsValue::from(*n as f64),
        })
        .collect::<Array>()
        .into()
}
fn attr(value: &JsValue) -> Result<Attr> {
    if let Some(v) = value.as_bool() {
        return Ok(Attr::Bool(v));
    }
    if value.is_bigint() {
        return Ok(Attr::Int(signed(value)?));
    }
    if let Some(v) = value.as_f64() {
        if !v.is_finite() {
            return Err(argument("attribute float must be finite"));
        }
        return Ok(Attr::Float(if v == 0.0 { 0.0 } else { v }));
    }
    Ok(Attr::String(string(value)?))
}
fn attr_js(value: &Attr) -> JsValue {
    match value {
        Attr::Bool(v) => (*v).into(),
        Attr::Int(v) => BigInt::from(*v).into(),
        Attr::Float(v) => (*v).into(),
        Attr::String(v) => v.into(),
    }
}
pub fn attrs(value: &JsValue) -> Result<Attrs> {
    let mut out = Attrs::new();
    for entry in array(value)?.iter() {
        let entry = array(&entry)?;
        let key = string(&entry.get(0))?;
        if out.insert(key, attr(&entry.get(1))?).is_some() {
            return Err(argument("duplicate attribute"));
        }
    }
    Ok(out)
}
pub fn patch(value: &JsValue) -> Result<AttrPatch> {
    let mut out = AttrPatch::new();
    for entry in array(value)?.iter() {
        let entry = array(&entry)?;
        let key = string(&entry.get(0))?;
        let value = entry.get(1);
        let value = if value.is_null() {
            None
        } else {
            Some(attr(&value)?)
        };
        if out.insert(key, value).is_some() {
            return Err(argument("duplicate attribute patch"));
        }
    }
    Ok(out)
}
fn attrs_js(value: &Attrs) -> JsValue {
    value
        .iter()
        .map(|(key, value)| Array::of2(&key.into(), &attr_js(value)))
        .collect::<Array>()
        .into()
}
fn patch_js(value: &AttrPatch) -> JsValue {
    value
        .iter()
        .map(|(key, value)| {
            Array::of2(
                &key.into(),
                &value.as_ref().map(attr_js).unwrap_or(JsValue::NULL),
            )
        })
        .collect::<Array>()
        .into()
}
pub fn value(input: &JsValue) -> Result<Value> {
    let mut nodes = 0;
    value_inner(input, 1, &mut nodes)
}
fn value_inner(input: &JsValue, depth: usize, nodes: &mut usize) -> Result<Value> {
    *nodes += 1;
    if depth > 100 || *nodes > 1_000_000 {
        return Err(Error::new(
            ErrorCode::LimitExceeded,
            "value input limit exceeded",
        ));
    }
    let input = array(input)?;
    let data = input.get(1);
    let value = match index(&input.get(0))? {
        0 => Value::null(),
        1 => Value::bool(data.as_bool().ok_or_else(|| argument("expected bool"))?),
        2 => Value::int(signed(&data)?),
        3 => Value::float(data.as_f64().ok_or_else(|| argument("expected float"))?)?,
        4 => Value::string(string(&data)?)?,
        5 => Value::text(string(&data)?)?,
        7 => Value::list(
            array(&data)?
                .iter()
                .map(|v| value_inner(&v, depth + 1, nodes))
                .collect::<Result<_>>()?,
        )?,
        8 => Value::map(
            array(&data)?
                .iter()
                .map(|entry| {
                    let entry = array(&entry)?;
                    Ok((
                        string(&entry.get(0))?,
                        value_inner(&entry.get(1), depth + 1, nodes)?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?,
        )?,
        6 => Value::rich_text(
            array(&data)?
                .iter()
                .map(|v| span_inner(&v, depth + 1, nodes))
                .collect::<Result<_>>()?,
        )?,
        9 => {
            if !data.is_instance_of::<Uint8Array>() {
                return Err(argument("expected encoded Value bytes"));
            }
            Value::decode(&Uint8Array::new(&data).to_vec())?
        }
        _ => return Err(argument("invalid input Value kind")),
    };
    Ok(value)
}
fn span_inner(input: &JsValue, depth: usize, nodes: &mut usize) -> Result<RichSpan> {
    let input = array(input)?;
    Ok(match index(&input.get(0))? {
        0 => RichSpan::Text {
            text: string(&input.get(1))?,
            attrs: attrs(&input.get(2))?,
        },
        1 => RichSpan::Embed {
            value: value_inner(&input.get(1), depth, nodes)?,
            attrs: attrs(&input.get(2))?,
        },
        _ => return Err(argument("invalid rich span kind")),
    })
}
pub fn span(input: &JsValue) -> Result<RichSpan> {
    let mut nodes = 0;
    span_inner(input, 1, &mut nodes)
}
pub fn projection(value: &Value) -> JsValue {
    let (kind, data): (u32, JsValue) = match value.body() {
        Body::Null => (0, JsValue::NULL),
        Body::Bool(v) => (1, (*v).into()),
        Body::Int(v) => (2, BigInt::from(*v).into()),
        Body::Float(v) => (3, (*v).into()),
        Body::String(v) => (4, v.into()),
        Body::Text(v) => (5, v.into()),
        Body::List(v) => (7, v.iter().map(projection).collect::<Array>().into()),
        Body::Map(v) => (
            8,
            v.iter()
                .map(|(key, value)| Array::of2(&key.into(), &projection(value)))
                .collect::<Array>()
                .into(),
        ),
        Body::RichText(v) => (6, v.iter().map(span_js).collect::<Array>().into()),
    };
    Array::of2(&kind.into(), &data).into()
}
fn span_js(span: &RichSpan) -> JsValue {
    match span {
        RichSpan::Text { text, attrs } => {
            Array::of3(&0.into(), &text.into(), &attrs_js(attrs)).into()
        }
        RichSpan::Embed { value, attrs } => {
            Array::of3(&1.into(), &projection(value), &attrs_js(attrs)).into()
        }
    }
}
fn operation_span_js(span: &RichSpan) -> JsValue {
    match span {
        RichSpan::Text { .. } => span_js(span),
        RichSpan::Embed { value, attrs } => Array::of3(
            &1.into(),
            &super::binding::CoreValue::from_value(value.clone()).into(),
            &attrs_js(attrs),
        )
        .into(),
    }
}
pub fn rich_ops(input: &JsValue) -> Result<Vec<RichOp>> {
    array(input)?
        .iter()
        .map(|op| {
            let op = array(&op)?;
            Ok(match index(&op.get(0))? {
                0 => RichOp::Retain {
                    len: index(&op.get(1))?,
                    attrs: patch(&op.get(2))?,
                },
                1 => RichOp::Insert(span(&op.get(1))?),
                2 => RichOp::Delete(index(&op.get(1))?),
                _ => return Err(argument("invalid RichText operation")),
            })
        })
        .collect()
}
pub fn change(input: &JsValue) -> Result<Change> {
    Change::new(
        array(input)?
            .iter()
            .map(|op| {
                let op = array(&op)?;
                let kind = index(&op.get(0))?;
                Ok(match kind {
                    0 => Operation::Insert {
                        path: path(&op.get(1))?,
                        value: value(&op.get(2))?,
                    },
                    1 => Operation::Delete {
                        path: path(&op.get(1))?,
                    },
                    2 => Operation::Set {
                        path: path(&op.get(1))?,
                        value: value(&op.get(2))?,
                    },
                    3 => Operation::ListMove {
                        path: path(&op.get(1))?,
                        from: index(&op.get(2))?,
                        to: index(&op.get(3))?,
                    },
                    4 => Operation::Text {
                        path: path(&op.get(1))?,
                        change: TextChange::from_ops(
                            array(&op.get(2))?
                                .iter()
                                .map(|op| {
                                    let op = array(&op)?;
                                    Ok(match index(&op.get(0))? {
                                        0 => TextOp::Retain(index(&op.get(1))?),
                                        1 => TextOp::Insert(string(&op.get(1))?),
                                        2 => TextOp::Delete(index(&op.get(1))?),
                                        _ => return Err(argument("invalid Text operation")),
                                    })
                                })
                                .collect::<Result<Vec<_>>>()?,
                        )?,
                    },
                    5 => Operation::Add {
                        path: path(&op.get(1))?,
                        delta: signed(&op.get(2))?,
                    },
                    6 => Operation::RichText {
                        path: path(&op.get(1))?,
                        operations: rich_ops(&op.get(2))?,
                    },
                    _ => return Err(argument("invalid operation kind")),
                })
            })
            .collect::<Result<Vec<_>>>()?,
    )
}
pub fn operations(change: &Change) -> JsValue {
    use super::binding::CoreValue;
    change
        .operations()
        .iter()
        .map(|op| match op {
            Operation::Insert { path, value } => Array::of3(
                &0.into(),
                &path_js(path),
                &CoreValue::from_value(value.clone()).into(),
            ),
            Operation::Delete { path } => Array::of2(&1.into(), &path_js(path)),
            Operation::Set { path, value } => Array::of3(
                &2.into(),
                &path_js(path),
                &CoreValue::from_value(value.clone()).into(),
            ),
            Operation::ListMove { path, from, to } => Array::of4(
                &3.into(),
                &path_js(path),
                &(*from as f64).into(),
                &(*to as f64).into(),
            ),
            Operation::Text { path, change } => Array::of3(
                &4.into(),
                &path_js(path),
                &change
                    .ops()
                    .iter()
                    .map(|op| match op {
                        TextOp::Retain(n) => Array::of2(&0.into(), &(*n as f64).into()),
                        TextOp::Insert(text) => Array::of2(&1.into(), &text.into()),
                        TextOp::Delete(n) => Array::of2(&2.into(), &(*n as f64).into()),
                    })
                    .collect::<Array>()
                    .into(),
            ),
            Operation::Add { path, delta } => {
                Array::of3(&5.into(), &path_js(path), &BigInt::from(*delta).into())
            }
            Operation::RichText { path, operations } => Array::of3(
                &6.into(),
                &path_js(path),
                &operations
                    .iter()
                    .map(|op| match op {
                        RichOp::Retain { len, attrs } => {
                            Array::of3(&0.into(), &(*len as f64).into(), &patch_js(attrs))
                        }
                        RichOp::Insert(span) => Array::of2(&1.into(), &operation_span_js(span)),
                        RichOp::Delete(n) => Array::of2(&2.into(), &(*n as f64).into()),
                    })
                    .collect::<Array>()
                    .into(),
            ),
        })
        .collect::<Array>()
        .into()
}
