use super::marshal as m;
use colla::CollaError as Error;
use colla::*;
use js_sys::{Array, BigInt, Object};
use wasm_bindgen::prelude::*;
type JsResult<T> = std::result::Result<T, JsValue>;

#[wasm_bindgen]
pub struct CoreValue {
    value: Value,
}
impl CoreValue {
    pub(crate) fn from_value(value: Value) -> Self {
        Self { value }
    }
}
#[wasm_bindgen]
impl CoreValue {
    pub fn from_input(input: JsValue) -> JsResult<Self> {
        m::value(&input).map(Self::from_value).map_err(m::error)
    }
    pub fn decode(bytes: &[u8]) -> JsResult<Self> {
        Value::decode(bytes).map(Self::from_value).map_err(m::error)
    }
    pub fn encode(&self) -> Vec<u8> {
        self.value.encode()
    }
    pub fn projection(&self) -> JsValue {
        m::projection(&self.value)
    }
    pub fn id(&self) -> String {
        self.value.id().to_string()
    }
    pub fn kind(&self) -> String {
        self.value.kind().into()
    }
    pub fn get(&self, location: JsValue) -> JsResult<Self> {
        m::location(&location)
            .and_then(|location| self.value.get(location))
            .map(Self::from_value)
            .map_err(m::error)
    }
    pub fn path_of(&self, id: &str) -> JsResult<JsValue> {
        id.parse()
            .map(|id| m::path(self.value.path_of(id)))
            .map_err(m::error)
    }
    pub fn resolve(&self, id: &str) -> JsResult<Option<Self>> {
        id.parse()
            .map(|target| self.value.resolve(Ref { target }).map(Self::from_value))
            .map_err(m::error)
    }
    pub fn references_to(&self, id: &str) -> JsResult<Array> {
        id.parse()
            .map(|id| {
                self.value
                    .references_to(id)
                    .into_iter()
                    .map(|id| JsValue::from(id.to_string()))
                    .collect()
            })
            .map_err(m::error)
    }
    pub fn equals(&self, other: &Self) -> bool {
        self.value == other.value
    }
    pub fn content_equals(&self, other: &Self) -> bool {
        self.value.content_equals(&other.value)
    }
    pub fn copied(&self) -> JsResult<Self> {
        self.value.copied().map(Self::from_value).map_err(m::error)
    }
}
#[wasm_bindgen]
pub fn core_validate_id(input: &str) -> JsResult<String> {
    input
        .parse::<ElementId>()
        .map(|id| id.to_string())
        .map_err(m::error)
}

#[wasm_bindgen]
pub struct CoreChange {
    change: Change,
}
#[wasm_bindgen]
impl CoreChange {
    pub fn from_input(input: JsValue) -> JsResult<Self> {
        m::change(&input)
            .map(|change| Self { change })
            .map_err(m::error)
    }
    pub fn decode(bytes: &[u8]) -> JsResult<Self> {
        Change::decode(bytes)
            .map(|change| Self { change })
            .map_err(m::error)
    }
    pub fn encode(&self) -> Vec<u8> {
        self.change.encode()
    }
    pub fn operations(&self) -> JsValue {
        m::operations(&self.change)
    }
    pub fn is_noop(&self) -> bool {
        self.change.is_noop()
    }
}
#[wasm_bindgen]
pub fn core_apply(base: &CoreValue, change: &CoreChange) -> JsResult<CoreValue> {
    apply(&base.value, &change.change)
        .map(CoreValue::from_value)
        .map_err(m::error)
}
#[wasm_bindgen]
pub fn core_invert(base: &CoreValue, change: &CoreChange) -> JsResult<CoreChange> {
    invert(&base.value, &change.change)
        .map(|change| CoreChange { change })
        .map_err(m::error)
}
#[wasm_bindgen]
pub fn core_compose(
    base: &CoreValue,
    first: &CoreChange,
    second: &CoreChange,
) -> JsResult<CoreChange> {
    compose(&base.value, &first.change, &second.change)
        .map(|change| CoreChange { change })
        .map_err(m::error)
}
#[wasm_bindgen]
pub fn core_transform(
    base: &CoreValue,
    left: &CoreChange,
    right: &CoreChange,
    left_priority: bool,
) -> JsResult<Array> {
    transform(
        &base.value,
        &left.change,
        &right.change,
        if left_priority {
            Priority::Left
        } else {
            Priority::Right
        },
    )
    .map(|(left, right)| {
        Array::of2(
            &CoreChange { change: left }.into(),
            &CoreChange { change: right }.into(),
        )
    })
    .map_err(m::error)
}
fn edit_js(edit: Option<EditResult>) -> JsValue {
    let Some(edit) = edit else {
        return JsValue::NULL;
    };
    let out = Object::new();
    m::set(&out, "before", &CoreValue::from_value(edit.before).into());
    m::set(&out, "after", &CoreValue::from_value(edit.after).into());
    m::set(&out, "editSteps", &m::operations(&edit.change));
    m::set(
        &out,
        "change",
        &CoreChange {
            change: edit.change,
        }
        .into(),
    );
    m::set(
        &out,
        "inverse",
        &CoreChange {
            change: edit.inverse,
        }
        .into(),
    );
    m::set(&out, "version", &BigInt::from(edit.version).into());
    m::set(
        &out,
        "origin",
        &match edit.origin {
            Origin::Local => "local",
            Origin::Remote => "remote",
            Origin::Undo => "undo",
            Origin::Redo => "redo",
        }
        .into(),
    );
    out.into()
}

