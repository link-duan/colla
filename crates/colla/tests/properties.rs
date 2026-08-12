use std::collections::BTreeMap;

use colla::{
    transform_pair, AttrChange, AttrPatch, AttrValue, Attrs, Change, ListChange, ListOp, MapChange,
    MapEntryChange, RichContent, RichSpan, RichText, RichTextChange, RichTextOp, TextChange,
    TextOp, TieBreak, Value,
};
use proptest::prelude::*;

fn text_change(base_len: usize, pos_seed: usize, del_seed: usize, insert: String) -> Change {
    let pos = pos_seed % (base_len + 1);
    let delete = del_seed % (base_len - pos + 1);
    Change::from(
        TextChange::from_ops(vec![
            TextOp::Retain(pos),
            TextOp::Delete(delete),
            TextOp::Insert(insert),
        ])
        .unwrap(),
    )
}

fn list_change(base_len: usize, pos_seed: usize, del_seed: usize, inserted: Vec<i64>) -> Change {
    let pos = pos_seed % (base_len + 1);
    let delete = del_seed % (base_len - pos + 1);
    Change::from(
        ListChange::from_ops(vec![
            ListOp::Retain(pos),
            ListOp::Delete(delete),
            ListOp::Insert(inserted.into_iter().map(Value::int).collect()),
        ])
        .unwrap(),
    )
}

fn rich_change(base_len: usize, pos_seed: usize, del_seed: usize, insert: String) -> Change {
    let pos = pos_seed % (base_len + 1);
    let delete = del_seed % (base_len - pos + 1);
    Change::from(
        RichTextChange::from_ops(vec![
            RichTextOp::Retain {
                len: pos,
                attrs: AttrPatch::new(),
            },
            RichTextOp::Delete(delete),
            RichTextOp::Insert {
                content: colla::RichContent::text(insert),
                attrs: Attrs::new(),
            },
        ])
        .unwrap(),
    )
}

fn arb_string() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![Just('a'), Just('b'), Just('c'), Just('😭')],
        0..12,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn arb_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::null()),
        any::<bool>().prop_map(Value::bool),
        any::<i64>().prop_map(Value::int),
        (-1_000_000f64..1_000_000f64).prop_map(|v| Value::float(v).unwrap()),
        arb_string().prop_map(Value::string),
        arb_string().prop_map(Value::text),
    ];
    leaf.prop_recursive(4, 128, 8, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Value::list),
            prop::collection::btree_map("[a-z]{1,5}", inner, 0..6)
                .prop_map(|map: BTreeMap<String, Value>| Value::map(map).unwrap()),
        ]
    })
}

#[derive(Clone, Debug)]
enum RichSpanSpec {
    Text(String, RichAttrsSpec),
    Embed(i64, RichAttrsSpec),
}

#[derive(Clone, Copy, Debug)]
enum TestColor {
    Red,
    Blue,
}

impl TestColor {
    fn as_str(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Blue => "blue",
        }
    }
}

#[derive(Clone, Debug)]
struct RichAttrsSpec {
    bold: Option<bool>,
    color: Option<TestColor>,
}

fn arb_rich_attrs() -> impl Strategy<Value = RichAttrsSpec> {
    (
        prop::option::of(any::<bool>()),
        prop::option::of(prop_oneof![Just(TestColor::Red), Just(TestColor::Blue)]),
    )
        .prop_map(|(bold, color)| RichAttrsSpec { bold, color })
}

fn arb_rich_span_specs() -> impl Strategy<Value = Vec<RichSpanSpec>> {
    prop::collection::vec(
        prop_oneof![
            (
                arb_string().prop_filter("nonempty", |value| !value.is_empty()),
                arb_rich_attrs(),
            )
                .prop_map(|(text, attrs)| RichSpanSpec::Text(text, attrs)),
            (any::<i64>(), arb_rich_attrs())
                .prop_map(|(value, attrs)| RichSpanSpec::Embed(value, attrs)),
        ],
        0..8,
    )
}

