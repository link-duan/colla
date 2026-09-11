#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if let Ok(value) = colla::Value::decode(data) {
        assert_eq!(value.encode(), data);
        assert_eq!(colla::Value::decode(&value.encode()).unwrap(), value);
    }
    if let Ok(snapshot) = colla::SyncSnapshot::decode(data) {
        assert_eq!(snapshot.encode(), data);
        assert_eq!(
            colla::SyncSnapshot::decode(&snapshot.encode()).unwrap(),
            snapshot
        );
    }
});
