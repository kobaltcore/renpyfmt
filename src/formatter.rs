use crate::{
    ast::{
        ArgumentInfo, AstNode, Call, Define, If, ImageSpecifier, Init, Jump, Menu, Python,
        PythonOneLine, Return, Say, Scene, Show, Style,
    },
    atl::{AtlStatement, RawBlock},
};

#[derive(Debug, Clone)]
pub struct FormatContext {
    pub atl_direct_parent: bool,
}

pub trait Format {
    fn format(&self, indent: usize, ctx: &FormatContext) -> String;
}

impl Format for ImageSpecifier {
    fn format(&self, _indent: usize, _ctx: &FormatContext) -> String {
        let mut rv = vec![];

        if self.image_name.len() > 0 {
            rv.push(self.image_name.join(" "));
        }

        if let Some(expr) = &self.expression {
            rv.push(format!("expression {expr}"));
        }

        if let Some(tag) = &self.tag {
            rv.push(format!("as {tag}"));
        }

        if self.at_list.len() > 0 {
            rv.push(format!("at {}", self.at_list.join(", ")));
        };

        if let Some(layer) = &self.layer {
            rv.push(format!("onlayer {}", layer));
        }

        if let Some(zorder) = &self.zorder {
            rv.push(format!("zorder {}", zorder));
        }

        if self.behind.len() > 0 {
            rv.push(format!("behind {}", self.behind.join(", ")));
        };

        rv.join(" ")
    }
}

fn encode_say_string(s: String) -> String {
    let mut s = s.replace("\\", "\\\\");
    s = s.replace("\n", "\\n");
    s = s.replace("\"", "\\\"");
    // s = s.replace(" ", "\\ ");
    format!("\"{}\"", s)
}

impl Format for Say {
    fn format(&self, indent: usize, ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        let mut rv = vec![];

        if let Some(who) = &self.who {
            rv.push(who.clone());
        }

        if let Some(attributes) = &self.attributes {
            rv.extend(attributes.clone());
        }

        if let Some(temporary_attributes) = &self.temporary_attributes {
            rv.push("@".to_string());
            rv.extend(temporary_attributes.clone());
        }

        let what = self.what.clone();

        rv.push(encode_say_string(what));

        if !self.interact {
            rv.push("nointeract".to_string());
        }

        if let Some(identifier) = &self.identifier {
            rv.push(format!("id {identifier}"));
        }

        if let Some(arguments) = &self.arguments {
            rv.push(arguments.format(indent, ctx));
        }

        if let Some(with) = &self.with {
            rv.push(format!("with {with}"));
        }

        format!("{indent_spaces}{}", rv.join(" "))
    }
}

impl Format for ArgumentInfo {
    fn format(&self, _indent: usize, _ctx: &FormatContext) -> String {
        let mut l = vec![];

        for (i, (keyword, expression)) in self.arguments.iter().enumerate() {
            if self.starred_indexes.contains(&i) {
                l.push(format!("*{}", expression.as_ref().unwrap()));
            } else if self.doublestarred_indexes.contains(&i) {
                l.push(format!("**{}", expression.as_ref().unwrap()));
            } else if let Some(keyword) = keyword {
                l.push(format!("{}={}", keyword, expression.as_ref().unwrap()));
            } else {
                l.push(expression.as_ref().unwrap().to_string());
            }
        }

        format!("({})", l.join(", "))
    }
}

impl Format for RawBlock {
    fn format(&self, indent: usize, ctx: &FormatContext) -> String {
        let atl_direct_parent = ctx.atl_direct_parent;
        let mut ctx = ctx.clone();
        ctx.atl_direct_parent = false;

        let mut rv = vec![];

        for statement in &self.statements {
            rv.push(statement.as_ref().unwrap().format(indent + 4, &ctx));
        }

        if atl_direct_parent {
            format!("{}", rv.join("\n"))
        } else {
            let indent_spaces_outer = " ".repeat(indent);
            format!("{indent_spaces_outer}block:\n{}", rv.join("\n"))
        }
    }
}

