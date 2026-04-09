use crate::ast::{
    Call, Define, Hide, If, Init, Jump, Label, Menu, Python, PythonOneLine, Return, Say, Scene,
    Show, Style, With,
};

use super::{
    core::{Formatter, Mode},
    inline::{encode_say_string, format_argument_info, format_image_specifier},
};

impl Formatter {
    pub(crate) fn emit_label(&mut self, node: &Label) {
        self.line(&format!("label {}:", node.name));
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_scene(&mut self, node: &Scene) {
        let image = node
            .imspec
            .as_ref()
            .expect("parser should construct scene image specifiers");

        let mut clauses = vec![format_image_specifier(image)];
        if let Some(layer) = &node.layer {
            clauses.push(format!("onlayer {layer}"));
        }

        if let Some(atl) = &node.atl {
            self.line(&format!("scene {}:", clauses.join(" ")));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line(&format!("scene {}", clauses.join(" ")));
        }
    }

    pub(crate) fn emit_show(&mut self, node: &Show) {
        let image = node
            .imspec
            .as_ref()
            .expect("parser should construct show image specifiers");

        if let Some(atl) = &node.atl {
            self.line(&format!("show {}:", format_image_specifier(image)));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line(&format!("show {}", format_image_specifier(image)));
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

        if !node.interact {
            parts.push("nointeract".to_string());
        }

        if let Some(identifier) = &node.identifier {
            parts.push(format!("id {identifier}"));
        }

        if let Some(arguments) = &node.arguments {
            parts.push(format_argument_info(arguments));
        }

        if let Some(with_clause) = &node.with {
            parts.push(format!("with {with_clause}"));
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
        if node.expression {
            self.line(&format!("jump expression {}", node.target));
        } else {
            self.line(&format!("jump {}", node.target));
        }
    }

    pub(crate) fn emit_menu(&mut self, node: &Menu) {
        let mut header = String::from("menu");
        if let Some(arguments) = &node.arguments {
            header.push_str(&format_argument_info(arguments));
        }
        if let Some(set) = &node.set {
            header.push_str(&format!(" set {set}"));
        }
        if let Some(with_clause) = &node.with_ {
            header.push_str(&format!(" with {with_clause}"));
        }
        header.push(':');
        self.line(&header);

        self.indented(|formatter| {
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
        let last_index = node.entries.len().saturating_sub(1);

        for (index, (condition, block)) in node.entries.iter().enumerate() {
            let header = if index == 0 {
                format!(
                    "if {}:",
                    condition
                        .as_ref()
                        .expect("parser should construct initial if conditions")
                )
            } else if index == last_index {
                match condition {
                    Some(condition) => format!("else if {condition}:"),
                    None => String::from("else:"),
                }
            } else {
                format!(
                    "elif {}:",
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
            self.line(&format!("return expression {expr}"));
        } else {
            self.line("return");
        }
    }

    pub(crate) fn emit_init(&mut self, node: &Init) {
        if node.block.len() > 1 {
            if node.priority != 0 {
                self.line(&format!("init {}:", node.priority));
            } else {
                self.line("init:");
            }
            self.indented(|formatter| formatter.nodes(&node.block));
        } else {
            self.nodes(&node.block);
        }
    }

    pub(crate) fn emit_style(&mut self, node: &Style) {
        if let Some(parent) = &node.parent {
            self.line(&format!("style {} is {}:", node.name, parent));
        } else {
            self.line(&format!("style {}:", node.name));
        }

        self.indented(|formatter| {
            if node.clear {
                formatter.line("clear");
            }
            if let Some(take) = &node.take {
                formatter.line(&format!("take {take}"));
            }
            for delattr in &node.delattr {
                formatter.line(&format!("del {delattr}"));
            }
            if let Some(variant) = &node.variant {
                formatter.line(&format!("variant {variant}"));
            }

            let mut properties = node.properties.iter().collect::<Vec<_>>();
            properties.sort_by(|a, b| a.0.cmp(b.0));
            for (name, expr) in properties {
                formatter.line(&format!("{name} {expr}"));
            }
        });
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

    pub(crate) fn emit_python(&mut self, node: &Python) {
        if node.store != "store" {
            self.line(&format!("init python in {}:", node.store));
        } else {
            self.line("init python:");
        }

        self.indented(|formatter| {
            for line in node.python_code.lines() {
                formatter.line(line);
            }
        });
    }
}
