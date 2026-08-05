use colla::{path, ChangeBuilder, Limits, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let limits = Limits::default();
    let base = Value::map([("title", Value::text("Draft"))])?;
    let mut builder = ChangeBuilder::new(&base, &limits)?;
    builder.text_insert(&path!["title"], 5, " v2")?;
    let after = builder.build().apply_to(&base, &limits)?;
    println!("{after:?}");
    Ok(())
}
