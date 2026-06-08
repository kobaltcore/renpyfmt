use std::sync::Arc;

use ruff_python_ast::{Alias, Expr, Stmt};
use ruff_python_parser::parse_unchecked_source;
use ruff_text_size::Ranged;
use tower_lsp::lsp_types::{Position, Range};

use crate::index::{
    IndexedSymbolKind, LocationRange, SymbolIndex, SymbolLanguage,
};
use crate::parse::{
    FragmentOrigin, ParseDiagnostic, ParseSeverity, ParsedDocument, PythonFragmentId,
    PythonFragmentKind, PythonFragmentParse,
};
use crate::source::{DocumentLanguage, LineIndex, SourceDocument};

pub fn parse_python_document(source: Arc<SourceDocument>) -> ParsedDocument {
    let origin = FragmentOrigin {
        file: source.path.clone(),
        header_line: 1,
        body_start_line: 1,
        base_indent: 0,
        line_map: build_identity_line_map(&source.text),
    };

    let fragment = parse_python_fragment(
        PythonFragmentId(0),
        PythonFragmentKind::File,
        source.text.clone(),
        origin,
    );

    let mut diagnostics = fragment.diagnostics.clone();
    let mut symbols = SymbolIndex::default();
    index_python_fragment(&source, &fragment, &mut symbols);

    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.range.start.line,
            diagnostic.range.start.character,
            diagnostic.message.clone(),
        )
    });

    ParsedDocument {
        source,
        language: DocumentLanguage::Python,
        renpy: None,
        python: vec![fragment],
        diagnostics,
        symbols,
    }
}

pub fn parse_python_fragment(
    id: PythonFragmentId,
    kind: PythonFragmentKind,
    source: String,
    origin: FragmentOrigin,
) -> PythonFragmentParse {
    let parsed = parse_unchecked_source(&source, ruff_python_ast::PySourceType::Python);
    let diagnostics = parsed
        .errors()
        .iter()
        .map(|error| ParseDiagnostic {
            message: error.to_string(),
            range: origin.map_text_range(&source, error.range()),
            severity: ParseSeverity::Error,
            source: "ruff_python_parser",
        })
        .collect::<Vec<_>>();

    PythonFragmentParse {
        id,
        kind,
        source,
        origin,
        parsed: Some(parsed),
        diagnostics,
    }
}

pub fn index_python_fragment(
    source: &Arc<SourceDocument>,
    fragment: &PythonFragmentParse,
    symbols: &mut SymbolIndex,
) {
    let Some(parsed) = &fragment.parsed else {
        return;
    };

    for stmt in parsed.suite() {
        index_python_stmt(source, fragment, stmt, symbols, true);
    }
}

fn index_python_stmt(
    source: &Arc<SourceDocument>,
    fragment: &PythonFragmentParse,
    stmt: &Stmt,
    symbols: &mut SymbolIndex,
    top_level: bool,
) {
    match stmt {
        Stmt::FunctionDef(node) => {
            let range = fragment
                .origin
                .map_text_range(&fragment.source, node.name.range());
            symbols.push_symbol(
                node.name.as_str(),
                IndexedSymbolKind::Function,
                SymbolLanguage::Python,
                LocationRange {
                    uri: source.file_url().expect("file url"),
                    range,
                },
                Some("python function".into()),
                Some(fragment.id.0),
            );

            for child in &node.body {
                index_python_stmt(source, fragment, child, symbols, false);
            }
        }
        Stmt::ClassDef(node) => {
            let range = fragment
                .origin
                .map_text_range(&fragment.source, node.name.range());
            symbols.push_symbol(
                node.name.as_str(),
                IndexedSymbolKind::Class,
                SymbolLanguage::Python,
                LocationRange {
                    uri: source.file_url().expect("file url"),
                    range,
                },
                Some("python class".into()),
                Some(fragment.id.0),
            );

            for child in &node.body {
                index_python_stmt(source, fragment, child, symbols, false);
            }
        }
        Stmt::Assign(node) if top_level => {
            for target in &node.targets {
                if let Expr::Name(name) = target {
                    let range = fragment
                        .origin
                        .map_text_range(&fragment.source, name.range());
                    symbols.push_symbol(
                        name.id.as_str(),
                        IndexedSymbolKind::Assignment,
                        SymbolLanguage::Python,
                        LocationRange {
                            uri: source.file_url().expect("file url"),
                            range,
                        },
                        Some("python assignment".into()),
                        Some(fragment.id.0),
                    );
                }
            }
        }
        Stmt::AnnAssign(node) if top_level => {
            if let Expr::Name(name) = node.target.as_ref() {
                let range = fragment
                    .origin
                    .map_text_range(&fragment.source, name.range());
                symbols.push_symbol(
                    name.id.as_str(),
                    IndexedSymbolKind::Assignment,
                    SymbolLanguage::Python,
                    LocationRange {
                        uri: source.file_url().expect("file url"),
                        range,
                    },
                    Some("python assignment".into()),
                    Some(fragment.id.0),
                );
            }
        }
        Stmt::Import(node) if top_level => {
            for alias in &node.names {
                push_import_symbol(source, fragment, alias, symbols);
            }
        }
        Stmt::ImportFrom(node) if top_level => {
            for alias in &node.names {
                push_import_symbol(source, fragment, alias, symbols);
            }
        }
        _ => {}
    }
}

fn push_import_symbol(
    source: &Arc<SourceDocument>,
    fragment: &PythonFragmentParse,
    alias: &Alias,
    symbols: &mut SymbolIndex,
) {
    let name = alias
        .asname
        .as_ref()
        .map(|identifier| identifier.as_str())
        .unwrap_or_else(|| alias.name.as_str());
    let range = fragment
        .origin
        .map_text_range(&fragment.source, alias.range());
    symbols.push_symbol(
        name,
        IndexedSymbolKind::Import,
        SymbolLanguage::Python,
        LocationRange {
            uri: source.file_url().expect("file url"),
            range,
        },
        Some("python import".into()),
        Some(fragment.id.0),
    );
}

fn build_identity_line_map(text: &str) -> Vec<usize> {
    let line_count = text.lines().count().max(1);
    (1..=line_count).collect()
}

impl FragmentOrigin {
    pub fn map_text_range(&self, fragment_source: &str, range: ruff_text_size::TextRange) -> Range {
        let index = LineIndex::new(fragment_source);
        let start = index.offset_to_position(range.start().to_usize());
        let end = index.offset_to_position(range.end().to_usize());
        Range::new(
            self.map_position(start, fragment_source),
            self.map_position(end, fragment_source),
        )
    }

    pub fn map_position(&self, position: Position, fragment_source: &str) -> Position {
        let line = usize::try_from(position.line).unwrap_or(0);
        let mapped_line = self
            .line_map
            .get(line)
            .copied()
            .unwrap_or_else(|| self.body_start_line.saturating_add(line));
        let column = if fragment_line_is_blank(fragment_source, line) {
            0
        } else {
            self.base_indent.saturating_add(position.character as usize)
        };
        Position::new(mapped_line.saturating_sub(1) as u32, column as u32)
    }
}

fn fragment_line_is_blank(fragment_source: &str, zero_based_line: usize) -> bool {
    fragment_source
        .lines()
        .nth(zero_based_line)
        .map(|line| line.trim().is_empty())
        .unwrap_or(false)
}
