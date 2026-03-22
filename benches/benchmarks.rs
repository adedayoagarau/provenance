use criterion::{black_box, criterion_group, criterion_main, Criterion};
use provenance::analysis::lexical;

fn bench_lexical_analysis(c: &mut Criterion) {
    let text = "The quick brown fox jumps over the lazy dog. ".repeat(100);

    c.bench_function("lexical_analysis", |b| {
        b.iter(|| lexical::analyze(black_box(&text)))
    });
}

criterion_group!(benches, bench_lexical_analysis);
criterion_main!(benches);
