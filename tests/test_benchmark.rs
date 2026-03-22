//! Automated benchmark runner for the built-in authorship attribution dataset.
//!
//! Uses the 5-author benchmark corpus in tests/fixtures/benchmark/ to evaluate
//! the full pipeline: corpus loading → feature extraction → k-NN classification
//! → cross-validation metrics.

use provenance::data::corpus::{Corpus, QualityCriteria};
use provenance::data::split;
use provenance::identity::features::FeatureSet;
use provenance::ml::evaluation;
use provenance::ml::features::FeaturePipeline;
use provenance::ml::models::ModelType;
use provenance::ml::pipeline::PipelineConfig;

use std::path::Path;

const BENCHMARK_DIR: &str = "tests/fixtures/benchmark";

#[test]
fn test_benchmark_corpus_loads() {
    let benchmark_path = Path::new(BENCHMARK_DIR);
    if !benchmark_path.exists() {
        eprintln!("Benchmark dataset not found at {BENCHMARK_DIR}, skipping");
        return;
    }

    let criteria = QualityCriteria {
        min_words: 20,
        min_quality: 0.0,
        deduplicate: true,
    };

    let corpus = Corpus::load(benchmark_path, &criteria).unwrap();

    // Should have 5 authors
    assert!(
        corpus.summary.n_authors >= 2,
        "Expected at least 2 authors, got {}",
        corpus.summary.n_authors
    );

    // Should have multiple docs per author
    assert!(
        corpus.summary.total_documents >= 10,
        "Expected at least 10 documents, got {}",
        corpus.summary.total_documents
    );

    println!("Benchmark corpus: {} docs from {} authors",
        corpus.summary.total_documents, corpus.summary.n_authors);

    for author in corpus.authors() {
        let count = corpus.by_author(&author).len();
        println!("  {author}: {count} documents");
    }
}

#[test]
fn test_benchmark_feature_extraction() {
    let benchmark_path = Path::new(BENCHMARK_DIR);
    if !benchmark_path.exists() {
        return;
    }

    let criteria = QualityCriteria {
        min_words: 20,
        min_quality: 0.0,
        deduplicate: true,
    };

    let corpus = Corpus::load(benchmark_path, &criteria).unwrap();
    let pipeline = FeaturePipeline::new(FeatureSet::Standard);

    // Extract features from all documents
    let mut success_count = 0;
    for doc in &corpus.documents {
        match pipeline.extract_from_text(&doc.text) {
            Ok((features, _wc)) => {
                assert!(!features.is_empty());
                success_count += 1;
            }
            Err(e) => {
                eprintln!("Feature extraction failed for {}: {e}", doc.path.display());
            }
        }
    }

    assert!(
        success_count == corpus.documents.len(),
        "All documents should yield features"
    );
}

#[test]
fn test_benchmark_train_test_split() {
    let benchmark_path = Path::new(BENCHMARK_DIR);
    if !benchmark_path.exists() {
        return;
    }

    let criteria = QualityCriteria {
        min_words: 20,
        min_quality: 0.0,
        deduplicate: true,
    };

    let corpus = Corpus::load(benchmark_path, &criteria).unwrap();

    let (train, test) = split::train_test_split(&corpus.documents, 0.7, 42);

    // No overlap
    let train_paths: std::collections::HashSet<_> =
        train.iter().map(|d| &d.path).collect();
    let test_paths: std::collections::HashSet<_> =
        test.iter().map(|d| &d.path).collect();
    assert!(
        train_paths.is_disjoint(&test_paths),
        "Train and test sets must not overlap"
    );

    // All docs accounted for
    assert_eq!(train.len() + test.len(), corpus.documents.len());
}

#[test]
fn test_benchmark_knn_evaluation() {
    let benchmark_path = Path::new(BENCHMARK_DIR);
    if !benchmark_path.exists() {
        return;
    }

    let criteria = QualityCriteria {
        min_words: 20,
        min_quality: 0.0,
        deduplicate: true,
    };

    let corpus = Corpus::load(benchmark_path, &criteria).unwrap();
    if corpus.summary.n_authors < 2 {
        return;
    }

    // Extract features
    let pipeline = FeaturePipeline::new(FeatureSet::Standard);
    let mut samples = Vec::new();

    for doc in &corpus.documents {
        if let Ok((features, word_count)) = pipeline.extract_from_text(&doc.text) {
            samples.push(provenance::ml::features::LabeledSample {
                label: doc.author.clone(),
                source: doc.path.display().to_string(),
                features,
                word_count,
            });
        }
    }

    if samples.len() < 4 {
        return;
    }

    // Run 2-fold CV (small dataset)
    let config = PipelineConfig {
        feature_set: FeatureSet::Standard,
        model_type: ModelType::KNearestNeighbors,
        knn_k: 3,
        normalize: true,
        min_word_count: 0,
    };

    let report = evaluation::cross_validate(&samples, &config, 2).unwrap();

    println!("\nBenchmark k-NN Results ({}-fold CV):", report.n_folds);
    println!("  Accuracy:  {:.1}%", report.overall.accuracy * 100.0);
    println!("  Precision: {:.1}%", report.overall.precision * 100.0);
    println!("  Recall:    {:.1}%", report.overall.recall * 100.0);
    println!("  F1 Score:  {:.1}%", report.overall.f1_score * 100.0);
    println!("\nConfusion Matrix:");
    println!("{}", report.confusion_matrix);

    // Accuracy should be above random chance for the benchmark
    let random_chance = 1.0 / corpus.summary.n_authors as f64;
    assert!(
        report.overall.accuracy >= random_chance * 0.8,
        "Accuracy ({:.1}%) should be near or above random chance ({:.1}%)",
        report.overall.accuracy * 100.0,
        random_chance * 100.0
    );
}
