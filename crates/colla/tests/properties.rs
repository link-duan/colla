use colla::*;
use proptest::prelude::*;

fn content() -> impl Strategy<Value = String> {
    prop::collection::vec(prop_oneof![Just('a'), Just('界'), Just('😀')], 0..12)
        .prop_map(|v| v.into_iter().collect())
}
fn branch(base: &Value, actions: &[(u8, usize, usize, String)]) -> Change {
    let doc = Document::create(base.clone()).unwrap();
    doc.edit(|tx| {
        for (kind, start, count, inserted) in actions {
            let current = tx.snapshot()?;
            match current.body() {
                Body::Text(text) => {
                    let len = text.chars().count();
                    let pos = start % (len + 1);
                    let count = count % (len - pos + 1);
                    tx.text_replace(current.id(), pos, count, inserted)?;
                }
                Body::RichText(spans) => {
                    let len: usize = spans
                        .iter()
                        .map(|s| match s {
                            RichSpan::Text { text, .. } => text.chars().count(),
                            RichSpan::Embed { .. } => 1,
                        })
                        .sum();
                    let pos = start % (len + 1);
                    let count = count % (len - pos + 1);
                    let mut ops = vec![RichOp::Retain {
                        len: pos,
                        attrs: AttrPatch::new(),
                    }];
                    if kind % 3 == 0 {
                        ops.push(RichOp::Retain {
                            len: count,
                            attrs: AttrPatch::from([(
                                "bold".into(),
                                if kind % 2 == 0 {
                                    Some(Attr::Bool(true))
                                } else {
                                    None
                                },
                            )]),
                        });
                    } else {
                        ops.push(RichOp::Delete(count));
                        ops.push(RichOp::Insert(if kind % 3 == 1 {
                            RichSpan::Text {
                                text: inserted.clone(),
                                attrs: Attrs::new(),
                            }
                        } else {
                            RichSpan::Embed {
                                value: Value::reference(current.id()),
                                attrs: Attrs::new(),
                            }
                        }));
                    }
                    tx.rich_text_edit(current.id(), ops)?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    })
    .unwrap()
    .map(|result| result.change)
    .unwrap_or_default()
}
fn laws(base: &Value, left: &Change, right: &Change) {
    let left_value = apply(base, left).unwrap();
    assert_eq!(
        apply(&left_value, &invert(base, left).unwrap()).unwrap(),
        *base
    );
    assert_eq!(Value::decode(&left_value.encode()).unwrap(), left_value);
    assert_eq!(Change::decode(&left.encode()).unwrap(), *left);
    for priority in [Priority::Left, Priority::Right] {
        let (l, r) = transform(base, left, right, priority).unwrap();
        assert_eq!(
            apply(&apply(base, right).unwrap(), &l).unwrap(),
            apply(&left_value, &r).unwrap()
        );
        assert_eq!(
            apply(base, &compose(base, left, &r).unwrap()).unwrap(),
            apply(&left_value, &r).unwrap()
        );
    }
}
proptest! {
    #[test]
    fn unicode_and_rich_text_algebra(
        text in content(),
        left in prop::collection::vec((any::<u8>(),0usize..100,0usize..100,content()),1..5),
        right in prop::collection::vec((any::<u8>(),0usize..100,0usize..100,content()),1..5),
    ) {
        for base in [Value::text(text.clone()).unwrap(),Value::rich_text(vec![RichSpan::Text{text,attrs:Attrs::new()},RichSpan::Embed{value:Value::int(1),attrs:Attrs::new()}]).unwrap()] {
            laws(&base,&branch(&base,&left),&branch(&base,&right));
        }
    }
    #[test]
    fn malformed_typed_payloads_never_panic(kind in 1u8..9, payload in prop::collection::vec(any::<u8>(),0..2048)) {
        let mut bytes=b"COLLA\x02\x00".to_vec();bytes.push(kind);bytes.extend(payload);
        match kind {
            1=>{let _=Value::decode(&bytes);},2=>{let _=Change::decode(&bytes);},
            3=>{let _=SyncSnapshot::decode(&bytes);},4=>{let _=Submission::decode(&bytes);},
            5=>{let _=ServerMessage::decode(&bytes);},6=>{let _=SessionCheckpoint::decode(&bytes);},
            7=>{let _=HistoryCheckpoint::decode(&bytes);},_=>{let _=AuthorityCheckpoint::decode(&bytes);},
        }
    }
}

#[test]
fn deep_snapshot_and_checkpoint_roundtrip_at_the_supported_limit() {
    let mut value = Value::int(1);
    for _ in 1..100 {
        value = Value::list(vec![value]).unwrap();
    }
    assert_eq!(Value::decode(&value.encode()).unwrap(), value);
    let snapshot = Authority::create("deep", value).unwrap().snapshot();
    let session = SyncSession::create("writer", snapshot).unwrap();
    let checkpoint = session.checkpoint().unwrap();
    let restored =
        SyncSession::restore(SessionCheckpoint::decode(&checkpoint.encode()).unwrap()).unwrap();
    assert_eq!(
        restored.document().snapshot().unwrap(),
        session.document().snapshot().unwrap()
    );
}
