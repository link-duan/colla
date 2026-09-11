use colla::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
fn benchmarks(c: &mut Criterion) {
    let base = Value::text("a".repeat(10000)).unwrap();
    let a = Change::new([Operation::Text {
        target: base.id(),
        change: TextChange::from_ops([TextOp::Retain(5000), TextOp::Insert("😀".into())]).unwrap(),
    }])
    .unwrap();
    let b = Change::new([Operation::Text {
        target: base.id(),
        change: TextChange::from_ops([TextOp::Retain(5001), TextOp::Delete(1)]).unwrap(),
    }])
    .unwrap();
    c.bench_function("apply text", |bench| {
        bench.iter(|| apply(black_box(&base), &a).unwrap())
    });
    c.bench_function("compose text", |bench| {
        bench.iter(|| compose(black_box(&base), &a, &b).unwrap())
    });
    c.bench_function("transform text", |bench| {
        bench.iter(|| transform(black_box(&base), &a, &b, Priority::Left).unwrap())
    });
    c.bench_function("encode", |bench| bench.iter(|| black_box(&base).encode()));
    let bytes = base.encode();
    c.bench_function("decode", |bench| {
        bench.iter(|| Value::decode(black_box(&bytes)).unwrap())
    });
    let items = Value::list((0..1000).map(Value::int).collect()).unwrap();
    let id = items.id_at(vec![Segment::Index(500)]).unwrap();
    let movement = Change::new([Operation::Move {
        target: id,
        destination: Destination {
            parent: items.id(),
            slot: Segment::Index(10),
        },
    }])
    .unwrap();
    c.bench_function("move 1000 items", |bench| {
        bench.iter(|| apply(black_box(&items), &movement).unwrap())
    });
    c.bench_function("lookup cached ID", |bench| {
        bench.iter(|| items.get(black_box(id)).unwrap())
    });
}
criterion_group!(benches, benchmarks);
criterion_main!(benches);
