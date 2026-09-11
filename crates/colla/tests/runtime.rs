use colla::{
    Authority, AuthorityCheckpoint, Body, CollaError, Document, ErrorCode, History,
    HistoryCheckpoint, Segment, ServerMessage, SessionCheckpoint, SyncSession, Value,
};

fn map(values: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::map(values.into_iter().map(|(k, v)| (k.into(), v))).unwrap()
}
fn publish(authority: &mut Authority, session: &SyncSession) -> ServerMessage {
    let request = session.outbound().unwrap().unwrap();
    let (next, message) = authority.accept(&request).unwrap();
    *authority = next;
    assert!(matches!(message, ServerMessage::Commit(_)), "{message:?}");
    message
}

#[test]
fn failed_and_nested_edits_leave_all_participants_unchanged() {
    let value = map([
        ("count", Value::int(i64::MAX)),
        ("text", Value::text("A😀B").unwrap()),
    ]);
    let authority = Authority::create("doc", value).unwrap();
    let session = SyncSession::create("a", authority.snapshot()).unwrap();
    let doc = session.document();
    let history = History::attach(&doc).unwrap();
    let before = session.checkpoint().unwrap().encode();
    let result = doc.edit(|tx| {
        tx.text_replace(vec![Segment::Key("text".into())], 0, 1, "a")?;
        assert_eq!(
            doc.edit(|_| Ok(())).unwrap_err().code,
            ErrorCode::InvalidState
        );
        assert_eq!(doc.close().unwrap_err().code, ErrorCode::InvalidState);
        tx.increment(vec![Segment::Key("count".into())], 1)
    });
    assert_eq!(result.unwrap_err().code, ErrorCode::IntegerOverflow);
    assert_eq!(before, session.checkpoint().unwrap().encode());
    assert!(!history.can_undo().unwrap());
    doc.edit(|tx| {
        let path = vec![Segment::Key("text".into())];
        assert_eq!(
            tx.utf16_to_scalar(path.clone(), 2).unwrap_err().code,
            ErrorCode::InvalidUtf16Boundary
        );
        assert_eq!(tx.utf16_to_scalar(path, 3)?, 2);
        Ok(())
    })
    .unwrap();
    assert_eq!(doc.version().unwrap(), 0);
    let snapshot = doc.snapshot().unwrap();
    doc.close().unwrap();
    doc.close().unwrap();
    assert_eq!(Value::decode(&snapshot.encode()).unwrap(), snapshot);
}

#[test]
fn groups_and_checkpoints_restore_original_identities() {
    let child = Value::int(1);
    let list = Value::list(vec![child.clone()]).unwrap();
    let doc = Document::create(list.clone()).unwrap();
    let history = History::attach(&doc).unwrap();
    for _ in 0..3 {
        doc.edit_group(Some("typing".into()), |tx| tx.increment(child.id(), 1))
            .unwrap();
    }
    assert_eq!(doc.get(child.id()).unwrap().body(), &Body::Int(4));
    let checkpoint = HistoryCheckpoint::decode(&history.checkpoint().unwrap().encode()).unwrap();
    let restored = Document::create(doc.snapshot().unwrap()).unwrap();
    let restored_history = History::restore(&restored, checkpoint).unwrap();
    restored_history.undo().unwrap();
    assert_eq!(restored.snapshot().unwrap(), list);
    assert!(!restored_history.can_undo().unwrap());
    restored_history.redo().unwrap();
    assert_eq!(restored.snapshot().unwrap(), doc.snapshot().unwrap());
}

