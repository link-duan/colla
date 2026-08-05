use colla::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = Value::map([("title", Value::text("hello"))])?;
    let bytes = value.encode();
    let decoded = Value::decode(&bytes)?;
    assert_eq!(decoded, value);
    Ok(())
}