fn spec_attrs(spec: &RichAttrsSpec) -> Attrs {
    let mut entries = Vec::new();
    if let Some(bold) = spec.bold {
        entries.push(("bold", AttrValue::Bool(bold)));
    }
    if let Some(color) = spec.color {
        entries.push(("color", AttrValue::string(color.as_str())));
    }
    Attrs::from_entries(entries).unwrap()
}

fn rich_from_specs(specs: &[RichSpanSpec]) -> RichText {
    RichText::from_spans(
        specs
            .iter()
            .map(|spec| match spec {
                RichSpanSpec::Text(text, attrs) => RichSpan::text(text.clone(), spec_attrs(attrs)),
                RichSpanSpec::Embed(value, attrs) => {
                    RichSpan::embed(Value::int(*value), spec_attrs(attrs))
                }
            })
            .collect(),
    )
    .unwrap()
}

#[derive(Clone, Debug)]
enum RichAction {
    InsertText(String, RichAttrsSpec),
    InsertEmbed(i64, RichAttrsSpec),
    Delete,
    FormatSet { bold: bool, color: TestColor },
    FormatRemove,
}

#[derive(Clone, Debug)]
struct RichActionSpec {
    gap_seed: usize,
    len_seed: usize,
    action: RichAction,
}

fn arb_rich_actions() -> impl Strategy<Value = Vec<RichActionSpec>> {
    prop::collection::vec(
        (
            any::<usize>(),
            any::<usize>(),
            prop_oneof![
                (
                    arb_string().prop_filter("nonempty", |value| !value.is_empty()),
                    arb_rich_attrs(),
                )
                    .prop_map(|(text, attrs)| RichAction::InsertText(text, attrs)),
                (any::<i64>(), arb_rich_attrs())
                    .prop_map(|(value, attrs)| RichAction::InsertEmbed(value, attrs)),
                Just(RichAction::Delete),
                (
                    any::<bool>(),
                    prop_oneof![Just(TestColor::Red), Just(TestColor::Blue)]
                )
                    .prop_map(|(bold, color)| RichAction::FormatSet { bold, color }),
                Just(RichAction::FormatRemove),
            ],
        )
            .prop_map(|(gap_seed, len_seed, action)| RichActionSpec {
                gap_seed,
                len_seed,
                action,
            }),
        1..8,
    )
}

fn rich_change_from_actions(base_len: usize, specs: &[RichActionSpec]) -> RichTextChange {
    let mut cursor = 0usize;
    let mut ops = Vec::new();
    for spec in specs {
        let remaining = base_len - cursor;
        let gap = spec.gap_seed % (remaining + 1);
        if gap > 0 {
            ops.push(RichTextOp::Retain {
                len: gap,
                attrs: AttrPatch::new(),
            });
            cursor += gap;
        }
        let remaining = base_len - cursor;
        match &spec.action {
            RichAction::InsertText(text, attrs) => ops.push(RichTextOp::Insert {
                content: RichContent::text(text.clone()),
                attrs: spec_attrs(attrs),
            }),
            RichAction::InsertEmbed(value, attrs) => ops.push(RichTextOp::Insert {
                content: RichContent::embed(Value::int(*value)),
                attrs: spec_attrs(attrs),
            }),
            RichAction::Delete => {
                let len = spec.len_seed % (remaining + 1);
                ops.push(RichTextOp::Delete(len));
                cursor += len;
            }
            RichAction::FormatSet { bold, color } => {
                let len = spec.len_seed % (remaining + 1);
                ops.push(RichTextOp::Retain {
                    len,
                    attrs: AttrPatch::from_entries([
                        ("bold", AttrChange::Set(AttrValue::Bool(*bold))),
                        ("color", AttrChange::Set(AttrValue::string(color.as_str()))),
                    ])
                    .unwrap(),
                });
                cursor += len;
            }
            RichAction::FormatRemove => {
                let len = spec.len_seed % (remaining + 1);
                ops.push(RichTextOp::Retain {
                    len,
                    attrs: AttrPatch::from_entries([
                        ("bold", AttrChange::Remove),
                        ("color", AttrChange::Remove),
                    ])
                    .unwrap(),
                });
                cursor += len;
            }
        }
    }
    RichTextChange::from_ops(ops).unwrap()
}