#[test]
fn immutable_retries_buffer_promotion_and_restart() {
    let value = Value::int(0);
    let id = value.id();
    let mut authority = Authority::create("doc", value).unwrap();
    let session = SyncSession::create("a", authority.snapshot()).unwrap();
    let doc = session.document();
    History::attach(&doc).unwrap();
    doc.edit(|tx| tx.increment(id, 1)).unwrap();
    let original = session.outbound().unwrap().unwrap();
    let bytes = original.encode();
    let mut external = original.encode();
    external.fill(0);
    doc.edit(|tx| tx.increment(id, 2)).unwrap();
    assert_eq!(session.outbound().unwrap().unwrap().encode(), bytes);
    let restored = SyncSession::restore(
        SessionCheckpoint::decode(&session.checkpoint().unwrap().encode()).unwrap(),
    )
    .unwrap();
    let message = publish(&mut authority, &restored);
    let revision = authority.revision();
    let (same, duplicate) = authority.accept(&original).unwrap();
    assert_eq!(same.revision(), revision);
    assert_eq!(duplicate, message);
    restored.receive(&message).unwrap();
    assert_eq!(restored.document().get(id).unwrap().body(), &Body::Int(3));
    let second = restored.outbound().unwrap().unwrap();
    assert_eq!(second.sequence(), 2);
    assert_eq!(second.base_revision(), 1);
    let message = publish(&mut authority, &restored);
    restored.receive(&message).unwrap();
    restored.receive(&message).unwrap();
    assert!(restored.outbound().unwrap().is_none());
    assert_eq!(
        restored.document().snapshot().unwrap(),
        *authority.snapshot().value()
    );
    let recovered =
        Authority::restore(AuthorityCheckpoint::decode(&authority.checkpoint().encode()).unwrap())
            .unwrap();
    assert_eq!(recovered.snapshot(), authority.snapshot());
}

#[test]
fn remote_reentry_gaps_and_recovery_preserve_work() {
    let value = Value::int(0);
    let id = value.id();
    let mut authority = Authority::create("doc", value).unwrap();
    let a = SyncSession::create("a", authority.snapshot()).unwrap();
    let b = SyncSession::create("b", authority.snapshot()).unwrap();
    b.document().edit(|tx| tx.increment(id, 1)).unwrap();
    let first = publish(&mut authority, &b);
    b.receive(&first).unwrap();
    b.document().edit(|tx| tx.increment(id, 1)).unwrap();
    let second = publish(&mut authority, &b);
    let before = a.checkpoint().unwrap().encode();
    assert_eq!(
        a.receive(&second).unwrap_err().code,
        ErrorCode::MissingRevision
    );
    assert_eq!(before, a.checkpoint().unwrap().encode());
    a.document()
        .edit(|_| {
            assert_eq!(a.receive(&first).unwrap_err().code, ErrorCode::InvalidState);
            Ok(())
        })
        .unwrap();
    assert_eq!(before, a.checkpoint().unwrap().encode());
    a.document().edit(|tx| tx.increment(id, 9)).unwrap();
    authority = authority.compact(2).unwrap();
    let (_, rejection) = authority.accept(&a.outbound().unwrap().unwrap()).unwrap();
    a.receive(&rejection).unwrap();
    assert_eq!(
        a.recovery_reason().unwrap().unwrap().code,
        ErrorCode::HistoryExpired
    );
    assert!(a.outbound().unwrap().is_none());
    a.document().edit(|tx| tx.increment(id, 1)).unwrap();
    assert_eq!(a.document().get(id).unwrap().body(), &Body::Int(10));
    SyncSession::restore(SessionCheckpoint::decode(&a.checkpoint().unwrap().encode()).unwrap())
        .unwrap();
}

#[test]
fn undo_move_keeps_remote_content_and_syncs_as_a_new_request() {
    let item = Value::int(0);
    let from = Value::list(vec![item.clone()]).unwrap();
    let to = Value::list(vec![]).unwrap();
    let mut authority =
        Authority::create("doc", map([("from", from.clone()), ("to", to.clone())])).unwrap();
    let a = SyncSession::create("a", authority.snapshot()).unwrap();
    let b = SyncSession::create("b", authority.snapshot()).unwrap();
    let history = History::attach(&a.document()).unwrap();
    a.document()
        .edit(|tx| tx.move_to(item.id(), to.id(), Segment::Index(0)))
        .unwrap();
    b.document().edit(|tx| tx.increment(item.id(), 7)).unwrap();
    let message = publish(&mut authority, &b);
    a.receive(&message).unwrap();
    b.receive(&message).unwrap();
    let message = publish(&mut authority, &a);
    a.receive(&message).unwrap();
    b.receive(&message).unwrap();
    history.undo().unwrap();
    assert_eq!(a.document().get(item.id()).unwrap().body(), &Body::Int(7));
    assert_eq!(
        a.document().snapshot().unwrap().path_of(item.id()).unwrap(),
        vec![Segment::Key("from".into()), Segment::Index(0)]
    );
    let message = publish(&mut authority, &a);
    a.receive(&message).unwrap();
    b.receive(&message).unwrap();
    assert_eq!(
        a.document().snapshot().unwrap(),
        b.document().snapshot().unwrap()
    );
}

