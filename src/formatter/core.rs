use crate::ast::AstNode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Script,
    AtlDirectChild,
    AtlNestedBlock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeKind {
    Scene,
    Other,
}

pub(crate) struct Formatter {
    out: String,
    indent: usize,
    pub(crate) mode: Mode,
    at_line_start: bool,
    previous_top_level_kind: Option<NodeKind>,
}

impl Formatter {
    pub(crate) fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
            mode: Mode::Script,
            at_line_start: true,
            previous_top_level_kind: None,
        }
    }

    pub(crate) fn finish(mut self) -> String {
        while self.out.ends_with('\n') {
            self.out.pop();
        }
        self.out
    }

    pub(crate) fn nodes(&mut self, nodes: &[AstNode]) {
        for node in nodes {
            self.node(node);
        }
    }

    pub(crate) fn node(&mut self, node: &AstNode) {
        if self.indent == 0 {
            let kind = self.node_kind(node);
            if matches!(kind, NodeKind::Scene) && self.previous_top_level_kind.is_some() {
                self.blank_line();
            }
            self.previous_top_level_kind = Some(kind);
        }

        match node {
            AstNode::Label(node) => self.emit_label(node),
            AstNode::Scene(node) => self.emit_scene(node),
            AstNode::Show(node) => self.emit_show(node),
            AstNode::With(node) => self.emit_with(node),
            AstNode::Say(node) => self.emit_say(node),
            AstNode::UserStatement(node) => self.line(&node.line),
            AstNode::Hide(node) => self.emit_hide(node),
            AstNode::PythonOneLine(node) => self.emit_python_one_line(node),
            AstNode::Jump(node) => self.emit_jump(node),
            AstNode::Menu(node) => self.emit_menu(node),
            AstNode::If(node) => self.emit_if(node),
            AstNode::While(_node) => todo!("while"),
            AstNode::CompileIf(_node) => todo!("compile if"),
            AstNode::Return(node) => self.emit_return(node),
            AstNode::Style(node) => self.emit_style(node),
            AstNode::Init(node) => self.emit_init(node),
            AstNode::Python(node) => self.emit_python(node),
            AstNode::EarlyPython(_node) => todo!("early python"),
            AstNode::Define(node) => self.emit_define(node),
            AstNode::Default(_node) => todo!("default"),
            AstNode::Call(node) => self.emit_call(node),
            AstNode::Pass(_node) => todo!("pass"),
            AstNode::Transform(_node) => todo!("transform"),
            AstNode::ShowLayer(_node) => todo!("show layer"),
            AstNode::Camera(_node) => todo!("camera"),
            AstNode::Screen(_node) => todo!("screen"),
            AstNode::Image(_node) => todo!("image"),
            AstNode::RPY(_node) => todo!("rpy"),
            AstNode::Translate(_node) => todo!("translate"),
            AstNode::EndTranslate(_node) => todo!("end translate"),
            AstNode::TranslateString(_node) => todo!("translate string"),
            AstNode::TranslateBlock(_node) => todo!("translate block"),
            AstNode::TranslateEarlyBlock(_node) => todo!("translate early block"),
            AstNode::Testcase(_node) => todo!("testcase"),
            AstNode::Testsuite(_node) => todo!("testsuite"),
        }
    }

    fn node_kind(&self, node: &AstNode) -> NodeKind {
        match node {
            AstNode::Scene(_) => NodeKind::Scene,
            _ => NodeKind::Other,
        }
    }

    pub(crate) fn line(&mut self, text: &str) {
        self.write_indent();
        self.out.push_str(text);
        self.out.push('\n');
        self.at_line_start = true;
    }

    pub(crate) fn blank_line(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with("\n\n") {
            self.out.push('\n');
            self.at_line_start = true;
        }
    }

    pub(crate) fn indented(&mut self, f: impl FnOnce(&mut Self)) {
        self.indent += 4;
        f(self);
        self.indent -= 4;
    }

    pub(crate) fn with_mode<T>(&mut self, mode: Mode, f: impl FnOnce(&mut Self) -> T) -> T {
        let previous = self.mode;
        self.mode = mode;
        let result = f(self);
        self.mode = previous;
        result
    }

    fn write_indent(&mut self) {
        if self.at_line_start {
            self.out.push_str(&" ".repeat(self.indent));
            self.at_line_start = false;
        }
    }
}

pub fn format_ast(ast: &[AstNode]) -> String {
    let mut formatter = Formatter::new();
    formatter.nodes(ast);
    formatter.finish()
}
