use colla::{
    apply, compose, invert, transform, Body, Change, Document, ErrorCode, Operation, Path,
    Priority, Segment, Value,
};
use colla::{TextChange, TextOp};

fn change(operations: impl IntoIterator<Item = Operation>) -> Change {
    Change::new(operations).unwrap()
}
fn key(name: &str) -> Segment {
    Segment::Key(name.into())
}
fn at(index: usize) -> Segment {
    Segment::Index(index)
}
fn map(values: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::map(values.into_iter().map(|(k, v)| (k.into(), v))).unwrap()
}
fn ints(values: impl IntoIterator<Item = i64>) -> Value {
    Value::list(values.into_iter().map(Value::int).collect()).unwrap()
}
fn insert_text(path: Path, at: usize, text: &str) -> Operation {
    Operation::Text {
        path,
        change: TextChange::from_ops([TextOp::Retain(at), TextOp::Insert(text.into())]).unwrap(),
    }
}
/// Checks TP1 for both priorities and returns the merged content for `priority`.
fn merge(base: &Value, left: &Change, right: &Change, priority: Priority) -> Value {
    let mut result = None;
    for each in [Priority::Left, Priority::Right] {
        let (l, r) = transform(base, left, right, each).unwrap_or_else(|error| {
            panic!("{error}\npriority: {each:?}\nbase: {base:?}\nleft: {left:?}\nright: {right:?}")
        });
        let merged = apply(&apply(base, left).unwrap(), &r).unwrap();
        assert_eq!(
            merged,
            apply(&apply(base, right).unwrap(), &l).unwrap(),
            "priority: {each:?}\nbase: {base:?}\nleft: {left:?}\nright: {right:?}"
        );
        if each == priority {
            result = Some(merged);
        }
    }
    result.unwrap()
}

#[test]
fn list_move_carries_concurrent_nested_edits() {
    let root = map([(
        "items",
        Value::list(vec![
            map([("title", Value::text("a").unwrap())]),
            map([("title", Value::text("b").unwrap())]),
        ])
        .unwrap(),
    )]);
    let movement = change([Operation::ListMove {
        path: vec![key("items")],
        from: 0,
        to: 1,
    }]);
    let edit = change([insert_text(vec![key("items"), at(0), key("title")], 1, "!")]);
    let merged = merge(&root, &movement, &edit, Priority::Left);
    assert_eq!(
        merged
            .get(&[key("items"), at(1), key("title")])
            .unwrap()
            .body(),
        &Body::Text("a!".into())
    );
    let (_, edit_after) = transform(&root, &movement, &edit, Priority::Left).unwrap();
    let sequence = compose(&root, &movement, &edit_after).unwrap();
    assert_eq!(apply(&root, &sequence).unwrap(), merged);
    assert_eq!(
        apply(&merged, &invert(&root, &sequence).unwrap()).unwrap(),
        root
    );
}

#[test]
fn delete_and_set_discard_concurrent_inner_edits() {
    let root = map([(
        "doc",
        map([("n", Value::int(1)), ("t", Value::text("x").unwrap())]),
    )]);
    let inner = change([
        Operation::Add {
            path: vec![key("doc"), key("n")],
            delta: 5,
        },
        insert_text(vec![key("doc"), key("t")], 0, "y"),
    ]);
    let deletion = change([Operation::Delete {
        path: vec![key("doc")],
    }]);
    for priority in [Priority::Left, Priority::Right] {
        assert!(!merge(&root, &inner, &deletion, priority).has(&[key("doc")]));
    }
    let replacement = map([("fresh", Value::null())]);
    let set = change([Operation::Set {
        path: vec![key("doc")],
        value: replacement.clone(),
    }]);
    for priority in [Priority::Left, Priority::Right] {
        assert_eq!(
            merge(&root, &inner, &set, priority)
                .get(&[key("doc")])
                .unwrap(),
            &replacement
        );
    }
}

#[test]
fn delete_wins_over_set_and_sets_follow_priority() {
    let root = map([("n", Value::int(1))]);
    let set = |n| {
        change([Operation::Set {
            path: vec![key("n")],
            value: Value::int(n),
        }])
    };
    let deletion = change([Operation::Delete {
        path: vec![key("n")],
    }]);
    for priority in [Priority::Left, Priority::Right] {
        assert!(!merge(&root, &set(2), &deletion, priority).has(&[key("n")]));
    }
    assert_eq!(
        merge(&root, &set(2), &set(3), Priority::Left)
            .get(&[key("n")])
            .unwrap(),
        &Value::int(2)
    );
    assert_eq!(
        merge(&root, &set(2), &set(3), Priority::Right)
            .get(&[key("n")])
            .unwrap(),
        &Value::int(3)
    );
    let add = change([Operation::Add {
        path: vec![key("n")],
        delta: 1,
    }]);
    assert_eq!(
        merge(&root, &add, &set(5), Priority::Left)
            .get(&[key("n")])
            .unwrap(),
        &Value::int(5)
    );
    assert_eq!(
        merge(&root, &add, &add, Priority::Left)
            .get(&[key("n")])
            .unwrap(),
        &Value::int(3)
    );
}