#[wasm_bindgen]
pub struct CoreDocument {
    document: Document,
    transaction: Option<Transaction>,
}
#[wasm_bindgen]
impl CoreDocument {
    pub fn create(value: &CoreValue) -> JsResult<Self> {
        Document::create(value.value.clone())
            .map(|document| Self {
                document,
                transaction: None,
            })
            .map_err(m::error)
    }
    pub fn snapshot(&self) -> JsResult<CoreValue> {
        self.document
            .snapshot()
            .map(CoreValue::from_value)
            .map_err(m::error)
    }
    pub fn version(&self) -> JsResult<u64> {
        self.document.version().map_err(m::error)
    }
    pub fn close(&self) -> JsResult<()> {
        self.document.close().map_err(m::error)
    }
    pub fn begin(&mut self, group: Option<String>) -> JsResult<()> {
        if self.transaction.is_some() {
            return Err(m::error(Error::new(
                ErrorCode::InvalidState,
                "transaction already active",
            )));
        }
        self.transaction = Some(binding::begin(&self.document, group).map_err(m::error)?);
        Ok(())
    }
    pub fn commit(&mut self) -> JsResult<JsValue> {
        let mut transaction = self.transaction.take().ok_or_else(|| {
            m::error(Error::new(
                ErrorCode::InvalidState,
                "transaction scope ended",
            ))
        })?;
        binding::commit(&mut transaction)
            .map(edit_js)
            .map_err(m::error)
    }
    pub fn rollback(&mut self) {
        self.transaction.take();
    }
    pub fn transaction_snapshot(&self) -> JsResult<CoreValue> {
        self.transaction
            .as_ref()
            .ok_or_else(|| {
                m::error(Error::new(
                    ErrorCode::InvalidState,
                    "transaction scope ended",
                ))
            })?
            .snapshot()
            .map(CoreValue::from_value)
            .map_err(m::error)
    }
    pub fn transaction_apply(&mut self, change: &CoreChange) -> JsResult<()> {
        self.transaction
            .as_mut()
            .ok_or_else(|| {
                m::error(Error::new(
                    ErrorCode::InvalidState,
                    "transaction scope ended",
                ))
            })?
            .apply(&change.change)
            .map_err(m::error)
    }
    pub fn apply(&self, change: &CoreChange) -> JsResult<JsValue> {
        self.document
            .apply(&change.change)
            .map(edit_js)
            .map_err(m::error)
    }
    pub fn command(&mut self, input: JsValue) -> JsResult<JsValue> {
        self.run(&input).map_err(m::error)
    }
    pub fn history(&self, capacity: JsValue) -> JsResult<CoreHistory> {
        m::index(&capacity)
            .and_then(|capacity| History::attach_with_capacity(&self.document, capacity))
            .map(|history| CoreHistory { history })
            .map_err(m::error)
    }
    pub fn restore_history(&self, checkpoint: &CoreWire) -> JsResult<CoreHistory> {
        let Wire::History(checkpoint) = &checkpoint.wire else {
            return Err(m::error(m::argument("expected HistoryCheckpoint")));
        };
        History::restore(&self.document, checkpoint.clone())
            .map(|history| CoreHistory { history })
            .map_err(m::error)
    }
}
impl CoreDocument {
    fn run(&mut self, input: &JsValue) -> Result<JsValue> {
        let tx = self
            .transaction
            .as_mut()
            .ok_or_else(|| Error::new(ErrorCode::InvalidState, "transaction scope ended"))?;
        let input = m::array(input)?;
        let target = m::location(&input.get(1))?;
        match m::index(&input.get(0))? {
            0 => tx.set(target, m::value(&input.get(2))?)?,
            1 => tx.delete(target)?,
            2 => tx.move_to(
                target,
                m::location(&input.get(2))?,
                m::segment(&input.get(3))?,
            )?,
            3 => {
                return Ok(tx
                    .copy(
                        target,
                        m::location(&input.get(2))?,
                        m::segment(&input.get(3))?,
                    )?
                    .to_string()
                    .into())
            }
            4 => tx.increment(target, m::signed(&input.get(2))?)?,
            5 => tx.list_replace(
                target,
                m::index(&input.get(2))?,
                m::index(&input.get(3))?,
                m::array(&input.get(4))?
                    .iter()
                    .map(|v| m::value(&v))
                    .collect::<Result<_>>()?,
            )?,
            6..=8 => {
                let index = m::index(&input.get(2))?;
                let count = m::index(&input.get(3))?;
                let start = tx.utf16_to_scalar(target.clone(), index)?;
                let end = tx.utf16_to_scalar(
                    target.clone(),
                    index
                        .checked_add(count)
                        .ok_or_else(|| Error::new(ErrorCode::OutOfBounds, "range overflow"))?,
                )?;
                match m::index(&input.get(0))? {
                    6 => tx.text_replace(target, start, end - start, &m::string(&input.get(4))?)?,
                    7 => {
                        let mut ops = vec![
                            RichOp::Retain {
                                len: start,
                                attrs: AttrPatch::new(),
                            },
                            RichOp::Delete(end - start),
                        ];
                        for input in m::array(&input.get(4))?.iter() {
                            let span = m::span(&input)?;
                            ops.push(RichOp::Insert(match span {
                                RichSpan::Embed { value, attrs } => RichSpan::Embed {
                                    value: value.copied()?,
                                    attrs,
                                },
                                other => other,
                            }));
                        }
                        tx.rich_text_edit(target, ops)?;
                    }
                    _ => tx.rich_text_edit(
                        target,
                        vec![
                            RichOp::Retain {
                                len: start,
                                attrs: AttrPatch::new(),
                            },
                            RichOp::Retain {
                                len: end - start,
                                attrs: m::patch(&input.get(4))?,
                            },
                        ],
                    )?,
                }
            }
            _ => return Err(m::argument("unknown editing command")),
        }
        Ok(JsValue::UNDEFINED)
    }
}