impl Format for AtlStatement {
    fn format(&self, indent: usize, ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        match self {
            AtlStatement::RawRepeat(node) => {
                if let Some(repeats) = &node.repeats {
                    format!("{indent_spaces}repeat {repeats}")
                } else {
                    format!("{indent_spaces}repeat")
                }
            }
            AtlStatement::RawBlock(node) => node.format(indent, ctx),
            AtlStatement::RawContainsExpr(node) => todo!("raw contains expr"),
            AtlStatement::RawChild(node) => todo!("raw child"),
            AtlStatement::RawParallel(node) => {
                let mut ctx = ctx.clone();
                ctx.atl_direct_parent = true;
                format!(
                    "{indent_spaces}parallel:\n{}",
                    node.block.format(indent + 4, &ctx)
                )
            }
            AtlStatement::RawChoice(node) => {
                let mut ctx = ctx.clone();
                ctx.atl_direct_parent = true;
                format!(
                    "{indent_spaces}choice:\n{}",
                    node.block.format(indent, &ctx)
                )
            }
            AtlStatement::RawOn(node) => todo!("raw on"),
            AtlStatement::RawTime(node) => todo!("raw time"),
            AtlStatement::RawFunction(node) => todo!("raw function"),
            AtlStatement::RawEvent(node) => todo!("raw event"),
            AtlStatement::RawMultipurpose(node) => {
                let mut rv = vec![];

                if let Some(warper) = &node.warper {
                    rv.push(warper.clone());
                }

                if let Some(duration) = &node.duration {
                    rv.push(duration.clone());
                }

                for (name, with) in &node.expressions {
                    if let Some(with) = with {
                        rv.push(format!("{name} with {with}"));
                    } else {
                        rv.push(name.clone());
                    }
                }

                let mut sorted = node.properties.clone();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));

                for (name, exprs) in sorted {
                    rv.push(format!("{} {}", name, exprs));
                }

                format!("{indent_spaces}{}", rv.join(" "))
            }
        }
    }
}

impl Format for Show {
    fn format(&self, indent: usize, ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        if let Some(atl) = &self.atl {
            format!(
                "{indent_spaces}show {}:\n{}",
                self.imspec.as_ref().unwrap().format(indent, ctx),
                atl.format(indent, ctx)
            )
        } else {
            format!(
                "{indent_spaces}show {}",
                self.imspec.as_ref().unwrap().format(indent, ctx)
            )
        }
    }
}

impl Format for Scene {
    fn format(&self, indent: usize, ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        if let Some(atl) = &self.atl {
            format!(
                "{indent_spaces}scene {}:\n{}",
                self.imspec.as_ref().unwrap().format(indent, ctx),
                atl.format(indent, ctx)
            )
        } else {
            format!(
                "{indent_spaces}scene {}",
                self.imspec.as_ref().unwrap().format(indent, ctx)
            )
        }
    }
}

impl Format for PythonOneLine {
    fn format(&self, indent: usize, _ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        format!("{indent_spaces}$ {}", self.python_code)
    }
}

impl Format for Jump {
    fn format(&self, indent: usize, _ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        if self.expression {
            format!("{indent_spaces}jump expression {}", self.target)
        } else {
            format!("{indent_spaces}jump {}", self.target)
        }
    }
}

impl Format for Menu {
    fn format(&self, indent: usize, _ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        let mut lines = vec![];

        lines.push(format!("{indent_spaces}menu:"));
        let indent_spaces = " ".repeat(indent + 4);
        for (i, (label, condition, block)) in self.items.iter().enumerate() {
            if self.has_caption && i == 0 {
                lines.push(format!("{indent_spaces}\"{}\"", label.clone().unwrap()));
            } else {
                match condition {
                    Some(condition) => {
                        lines.push(format!(
                            "{indent_spaces}\"{}\" if {condition}:",
                            label.clone().unwrap()
                        ));
                    }
                    None => {
                        lines.push(format!("{indent_spaces}\"{}\":", label.clone().unwrap()));
                    }
                }
            }

            if let Some(block) = block {
                lines.extend(format_ast(block, indent + 8));
            }
        }

        lines.join("\n")
    }
}

impl Format for If {
    fn format(&self, indent: usize, _ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        let mut lines = vec![];

        let last_idx = self.entries.len() - 1;

        for (i, (cond, block)) in self.entries.iter().enumerate() {
            if i == 0 {
                lines.push(format!("{indent_spaces}if {}:", cond.as_ref().unwrap()));
                lines.extend(format_ast(block, indent + 4));
            } else if i == last_idx {
                match cond {
                    Some(cond) => {
                        lines.push(format!("{indent_spaces}else if {}:", cond));
                    }
                    None => {
                        lines.push(format!("{indent_spaces}else:"));
                    }
                }
                lines.extend(format_ast(block, indent + 4));
            } else {
                lines.push(format!("{indent_spaces}elif {}:", cond.as_ref().unwrap()));
                lines.extend(format_ast(block, indent + 4));
            }
        }

        lines.join("\n")
    }
}

impl Format for Return {
    fn format(&self, indent: usize, _ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        if let Some(expr) = &self.expression {
            format!("{indent_spaces}return expression {expr}")
        } else {
            format!("{indent_spaces}return")
        }
    }
}

impl Format for Init {
    fn format(&self, indent: usize, _ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        let mut lines = vec![];

        if self.block.len() > 1 {
            if self.priority != 0 {
                lines.push(format!("{indent_spaces}init {}:", self.priority));
            } else {
                lines.push(format!("{indent_spaces}init:"));
            }

            lines.extend(format_ast(&self.block, indent + 4));
        } else {
            lines.extend(format_ast(&self.block, indent));
        }

        lines.join("\n")
    }
}

