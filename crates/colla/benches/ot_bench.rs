use colla::{
    transform_pair, AttrChange, AttrPatch, AttrValue, Attrs, Change, ListChange, ListOp,
    RichContent, RichSpan, RichText, RichTextChange, RichTextOp, TextChange, TextOp, TieBreak,
    Value,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn apply_text(c: &mut Criterion) {
    let base = Value::text("a".repeat(10_000));
    let change = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(5_000), TextOp::Insert("x".into())]).unwrap(),
    );
    c.bench_function("apply text insert", |b| {
        b.iter(|| change.apply_to(black_box(&base)).unwrap())
    });
}

fn compose_large_retains(c: &mut Criterion) {
    let mut group = c.benchmark_group("compose large retains");
    for len in [10_000usize, 100_000, 900_000] {
        group.throughput(Throughput::Elements(len as u64));

        let text_left = Change::from(
            TextChange::from_ops(vec![TextOp::Retain(len), TextOp::Insert("x".into())]).unwrap(),
        );
        let text_right = Change::from(
            TextChange::from_ops(vec![TextOp::Retain(len), TextOp::Insert("y".into())]).unwrap(),
        );
        group.bench_with_input(BenchmarkId::new("text", len), &len, |b, _| {
            b.iter(|| {
                black_box(&text_left)
                    .compose(black_box(&text_right))
                    .unwrap()
            })
        });

        let list_left = Change::from(
            ListChange::from_ops(vec![
                ListOp::Retain(len),
                ListOp::Insert(vec![Value::int(1)]),
            ])
            .unwrap(),
        );
        let list_right = Change::from(
            ListChange::from_ops(vec![
                ListOp::Retain(len),
                ListOp::Insert(vec![Value::int(2)]),
            ])
            .unwrap(),
        );
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

        let text_left = Change::from(
            TextChange::from_ops(vec![TextOp::Retain(len), TextOp::Insert("x".into())]).unwrap(),
        );
        let text_right = Change::from(
            TextChange::from_ops(vec![TextOp::Retain(len), TextOp::Insert("y".into())]).unwrap(),
        );
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

        let rich_left = Change::from(
            RichTextChange::from_ops(vec![RichTextOp::Retain {
                len,
                attrs: red.clone(),
            }])
            .unwrap(),
        );
        let rich_right = Change::from(
            RichTextChange::from_ops(vec![RichTextOp::Retain {
                len,
                attrs: bold.clone(),
            }])
            .unwrap(),
        );
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

fn apply_large_rich_text(c: &mut Criterion) {
    let base = Value::rich_text(
        RichText::from_spans(vec![RichSpan::text("a".repeat(1_000_000), Attrs::new())])
            .expect("benchmark RichText is valid"),
    );
    let insert = Change::from(
        RichTextChange::from_ops(vec![
            RichTextOp::Retain {
                len: 500_000,
                attrs: AttrPatch::new(),
            },
            RichTextOp::Insert {
                content: RichContent::text("x"),
                attrs: Attrs::new(),
            },
        ])
        .unwrap(),
    );
    let format = Change::from(
        RichTextChange::from_ops(vec![
            RichTextOp::Retain {
                len: 499_900,
                attrs: AttrPatch::new(),
            },
            RichTextOp::Retain {
                len: 200,
                attrs: AttrPatch::from_entries([("bold", AttrChange::Set(AttrValue::Bool(true)))])
                    .unwrap(),
            },
        ])
        .unwrap(),
    );

    let mut group = c.benchmark_group("apply million scalar rich text");
    group.throughput(Throughput::Elements(1_000_000));
    group.bench_function("localized insert", |b| {
        b.iter(|| insert.apply_to(black_box(&base)).unwrap())
    });
    group.bench_function("localized format", |b| {
        b.iter(|| format.apply_to(black_box(&base)).unwrap())
    });
    group.finish();
}

fn apply_many_rich_text_spans(c: &mut Criterion) {
    let marked = Attrs::from_entries([("marked", AttrValue::Bool(true))]).unwrap();
    let spans = (0..100_000)
        .map(|index| {
            RichSpan::text(
                "x",
                if index % 2 == 0 {
                    Attrs::new()
                } else {
                    marked.clone()
                },
            )
        })
        .collect();
    let base = Value::rich_text(RichText::from_spans(spans).expect("benchmark RichText is valid"));
    let change = Change::from(
        RichTextChange::from_ops(vec![
            RichTextOp::Retain {
                len: 25_000,
                attrs: AttrPatch::new(),
            },
            RichTextOp::Delete(50_000),
        ])
        .unwrap(),
    );

    c.bench_function("apply rich text delete across 100k spans", |b| {
        b.iter(|| change.apply_to(black_box(&base)).unwrap())
    });
}

fn invert_mixed_rich_text(c: &mut Criterion) {
    let mut spans = Vec::with_capacity(20_000);
    for index in 0..10_000 {
        spans.push(RichSpan::text("ab", Attrs::new()));
        spans.push(RichSpan::embed(Value::int(index), Attrs::new()));
    }
    let rich = RichText::from_spans(spans).expect("benchmark RichText is valid");
    let len = rich.len();
    let base = Value::rich_text(rich);
    let change = Change::from(RichTextChange::from_ops(vec![RichTextOp::Delete(len)]).unwrap());

    c.bench_function("invert mixed rich text delete", |b| {
        b.iter(|| change.invert(black_box(&base)).unwrap())
    });
}

fn convert_rich_text_coordinates(c: &mut Criterion) {
    let marked = Attrs::from_entries([("marked", AttrValue::Bool(true))]).unwrap();
    let spans = (0..100_000)
        .map(|index| {
            RichSpan::text(
                "😀",
                if index % 2 == 0 {
                    Attrs::new()
                } else {
                    marked.clone()
                },
            )
        })
        .collect();
    let rich = RichText::from_spans(spans).expect("benchmark RichText is valid");

    let mut group = c.benchmark_group("convert rich text coordinates across 100k spans");
    group.bench_function("code point to UTF-16 near end", |b| {
        b.iter(|| rich.code_point_to_utf16(black_box(99_999)).unwrap())
    });
    group.bench_function("UTF-16 to code point near end", |b| {
        b.iter(|| rich.utf16_to_code_point(black_box(199_998)).unwrap())
    });
    group.finish();
}

fn compose_split_unicode_insert(c: &mut Criterion) {
    let repetitions = 20_000usize;
    let text = "a😀".repeat(repetitions);
    let first = Change::from(
        RichTextChange::from_ops(vec![RichTextOp::Insert {
            content: RichContent::text(text),
            attrs: Attrs::new(),
        }])
        .unwrap(),
    );
    let red =
        AttrPatch::from_entries([("color", AttrChange::Set(AttrValue::string("red")))]).unwrap();
    let bold = AttrPatch::from_entries([("bold", AttrChange::Set(AttrValue::Bool(true)))]).unwrap();
    let second = Change::from(
        RichTextChange::from_ops(
            (0..repetitions * 2)
                .map(|index| RichTextOp::Retain {
                    len: 1,
                    attrs: if index % 2 == 0 {
                        red.clone()
                    } else {
                        bold.clone()
                    },
                })
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    );

    c.bench_function("compose repeatedly split unicode insert", |b| {
        b.iter(|| black_box(&first).compose(black_box(&second)).unwrap())
    });
}

fn full_span_rich_text_format(c: &mut Criterion) {
    let base = Value::rich_text(
        RichText::from_spans(vec![RichSpan::text("a".repeat(1_000_000), Attrs::new())])
            .expect("benchmark RichText is valid"),
    );
    let change = Change::from(
        RichTextChange::from_ops(vec![RichTextOp::Retain {
            len: 1_000_000,
            attrs: AttrPatch::from_entries([("bold", AttrChange::Set(AttrValue::Bool(true)))])
                .unwrap(),
        }])
        .unwrap(),
    );

    let mut group = c.benchmark_group("full span million scalar rich text format");
    group.throughput(Throughput::Elements(1_000_000));
    group.bench_function("apply", |b| {
        b.iter(|| change.apply_to(black_box(&base)).unwrap())
    });
    group.bench_function("invert", |b| {
        b.iter(|| change.invert(black_box(&base)).unwrap())
    });
    group.finish();
}

fn decode_many_rich_text_spans(c: &mut Criterion) {
    let marked = Attrs::from_entries([("marked", AttrValue::Bool(true))]).unwrap();
    let spans = (0..100_000)
        .map(|index| {
            RichSpan::text(
                "x",
                if index % 2 == 0 {
                    Attrs::new()
                } else {
                    marked.clone()
                },
            )
        })
        .collect();
    let encoded =
        Value::rich_text(RichText::from_spans(spans).expect("benchmark RichText is valid"))
            .encode();

    c.bench_function("decode rich text snapshot with 100k spans", |b| {
        b.iter(|| Value::decode(black_box(&encoded)).unwrap())
    });
}

criterion_group!(
    benches,
    apply_text,
    compose_large_retains,
    transform_large_retains,
    apply_large_rich_text,
    apply_many_rich_text_spans,
    invert_mixed_rich_text,
    convert_rich_text_coordinates,
    compose_split_unicode_insert,
    full_span_rich_text_format,
    decode_many_rich_text_spans
);
criterion_main!(benches);
