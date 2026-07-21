// SPDX-License-Identifier: Apache-2.0
use criterion::{criterion_group, criterion_main, Criterion};
use nameparser::regexes;
use nameparser::token::tokenize;
// criterion 0.8 deprecates its own re-export in favour of the std one.
use std::hint::black_box;

fn load_corpus() -> Vec<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/benchmark-data.txt"
    );
    std::fs::read_to_string(path)
        .expect("benchmark-data.txt missing — run Task 3 Step 1")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split('\t').next().unwrap().to_string())
        .collect()
}

fn bench(c: &mut Criterion) {
    let names = load_corpus();
    let count = names.len();
    eprintln!("corpus: {count} names (divide the reported time by {count} for µs/name)");

    c.bench_function("tokenize_corpus", |b| {
        b.iter(|| {
            for n in &names {
                black_box(tokenize(black_box(n)));
            }
        })
    });

    c.bench_function("regex_batch_corpus", |b| {
        b.iter(|| {
            for n in &names {
                black_box(regexes::SIC.replace_all(n, ""));
                black_box(regexes::AGGREGATE.replace_all(n, ""));
                black_box(regexes::TAX_NOTE.replace_all(n, ""));
                black_box(regexes::PUBLISHED_PAGE.replace_all(n, ""));
            }
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
