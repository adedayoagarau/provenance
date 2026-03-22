//! Corpus loading, indexing, deduplication, and quality scoring.
//!
//! A corpus follows the convention: `corpus_dir/<author_name>/<text_files>`.
//! The loader indexes all documents, computes quality scores, and optionally
//! deduplicates near-identical texts.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::extraction;
use crate::utils::errors::{ProvenanceError, Result};

/// A single document in the corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Author label
    pub author: String,
    /// Path to the source file
    pub path: PathBuf,
    /// Extracted text content
    pub text: String,
    /// Word count
    pub word_count: usize,
    /// SHA-256 hash of the extracted text (for deduplication)
    pub text_hash: String,
    /// Quality score (0.0–1.0)
    pub quality_score: f64,
}

/// Summary statistics for a loaded corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusSummary {
    /// Total documents loaded
    pub total_documents: usize,
    /// Documents per author
    pub documents_per_author: HashMap<String, usize>,
    /// Total word count across corpus
    pub total_words: usize,
    /// Number of unique authors
    pub n_authors: usize,
    /// Documents removed by deduplication
    pub duplicates_removed: usize,
    /// Documents below quality threshold
    pub low_quality_count: usize,
}

/// Quality criteria for filtering documents.
#[derive(Debug, Clone)]
pub struct QualityCriteria {
    /// Minimum word count to include a document
    pub min_words: usize,
    /// Minimum quality score to include
    pub min_quality: f64,
    /// Whether to deduplicate by text hash
    pub deduplicate: bool,
}

impl Default for QualityCriteria {
    fn default() -> Self {
        Self {
            min_words: 100,
            min_quality: 0.3,
            deduplicate: true,
        }
    }
}

/// A loaded and indexed corpus ready for ML pipeline consumption.
#[derive(Debug, Clone)]
pub struct Corpus {
    /// All documents (after filtering)
    pub documents: Vec<Document>,
    /// Summary statistics
    pub summary: CorpusSummary,
}

impl Corpus {
    /// Load a corpus from a directory.
    ///
    /// Expected structure: `corpus_dir/<author_name>/<text_files>`
    pub fn load(corpus_dir: &Path, criteria: &QualityCriteria) -> Result<Self> {
        if !corpus_dir.is_dir() {
            return Err(ProvenanceError::FileNotFound {
                path: corpus_dir.display().to_string(),
            });
        }

        let mut all_documents = Vec::new();

        let entries = std::fs::read_dir(corpus_dir).map_err(|e| ProvenanceError::IoWithPath {
            path: corpus_dir.display().to_string(),
            source: e,
        })?;

        for entry in entries {
            let entry = entry?;
            let author_dir = entry.path();
            if !author_dir.is_dir() {
                continue;
            }

            let author_name = author_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let files =
                std::fs::read_dir(&author_dir).map_err(|e| ProvenanceError::IoWithPath {
                    path: author_dir.display().to_string(),
                    source: e,
                })?;

            for file_entry in files {
                let file_entry = file_entry?;
                let file_path = file_entry.path();
                if !file_path.is_file() {
                    continue;
                }

                match load_document(&author_name, &file_path) {
                    Ok(doc) => all_documents.push(doc),
                    Err(_) => continue, // Skip unsupported formats
                }
            }
        }

        // Sort by author then path for determinism
        all_documents.sort_by(|a, b| {
            a.author.cmp(&b.author).then_with(|| a.path.cmp(&b.path))
        });

        // Apply filters
        let pre_filter_count = all_documents.len();
        let (documents, duplicates_removed) = filter_documents(all_documents, criteria);
        let low_quality_count = pre_filter_count - documents.len() - duplicates_removed;

        // Build summary
        let mut documents_per_author: HashMap<String, usize> = HashMap::new();
        let mut total_words = 0;
        for doc in &documents {
            *documents_per_author.entry(doc.author.clone()).or_default() += 1;
            total_words += doc.word_count;
        }

        let summary = CorpusSummary {
            total_documents: documents.len(),
            documents_per_author,
            total_words,
            n_authors: documents
                .iter()
                .map(|d| &d.author)
                .collect::<HashSet<_>>()
                .len(),
            duplicates_removed,
            low_quality_count,
        };

        Ok(Corpus { documents, summary })
    }

