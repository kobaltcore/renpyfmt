use crate::{
    ast::{ArgumentInfo, AstNode, ImageSpecifier, Say, Scene, Show},
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
        // ctx = renpy.atl.Context({})
        // item = atl.compile(ctx)
        // code = f"unknown: {item}"
        // if isinstance(item, renpy.atl.Interpolation):
        //     merged_properties = f"\n{'    ' * depth}".join([f"{k} {v}" for k, v in item.properties])
        //     if item.warper == "instant":
        //         code = f"{merged_properties}"
        //     else:
        //         code = f"{item.warper} {item.duration} {merged_properties}"
        // elif isinstance(item, renpy.atl.Child):
        //     if item.transition:
        //         code = f"{' '.join(item.child.name)} {item.transition}"
        //     else:
        //         code = f"{' '.join(item.child.name)}"
        // assembled_code = f"{'    ' * depth}{code}\n"
        // return assembled_code

        let indent_spaces = " ".repeat(indent);

        match self {
            AtlStatement::RawRepeat(node) => todo!(),
            AtlStatement::RawBlock(node) => node.format(indent, ctx),
            AtlStatement::RawContainsExpr(node) => todo!(),
            AtlStatement::RawChild(node) => todo!(),
            AtlStatement::RawParallel(node) => todo!(),
            AtlStatement::RawChoice(node) => todo!(),
            AtlStatement::RawOn(node) => todo!(),
            AtlStatement::RawTime(node) => todo!(),
            AtlStatement::RawFunction(node) => todo!(),
            AtlStatement::RawEvent(node) => todo!(),
            AtlStatement::RawMultipurpose(node) => {
                let mut rv = vec![];

                for (name, with) in &node.expressions {
                    if let Some(with) = with {
                        rv.push(format!("{name} with {with}"));
                    } else {
                        rv.push(name.clone());
                    }
                }

                if let Some(warper) = &node.warper {
                    rv.push(warper.clone());
                }

                if let Some(duration) = &node.duration {
                    rv.push(duration.clone());
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

pub fn format_ast(ast: &Vec<AstNode>, indent: usize) {
    let indent_spaces = " ".repeat(indent);

    let mut ctx = FormatContext {
        atl_direct_parent: false,
    };

    let mut prev_node = None;

    for node in ast {
        match node {
            AstNode::Label(node) => {
                println!("label {}:", node.name);
                format_ast(&node.block, indent + 4);
            }
            AstNode::Scene(node) => {
                ctx.atl_direct_parent = true;
                println!();
                println!("{}", node.format(indent, &ctx));
            }
            AstNode::Show(node) => {
                ctx.atl_direct_parent = true;
                println!("{}", node.format(indent, &ctx));
            }
            AstNode::With(node) => {
                if node.expr != "None" {
                    println!("{indent_spaces}with {}", node.expr);
                }
            }
            AstNode::Say(node) => {
                // if prev_node.is_some() && !matches!(prev_node.unwrap(), AstNode::Say(_)) {
                //     println!();
                // }
                println!("{}\n", node.format(indent, &ctx));
            }
            AstNode::UserStatement(node) => {
                println!("{indent_spaces}{}", node.line);
            }
            AstNode::Hide(node) => todo!(),
            AstNode::PythonOneLine(node) => todo!(),
            AstNode::Jump(node) => todo!(),
            AstNode::Menu(node) => todo!(),
            AstNode::If(node) => todo!(),
            AstNode::Return(node) => todo!(),
            AstNode::Style(node) => todo!(),
            AstNode::Init(node) => todo!(),
            AstNode::Python(node) => todo!(),
            AstNode::EarlyPython(node) => todo!(),
            AstNode::Define(node) => todo!(),
            AstNode::Default(node) => todo!(),
            AstNode::Call(node) => todo!(),
            AstNode::Pass(node) => todo!(),
        }

        prev_node = Some(node.clone());
    }
}
