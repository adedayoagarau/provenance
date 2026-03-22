pub mod lexical;
pub mod syntactic;
pub mod semantic;
pub mod stylometric;
pub mod function_words;
pub mod ngrams;

use serde::{Deserialize, Serialize};

/// Complete text analysis result combining all analysis layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub lexical: lexical::LexicalProfile,
    pub syntactic: syntactic::SyntacticProfile,
    pub semantic: semantic::SemanticProfile,
    pub stylometric: stylometric::StylometricProfile,
    pub function_words: function_words::FunctionWordProfile,
    pub ngrams: ngrams::NgramProfile,
}

/// Run all analysis layers on the given text.
pub fn analyze_text(text: &str) -> crate::utils::errors::Result<AnalysisResult> {
    let lexical = lexical::analyze(text);
    let syntactic = syntactic::analyze(text);
    let semantic = semantic::analyze(text);
    let stylometric = stylometric::analyze(text);
    let function_words = function_words::analyze(text);
    let ngrams = ngrams::analyze(text);

    Ok(AnalysisResult {
        lexical,
        syntactic,
        semantic,
        stylometric,
        function_words,
        ngrams,
    })
}
