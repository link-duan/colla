#![no_main]

use colla::codec::{decode_change, encode_change};
use colla::InputLimits;
use libfuzzer_sys::fuzz_target;

// Decoding untrusted bytes must never panic, and any change the strict decoder
// accepts must be canonical: re-encoding it and decoding again yields the same
// change with byte-stable output.
fuzz_target!(|data: &[u8]| {
    let limits = InputLimits::default();
    if let Ok(change) = decode_change(data, &limits) {
        let encoded = encode_change(&change);
        let redecoded = decode_change(&encoded, &limits)
            .expect("canonical re-encode must decode");
        assert_eq!(change, redecoded, "decode/encode/decode must round-trip");
        assert_eq!(
            encoded,
            encode_change(&redecoded),
            "canonical encoding must be byte-stable",
        );
    }
});
