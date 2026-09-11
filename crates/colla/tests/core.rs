use colla::{
    apply, compose, invert, transform, Body, Change, Destination, ErrorCode, IdAllocator,
    Operation, Priority, Ref, Segment, Value,
};
use colla::{TextChange, TextOp};

fn change(op: Operation) -> Change {
    Change::new([op]).unwrap()
}
fn key(parent: &Value, name: &str) -> Destination {
    Destination {
        parent: parent.id(),
        slot: Segment::Key(name.into()),
    }
}
fn index(parent: &Value, index: usize) -> Destination {
    Destination {
        parent: parent.id(),
        slot: Segment::Index(index),
    }
}
fn map(values: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::map(values.into_iter().map(|(k, v)| (k.into(), v))).unwrap()
}
fn tp1(base: &Value, left: &Change, right: &Change) -> Value {
    for priority in [Priority::Left, Priority::Right] {
        let (l, r) = transform(base, left, right, priority).unwrap_or_else(|error| {
            panic!(
                "{error}\npriority: {priority:?}\nbase: {base:?}\nleft: {left:?}\nright: {right:?}"
            )
        });
        assert_eq!(
            apply(&apply(base, left).unwrap(), &r).unwrap(),
            apply(&apply(base, right).unwrap(), &l).unwrap()
        );
    }
    let (_, r) = transform(base, left, right, Priority::Left).unwrap();
    apply(&apply(base, left).unwrap(), &r).unwrap()
}

#[test]
fn moves_preserve_descendants_references_and_inverse() {
    let title = Value::text("draft").unwrap();
    let block = map([("title", title.clone())]);
    let active = Value::list(vec![block.clone()]).unwrap();
    let archive = Value::list(vec![]).unwrap();
    let root = map([
        ("active", active),
        ("archive", archive.clone()),
        ("featured", Value::reference(block.id())),
    ]);
    let movement = change(Operation::Move {
        target: block.id(),
        destination: index(&archive, 0),
    });
    let edit = change(Operation::Text {
        target: title.id(),
        change: TextChange::from_ops([TextOp::Retain(5), TextOp::Insert("!".into())]).unwrap(),
    });
    let result = tp1(&root, &movement, &edit);
    assert_eq!(
        result
            .resolve(Ref { target: block.id() })
            .unwrap()
            .get(title.id())
            .unwrap()
            .body(),
        &Body::Text("draft!".into())
    );
    assert_eq!(
        apply(
            &apply(&root, &movement).unwrap(),
            &invert(&root, &movement).unwrap()
        )
        .unwrap(),
        root
    );
    let sequence = compose(&root, &movement, &edit).unwrap();
    assert_eq!(apply(&root, &sequence).unwrap(), result);
    assert_eq!(
        apply(&result, &invert(&root, &sequence).unwrap()).unwrap(),
        root
    );
}

#[test]
fn copy_remaps_internal_refs_and_preserves_external_refs() {
    let target = Value::int(7);
    let outside = Value::null();
    let source = map([
        ("target", target.clone()),
        ("inside", Value::reference(target.id())),
        ("outside", Value::reference(outside.id())),
    ]);
    let copy = source.copied().unwrap();
    let Body::Map(children) = copy.body() else {
        panic!()
    };
    assert_ne!(source.id(), copy.id());
    assert_ne!(children["target"].id(), target.id());
    assert_eq!(
        children["inside"].body(),
        &Body::Ref(Ref {
            target: children["target"].id()
        })
    );
    assert_eq!(
        children["outside"].body(),
        &Body::Ref(Ref {
            target: outside.id()
        })
    );
    assert_eq!(Value::decode(&copy.encode()).unwrap(), copy);
}

#[test]
fn deleted_baseline_ancestor_wins_over_escape() {
    let child = Value::text("kept by identity").unwrap();
    let parent = map([("child", child.clone())]);
    let root = map([("parent", parent.clone())]);
    let escape = change(Operation::Move {
        target: child.id(),
        destination: key(&root, "escaped"),
    });
    let deletion = change(Operation::Delete {
        target: parent.id(),
    });
    let result = tp1(&root, &escape, &deletion);
    assert!(!result.has(child.id()));
    assert!(!result.has(parent.id()));
}

#[test]
fn move_competition_preserves_other_edits() {
    let child = Value::int(1);
    let a = Value::list(vec![child.clone()]).unwrap();
    let b = Value::list(vec![]).unwrap();
    let c = Value::list(vec![]).unwrap();
    let root = map([("a", a), ("b", b.clone()), ("c", c.clone())]);
    let left = change(Operation::Move {
        target: child.id(),
        destination: index(&b, 0),
    });
    let right = Change::new([
        Operation::Move {
            target: child.id(),
            destination: index(&c, 0),
        },
        Operation::Add {
            target: child.id(),
            delta: 2,
        },
    ])
    .unwrap();
    let result = tp1(&root, &left, &right);
    assert_eq!(result.get(child.id()).unwrap().body(), &Body::Int(3));
    assert_eq!(
        result.get(b.id()).unwrap().body(),
        &Body::List(vec![result.get(child.id()).unwrap()])
    );
}

