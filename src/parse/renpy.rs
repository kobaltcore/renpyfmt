use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, bail};

use crate::ast::{AstNode, Call, Hide, ImageSpecifier, Jump, Python, PythonOneLine, Scene, Screen, ScreenStatement, Style, Transform};
use crate::comments::{Comment, CommentMap, EOF_LINE};
use crate::index::{
    IndexedReferenceKind, IndexedSymbolKind, LocationRange, SymbolIndex, SymbolLanguage,
};
use crate::lexer::{Block, Lexer};
use crate::parse::python::{index_python_fragment, parse_python_fragment};
use crate::parse::{
    FragmentOrigin, ParseDiagnostic, ParseSeverity, ParsedDocument, PythonFragmentId,
    PythonFragmentKind, PythonFragmentParse, RenpyParse,
};
use crate::parser::parse_block;
use crate::slast;
use crate::source::{DocumentLanguage, SourceDocument};
use crate::testast::{TestNode, TestSuiteEntry};

#[derive(Debug, Clone)]
pub struct LogicalLine {
    pub path: PathBuf,
    pub line_number: usize,
    pub text: String,
    pub indent: usize,
    pub range: Option<tower_lsp::lsp_types::Range>,
}

#[derive(Clone, Copy, Debug)]
pub struct VisitContext {
    pub line_number: usize,
}

pub trait AstVisitor {
    fn visit_node(&mut self, _node: &AstNode, _ctx: VisitContext) {}
    fn visit_screen_node(&mut self, _node: &slast::Node, _ctx: VisitContext) {}
}

pub fn walk_ast(nodes: &[AstNode], visitor: &mut dyn AstVisitor) {
    for node in nodes {
        walk_node(node, visitor);
    }
}

pub fn parse_renpy_document(source: Arc<SourceDocument>) -> ParsedDocument {
    let mut diagnostics = Vec::new();
    let mut renpy = None;

    match parse_renpy(&source) {
        Ok(parsed) => renpy = Some(parsed),
        Err(err) => diagnostics.push(ParseDiagnostic {
            message: err.to_string(),
            range: source.line_index.one_based_line_range(extract_error_line(&err).unwrap_or(1)),
            severity: ParseSeverity::Error,
            source: "renpy_parser",
        }),
    }

    let mut python = Vec::new();
    if let Some(renpy_parse) = &renpy {
        python = extract_python_fragments(&source, renpy_parse);
        diagnostics.extend(
            python
                .iter()
                .flat_map(|fragment| fragment.diagnostics.clone())
                .collect::<Vec<_>>(),
        );
    }

    let mut symbols = SymbolIndex::default();
    if let Some(renpy_parse) = &renpy {
        index_renpy(&source, renpy_parse, &mut symbols);
    }
    for fragment in &python {
        index_python_fragment(&source, fragment, &mut symbols);
    }

    ParsedDocument {
        source,
        language: DocumentLanguage::Renpy,
        renpy,
        python,
        diagnostics,
        symbols,
    }
}

fn parse_renpy(source: &Arc<SourceDocument>) -> Result<RenpyParse> {
    let (lines, comments) = list_logical_lines_from_source(source)?;
    let grouped = group_logical_lines(
        lines.iter()
            .map(|line| (line.path.clone(), line.line_number, line.text.clone()))
            .collect(),
    )?;
    let mut lexer = Lexer::new(grouped);
    let ast = parse_block(&mut lexer).map_err(anyhow::Error::from)?;
    Ok(RenpyParse {
        ast,
        comments,
        logical_lines: lines,
    })
}

fn extract_error_line(err: &anyhow::Error) -> Option<usize> {
    err.chain()
        .find_map(|cause| cause.downcast_ref::<crate::error::ParseError>())
        .and_then(|parse_error| parse_error.location.as_ref().map(|(_, line)| *line))
}

