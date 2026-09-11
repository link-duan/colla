use colla::*;
use serde_json::{json, Value as Json};
fn hex(bytes: Vec<u8>) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
fn build() -> Vec<Json> {
    let mut ids = IdAllocator::deterministic([17; 16]);
    let item = Value::with_allocator(Body::Text("A😀B".into()), &mut ids).unwrap();
    let reference = Value::with_allocator(Body::Ref(Ref { target: item.id() }), &mut ids).unwrap();
    let list = Value::with_allocator(Body::List(vec![item.clone(), reference]), &mut ids).unwrap();
    let destination = Value::with_allocator(Body::List(vec![]), &mut ids).unwrap();
    let integer = Value::with_allocator(Body::Int(-5), &mut ids).unwrap();
    let rich = Value::with_allocator(
        Body::RichText(vec![RichSpan::Text {
            text: "hello".into(),
            attrs: Attrs::new(),
        }]),
        &mut ids,
    )
    .unwrap();
    let base = Value::with_allocator(
        Body::Map(
            [
                ("from".into(), list.clone()),
                ("to".into(), destination.clone()),
                ("count".into(), integer.clone()),
                ("rich".into(), rich.clone()),
            ]
            .into(),
        ),
        &mut ids,
    )
    .unwrap();
    let movement = Change::new([
        Operation::Move {
            target: item.id(),
            destination: Destination {
                parent: destination.id(),
                slot: Segment::Index(0),
            },
        },
        Operation::Add {
            target: integer.id(),
            delta: 9,
        },
    ])
    .unwrap();
    let text = Change::new([
        Operation::Text {
            target: item.id(),
            change: TextChange::from_ops([
                TextOp::Retain(1),
                TextOp::Insert("X".into()),
                TextOp::Delete(1),
            ])
            .unwrap(),
        },
        Operation::RichText {
            target: rich.id(),
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
            Change::new([Operation::Delete { target: list.id() }]).unwrap(),
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
    session.document().apply(&movement).unwrap();
    session.document().apply(&text).unwrap();
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
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/v2.json"),
            &actual,
        )
        .unwrap();
    }
    let expected =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/v2.json"))
            .unwrap();
    assert_eq!(
        actual, expected,
        "v2 canonical bytes changed; review before updating fixtures"
    );
}
