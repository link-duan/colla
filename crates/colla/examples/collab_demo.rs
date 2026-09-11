use colla::{Authority, ServerMessage, SessionCheckpoint, SyncSession, Value};

fn main() -> colla::Result<()> {
    let value = Value::int(0);
    let id = value.id();
    let mut authority = Authority::create("counter", value)?;
    let alice = SyncSession::create("alice", authority.snapshot())?;
    let bob = SyncSession::create("bob", authority.snapshot())?;
    alice.document().edit(|tx| tx.increment(id, 1))?;
    bob.document().edit(|tx| tx.increment(id, 2))?;

    let bytes = alice.checkpoint()?.encode();
    alice.close()?; // Stop the old writer before restoring its identity.
    let alice = SyncSession::restore(SessionCheckpoint::decode(&bytes)?)?;
    println!(
        "Alice after restart: {:?}",
        alice.document().get(id)?.body()
    );

    for client in [&alice, &bob] {
        let Some(request) = client.outbound()? else {
            continue;
        };
        let (next, message) = authority.accept(&request)?;
        if let ServerMessage::Rejection(rejection) = &message {
            client.receive(&message)?;
            println!("Synchronization stopped: {}", rejection.reason());
            continue;
        }
        // In production, persist next before adopting it and broadcasting.
        authority = next;
        alice.receive(&message)?;
        bob.receive(&message)?;
        println!("Confirmed server revision: {}", authority.revision());
    }
    println!("Alice after sync: {:?}", alice.document().get(id)?.body());
    println!("Bob after sync: {:?}", bob.document().get(id)?.body());
    alice.close()?;
    bob.close()?;
    Ok(())
}
