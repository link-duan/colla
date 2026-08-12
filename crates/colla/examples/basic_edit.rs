use colla::{MapChange, MapEntryChange, TextChange, TextOp, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = Value::map([("title", Value::text("Draft"))])?;
    let title = TextChange::from_ops([TextOp::Retain(5), TextOp::Insert(" v2".into())])?;
    let change = MapChange::from_entries([("title", MapEntryChange::Modify(title.into()))])?;
    let after = colla::Change::from(change).apply_to(&base)?;
    println!("{after:?}");
    Ok(())
}