impl Format for Style {
    fn format(&self, indent: usize, _ctx: &FormatContext) -> String {
        let indent_spaces_outer = " ".repeat(indent);
        let indent_spaces_inner = " ".repeat(indent + 4);

        let mut lines = vec![];

        if let Some(parent) = &self.parent {
            lines.push(format!(
                "{indent_spaces_outer}style {} is {}:",
                self.name, parent
            ));
        } else {
            lines.push(format!("{indent_spaces_outer}style {}:", self.name));
        }

        for (name, expr) in &self.properties {
            lines.push(format!("{indent_spaces_inner}{} {}", name, expr));
        }

        lines.join("\n")
    }
}

impl Format for Define {
    fn format(&self, indent: usize, _ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        let name = if let Some(index) = &self.index {
            format!("{}[{}]", self.name, index)
        } else {
            self.name.clone()
        };

        if self.store == "store" {
            format!("{indent_spaces}define {} = {}", name, self.expr)
        } else {
            format!(
                "{indent_spaces}define {}.{} = {}",
                self.store.trim_start_matches("store."),
                name,
                self.expr
            )
        }
    }
}

impl Format for Call {
    fn format(&self, indent: usize, _ctx: &FormatContext) -> String {
        let indent_spaces = " ".repeat(indent);

        let label = if let Some(global_label) = &self.global_label {
            format!("{}.{}", global_label, self.label)
        } else {
            self.label.clone()
        };

        if self.expression {
            format!("{indent_spaces}call expression {}", label)
        } else {
            format!("{indent_spaces}call {}", label)
        }
    }
}

impl Format for Python {
    fn format(&self, indent: usize, _ctx: &FormatContext) -> String {
        let indent_spaces_outer = " ".repeat(indent);
        let indent_spaces_inner = " ".repeat(indent + 4);

        let mut lines = vec![];

        if self.store != "store" {
            lines.push(format!(
                "{indent_spaces_outer}init python in {}:",
                self.store
            ));
        } else {
            lines.push(format!("{indent_spaces_outer}init python:"));
        }

        // TODO: format python with ruff
        lines.push(format!("{indent_spaces_inner}{}", self.python_code));

        lines.join("\n")
    }
}

pub fn format_ast(ast: &Vec<AstNode>, indent: usize) -> Vec<String> {
    let indent_spaces = " ".repeat(indent);

    let mut ctx = FormatContext {
        atl_direct_parent: false,
    };

    // let mut prev_node = None;

    let mut lines = vec![];

    for node in ast {
        match node {
            AstNode::Label(node) => {
                lines.push(format!("label {}:", node.name));
                lines.extend(format_ast(&node.block, indent + 4));
            }
            AstNode::Scene(node) => {
                ctx.atl_direct_parent = true;
                // TODO: only add newline if previous line wasn't a newline already
                lines.push(format!(""));
                lines.push(node.format(indent, &ctx));
            }
            AstNode::Show(node) => {
                ctx.atl_direct_parent = true;
                lines.push(node.format(indent, &ctx));
            }
            AstNode::With(node) => {
                if node.expr != "None" {
                    lines.push(format!("{indent_spaces}with {}", node.expr));
                }
            }
            AstNode::Say(node) => {
                // if prev_node.is_some() && !matches!(prev_node.unwrap(), AstNode::Say(_)) {
                // lines.push(format!());
                // }
                lines.push(format!("{}\n", node.format(indent, &ctx)));
            }
            AstNode::UserStatement(node) => {
                lines.push(format!("{indent_spaces}{}", node.line));
            }
            AstNode::Hide(node) => {
                lines.push(format!(
                    "{indent_spaces}hide {}",
                    node.imgspec.format(indent, &ctx)
                ));
            }
            AstNode::PythonOneLine(node) => {
                lines.push(node.format(indent, &ctx));
            }
            AstNode::Jump(node) => {
                lines.push(format!("{}\n", node.format(indent, &ctx)));
            }
            AstNode::Menu(node) => {
                lines.push(node.format(indent, &ctx));
            }
            AstNode::If(node) => {
                lines.push(node.format(indent, &ctx));
            }
            AstNode::Return(node) => {
                lines.push(format!("{}\n", node.format(indent, &ctx)));
            }
            AstNode::Style(node) => {
                lines.push(node.format(indent, &ctx));
            }
            AstNode::Init(node) => {
                lines.push(format!("{}\n", node.format(indent, &ctx)));
            }
            AstNode::Python(node) => {
                lines.push(node.format(indent, &ctx));
            }
            AstNode::EarlyPython(node) => todo!("early python"),
            AstNode::Define(node) => {
                lines.push(node.format(indent, &ctx));
            }
            AstNode::Default(node) => todo!("default"),
            AstNode::Call(node) => {
                lines.push(node.format(indent, &ctx));
            }
            AstNode::Pass(node) => todo!("pass"),
            AstNode::Transform(node) => todo!("transform"),
            AstNode::Screen(node) => todo!("screen"),
            AstNode::Image(node) => todo!("image"),
        }

        // prev_node = Some(node.clone());
    }

    lines
}
