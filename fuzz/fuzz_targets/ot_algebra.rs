#![no_main]
use colla::*;
use libfuzzer_sys::fuzz_target;
fn branch(base: &Value, data: &[u8]) -> Change {
    let doc = Document::create(base.clone()).unwrap();
    doc.edit(|tx| {
        for chunk in data.chunks_exact(3).take(6) {
            // Edit the root List or one of its nested Lists.
            let root = tx.snapshot()?;
            let Body::List(outer) = root.body() else {
                unreachable!()
            };
            let mut path = Vec::new();
            if chunk[2] % 3 != 0 && !outer.is_empty() {
                let index = chunk[2] as usize % outer.len();
                if matches!(outer[index].body(), Body::List(_)) {
                    path.push(Segment::Index(index));
                }
            }
            let Body::List(items) = tx.get(&path)?.body().clone() else {
                unreachable!()
            };
            let len = items.len();
            let item = |n: u8| [path.as_slice(), &[Segment::Index(n as usize % len)]].concat();
            if len == 0 || chunk[0] % 6 == 0 {
                let value = if chunk[0] % 2 == 0 {
                    Value::int(7)
                } else {
                    Value::list(vec![Value::int(8)]).unwrap()
                };
                tx.list_replace(&path, chunk[1] as usize % (len + 1), 0, vec![value])?;
            } else {
                match chunk[0] % 6 {
                    1 => tx.delete(&item(chunk[1]))?,
                    2 => tx.list_move(&path, chunk[1] as usize % len, chunk[2] as usize % len)?,
                    3 if matches!(items[chunk[1] as usize % len].body(), Body::Int(_)) => {
                        tx.increment(&item(chunk[1]), 1)?
                    }
                    _ => tx.set(&item(chunk[1]), Value::int(9))?,
                }
            }
        }
        Ok(())
    })
    .unwrap()
    .map(|v| v.change)
    .unwrap_or_default()
}
fuzz_target!(|data: &[u8]| {
    let nested = |n: i64| Value::list((n..n + 3).map(Value::int).collect()).unwrap();
    let base = Value::list(vec![nested(0), Value::int(3), nested(4), Value::int(7)]).unwrap();
    let (a, b) = data.split_at(data.len() / 2);
    let a = branch(&base, a);
    let b = branch(&base, b);
    let after = apply(&base, &a).unwrap();
    assert_eq!(apply(&after, &invert(&base, &a).unwrap()).unwrap(), base);
    assert_eq!(Change::decode(&a.encode()).unwrap(), a);
    for priority in [Priority::Left, Priority::Right] {
        let (ap, bp) = transform(&base, &a, &b, priority).unwrap();
        let merged = apply(&after, &bp).unwrap();
        assert_eq!(merged, apply(&apply(&base, &b).unwrap(), &ap).unwrap());
        assert_eq!(
            merged,
            apply(&base, &compose(&base, &a, &bp).unwrap()).unwrap()
        );
    }
});
