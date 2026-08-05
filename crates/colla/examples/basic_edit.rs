use colla::{path, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = Value::map([("title", Value::text("Draft"))])?;
    let mut builder = base.change();
    builder.text_insert(&path!["title"], 5, " v2")?;
    let after = builder.build().apply_to(&base)?;
    println!("{after:?}");
    Ok(())
}
