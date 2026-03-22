use super::engine::UnifiedScore;

/// Render a unified score into a human-readable report.
pub fn render(score: &UnifiedScore) -> crate::utils::errors::Result<String> {
    let mut r = String::new();

    r.push_str("═══════════════════════════════════════════════════\n");
    r.push_str("  PROVENANCE — Authorship Analysis Report\n");
    r.push_str("═══════════════════════════════════════════════════\n\n");

    // File forensics
    r.push_str("── File Forensics ──\n");
    r.push_str(&format!("  File:     {}\n", score.forensic_report.metadata.file_name));
    r.push_str(&format!("  Size:     {} bytes\n", score.forensic_report.metadata.file_size));
    r.push_str(&format!("  SHA-256:  {}\n", score.forensic_report.integrity.sha256));
    r.push_str(&format!("  Format:   {:?}\n\n", score.forensic_report.format.detected_type));

    // Lexical analysis
    let lex = &score.analysis_result.lexical;
    r.push_str("── Lexical Analysis ──\n");
    r.push_str(&format!("  Words:              {}\n", lex.total_words));
    r.push_str(&format!("  Unique words:       {}\n", lex.unique_words));
    r.push_str(&format!("  MATTR:              {:.4}\n", lex.mattr));
    r.push_str(&format!("  Yule's K:           {:.2}\n", lex.yules_k));
    r.push_str(&format!("  Honoré's R:         {:.2}\n", lex.honores_r));
    r.push_str(&format!("  Hapax legomena:     {} ({:.1}%)\n", lex.hapax_legomena, lex.hapax_ratio * 100.0));
    r.push_str(&format!("  Avg word length:    {:.2}\n\n", lex.avg_word_length));

    // Syntactic analysis
    let syn = &score.analysis_result.syntactic;
    r.push_str("── Syntactic Analysis ──\n");
    r.push_str(&format!("  Sentences:          {}\n", syn.total_sentences));
    r.push_str(&format!("  Avg words/sentence: {:.1}\n", syn.avg_words_per_sentence));
    r.push_str(&format!("  Length variance:     {:.1}\n", syn.sentence_length_variance));
    r.push_str(&format!("  Declarative:        {}\n", syn.declarative_count));
    r.push_str(&format!("  Interrogative:      {} ({:.1}%)\n", syn.interrogative_count, syn.interrogative_ratio * 100.0));
    r.push_str(&format!("  Exclamatory:        {} ({:.1}%)\n", syn.exclamatory_count, syn.exclamatory_ratio * 100.0));
    r.push_str(&format!("  Passive voice:      {:.1}%\n\n", syn.passive_voice_ratio * 100.0));

    // Stylometric analysis
    let sty = &score.analysis_result.stylometric;
    r.push_str("── Stylometric Analysis ──\n");
    r.push_str(&format!("  Paragraphs:         {}\n", sty.paragraph_count));
    r.push_str(&format!("  Contractions:       {} ({:.2}%)\n", sty.contraction_count, sty.contraction_ratio * 100.0));
    r.push_str(&format!("  Hedge words:        {} ({:.2}%)\n", sty.hedge_word_count, sty.hedge_word_ratio * 100.0));
    r.push_str(&format!("  Intensifiers:       {} ({:.2}%)\n", sty.intensifier_count, sty.intensifier_ratio * 100.0));
    r.push_str(&format!("  Em-dashes:          {}\n", sty.em_dash_count));
    r.push_str(&format!("  Ellipses:           {}\n", sty.ellipsis_count));
    r.push_str(&format!("  Parentheticals:     {}\n\n", sty.parenthetical_count));

    // Function word analysis
    let fw = &score.analysis_result.function_words;
    r.push_str("── Function Word Analysis ──\n");
    r.push_str(&format!("  Function word ratio: {:.1}%\n", fw.function_word_ratio * 100.0));
    r.push_str(&format!("  Distinct FW used:    {}\n", fw.function_word_diversity));
    // Show top 10 function words
    let mut top_fw: Vec<(&String, &f64)> = fw.frequencies.iter().collect();
    top_fw.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
    r.push_str("  Top function words (per 1000 words):\n");
    for (word, freq) in top_fw.iter().take(10) {
        r.push_str(&format!("    {:<15} {:.1}\n", word, freq));
    }
    r.push('\n');

    // Readability
    let sem = &score.analysis_result.semantic;
    r.push_str("── Readability & Discourse ──\n");
    r.push_str(&format!("  Flesch-Kincaid:     {:.1} grade\n", sem.flesch_kincaid_grade));
    r.push_str(&format!("  Gunning Fog:        {:.1}\n", sem.gunning_fog_index));
    r.push_str(&format!("  Coleman-Liau:       {:.1}\n", sem.coleman_liau_index));
    r.push_str(&format!("  Discourse markers:  {:.2}%\n\n", sem.discourse_marker_ratio * 100.0));

    // Identity comparison (if available)
    if let Some(ref comparison) = score.comparison {
        r.push_str("── Authorship Comparison ──\n");
        r.push_str(&format!("  Author:     {}\n", comparison.author_name));
        r.push_str(&format!("  Confidence: {:.1}% ({:?})\n",
            comparison.confidence.value * 100.0,
            comparison.confidence.level,
        ));
        r.push_str("\n  Feature Distances:\n");
        for fd in &comparison.feature_distances {
            r.push_str(&format!("    {:<28} expected: {:>8.4}  actual: {:>8.4}  dist: {:.4}\n",
                fd.feature_name, fd.expected, fd.actual, fd.normalized_distance
            ));
        }
        r.push('\n');
    }

    r.push_str("═══════════════════════════════════════════════════\n");

    Ok(r)
}
