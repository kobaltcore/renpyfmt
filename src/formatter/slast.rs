use crate::{
    ast::Screen as AstScreen,
    slast::{self, Node},
};

use super::{
    core::Formatter,
    inline::{format_argument_info, format_parameter_signature},
};

impl Formatter {
    pub(crate) fn emit_screen(&mut self, node: &AstScreen) {
        let mut line = format!("screen {}", node.screen.name);
        if let Some(parameters) = &node.screen.parameters {
            line.push_str(&format_parameter_signature(parameters));
        }
        line.push(':');

        self.line_with_trailing(&line);
        self.indented(|formatter| {
            if let Some(docstring) = &node.screen.docstring {
                formatter.line(docstring);
            }

            for (name, expr) in &node.screen.properties {
                if name == "tag" {
                    formatter.line(&format!("tag {expr}"));
                } else {
                    formatter.line(&format!("{name} {expr}"));
                }
            }

            formatter.emit_sl_nodes(&node.screen.children);
        });
    }

    fn emit_sl_nodes(&mut self, nodes: &[Node]) {
        for node in nodes {
            self.emit_sl_node(node);
        }
    }

    fn emit_sl_node(&mut self, node: &Node) {
        match node {
            Node::Displayable(node) => self.emit_sl_displayable(node),
            Node::If(node) => self.emit_sl_conditional("if", &node.entries),
            Node::ShowIf(node) => self.emit_sl_conditional("showif", &node.entries),
            Node::For(node) => self.emit_sl_for(node),
            Node::Python(node) => self.emit_sl_python(node),
            Node::Default(node) => self.line(&format!("default {} = {}", node.name, node.expr)),
            Node::Use(node) => self.emit_sl_use(node),
            Node::Transclude(_) => self.line("transclude"),
            Node::Pass(_) => self.line("pass"),
            Node::Break(_) => self.line("break"),
            Node::Continue(_) => self.line("continue"),
        }
    }

    fn emit_sl_displayable(&mut self, node: &slast::Displayable) {
        let mut line = node.name.clone();
        if !node.positional.is_empty() {
            let rendered = node
                .positional
                .iter()
                .map(|positional| {
                    if node.name == "textbutton" || node.name == "label" {
                        positional.clone()
                    } else if node.name == "text" {
                        positional.clone()
                    } else {
                        positional.clone()
                    }
                })
                .collect::<Vec<_>>();
            line.push(' ');
            line.push_str(&rendered.join(" "));
        }

        let needs_block = node.variable.is_some()
            || !node.properties.is_empty()
            || node.atl_transform.is_some()
            || node.layout_child.is_some()
            || !node.children.is_empty();

        if !needs_block {
            self.line(&line);
            return;
        }

        self.line(&format!("{line}:"));
        self.indented(|formatter| {
            if let Some(variable) = &node.variable {
                formatter.line(&format!("as {variable}"));
            }

            for (name, expr) in &node.properties {
                formatter.line(&format!("{name} {expr}"));
            }

            if let Some(layout_child) = &node.layout_child {
                formatter.emit_sl_has_child(layout_child);
            }

            if let Some(atl) = &node.atl_transform {
                formatter.line("at transform:");
                formatter.indented(|formatter| {
                    formatter.with_mode(super::core::Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(atl)
                    });
                });
            }

            formatter.emit_sl_nodes(&node.children);
        });
    }

    fn emit_sl_has_child(&mut self, node: &Node) {
        match node {
            Node::Displayable(displayable) => {
                let mut line = format!("has {}", displayable.name);
                if !displayable.positional.is_empty() {
                    line.push(' ');
                    line.push_str(&displayable.positional.join(" "));
                }

                let has_body = displayable.variable.is_some()
                    || !displayable.properties.is_empty()
                    || displayable.atl_transform.is_some()
                    || displayable.layout_child.is_some()
                    || !displayable.children.is_empty();

                if !has_body {
                    self.line(&line);
                    return;
                }

                self.line(&format!("{line}:"));
                self.indented(|formatter| formatter.emit_sl_displayable_body(displayable));
            }
            _ => self.line("has pass"),
        }
    }

    fn emit_sl_displayable_body(&mut self, node: &slast::Displayable) {
        if let Some(variable) = &node.variable {
            self.line(&format!("as {variable}"));
        }
        for (name, expr) in &node.properties {
            self.line(&format!("{name} {expr}"));
        }
        if let Some(layout_child) = &node.layout_child {
            self.emit_sl_has_child(layout_child);
        }
        if let Some(atl) = &node.atl_transform {
            self.line("at transform:");
            self.indented(|formatter| {
                formatter.with_mode(super::core::Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        }
        self.emit_sl_nodes(&node.children);
    }

    fn emit_sl_conditional(
        &mut self,
        first_keyword: &str,
        entries: &[(Option<String>, Vec<Node>)],
    ) {
        for (index, (condition, children)) in entries.iter().enumerate() {
            let header = if index == 0 {
                format!(
                    "{first_keyword} {}:",
                    condition
                        .as_deref()
                        .expect("screen conditionals should have a first condition")
                )
            } else if let Some(condition) = condition {
                format!("elif {condition}:")
            } else {
                "else:".to_string()
            };

            self.line(&header);
            self.indented(|formatter| formatter.emit_sl_nodes(children));
        }
    }

    fn emit_sl_for(&mut self, node: &slast::For) {
        let mut line = format!("for {}", node.target);
        if let Some(index_expression) = &node.index_expression {
            line.push_str(&format!(" index {index_expression}"));
        }
        line.push_str(&format!(" in {}:", node.iterable));
        self.line(&line);
        self.indented(|formatter| formatter.emit_sl_nodes(&node.children));
    }

    fn emit_sl_python(&mut self, node: &slast::Python) {
        if node.block {
            let formatted = self.format_python_block_source(&node.source, node.loc.1);
            self.line("python:");
            self.indented(|formatter| {
                for line in formatted.lines() {
                    formatter.line(line);
                }
            });
        } else {
            self.line(&format!("$ {}", node.source));
        }
    }

    fn emit_sl_use(&mut self, node: &slast::Use) {
        let mut line = String::from("use ");
        match &node.target {
            slast::UseTarget::Name(name) => line.push_str(name),
            slast::UseTarget::Expression(expr) => {
                line.push_str(&format!("expression {expr}"));
                if node.pass_context {
                    line.push_str(" pass");
                }
            }
        }
        if let Some(arguments) = &node.arguments {
            line.push_str(&format_argument_info(arguments));
        }
        if let Some(id_expr) = &node.id_expr {
            line.push_str(&format!(" id {id_expr}"));
        }
        if let Some(variable) = &node.variable {
            line.push_str(&format!(" as {variable}"));
        }

        if let Some(block) = &node.block {
            self.line(&format!("{line}:"));
            self.indented(|formatter| formatter.emit_sl_nodes(block));
        } else {
            self.line(&line);
        }
    }
}