    /// Get documents for a specific author.
    pub fn by_author(&self, author: &str) -> Vec<&Document> {
        self.documents.iter().filter(|d| d.author == author).collect()
    }

    /// Get all unique author names, sorted.
    pub fn authors(&self) -> Vec<String> {
        let mut authors: Vec<String> = self
            .documents
            .iter()
            .map(|d| d.author.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        authors.sort();
        authors
    }

    /// Get documents grouped by author.
    pub fn grouped(&self) -> HashMap<&str, Vec<&Document>> {
        let mut groups: HashMap<&str, Vec<&Document>> = HashMap::new();
        for doc in &self.documents {
            groups.entry(&doc.author).or_default().push(doc);
        }
        groups
    }
}

/// Load a single document, extracting text and computing metadata.
fn load_document(author: &str, path: &Path) -> Result<Document> {
    let path_str = path.to_str().unwrap_or("");
    let text = extraction::extract_text(path_str)?;
    let word_count = text.split_whitespace().count();
    let text_hash = compute_hash(&text);
    let quality_score = compute_quality(&text, word_count);

    Ok(Document {
        author: author.to_string(),
        path: path.to_path_buf(),
        text,
        word_count,
        text_hash,
        quality_score,
    })
}

/// Compute SHA-256 hash of text content.
fn compute_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Compute quality score for a document (0.0–1.0).
///
/// Factors:
/// - Word count (more words = higher quality, capped at 2000)
/// - Character diversity (too low suggests encoding issues)
/// - Sentence count (multiple sentences preferred)
fn compute_quality(text: &str, word_count: usize) -> f64 {
    if word_count == 0 {
        return 0.0;
    }

    // Word count factor: linear ramp from 0 at 0 words to 1.0 at 2000 words
    let word_factor = (word_count as f64 / 2000.0).min(1.0);

    // Character diversity: unique chars / total chars
    let chars: Vec<char> = text.chars().collect();
    let unique_chars: HashSet<&char> = chars.iter().collect();
    let diversity = if chars.is_empty() {
        0.0
    } else {
        (unique_chars.len() as f64 / chars.len() as f64).min(1.0)
    };
    // Penalize very low diversity (likely garbage)
    let diversity_factor = if diversity < 0.01 { 0.1 } else { 1.0 };

    // Sentence factor: at least 3 sentences for reasonable analysis
    let sentence_count = text
        .chars()
        .filter(|c| *c == '.' || *c == '!' || *c == '?')
        .count();
    let sentence_factor = (sentence_count as f64 / 3.0).min(1.0);

    // Weighted combination
    let score = word_factor * 0.5 + diversity_factor * 0.2 + sentence_factor * 0.3;
    score.clamp(0.0, 1.0)
}

/// Filter documents by quality criteria and deduplicate.
fn filter_documents(
    documents: Vec<Document>,
    criteria: &QualityCriteria,
) -> (Vec<Document>, usize) {
    let mut seen_hashes: HashSet<String> = HashSet::new();
    let mut duplicates_removed = 0;

    let filtered: Vec<Document> = documents
        .into_iter()
        .filter(|doc| {
            // Deduplication
            if criteria.deduplicate {
                if seen_hashes.contains(&doc.text_hash) {
                    duplicates_removed += 1;
                    return false;
                }
                seen_hashes.insert(doc.text_hash.clone());
            }

            // Min words
            if doc.word_count < criteria.min_words {
                return false;
            }

            // Min quality
            if doc.quality_score < criteria.min_quality {
                return false;
            }

            true
        })
        .collect();

    (filtered, duplicates_removed)
}

/// Index a flat directory of files (no author subdirectories).
///
/// All documents are assigned the same author label.
pub fn load_flat(dir: &Path, author: &str) -> Result<Vec<Document>> {
    let mut documents = Vec::new();

    let entries = std::fs::read_dir(dir).map_err(|e| ProvenanceError::IoWithPath {
        path: dir.display().to_string(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry?;
        let file_path = entry.path();
        if !file_path.is_file() {
            continue;
        }

        match load_document(author, &file_path) {
            Ok(doc) => documents.push(doc),
            Err(_) => continue,
        }
    }

    documents.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(documents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_corpus(base: &Path) {
        let alice = base.join("alice");
        let bob = base.join("bob");
        fs::create_dir_all(&alice).unwrap();
        fs::create_dir_all(&bob).unwrap();

        fs::write(
            alice.join("essay1.txt"),
            "Alice wrote this essay about the nature of consciousness. \
             She pondered the deep questions of existence and meaning. \
             The philosophical inquiry continued for many paragraphs, \
             exploring ideas about free will, determinism, and the self. \
             Her arguments were carefully constructed with logical precision.",
        ).unwrap();

        fs::write(
            alice.join("essay2.txt"),
            "In her second work, Alice explored the boundaries of language. \
             Words carry meaning beyond their definitions, she argued. \
             The semiotics of everyday communication fascinated her deeply. \
             She wrote extensively about how context shapes interpretation. \
             This essay was considered her finest academic contribution.",
        ).unwrap();

        fs::write(
            bob.join("story1.txt"),
            "Bob told a tale of adventure and discovery in distant lands. \
             The hero journeyed through mountains and across vast seas. \
             Each chapter brought new challenges and unexpected allies. \
             The narrative built toward a climactic confrontation. \
             Readers praised the vivid descriptions and compelling plot.",
        ).unwrap();

        fs::write(
            bob.join("story2.txt"),
            "Another story by Bob featured a detective in a noir setting. \
             The rain-soaked streets hid secrets at every corner. \
             Clues were scattered throughout the shadowy cityscape. \
             The detective followed each lead with dogged determination. \
             The resolution surprised even the most astute readers.",
        ).unwrap();
    }

    #[test]
    fn test_load_corpus() {
        let dir = std::env::temp_dir().join("provenance_test_corpus");
        let _ = fs::remove_dir_all(&dir);
        create_test_corpus(&dir);

        let criteria = QualityCriteria {
            min_words: 10,
            min_quality: 0.0,
            deduplicate: true,
        };

        let corpus = Corpus::load(&dir, &criteria).unwrap();
        assert_eq!(corpus.summary.n_authors, 2);
        assert_eq!(corpus.summary.total_documents, 4);
        assert!(corpus.summary.total_words > 0);

        let authors = corpus.authors();
        assert_eq!(authors, vec!["alice", "bob"]);

        let alice_docs = corpus.by_author("alice");
        assert_eq!(alice_docs.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_deduplication() {
        let dir = std::env::temp_dir().join("provenance_test_dedup");
        let _ = fs::remove_dir_all(&dir);
        let author_dir = dir.join("writer");
        fs::create_dir_all(&author_dir).unwrap();

        let text = "This is a duplicated text with enough words to pass the minimum threshold. \
                    It contains several sentences for quality scoring purposes. \
                    The quick brown fox jumps over the lazy dog repeatedly.";
        fs::write(author_dir.join("doc1.txt"), text).unwrap();
        fs::write(author_dir.join("doc2.txt"), text).unwrap(); // Exact duplicate

        let criteria = QualityCriteria {
            min_words: 5,
            min_quality: 0.0,
            deduplicate: true,
        };

        let corpus = Corpus::load(&dir, &criteria).unwrap();
        assert_eq!(corpus.summary.total_documents, 1);
        assert_eq!(corpus.summary.duplicates_removed, 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_quality_scoring() {
        // Empty text
        assert_eq!(compute_quality("", 0), 0.0);

        // Very short text
        let short = "Hi.";
        let score = compute_quality(short, 1);
        assert!(score > 0.0 && score < 0.5);

        // Longer, well-formed text
        let good = "The quick brown fox jumps over the lazy dog. \
                    This sentence adds more content. \
                    And this provides even more variety in the text. \
                    We need sufficient words for a good quality score.";
        let good_score = compute_quality(good, good.split_whitespace().count());
        assert!(good_score > score);
    }

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash("hello world");
        let hash2 = compute_hash("hello world");
        let hash3 = compute_hash("different text");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA-256 hex is 64 chars
    }
}
