mod support;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use renpyfmt::formatter::{format_ast_with_config, format_python_block};
use std::fs;
use support::{RPY_FIXTURES, fixture_path, parse_fixture, python_config};

fn bench_formatter(c: &mut Criterion) {
    let mut group = c.benchmark_group("formatter");
    let config = python_config();

    for fixture in RPY_FIXTURES {
        let (ast, comments) = parse_fixture(fixture);
        group.bench_with_input(BenchmarkId::new("format_ast", fixture), fixture, |b, _| {
            b.iter(|| {
                let _ = format_ast_with_config(&ast, &comments, &config);
            });
        });
    }

    let python_source = fs::read_to_string(fixture_path("embedded_python_block.pyfrag"))
        .expect("python fixture should load");
    group.bench_function("ruff_python_block", |b| {
        b.iter(|| {
            let _ = format_python_block(&python_source, &config);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_formatter);
criterion_main!(benches);
