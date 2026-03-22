#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let config = provenance::utils::validation::ValidationConfig::default();

        // Fuzz XML bomb detection
        let _ = provenance::utils::validation::check_xml_bomb(text, &config);

        // Fuzz path sanitization
        let _ = provenance::utils::validation::sanitize_filename(text);

        // Fuzz archive entry validation
        let _ = provenance::utils::validation::validate_archive_entry_path(text);
    }
});
