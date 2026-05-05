mod support;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use renpyfmt::project::format_file_source;
use std::fs;
use support::{
    RPY_FIXTURES, copy_fixture, create_temp_fixture_dir, fixture_path, format_fixture,
    python_config,
};

fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");
    let config = python_config();
    let root = support::fixtures_dir();

    for fixture in RPY_FIXTURES {
        let path = fixture_path(fixture);
        let original = fs::read_to_string(&path).expect("fixture should load");

        group.bench_with_input(BenchmarkId::new("format", fixture), fixture, |b, _| {
            b.iter(|| {
                let _ = format_file_source(&root, &path, &config).expect("fixture should format");
            });
        });

        group.bench_with_input(BenchmarkId::new("check", fixture), fixture, |b, _| {
            b.iter(|| {
                let _ = format_fixture(fixture) == original;
            });
        });
    }

    let temp_root = create_temp_fixture_dir();
    let temp_fixture = copy_fixture("embedded_python.rpy", &temp_root);
    group.bench_function("format_temp_copy", |b| {
        b.iter(|| {
            let _ = format_file_source(&temp_root, &temp_fixture, &config)
                .expect("temp fixture should format");
        });
    });

    group.finish();
}

criterion_group!(benches, bench_end_to_end);
criterion_main!(benches);
