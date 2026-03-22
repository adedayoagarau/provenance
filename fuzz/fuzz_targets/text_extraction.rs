#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    // Write fuzz data to a temp file and attempt extraction
    let dir = std::env::temp_dir().join("provenance_fuzz");
    let _ = std::fs::create_dir_all(&dir);

    // Try as plain text
    let txt_path = dir.join("fuzz_input.txt");
    if let Ok(mut f) = std::fs::File::create(&txt_path) {
        let _ = f.write_all(data);
        let _ = provenance::extraction::extract_text(txt_path.to_str().unwrap_or(""));
    }

    // Try as HTML
    let html_path = dir.join("fuzz_input.html");
    if let Ok(mut f) = std::fs::File::create(&html_path) {
        let _ = f.write_all(data);
        let _ = provenance::extraction::extract_text(html_path.to_str().unwrap_or(""));
    }

    // Try as RTF
    let rtf_path = dir.join("fuzz_input.rtf");
    if let Ok(mut f) = std::fs::File::create(&rtf_path) {
        let _ = f.write_all(data);
        let _ = provenance::extraction::extract_text(rtf_path.to_str().unwrap_or(""));
    }
});
