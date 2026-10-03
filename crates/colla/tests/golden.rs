use colla::*;
use serde_json::{json, Value as Json};
fn hex(bytes: Vec<u8>) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
fn build() -> Vec<Json> {
    let key = |name: &str| Segment::Key(name.into());
    let base = Value::map([
        (
            "from".into(),
            Value::list(vec![Value::text("A😀B").unwrap(), Value::int(2)]).unwrap(),
        ),
        ("to".into(), Value::list(vec![]).unwrap()),
        ("count".into(), Value::int(-5)),
        (
            "rich".into(),
            Value::rich_text(vec![RichSpan::Text {
                text: "hello".into(),
                attrs: Attrs::new(),
            }])
            .unwrap(),
        ),
    ])
    .unwrap();
    let movement = Change::new([
        Operation::ListMove {
            path: vec![key("from")],
            from: 0,
            to: 1,
        },
        Operation::Add {
            path: vec![key("count")],
            delta: 9,
        },
    ])
    .unwrap();
    let text = Change::new([
        Operation::Text {
            path: vec![key("from"), Segment::Index(0)],
            change: TextChange::from_ops([
                TextOp::Retain(1),
                TextOp::Insert("X".into()),
                TextOp::Delete(1),
            ])
            .unwrap(),
        },
        Operation::RichText {
            path: vec![key("rich")],
            operations: vec![RichOp::Retain {
                len: 5,
                attrs: AttrPatch::from([("bold".into(), Some(Attr::Bool(true)))]),
            }],
        },
    ])
    .unwrap();
    let mut fixtures = Vec::new();
    for (name, left, right) in [
        ("move-edit", movement.clone(), text.clone()),
        ("text-tie", text.clone(), text.clone()),
        (
            "delete-move",
            Change::new([Operation::Delete {
                path: vec![key("from")],
            }])
            .unwrap(),
            movement.clone(),
        ),
    ] {
        let (l, r) = transform(&base, &left, &right, Priority::Left).unwrap();
        let after = apply(&base, &left).unwrap();
        fixtures.push(json!({"name":name,"kind":"algebra","base":hex(base.encode()),"left":hex(left.encode()),"right":hex(right.encode()),"leftAfterRight":hex(l.encode()),"rightAfterLeft":hex(r.encode()),"after":hex(after.encode()),"inverse":hex(invert(&base,&left).unwrap().encode()),"composed":hex(compose(&base,&left,&r).unwrap().encode()),"merged":hex(apply(&after,&r).unwrap().encode())}));
    }
    let authority = Authority::create("fixture", base.clone()).unwrap();
    let session = SyncSession::create("writer", authority.snapshot()).unwrap();
    let history = History::attach(&session.document()).unwrap();
    session.document().apply(&text).unwrap();
    session.document().apply(&movement).unwrap();
    let request = session.outbound().unwrap().unwrap();
    let (authority, message) = authority.accept(&request).unwrap();
    for (name, kind, bytes) in [
        ("value", 1, base.encode()),
        ("change", 2, movement.encode()),
        ("snapshot", 3, authority.snapshot().encode()),
        ("submission", 4, request.encode()),
        ("commit", 5, message.encode()),
        ("session", 6, session.checkpoint().unwrap().encode()),
        ("history", 7, history.checkpoint().unwrap().encode()),
        ("authority", 8, authority.checkpoint().encode()),
    ] {
        fixtures.push(json!({"name":name,"kind":"codec","type":kind,"bytes":hex(bytes)}));
    }
    fixtures
}
#[test]
fn canonical_shared_fixtures() {
    let actual = serde_json::to_string_pretty(&build()).unwrap() + "\n";
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/fixtures.json"),
            &actual,
        )
        .unwrap();
    }
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../golden/fixtures.json"
    ))
    .unwrap();
    assert_eq!(
        actual, expected,
        "Canonical bytes changed; review before updating fixtures"
    );
}
