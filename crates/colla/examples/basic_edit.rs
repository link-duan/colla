use colla::{Document, History, Segment, Value};

fn main() -> colla::Result<()> {
    let doc = Document::create(Value::map([(
        "items".into(),
        Value::list(vec![Value::text("Draft")?, Value::text("Review")?])?,
    )])?);
    let items = [Segment::Key("items".into())];
    let first = [items[0].clone(), Segment::Index(0)];
    let history = History::attach(&doc)?;
    doc.edit(|tx| {
        tx.text_replace(&first, 5, 0, " updated")?;
        tx.list_move(&items, 0, 1)
    })?;
    println!("Items after move: {:?}", doc.get(&items)?.body());
    history.undo()?;
    println!("Items after undo: {:?}", doc.get(&items)?.body());
    history.close()?;
    doc.close()?;
    Ok(())
}