#[wasm_bindgen]
pub struct CoreHistory {
    history: History,
}
#[wasm_bindgen]
impl CoreHistory {
    pub fn can_undo(&self) -> JsResult<bool> {
        self.history.can_undo().map_err(m::error)
    }
    pub fn can_redo(&self) -> JsResult<bool> {
        self.history.can_redo().map_err(m::error)
    }
    pub fn undo(&self) -> JsResult<JsValue> {
        self.history.undo().map(edit_js).map_err(m::error)
    }
    pub fn redo(&self) -> JsResult<JsValue> {
        self.history.redo().map(edit_js).map_err(m::error)
    }
    pub fn clear(&self) -> JsResult<()> {
        self.history.clear().map_err(m::error)
    }
    pub fn close(&self) -> JsResult<()> {
        self.history.close().map_err(m::error)
    }
    pub fn checkpoint(&self) -> JsResult<CoreWire> {
        self.history
            .checkpoint()
            .map(|value| CoreWire {
                wire: Wire::History(value),
            })
            .map_err(m::error)
    }
}
enum Wire {
    Snapshot(SyncSnapshot),
    Submission(Submission),
    Message(ServerMessage),
    Session(Box<SessionCheckpoint>),
    History(HistoryCheckpoint),
    Authority(AuthorityCheckpoint),
}
#[wasm_bindgen]
pub struct CoreWire {
    wire: Wire,
}
#[wasm_bindgen]
impl CoreWire {
    pub fn decode(kind: u8, bytes: &[u8]) -> JsResult<Self> {
        let result = match kind {
            3 => SyncSnapshot::decode(bytes).map(Wire::Snapshot),
            4 => Submission::decode(bytes).map(Wire::Submission),
            5 => ServerMessage::decode(bytes).map(Wire::Message),
            6 => SessionCheckpoint::decode(bytes).map(|v| Wire::Session(Box::new(v))),
            7 => HistoryCheckpoint::decode(bytes).map(Wire::History),
            8 => AuthorityCheckpoint::decode(bytes).map(Wire::Authority),
            _ => Err(m::argument("unknown wire object")),
        };
        result.map(|wire| Self { wire }).map_err(m::error)
    }
    pub fn encode(&self) -> Vec<u8> {
        match &self.wire {
            Wire::Snapshot(v) => v.encode(),
            Wire::Submission(v) => v.encode(),
            Wire::Message(v) => v.encode(),
            Wire::Session(v) => v.encode(),
            Wire::History(v) => v.encode(),
            Wire::Authority(v) => v.encode(),
        }
    }
    pub fn info(&self) -> JsValue {
        let out = Object::new();
        match &self.wire {
            Wire::Snapshot(v) => {
                m::set(&out, "documentId", &v.document_id().into());
                m::set(&out, "revision", &BigInt::from(v.revision()).into());
                m::set(
                    &out,
                    "value",
                    &CoreValue::from_value(v.value().clone()).into(),
                );
            }
            Wire::Submission(v) => {
                m::set(&out, "documentId", &v.document_id().into());
                m::set(&out, "clientId", &v.client_id().into());
                m::set(&out, "sequence", &BigInt::from(v.sequence()).into());
                m::set(
                    &out,
                    "baseRevision",
                    &BigInt::from(v.base_revision()).into(),
                );
                m::set(
                    &out,
                    "change",
                    &CoreChange {
                        change: v.change().clone(),
                    }
                    .into(),
                );
            }
            Wire::Message(ServerMessage::Commit(v)) => {
                m::set(&out, "type", &"commit".into());
                m::set(&out, "documentId", &v.document_id().into());
                m::set(&out, "clientId", &v.client_id().into());
                m::set(&out, "sequence", &BigInt::from(v.sequence()).into());
                m::set(&out, "revision", &BigInt::from(v.revision()).into());
                m::set(
                    &out,
                    "change",
                    &CoreChange {
                        change: v.change().clone(),
                    }
                    .into(),
                );
            }
            Wire::Message(ServerMessage::Rejection(v)) => {
                m::set(&out, "type", &"rejection".into());
                m::set(&out, "documentId", &v.document_id().into());
                m::set(&out, "clientId", &v.client_id().into());
                m::set(&out, "sequence", &BigInt::from(v.sequence()).into());
                m::set(&out, "reason", &m::error(v.reason().clone()));
            }
            _ => {}
        }
        out.into()
    }
}

