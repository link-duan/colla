use colla::{codec, Limits, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = Value::map([("title", Value::text("hello"))])?;
    let bytes = codec::encode_value(&value);
    let decoded = codec::decode_value(&bytes, &Limits::default())?;
    assert_eq!(decoded, value);
    Ok(())
}
