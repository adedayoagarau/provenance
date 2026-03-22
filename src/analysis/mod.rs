pub mod lexical;
pub mod syntactic;
pub mod semantic;
pub mod stylometric;
pub mod function_words;
pub mod ngrams;
pub mod register;
pub mod baselines;

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

/// Run all analysis layers on the given text in parallel using rayon.
pub fn analyze_text(text: &str) -> crate::utils::errors::Result<AnalysisResult> {
    let (
        (lexical, syntactic),
        ((semantic, stylometric), (function_words, ngrams)),
    ) = rayon::join(
        || rayon::join(|| lexical::analyze(text), || syntactic::analyze(text)),
        || {
            rayon::join(
                || rayon::join(|| semantic::analyze(text), || stylometric::analyze(text)),
                || rayon::join(|| function_words::analyze(text), || ngrams::analyze(text)),
            )
        },
    );

    Ok(AnalysisResult {
        lexical,
        syntactic,
        semantic,
        stylometric,
        function_words,
        ngrams,
    })
}