#[wasm_bindgen]
pub struct CoreAuthority {
    authority: Authority,
}
#[wasm_bindgen]
impl CoreAuthority {
    pub fn create(document_id: &str, value: &CoreValue) -> JsResult<Self> {
        Authority::create(document_id, value.value.clone())
            .map(|authority| Self { authority })
            .map_err(m::error)
    }
    pub fn restore(checkpoint: &CoreWire) -> JsResult<Self> {
        let Wire::Authority(checkpoint) = &checkpoint.wire else {
            return Err(m::error(m::argument("expected AuthorityCheckpoint")));
        };
        Authority::restore(checkpoint.clone())
            .map(|authority| Self { authority })
            .map_err(m::error)
    }
    pub fn snapshot(&self) -> CoreWire {
        CoreWire {
            wire: Wire::Snapshot(self.authority.snapshot()),
        }
    }
    pub fn checkpoint(&self) -> CoreWire {
        CoreWire {
            wire: Wire::Authority(self.authority.checkpoint()),
        }
    }
    pub fn revision(&self) -> u64 {
        self.authority.revision()
    }
    pub fn compact(&self, revision: JsValue) -> JsResult<Self> {
        m::unsigned(&revision)
            .and_then(|revision| self.authority.compact(revision))
            .map(|authority| Self { authority })
            .map_err(m::error)
    }
    pub fn commits_since(&self, revision: JsValue) -> JsResult<Array> {
        m::unsigned(&revision)
            .and_then(|revision| self.authority.commits_since(revision))
            .map(|commits| {
                commits
                    .into_iter()
                    .map(|commit| {
                        JsValue::from(CoreWire {
                            wire: Wire::Message(ServerMessage::Commit(commit)),
                        })
                    })
                    .collect()
            })
            .map_err(m::error)
    }
    pub fn accept(&self, submission: &CoreWire) -> JsResult<Array> {
        let Wire::Submission(submission) = &submission.wire else {
            return Err(m::error(m::argument("expected Submission")));
        };
        self.authority
            .accept(submission)
            .map(|(authority, message)| {
                Array::of2(
                    &Self { authority }.into(),
                    &CoreWire {
                        wire: Wire::Message(message),
                    }
                    .into(),
                )
            })
            .map_err(m::error)
    }
}

