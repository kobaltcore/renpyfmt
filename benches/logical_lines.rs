mod support;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use renpyfmt::project::group_logical_lines;
use support::{LOGICAL_FIXTURES, grouped_fixture, logical_lines_fixture};

fn bench_logical_lines(c: &mut Criterion) {
    let mut group = c.benchmark_group("logical_lines");

    for fixture in LOGICAL_FIXTURES {
        group.bench_with_input(
            BenchmarkId::new("list_logical_lines", fixture),
            fixture,
            |b, name| {
                b.iter(|| {
                    let _ = logical_lines_fixture(name);
                });
            },
        );

        let lines = logical_lines_fixture(fixture).0;
        group.bench_with_input(
            BenchmarkId::new("group_logical_lines", fixture),
            fixture,
            |b, _| {
                b.iter(|| {
                    let _ = group_logical_lines(lines.clone()).expect("fixture should group");
                });
            },
        );

        let _ = grouped_fixture(fixture);
    }

    group.finish();
}

criterion_group!(benches, bench_logical_lines);
criterion_main!(benches);
