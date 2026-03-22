#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        // Fuzz JSON deserialization of author profiles
        let _ = serde_json::from_str::<provenance::identity::profile::AuthorProfile>(text);

        // Fuzz feature vector deserialization
        let _ = serde_json::from_str::<provenance::identity::features::FeatureVector>(text);
    }
});
