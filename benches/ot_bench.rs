use colla::{Change, Limits, TextChange, TextOp, Value};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn apply_text(c: &mut Criterion) {
    let limits = Limits::default();
    let base = Value::text("a".repeat(10_000));
    let change = Change::text(TextChange::new(vec![
        TextOp::Retain(5_000),
        TextOp::Insert("x".into()),
    ]));
    c.bench_function("apply text insert", |b| {
        b.iter(|| {
            change
                .apply_to(black_box(&base), black_box(&limits))
                .unwrap()
        })
    });
}

criterion_group!(benches, apply_text);
criterion_main!(benches);
