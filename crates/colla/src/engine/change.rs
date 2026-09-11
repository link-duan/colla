use super::{
    codec, value::missing, Body, ElementId, Error, ErrorCode, Path, Result, Segment, Value,
};
use cocodec::{Decode, Encode};
use std::sync::Arc;

/// The destination index is interpreted after detaching a moved source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    /// Stable ID of the destination owning container.
    pub parent: ElementId,
    /// Vacant Map key or post-removal List position.
    pub slot: Segment,
}
codec::record_codec!(Destination, parent, slot);

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// One ordered identity-addressed content operation.
pub enum Operation {
    #[cocodec(tag = 0)]
    /// Inserts owning content at a sequence or container position.
    Insert {
        /// Owning location for an insertion or native move.
        destination: Destination,
        /// Immutable owning content carried by this operation or embed.
        value: Value,
    },
    #[cocodec(tag = 1)]
    /// Deletes existing owning content or scalar sequence units.
    Delete {
        /// Stable target identity; Ref targets may be absent.
        target: ElementId,
    },
    #[cocodec(tag = 2)]
    /// Replaces content while retaining the target root identity.
    Set {
        /// Stable target identity; Ref targets may be absent.
        target: ElementId,
        /// Immutable owning content carried by this operation or embed.
        value: Value,
    },
    #[cocodec(tag = 3)]
    /// Moves the same owning subtree without changing any of its IDs.
    Move {
        /// Stable target identity; Ref targets may be absent.
        target: ElementId,
        /// Owning location for an insertion or native move.
        destination: Destination,
    },
    #[cocodec(tag = 4)]
    /// Collaborative Unicode scalar text or a scalar text operation.
    Text {
        /// Stable target identity; Ref targets may be absent.
        target: ElementId,
        /// Immutable identity-addressed or scalar change content.
        change: crate::sequence::change::TextChange,
    },
    #[cocodec(tag = 5)]
    /// Adds a checked signed integer delta.
    Add {
        /// Stable target identity; Ref targets may be absent.
        target: ElementId,
        /// Signed delta applied with checked integer arithmetic.
        delta: i64,
    },
    #[cocodec(tag = 6)]
    /// Formatted scalar text with atomic embeds.
    RichText {
        /// Stable target identity; Ref targets may be absent.
        target: ElementId,
        /// Scalar sequence operations in execution order.
        operations: Vec<super::RichOp>,
    },
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
            if let Operation::RichText { operations, .. } = &mut operation {
                *operations = super::rich::normalized(operations)?;
                if operations.is_empty() {
                    continue;
                }
            }
            match &operation {
                Operation::Set { target, value } => {
                    if *target != value.id() {
                        return Err(Error::new(
                            ErrorCode::InvalidValue,
                            "set must preserve target identity",
                        ));
                    }
                    value.validate()?;
                }
                Operation::Insert { value, .. } => value.validate()?,
                Operation::Text { change, .. } => {
                    if &crate::sequence::change::TextChange::from_ops(change.ops().iter().cloned())?
                        != change
                    {
                        return Err(Error::new(
                            ErrorCode::InvalidValue,
                            "noncanonical text operations",
                        ));
                    }
                    if change.is_empty() {
                        continue;
                    }
                }
                Operation::Add { delta: 0, .. } => continue,
                _ => {}
            }
            if result.len() >= 1_000_000 {
                return Err(Error::new(
                    ErrorCode::LimitExceeded,
                    "operation limit exceeded",
                ));
            }
            result.push(operation);
        }
        Ok(Self(Arc::new(result)))
    }
    /// Creates the identity Change with no operations.
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
    /// Returns independent canonical bytes in a typed version-2 envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(2, self)
    }
    /// Strictly decodes and validates a typed version-2 envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(2, bytes)?;
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

/// Applies an identity-addressed Change atomically against the current content.
pub fn apply(base: &Value, change: &Change) -> Result<Value> {
    let mut value = base.clone();
    for operation in change.operations() {
        value = apply_operation(&value, operation)?;
    }
    Ok(value)
}

