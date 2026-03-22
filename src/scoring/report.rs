use super::engine::UnifiedScore;

/// Render a unified score into a human-readable report.
pub fn render(score: &UnifiedScore) -> crate::utils::errors::Result<String> {
    let mut report = String::new();

    report.push_str("═══════════════════════════════════════════\n");
    report.push_str("  PROVENANCE — Authorship Analysis Report\n");
    report.push_str("═══════════════════════════════════════════\n\n");

    // File info
    report.push_str("── File Forensics ──\n");
    report.push_str(&format!("  File:     {}\n", score.forensic_report.metadata.file_name));
    report.push_str(&format!("  Size:     {} bytes\n", score.forensic_report.metadata.file_size));
    report.push_str(&format!("  SHA-256:  {}\n", score.forensic_report.integrity.sha256));
    report.push_str(&format!("  Format:   {:?}\n\n", score.forensic_report.format.detected_type));

    // Text analysis
    report.push_str("── Text Analysis ──\n");
    report.push_str(&format!("  Words:              {}\n", score.analysis_result.lexical.total_words));
    report.push_str(&format!("  Unique words:       {}\n", score.analysis_result.lexical.unique_words));
    report.push_str(&format!("  Type-token ratio:   {:.4}\n", score.analysis_result.lexical.type_token_ratio));
    report.push_str(&format!("  Hapax legomena:     {}\n", score.analysis_result.lexical.hapax_legomena));
    report.push_str(&format!("  Avg word length:    {:.2}\n", score.analysis_result.lexical.avg_word_length));
    report.push_str(&format!("  Sentences:          {}\n", score.analysis_result.syntactic.total_sentences));
    report.push_str(&format!("  Avg words/sentence: {:.1}\n", score.analysis_result.syntactic.avg_words_per_sentence));
    report.push_str(&format!("  Paragraphs:         {}\n\n", score.analysis_result.semantic.paragraph_count));

    // Identity comparison (if available)
    if let Some(ref comparison) = score.comparison {
        report.push_str("── Authorship Comparison ──\n");
        report.push_str(&format!("  Author:     {}\n", comparison.author_name));
        report.push_str(&format!("  Confidence: {:.1}% ({:?})\n",
            comparison.confidence.value * 100.0,
            comparison.confidence.level,
        ));
        report.push_str("\n  Feature Distances:\n");
        for fd in &comparison.feature_distances {
            report.push_str(&format!("    {:<25} expected: {:.4}  actual: {:.4}  dist: {:.4}\n",
                fd.feature_name, fd.expected, fd.actual, fd.normalized_distance
            ));
        }
        report.push('\n');
    }

    report.push_str("═══════════════════════════════════════════\n");

    Ok(report)
}
