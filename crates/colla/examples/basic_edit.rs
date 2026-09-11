use colla::{Document, History, Ref, Segment, Value};

fn main() -> colla::Result<()> {
    let item = Value::text("Draft")?;
    let id = item.id();
    let doc = Document::create(Value::map([
        ("from".into(), Value::list(vec![item])?),
        ("to".into(), Value::list(vec![])?),
        ("selected".into(), Value::reference(id)),
    ])?)?;
    let history = History::attach(&doc)?;
    doc.edit(|tx| {
        tx.text_replace(id, 5, 0, " v2")?;
        tx.move_to(id, vec![Segment::Key("to".into())], Segment::Index(0))
    })?;
    println!("Path after move: {:?}", doc.path_of(id)?);
    println!(
        "Ref still targets the same item: {}",
        doc.resolve(Ref { target: id })?.map(|value| value.id()) == Some(id)
    );
    println!("Edited title: {:?}", doc.get(id)?.body());
    history.undo()?;
    println!("Path after undo: {:?}", doc.path_of(id)?);
    println!("Restored title: {:?}", doc.get(id)?.body());
    history.close()?;
    doc.close()?;
    Ok(())
}
