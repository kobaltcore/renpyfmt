use crate::ast::{AstNode, With};
use crate::comments::{Comment, CommentMap, EOF_LINE};
use std::collections::BTreeMap;

use super::python::PythonFormatConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Script,
    AtlDirectChild,
    AtlNestedBlock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeKind {
    Block,
    Other,
}

pub(crate) struct Formatter {
    out: String,
    indent: usize,
    indent_text: String,
    pub(crate) mode: Mode,
    at_line_start: bool,
    previous_node_kind: Option<NodeKind>,
    comments: CommentMap,
    last_emitted_line: usize,
    current_trailing_line: Option<usize>,
    current_next_line: Option<usize>,
    pub(crate) current_init_offset: isize,
    pub(crate) python_format_config: PythonFormatConfig,
}

impl Formatter {
    pub(crate) fn new(comments: CommentMap, python_format_config: PythonFormatConfig) -> Self {
        Self {
            out: String::new(),
            indent: 0,
            indent_text: String::new(),
            mode: Mode::Script,
            at_line_start: true,
            previous_node_kind: None,
            comments,
            last_emitted_line: 0,
            current_trailing_line: None,
            current_next_line: None,
            current_init_offset: 0,
            python_format_config,
        }
    }

    pub(crate) fn finish(mut self) -> String {
        self.flush_remaining_comments();
        while self.out.ends_with('\n') || self.out.ends_with(' ') || self.out.ends_with('\t') {
            if self.out.ends_with('\n') {
                self.out.pop();
            } else {
                trim_trailing_horizontal_whitespace(&mut self.out);
            }
        }
        self.out
    }