fn extract_python_fragments(
    source: &Arc<SourceDocument>,
    renpy: &RenpyParse,
) -> Vec<PythonFragmentParse> {
    let mut fragments = Vec::new();
    let mut next_id = 0usize;

    fn push_block_fragment(
        fragments: &mut Vec<PythonFragmentParse>,
        source: &Arc<SourceDocument>,
        next_id: &mut usize,
        kind: PythonFragmentKind,
        header_line: usize,
        text: &str,
    ) {
        let line_count = text.lines().count().max(1);
        let base_indent = indent_of_line(&source.text, header_line + 1);
        let origin = FragmentOrigin {
            file: source.path.clone(),
            header_line,
            body_start_line: header_line + 1,
            base_indent,
            line_map: (0..line_count).map(|i| header_line + i).collect(),
        };
        fragments.push(parse_python_fragment(
            PythonFragmentId(*next_id),
            kind,
            text.to_string(),
            origin,
        ));
        *next_id += 1;
    }

    fn collect_from_nodes(
        source: &Arc<SourceDocument>,
        nodes: &[AstNode],
        fragments: &mut Vec<PythonFragmentParse>,
        next_id: &mut usize,
    ) {
        for node in nodes {
            match node {
                AstNode::Label(node) => collect_from_nodes(source, &node.block, fragments, next_id),
                AstNode::Menu(node) => {
                    for (_, _, block) in &node.items {
                        if let Some(block) = block {
                            collect_from_nodes(source, block, fragments, next_id);
                        }
                    }
                }
                AstNode::If(node) => {
                    for (_, block) in &node.entries {
                        collect_from_nodes(source, block, fragments, next_id);
                    }
                }
                AstNode::CompileIf(node) => {
                    for (_, block) in &node.entries {
                        collect_from_nodes(source, block, fragments, next_id);
                    }
                }
                AstNode::While(node) => collect_from_nodes(source, &node.block, fragments, next_id),
                AstNode::Init(node) => collect_from_nodes(source, &node.block, fragments, next_id),
                AstNode::Translate(node) => collect_from_nodes(source, &node.block, fragments, next_id),
                AstNode::TranslateBlock(node) => collect_from_nodes(source, &node.block, fragments, next_id),
                AstNode::TranslateEarlyBlock(node) => collect_from_nodes(source, &node.block, fragments, next_id),
                AstNode::Screen(Screen { screen, .. }) => {
                    extract_screen_python(source, screen, fragments, next_id);
                }
                AstNode::Testcase(testcase) => {
                    extract_test_python_nodes(source, &testcase.test.statements, fragments, next_id);
                }
                AstNode::Testsuite(testsuite) => {
                    extract_suite_python(source, &testsuite.suite.entries, fragments, next_id);
                }
                AstNode::Python(Python { loc, python_code, .. }) => {
                    push_block_fragment(
                        fragments,
                        source,
                        next_id,
                        PythonFragmentKind::Block,
                        loc.1,
                        python_code,
                    );
                }
                AstNode::EarlyPython(crate::ast::EarlyPython { loc, python_code, .. }) => {
                    push_block_fragment(
                        fragments,
                        source,
                        next_id,
                        PythonFragmentKind::EarlyBlock,
                        loc.1,
                        python_code,
                    );
                }
                AstNode::PythonOneLine(PythonOneLine { loc, python_code }) => {
                    let base_indent = python_inline_column(source, loc.1);
                    let origin = FragmentOrigin {
                        file: source.path.clone(),
                        header_line: loc.1,
                        body_start_line: loc.1,
                        base_indent,
                        line_map: vec![loc.1],
                    };
                    fragments.push(parse_python_fragment(
                        PythonFragmentId(*next_id),
                        PythonFragmentKind::OneLine,
                        python_code.clone(),
                        origin,
                    ));
                    *next_id += 1;
                }
                _ => {}
            }
        }
    }

    collect_from_nodes(source, &renpy.ast, &mut fragments, &mut next_id);

    fragments
}

