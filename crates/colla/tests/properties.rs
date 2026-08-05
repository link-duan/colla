use std::collections::BTreeMap;

use colla::{
    transform_pair, AttrChange, AttrPatch, AttrValue, Attrs, Change, ListChange, ListOp, MapChange,
    MapEntryChange, RichSpan, RichText, RichTextChange, RichTextOp, TextChange, TextOp, TieBreak,
    Value,
};
use proptest::prelude::*;

fn text_change(base_len: usize, pos_seed: usize, del_seed: usize, insert: String) -> Change {
    let pos = pos_seed % (base_len + 1);
    let delete = del_seed % (base_len - pos + 1);
    Change::text(TextChange::new(vec![
        TextOp::Retain(pos),
        TextOp::Delete(delete),
        TextOp::Insert(insert),
    ]))
}

fn list_change(base_len: usize, pos_seed: usize, del_seed: usize, inserted: Vec<i64>) -> Change {
    let pos = pos_seed % (base_len + 1);
    let delete = del_seed % (base_len - pos + 1);
    Change::list(ListChange::new(vec![
        ListOp::Retain(pos),
        ListOp::Delete(delete),
        ListOp::Insert(inserted.into_iter().map(Value::int).collect()),
    ]))
}

fn rich_change(base_len: usize, pos_seed: usize, del_seed: usize, insert: String) -> Change {
    let pos = pos_seed % (base_len + 1);
    let delete = del_seed % (base_len - pos + 1);
    Change::rich_text(RichTextChange::new(vec![
        RichTextOp::Retain {
            len: pos,
            attrs: AttrPatch::new(),
        },
        RichTextOp::Delete(delete),
        RichTextOp::Insert {
            content: colla::RichInsert::text(insert),
            attrs: Attrs::new(),
        },
    ]))
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
        let a = Change::map(MapChange::from_entries([("text", MapEntryChange::Modify(text_change(base.chars().count(), ap, ad, ai)))]).unwrap());
        let b = Change::map(MapChange::from_entries([("text", MapEntryChange::Modify(text_change(base.chars().count(), bp, bd, bi)))]).unwrap());
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
        let base_value = Value::rich_text(RichText::new(if base.is_empty() { vec![] } else { vec![RichSpan::text(base.clone(), Attrs::new())] }));
        let a = rich_change(base.chars().count(), ap, ad, ai);
        let b = rich_change(base.chars().count(), bp, bd, bi);
        let (a_prime, b_prime) = transform_pair(&a, &b, TieBreak::LeftFirst).unwrap();
        let left = b_prime.apply_to(&a.apply_to(&base_value).unwrap()).unwrap();
        let right = a_prime.apply_to(&b.apply_to(&base_value).unwrap()).unwrap();
        prop_assert_eq!(left, right);
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
    let base = Value::rich_text(RichText::new(vec![RichSpan::text("hello", Attrs::new())]));
    let red =
        AttrPatch::from_entries([("color", AttrChange::Set(AttrValue::string("red")))]).unwrap();
    let blue =
        AttrPatch::from_entries([("color", AttrChange::Set(AttrValue::string("blue")))]).unwrap();
    let a = Change::rich_text(RichTextChange::new(vec![RichTextOp::Retain {
        len: 5,
        attrs: red,
    }]));
    let b = Change::rich_text(RichTextChange::new(vec![RichTextOp::Retain {
        len: 5,
        attrs: blue,
    }]));
    let (ap, bp) = transform_pair(&a, &b, TieBreak::LeftFirst).unwrap();
    let left = bp.apply_to(&a.apply_to(&base).unwrap()).unwrap();
    let right = ap.apply_to(&b.apply_to(&base).unwrap()).unwrap();
    assert_eq!(left, right);
}
