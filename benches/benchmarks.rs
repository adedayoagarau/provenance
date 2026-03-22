use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use provenance::analysis::{self, function_words, lexical, ngrams, semantic, stylometric, syntactic};
use provenance::identity::delta;
use provenance::identity::features::{self, CorpusStats, FeatureSet, FeatureVector};

/// Generate sample text of a given approximate word count.
fn sample_text(words: usize) -> String {
    let sentences = [
        "The quick brown fox jumps over the lazy dog near the river bank. ",
        "She walked carefully through the ancient forest, listening to birdsong. ",
        "Perhaps the most important thing is that we never stop questioning. ",
        "However, it seems unlikely that anyone would disagree with this view. ",
        "The committee decided to postpone the vote until further notice. ",
        "In conclusion, the evidence strongly suggests a different interpretation. ",
    ];
    let mut text = String::new();
    let mut word_count = 0;
    let mut i = 0;
    while word_count < words {
        text.push_str(sentences[i % sentences.len()]);
        word_count += sentences[i % sentences.len()].split_whitespace().count();
        i += 1;
    }
    text
}

// ---------------------------------------------------------------------------
// Per-module benchmarks
// ---------------------------------------------------------------------------

fn bench_lexical_analysis(c: &mut Criterion) {
    let text = sample_text(1000);
    c.bench_function("analysis/lexical", |b| {
        b.iter(|| lexical::analyze(black_box(&text)))
    });
}

fn bench_syntactic_analysis(c: &mut Criterion) {
    let text = sample_text(1000);
    c.bench_function("analysis/syntactic", |b| {
        b.iter(|| syntactic::analyze(black_box(&text)))
    });
}

fn bench_semantic_analysis(c: &mut Criterion) {
    let text = sample_text(1000);
    c.bench_function("analysis/semantic", |b| {
        b.iter(|| semantic::analyze(black_box(&text)))
    });
}

fn bench_stylometric_analysis(c: &mut Criterion) {
    let text = sample_text(1000);
    c.bench_function("analysis/stylometric", |b| {
        b.iter(|| stylometric::analyze(black_box(&text)))
    });
}

fn bench_function_word_analysis(c: &mut Criterion) {
    let text = sample_text(1000);
    c.bench_function("analysis/function_words", |b| {
        b.iter(|| function_words::analyze(black_box(&text)))
    });
}

fn bench_ngram_analysis(c: &mut Criterion) {
    let text = sample_text(1000);
    c.bench_function("analysis/ngrams", |b| {
        b.iter(|| ngrams::analyze(black_box(&text)))
    });
}

// ---------------------------------------------------------------------------
// Full pipeline benchmark (analyze_text runs all 6 analyses in parallel)
// ---------------------------------------------------------------------------

fn bench_analyze_text(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline/analyze_text");
    for &word_count in &[100, 500, 1000, 5000] {
        let text = sample_text(word_count);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{word_count}_words")),
            &text,
            |b, text| b.iter(|| analysis::analyze_text(black_box(text))),
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Feature extraction benchmarks
// ---------------------------------------------------------------------------

fn bench_feature_extraction(c: &mut Criterion) {
    let text = sample_text(1000);
    let result = analysis::analyze_text(&text).unwrap();

    let mut group = c.benchmark_group("features/extract");
    for &(name, set) in &[
        ("minimal", FeatureSet::Minimal),
        ("standard", FeatureSet::Standard),
        ("comprehensive", FeatureSet::Comprehensive),
    ] {
        group.bench_function(name, |b| {
            b.iter(|| features::extract(black_box(&result), set))
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Delta computation benchmarks
// ---------------------------------------------------------------------------

fn bench_delta(c: &mut Criterion) {
    let text_a = sample_text(1000);
    let text_b = sample_text(800);
    let result_a = analysis::analyze_text(&text_a).unwrap();
    let result_b = analysis::analyze_text(&text_b).unwrap();
    let fv_a = features::extract(&result_a, FeatureSet::Standard);
    let fv_b = features::extract(&result_b, FeatureSet::Standard);

    c.bench_function("delta/from_vectors", |b| {
        b.iter(|| delta::delta_from_vectors(black_box(&fv_a), black_box(&fv_b)))
    });
}

fn bench_rank_candidates(c: &mut Criterion) {
    let text = sample_text(1000);
    let result = analysis::analyze_text(&text).unwrap();
    let query = features::extract(&result, FeatureSet::Standard);

    // Build 10 candidate feature vectors
    let candidates: Vec<(String, FeatureVector)> = (0..10)
        .map(|i| {
            let t = sample_text(800 + i * 50);
            let r = analysis::analyze_text(&t).unwrap();
            (format!("author_{i}"), features::extract(&r, FeatureSet::Standard))
        })
        .collect();

    let all_vecs: Vec<FeatureVector> = std::iter::once(query.clone())
        .chain(candidates.iter().map(|(_, v)| v.clone()))
        .collect();
    let stats = CorpusStats::from_vectors(&all_vecs).unwrap();

    c.bench_function("delta/rank_10_candidates", |b| {
        b.iter(|| delta::rank_candidates(black_box(&query), black_box(&candidates), black_box(&stats)))
    });
}

// ---------------------------------------------------------------------------
// Memory usage tracking benchmark — measures allocation size via text length
// ---------------------------------------------------------------------------

fn bench_memory_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/feature_vector_size");
    for &word_count in &[100, 500, 2000, 10000] {
        let text = sample_text(word_count);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{word_count}_words")),
            &text,
            |b, text| {
                b.iter(|| {
                    let result = analysis::analyze_text(black_box(text)).unwrap();
                    let fv = features::extract(&result, FeatureSet::Comprehensive);
                    // Return size to prevent optimization
                    (fv.names.len(), fv.values.len())
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    // Per-module
    bench_lexical_analysis,
    bench_syntactic_analysis,
    bench_semantic_analysis,
    bench_stylometric_analysis,
    bench_function_word_analysis,
    bench_ngram_analysis,
    // Full pipeline
    bench_analyze_text,
    // Feature extraction
    bench_feature_extraction,
    // Delta
    bench_delta,
    bench_rank_candidates,
    // Memory scaling
    bench_memory_scaling,
);
criterion_main!(benches);
