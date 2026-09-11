use colla::{
    Authority, AuthorityCheckpoint, Body, Change, Document, ErrorCode, History, HistoryCheckpoint,
    Operation, Result, SessionCheckpoint, SyncSession, Value,
};

const MIB: usize = 1024 * 1024;

#[test]
fn string_and_text_constructors_enforce_utf8_byte_limits() -> Result<()> {
    let constructors: [fn(String) -> Result<Value>; 2] = [Value::string, Value::text];
    for create in constructors {
        let text = "😀".repeat(4 * MIB);
        let value = create(text.clone())?;
        value.validate()?;
        assert!(Value::decode(&value.encode())? == value);
        assert_eq!(
            create(text + "x").unwrap_err().code,
            ErrorCode::LimitExceeded
        );
    }
    Ok(())
}

#[test]
fn content_and_changes_larger_than_64_mib_roundtrip() -> Result<()> {
    let values = (0..4)
        .map(|_| Value::string("x".repeat(16 * MIB)))
        .collect::<Result<Vec<_>>>()?;
    let value = Value::list(values)?;
    let bytes = value.encode();
    assert!(bytes.len() > 64 * MIB);
    assert!(Value::decode(&bytes)? == value);
    let change = Change::new([Operation::Set {
        target: value.id(),
        value: value.clone(),
    }])?;
    assert!(Change::decode(&change.encode())? == change);
    let snapshot = Authority::create("size", value)?.snapshot();
    assert!(colla::SyncSnapshot::decode(&snapshot.encode())? == snapshot);
    Ok(())
}

#[test]
fn pending_growth_beyond_64_mib_preserves_restart_and_retry() -> Result<()> {
    let count = Value::int(0);
    let id = count.id();
    let value = Value::list(vec![
        Value::string("x".repeat(12 * MIB))?,
        Value::string("y".repeat(12 * MIB))?,
        count,
    ])?;
    let authority = Authority::create("size", value)?;
    let session = SyncSession::create("a", authority.snapshot())?;
    assert!(session.checkpoint()?.encode().len() < 64 * MIB);
    session.document().edit(|tx| tx.increment(id, 1))?;
    let pending = session.outbound()?.unwrap().encode();
    let saved = session.checkpoint()?.encode();
    assert!(saved.len() > 64 * MIB);
    session.close()?;
    let restored = SyncSession::restore(SessionCheckpoint::decode(&saved)?)?;
    assert!(restored.outbound()?.unwrap().encode() == pending);
    assert_eq!(restored.document().get(id)?.body(), &Body::Int(1));
    let (authority, message) = authority.accept(&restored.outbound()?.unwrap())?;
    restored.receive(&message)?;
    assert!(restored.document().snapshot()? == *authority.snapshot().value());
    assert!(restored.outbound()?.is_none());
    restored.close()?;
    Ok(())
}

#[test]
fn history_and_authority_checkpoints_larger_than_64_mib_restore() -> Result<()> {
    let doc = Document::create(Value::string("x".repeat(4 * MIB))?)?;
    let id = doc.snapshot()?.id();
    let history = History::attach(&doc)?;
    for n in 0..17 {
        let next = Value::string(if n % 2 == 0 { "y" } else { "z" }.repeat(4 * MIB))?;
        doc.edit(|tx| tx.set(id, next))?;
    }
    let content = doc.snapshot()?;
    let saved = history.checkpoint()?.encode();
    assert!(saved.len() > 64 * MIB);
    doc.close()?;
    let restored = Document::create(content)?;
    let history = History::restore(&restored, HistoryCheckpoint::decode(&saved)?)?;
    history.undo()?;
    assert!(restored.get(id)?.body() == &Body::String("z".repeat(4 * MIB)));
    restored.close()?;

    let mut authority = Authority::create("size", Value::string("x".repeat(4 * MIB))?)?;
    let session = SyncSession::create("a", authority.snapshot())?;
    let id = session.document().snapshot()?.id();
    let mut last_request = None;
    for n in 0..9 {
        let next = Value::string(if n % 2 == 0 { "y" } else { "z" }.repeat(4 * MIB))?;
        session.document().edit(|tx| tx.set(id, next))?;
        let request = session.outbound()?.unwrap();
        let (next, message) = authority.accept(&request)?;
        session.receive(&message)?;
        authority = next;
        last_request = Some(request);
    }
    let saved = authority.checkpoint().encode();
    assert!(saved.len() > 64 * MIB);
    let restored = Authority::restore(AuthorityCheckpoint::decode(&saved)?)?;
    assert!(restored.snapshot() == authority.snapshot());
    // Deduplication receipts still identify retries after a large checkpoint restore.
    let (duplicate, _) = restored.accept(&last_request.unwrap())?;
    assert_eq!(duplicate.revision(), authority.revision());
    session.close()?;
    Ok(())
}
