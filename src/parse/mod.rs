use std::path::PathBuf;
use std::sync::Arc;

use crate::ast::AstNode;
use crate::comments::CommentMap;
use crate::index::SymbolIndex;
use crate::source::{DocumentLanguage, SourceDocument};
use tower_lsp::lsp_types::Range;

pub mod python;
pub mod renpy;

pub use renpy::{AstVisitor, LogicalLine, VisitContext, walk_ast};

#[derive(Debug, Clone, Default)]
pub struct ParseOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSeverity {
    Error,
    Warning,
    Information,
}

#[derive(Debug, Clone)]
pub struct ParseDiagnostic {
    pub message: String,
    pub range: Range,
    pub severity: ParseSeverity,
    pub source: &'static str,
}

#[derive(Debug, Clone)]
pub struct RenpyParse {
    pub ast: Vec<AstNode>,
    pub comments: CommentMap,
    pub logical_lines: Vec<LogicalLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PythonFragmentId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonFragmentKind {
    File,
    Block,
    EarlyBlock,
    ScreenBlock,
    OneLine,
}

#[derive(Debug, Clone)]
pub struct FragmentOrigin {
    pub file: PathBuf,
    pub header_line: usize,
    pub body_start_line: usize,
    pub base_indent: usize,
    pub line_map: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct PythonFragmentParse {
    pub id: PythonFragmentId,
    pub kind: PythonFragmentKind,
    pub source: String,
    pub origin: FragmentOrigin,
    pub parsed: Option<ruff_python_parser::Parsed<ruff_python_ast::ModModule>>,
    pub diagnostics: Vec<ParseDiagnostic>,
}

#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub source: Arc<SourceDocument>,
    pub language: DocumentLanguage,
    pub renpy: Option<RenpyParse>,
    pub python: Vec<PythonFragmentParse>,
    pub diagnostics: Vec<ParseDiagnostic>,
    pub symbols: SymbolIndex,
}

pub fn parse_document(source: SourceDocument, _options: ParseOptions) -> ParsedDocument {
    let source = Arc::new(source);
    match source.language() {
        DocumentLanguage::Renpy => renpy::parse_renpy_document(source),
        DocumentLanguage::Python => python::parse_python_document(source),
        DocumentLanguage::Unknown => ParsedDocument {
            source: source.clone(),
            language: DocumentLanguage::Unknown,
            renpy: None,
            python: Vec::new(),
            diagnostics: Vec::new(),
            symbols: SymbolIndex::default(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tower_lsp::lsp_types::{Position, Url};

    use crate::source::SourceDocument;

    use super::{ParseOptions, parse_document};

    #[test]
    fn parses_in_memory_renpy_document_and_extracts_python() {
        let path = PathBuf::from("/tmp/script.rpy");
        let source = SourceDocument::new(
            Url::from_file_path(&path).ok(),
            path,
            "label start:\n    python:\n        def helper():\n            return 1\n".into(),
        );

        let parsed = parse_document(source, ParseOptions);

        assert!(parsed.renpy.is_some());
        assert_eq!(parsed.python.len(), 1);
        assert_eq!(parsed.python[0].origin.header_line, 2);
        assert_eq!(parsed.python[0].origin.body_start_line, 3);
        assert_eq!(parsed.python[0].origin.line_map[0], 2);
        assert!(parsed
            .symbols
            .symbols
            .iter()
            .any(|symbol| symbol.name == "start"));
        assert!(parsed
            .symbols
            .symbols
            .iter()
            .any(|symbol| symbol.name == "helper"));
    }

    #[test]
    fn maps_embedded_python_diagnostics_back_to_renpy_lines() {
        let path = PathBuf::from("/tmp/bad_script.rpy");
        let source = SourceDocument::new(
            Url::from_file_path(&path).ok(),
            path,
            "label start:\n    $ value =\n".into(),
        );

        let parsed = parse_document(source, ParseOptions);
        let diagnostic = parsed
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.source == "ruff_python_parser")
            .expect("python diagnostic");

        assert_eq!(diagnostic.range.start.line, 1);
    }

    #[test]
    fn indexes_python_file_symbols() {
        let path = PathBuf::from("/tmp/module.py");
        let source = SourceDocument::new(
            Url::from_file_path(&path).ok(),
            path,
            "import os\nVALUE = 1\nclass Greeter:\n    pass\n\ndef greet():\n    return VALUE\n".into(),
        );

        let parsed = parse_document(source, ParseOptions);
        let names = parsed
            .symbols
            .symbols
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"os"));
        assert!(names.contains(&"VALUE"));
        assert!(names.contains(&"Greeter"));
        assert!(names.contains(&"greet"));
    }

    #[test]
    fn resolves_renpy_label_definition() {
        let path = PathBuf::from("/tmp/labels.rpy");
        let uri = Url::from_file_path(&path).ok().unwrap();
        let text = "label start:\n    jump elsewhere\n\nlabel elsewhere:\n    pass\n";
        let source = SourceDocument::new(Some(uri.clone()), path, text.into());
        let parsed = parse_document(source, ParseOptions);

        let definition = parsed
            .symbols
            .goto_definition(&uri, Position::new(1, 10))
            .expect("definition");

        assert_eq!(definition.range.start.line, 3);
    }
}
