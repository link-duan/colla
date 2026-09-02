#![no_main]

use colla::codec::{decode_value, encode_value};
use libfuzzer_sys::fuzz_target;

// Decoding untrusted bytes must never panic, and any value the strict decoder
// accepts must be canonical: re-encoding it and decoding again yields the same
// value with byte-stable output.
fuzz_target!(|data: &[u8]| {
    if let Ok(value) = decode_value(data) {
        let encoded = encode_value(&value);
        let redecoded = decode_value(&encoded)
            .expect("canonical re-encode must decode");
        assert_eq!(value, redecoded, "decode/encode/decode must round-trip");
        assert_eq!(
            encoded,
            encode_value(&redecoded),
            "canonical encoding must be byte-stable",
        );
    }
});
