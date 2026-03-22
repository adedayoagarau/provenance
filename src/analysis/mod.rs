pub mod lexical;
pub mod syntactic;
pub mod semantic;
pub mod stylometric;

use anyhow::Result;

/// Complete text analysis result combining all analysis layers.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub lexical: lexical::LexicalProfile,
    pub syntactic: syntactic::SyntacticProfile,
    pub semantic: semantic::SemanticProfile,
    pub stylometric: stylometric::StylometricProfile,
}

/// Run all analysis layers on the given text.
pub fn analyze_text(text: &str) -> Result<AnalysisResult> {
    let lexical = lexical::analyze(text)?;
    let syntactic = syntactic::analyze(text)?;
    let semantic = semantic::analyze(text)?;
    let stylometric = stylometric::analyze(text)?;

    Ok(AnalysisResult {
        lexical,
        syntactic,
        semantic,
        stylometric,
    })
}
