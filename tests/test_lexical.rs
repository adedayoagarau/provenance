use provenance::analysis::lexical;

#[test]
fn test_basic_lexical_analysis() {
    let text = "The quick brown fox jumps over the lazy dog. The dog barked loudly.";
    let result = lexical::analyze(text);

    assert!(result.total_words > 0);
    assert!(result.unique_words > 0);
    assert!(result.unique_words <= result.total_words);
    assert!(result.type_token_ratio > 0.0);
    assert!(result.type_token_ratio <= 1.0);
    assert!(result.avg_word_length > 0.0);
}

#[test]
fn test_empty_text() {
    let result = lexical::analyze("");
    assert_eq!(result.total_words, 0);
    assert_eq!(result.unique_words, 0);
    assert_eq!(result.type_token_ratio, 0.0);
}

#[test]
fn test_hapax_legomena() {
    let text = "apple banana cherry apple banana date";
    let result = lexical::analyze(text);

    // "cherry" and "date" appear once each
    assert_eq!(result.hapax_legomena, 2);
}
