//! Train/validation/test split utilities for corpus data.
//!
//! Provides stratified splitting to maintain author class proportions
//! across all splits.

use std::collections::HashMap;

use super::corpus::Document;

/// Configuration for data splitting.
#[derive(Debug, Clone)]
pub struct SplitConfig {
    /// Fraction for training set (e.g., 0.7)
    pub train_ratio: f64,
    /// Fraction for validation set (e.g., 0.15)
    pub val_ratio: f64,
    /// Fraction for test set (e.g., 0.15)
    pub test_ratio: f64,
    /// Random seed for reproducibility
    pub seed: u64,
}

impl Default for SplitConfig {
    fn default() -> Self {
        Self {
            train_ratio: 0.7,
            val_ratio: 0.15,
            test_ratio: 0.15,
            seed: 42,
        }
    }
}

/// Result of splitting a corpus into train/validation/test sets.
#[derive(Debug, Clone)]
pub struct DataSplit<'a> {
    pub train: Vec<&'a Document>,
    pub validation: Vec<&'a Document>,
    pub test: Vec<&'a Document>,
}

/// Split documents into train/validation/test sets with stratification.
///
/// Maintains approximately equal author proportions across all splits.
/// Uses a deterministic shuffling based on the seed.
pub fn stratified_split<'a>(
    documents: &'a [Document],
    config: &SplitConfig,
) -> DataSplit<'a> {
    // Group documents by author
    let mut by_author: HashMap<&str, Vec<&'a Document>> = HashMap::new();
    for doc in documents {
        by_author.entry(&doc.author).or_default().push(doc);
    }

    let mut train = Vec::new();
    let mut validation = Vec::new();
    let mut test = Vec::new();

    // Split each author's documents proportionally
    for (_author, docs) in &mut by_author {
        // Deterministic shuffle using seed-based permutation
        let mut indices: Vec<usize> = (0..docs.len()).collect();
        deterministic_shuffle(&mut indices, config.seed);

        let n = docs.len();
        let n_train = ((n as f64) * config.train_ratio).round() as usize;
        let n_val = ((n as f64) * config.val_ratio).round() as usize;

        for (i, &idx) in indices.iter().enumerate() {
            if i < n_train {
                train.push(docs[idx]);
            } else if i < n_train + n_val {
                validation.push(docs[idx]);
            } else {
                test.push(docs[idx]);
            }
        }
    }

    // Sort within each split for determinism
    train.sort_by(|a, b| a.path.cmp(&b.path));
    validation.sort_by(|a, b| a.path.cmp(&b.path));
    test.sort_by(|a, b| a.path.cmp(&b.path));

    DataSplit {
        train,
        validation,
        test,
    }
}

/// Split documents into train/test only (no validation set).
pub fn train_test_split<'a>(
    documents: &'a [Document],
    train_ratio: f64,
    seed: u64,
) -> (Vec<&'a Document>, Vec<&'a Document>) {
    let config = SplitConfig {
        train_ratio,
        val_ratio: 0.0,
        test_ratio: 1.0 - train_ratio,
        seed,
    };
    let split = stratified_split(documents, &config);
    // Merge validation into test since val_ratio is 0
    let mut test = split.test;
    test.extend(split.validation);
    test.sort_by(|a, b| a.path.cmp(&b.path));
    (split.train, test)
}

/// Deterministic shuffle using a simple LCG PRNG.
fn deterministic_shuffle(indices: &mut [usize], seed: u64) {
    let n = indices.len();
    if n <= 1 {
        return;
    }

    // LCG parameters (Numerical Recipes)
    let mut state = seed;
    for i in (1..n).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (state >> 33) as usize % (i + 1);
        indices.swap(i, j);
    }
}

/// Compute split statistics for reporting.
pub fn split_stats(split: &DataSplit) -> HashMap<String, (usize, usize, usize)> {
    let mut stats: HashMap<String, (usize, usize, usize)> = HashMap::new();

    for doc in &split.train {
        stats.entry(doc.author.clone()).or_default().0 += 1;
    }
    for doc in &split.validation {
        stats.entry(doc.author.clone()).or_default().1 += 1;
    }
    for doc in &split.test {
        stats.entry(doc.author.clone()).or_default().2 += 1;
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_doc(author: &str, idx: usize) -> Document {
        Document {
            author: author.to_string(),
            path: PathBuf::from(format!("{author}_{idx}.txt")),
            text: format!("Sample text for {author} document {idx}"),
            word_count: 500,
            text_hash: format!("hash_{author}_{idx}"),
            quality_score: 0.8,
        }
    }

    #[test]
    fn test_stratified_split_proportions() {
        // 10 docs per author, 2 authors
        let docs: Vec<Document> = (0..10)
            .map(|i| make_doc("alice", i))
            .chain((0..10).map(|i| make_doc("bob", i)))
            .collect();

        let config = SplitConfig {
            train_ratio: 0.7,
            val_ratio: 0.15,
            test_ratio: 0.15,
            seed: 42,
        };

        let split = stratified_split(&docs, &config);

        // With 10 docs per author and 70/15/15 split:
        // train=7, val=2, test=1 per author (rounding)
        assert_eq!(split.train.len(), 14); // 7 + 7
        assert!(split.validation.len() + split.test.len() == 6); // rest

        // Check both authors are in train
        let train_authors: std::collections::HashSet<&str> =
            split.train.iter().map(|d| d.author.as_str()).collect();
        assert!(train_authors.contains("alice"));
        assert!(train_authors.contains("bob"));
    }

    #[test]
    fn test_deterministic_split() {
        let docs: Vec<Document> = (0..6)
            .map(|i| make_doc("alice", i))
            .chain((0..6).map(|i| make_doc("bob", i)))
            .collect();

        let config = SplitConfig::default();

        let split1 = stratified_split(&docs, &config);
        let split2 = stratified_split(&docs, &config);

        // Same seed → same split
        let paths1: Vec<&PathBuf> = split1.train.iter().map(|d| &d.path).collect();
        let paths2: Vec<&PathBuf> = split2.train.iter().map(|d| &d.path).collect();
        assert_eq!(paths1, paths2);
    }

    #[test]
    fn test_train_test_split() {
        let docs: Vec<Document> = (0..10)
            .map(|i| make_doc("author", i))
            .collect();

        let (train, test) = train_test_split(&docs, 0.8, 42);

        assert_eq!(train.len(), 8);
        assert_eq!(test.len(), 2);
        assert_eq!(train.len() + test.len(), 10);
    }

    #[test]
    fn test_split_stats() {
        let docs: Vec<Document> = (0..6)
            .map(|i| make_doc("alice", i))
            .chain((0..4).map(|i| make_doc("bob", i)))
            .collect();

        let config = SplitConfig::default();
        let split = stratified_split(&docs, &config);
        let stats = split_stats(&split);

        // Both authors should appear in stats
        assert!(stats.contains_key("alice"));
        assert!(stats.contains_key("bob"));
    }
}
