// Randomized runtime invariants: History round trips, centralized convergence
// across interleaved clients, and TP1 for identity-based Move/Ref changes.
use colla::*;
use proptest::prelude::*;

fn key(name: &str) -> Segment {
    Segment::Key(name.into())
}
fn initial() -> Value {
    Value::map([
        ("count".into(), Value::int(0)),
        ("title".into(), Value::text("ab").unwrap()),
        (
            "todo".into(),
            Value::list(vec![Value::int(1), Value::int(2), Value::int(3)]).unwrap(),
        ),
        ("done".into(), Value::list(vec![]).unwrap()),
        ("selected".into(), Value::null()),
    ])
    .unwrap()
}
fn list_len(tx: &Transaction, name: &str) -> Result<usize> {
    match tx.get(vec![key(name)])?.body() {
        Body::List(list) => Ok(list.len()),
        _ => unreachable!("fixture lists stay lists"),
    }
}
fn text_len(tx: &Transaction) -> Result<usize> {
    match tx.get(vec![key("title")])?.body() {
        Body::Text(text) => Ok(text.chars().count()),
        _ => unreachable!("fixture title stays Text"),
    }
}

/// One local edit chosen from `action`; positions are reduced modulo the current size.
fn edit(tx: &mut Transaction, action: (u8, usize, usize), structural: bool) -> Result<()> {
    let (kind, a, b) = action;
    match kind % if structural { 6 } else { 3 } {
        0 => tx.increment(vec![key("count")], 1 + (a % 5) as i64),
        1 => {
            let pos = a % (text_len(tx)? + 1);
            tx.text_replace(vec![key("title")], pos, 0, "x")
        }
        2 => {
            let index = a % (list_len(tx, "todo")? + 1);
            tx.list_replace(vec![key("todo")], index, 0, vec![Value::int(b as i64)])
        }
        3 => {
            let len = list_len(tx, "todo")?;
            if len == 0 {
                return Ok(());
            }
            let index = b % (list_len(tx, "done")? + 1);
            tx.move_to(
                vec![key("todo"), Segment::Index(a % len)],
                vec![key("done")],
                Segment::Index(index),
            )
        }
        4 => {
            let len = list_len(tx, "todo")?;
            if len == 0 {
                return Ok(());
            }
            let target = tx.get(vec![key("todo"), Segment::Index(a % len)])?.id();
            tx.set(vec![key("selected")], Value::reference(target))
        }
        _ => {
            let len = list_len(tx, "done")?;
            if len == 0 {
                return Ok(());
            }
            tx.delete(vec![key("done"), Segment::Index(a % len)])
        }
    }
}

fn action() -> impl Strategy<Value = (u8, usize, usize)> {
    (any::<u8>(), 0usize..64, 0usize..64)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Undoing every edit restores the exact initial snapshot, identities included,
    /// and redoing every edit restores the exact final snapshot.
    #[test]
    fn history_undo_redo_round_trips(actions in prop::collection::vec(action(), 1..12)) {
        let base = initial();
        let doc = Document::create(base.clone()).unwrap();
        let history = History::attach_with_capacity(&doc, actions.len()).unwrap();
        for action in &actions {
            doc.edit(|tx| edit(tx, *action, true)).unwrap();
        }
        let after = doc.snapshot().unwrap();
        while history.undo().unwrap().is_some() {}
        prop_assert_eq!(doc.snapshot().unwrap(), base);
        while history.redo().unwrap().is_some() {}
        prop_assert_eq!(doc.snapshot().unwrap(), after);
    }

    /// Clients that edit, submit and receive in any interleaving converge on the
    /// Authority's content once every request is confirmed.
    #[test]
    fn interleaved_sessions_converge(
        steps in prop::collection::vec((0usize..3, 0u8..3, action()), 1..40),
    ) {
        let mut authority = Authority::create("doc", initial()).unwrap();
        let sessions: Vec<_> = ["a", "b", "c"]
            .iter()
            .map(|id| SyncSession::create(*id, authority.snapshot()).unwrap())
            .collect();
        let mut submitted = vec![None; sessions.len()];
        let submit = |authority: &mut Authority, submitted: &mut Vec<Option<u64>>, client: usize| {
            if let Some(request) = sessions[client].outbound().unwrap() {
                if submitted[client] != Some(request.sequence()) {
                    let (next, message) = authority.accept(&request).unwrap();
                    assert!(matches!(message, ServerMessage::Commit(_)), "{message:?}");
                    *authority = next;
                    submitted[client] = Some(request.sequence());
                }
            }
        };
        let deliver = |authority: &Authority, client: usize, limit: usize| {
            let session = &sessions[client];
            for commit in authority.commits_since(session.revision().unwrap()).unwrap().into_iter().take(limit) {
                session.receive(&ServerMessage::Commit(commit)).unwrap();
            }
        };
        for (client, kind, action) in steps {
            match kind {
                0 => {
                    sessions[client].document().edit(|tx| edit(tx, action, false)).unwrap();
                }
                1 => submit(&mut authority, &mut submitted, client),
                _ => deliver(&authority, client, 1 + action.1 % 3),
            }
        }
        loop {
            let before = authority.revision();
            for client in 0..sessions.len() {
                deliver(&authority, client, usize::MAX);
                submit(&mut authority, &mut submitted, client);
            }
            if authority.revision() == before
                && sessions.iter().all(|s| s.outbound().unwrap().is_none())
            {
                break;
            }
        }
        let expected = authority.snapshot().value().clone();
        for session in &sessions {
            prop_assert_eq!(session.revision().unwrap(), authority.revision());
            prop_assert_eq!(session.document().snapshot().unwrap(), expected.clone());
        }
    }

    /// Concurrent Move/Ref/Delete edits either transform to the same content from
    /// both orders (TP1) or fail atomically with a typed conflict.
    #[test]
    fn identity_edits_satisfy_tp1(
        left in prop::collection::vec(action(), 1..4),
        right in prop::collection::vec(action(), 1..4),
    ) {
        let base = initial();
        let branch = |actions: &[(u8, usize, usize)]| {
            let doc = Document::create(base.clone()).unwrap();
            doc.edit(|tx| {
                for action in actions {
                    edit(tx, *action, true)?;
                }
                Ok(())
            })
            .unwrap()
            .map(|result| result.change)
            .unwrap_or_default()
        };
        let (left, right) = (branch(&left), branch(&right));
        let left_value = apply(&base, &left).unwrap();
        let right_value = apply(&base, &right).unwrap();
        for priority in [Priority::Left, Priority::Right] {
            match transform(&base, &left, &right, priority) {
                Ok((left_after_right, right_after_left)) => {
                    let merged = apply(&right_value, &left_after_right).unwrap();
                    prop_assert_eq!(&merged, &apply(&left_value, &right_after_left).unwrap());
                    prop_assert_eq!(Value::decode(&merged.encode()).unwrap(), merged);
                }
                Err(error) => prop_assert!(
                    matches!(error.code, ErrorCode::IncompatibleChange | ErrorCode::StructuralConflict),
                    "unexpected transform error: {error:?}"
                ),
            }
        }
    }
}
