use colla::{transform_pair, Change, Limits, TextChange, TextOp, TieBreak, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let limits = Limits::default();
    let base = Value::text("Hello");
    let left = Change::text(TextChange::new(vec![TextOp::Insert("A: ".into())]));
    let right = Change::text(TextChange::new(vec![
        TextOp::Retain(5),
        TextOp::Insert("!".into()),
    ]));
    let (left_prime, right_prime) = transform_pair(&left, &right, TieBreak::LeftFirst, &limits)?;
    let a = right_prime.apply_to(&left.apply_to(&base, &limits)?, &limits)?;
    let b = left_prime.apply_to(&right.apply_to(&base, &limits)?, &limits)?;
    assert_eq!(a, b);
    println!("{a:?}");
    Ok(())
}