fn extract_screen_python(
    source: &Arc<SourceDocument>,
    screen: &slast::Screen,
    fragments: &mut Vec<PythonFragmentParse>,
    next_id: &mut usize,
) {
    fn walk_screen(
        source: &Arc<SourceDocument>,
        nodes: &[slast::Node],
        fragments: &mut Vec<PythonFragmentParse>,
        next_id: &mut usize,
    ) {
        for node in nodes {
            match node {
                slast::Node::Python(python) => {
                    let line_count = python.source.lines().count().max(1);
                    let origin = FragmentOrigin {
                        file: source.path.clone(),
                        header_line: python.loc.1,
                        body_start_line: if python.block { python.loc.1 + 1 } else { python.loc.1 },
                        base_indent: if python.block {
                            indent_of_line(&source.text, python.loc.1 + 1)
                        } else {
                            indent_of_line(&source.text, python.loc.1)
                        },
                        line_map: if python.block {
                            (0..line_count).map(|i| python.loc.1 + i).collect()
                        } else {
                            vec![python.loc.1]
                        },
                    };
                    fragments.push(parse_python_fragment(
                        PythonFragmentId(*next_id),
                        PythonFragmentKind::ScreenBlock,
                        python.source.clone(),
                        origin,
                    ));
                    *next_id += 1;
                }
                slast::Node::Displayable(displayable) => {
                    walk_screen(source, &displayable.children, fragments, next_id);
                    if let Some(layout_child) = &displayable.layout_child {
                        walk_screen(source, std::slice::from_ref(layout_child.as_ref()), fragments, next_id);
                    }
                }
                slast::Node::If(if_node) => {
                    for (_, block) in &if_node.entries {
                        walk_screen(source, &block.children, fragments, next_id);
                    }
                }
                slast::Node::ShowIf(show_if) => {
                    for (_, block) in &show_if.entries {
                        walk_screen(source, &block.children, fragments, next_id);
                    }
                }
                slast::Node::For(for_node) => {
                    walk_screen(source, &for_node.block.children, fragments, next_id);
                }
                slast::Node::Use(use_node) => {
                    if let Some(block) = &use_node.block {
                        walk_screen(source, &block.children, fragments, next_id);
                    }
                }
                _ => {}
            }
        }
    }

    walk_screen(source, &screen.children, fragments, next_id);
}

fn extract_test_python_nodes(
    source: &Arc<SourceDocument>,
    nodes: &[TestNode],
    fragments: &mut Vec<PythonFragmentParse>,
    next_id: &mut usize,
) {
    for node in nodes {
        match node {
            TestNode::Python(python) => {
                let line_count = python.code.lines().count().max(1);
                let origin = FragmentOrigin {
                    file: source.path.clone(),
                    header_line: python.loc.1,
                    body_start_line: if python.block { python.loc.1 + 1 } else { python.loc.1 },
                    base_indent: if python.block {
                        indent_of_line(&source.text, python.loc.1 + 1)
                    } else {
                        indent_of_line(&source.text, python.loc.1)
                    },
                    line_map: if python.block {
                        (0..line_count).map(|i| python.loc.1 + i).collect()
                    } else {
                        vec![python.loc.1]
                    },
                };
                fragments.push(parse_python_fragment(
                    PythonFragmentId(*next_id),
                    PythonFragmentKind::Block,
                    python.code.clone(),
                    origin,
                ));
                *next_id += 1;
            }
            TestNode::If(test_if) => {
                for branch in &test_if.branches {
                    extract_test_python_nodes(source, &branch.block, fragments, next_id);
                }
                if let Some(else_block) = &test_if.else_block {
                    extract_test_python_nodes(source, else_block, fragments, next_id);
                }
            }
            TestNode::While(test_while) => {
                extract_test_python_nodes(source, &test_while.block, fragments, next_id);
            }
            _ => {}
        }
    }
}

fn extract_suite_python(
    source: &Arc<SourceDocument>,
    entries: &[TestSuiteEntry],
    fragments: &mut Vec<PythonFragmentParse>,
    next_id: &mut usize,
) {
    for entry in entries {
        match entry {
            TestSuiteEntry::Hook(hook) => {
                extract_test_python_nodes(source, &hook.statements, fragments, next_id);
            }
            TestSuiteEntry::TestCase(case) => {
                extract_test_python_nodes(source, &case.statements, fragments, next_id);
            }
            TestSuiteEntry::TestSuite(suite) => {
                extract_suite_python(source, &suite.entries, fragments, next_id);
            }
        }
    }
}

