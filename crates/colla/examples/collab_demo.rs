use colla::{apply, transform_pair, Change, TextChange, TextOp, TieBreak, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = Value::text("Hello");
    let left = Change::from(TextChange::from_ops(vec![TextOp::Insert("A: ".into())]).unwrap());
    let right = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(5), TextOp::Insert("!".into())]).unwrap(),
    );
    let (left_prime, right_prime) = transform_pair(&left, &right, TieBreak::LeftFirst)?;
    let a = apply(&apply(&base, &left)?, &right_prime)?;
    let b = apply(&apply(&base, &right)?, &left_prime)?;
    assert_eq!(a, b);
    println!("{a:?}");
    Ok(())
}