#[wasm_bindgen]
pub struct CoreSession {
    session: SyncSession,
}
#[wasm_bindgen]
impl CoreSession {
    pub fn create(client_id: &str, snapshot: &CoreWire) -> JsResult<Self> {
        let Wire::Snapshot(snapshot) = &snapshot.wire else {
            return Err(m::error(m::argument("expected SyncSnapshot")));
        };
        SyncSession::create(client_id, snapshot.clone())
            .map(|session| Self { session })
            .map_err(m::error)
    }
    pub fn restore(checkpoint: &CoreWire) -> JsResult<Self> {
        let Wire::Session(checkpoint) = &checkpoint.wire else {
            return Err(m::error(m::argument("expected SessionCheckpoint")));
        };
        SyncSession::restore((**checkpoint).clone())
            .map(|session| Self { session })
            .map_err(m::error)
    }
    pub fn document(&self) -> CoreDocument {
        CoreDocument {
            document: self.session.document(),
            transaction: None,
        }
    }
    pub fn outbound(&self) -> JsResult<Option<CoreWire>> {
        self.session
            .outbound()
            .map(|value| {
                value.map(|value| CoreWire {
                    wire: Wire::Submission(value),
                })
            })
            .map_err(m::error)
    }
    pub fn checkpoint(&self) -> JsResult<CoreWire> {
        self.session
            .checkpoint()
            .map(|value| CoreWire {
                wire: Wire::Session(Box::new(value)),
            })
            .map_err(m::error)
    }
    pub fn revision(&self) -> JsResult<u64> {
        self.session.revision().map_err(m::error)
    }
    pub fn recovery_reason(&self) -> JsResult<JsValue> {
        self.session
            .recovery_reason()
            .map(|reason| reason.map(m::error).unwrap_or(JsValue::UNDEFINED))
            .map_err(m::error)
    }
    pub fn close(&self) -> JsResult<()> {
        self.session.close().map_err(m::error)
    }
    pub fn receive(&self, message: &CoreWire) -> JsResult<JsValue> {
        let Wire::Message(message) = &message.wire else {
            return Err(m::error(m::argument("expected ServerMessage")));
        };
        self.session.receive(message).map(edit_js).map_err(m::error)
    }
}
