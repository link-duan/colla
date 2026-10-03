use super::{codec, Body, Error, ErrorCode, Path, Result, Segment, Value};
use cocodec::{Decode, Encode};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// One ordered path-addressed content operation. Each path is interpreted
/// against the content produced by the preceding operations.
pub enum Operation {
    #[cocodec(tag = 0)]
    /// Inserts a subtree at a vacant Map key or a List position.
    Insert {
        /// Parent container path followed by the vacant key or insertion index.
        path: Path,
        /// Subtree to insert.
        value: Value,
    },
    #[cocodec(tag = 1)]
    /// Deletes an existing Map member or List element and its subtree.
    Delete {
        /// Path of the existing Map member or List element.
        path: Path,
    },
    #[cocodec(tag = 2)]
    /// Replaces an existing element, including the root.
    Set {
        /// Path of the element to replace.
        path: Path,
        /// Replacement subtree.
        value: Value,
    },
    #[cocodec(tag = 3)]
    /// Moves one element within the same List.
    ListMove {
        /// Path of the List.
        path: Path,
        /// Current index of the moved element.
        from: usize,
        /// Destination index after the element is removed.
        to: usize,
    },
    #[cocodec(tag = 4)]
    /// Edits a collaborative Text value using Unicode scalar positions.
    Text {
        /// Path of the existing Text element.
        path: Path,
        /// Ordered scalar retains, insertions and deletions.
        change: crate::sequence::change::TextChange,
    },
    #[cocodec(tag = 5)]
    /// Adds a checked signed integer delta.
    Add {
        /// Path of the existing Int element.
        path: Path,
        /// Signed delta applied with checked integer arithmetic.
        delta: i64,
    },
    #[cocodec(tag = 6)]
    /// Edits RichText content and formatting; each embed counts as one scalar unit.
    RichText {
        /// Path of the existing RichText element.
        path: Path,
        /// Scalar sequence operations in execution order.
        operations: Vec<super::RichOp>,
    },
}
impl Operation {
    /// Returns the path this operation addresses.
    pub fn path(&self) -> &Path {
        match self {
            Self::Insert { path, .. }
            | Self::Delete { path }
            | Self::Set { path, .. }
            | Self::ListMove { path, .. }
            | Self::Text { path, .. }
            | Self::Add { path, .. }
            | Self::RichText { path, .. } => path,
        }
    }
    /// Whether this operation is a Noop on every base.
    fn is_noop(&self) -> bool {
        match self {
            Self::ListMove { from, to, .. } => from == to,
            Self::Add { delta, .. } => *delta == 0,
            Self::Text { change, .. } => change.is_empty(),
            Self::RichText { operations, .. } => operations.is_empty(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode)]
#[cocodec(transparent)]
/// Immutable canonical operation sequence; clones share storage.
pub struct Change(Arc<Vec<Operation>>);
impl Change {
    /// Constructs and validates a canonical object.
    pub fn new(operations: impl IntoIterator<Item = Operation>) -> Result<Self> {
        let mut result = Vec::new();
        for mut operation in operations {
            // TextChange is canonical by construction; decoders reject any
            // encoding that this RichText normalization would change.
            match &mut operation {
                Operation::Insert { path, value } => {
                    if path.is_empty() {
                        return Err(root_error());
                    }
                    value.validate()?;
                }
                Operation::Delete { path } if path.is_empty() => return Err(root_error()),
                Operation::Set { value, .. } => value.validate()?,
                Operation::RichText { operations, .. } => {
                    *operations = super::rich::normalized(operations)?;
                }
                _ => {}
            }
            result.push(operation);
        }
        Self::from_canonical(result)
    }
    /// Wraps operations that are already validated and canonical, such as
    /// those taken from existing Changes, dropping trivial Noops.
    pub(crate) fn from_canonical(mut operations: Vec<Operation>) -> Result<Self> {
        operations.retain(|operation| !operation.is_noop());
        if operations.len() > 1_000_000 {
            return Err(Error::new(
                ErrorCode::LimitExceeded,
                "operation limit exceeded",
            ));
        }
        Ok(Self(Arc::new(operations)))
    }
    /// Creates the empty Change with no operations.
    pub fn noop() -> Self {
        Self::default()
    }
    /// Borrows the canonical operations in execution order.
    pub fn operations(&self) -> &[Operation] {
        &self.0
    }
    /// Returns whether the canonical operation sequence is empty.
    pub fn is_noop(&self) -> bool {
        self.0.is_empty()
    }
    /// Returns independent canonical bytes in a typed binary envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(codec::Kind::Change, self)
    }
    /// Strictly decodes and validates a typed binary envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(codec::Kind::Change, bytes)?;
        let canonical = Self::new(value.operations().iter().cloned())?;
        if canonical != value {
            return Err(Error::new(
                ErrorCode::InvalidEncoding,
                "noncanonical change",
            ));
        }
        Ok(value)
    }
    pub(crate) fn normalized(&self, base: &Value) -> Result<(Self, Value)> {
        let mut value = base.clone();
        let mut operations = Vec::new();
        for operation in self.operations() {
            let next = apply_operation(&value, operation)?;
            if next != value {
                operations.push(operation.clone());
            }
            value = next;
        }
        if &value == base {
            operations.clear();
        }
        Ok((Self(Arc::new(operations)), value))
    }
}

