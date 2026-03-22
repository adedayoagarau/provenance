use provenance::extraction;

#[test]
fn test_plaintext_extraction() {
    let text = extraction::extract_text("tests/fixtures/sample.txt").unwrap();
    assert!(text.contains("quick brown fox"));
    assert!(text.contains("third paragraph"));
    assert!(!text.is_empty());
}

#[test]
fn test_markdown_extraction() {
    let text = extraction::extract_text("tests/fixtures/sample.md").unwrap();

    // Should contain text content
    assert!(text.contains("Sample Document"));
    assert!(text.contains("bold"));
    assert!(text.contains("italic"));
    assert!(text.contains("inline code"));
    assert!(text.contains("Final paragraph"));

    // Should NOT contain markdown syntax
    assert!(!text.contains("**bold**"));
    assert!(!text.contains("*italic*"));
    assert!(!text.contains("```"));
    assert!(!text.contains("##"));

    // Code blocks should be excluded
    assert!(!text.contains("def hello"));
}

#[test]
fn test_html_extraction() {
    let text = extraction::extract_text("tests/fixtures/sample.html").unwrap();

    // Should contain text content
    assert!(text.contains("first paragraph"));
    assert!(text.contains("bold"));
    assert!(text.contains("italic"));
    assert!(text.contains("Final paragraph"));

    // Should NOT contain HTML tags
    assert!(!text.contains("<p>"));
    assert!(!text.contains("</p>"));
    assert!(!text.contains("<strong>"));
    assert!(!text.contains("<html>"));

    // Script content should be excluded
    assert!(!text.contains("console.log"));
    assert!(!text.contains("This script content"));
}

#[test]
fn test_file_not_found() {
    let result = extraction::extract_text("nonexistent_file.txt");
    assert!(result.is_err());
}

#[test]
fn test_text_normalization() {
    // Test that extracted text is unicode-normalized
    let text = extraction::extract_text("tests/fixtures/sample.txt").unwrap();

    // Should not have excessive whitespace
    assert!(!text.contains("\n\n\n"));

    // Should be trimmed
    assert!(!text.starts_with('\n'));
    assert!(!text.ends_with('\n'));
}

#[test]
fn test_markdown_strip_function() {
    use provenance::extraction::markdown::strip_markdown;

    let md = "# Hello\n\nThis is **bold** and *italic*.\n\n- item 1\n- item 2\n";
    let text = strip_markdown(md);

    assert!(text.contains("Hello"));
    assert!(text.contains("bold"));
    assert!(text.contains("italic"));
    assert!(text.contains("item 1"));
    assert!(!text.contains("**"));
    assert!(!text.contains("*italic*"));
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

#[test]
fn test_collect_samples() {
    let samples = extraction::collect_samples("tests/fixtures").unwrap();
    assert!(samples.len() >= 3); // At least our 3 fixture files
    assert!(samples.iter().any(|s| s.ends_with(".txt")));
    assert!(samples.iter().any(|s| s.ends_with(".md")));
    assert!(samples.iter().any(|s| s.ends_with(".html")));
}