/// Composes sequential Changes against their original base, validating intermediate steps.
pub fn compose(base: &Value, first: &Change, second: &Change) -> Result<Change> {
    let sequence = Change::new(
        first
            .operations()
            .iter()
            .chain(second.operations())
            .cloned(),
    )?;
    // Validate the original execution order before compacting. In particular,
    // cancelling additions must not conceal an intermediate integer overflow.
    let (sequence, _) = sequence.normalized(base)?;
    let mut operations = Vec::new();
    for operation in sequence.operations() {
        let merged = match (operations.last(), operation) {
            (
                Some(Operation::Add {
                    target: a,
                    delta: x,
                }),
                Operation::Add {
                    target: b,
                    delta: y,
                },
            ) if a == b => x
                .checked_add(*y)
                .map(|delta| Operation::Add { target: *a, delta }),
            (
                Some(Operation::Text {
                    target: a,
                    change: x,
                }),
                Operation::Text {
                    target: b,
                    change: y,
                },
            ) if a == b => {
                let change = crate::sequence::op::compose(&x.clone().into(), &y.clone().into())
                    .map_err(|e| Error::new(ErrorCode::IncompatibleChange, e.to_string()))?;
                Some(Operation::Text {
                    target: *a,
                    change: change.as_text().cloned().unwrap_or_default(),
                })
            }
            (
                Some(Operation::RichText {
                    target: a,
                    operations: x,
                }),
                Operation::RichText {
                    target: b,
                    operations: y,
                },
            ) if a == b => {
                let change = crate::sequence::op::compose(
                    &super::rich::to_old(x)?,
                    &super::rich::to_old(y)?,
                )
                .map_err(|e| Error::new(ErrorCode::IncompatibleChange, e.to_string()))?;
                Some(Operation::RichText {
                    target: *a,
                    operations: super::rich::from_old(&change)?,
                })
            }
            (Some(Operation::Set { target: a, .. }), Operation::Set { target: b, .. })
                if a == b =>
            {
                Some(operation.clone())
            }
            (Some(Operation::Move { target: a, .. }), Operation::Move { target: b, .. })
                if a == b =>
            {
                Some(operation.clone())
            }
            _ => None,
        };
        if let Some(merged) = merged {
            operations.pop();
            operations.push(merged);
        } else {
            operations.push(operation.clone());
        }
    }
    Ok(Change::new(operations)?.normalized(base)?.0)
}

/// Constructs a reverse Change that restores the base content and identities.
pub fn invert(base: &Value, change: &Change) -> Result<Change> {
    let mut before = base.clone();
    let mut inverses = Vec::new();
    for operation in change.operations() {
        let inverse = match operation {
            Operation::Insert { value, .. } => vec![Operation::Delete { target: value.id() }],
            Operation::Delete { target } => vec![Operation::Insert {
                destination: destination_of(&before, *target)?,
                value: before.get(*target)?,
            }],
            Operation::Set { target, .. } => vec![Operation::Set {
                target: *target,
                value: before.get(*target)?,
            }],
            Operation::Move { target, .. } => vec![Operation::Move {
                target: *target,
                destination: destination_of(&before, *target)?,
            }],
            Operation::Text { target, change } => {
                let value = before.get(*target)?;
                let Body::Text(text) = value.body() else {
                    return Err(type_error("expected Text"));
                };
                let inverse = crate::sequence::op::invert(
                    &change.clone().into(),
                    &crate::sequence::value::Value::text(text.clone()),
                )
                .map_err(|e| Error::new(ErrorCode::IncompatibleChange, e.to_string()))?;
                vec![Operation::Text {
                    target: *target,
                    change: inverse.as_text().cloned().unwrap_or_default(),
                }]
            }
            Operation::Add { target, delta } => match delta.checked_neg() {
                Some(delta) => vec![Operation::Add {
                    target: *target,
                    delta,
                }],
                None => vec![
                    Operation::Add {
                        target: *target,
                        delta: i64::MAX,
                    },
                    Operation::Add {
                        target: *target,
                        delta: 1,
                    },
                ],
            },
            Operation::RichText { target, operations } => {
                let value = before.get(*target)?;
                let Body::RichText(spans) = value.body() else {
                    return Err(type_error("expected RichText"));
                };
                let inverse = crate::sequence::op::invert(
                    &super::rich::to_old(operations)?,
                    &super::rich::value_to_old(spans)?,
                )
                .map_err(|e| Error::new(ErrorCode::IncompatibleChange, e.to_string()))?;
                vec![Operation::RichText {
                    target: *target,
                    operations: super::rich::from_old(&inverse)?,
                }]
            }
        };
        before = apply_operation(&before, operation)?;
        inverses.push(inverse);
    }
    Change::new(inverses.into_iter().rev().flatten())
}