    pub(crate) fn nodes(&mut self, nodes: &[AstNode]) {
        let mut i = 0;
        while i < nodes.len() {
            let node = &nodes[i];

            self.emit_leading_comments(node.line_number());

            if matches!(node, AstNode::Say(_))
                && matches!(
                    nodes.get(i + 1),
                    Some(AstNode::Menu(menu)) if menu.say_caption.is_some()
                )
            {
                i += 1;
                continue;
            }

            let with_suffix = match node {
                AstNode::Say(_) => nodes.get(i + 1).and_then(|next| match next {
                    AstNode::With(w) if w.paired.is_none() && w.expr != "None" => Some(w),
                    _ => None,
                }),
                AstNode::Show(_) | AstNode::Scene(_) | AstNode::Hide(_) => {
                    let has_paired_with = i > 0
                        && matches!(
                            &nodes[i - 1],
                            AstNode::With(w) if w.paired.is_some()
                        );
                    if has_paired_with {
                        nodes.get(i + 1).and_then(|next| match next {
                            AstNode::With(w) if w.paired.is_none() && w.expr != "None" => Some(w),
                            _ => None,
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if with_suffix.is_some() {
                self.current_next_line = nodes.get(i + 2).map(AstNode::line_number);
                self.current_trailing_line = Some(node.line_number());
                self.node_with_suffix(node, with_suffix);
                self.current_trailing_line = None;
                self.current_next_line = None;
                i += 2;
            } else {
                self.current_next_line = nodes.get(i + 1).map(AstNode::line_number);
                self.current_trailing_line = Some(node.line_number());
                self.node(node);
                self.current_trailing_line = None;
                self.current_next_line = None;
                i += 1;
            }

            self.last_emitted_line = node.line_number();
        }
    }

    pub(crate) fn node(&mut self, node: &AstNode) {
        self.node_with_suffix(node, None);
    }

    pub(crate) fn node_with_suffix(&mut self, node: &AstNode, with_suffix: Option<&With>) {
        let kind = self.node_kind(node);
        if self.indent == 0
            && matches!(node, AstNode::Scene(_))
            && self.previous_node_kind.is_some()
        {
            self.blank_line();
        } else if let Some(previous_kind) = self.previous_node_kind {
            if matches!(kind, NodeKind::Block) || matches!(previous_kind, NodeKind::Block) {
                self.blank_line();
            }
        }
        self.previous_node_kind = Some(kind);

        match node {
            AstNode::Label(node) => self.emit_label(node),
            AstNode::Scene(node) => self.emit_scene(node, with_suffix),
            AstNode::Show(node) => self.emit_show(node, with_suffix),
            AstNode::With(node) => self.emit_with(node),
            AstNode::Say(node) => self.emit_say(node, with_suffix),
            AstNode::UserStatement(node) => {
                let text = self.take_trailing_comment_for_current_line(&node.line);
                self.line(&text);
            }
            AstNode::AudioStatement(node) => self.emit_audio_statement(node),
            AstNode::PauseStatement(node) => self.emit_pause_statement(node),
            AstNode::ScreenStatement(node) => self.emit_screen_statement(node),
            AstNode::WindowStatement(node) => self.emit_window_statement(node),
            AstNode::WindowAutoStatement(node) => self.emit_window_auto_statement(node),
            AstNode::Hide(node) => self.emit_hide(node, with_suffix),
            AstNode::PythonOneLine(node) => self.emit_python_one_line(node),
            AstNode::Jump(node) => self.emit_jump(node),
            AstNode::Menu(node) => self.emit_menu(node),
            AstNode::If(node) => self.emit_if(node),
            AstNode::While(node) => self.emit_while(node),
            AstNode::CompileIf(node) => self.emit_compile_if(node),
            AstNode::Return(node) => self.emit_return(node),
            AstNode::Style(node) => self.emit_style(node),
            AstNode::Init(node) => self.emit_init(node),
            AstNode::InitOffset(node) => self.emit_init_offset(node),
            AstNode::Python(node) => self.emit_python(node),
            AstNode::EarlyPython(node) => self.emit_early_python(node),
            AstNode::Define(node) => self.emit_define(node),
            AstNode::Default(node) => self.emit_default(node),
            AstNode::Call(node) => self.emit_call(node),
            AstNode::Pass(node) => self.emit_pass(node),
            AstNode::Transform(node) => self.emit_transform(node),
            AstNode::ShowLayer(node) => self.emit_show_layer(node),
            AstNode::Camera(node) => self.emit_camera(node),
            AstNode::Screen(node) => self.emit_screen(node),
            AstNode::Image(node) => self.emit_image(node),
            AstNode::LayeredImage(node) => self.emit_layered_image(node),
            AstNode::RPY(node) => self.emit_rpy(node),
            AstNode::Translate(node) => self.emit_translate(node),
            AstNode::EndTranslate(node) => self.emit_end_translate(node),
            AstNode::TranslateString(node) => self.emit_translate_string(node),
            AstNode::TranslateBlock(node) => self.emit_translate_block(node),
            AstNode::TranslateEarlyBlock(node) => self.emit_translate_early_block(node),
            AstNode::Testcase(node) => self.emit_testcase(node),
            AstNode::Testsuite(node) => self.emit_testsuite(node),
        }
    }

    fn node_kind(&self, node: &AstNode) -> NodeKind {
        match node {
            AstNode::Label(_)
            | AstNode::Menu(_)
            | AstNode::If(_)
            | AstNode::While(_)
            | AstNode::CompileIf(_)
            | AstNode::Python(_)
            | AstNode::EarlyPython(_)
            | AstNode::Translate(_)
            | AstNode::TranslateBlock(_)
            | AstNode::TranslateEarlyBlock(_)
            | AstNode::Testcase(_)
            | AstNode::Testsuite(_)
            | AstNode::Screen(_) => NodeKind::Block,
            AstNode::Init(node) if self.init_emits_block(node) => NodeKind::Block,
            AstNode::Style(node) if self.style_emits_block(node) => NodeKind::Block,
            AstNode::Transform(_) => NodeKind::Block,
            AstNode::Scene(node) if node.atl.is_some() => NodeKind::Block,
            AstNode::Show(node) if node.atl.is_some() => NodeKind::Block,
            AstNode::ShowLayer(node) if node.atl.is_some() => NodeKind::Block,
            AstNode::Camera(node) if node.atl.is_some() => NodeKind::Block,
            AstNode::Image(node) if node.atl.is_some() && node.expr.is_none() => NodeKind::Block,
            AstNode::LayeredImage(_) => NodeKind::Block,
            _ => NodeKind::Other,
        }
    }

    fn style_emits_block(&self, node: &crate::ast::Style) -> bool {
        node.clear
            || node.take.is_some()
            || !node.delattr.is_empty()
            || node.variant.is_some()
            || !node.properties.is_empty()
    }

    fn init_emits_block(&self, node: &crate::ast::Init) -> bool {
        if self.try_init_emits_implicit(node) {
            return false;
        }

        true
    }

    fn try_init_emits_implicit(&self, node: &crate::ast::Init) -> bool {
        let [child] = node.block.as_slice() else {
            return false;
        };

        match child {
            AstNode::Define(_)
            | AstNode::Default(_)
            | AstNode::Style(_)
            | AstNode::Transform(_)
                if node.priority == self.current_init_offset =>
            {
                true
            }
            AstNode::Image(_) if node.priority == 500 + self.current_init_offset => true,
            _ => false,
        }
    }

    pub(crate) fn emit_leading_comments(&mut self, line_number: usize) {
        let standalone_texts: Vec<String> =
            if let Some(comments) = self.comments.get_mut(&line_number) {
                let mut texts = vec![];
                let mut i = 0;
                while i < comments.len() {
                    if let Comment::Standalone { text, .. } = &comments[i] {
                        texts.push(text.clone());
                        comments.remove(i);
                    } else {
                        i += 1;
                    }
                }
                if comments.is_empty() {
                    self.comments.remove(&line_number);
                }
                texts
            } else {
                vec![]
            };

        for text in standalone_texts {
            self.line(&text);
        }
    }

    pub(crate) fn format_python_block_source(
        &mut self,
        source: &str,
        header_line: usize,
    ) -> String {
        let mut standalone_comments: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        let mut trailing_comments: BTreeMap<usize, String> = BTreeMap::new();
        let body_indent = self.current_indent() + 4;
        let body_start_line = header_line + 1;
        let body_lines: Vec<&str> = source.lines().skip(1).collect();
        let body_end_line = body_start_line.saturating_add(body_lines.len().saturating_sub(1));

        let comment_keys: Vec<usize> = self.comments.keys().copied().collect();
        for key in comment_keys {
            let Some(comments) = self.comments.get_mut(&key) else {
                continue;
            };

            let mut i = 0;
            while i < comments.len() {
                let matches = match &comments[i] {
                    Comment::Standalone {
                        indent,
                        line_number,
                        ..
                    } => {
                        *line_number >= body_start_line
                            && self
                                .current_next_line
                                .is_none_or(|next| *line_number < next)
                            && *indent >= body_indent
                    }
                    Comment::Trailing { line_number, .. } => {
                        *line_number >= body_start_line && *line_number <= body_end_line
                    }
                };

                if !matches {
                    i += 1;
                    continue;
                }

                let comment = comments.remove(i);
                match comment {
                    Comment::Standalone {
                        text,
                        line_number,
                        indent,
                    } => {
                        let relative_indent = " ".repeat(indent.saturating_sub(body_indent));
                        standalone_comments
                            .entry(line_number)
                            .or_default()
                            .push(format!("{relative_indent}{text}"));
                    }
                    Comment::Trailing { text, line_number } => {
                        trailing_comments.insert(line_number, text);
                    }
                }
            }

            if comments.is_empty() {
                self.comments.remove(&key);
            }
        }

        let final_line = standalone_comments
            .keys()
            .copied()
            .max()
            .map_or(body_end_line, |line| line.max(body_end_line));

        let mut merged_lines = vec![];
        for line_number in body_start_line..=final_line {
            let standalone_for_line = standalone_comments.remove(&line_number);
            if let Some(comment_lines) = standalone_for_line.as_ref() {
                merged_lines.extend(comment_lines.iter().cloned());
            }

            let raw_line = body_lines
                .get(line_number.saturating_sub(body_start_line))
                .copied()
                .unwrap_or("");
            if standalone_for_line.is_some() && raw_line.trim().is_empty() {
                continue;
            }

            let line = if let Some(comment) = trailing_comments.remove(&line_number) {
                if raw_line.trim().is_empty() {
                    comment
                } else {
                    format!("{raw_line}  {comment}")
                }
            } else {
                raw_line.to_string()
            };

            if !(raw_line.trim().is_empty() && merged_lines.last().is_some_and(String::is_empty)) {
                merged_lines.push(line);
            }
        }

        let start = merged_lines
            .iter()
            .position(|line| !line.is_empty())
            .unwrap_or(merged_lines.len());
        let end = merged_lines
            .iter()
            .rposition(|line| !line.is_empty())
            .map(|index| index + 1)
            .unwrap_or(start);

        let source = if start == end {
            String::new()
        } else {
            merged_lines[start..end].join("\n")
        };

        super::python::format_python_block(&source, &self.python_format_config)
    }

    fn take_trailing_comment_for_current_line(&mut self, text: &str) -> String {
        let line_number = self.current_trailing_line.unwrap_or(0);
        if let Some(comments) = self.comments.get_mut(&line_number) {
            let mut i = 0;
            while i < comments.len() {
                if let Comment::Trailing {
                    text: comment_text, ..
                } = &comments[i]
                {
                    let comment_text = comment_text.clone();
                    comments.remove(i);
                    if comments.is_empty() {
                        self.comments.remove(&line_number);
                    }
                    return format!("{text}  {comment_text}");
                }
                i += 1;
            }
        }
        text.to_string()
    }

    fn flush_remaining_comments(&mut self) {
        if let Some(comments) = self.comments.remove(&EOF_LINE) {
            for comment in comments {
                if let Comment::Standalone { text, .. } = comment {
                    self.line(&text);
                }
            }
        }

        let remaining_keys: Vec<usize> = self.comments.keys().copied().collect();
        for key in remaining_keys {
            if let Some(comments) = self.comments.remove(&key) {
                for comment in comments {
                    if let Comment::Standalone { text, .. } = comment {
                        self.line(&text);
                    }
                }
            }
        }
    }

    pub(crate) fn line(&mut self, text: &str) {
        if !text.is_empty() {
            self.write_indent();
        }
        self.out.push_str(text);
        self.out.push('\n');
        self.at_line_start = true;
    }

    pub(crate) fn line_with_trailing(&mut self, text: &str) {
        let full_text = self.take_trailing_comment_for_current_line(text);
        if !full_text.is_empty() {
            self.write_indent();
        }
        self.out.push_str(&full_text);
        self.out.push('\n');
        self.at_line_start = true;
    }

    pub(crate) fn line_for_source(&mut self, text: &str, line_number: usize) {
        self.emit_leading_comments(line_number);
        let previous = self.current_trailing_line;
        self.current_trailing_line = Some(line_number);
        self.line_with_trailing(text);
        self.current_trailing_line = previous;
        self.last_emitted_line = line_number;
    }

    pub(crate) fn blank_line(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with("\n\n") {
            self.out.push('\n');
            self.at_line_start = true;
        }
    }

    pub(crate) fn indented(&mut self, f: impl FnOnce(&mut Self)) {
        let previous_kind = self.previous_node_kind;
        self.previous_node_kind = None;
        self.indent += 4;
        self.indent_text.push_str("    ");
        f(self);
        self.indent -= 4;
        self.indent_text.truncate(self.indent);
        self.previous_node_kind = previous_kind;
    }

    pub(crate) fn with_mode<T>(&mut self, mode: Mode, f: impl FnOnce(&mut Self) -> T) -> T {
        let previous = self.mode;
        self.mode = mode;
        let result = f(self);
        self.mode = previous;
        result
    }

    pub(crate) fn current_indent(&self) -> usize {
        self.indent
    }

    fn write_indent(&mut self) {
        if self.at_line_start {
            self.out.push_str(&self.indent_text);
            self.at_line_start = false;
        }
    }
}

fn trim_trailing_horizontal_whitespace(out: &mut String) {
    while matches!(out.as_bytes().last(), Some(b' ' | b'\t')) {
        out.pop();
    }
}

pub fn format_ast(ast: &[AstNode], comments: &CommentMap) -> String {
    format_ast_with_config(ast, comments, &PythonFormatConfig::default())
}

pub fn format_ast_with_config(
    ast: &[AstNode],
    comments: &CommentMap,
    python_format_config: &PythonFormatConfig,
) -> String {
    let mut formatter = Formatter::new(comments.clone(), python_format_config.clone());
    formatter.nodes(ast);
    formatter.finish()
}