fn index_renpy(source: &Arc<SourceDocument>, renpy: &RenpyParse, symbols: &mut SymbolIndex) {
    let uri = source.file_url().expect("file url");
    fn index_nodes(
        source: &Arc<SourceDocument>,
        uri: &tower_lsp::lsp_types::Url,
        nodes: &[AstNode],
        symbols: &mut SymbolIndex,
    ) {
        for node in nodes {
        match node {
            AstNode::Label(node) => {
                push_symbol_at_line(
                    symbols,
                    source,
                    &uri,
                    node.loc.1,
                    &node.name,
                    IndexedSymbolKind::Label,
                    Some("label".into()),
                );
                index_nodes(source, uri, &node.block, symbols);
            }
            AstNode::Screen(node) => {
                push_symbol_at_line(
                    symbols,
                    source,
                    &uri,
                    node.loc.1,
                    &node.screen.name,
                    IndexedSymbolKind::Screen,
                    Some("screen".into()),
                );
                index_screen_references(source, &uri, &node.screen.children, symbols);
            }
            AstNode::Transform(Transform { loc, name, .. }) => {
                push_symbol_at_line(
                    symbols,
                    source,
                    &uri,
                    loc.1,
                    name,
                    IndexedSymbolKind::Transform,
                    Some("transform".into()),
                );
            }
            AstNode::Style(Style { loc, name, parent, take, .. }) => {
                push_symbol_at_line(
                    symbols,
                    source,
                    &uri,
                    loc.1,
                    name,
                    IndexedSymbolKind::Style,
                    Some("style".into()),
                );
                if let Some(parent) = parent {
                    push_reference_at_line(
                        symbols,
                        source,
                        &uri,
                        loc.1,
                        parent,
                        IndexedReferenceKind::Style,
                    );
                }
                if let Some(take) = take {
                    push_reference_at_line(
                        symbols,
                        source,
                        &uri,
                        loc.1,
                        take,
                        IndexedReferenceKind::Style,
                    );
                }
            }
            AstNode::Image(node) => {
                push_symbol_at_line(
                    symbols,
                    source,
                    &uri,
                    node.loc.1,
                    &node.name.join(" "),
                    IndexedSymbolKind::Image,
                    Some("image".into()),
                );
            }
            AstNode::LayeredImage(node) => {
                push_symbol_at_line(
                    symbols,
                    source,
                    &uri,
                    node.loc.1,
                    &node.name.join(" "),
                    IndexedSymbolKind::Image,
                    Some("layered image".into()),
                );
            }
            AstNode::Define(node) => {
                push_symbol_at_line(
                    symbols,
                    source,
                    &uri,
                    node.loc.1,
                    &node.name,
                    IndexedSymbolKind::Variable,
                    Some("define".into()),
                );
            }
            AstNode::Default(node) => {
                push_symbol_at_line(
                    symbols,
                    source,
                    &uri,
                    node.loc.1,
                    &node.name,
                    IndexedSymbolKind::Variable,
                    Some("default".into()),
                );
            }
            AstNode::Jump(Jump { loc, target, expression, .. }) if !expression => {
                push_reference_at_line(
                    symbols,
                    source,
                    &uri,
                    loc.1,
                    target,
                    IndexedReferenceKind::Label,
                );
            }
            AstNode::Call(Call { loc, label, expression, .. }) if !expression => {
                push_reference_at_line(
                    symbols,
                    source,
                    &uri,
                    loc.1,
                    label,
                    IndexedReferenceKind::Label,
                );
            }
            AstNode::ScreenStatement(ScreenStatement { loc, screen, .. }) if !screen.expression => {
                push_reference_at_line(
                    symbols,
                    source,
                    &uri,
                    loc.1,
                    &screen.value,
                    IndexedReferenceKind::Screen,
                );
            }
            AstNode::Show(node) => {
                index_image_specifier(source, &uri, node.loc.1, node.imspec.as_ref(), symbols);
            }
            AstNode::Scene(Scene { loc, imspec, .. }) => {
                index_image_specifier(source, &uri, loc.1, imspec.as_ref(), symbols);
            }
            AstNode::Hide(Hide { loc, imgspec }) => {
                index_image_specifier(source, &uri, loc.1, Some(imgspec), symbols);
            }
            AstNode::Menu(node) => {
                for (_, _, block) in &node.items {
                    if let Some(block) = block {
                        index_nodes(source, uri, block, symbols);
                    }
                }
            }
            AstNode::If(node) => {
                for (_, block) in &node.entries {
                    index_nodes(source, uri, block, symbols);
                }
            }
            AstNode::CompileIf(node) => {
                for (_, block) in &node.entries {
                    index_nodes(source, uri, block, symbols);
                }
            }
            AstNode::While(node) => index_nodes(source, uri, &node.block, symbols),
            AstNode::Init(node) => index_nodes(source, uri, &node.block, symbols),
            AstNode::Translate(node) => index_nodes(source, uri, &node.block, symbols),
            AstNode::TranslateBlock(node) => index_nodes(source, uri, &node.block, symbols),
            AstNode::TranslateEarlyBlock(node) => index_nodes(source, uri, &node.block, symbols),
            _ => {}
        }
    }
    }

    index_nodes(source, &uri, &renpy.ast, symbols);
}

