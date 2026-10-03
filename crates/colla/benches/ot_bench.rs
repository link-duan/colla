use colla::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
fn benchmarks(c: &mut Criterion) {
    let base = Value::text("a".repeat(10000)).unwrap();
    let a = Change::new([Operation::Text {
        path: vec![],
        change: TextChange::from_ops([TextOp::Retain(5000), TextOp::Insert("😀".into())]).unwrap(),
    }])
    .unwrap();
    let b = Change::new([Operation::Text {
        path: vec![],
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
    let movement = Change::new([Operation::ListMove {
        path: vec![],
        from: 500,
        to: 10,
    }])
    .unwrap();
    c.bench_function("move 1000 items", |bench| {
        bench.iter(|| apply(black_box(&items), &movement).unwrap())
    });
    // 10 levels with 3 children each: 88,573 nodes.
    let nested = (0..10).fold(Value::int(0), |value, _| {
        Value::list((0..3).map(|_| value.clone()).collect()).unwrap()
    });
    let path = vec![Segment::Index(1); 10];
    let increment = Change::new([Operation::Add { path, delta: 1 }]).unwrap();
    c.bench_function("edit deep leaf", |bench| {
        bench.iter(|| apply(black_box(&nested), &increment).unwrap())
    });
}
criterion_group!(benches, benchmarks);
criterion_main!(benches);
