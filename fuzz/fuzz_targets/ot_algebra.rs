#![no_main]
use colla::*;
use libfuzzer_sys::fuzz_target;
fn branch(base: &Value, data: &[u8]) -> Change {
    let doc = Document::create(base.clone()).unwrap();
    doc.edit(|tx| {
        for chunk in data.chunks_exact(3).take(6) {
            let value = tx.snapshot()?;
            let Body::List(items) = value.body() else {
                unreachable!()
            };
            if items.is_empty() || chunk[0] % 5 == 0 {
                tx.list_replace(
                    value.id(),
                    chunk[1] as usize % (items.len() + 1),
                    0,
                    vec![Value::int(7)],
                )?;
            } else {
                let id = items[chunk[1] as usize % items.len()].id();
                match chunk[0] % 5 {
                    1 => tx.delete(id)?,
                    2 => tx.move_to(
                        id,
                        value.id(),
                        Segment::Index(chunk[2] as usize % items.len()),
                    )?,
                    3 => tx.increment(id, 1)?,
                    _ => tx.set(id, Value::int(9))?,
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
    let base = Value::list((0..4).map(Value::int).collect()).unwrap();
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