fn index_screen_references(
    source: &Arc<SourceDocument>,
    uri: &tower_lsp::lsp_types::Url,
    nodes: &[slast::Node],
    symbols: &mut SymbolIndex,
) {
    for node in nodes {
        match node {
            slast::Node::Use(use_node) => {
                if let slast::UseTarget::Name(name) = &use_node.target {
                    push_reference_at_line(
                        symbols,
                        source,
                        uri,
                        use_node.loc.1,
                        name,
                        IndexedReferenceKind::Screen,
                    );
                }
                if let Some(block) = &use_node.block {
                    index_screen_references(source, uri, &block.children, symbols);
                }
            }
            slast::Node::Displayable(displayable) => {
                index_screen_references(source, uri, &displayable.children, symbols);
                if let Some(layout_child) = &displayable.layout_child {
                    index_screen_references(source, uri, std::slice::from_ref(layout_child.as_ref()), symbols);
                }
            }
            slast::Node::If(if_node) => {
                for (_, block) in &if_node.entries {
                    index_screen_references(source, uri, &block.children, symbols);
                }
            }
            slast::Node::ShowIf(show_if) => {
                for (_, block) in &show_if.entries {
                    index_screen_references(source, uri, &block.children, symbols);
                }
            }
            slast::Node::For(for_node) => {
                index_screen_references(source, uri, &for_node.block.children, symbols);
            }
            _ => {}
        }
    }
}

fn index_image_specifier(
    source: &Arc<SourceDocument>,
    uri: &tower_lsp::lsp_types::Url,
    line: usize,
    specifier: Option<&ImageSpecifier>,
    symbols: &mut SymbolIndex,
) {
    let Some(specifier) = specifier else {
        return;
    };
    if !specifier.image_name.is_empty() {
        push_reference_at_line(
            symbols,
            source,
            uri,
            line,
            &specifier.image_name.join(" "),
            IndexedReferenceKind::Image,
        );
    }
    for transform in &specifier.at_list {
        push_reference_at_line(
            symbols,
            source,
            uri,
            line,
            transform,
            IndexedReferenceKind::Transform,
        );
    }
}

fn push_symbol_at_line(
    symbols: &mut SymbolIndex,
    source: &Arc<SourceDocument>,
    uri: &tower_lsp::lsp_types::Url,
    line: usize,
    name: &str,
    kind: IndexedSymbolKind,
    detail: Option<String>,
) {
    let range = find_name_range(source, line, name)
        .unwrap_or_else(|| source.line_index.one_based_line_range(line));
    symbols.push_symbol(
        name,
        kind,
        SymbolLanguage::Renpy,
        LocationRange {
            uri: uri.clone(),
            range,
        },
        detail,
        None,
    );
}

fn push_reference_at_line(
    symbols: &mut SymbolIndex,
    source: &Arc<SourceDocument>,
    uri: &tower_lsp::lsp_types::Url,
    line: usize,
    name: &str,
    kind: IndexedReferenceKind,
) {
    let range = find_name_range(source, line, name)
        .unwrap_or_else(|| source.line_index.one_based_line_range(line));
    symbols.push_reference(
        name,
        kind,
        SymbolLanguage::Renpy,
        LocationRange {
            uri: uri.clone(),
            range,
        },
        None,
    );
}

fn find_name_range(
    source: &Arc<SourceDocument>,
    one_based_line: usize,
    name: &str,
) -> Option<tower_lsp::lsp_types::Range> {
    let line_text = source.line_index.line_text(&source.text, one_based_line)?;
    let start = line_text.find(name)?;
    let line_start = source.line_index.position_to_offset(tower_lsp::lsp_types::Position::new(
        (one_based_line - 1) as u32,
        0,
    ))?;
    Some(
        source
            .line_index
            .range_from_offsets(line_start + start, line_start + start + name.len()),
    )
}

fn python_inline_column(source: &Arc<SourceDocument>, line: usize) -> usize {
    let line_text = source
        .line_index
        .line_text(&source.text, line)
        .unwrap_or_default();
    match line_text.find('$') {
        Some(index) => {
            let mut column = index + 1;
            while line_text.as_bytes().get(column) == Some(&b' ') {
                column += 1;
            }
            column
        }
        None => indent_of_line(&source.text, line),
    }
}