#[test]
fn three_clients_with_delays_retries_buffers_and_restarts_converge() {
    let items: Vec<_> = (0..4).map(Value::int).collect();
    let list = Value::list(items.clone()).unwrap();
    let text = Value::text("😀").unwrap();
    let mut authority = Authority::create(
        "random",
        map([("items", list.clone()), ("title", text.clone())]),
    )
    .unwrap();
    let mut sessions: Vec<_> = (0..3)
        .map(|i| SyncSession::create(format!("c{i}"), authority.snapshot()).unwrap())
        .collect();
    for session in &sessions {
        History::attach(&session.document()).unwrap();
    }
    let mut seed = 41u64;
    for step in 0..180 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let client = (seed >> 32) as usize % 3;
        let session = &sessions[client];
        session
            .document()
            .edit(|tx| match step % 3 {
                0 => tx.increment(items[step % items.len()].id(), 1),
                1 => tx.text_replace(text.id(), 0, 0, &format!("{client}")),
                _ => tx.move_to(
                    items[step % items.len()].id(),
                    list.id(),
                    Segment::Index(step % 4),
                ),
            })
            .unwrap();
        if step % 2 == 0 && session.outbound().unwrap().is_some() {
            publish(&mut authority, session);
        }
        let receiver = (client + 1) % 3;
        let commits = authority
            .commits_since(sessions[receiver].revision().unwrap())
            .unwrap();
        if let Some(commit) = commits.first() {
            let message = ServerMessage::Commit(commit.clone());
            sessions[receiver].receive(&message).unwrap();
            sessions[receiver].receive(&message).unwrap();
        }
        if step % 17 == 0 {
            sessions[client] = SyncSession::restore(
                SessionCheckpoint::decode(&sessions[client].checkpoint().unwrap().encode())
                    .unwrap(),
            )
            .unwrap();
        }
        if step % 23 == 0 {
            authority = Authority::restore(
                AuthorityCheckpoint::decode(&authority.checkpoint().encode()).unwrap(),
            )
            .unwrap();
        }
    }
    for _ in 0..30 {
        for session in &sessions {
            if session.outbound().unwrap().is_some() {
                publish(&mut authority, session);
            }
        }
        for session in &sessions {
            for commit in authority
                .commits_since(session.revision().unwrap())
                .unwrap()
            {
                session.receive(&ServerMessage::Commit(commit)).unwrap();
            }
        }
        if sessions.iter().all(|s| s.outbound().unwrap().is_none()) {
            break;
        }
    }
    for session in &sessions {
        assert!(session.outbound().unwrap().is_none());
        assert!(session.recovery_reason().unwrap().is_none());
        assert_eq!(
            session.document().snapshot().unwrap(),
            *authority.snapshot().value()
        );
    }
}

#[test]
fn explicit_callback_error_rolls_back() {
    let value = Value::int(0);
    let id = value.id();
    let doc = Document::create(value.clone()).unwrap();
    let result = doc.edit(|tx| {
        tx.increment(id, 1)?;
        Err(CollaError::new(ErrorCode::InvalidArgument, "abort"))
    });
    assert!(result.is_err());
    assert_eq!(doc.snapshot().unwrap(), value);
}

#[test]
fn undo_one_move_preserves_the_other_clients_move() {
    let a = Value::int(0);
    let b = Value::int(1);
    let c = Value::int(2);
    let value = Value::list(vec![a.clone(), b.clone(), c.clone()]).unwrap();
    let root = value.id();
    let mut authority = Authority::create("moves", value).unwrap();
    let left = SyncSession::create("left", authority.snapshot()).unwrap();
    let right = SyncSession::create("right", authority.snapshot()).unwrap();
    let history = History::attach(&left.document()).unwrap();
    left.document()
        .edit(|tx| tx.move_to(a.id(), root, Segment::Index(2)))
        .unwrap();
    right
        .document()
        .edit(|tx| tx.move_to(b.id(), root, Segment::Index(2)))
        .unwrap();
    let message = publish(&mut authority, &right);
    left.receive(&message).unwrap();
    right.receive(&message).unwrap();
    history.undo().unwrap();
    assert_eq!(
        left.document().snapshot().unwrap().body(),
        &Body::List(vec![a, c, b])
    );
}