pub(crate) fn apply_operation(base: &Value, operation: &Operation) -> Result<Value> {
    let result = match operation {
        Operation::Insert { destination, value } => insert(base, destination, value)?,
        Operation::Delete { target } => remove(base, *target)?.0,
        Operation::Set { target, value } => {
            if value.id() != *target {
                return Err(Error::new(ErrorCode::InvalidValue, "set identity mismatch"));
            }
            base.replace_at(*target, value)?
        }
        Operation::Move {
            target,
            destination,
        } => {
            if *target == base.id() {
                return Err(Error::new(ErrorCode::InvalidArgument, "root cannot move"));
            }
            let source = base.get(*target)?;
            // Resolve and check the parent before detaching the source.
            base.get(destination.parent)?;
            if source.find(destination.parent).is_some() {
                return Err(structural(
                    "cannot move an element into itself or its descendant",
                ));
            }
            if destination_of(base, *target)? == *destination {
                return Ok(base.clone());
            }
            let (detached, source) = remove(base, *target)?;
            insert(&detached, destination, &source)?
        }
        Operation::Text { target, change } => {
            let value = base.get(*target)?;
            let Body::Text(text) = value.body() else {
                return Err(type_error("expected Text"));
            };
            let result = crate::sequence::op::apply(
                &crate::sequence::value::Value::text(text.clone()),
                &change.clone().into(),
            )?;
            let value = Value::trusted(
                *target,
                Body::Text(result.as_text().unwrap().as_str().into()),
            );
            base.replace_at(*target, &value)?
        }
        Operation::Add { target, delta } => {
            let value = base.get(*target)?;
            let Body::Int(number) = value.body() else {
                return Err(type_error("expected Int"));
            };
            let number = number.checked_add(*delta).ok_or_else(|| {
                Error::new(ErrorCode::IntegerOverflow, "integer addition overflow")
            })?;
            base.replace_at(*target, &Value::trusted(*target, Body::Int(number)))?
        }
        Operation::RichText { target, operations } => {
            let value = base.get(*target)?;
            let Body::RichText(spans) = value.body() else {
                return Err(type_error("expected RichText"));
            };
            let result = crate::sequence::op::apply(
                &super::rich::value_to_old(spans)?,
                &super::rich::to_old(operations)?,
            )?;
            base.replace_at(
                *target,
                &Value::trusted(
                    *target,
                    Body::RichText(super::rich::value_from_old(&result)?),
                ),
            )?
        }
    };
    result.validate()?;
    Ok(result)
}

pub(crate) fn destination_of(base: &Value, id: ElementId) -> Result<Destination> {
    let mut path: Path = base.path_of(id).ok_or_else(|| missing(id))?;
    let slot = path
        .pop()
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "root has no owning parent"))?;
    Ok(Destination {
        parent: base.id_at(path)?,
        slot,
    })
}

fn insert(base: &Value, destination: &Destination, value: &Value) -> Result<Value> {
    let parent = base.get(destination.parent)?;
    let body = match (parent.body(), &destination.slot) {
        (Body::Map(map), Segment::Key(key)) => {
            if map.contains_key(key) {
                return Err(structural("Map destination key is occupied").detail("key", key));
            }
            let mut next = map.clone();
            next.insert(key.clone(), value.clone());
            Body::Map(next)
        }
        (Body::List(list), Segment::Index(index)) => {
            if *index > list.len() {
                return Err(Error::new(
                    ErrorCode::OutOfBounds,
                    "List insertion index is out of bounds",
                ));
            }
            let mut next = list.clone();
            next.insert(*index, value.clone());
            Body::List(next)
        }
        _ => return Err(type_error("destination does not match parent kind")),
    };
    base.replace_at(parent.id(), &Value::trusted(parent.id(), body))
}

fn remove(base: &Value, target: ElementId) -> Result<(Value, Value)> {
    let destination = destination_of(base, target)?;
    let parent = base.get(destination.parent)?;
    let (body, removed) = match (parent.body(), destination.slot) {
        (Body::Map(map), Segment::Key(key)) => {
            let mut next = map.clone();
            let removed = next.remove(&key).unwrap();
            (Body::Map(next), removed)
        }
        (Body::List(list), Segment::Index(index)) => {
            let mut next = list.clone();
            let removed = next.remove(index);
            (Body::List(next), removed)
        }
        _ => unreachable!("destination_of returns the existing owning edge"),
    };
    Ok((
        base.replace_at(parent.id(), &Value::trusted(parent.id(), body))?,
        removed,
    ))
}
pub(crate) fn structural(reason: &str) -> Error {
    Error::new(ErrorCode::StructuralConflict, reason)
}
pub(crate) fn type_error(reason: &str) -> Error {
    Error::new(ErrorCode::TypeMismatch, reason)
}