fn indent_of_line(text: &str, one_based_line: usize) -> usize {
    text.lines()
        .nth(one_based_line.saturating_sub(1))
        .map(|line| line.len() - line.trim_start_matches(' ').len())
        .unwrap_or(0)
}

fn walk_node(node: &AstNode, visitor: &mut dyn AstVisitor) {
    visitor.visit_node(
        node,
        VisitContext {
            line_number: node.line_number(),
        },
    );
    match node {
        AstNode::Label(node) => walk_ast(&node.block, visitor),
        AstNode::Menu(node) => {
            for (_, _, block) in &node.items {
                if let Some(block) = block {
                    walk_ast(block, visitor);
                }
            }
        }
        AstNode::If(node) => {
            for (_, block) in &node.entries {
                walk_ast(block, visitor);
            }
        }
        AstNode::CompileIf(node) => {
            for (_, block) in &node.entries {
                walk_ast(block, visitor);
            }
        }
        AstNode::While(node) => walk_ast(&node.block, visitor),
        AstNode::Init(node) => walk_ast(&node.block, visitor),
        AstNode::Translate(node) => walk_ast(&node.block, visitor),
        AstNode::TranslateBlock(node) => walk_ast(&node.block, visitor),
        AstNode::TranslateEarlyBlock(node) => walk_ast(&node.block, visitor),
        AstNode::Screen(screen) => walk_screen_nodes(&screen.screen.children, visitor),
        _ => {}
    }
}

fn walk_screen_nodes(nodes: &[slast::Node], visitor: &mut dyn AstVisitor) {
    for node in nodes {
        let line_number = match node {
            slast::Node::Displayable(node) => node.loc.1,
            slast::Node::If(node) => node.loc.1,
            slast::Node::ShowIf(node) => node.loc.1,
            slast::Node::For(node) => node.loc.1,
            slast::Node::Python(node) => node.loc.1,
            slast::Node::Default(node) => node.loc.1,
            slast::Node::Use(node) => node.loc.1,
            slast::Node::Transclude(node) => node.loc.1,
            slast::Node::Pass(node) => node.loc.1,
            slast::Node::Break(node) => node.loc.1,
            slast::Node::Continue(node) => node.loc.1,
        };
        visitor.visit_screen_node(node, VisitContext { line_number });
        match node {
            slast::Node::Displayable(displayable) => {
                walk_screen_nodes(&displayable.children, visitor);
                if let Some(layout_child) = &displayable.layout_child {
                    walk_screen_nodes(std::slice::from_ref(layout_child.as_ref()), visitor);
                }
            }
            slast::Node::If(if_node) => {
                for (_, block) in &if_node.entries {
                    walk_screen_nodes(&block.children, visitor);
                }
            }
            slast::Node::ShowIf(show_if) => {
                for (_, block) in &show_if.entries {
                    walk_screen_nodes(&block.children, visitor);
                }
            }
            slast::Node::For(for_node) => walk_screen_nodes(&for_node.block.children, visitor),
            slast::Node::Use(use_node) => {
                if let Some(block) = &use_node.block {
                    walk_screen_nodes(&block.children, visitor);
                }
            }
            _ => {}
        }
    }
}

fn ren_py_to_rpy(data: &str, filename: Option<&PathBuf>) -> Result<String> {
    let mut result = String::with_capacity(data.len());
    let mut prefix_len = 0usize;
    let mut state = 0;
    let mut open_linenumber = 0;

    for (line_num, line) in data.lines().enumerate() {
        if state != 1 && line.starts_with("\"\"\"renpy") {
            state = 1;
            result.push('\n');
            open_linenumber = line_num;
            continue;
        }
        if state == 1 {
            if line == "\"\"\"" {
                state = 2;
                result.push('\n');
                continue;
            }

            let line_trimmed = line.trim();
            if line_trimmed.is_empty() || line_trimmed.starts_with('#') {
                result.push_str(line);
                result.push('\n');
                continue;
            }

            prefix_len = line.len() - line.trim_start_matches(' ').len();
            if line_trimmed.ends_with(':') {
                prefix_len += 4;
            }
            result.push_str(line);
            result.push('\n');
            continue;
        }
        if state == 2 {
            result.push_str(&" ".repeat(prefix_len));
            result.push_str(line);
            result.push('\n');
            continue;
        }
        result.push('\n');
    }

    if let Some(path) = filename {
        if state == 0 {
            bail!(
                "In {}, there are no \"\"\"renpy blocks, so every line is ignored.",
                path.display()
            );
        }
        if state == 1 {
            bail!(
                "In {}, there is an open \"\"\"renpy block at line {} that is not terminated by \"\"\".",
                path.display(),
                open_linenumber
            );
        }
    }

    result.pop();
    Ok(result)
}