#[test]
fn cycles_and_map_collisions_are_atomic_conflicts() {
    let a = Value::list(vec![]).unwrap();
    let b = Value::list(vec![]).unwrap();
    let root = map([("a", a.clone()), ("b", b.clone())]);
    let left = change(Operation::Move {
        target: a.id(),
        destination: index(&b, 0),
    });
    let right = change(Operation::Move {
        target: b.id(),
        destination: index(&a, 0),
    });
    assert_eq!(
        transform(&root, &left, &right, Priority::Left)
            .unwrap_err()
            .code,
        ErrorCode::StructuralConflict
    );
    let left = change(Operation::Move {
        target: a.id(),
        destination: key(&root, "same"),
    });
    let right = change(Operation::Move {
        target: b.id(),
        destination: key(&root, "same"),
    });
    assert_eq!(
        transform(&root, &left, &right, Priority::Right)
            .unwrap_err()
            .code,
        ErrorCode::StructuralConflict
    );
    assert_eq!(root.get(a.id()).unwrap(), a);
}

#[test]
fn scalar_algebra_and_codec_envelope_are_strict() {
    let root = Value::int(0);
    let add = change(Operation::Add {
        target: root.id(),
        delta: i64::MIN,
    });
    let result = apply(&root, &add).unwrap();
    assert_eq!(apply(&result, &invert(&root, &add).unwrap()).unwrap(), root);
    let mut bytes = root.encode();
    assert!(Change::decode(&bytes).is_err());
    bytes.push(0);
    assert!(Value::decode(&bytes).is_err());
    bytes.pop();
    bytes[5] = 1;
    assert!(Value::decode(&bytes).is_err());
    let mut allocator = IdAllocator::deterministic([3; 16]);
    assert_eq!(allocator.allocate().unwrap().sequence(), 1);
    assert_eq!(allocator.allocate().unwrap().sequence(), 2);
    assert!(Value::list(vec![root.clone(), root]).is_err());
}

#[test]
fn all_pairs_of_single_list_moves_converge() {
    let values: Vec<_> = (0..5).map(Value::int).collect();
    let root = Value::list(values.clone()).unwrap();
    for a in &values {
        for ai in 0..5 {
            for b in &values {
                for bi in 0..5 {
                    tp1(
                        &root,
                        &change(Operation::Move {
                            target: a.id(),
                            destination: index(&root, ai),
                        }),
                        &change(Operation::Move {
                            target: b.id(),
                            destination: index(&root, bi),
                        }),
                    );
                }
            }
        }
    }
}

#[test]
fn multi_step_map_permutation_merges_with_descendant_edit() {
    let a = Value::int(1);
    let b = Value::int(2);
    let root = map([("a", a.clone()), ("b", b.clone())]);
    let swap = Change::new([
        Operation::Move {
            target: a.id(),
            destination: key(&root, "temporary"),
        },
        Operation::Move {
            target: b.id(),
            destination: key(&root, "a"),
        },
        Operation::Move {
            target: a.id(),
            destination: key(&root, "b"),
        },
    ])
    .unwrap();
    let edit = change(Operation::Add {
        target: a.id(),
        delta: 3,
    });
    let result = tp1(&root, &swap, &edit);
    assert_eq!(result.get(a.id()).unwrap().body(), &Body::Int(4));
    assert_eq!(
        result.path_of(a.id()).unwrap(),
        vec![Segment::Key("b".into())]
    );
}

#[test]
fn transformed_moves_retain_the_original_source_association() {
    let a = Value::int(0);
    let b = Value::int(1);
    let root = Value::list(vec![a.clone(), b.clone(), Value::int(2)]).unwrap();
    let left = change(Operation::Move {
        target: a.id(),
        destination: index(&root, 2),
    });
    let right = change(Operation::Move {
        target: b.id(),
        destination: index(&root, 2),
    });
    let (left, right) = transform(&root, &left, &right, Priority::Right).unwrap();
    for op in left.operations() {
        assert!(matches!(op,Operation::Move {target,..} if *target==a.id()));
    }
    for op in right.operations() {
        assert!(matches!(op,Operation::Move {target,..} if *target==b.id()));
    }
}

