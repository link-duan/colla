// Also compiled as a standalone registry consumer by scripts/verify-release.mjs.
use colla::{apply, Body, Change, Operation, TextChange, TextOp, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = Value::text("Draft")?;
    let change = Change::new([Operation::Text {
        path: vec![],
        change: TextChange::from_ops([TextOp::Retain(5), TextOp::Insert(" updated".into())])?,
    }])?;
    let next = apply(&base, &change)?;
    assert_eq!(next.body(), &Body::Text("Draft updated".into()));
    assert_eq!(Value::decode(&next.encode())?, next);
    assert_eq!(Change::decode(&change.encode())?, change);
    Ok(())
}

#[test]
fn release_consumer() -> Result<(), Box<dyn std::error::Error>> {
    main()
}