#[test]
fn competing_map_insertions_follow_priority() {
    let root = map([]);
    let insert = |n| {
        change([Operation::Insert {
            path: vec![key("k")],
            value: Value::int(n),
        }])
    };
    assert_eq!(
        merge(&root, &insert(1), &insert(2), Priority::Left)
            .get(&[key("k")])
            .unwrap(),
        &Value::int(1)
    );
    assert_eq!(
        merge(&root, &insert(1), &insert(2), Priority::Right)
            .get(&[key("k")])
            .unwrap(),
        &Value::int(2)
    );
}

#[test]
fn competing_list_insertions_and_moves_follow_priority() {
    let root = ints([0, 1, 2]);
    let insert = |n| {
        change([Operation::Insert {
            path: vec![at(1)],
            value: Value::int(n),
        }])
    };
    assert_eq!(
        merge(&root, &insert(8), &insert(9), Priority::Left),
        ints([0, 8, 9, 1, 2])
    );
    assert_eq!(
        merge(&root, &insert(8), &insert(9), Priority::Right),
        ints([0, 9, 8, 1, 2])
    );
    let movement = |to| {
        change([Operation::ListMove {
            path: vec![],
            from: 0,
            to,
        }])
    };
    assert_eq!(
        merge(&root, &movement(1), &movement(2), Priority::Left),
        ints([1, 0, 2])
    );
    assert_eq!(
        merge(&root, &movement(1), &movement(2), Priority::Right),
        ints([1, 2, 0])
    );
}

#[test]
fn deletion_cancels_a_concurrent_move_of_the_same_element() {
    let root = ints([0, 1, 2]);
    let movement = change([Operation::ListMove {
        path: vec![],
        from: 0,
        to: 2,
    }]);
    let deletion = change([Operation::Delete { path: vec![at(0)] }]);
    for priority in [Priority::Left, Priority::Right] {
        assert_eq!(merge(&root, &movement, &deletion, priority), ints([1, 2]));
    }
}

#[test]
fn scalar_algebra_and_codec_envelope_are_strict() {
    let root = map([("n", Value::int(0))]);
    let add = change([Operation::Add {
        path: vec![key("n")],
        delta: i64::MIN,
    }]);
    let result = apply(&root, &add).unwrap();
    assert_eq!(apply(&result, &invert(&root, &add).unwrap()).unwrap(), root);
    let mut bytes = root.encode();
    assert!(Change::decode(&bytes).is_err());
    bytes.push(0);
    assert!(Value::decode(&bytes).is_err());
    bytes.pop();
    for version in [0, 2] {
        bytes[5] = version;
        assert!(Value::decode(&bytes).is_err());
    }
    assert_eq!(
        Change::new([Operation::Delete { path: vec![] }])
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    assert!(change([Operation::ListMove {
        path: vec![],
        from: 1,
        to: 1
    }])
    .is_noop());
}

#[test]
fn values_compare_structurally() {
    let a = map([("x", ints([1, 2]))]);
    let b = Value::decode(&map([("x", ints([1, 2]))]).encode()).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, map([("x", ints([2, 1]))]));
}