#[test]
fn container_set_preserves_concurrently_incoming_elements() {
    let old = Value::int(1);
    let incoming = Value::int(2);
    let parent = Value::list(vec![old]).unwrap();
    let root = map([("parent", parent.clone()), ("incoming", incoming.clone())]);
    let movement = change(Operation::Move {
        target: incoming.id(),
        destination: index(&parent, 1),
    });
    let doc = colla::Document::create(root.clone()).unwrap();
    let set = doc
        .edit(|tx| tx.set(parent.id(), Value::list(vec![Value::int(3)]).unwrap()))
        .unwrap()
        .unwrap()
        .change;
    let result = tp1(&root, &movement, &set);
    assert_eq!(result.get(incoming.id()).unwrap().body(), &Body::Int(2));
    let deletion = change(Operation::Delete {
        target: parent.id(),
    });
    for priority in [Priority::Left, Priority::Right] {
        assert_eq!(
            transform(&root, &movement, &deletion, priority)
                .unwrap_err()
                .code,
            ErrorCode::StructuralConflict
        );
    }
}

#[test]
fn mixed_multi_operation_list_changes_converge() {
    let mut seed = 87234u64;
    fn random(seed: &mut u64) -> usize {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (*seed >> 32) as usize
    }
    fn branch(base: &Value, seed: &mut u64) -> Change {
        let doc = colla::Document::create(base.clone()).unwrap();
        doc.edit(|tx| {
            for _ in 0..4 {
                let value = tx.snapshot()?;
                let Body::List(items) = value.body() else {
                    panic!()
                };
                let operation = random(seed) % 5;
                if items.is_empty() || operation == 0 {
                    tx.list_replace(
                        value.id(),
                        random(seed) % (items.len() + 1),
                        0,
                        vec![Value::int(7)],
                    )?;
                } else {
                    let source = random(seed) % items.len();
                    match operation {
                        1 => tx.delete(items[source].id())?,
                        2 => tx.move_to(
                            items[source].id(),
                            value.id(),
                            Segment::Index(random(seed) % items.len()),
                        )?,
                        3 => tx.increment(items[source].id(), 1)?,
                        _ => tx.set(items[source].id(), Value::int(99))?,
                    }
                }
            }
            Ok(())
        })
        .unwrap()
        .map(|edit| edit.change)
        .unwrap_or_default()
    }
    for _ in 0..1000 {
        let base = Value::list((0..5).map(Value::int).collect()).unwrap();
        let left = branch(&base, &mut seed);
        let right = branch(&base, &mut seed);
        tp1(&base, &left, &right);
    }
}

#[test]
fn mixed_cross_parent_changes_compose_invert_and_converge() {
    fn random(seed: &mut u64) -> usize {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (*seed >> 32) as usize
    }
    fn branch(base: &Value, seed: &mut u64) -> Change {
        let doc = colla::Document::create(base.clone()).unwrap();
        let mut combined = Change::noop();
        for _ in 0..5 {
            let result = doc
                .edit(|tx| {
                    let parent = tx.get(vec![Segment::Index(random(seed) % 3)])?;
                    let Body::List(items) = parent.body() else {
                        panic!()
                    };
                    let action = random(seed) % 5;
                    if items.is_empty() || action == 0 {
                        tx.list_replace(
                            parent.id(),
                            random(seed) % (items.len() + 1),
                            0,
                            vec![Value::int(6)],
                        )?;
                    } else {
                        let item = &items[random(seed) % items.len()];
                        match action {
                            1 => tx.delete(item.id())?,
                            2 => {
                                let destination = tx.get(vec![Segment::Index(random(seed) % 3)])?;
                                let Body::List(list) = destination.body() else {
                                    panic!()
                                };
                                let size =
                                    list.len() + usize::from(destination.id() != parent.id());
                                tx.move_to(
                                    item.id(),
                                    destination.id(),
                                    Segment::Index(random(seed) % size),
                                )?;
                            }
                            3 => tx.increment(item.id(), 1)?,
                            _ => tx.set(item.id(), Value::int(80))?,
                        }
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
    let mut seed = 78324;
    for _ in 0..1000 {
        let base = Value::list(
            (0..3)
                .map(|_| Value::list((0..3).map(Value::int).collect()).unwrap())
                .collect(),
        )
        .unwrap();
        let left = branch(&base, &mut seed);
        let right = branch(&base, &mut seed);
        tp1(&base, &left, &right);
    }
}

#[test]
fn concurrent_ancestor_and_descendant_moves_preserve_each_intent() {
    let child = Value::text("child").unwrap();
    let ancestor = map([("child", child.clone())]);
    let destination = Value::list(vec![]).unwrap();
    let root = map([
        ("ancestor", ancestor.clone()),
        ("destination", destination.clone()),
    ]);
    let left = change(Operation::Move {
        target: ancestor.id(),
        destination: index(&destination, 0),
    });
    let right = change(Operation::Move {
        target: child.id(),
        destination: key(&root, "escaped"),
    });
    let merged = tp1(&root, &left, &right);
    assert_eq!(
        merged.get(ancestor.id()).unwrap().body(),
        &Body::Map(Default::default())
    );
    assert_eq!(
        merged.path_of(child.id()).unwrap(),
        vec![Segment::Key("escaped".into())]
    );
}
