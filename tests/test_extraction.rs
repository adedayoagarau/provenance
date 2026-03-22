use provenance::extraction;

// === Plain Text ===

#[test]
fn test_plaintext_extraction() {
    let text = extraction::extract_text("tests/fixtures/sample.txt").unwrap();
    assert!(text.contains("quick brown fox"));
    assert!(text.contains("third paragraph"));
    assert!(!text.is_empty());
}

// === Markdown ===

#[test]
fn test_markdown_extraction() {
    let text = extraction::extract_text("tests/fixtures/sample.md").unwrap();
    assert!(text.contains("Sample Document"));
    assert!(text.contains("bold"));
    assert!(text.contains("italic"));
    assert!(text.contains("Final paragraph"));
    assert!(!text.contains("**bold**"));
    assert!(!text.contains("```"));
    assert!(!text.contains("def hello"));
}

#[test]
fn test_markdown_strip_function() {
    use provenance::extraction::markdown::strip_markdown;
    let md = "# Hello\n\nThis is **bold** and *italic*.\n\n- item 1\n- item 2\n";
    let text = strip_markdown(md);
    assert!(text.contains("Hello"));
    assert!(text.contains("bold"));
    assert!(!text.contains("**"));
}

// === HTML ===

#[test]
fn test_html_extraction() {
    let text = extraction::extract_text("tests/fixtures/sample.html").unwrap();
    assert!(text.contains("first paragraph"));
    assert!(text.contains("bold"));
    assert!(text.contains("Final paragraph"));
    assert!(!text.contains("<p>"));
    assert!(!text.contains("console.log"));
}

#[test]
fn test_html_strip_function() {
    use provenance::extraction::html::strip_html;
    let html = "<html><body><p>Hello <strong>world</strong></p><script>bad();</script></body></html>";
    let text = strip_html(html);
    assert!(text.contains("Hello"));
    assert!(text.contains("world"));
    assert!(!text.contains("<p>"));
    assert!(!text.contains("bad()"));
}

// === RTF ===

#[test]
fn test_rtf_extraction() {
    let text = extraction::extract_text("tests/fixtures/sample.rtf").unwrap();
    assert!(text.contains("sample RTF document"));
    assert!(text.contains("bold text"));
    assert!(text.contains("italic text"));
    assert!(text.contains("Final paragraph"));
    // Should not contain RTF control codes
    assert!(!text.contains("\\rtf"));
    assert!(!text.contains("\\fonttbl"));
    assert!(!text.contains("\\b "));
}

#[test]
fn test_rtf_strip_function() {
    use provenance::extraction::rtf::strip_rtf;
    let rtf = r"{\rtf1\ansi Hello \b world\b0  end.}";
    let text = strip_rtf(rtf);
    assert!(text.contains("Hello"));
    assert!(text.contains("world"));
    assert!(text.contains("end"));
    assert!(!text.contains("\\rtf"));
}

// === LaTeX ===

#[test]
fn test_latex_extraction() {
    let text = extraction::extract_text("tests/fixtures/sample.tex").unwrap();
    assert!(text.contains("introduction paragraph"));
    assert!(text.contains("bold text"));
    assert!(text.contains("emphasized text"));
    assert!(text.contains("Methodology"));
    assert!(text.contains("final sentence"));
    // Should NOT contain LaTeX commands
    assert!(!text.contains("\\documentclass"));
    assert!(!text.contains("\\usepackage"));
    assert!(!text.contains("\\begin{equation}"));
    // Math should be excluded
    assert!(!text.contains("mc^2"));
    // Comments should be excluded
    assert!(!text.contains("This is a comment"));
}

#[test]
fn test_latex_strip_function() {
    use provenance::extraction::latex::strip_latex;
    let tex = r"\section{Hello} This is \textbf{bold} text. $E=mc^2$ End.";
    let text = strip_latex(tex);
    assert!(text.contains("Hello"));
    assert!(text.contains("bold"));
    assert!(text.contains("End"));
    assert!(!text.contains("\\section"));
    assert!(!text.contains("\\textbf"));
    // Inline math excluded
    assert!(!text.contains("mc^2"));
}

// === Email ===

#[test]
fn test_eml_extraction() {
    let text = extraction::extract_text("tests/fixtures/sample.eml").unwrap();
    assert!(text.contains("body of a sample email"));
    assert!(text.contains("multiple paragraphs"));
    assert!(text.contains("Best regards"));
}

// === Chat Export (JSON) ===

#[test]
fn test_json_chat_extraction() {
    let text = extraction::extract_text("tests/fixtures/chat_export.json").unwrap();
    assert!(text.contains("first message"));
    assert!(text.contains("project direction"));
    assert!(text.contains("implementation strategy"));
}

// === Error Cases ===

#[test]
fn test_file_not_found() {
    let result = extraction::extract_text("nonexistent_file.txt");
    assert!(result.is_err());
}

#[test]
fn test_text_normalization() {
    let text = extraction::extract_text("tests/fixtures/sample.txt").unwrap();
    assert!(!text.contains("\n\n\n"));
    assert!(!text.starts_with('\n'));
    assert!(!text.ends_with('\n'));
}

// === Collection ===

#[test]
fn test_collect_samples() {
    let samples = extraction::collect_samples("tests/fixtures").unwrap();
    assert!(samples.len() >= 7); // txt, md, html, rtf, tex, eml, json
    assert!(samples.iter().any(|s| s.ends_with(".txt")));
    assert!(samples.iter().any(|s| s.ends_with(".md")));
    assert!(samples.iter().any(|s| s.ends_with(".html")));
    assert!(samples.iter().any(|s| s.ends_with(".rtf")));
    assert!(samples.iter().any(|s| s.ends_with(".tex")));
    assert!(samples.iter().any(|s| s.ends_with(".eml")));
}
