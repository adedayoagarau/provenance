use crate::utils::errors::{ProvenanceError, Result};
use std::path::Path;

/// Extract plain text from an RTF file.
///
/// RTF (Rich Text Format) is a legacy format still common in legal and
/// academic contexts. Scrivener also uses RTF internally.
pub fn extract(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(strip_rtf(&content))
}

/// Strip RTF control codes and return plain text.
///
/// RTF uses backslash-prefixed control words and curly brace groups.
/// This parser handles the core RTF spec sufficient for text extraction.
pub fn strip_rtf(rtf: &str) -> String {
    let mut output = String::with_capacity(rtf.len() / 2);
    let mut chars = rtf.chars().peekable();
    let mut group_depth: i32 = 0;
    let mut skip_group = false;
    let mut skip_depth: i32 = 0;

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                group_depth += 1;
                if skip_group {
                    continue;
                }
            }
            '}' => {
                if group_depth == skip_depth && skip_group {
                    skip_group = false;
                }
                group_depth = group_depth.saturating_sub(1);
                continue;
            }
            '\\' if !skip_group => {
                // RTF control word or symbol
                if let Some(&next) = chars.peek() {
                    match next {
                        // Escaped special characters
                        '{' | '}' | '\\' => {
                            output.push(chars.next().unwrap());
                        }
                        // Unicode character: \uN?
                        'u' => {
                            chars.next(); // consume 'u'
                            let mut num_str = String::new();
                            // Handle negative numbers
                            if let Some(&'-') = chars.peek() {
                                num_str.push('-');
                                chars.next();
                            }
                            while let Some(&c) = chars.peek() {
                                if c.is_ascii_digit() {
                                    num_str.push(c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            // Skip replacement character (usually '?')
                            if let Some(&'?') = chars.peek() {
                                chars.next();
                            }
                            if let Ok(code) = num_str.parse::<i32>() {
                                let code = if code < 0 { (code + 65536) as u32 } else { code as u32 };
                                if let Some(c) = char::from_u32(code) {
                                    output.push(c);
                                }
                            }
                        }
                        '\'' => {
                            // Hex-encoded character: \'xx
                            chars.next(); // consume '\''
                            let mut hex = String::new();
                            for _ in 0..2 {
                                if let Some(&c) = chars.peek() {
                                    if c.is_ascii_hexdigit() {
                                        hex.push(c);
                                        chars.next();
                                    }
                                }
                            }
                            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                                // Windows-1252 decoding for high bytes
                                if byte < 0x80 {
                                    output.push(byte as char);
                                } else {
                                    let bytes = [byte];
                                    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
                                    output.push_str(&decoded);
                                }
                            }
                        }
                        '\n' | '\r' => {
                            chars.next();
                            output.push('\n');
                        }
                        _ => {
                            // Control word: read until space or non-alpha
                            let mut word = String::new();
                            while let Some(&c) = chars.peek() {
                                if c.is_ascii_alphabetic() {
                                    word.push(c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            // Skip optional numeric parameter
                            let mut _param = String::new();
                            if let Some(&'-') = chars.peek() {
                                _param.push('-');
                                chars.next();
                            }
                            while let Some(&c) = chars.peek() {
                                if c.is_ascii_digit() {
                                    _param.push(c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            // Consume trailing space (delimiter)
                            if let Some(&' ') = chars.peek() {
                                chars.next();
                            }

                            // Handle known control words
                            match word.as_str() {
                                "par" | "line" => output.push('\n'),
                                "tab" => output.push('\t'),
                                "emdash" => output.push('\u{2014}'),
                                "endash" => output.push('\u{2013}'),
                                "lquote" => output.push('\u{2018}'),
                                "rquote" => output.push('\u{2019}'),
                                "ldblquote" => output.push('\u{201C}'),
                                "rdblquote" => output.push('\u{201D}'),
                                "bullet" => output.push('\u{2022}'),
                                // Skip groups we don't want text from
                                "fonttbl" | "colortbl" | "stylesheet" | "info"
                                | "pict" | "object" | "fldinst" | "header"
                                | "footer" | "headerf" | "footerf" => {
                                    skip_group = true;
                                    skip_depth = group_depth;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ if !skip_group => {
                output.push(ch);
            }
            _ => {}
        }
    }

    // Clean up: collapse multiple blank lines, trim
    let lines: Vec<&str> = output.lines().collect();
    let mut result = String::new();
    let mut prev_empty = false;

    for line in lines {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            if !prev_empty {
                result.push('\n');
                prev_empty = true;
            }
        } else {
            if prev_empty {
                result.push('\n');
            }
            result.push_str(trimmed);
            result.push('\n');
            prev_empty = false;
        }
    }

    result.trim().to_string()
}