/// Applies a path-addressed Change atomically.
pub fn apply(base: &Value, change: &Change) -> Result<Value> {
    let mut value = base.clone();
    for operation in change.operations() {
        value = apply_operation(&value, operation)?;
    }
    Ok(value)
}

/// Composes sequential Changes against their original base, validating intermediate steps.
pub fn compose(base: &Value, first: &Change, second: &Change) -> Result<Change> {
    let sequence = Change::from_canonical(
        first
            .operations()
            .iter()
            .chain(second.operations())
            .cloned()
            .collect(),
    )?;
    // Validate the original execution order before compacting. In particular,
    // cancelling additions must not conceal an intermediate integer overflow.
    let (sequence, _) = sequence.normalized(base)?;
    let mut operations: Vec<Operation> = Vec::new();
    for operation in sequence.operations() {
        let merged = match (operations.last(), operation) {
            (Some(Operation::Add { path: a, delta: x }), Operation::Add { path: b, delta: y })
                if a == b =>
            {
                x.checked_add(*y).map(|delta| Operation::Add {
                    path: a.clone(),
                    delta,
                })
            }
            (
                Some(Operation::Text { path: a, change: x }),
                Operation::Text { path: b, change: y },
            ) if a == b => {
                let change = crate::sequence::op::compose(&x.clone().into(), &y.clone().into())
                    .map_err(algebra_error)?;
                Some(Operation::Text {
                    path: a.clone(),
                    change: change.as_text().cloned().unwrap_or_default(),
                })
            }
            (
                Some(Operation::RichText {
                    path: a,
                    operations: x,
                }),
                Operation::RichText {
                    path: b,
                    operations: y,
                },
            ) if a == b => {
                let change = crate::sequence::op::compose(
                    &super::rich::to_sequence_change(x)?,
                    &super::rich::to_sequence_change(y)?,
                )
                .map_err(algebra_error)?;
                Some(Operation::RichText {
                    path: a.clone(),
                    operations: super::rich::from_sequence_change(&change)?,
                })
            }
            (Some(Operation::Set { path: a, .. }), Operation::Set { path: b, .. }) if a == b => {
                Some(operation.clone())
            }
            (
                Some(Operation::ListMove {
                    path: a,
                    from,
                    to: middle,
                }),
                Operation::ListMove {
                    path: b,
                    from: next,
                    to,
                },
            ) if a == b && middle == next => Some(Operation::ListMove {
                path: a.clone(),
                from: *from,
                to: *to,
            }),
            _ => None,
        };
        if let Some(merged) = merged {
            // A cancelled merge exposes the previous operation to the next one.
            operations.pop();
            if !merged.is_noop() {
                operations.push(merged);
            }
        } else {
            operations.push(operation.clone());
        }
    }
    Ok(Change::from_canonical(operations)?.normalized(base)?.0)
}

/// Constructs a reverse Change that restores the base content.
pub fn invert(base: &Value, change: &Change) -> Result<Change> {
    let mut before = base.clone();
    let mut inverses = Vec::new();
    for operation in change.operations() {
        let inverse = match operation {
            Operation::Insert { path, .. } => vec![Operation::Delete { path: path.clone() }],
            Operation::Delete { path } => vec![Operation::Insert {
                path: path.clone(),
                value: before.get(path)?.clone(),
            }],
            Operation::Set { path, .. } => vec![Operation::Set {
                path: path.clone(),
                value: before.get(path)?.clone(),
            }],
            Operation::ListMove { path, from, to } => vec![Operation::ListMove {
                path: path.clone(),
                from: *to,
                to: *from,
            }],
            Operation::Text { path, change } => {
                let Body::Text(text) = before.get(path)?.body() else {
                    return Err(type_error("expected Text"));
                };
                let inverse = crate::sequence::op::invert(
                    &change.clone().into(),
                    &crate::sequence::value::Value::text(text.clone()),
                )
                .map_err(algebra_error)?;
                vec![Operation::Text {
                    path: path.clone(),
                    change: inverse.as_text().cloned().unwrap_or_default(),
                }]
            }
            Operation::Add { path, delta } => match delta.checked_neg() {
                Some(delta) => vec![Operation::Add {
                    path: path.clone(),
                    delta,
                }],
                None => vec![
                    Operation::Add {
                        path: path.clone(),
                        delta: i64::MAX,
                    },
                    Operation::Add {
                        path: path.clone(),
                        delta: 1,
                    },
                ],
            },
            Operation::RichText { path, operations } => {
                let Body::RichText(spans) = before.get(path)?.body() else {
                    return Err(type_error("expected RichText"));
                };
                let inverse = crate::sequence::op::invert(
                    &super::rich::to_sequence_change(operations)?,
                    &super::rich::to_sequence_value(spans)?,
                )
                .map_err(algebra_error)?;
                vec![Operation::RichText {
                    path: path.clone(),
                    operations: super::rich::from_sequence_change(&inverse)?,
                }]
            }
        };
        before = apply_operation(&before, operation)?;
        inverses.push(inverse);
    }
    Change::from_canonical(inverses.into_iter().rev().flatten().collect())
}