#[derive(Clone)]
struct ReferenceAtom {
    content: RichContent,
    attrs: Attrs,
}

fn reference_span(content: RichContent, attrs: Attrs) -> RichSpan {
    match content {
        RichContent::Text(text) => RichSpan::text(text.as_str(), attrs),
        RichContent::Embed(value) => RichSpan::embed(value, attrs),
    }
}

fn reference_atoms(rich: &RichText) -> Vec<ReferenceAtom> {
    let mut atoms = Vec::new();
    for span in rich.iter_spans() {
        match span.content() {
            RichContent::Text(text) => {
                for character in text.chars() {
                    atoms.push(ReferenceAtom {
                        content: RichContent::text(character.to_string()),
                        attrs: span.attrs().clone(),
                    });
                }
            }
            RichContent::Embed(value) => atoms.push(ReferenceAtom {
                content: RichContent::embed(value.clone()),
                attrs: span.attrs().clone(),
            }),
        }
    }
    atoms
}

fn reference_apply_rich(base: &RichText, change: &RichTextChange) -> RichText {
    let atoms = reference_atoms(base);
    let mut index = 0usize;
    let mut out = Vec::new();
    for op in change.ops() {
        match op {
            RichTextOp::Retain { len, attrs } => {
                for atom in &atoms[index..index + len] {
                    out.push(reference_span(
                        atom.content.clone(),
                        atom.attrs.apply_patch(attrs),
                    ));
                }
                index += len;
            }
            RichTextOp::Insert { content, attrs } => match content {
                RichContent::Text(text) => {
                    for character in text.chars() {
                        out.push(RichSpan::text(character.to_string(), attrs.clone()));
                    }
                }
                RichContent::Embed(value) => {
                    out.push(RichSpan::embed(value.clone(), attrs.clone()))
                }
            },
            RichTextOp::Delete(len) => index += len,
        }
    }
    for atom in &atoms[index..] {
        out.push(reference_span(atom.content.clone(), atom.attrs.clone()));
    }
    RichText::from_spans(out).unwrap()
}

fn reference_invert_rich(base: &RichText, change: &RichTextChange) -> Change {
    let atoms = reference_atoms(base);
    let mut index = 0usize;
    let mut out = Vec::new();
    for op in change.ops() {
        match op {
            RichTextOp::Retain { len, attrs } => {
                for atom in &atoms[index..index + len] {
                    let inverse = attrs
                        .iter()
                        .map(|(key, _)| {
                            (
                                key.clone(),
                                atom.attrs.get(key).map_or(AttrChange::Remove, |value| {
                                    AttrChange::Set(value.clone())
                                }),
                            )
                        })
                        .collect::<Vec<_>>();
                    out.push(RichTextOp::Retain {
                        len: 1,
                        attrs: AttrPatch::from_entries(inverse).unwrap(),
                    });
                }
                index += len;
            }
            RichTextOp::Insert { content, .. } => out.push(RichTextOp::Delete(content.len())),
            RichTextOp::Delete(len) => {
                for atom in &atoms[index..index + len] {
                    out.push(RichTextOp::Insert {
                        content: atom.content.clone(),
                        attrs: atom.attrs.clone(),
                    });
                }
                index += len;
            }
        }
    }
    Change::from(RichTextChange::from_ops(out).unwrap())
}