fn random(seed: &mut u64) -> usize {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 33) as usize
}
/// Picks a random existing path whose value satisfies `accept`.
fn pick(value: &Value, seed: &mut u64, accept: &dyn Fn(&Value) -> bool) -> Option<Path> {
    let mut candidates = Vec::new();
    fn walk(value: &Value, path: &mut Path, out: &mut Vec<Path>, accept: &dyn Fn(&Value) -> bool) {
        if accept(value) {
            out.push(path.clone());
        }
        match value.body() {
            Body::Map(map) => {
                for (k, v) in map {
                    path.push(Segment::Key(k.clone()));
                    walk(v, path, out, accept);
                    path.pop();
                }
            }
            Body::List(list) => {
                for (i, v) in list.iter().enumerate() {
                    path.push(Segment::Index(i));
                    walk(v, path, out, accept);
                    path.pop();
                }
            }
            _ => {}
        }
    }
    walk(value, &mut Vec::new(), &mut candidates, accept);
    (!candidates.is_empty()).then(|| candidates.swap_remove(random(seed) % candidates.len()))
}
fn leaf(seed: &mut u64) -> Value {
    match random(seed) % 4 {
        0 => Value::int(random(seed) as i64 % 10),
        1 => Value::text("ab").unwrap(),
        2 => ints([7]),
        _ => map([("n", Value::int(1))]),
    }
}
fn random_edit(tx: &mut colla::Transaction, seed: &mut u64) -> colla::Result<()> {
    let value = tx.snapshot()?;
    let is_list = |v: &Value| matches!(v.body(), Body::List(_));
    let is_map = |v: &Value| matches!(v.body(), Body::Map(_));
    match random(seed) % 8 {
        0 => {
            let Some(path) = pick(&value, seed, &is_list) else {
                return Ok(());
            };
            let Body::List(list) = value.get(&path)?.body() else {
                unreachable!()
            };
            let index = random(seed) % (list.len() + 1);
            tx.list_replace(&path, index, 0, vec![leaf(seed)])
        }
        1 => {
            let Some(path) = pick(&value, seed, &is_map) else {
                return Ok(());
            };
            let name = ["a", "b", "c", "n"][random(seed) % 4];
            tx.set(&[path, vec![key(name)]].concat(), leaf(seed))
        }
        2 => match pick(&value, seed, &|_| true) {
            Some(path) if !path.is_empty() => tx.delete(&path),
            _ => Ok(()),
        },
        3 => {
            let Some(path) = pick(
                &value,
                seed,
                &|v| matches!(v.body(), Body::List(list) if list.len() > 1),
            ) else {
                return Ok(());
            };
            let Body::List(list) = value.get(&path)?.body() else {
                unreachable!()
            };
            let from = random(seed) % list.len();
            tx.list_move(&path, from, random(seed) % list.len())
        }
        4 => match pick(&value, seed, &|v| matches!(v.body(), Body::Int(_))) {
            Some(path) => tx.increment(&path, 1),
            None => Ok(()),
        },
        5 => match pick(&value, seed, &|v| matches!(v.body(), Body::Text(_))) {
            Some(path) => {
                let Body::Text(text) = value.get(&path)?.body() else {
                    unreachable!()
                };
                let len = text.chars().count();
                let index = random(seed) % (len + 1);
                let count = random(seed) % (len - index + 1);
                tx.text_replace(&path, index, count, "z")
            }
            None => Ok(()),
        },
        _ => match pick(&value, seed, &|_| true) {
            Some(path) if !path.is_empty() => tx.set(&path, leaf(seed)),
            _ => Ok(()),
        },
    }
}
fn branch(base: &Value, seed: &mut u64) -> Change {
    let doc = Document::create(base.clone());
    let mut combined = Change::noop();
    for _ in 0..1 + random(seed) % 4 {
        let result = doc
            .edit(|tx| {
                for _ in 0..1 + random(seed) % 3 {
                    random_edit(tx, seed)?;
                }
                Ok(())
            })
            .unwrap();
        if let Some(result) = result {
            combined = compose(base, &combined, &result.change).unwrap();
        }
    }
    let after = apply(base, &combined).unwrap();
    assert_eq!(after, doc.snapshot().unwrap());
    assert_eq!(
        apply(&after, &invert(base, &combined).unwrap()).unwrap(),
        *base
    );
    combined
}

#[test]
fn random_nested_multi_operation_changes_converge() {
    let mut seed = 78324;
    for _ in 0..3000 {
        let base = map([
            (
                "list",
                Value::list(vec![
                    ints([0, 1, 2]),
                    map([("t", Value::text("hi").unwrap())]),
                ])
                .unwrap(),
            ),
            ("text", Value::text("abc").unwrap()),
            ("n", Value::int(0)),
        ]);
        let left = branch(&base, &mut seed);
        let right = branch(&base, &mut seed);
        merge(&base, &left, &right, Priority::Left);
    }
}

#[test]
fn decode_rejects_noncanonical_text_operations() {
    let text = |retain| {
        change([Operation::Text {
            path: vec![],
            change: TextChange::from_ops([TextOp::Retain(retain), TextOp::Insert("a".into())])
                .unwrap(),
        }])
        .encode()
    };
    let (one, two) = (text(1), text(2));
    let at = (0..one.len()).find(|&i| one[i] != two[i]).unwrap();
    assert!(Change::decode(&one).is_ok());
    // Rewriting the retain length to zero yields a noncanonical `Retain(0)`.
    let mut zero = one;
    zero[at] = 0;
    let error = Change::decode(&zero).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidEncoding);
    assert!(format!("{error:?}").contains("noncanonical text operations"));
}
