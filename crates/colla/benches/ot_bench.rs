use colla::{
    transform_pair, AttrChange, AttrPatch, AttrValue, Change, ListChange, ListOp, RichTextChange,
    RichTextOp, TextChange, TextOp, TieBreak, Value,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn apply_text(c: &mut Criterion) {
    let base = Value::text("a".repeat(10_000));
    let change = Change::text(TextChange::new(vec![
        TextOp::Retain(5_000),
        TextOp::Insert("x".into()),
    ]));
    c.bench_function("apply text insert", |b| {
        b.iter(|| change.apply_to(black_box(&base)).unwrap())
    });
}

fn compose_large_retains(c: &mut Criterion) {
    let mut group = c.benchmark_group("compose large retains");
    for len in [10_000usize, 100_000, 900_000] {
        group.throughput(Throughput::Elements(len as u64));

        let text_left = Change::text(TextChange::new(vec![
            TextOp::Retain(len),
            TextOp::Insert("x".into()),
        ]));
        let text_right = Change::text(TextChange::new(vec![
            TextOp::Retain(len),
            TextOp::Insert("y".into()),
        ]));
        group.bench_with_input(BenchmarkId::new("text", len), &len, |b, _| {
            b.iter(|| {
                black_box(&text_left)
                    .compose(black_box(&text_right))
                    .unwrap()
            })
        });

        let list_left = Change::list(ListChange::new(vec![
            ListOp::Retain(len),
            ListOp::Insert(vec![Value::int(1)]),
        ]));
        let list_right = Change::list(ListChange::new(vec![
            ListOp::Retain(len),
            ListOp::Insert(vec![Value::int(2)]),
        ]));
        group.bench_with_input(BenchmarkId::new("list", len), &len, |b, _| {
            b.iter(|| {
                black_box(&list_left)
                    .compose(black_box(&list_right))
                    .unwrap()
            })
        });
    }
    group.finish();
}

fn transform_large_retains(c: &mut Criterion) {
    let red =
        AttrPatch::from_entries([("color", AttrChange::Set(AttrValue::string("red")))]).unwrap();
    let bold = AttrPatch::from_entries([("bold", AttrChange::Set(AttrValue::Bool(true)))]).unwrap();
    let mut group = c.benchmark_group("transform large retains");
    for len in [10_000usize, 100_000, 900_000] {
        group.throughput(Throughput::Elements(len as u64));

        let text_left = Change::text(TextChange::new(vec![
            TextOp::Retain(len),
            TextOp::Insert("x".into()),
        ]));
        let text_right = Change::text(TextChange::new(vec![
            TextOp::Retain(len),
            TextOp::Insert("y".into()),
        ]));
        group.bench_with_input(BenchmarkId::new("text", len), &len, |b, _| {
            b.iter(|| {
                transform_pair(
                    black_box(&text_left),
                    black_box(&text_right),
                    TieBreak::LeftFirst,
                )
                .unwrap()
            })
        });

        let rich_left = Change::rich_text(RichTextChange::new(vec![RichTextOp::Retain {
            len,
            attrs: red.clone(),
        }]));
        let rich_right = Change::rich_text(RichTextChange::new(vec![RichTextOp::Retain {
            len,
            attrs: bold.clone(),
        }]));
        group.bench_with_input(BenchmarkId::new("rich text attrs", len), &len, |b, _| {
            b.iter(|| {
                transform_pair(
                    black_box(&rich_left),
                    black_box(&rich_right),
                    TieBreak::LeftFirst,
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    apply_text,
    compose_large_retains,
    transform_large_retains
);
criterion_main!(benches);