fn munge_filename(path: &Path) -> Result<String> {
    let mut stem = path.file_stem().unwrap().to_str().unwrap().to_string();
    if stem.ends_with("_ren") && path.extension() == Some("py".as_ref()) {
        stem = stem.strip_suffix("_ren").unwrap().into();
    }
    stem = stem.replace(' ', "_");
    let mut result = String::with_capacity(stem.len());
    for c in stem.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => result.push(c),
            _ => write!(&mut result, "0x{:x}", c as u32).expect("write to string"),
        }
    }
    Ok(format!("_m1_{result}__"))
}

fn match_logical_word(s: &str, pos: usize) -> (&str, bool, usize) {
    let bytes = s.as_bytes();
    let start = pos;
    let mut end = pos;
    let byte = bytes[pos];

    if byte == b' ' {
        end += 1;
        while end < bytes.len() && bytes[end] == b' ' {
            end += 1;
        }
    } else if byte.is_ascii_alphanumeric() || byte == b'_' {
        end += 1;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            end += 1;
        }
    } else {
        let mut chars = s[pos..].char_indices();
        let (_, first) = chars.next().expect("char boundary");
        end += first.len_utf8();
        if first.is_alphanumeric() || first == '_' {
            for (offset, c) in chars {
                if !(c.is_alphanumeric() || c == '_') {
                    end = pos + offset;
                    break;
                }
                end = pos + offset + c.len_utf8();
            }
        }
    }

    let word = &s[start..end];
    if (end - start) >= 3 && word.starts_with("__") {
        let rest = &word[2..];
        if !rest.contains("__") {
            return (word, true, end);
        }
    }
    (word, false, end)
}

