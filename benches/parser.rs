mod support;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use renpyfmt::lexer::Lexer;
use renpyfmt::parser::parse_block;
use renpyfmt::project::parse_file_ast;
use support::{PARSE_FIXTURES, grouped_fixture};

fn bench_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    for fixture in PARSE_FIXTURES {
        let blocks = grouped_fixture(fixture);
        group.bench_with_input(
            BenchmarkId::new("parse_from_blocks", fixture),
            fixture,
            |b, _| {
                b.iter(|| {
                    let mut lex = Lexer::new(blocks.clone());
                    let _ = parse_block(&mut lex).expect("fixture should parse");
                });
            },
        );

        let root = support::fixtures_dir();
        let path = support::fixture_path(fixture);
        group.bench_with_input(
            BenchmarkId::new("parse_file_ast", fixture),
            fixture,
            |b, _| {
                b.iter(|| {
                    let _ = parse_file_ast(&root, &path).expect("fixture should parse");
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_parser);
criterion_main!(benches);
