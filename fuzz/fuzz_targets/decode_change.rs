#![no_main]
use colla::*;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    macro_rules! canonical {
        ($ty:ty) => {
            if let Ok(value) = <$ty>::decode(data) {
                let encoded = value.encode();
                assert_eq!(encoded, data);
                assert_eq!(<$ty>::decode(&encoded).unwrap().encode(), encoded);
            }
        };
    }
    canonical!(Change);
    canonical!(Submission);
    canonical!(ServerMessage);
    canonical!(SessionCheckpoint);
    canonical!(HistoryCheckpoint);
    canonical!(AuthorityCheckpoint);
});