fn list_logical_lines_from_source(source: &Arc<SourceDocument>) -> Result<(Vec<LogicalLine>, CommentMap)> {
    let mut data = source.text.clone();
    let stem = source.path.file_stem().unwrap().to_str().unwrap();
    if stem.ends_with("_ren") && source.path.extension() == Some("py".as_ref()) {
        data = ren_py_to_rpy(&data, Some(&source.path))?;
    }

    let prefix = munge_filename(&source.path)?;
    data.push('\n');
    data.push('\n');

    let mut result = Vec::new();
    let mut comment_map: CommentMap = BTreeMap::new();
    let mut number = 1usize;
    let mut pos = 0usize;
    let bytes = data.as_bytes();
    let data_len = bytes.len();

    if data.starts_with('\u{feff}') {
        pos = '\u{feff}'.len_utf8();
    }

    let mut pending_standalone = Vec::new();

    while pos < data_len {
        let start_number = number;
        let mut line = String::new();
        let mut parendepth = 0;
        let mut trailing_comment: Option<String> = None;

        while pos < data_len {
            let startpos = pos;
            let c = bytes[pos];

            if c == b'\t' {
                bail!("Tab characters are not allowed in Ren'Py scripts: {}:{}", source.path.display(), start_number);
            }

            if c == b'\n' && parendepth == 0 {
                if let Some(comment_text) = trailing_comment.take() {
                    comment_map
                        .entry(start_number)
                        .or_default()
                        .push(Comment::Trailing { text: comment_text, line_number: start_number });
                }
                let final_line = std::mem::take(&mut line);
                if !final_line.trim().is_empty() {
                    for comment in pending_standalone.drain(..) {
                        comment_map.entry(start_number).or_default().push(comment);
                    }
                    let indent = final_line.len() - final_line.trim_start_matches(' ').len();
                    result.push(LogicalLine {
                        path: source.path.clone(),
                        line_number: start_number,
                        range: Some(source.line_index.one_based_line_range(start_number)),
                        indent,
                        text: final_line,
                    });
                }
                pos += 1;
                number += 1;
                break;
            }

            if c == b'\n' {
                number += 1;
            }
            if c == b'\r' {
                pos += 1;
                continue;
            }
            if c == b'\\' && bytes.get(pos + 1) == Some(&b'\n') {
                pos += 2;
                number += 1;
                line.push('\\');
                line.push('\n');
                continue;
            }
            if matches!(c, b'(' | b'[' | b'{') {
                parendepth += 1;
            }
            if matches!(c, b')' | b']' | b'}') && parendepth > 0 {
                parendepth -= 1;
            }
            if c == b'#' {
                let comment_start = pos;
                while pos < data_len && bytes[pos] != b'\n' {
                    pos += 1;
                }
                let comment_text = data[comment_start..pos].to_string();
                if line.trim().is_empty() && parendepth == 0 {
                    pending_standalone.push(Comment::Standalone {
                        indent: line.len() - line.trim_start().len(),
                        text: comment_text,
                        line_number: start_number,
                    });
                } else {
                    trailing_comment = Some(comment_text);
                }
                continue;
            }

            if matches!(c, b'"' | b'\'' | b'`') {
                let delim = c;
                line.push(delim as char);
                pos += 1;
                let mut escape = false;
                let mut triple_quote = false;
                if pos < data_len - 1 && bytes[pos] == delim && bytes[pos + 1] == delim {
                    line.push(delim as char);
                    line.push(delim as char);
                    pos += 2;
                    triple_quote = true;
                }
                let string_start = pos;
                while pos < data_len {
                    let c = bytes[pos];
                    if c == b'\n' {
                        number += 1;
                    }
                    if c == b'\r' {
                        pos += 1;
                        continue;
                    }
                    if escape {
                        escape = false;
                        pos += 1;
                        continue;
                    }
                    if c == delim {
                        if !triple_quote {
                            pos += 1;
                            break;
                        }
                        if pos < data_len - 2 && bytes[pos + 1] == delim && bytes[pos + 2] == delim {
                            pos += 3;
                            break;
                        }
                    }
                    if c == b'\\' {
                        escape = true;
                    }
                    pos += 1;
                }
                line.push_str(&data[string_start..pos]);
                continue;
            }

            let (word, magic, end) = match_logical_word(&data, pos);
            if magic {
                let rest = &word[2..];
                if !rest.contains("__") {
                    line.push_str(&prefix);
                    line.push_str(rest);
                    pos = end;
                    continue;
                }
            }
            line.push_str(word);
            pos = end;
            if pos - startpos > 65536 {
                bail!(
                    "Overly long logical line. (Check strings and parenthesis): {}:{}",
                    source.path.display(),
                    start_number
                );
            }
        }

        if !line.is_empty() {
            bail!(
                "Line is not terminated with a newline. (Check strings and parenthesis): {}:{}",
                source.path.display(),
                start_number
            );
        }
    }

    if !pending_standalone.is_empty() {
        comment_map.entry(EOF_LINE).or_default().extend(pending_standalone);
    }

    Ok((result, comment_map))
}

fn depth_split(s: &str) -> (usize, &str) {
    let depth = s.len() - s.trim_start_matches(' ').len();
    (depth, &s[depth..])
}

fn gll_core(lines: &[(PathBuf, usize, String)], i: usize, min_depth: usize) -> Result<(Vec<Block>, usize)> {
    let mut idx = i;
    let mut result = vec![];
    let mut depth: Option<usize> = None;

    while idx < lines.len() {
        let (filename, number, text) = &lines[idx];
        let (line_depth, rest) = depth_split(text);
        if line_depth < min_depth {
            break;
        }
        if depth.is_none() {
            depth = Some(line_depth);
        }
        if depth.unwrap() != line_depth {
            bail!("Indentation mismatch: {}:{}", filename.display(), number);
        }
        idx += 1;
        let (block, next_idx) = gll_core(lines, idx, depth.unwrap() + 1)?;
        idx = next_idx;
        result.push(Block {
            filename: filename.clone(),
            number: *number,
            text: rest.to_string(),
            block,
        });
    }

    Ok((result, idx))
}

pub fn group_logical_lines(lines: Vec<(PathBuf, usize, String)>) -> Result<Vec<Block>> {
    if lines.is_empty() {
        return Ok(vec![]);
    }
    let (filename, number, text) = lines.first().unwrap();
    let (depth, _) = depth_split(text);
    if depth != 0 {
        bail!("Unexpected indentation at start of file: {}:{}", filename.display(), number);
    }
    let (block, _) = gll_core(&lines, 0, 0)?;
    Ok(block)
}
