![](docs/banner.jpg "renpyfmt logo")

# renpyfmt

> <picture>
>   <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/Mqxx/GitHub-Markdown/main/blockquotes/badge/light-theme/danger.svg">
>   <img alt="Danger" src="https://raw.githubusercontent.com/Mqxx/GitHub-Markdown/main/blockquotes/badge/dark-theme/danger.svg">
> </picture><br>
>
> This is an early alpha version of the code!
> Things WILL break, the API WILL change and the code desperately needs to be cleaned up.
>
> Right now the code is purely a proof of concept, so please keep that in mind if you intend to make use of it in any way.

`renpyfmt` is an opinionated source code formatter for Ren'Py script.

It ships with a complete, standalone parser for the Ren'Py language, allowing for deep understanding of the code and thus proper formatting. Embedded Python blocks are formatted via `ruff`.

## Benchmarks

Run the full benchmark suite with:

```sh
cargo bench
```

The benchmark targets are split by phase:

- `logical_lines`: logical-line scanning and grouping.
- `parser`: parse from fixture files and pre-grouped blocks.
- `formatter`: Ren'Py AST formatting and embedded Python formatting.
- `end_to_end`: full `.rpy` format/check flows on representative fixtures.

To compare before and after a change, run `cargo bench` on each revision and compare the Criterion reports under `target/criterion/`.

Correctness gates stay separate from performance measurement:

- `cargo test`
- `cargo tarpaulin`

<a href="https://unsplash.com/photos/E8Ufcyxz514?utm_source=unsplash&utm_medium=referral&utm_content=creditShareLink">Photo by Milad Fakurian on Unsplash</a>