proptest! {
    #[test]
    fn text_tp1_property(
        base in arb_string(),
        ap in any::<usize>(), ad in any::<usize>(), ai in arb_string(),
        bp in any::<usize>(), bd in any::<usize>(), bi in arb_string(),
    ) {
        let base_value = Value::text(base.clone());
        let len = base.chars().count();
        let a = text_change(len, ap, ad, ai);
        let b = text_change(len, bp, bd, bi);
        prop_assert_eq!(Change::decode(&a.encode()).unwrap(), a.clone());
        let (a_prime, b_prime) = transform_pair(&a, &b, TieBreak::LeftFirst).unwrap();
        let left = b_prime.apply_to(&a.apply_to(&base_value).unwrap()).unwrap();
        let right = a_prime.apply_to(&b.apply_to(&base_value).unwrap()).unwrap();
        prop_assert_eq!(left, right);
    }

    #[test]
    fn text_compose_and_invert_property(
        base in arb_string(),
        ap in any::<usize>(), ad in any::<usize>(), ai in arb_string(),
        bp in any::<usize>(), bd in any::<usize>(), bi in arb_string(),
    ) {
        let base_value = Value::text(base.clone());
        let a = text_change(base.chars().count(), ap, ad, ai);
        let middle = a.apply_to(&base_value).unwrap();
        let middle_len = middle.as_text().unwrap().len();
        let b = text_change(middle_len, bp, bd, bi);
        let combined = a.compose(&b).unwrap();
        let sequential = b.apply_to(&middle).unwrap();
        prop_assert_eq!(combined.apply_to(&base_value).unwrap(), sequential.clone());
        let inverse = combined.invert(&base_value).unwrap();
        prop_assert_eq!(inverse.apply_to(&sequential).unwrap(), base_value);
    }

    #[test]
    fn list_tp1_property(
        base in prop::collection::vec(-20i64..20, 0..10),
        ap in any::<usize>(), ad in any::<usize>(), ai in prop::collection::vec(-20i64..20, 0..5),
        bp in any::<usize>(), bd in any::<usize>(), bi in prop::collection::vec(-20i64..20, 0..5),
    ) {
        let base_value = Value::list(base.iter().copied().map(Value::int));
        let a = list_change(base.len(), ap, ad, ai);
        let b = list_change(base.len(), bp, bd, bi);
        let (a_prime, b_prime) = transform_pair(&a, &b, TieBreak::LeftFirst).unwrap();
        let left = b_prime.apply_to(&a.apply_to(&base_value).unwrap()).unwrap();
        let right = a_prime.apply_to(&b.apply_to(&base_value).unwrap()).unwrap();
        prop_assert_eq!(left, right);
    }

    #[test]
    fn recursive_map_tp1_property(
        base in arb_string(),
        ap in any::<usize>(), ad in any::<usize>(), ai in arb_string(),
        bp in any::<usize>(), bd in any::<usize>(), bi in arb_string(),
    ) {
        let base_value = Value::map([("text", Value::text(base.clone()))]).unwrap();
        let a = Change::from(MapChange::from_entries([("text", MapEntryChange::Modify(text_change(base.chars().count(), ap, ad, ai)))]).unwrap());
        let b = Change::from(MapChange::from_entries([("text", MapEntryChange::Modify(text_change(base.chars().count(), bp, bd, bi)))]).unwrap());
        let (a_prime, b_prime) = transform_pair(&a, &b, TieBreak::LeftFirst).unwrap();
        let left = b_prime.apply_to(&a.apply_to(&base_value).unwrap()).unwrap();
        let right = a_prime.apply_to(&b.apply_to(&base_value).unwrap()).unwrap();
        prop_assert_eq!(left, right);
    }

    #[test]
    fn rich_text_tp1_property(
        base in arb_string(),
        ap in any::<usize>(), ad in any::<usize>(), ai in arb_string().prop_filter("nonempty", |s| !s.is_empty()),
        bp in any::<usize>(), bd in any::<usize>(), bi in arb_string().prop_filter("nonempty", |s| !s.is_empty()),
    ) {
        let base_value = Value::rich_text(RichText::from_spans(if base.is_empty() { vec![] } else { vec![RichSpan::text(base.clone(), Attrs::new())] }).unwrap());
        let a = rich_change(base.chars().count(), ap, ad, ai);
        let b = rich_change(base.chars().count(), bp, bd, bi);
        let (a_prime, b_prime) = transform_pair(&a, &b, TieBreak::LeftFirst).unwrap();
        let left = b_prime.apply_to(&a.apply_to(&base_value).unwrap()).unwrap();
        let right = a_prime.apply_to(&b.apply_to(&base_value).unwrap()).unwrap();
        prop_assert_eq!(left, right);
    }

    #[test]
    fn rich_text_span_algorithm_matches_atom_reference(
        specs in arb_rich_span_specs(),
        actions in arb_rich_actions(),
    ) {
        let rich = rich_from_specs(&specs);
        let base = Value::rich_text(rich.clone());
        let rich_change = rich_change_from_actions(rich.len(), &actions);
        let change = Change::from(rich_change.clone());

        let expected = Value::rich_text(reference_apply_rich(&rich, &rich_change));
        let actual = change.apply_to(&base).unwrap();
        prop_assert_eq!(&actual, &expected);

        let expected_inverse = reference_invert_rich(&rich, &rich_change);
        let actual_inverse = change.invert(&base).unwrap();
        prop_assert_eq!(&actual_inverse, &expected_inverse);
        prop_assert_eq!(actual_inverse.apply_to(&actual).unwrap(), base);
    }

    #[test]
    fn value_codec_roundtrip_property(value in arb_value()) {
        let bytes = value.encode();
        prop_assert_eq!(Value::decode(&bytes).unwrap(), value);
    }

    #[test]
    fn arbitrary_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        let _ = Value::decode(&bytes);
        let _ = Change::decode(&bytes);
    }
}

