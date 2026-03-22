//! Integration tests for the full analysis pipeline (Phases 13-16).

use provenance::scoring::engine::OutputFormat;

#[test]
fn test_full_pipeline_text_output() {
    let result = provenance::analyze_with_format(
        "tests/fixtures/sample.txt",
        None,
        OutputFormat::Text,
    )
    .unwrap();

    // Phase 15: Reliability disclosure always present
    assert!(
        result.contains("Reliability Disclosure"),
        "Report must include reliability disclosure"
    );

    // Phase 15: Executive summary
    assert!(
        result.contains("Executive Summary"),
        "Report must include executive summary"
    );

    // Phase 13: Register classification
    assert!(
        result.contains("Register Classification"),
        "Report must include register classification"
    );

    // Phase 14: ACS
    assert!(
        result.contains("Authorship Confidence Score"),
        "Report must include ACS"
    );

    // Phase 14: PII
    assert!(
        result.contains("Process Integrity Index"),
        "Report must include PII"
    );

    // Phase 13: Register baselines
    assert!(
        result.contains("Register Baseline Comparison"),
        "Report must include baseline comparison"
    );

    // Existing sections still present
    assert!(result.contains("Lexical Analysis"));
    assert!(result.contains("Syntactic Analysis"));
    assert!(result.contains("Audit Trail"));
}

#[test]
fn test_full_pipeline_json_output() {
    let result = provenance::analyze_with_format(
        "tests/fixtures/sample.txt",
        None,
        OutputFormat::Json,
    )
    .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();

    // Phase 13: Register present
    assert!(json.get("register").is_some(), "JSON must contain register");
    assert!(
        json["register"]["label"].is_string(),
        "Register must have label"
    );

    // Phase 14: ACS present
    assert!(json.get("acs").is_some(), "JSON must contain ACS");
    let acs_score = json["acs"]["score"].as_f64().unwrap();
    assert!(
        (0.0..=100.0).contains(&acs_score),
        "ACS score must be 0-100, got {acs_score}"
    );

    // Phase 14: PII present
    assert!(json.get("pii").is_some(), "JSON must contain PII");

    // Phase 15: Evaluator report present
    assert!(
        json.get("evaluator_report").is_some(),
        "JSON must contain evaluator_report"
    );

    // Baseline report
    assert!(
        json.get("baseline_report").is_some(),
        "JSON must contain baseline_report"
    );
}

#[test]
fn test_full_pipeline_html_output() {
    let result = provenance::analyze_with_format(
        "tests/fixtures/sample.txt",
        None,
        OutputFormat::Html,
    )
    .unwrap();

    // Valid HTML
    assert!(result.contains("<!DOCTYPE html>"));
    assert!(result.contains("</html>"));

    // Phase 15: Reliability disclosure
    assert!(
        result.contains("Reliability Disclosure"),
        "HTML must include reliability disclosure"
    );

    // Phase 14: Score cards
    assert!(
        result.contains("Authorship Confidence"),
        "HTML must include ACS card"
    );
    assert!(
        result.contains("Process Integrity"),
        "HTML must include PII card"
    );

    // Phase 13: Register
    assert!(
        result.contains("Detected Register"),
        "HTML must include register card"
    );

    // Phase 13: Baselines
    assert!(
        result.contains("Register Baseline Comparison"),
        "HTML must include baseline section"
    );

    // Recommendations
    assert!(
        result.contains("Recommended Actions"),
        "HTML must include recommendations"
    );
}

#[test]
fn test_full_pipeline_markdown_input() {
    let result = provenance::analyze_with_format(
        "tests/fixtures/sample.md",
        None,
        OutputFormat::Json,
    )
    .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();

    // All Phase 13-15 components present regardless of input format
    assert!(json.get("register").is_some());
    assert!(json.get("acs").is_some());
    assert!(json.get("pii").is_some());
    assert!(json.get("evaluator_report").is_some());
}

#[test]
fn test_full_pipeline_html_input() {
    let result = provenance::analyze_with_format(
        "tests/fixtures/sample.html",
        None,
        OutputFormat::Json,
    )
    .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(json.get("register").is_some());
    assert!(json.get("acs").is_some());
}

#[test]
fn test_pipeline_framing_no_ai_claims() {
    let result = provenance::analyze_with_format(
        "tests/fixtures/sample.txt",
        None,
        OutputFormat::Text,
    )
    .unwrap();

    let lower = result.to_lowercase();
    assert!(
        !lower.contains("ai-generated"),
        "Report must never claim AI generation"
    );
    assert!(
        !lower.contains("written by ai"),
        "Report must never claim AI authorship"
    );
    assert!(
        !lower.contains("% ai"),
        "Report must never give AI percentage"
    );
}

#[test]
fn test_forensics_command_uses_full_pipeline() {
    let result = provenance::run_forensics_with_format(
        "tests/fixtures/sample.txt",
        OutputFormat::Json,
    )
    .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    // Forensics-only command now uses full pipeline
    assert!(json.get("register").is_some());
    assert!(json.get("acs").is_some());
    assert!(json.get("pii").is_some());
}
