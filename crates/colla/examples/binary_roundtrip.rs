use colla::{Authority, AuthorityCheckpoint, Value};

fn main() -> colla::Result<()> {
    let value = Value::text("😀 stable identity")?;
    let decoded = Value::decode(&value.encode())?;
    println!("Value content and IDs preserved: {}", decoded == value);
    let authority = Authority::create("example", value)?;
    let bytes = authority.checkpoint().encode();
    let restored = Authority::restore(AuthorityCheckpoint::decode(&bytes)?)?;
    println!(
        "Authority snapshot preserved: {}",
        restored.snapshot() == authority.snapshot()
    );
    Ok(())
}