pub(crate) fn apply_operation(base: &Value, operation: &Operation) -> Result<Value> {
    let result = match operation {
        Operation::Insert { path, value } => {
            let (slot, parent) = path.split_last().ok_or_else(root_error)?;
            base.update(parent, |parent| {
                Ok(Value::trusted(match (parent.body(), slot) {
                    (Body::Map(map), Segment::Key(key)) => {
                        if map.contains_key(key) {
                            return Err(Error::new(
                                ErrorCode::InvalidArgument,
                                "Map insertion key is occupied",
                            )
                            .detail("key", key));
                        }
                        let mut map = map.clone();
                        map.insert(key.clone(), value.clone());
                        Body::Map(map)
                    }
                    (Body::List(list), Segment::Index(index)) => {
                        if *index > list.len() {
                            return Err(Error::new(
                                ErrorCode::OutOfBounds,
                                "List insertion index is out of bounds",
                            ));
                        }
                        let mut list = list.clone();
                        list.insert(*index, value.clone());
                        Body::List(list)
                    }
                    _ => return Err(type_error("insertion slot does not match parent kind")),
                }))
            })?
        }
        Operation::Delete { path } => {
            let (slot, parent) = path.split_last().ok_or_else(root_error)?;
            base.get(path)?;
            base.update(parent, |parent| {
                Ok(Value::trusted(match (parent.body(), slot) {
                    (Body::Map(map), Segment::Key(key)) => {
                        let mut map = map.clone();
                        map.remove(key);
                        Body::Map(map)
                    }
                    (Body::List(list), Segment::Index(index)) => {
                        let mut list = list.clone();
                        list.remove(*index);
                        Body::List(list)
                    }
                    _ => unreachable!("get() resolved the deleted element"),
                }))
            })?
        }
        Operation::Set { path, value } => base.update(path, |_| Ok(value.clone()))?,
        Operation::ListMove { path, from, to } => base.update(path, |list| {
            let Body::List(list) = list.body() else {
                return Err(type_error("expected List"));
            };
            if *from >= list.len() || *to >= list.len() {
                return Err(Error::new(
                    ErrorCode::OutOfBounds,
                    "List move index is out of bounds",
                ));
            }
            let mut list = list.clone();
            let moved = list.remove(*from);
            list.insert(*to, moved);
            Ok(Value::trusted(Body::List(list)))
        })?,
        Operation::Text { path, change } => base.update(path, |value| {
            let Body::Text(text) = value.body() else {
                return Err(type_error("expected Text"));
            };
            let result = crate::sequence::op::apply(
                &crate::sequence::value::Value::text(text.clone()),
                &change.clone().into(),
            )?;
            Value::text(result.as_text().unwrap().as_str())
        })?,
        Operation::Add { path, delta } => base.update(path, |value| {
            let Body::Int(number) = value.body() else {
                return Err(type_error("expected Int"));
            };
            let number = number.checked_add(*delta).ok_or_else(|| {
                Error::new(ErrorCode::IntegerOverflow, "integer addition overflow")
            })?;
            Ok(Value::int(number))
        })?,
        Operation::RichText { path, operations } => base.update(path, |value| {
            let Body::RichText(spans) = value.body() else {
                return Err(type_error("expected RichText"));
            };
            let result = crate::sequence::op::apply(
                &super::rich::to_sequence_value(spans)?,
                &super::rich::to_sequence_change(operations)?,
            )?;
            Value::rich_text(super::rich::from_sequence_value(&result)?)
        })?,
    };
    result.check_limits()?;
    Ok(result)
}

fn root_error() -> Error {
    Error::new(
        ErrorCode::InvalidArgument,
        "the root cannot be inserted or deleted",
    )
}
pub(crate) fn type_error(reason: &str) -> Error {
    Error::new(ErrorCode::TypeMismatch, reason)
}
pub(crate) fn algebra_error(error: impl std::fmt::Display) -> Error {
    Error::new(ErrorCode::IncompatibleChange, error.to_string())
}