#[test]
fn undo_rich_insertion_preserves_remote_formatting() {
    use colla::{Attr, AttrPatch, Attrs, RichOp, RichSpan};
    let value = Value::rich_text(vec![RichSpan::Text {
        text: "ab".into(),
        attrs: Attrs::new(),
    }])
    .unwrap();
    let root = value.id();
    let mut authority = Authority::create("rich", value).unwrap();
    let left = SyncSession::create("left", authority.snapshot()).unwrap();
    let right = SyncSession::create("right", authority.snapshot()).unwrap();
    let history = History::attach(&left.document()).unwrap();
    left.document()
        .edit(|tx| {
            tx.rich_text_edit(
                root,
                vec![
                    RichOp::Retain {
                        len: 1,
                        attrs: AttrPatch::new(),
                    },
                    RichOp::Insert(RichSpan::Text {
                        text: "X".into(),
                        attrs: Attrs::new(),
                    }),
                ],
            )
        })
        .unwrap();
    right
        .document()
        .edit(|tx| {
            tx.rich_text_edit(
                root,
                vec![
                    RichOp::Retain {
                        len: 1,
                        attrs: AttrPatch::new(),
                    },
                    RichOp::Retain {
                        len: 1,
                        attrs: AttrPatch::from([("bold".into(), Some(Attr::Bool(true)))]),
                    },
                ],
            )
        })
        .unwrap();
    let expected = right.document().snapshot().unwrap();
    let message = publish(&mut authority, &right);
    left.receive(&message).unwrap();
    right.receive(&message).unwrap();
    history.undo().unwrap();
    assert_eq!(left.document().snapshot().unwrap(), expected);
}

#[test]
fn history_restores_deleted_and_copied_identities_and_skips_remote_noops() {
    let value = Value::int(1);
    let original = value.id();
    let root = map([
        ("item", value),
        ("list", Value::list(vec![]).unwrap()),
        ("link", Value::reference(original)),
    ]);
    let doc = Document::create(root.clone()).unwrap();
    let history = History::attach_with_capacity(&doc, 2).unwrap();
    doc.edit(|tx| tx.delete(original)).unwrap();
    assert!(doc
        .resolve(colla::Ref { target: original })
        .unwrap()
        .is_none());
    history.undo().unwrap();
    assert_eq!(doc.snapshot().unwrap(), root);
    let mut copied = None;
    doc.edit(|tx| {
        copied = Some(tx.copy(
            original,
            vec![Segment::Key("list".into())],
            Segment::Index(0),
        )?);
        Ok(())
    })
    .unwrap();
    let copied = copied.unwrap();
    history.undo().unwrap();
    assert!(!doc.has(copied).unwrap());
    history.redo().unwrap();
    assert!(doc.has(copied).unwrap());
    assert_eq!(doc.references_to(original).unwrap().len(), 1);
    let authority = Authority::create("history", root).unwrap();
    let a = SyncSession::create("a", authority.snapshot()).unwrap();
    let b = SyncSession::create("b", authority.snapshot()).unwrap();
    let history = History::attach(&a.document()).unwrap();
    a.document().edit(|tx| tx.increment(original, 1)).unwrap();
    b.document().edit(|tx| tx.delete(original)).unwrap();
    let (authority, commit) = authority.accept(&b.outbound().unwrap().unwrap()).unwrap();
    a.receive(&commit).unwrap();
    assert!(!history.can_undo().unwrap());
    assert!(history.undo().unwrap().is_none());
    let (_, own) = authority.accept(&a.outbound().unwrap().unwrap()).unwrap();
    a.receive(&own).unwrap();
    assert!(!a.document().has(original).unwrap());
    assert!(a.outbound().unwrap().is_none());
}

#[test]
fn injected_allocator_burns_failed_transaction_ids() {
    let mut allocator = colla::IdAllocator::deterministic([91; 16]);
    allocator.scope(|| {
        let base = Value::int(0);
        let doc = Document::create(base.clone()).unwrap();
        let abandoned = Value::int(1);
        assert!(doc
            .edit(|tx| {
                tx.set(base.id(), abandoned.clone())?;
                Err(CollaError::new(ErrorCode::InvalidArgument, "abort"))
            })
            .is_err());
        let next = Value::int(2);
        assert_eq!(next.id().namespace(), [91; 16]);
        assert!(next.id().sequence() > abandoned.id().sequence() + 1);
        assert_eq!(doc.snapshot().unwrap(), base);
    });
    assert!(allocator.allocate().unwrap().sequence() > 4);
}
