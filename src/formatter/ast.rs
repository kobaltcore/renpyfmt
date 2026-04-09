use crate::ast::{
    Call, Camera, CompileIf, Default_, Define, EarlyPython, EndTranslate, Hide, If, Image, Init,
    Jump, Label, Menu, Pass, Python, PythonOneLine, Return, Say, Scene, Show, ShowLayer, Style,
    Testcase, Testsuite, Transform, Translate, TranslateBlock, TranslateEarlyBlock,
    TranslateString, While, With, RPY,
};

use super::{
    core::{Formatter, Mode},
    inline::{
        encode_say_string, format_argument_info, format_image_specifier,
        format_parameter_signature, format_raw_block,
    },
};

impl Formatter {
    pub(crate) fn emit_label(&mut self, node: &Label) {
        let mut line = format!("label {}", node.name);
        if let Some(parameters) = &node.parameters {
            line.push_str(&format_parameter_signature(parameters));
        }
        if node.hide {
            line.push_str(" hide");
        }
        line.push(':');

        self.line(&line);
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_scene(&mut self, node: &Scene) {
        let line = match &node.imspec {
            Some(image) => format!("scene {}", format_image_specifier(image)),
            None => match &node.layer {
                Some(layer) => format!("scene onlayer {layer}"),
                None => String::from("scene"),
            },
        };

        if let Some(atl) = &node.atl {
            self.line(&format!("{line}:"));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line(&line);
        }
    }

    pub(crate) fn emit_show(&mut self, node: &Show) {
        let image = node
            .imspec
            .as_ref()
            .expect("parser should construct show image specifiers");
        let line = format!("show {}", format_image_specifier(image));

        if let Some(atl) = &node.atl {
            self.line(&format!("{line}:"));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line(&line);
        }
    }

    pub(crate) fn emit_with(&mut self, node: &With) {
        if node.expr != "None" {
            self.line(&format!("with {}", node.expr));
        }
    }

    pub(crate) fn emit_say(&mut self, node: &Say) {
        let mut parts = vec![];

        if let Some(who) = &node.who {
            parts.push(who.clone());
        }

        if let Some(attributes) = &node.attributes {
            parts.extend(attributes.clone());
        }

        if let Some(temporary_attributes) = &node.temporary_attributes {
            parts.push("@".to_string());
            parts.extend(temporary_attributes.clone());
        }

        parts.push(encode_say_string(&node.what));

        if let Some(arguments) = &node.arguments {
            parts.push(format_argument_info(arguments));
        }

        if let Some(with_clause) = &node.with {
            parts.push(format!("with {with_clause}"));
        }

        if !node.interact {
            parts.push("nointeract".to_string());
        }

        if let Some(identifier) = &node.identifier {
            parts.push(format!("id {identifier}"));
        }

        self.line(&parts.join(" "));
    }

    pub(crate) fn emit_hide(&mut self, node: &Hide) {
        self.line(&format!("hide {}", format_image_specifier(&node.imgspec)));
    }

    pub(crate) fn emit_python_one_line(&mut self, node: &PythonOneLine) {
        self.line(&format!("$ {}", node.python_code));
    }

    pub(crate) fn emit_jump(&mut self, node: &Jump) {
        let target = if let Some(global_label) = &node.global_label {
            format!("{global_label}.{}", node.target)
        } else {
            node.target.clone()
        };

        if node.expression {
            self.line(&format!("jump expression {target}"));
        } else {
            self.line(&format!("jump {target}"));
        }
    }

    pub(crate) fn emit_menu(&mut self, node: &Menu) {
        let mut header = String::from("menu");
        if let Some(arguments) = &node.arguments {
            header.push_str(&format_argument_info(arguments));
        }
        header.push(':');
        self.line(&header);

        self.indented(|formatter| {
            if let Some(with_clause) = &node.with_ {
                formatter.line(&format!("with {with_clause}"));
            }

            if let Some(set) = &node.set {
                formatter.line(&format!("set {set}"));
            }

            for (index, (label, condition, block)) in node.items.iter().enumerate() {
                if node.has_caption && index == 0 {
                    formatter.line(&encode_say_string(
                        label
                            .as_ref()
                            .expect("parser should construct menu captions"),
                    ));
                } else {
                    let label = label
                        .as_ref()
                        .expect("parser should construct menu choice labels");
                    let mut line = encode_say_string(label);
                    if let Some(arguments) = node
                        .item_arguments
                        .get(index)
                        .and_then(|args| args.as_ref())
                    {
                        line.push_str(&format_argument_info(arguments));
                    }
                    if let Some(condition) = condition {
                        line.push_str(&format!(" if {condition}"));
                    }
                    line.push(':');
                    formatter.line(&line);
                }

                if let Some(block) = block {
                    formatter.indented(|formatter| formatter.nodes(block));
                }
            }
        });
    }

    pub(crate) fn emit_if(&mut self, node: &If) {
        self.emit_conditional_entries(&node.entries, false);
    }

    pub(crate) fn emit_while(&mut self, node: &While) {
        self.line(&format!("while {}:", node.condition));
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_compile_if(&mut self, node: &CompileIf) {
        self.emit_conditional_entries(&node.entries, true);
    }

    fn emit_conditional_entries(
        &mut self,
        entries: &[(Option<String>, Vec<crate::ast::AstNode>)],
        compile: bool,
    ) {
        let first = if compile { "IF" } else { "if" };
        let middle = if compile { "ELIF" } else { "elif" };
        let final_with_condition = if compile { "ELIF" } else { "else if" };
        let final_without_condition = if compile { "ELSE" } else { "else" };
        let last_index = entries.len().saturating_sub(1);

        for (index, (condition, block)) in entries.iter().enumerate() {
            let header = if index == 0 {
                format!(
                    "{first} {}:",
                    condition
                        .as_ref()
                        .expect("parser should construct initial conditional conditions")
                )
            } else if index == last_index {
                match condition {
                    Some(condition) => format!("{final_with_condition} {condition}:"),
                    None => format!("{final_without_condition}:"),
                }
            } else {
                format!(
                    "{middle} {}:",
                    condition
                        .as_ref()
                        .expect("parser should construct elif conditions")
                )
            };

            self.line(&header);
            self.indented(|formatter| formatter.nodes(block));
        }
    }

    pub(crate) fn emit_return(&mut self, node: &Return) {
        if let Some(expr) = &node.expression {
            self.line(&format!("return {expr}"));
        } else {
            self.line("return");
        }
    }

    pub(crate) fn emit_init(&mut self, node: &Init) {
        if self.try_emit_translate_strings(node) {
            return;
        }

        if node.priority != 0 {
            self.line(&format!("init {}:", node.priority));
        } else {
            self.line("init:");
        }

        self.indented(|formatter| formatter.nodes(&node.block));
    }

    fn try_emit_translate_strings(&mut self, node: &Init) -> bool {
        if node.block.is_empty() {
            return false;
        }

        let Some(first_language) = node.block.iter().find_map(|child| match child {
            crate::ast::AstNode::TranslateString(child) => Some(child.language.clone()),
            _ => None,
        }) else {
            return false;
        };

        if !node
            .block
            .iter()
            .all(|child| matches!(child, crate::ast::AstNode::TranslateString(translate) if translate.language == first_language))
        {
            return false;
        }

        let language = first_language.as_deref().unwrap_or("None");
        self.line(&format!("translate {language} strings:"));
        self.indented(|formatter| {
            for child in &node.block {
                let crate::ast::AstNode::TranslateString(translate) = child else {
                    unreachable!();
                };
                formatter.line(&format!("old {}", translate.old));
                formatter.line(&format!("new {}", translate.new));
            }
        });
        true
    }

    pub(crate) fn emit_style(&mut self, node: &Style) {
        let mut line = format!("style {}", node.name);
        if let Some(parent) = &node.parent {
            line.push_str(&format!(" is {parent}"));
        }

        let mut clauses = vec![];
        if node.clear {
            clauses.push(String::from("clear"));
        }
        if let Some(take) = &node.take {
            clauses.push(format!("take {take}"));
        }
        for delattr in &node.delattr {
            clauses.push(format!("del {delattr}"));
        }
        if let Some(variant) = &node.variant {
            clauses.push(format!("variant {variant}"));
        }

        let mut properties = node.properties.iter().collect::<Vec<_>>();
        properties.sort_by(|a, b| a.0.cmp(b.0));
        for (name, expr) in properties {
            clauses.push(format!("{name} {expr}"));
        }

        if clauses.is_empty() {
            self.line(&format!("{line}:"));
        } else {
            self.line(&format!("{line}:"));
            self.indented(|formatter| {
                for clause in clauses {
                    formatter.line(&clause);
                }
            });
        }
    }

    pub(crate) fn emit_define(&mut self, node: &Define) {
        let name = if let Some(index) = &node.index {
            format!("{}[{index}]", node.name)
        } else {
            node.name.clone()
        };

        if node.store == "store" {
            self.line(&format!("define {name} {} {}", node.operator, node.expr));
        } else {
            self.line(&format!(
                "define {}.{name} {} {}",
                node.store.trim_start_matches("store."),
                node.operator,
                node.expr
            ));
        }
    }

    pub(crate) fn emit_default(&mut self, node: &Default_) {
        if node.store == "store" {
            self.line(&format!(
                "default {} = {}",
                node.name,
                node.expr.as_deref().unwrap_or("None")
            ));
        } else {
            self.line(&format!(
                "default {}.{} = {}",
                node.store.trim_start_matches("store."),
                node.name,
                node.expr.as_deref().unwrap_or("None")
            ));
        }
    }

    pub(crate) fn emit_call(&mut self, node: &Call) {
        let label = if let Some(global_label) = &node.global_label {
            format!("{global_label}.{}", node.label)
        } else {
            node.label.clone()
        };

        let mut line = if node.expression {
            format!("call expression {label}")
        } else {
            format!("call {label}")
        };

        if let Some(arguments) = &node.arguments {
            line.push_str(&format_argument_info(arguments));
        }

        self.line(&line);
    }

    pub(crate) fn emit_pass(&mut self, _node: &Pass) {
        self.line("pass");
    }

    pub(crate) fn emit_transform(&mut self, node: &Transform) {
        let mut line = if node.store == "store" {
            format!("transform {}", node.name)
        } else {
            format!(
                "transform {}.{}",
                node.store.trim_start_matches("store."),
                node.name
            )
        };

        if let Some(parameters) = &node.parameters {
            line.push_str(&format_parameter_signature(parameters));
        }

        self.line(&format!("{line}:"));
        if let Some(atl) = &node.atl {
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        }
    }

    pub(crate) fn emit_show_layer(&mut self, node: &ShowLayer) {
        let mut line = format!("show layer {}", node.layer);
        if !node.at_list.is_empty() {
            line.push_str(&format!(" at {}", node.at_list.join(", ")));
        }

        if let Some(atl) = &node.atl {
            self.line(&format!("{line}:"));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line(&line);
        }
    }

    pub(crate) fn emit_camera(&mut self, node: &Camera) {
        let mut parts = vec![String::from("camera")];
        if !node.layer.is_empty() && node.layer != "master" {
            parts.push(node.layer.clone());
        }
        if !node.at_list.is_empty() {
            parts.push(format!("at {}", node.at_list.join(", ")));
        }
        let line = parts.join(" ");

        if let Some(atl) = &node.atl {
            self.line(&format!("{line}:"));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line(&line);
        }
    }

    pub(crate) fn emit_image(&mut self, node: &Image) {
        let line = format!("image {}", node.name.join(" "));
        if let Some(expr) = &node.expr {
            self.line(&format!("{line} = {expr}"));
        } else if let Some(atl) = &node.atl {
            self.line(&format!("{line}:"));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line(&line);
        }
    }

    pub(crate) fn emit_rpy(&mut self, node: &RPY) {
        self.line(&format!("rpy {}", node.rest.join(" ")));
    }

    pub(crate) fn emit_translate(&mut self, node: &Translate) {
        let language = node.language.as_deref().unwrap_or("None");
        self.line(&format!("translate {language} {}:", node.identifier));
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_end_translate(&mut self, _node: &EndTranslate) {}

    pub(crate) fn emit_translate_string(&mut self, node: &TranslateString) {
        self.line(&format!("old {}", node.old));
        self.line(&format!("new {}", node.new));
    }

    pub(crate) fn emit_translate_block(&mut self, node: &TranslateBlock) {
        let language = node.language.as_deref().unwrap_or("None");

        if node.block.len() == 1 {
            if let crate::ast::AstNode::Style(style) = &node.block[0] {
                let mut line = format!("translate {language} style {}", style.name);
                if let Some(parent) = &style.parent {
                    line.push_str(&format!(" is {parent}"));
                }
                line.push(':');
                self.line(&line);

                self.indented(|formatter| {
                    let mut clauses = vec![];
                    if style.clear {
                        clauses.push(String::from("clear"));
                    }
                    if let Some(take) = &style.take {
                        clauses.push(format!("take {take}"));
                    }
                    for delattr in &style.delattr {
                        clauses.push(format!("del {delattr}"));
                    }
                    if let Some(variant) = &style.variant {
                        clauses.push(format!("variant {variant}"));
                    }

                    let mut properties = style.properties.iter().collect::<Vec<_>>();
                    properties.sort_by(|a, b| a.0.cmp(b.0));
                    for (name, expr) in properties {
                        clauses.push(format!("{name} {expr}"));
                    }

                    for clause in clauses {
                        formatter.line(&clause);
                    }
                });
                return;
            }
        }

        self.line(&format!("translate {language}:"));
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_translate_early_block(&mut self, node: &TranslateEarlyBlock) {
        let language = node.language.as_deref().unwrap_or("None");

        if node.block.len() == 1 {
            if let crate::ast::AstNode::Python(python) = &node.block[0] {
                self.line(&format!("translate {language} python:"));
                self.indented(|formatter| {
                    for code_line in python.python_code.lines() {
                        formatter.line(code_line);
                    }
                });
                return;
            }
        }

        self.line(&format!("translate {language}:"));
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_testcase(&mut self, node: &Testcase) {
        self.line(&format!("testcase {}:", node.name));
        for line in format_raw_block(&node.block, self.current_indent() + 4) {
            self.literal_line(&line);
        }
    }

    pub(crate) fn emit_testsuite(&mut self, node: &Testsuite) {
        self.line(&format!("testsuite {}:", node.name));
        for line in format_raw_block(&node.block, self.current_indent() + 4) {
            self.literal_line(&line);
        }
    }

    pub(crate) fn emit_python(&mut self, node: &Python) {
        self.emit_python_block(node, false);
    }

    pub(crate) fn emit_early_python(&mut self, node: &EarlyPython) {
        self.emit_python_block(node, true);
    }

    fn emit_python_block(&mut self, node: &impl PythonBlockLike, early: bool) {
        let mut line = String::from("python");
        if early {
            line.push_str(" early");
        }
        if node.hide() {
            line.push_str(" hide");
        }
        if node.store() != "store" {
            line.push_str(&format!(
                " in {}",
                node.store().trim_start_matches("store.")
            ));
        }
        line.push(':');

        self.line(&line);
        self.indented(|formatter| {
            for code_line in node.python_code().lines() {
                formatter.line(code_line);
            }
        });
    }
}

trait PythonBlockLike {
    fn python_code(&self) -> &str;
    fn store(&self) -> &str;
    fn hide(&self) -> bool;
}

impl PythonBlockLike for Python {
    fn python_code(&self) -> &str {
        &self.python_code
    }

    fn store(&self) -> &str {
        &self.store
    }

    fn hide(&self) -> bool {
        self.hide
    }
}

impl PythonBlockLike for EarlyPython {
    fn python_code(&self) -> &str {
        &self.python_code
    }

    fn store(&self) -> &str {
        &self.store
    }

    fn hide(&self) -> bool {
        self.hide
    }
}