#[test]
fn rich_text_attribute_tp1() {
    let base = Value::rich_text(
        RichText::from_spans(vec![RichSpan::text("hello", Attrs::new())]).unwrap(),
    );
    let red =
        AttrPatch::from_entries([("color", AttrChange::Set(AttrValue::string("red")))]).unwrap();
    let blue =
        AttrPatch::from_entries([("color", AttrChange::Set(AttrValue::string("blue")))]).unwrap();
    let a = Change::from(
        RichTextChange::from_ops(vec![RichTextOp::Retain { len: 5, attrs: red }]).unwrap(),
    );
    let b = Change::from(
        RichTextChange::from_ops(vec![RichTextOp::Retain {
            len: 5,
            attrs: blue,
        }])
        .unwrap(),
    );
    let (ap, bp) = transform_pair(&a, &b, TieBreak::LeftFirst).unwrap();
    let left = bp.apply_to(&a.apply_to(&base).unwrap()).unwrap();
    let right = ap.apply_to(&b.apply_to(&base).unwrap()).unwrap();
    assert_eq!(left, right);
}

#[test]
fn rich_text_attribute_overwrite_and_remove_cover_text_and_embed() {
    let original_attrs = Attrs::from_entries([
        ("bold", AttrValue::Bool(false)),
        ("color", AttrValue::string("blue")),
    ])
    .unwrap();
    let base = Value::rich_text(
        RichText::from_spans(vec![
            RichSpan::text("a", original_attrs.clone()),
            RichSpan::embed(Value::int(7), original_attrs),
        ])
        .unwrap(),
    );
    let change = Change::from(
        RichTextChange::from_ops(vec![RichTextOp::Retain {
            len: 2,
            attrs: AttrPatch::from_entries([
                ("bold", AttrChange::Set(AttrValue::Bool(true))),
                ("color", AttrChange::Set(AttrValue::string("red"))),
            ])
            .unwrap(),
        }])
        .unwrap(),
    );

    let after = change.apply_to(&base).unwrap();
    for span in after.as_rich_text().unwrap().iter_spans() {
        assert_eq!(span.attrs().get("bold"), Some(&AttrValue::Bool(true)));
        assert_eq!(span.attrs().get("color"), Some(&AttrValue::string("red")));
    }
    let inverse = change.invert(&base).unwrap();
    assert_eq!(inverse.apply_to(&after).unwrap(), base);

    let remove = Change::from(
        RichTextChange::from_ops(vec![RichTextOp::Retain {
            len: 2,
            attrs: AttrPatch::from_entries([
                ("bold", AttrChange::Remove),
                ("color", AttrChange::Remove),
            ])
            .unwrap(),
        }])
        .unwrap(),
    );
    let removed = remove.apply_to(&after).unwrap();
    for span in removed.as_rich_text().unwrap().iter_spans() {
        assert!(span.attrs().is_empty());
    }
}
