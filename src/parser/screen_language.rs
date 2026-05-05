use super::{parse_arguments, parse_atl, parse_parameters};
use crate::{
    ast::ArgumentInfo,
    error::Result,
    lexer::{Lexer, LexerType, LexerTypeOptions, RegexType},
    slast,
};
use std::{collections::HashSet, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChildCount {
    Zero,
    One,
    Many,
}

#[derive(Clone, Copy, Debug)]
struct DisplayableSpec {
    name: &'static str,
    positional: &'static [&'static str],
    nchildren: ChildCount,
    default_properties: bool,
}

const DISPLAYABLE_SPECS: &[DisplayableSpec] = &[
    DisplayableSpec {
        name: "null",
        positional: &[],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "text",
        positional: &["text"],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "hbox",
        positional: &[],
        nchildren: ChildCount::Many,
        default_properties: true,
    },
    DisplayableSpec {
        name: "vbox",
        positional: &[],
        nchildren: ChildCount::Many,
        default_properties: true,
    },
    DisplayableSpec {
        name: "fixed",
        positional: &[],
        nchildren: ChildCount::Many,
        default_properties: true,
    },
    DisplayableSpec {
        name: "grid",
        positional: &["cols", "rows"],
        nchildren: ChildCount::Many,
        default_properties: true,
    },
    DisplayableSpec {
        name: "side",
        positional: &["positions"],
        nchildren: ChildCount::Many,
        default_properties: true,
    },
    DisplayableSpec {
        name: "window",
        positional: &[],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "frame",
        positional: &[],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "key",
        positional: &["key"],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "timer",
        positional: &["delay"],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "input",
        positional: &[],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "button",
        positional: &[],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "imagebutton",
        positional: &[],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "textbutton",
        positional: &["label"],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "iconbutton",
        positional: &["icon"],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "label",
        positional: &["label"],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "bar",
        positional: &[],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "vbar",
        positional: &[],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "viewport",
        positional: &[],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "vpgrid",
        positional: &[],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "imagemap",
        positional: &[],
        nchildren: ChildCount::Many,
        default_properties: true,
    },
    DisplayableSpec {
        name: "hotspot",
        positional: &[],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "hotbar",
        positional: &[],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "transform",
        positional: &[],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "add",
        positional: &["displayable"],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "drag",
        positional: &[],
        nchildren: ChildCount::Many,
        default_properties: true,
    },
    DisplayableSpec {
        name: "draggroup",
        positional: &[],
        nchildren: ChildCount::Many,
        default_properties: true,
    },
    DisplayableSpec {
        name: "mousearea",
        positional: &[],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "on",
        positional: &["event"],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "nearrect",
        positional: &[],
        nchildren: ChildCount::One,
        default_properties: true,
    },
    DisplayableSpec {
        name: "dismiss",
        positional: &[],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "areapicker",
        positional: &[],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
    DisplayableSpec {
        name: "icon",
        positional: &["name"],
        nchildren: ChildCount::Zero,
        default_properties: true,
    },
];

const SCREEN_PROPERTY_NAMES: &[&str] = &[
    "modal",
    "zorder",
    "variant",
    "predict",
    "style_group",
    "style_prefix",
    "layer",
    "sensitive",
    "roll_forward",
    "tag",
];

const USE_BLOCK_PROPERTY_NAMES: &[&str] = &["style_group", "style_prefix"];

const RESERVED_CHILD_NAMES: &[&str] = &[
    "if",
    "elif",
    "else",
    "showif",
    "for",
    "break",
    "continue",
    "$",
    "python",
    "pass",
    "default",
    "use",
    "transclude",
];

#[derive(Clone, Copy)]
struct PropertyContext<'a> {
    owner: &'a str,
    allow_default_properties: bool,
    allowed_names: Option<&'a [&'a str]>,
}

#[derive(Clone, Copy)]
struct BlockContext<'a> {
    container_name: &'a str,
    allow_break_continue: bool,
    property_context: Option<PropertyContext<'a>>,
}

pub(super) fn parse_screen(lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<slast::Screen> {
    let name = lex.require_or_error(
        LexerType::Type(LexerTypeOptions::Word),
        "expected screen name",
    )?;
    let parameters = parse_parameters(lex)?;

    let mut screen = slast::Screen {
        loc: loc.clone(),
        name,
        parameters,
        properties: vec![],
        docstring: None,
        children: vec![],
    };

    let mut seen = HashSet::new();
    while !lex.eol() {
        if lex.rmatch(RegexType::Simple(":")).is_some() {
            break;
        }
        parse_named_property(lex, &mut screen.properties, &mut seen, true, "screen", None)?;
    }

    lex.expect_eol()?;
    lex.expect_block()?;

    let mut sub = lex.subblock_lexer(false);
    if sub.advance() {
        let state = sub.checkpoint();
        if parse_docstring(&mut sub, &mut screen.docstring)? {
            if !sub.eob {
                // already advanced by docstring parser
            }
        } else {
            sub.revert(state);
        }

        while !sub.eob {
            if try_parse_screen_block_property(&mut sub, &mut screen.properties, &mut seen)? {
                continue;
            }

            let node = parse_node(
                &mut sub,
                BlockContext {
                    container_name: "screen",
                    allow_break_continue: false,
                    property_context: None,
                },
            )?;
            screen.children.push(node);
        }
    }

    lex.advance();
    Ok(screen)
}

fn parse_docstring(lex: &mut Lexer, docstring: &mut Option<String>) -> Result<bool> {
    let start = lex.pos;
    if !lex.python_string()? {
        return Ok(false);
    }
    let text = lex.text[start..lex.pos].trim().to_string();
    lex.expect_eol()?;
    lex.expect_noblock()?;
    *docstring = Some(text);
    lex.advance();
    Ok(true)
}

fn try_parse_screen_block_property(
    lex: &mut Lexer,
    properties: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
) -> Result<bool> {
    let state = lex.checkpoint();
    let Some(word) = lex.word() else {
        return Ok(false);
    };

    if !SCREEN_PROPERTY_NAMES.contains(&word.as_str()) {
        lex.revert(state);
        return Ok(false);
    }

    lex.revert(state);
    parse_named_property(lex, properties, seen, true, "screen", None)?;
    lex.expect_eol()?;
    lex.expect_noblock()?;
    lex.advance();
    Ok(true)
}

fn parse_node(lex: &mut Lexer, ctx: BlockContext<'_>) -> Result<slast::Node> {
    let start_loc = lex.get_location();
    if lex.rmatch(RegexType::Simple("$")).is_some() {
        let source = lex
            .rest_statement()
            .ok_or_else(|| lex.parse_error("expected python code"))?
            .trim()
            .to_string();
        lex.expect_eol()?;
        lex.expect_noblock()?;
        lex.advance();
        return Ok(slast::Node::Python(slast::Python {
            loc: start_loc,
            source,
            block: false,
        }));
    }

    let word = lex.word().ok_or_else(|| {
        lex.parse_error(format!(
            "expected child statement in {}",
            ctx.container_name
        ))
    })?;

    match word.as_str() {
        "if" => parse_conditional(lex, start_loc, false, ctx),
        "showif" => parse_conditional(lex, start_loc, true, ctx),
        "for" => parse_for(lex, start_loc, ctx),
        "break" => {
            if !ctx.allow_break_continue {
                return Err(
                    lex.parse_error("break may only appear inside a screen language for block")
                );
            }
            lex.expect_eol()?;
            lex.expect_noblock()?;
            lex.advance();
            Ok(slast::Node::Break(slast::Break { loc: start_loc }))
        }
        "continue" => {
            if !ctx.allow_break_continue {
                return Err(
                    lex.parse_error("continue may only appear inside a screen language for block")
                );
            }
            lex.expect_eol()?;
            lex.expect_noblock()?;
            lex.advance();
            Ok(slast::Node::Continue(slast::Continue { loc: start_loc }))
        }
        "python" => parse_python_block(lex, start_loc),
        "pass" => {
            lex.expect_eol()?;
            lex.expect_noblock()?;
            lex.advance();
            Ok(slast::Node::Pass(slast::Pass { loc: start_loc }))
        }
        "default" => parse_default(lex, start_loc),
        "use" => parse_use(lex, start_loc),
        "transclude" => {
            lex.expect_eol()?;
            lex.expect_noblock()?;
            lex.advance();
            Ok(slast::Node::Transclude(slast::Transclude {
                loc: start_loc,
            }))
        }
        _ => {
            if let Some(spec) = displayable_spec(&word) {
                parse_displayable(lex, start_loc, spec)
            } else {
                Err(lex.parse_error(format!(
                    "{word:?} is not a valid child statement of the {} statement.",
                    ctx.container_name
                )))
            }
        }
    }
}

fn parse_conditional(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
    showif: bool,
    ctx: BlockContext<'_>,
) -> Result<slast::Node> {
    let condition = lex
        .python_expression()
        .map_err(|_| lex.parse_error("expected condition"))?;
    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;
    lex.expect_block()?;

    let mut entries = vec![(
        Some(condition),
        parse_block(&mut lex.subblock_lexer(false), nested_ctx(showif, ctx))?,
    )];
    lex.advance();
    while !lex.eob {
        let state = lex.checkpoint();
        if lex.keyword("elif".into()).is_some() {
            let condition = lex
                .python_expression()
                .map_err(|_| lex.parse_error("expected condition"))?;
            lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
            lex.expect_eol()?;
            lex.expect_block()?;
            entries.push((
                Some(condition),
                parse_block(&mut lex.subblock_lexer(false), nested_ctx(showif, ctx))?,
            ));
            lex.advance();
            continue;
        }

        if lex.keyword("else".into()).is_some() {
            lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
            lex.expect_eol()?;
            lex.expect_block()?;
            entries.push((
                None,
                parse_block(&mut lex.subblock_lexer(false), nested_ctx(showif, ctx))?,
            ));
            lex.advance();
            break;
        }

        lex.revert(state);
        break;
    }

    if showif {
        Ok(slast::Node::ShowIf(slast::ShowIf { loc, entries }))
    } else {
        Ok(slast::Node::If(slast::If { loc, entries }))
    }
}

fn nested_ctx(showif: bool, ctx: BlockContext<'_>) -> BlockContext<'_> {
    if showif {
        BlockContext {
            container_name: "showif",
            allow_break_continue: ctx.allow_break_continue,
            property_context: ctx.property_context,
        }
    } else {
        ctx
    }
}

fn parse_for(lex: &mut Lexer, loc: (PathBuf, usize), ctx: BlockContext<'_>) -> Result<slast::Node> {
    lex.skip_whitespace();
    let target_start = lex.pos;
    let mut target_end = None;

    while !lex.eol() {
        let probe_pos = lex.pos;
        let state = lex.checkpoint();
        if lex.keyword("index".into()).is_some() {
            target_end = Some(probe_pos);
            lex.revert(state);
            break;
        }
        if lex.keyword("in".into()).is_some() {
            target_end = Some(probe_pos);
            lex.revert(state);
            break;
        }
        lex.revert(state);
        lex.pos += 1;
    }

    let target_end = target_end.ok_or_else(|| lex.parse_error("expected 'in' in for statement"))?;
    let target = lex.text[target_start..target_end].trim().to_string();
    lex.pos = target_end;

    let index_expression = if lex.keyword("index".into()).is_some() {
        Some(take_python_until_keyword(lex, "in")?)
    } else {
        None
    };

    if lex.keyword("in".into()).is_none() {
        return Err(lex.parse_error("expected 'in'"));
    }

    let iterable = lex.python_expression()?;
    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;
    lex.expect_block()?;
    let block = parse_block(
        &mut lex.subblock_lexer(false),
        BlockContext {
            container_name: "for",
            allow_break_continue: true,
            property_context: ctx.property_context,
        },
    )?;
    lex.advance();

    Ok(slast::Node::For(slast::For {
        loc,
        target,
        index_expression,
        iterable,
        block,
    }))
}

fn parse_python_block(lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<slast::Node> {
    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;
    lex.expect_block()?;
    let source = lex
        .python_block()
        .ok_or_else(|| lex.parse_error("expected python block"))?;
    lex.advance();
    Ok(slast::Node::Python(slast::Python {
        loc,
        source,
        block: true,
    }))
}

fn parse_default(lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<slast::Node> {
    let name = lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected name")?;
    lex.require_or_error(LexerType::String("=".into()), "expected '='")?;
    let expr = lex
        .rest()
        .ok_or_else(|| lex.parse_error("expected expression"))?;
    lex.expect_noblock()?;
    lex.advance();
    Ok(slast::Node::Default(slast::Default_ { loc, name, expr }))
}

fn parse_use(lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<slast::Node> {
    let (target, pass_context) = if lex.keyword("expression".into()).is_some() {
        let expr = lex
            .simple_expression(false, true)?
            .ok_or_else(|| lex.parse_error("expected screen expression"))?;
        let pass_context = lex.keyword("pass".into()).is_some();
        (slast::UseTarget::Expression(expr), pass_context)
    } else {
        let name = lex.require_or_error(
            LexerType::Type(LexerTypeOptions::Word),
            "expected screen name",
        )?;
        (slast::UseTarget::Name(name), false)
    };

    let arguments = parse_arguments(lex)?;
    let mut id_expr = None;
    let mut variable = None;

    while !lex.eol() {
        if lex.keyword("id".into()).is_some() {
            if id_expr.is_some() {
                return Err(
                    lex.parse_error("the id keyword may only appear once in a use statement")
                );
            }
            id_expr = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected id expression"))?,
            );
            continue;
        }
        if lex.keyword("as".into()).is_some() {
            if variable.is_some() {
                return Err(
                    lex.parse_error("the as keyword may only appear once in a use statement")
                );
            }
            variable = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Word),
                "expected variable name",
            )?);
            continue;
        }
        break;
    }

    let block = if lex.rmatch(RegexType::Simple(":")).is_some() {
        lex.expect_eol()?;
        lex.expect_block()?;
        Some(parse_block(
            &mut lex.subblock_lexer(false),
            BlockContext {
                container_name: "use",
                allow_break_continue: false,
                property_context: Some(PropertyContext {
                    owner: "use",
                    allow_default_properties: false,
                    allowed_names: Some(USE_BLOCK_PROPERTY_NAMES),
                }),
            },
        )?)
    } else {
        lex.expect_eol()?;
        lex.expect_noblock()?;
        None
    };

    lex.advance();
    Ok(slast::Node::Use(slast::Use {
        loc,
        target,
        arguments,
        id_expr,
        variable,
        pass_context,
        block,
    }))
}

fn parse_displayable(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
    spec: DisplayableSpec,
) -> Result<slast::Node> {
    let mut displayable = slast::Displayable {
        loc: loc.clone(),
        name: spec.name.to_string(),
        positional: vec![],
        properties: vec![],
        variable: None,
        atl_transform: None,
        children: vec![],
        layout_child: None,
    };

    for _ in spec.positional {
        let state = lex.checkpoint();
        if let Some(expr) = lex.simple_expression(false, true)? {
            displayable.positional.push(expr);
        } else {
            lex.revert(state);
            break;
        }
    }

    let mut seen_properties = HashSet::new();
    while !lex.eol() {
        if lex.rmatch(RegexType::Simple(":")).is_some() {
            lex.expect_eol()?;
            lex.expect_block()?;
            parse_displayable_block(lex, &mut displayable, spec)?;
            lex.advance();
            return Ok(slast::Node::Displayable(displayable));
        }

        if lex.keyword("as".into()).is_some() {
            if displayable.variable.is_some() {
                return Err(lex.parse_error(format!(
                    "an as clause may only appear once in a {} statement",
                    spec.name
                )));
            }
            displayable.variable = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Word),
                "expected variable name",
            )?);
            continue;
        }

        let state = lex.checkpoint();
        if lex.keyword("at".into()).is_some() {
            if lex.keyword("transform".into()).is_some()
                && lex.rmatch(RegexType::Simple(":")).is_some()
            {
                if displayable.atl_transform.is_some() {
                    return Err(lex.parse_error("more than one 'at transform' block is given"));
                }
                lex.expect_eol()?;
                lex.expect_block()?;
                displayable.atl_transform = Some(parse_atl(&mut lex.subblock_lexer(false))?);
                lex.advance();
                return Ok(slast::Node::Displayable(displayable));
            }
        }
        lex.revert(state);

        parse_named_property(
            lex,
            &mut displayable.properties,
            &mut seen_properties,
            spec.default_properties,
            spec.name,
            None,
        )?;
    }

    lex.expect_noblock()?;
    lex.expect_eol()?;
    lex.advance();
    Ok(slast::Node::Displayable(displayable))
}

fn parse_displayable_block(
    lex: &mut Lexer,
    displayable: &mut slast::Displayable,
    spec: DisplayableSpec,
) -> Result<()> {
    let mut sub = lex.subblock_lexer(false);
    let mut seen_properties = displayable
        .properties
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<HashSet<_>>();

    if !sub.advance() {
        return Ok(());
    }

    while !sub.eob {
        let state = sub.checkpoint();
        if sub.keyword("has".into()).is_some() {
            if spec.nchildren != ChildCount::One {
                return Err(sub.parse_error("the has statement is not allowed here"));
            }
            if displayable.layout_child.is_some() {
                return Err(sub.parse_error("the has statement may only appear once"));
            }
            displayable.layout_child = Some(Box::new(parse_node(
                &mut sub,
                BlockContext {
                    container_name: spec.name,
                    allow_break_continue: false,
                    property_context: Some(PropertyContext {
                        owner: spec.name,
                        allow_default_properties: spec.default_properties,
                        allowed_names: None,
                    }),
                },
            )?));
            continue;
        }
        sub.revert(state);

        let state = sub.checkpoint();
        if sub.keyword("at".into()).is_some() {
            if sub.keyword("transform".into()).is_some()
                && sub.rmatch(RegexType::Simple(":")).is_some()
            {
                if displayable.atl_transform.is_some() {
                    return Err(sub.parse_error("more than one 'at transform' block is given"));
                }
                sub.expect_eol()?;
                sub.expect_block()?;
                displayable.atl_transform = Some(parse_atl(&mut sub.subblock_lexer(false))?);
                sub.advance();
                continue;
            }
        }
        sub.revert(state);

        if try_parse_property_line(
            &mut sub,
            &mut displayable.properties,
            &mut seen_properties,
            spec.default_properties,
            spec.name,
            None,
        )? {
            continue;
        }

        let node = parse_node(
            &mut sub,
            BlockContext {
                container_name: spec.name,
                allow_break_continue: false,
                property_context: Some(PropertyContext {
                    owner: spec.name,
                    allow_default_properties: spec.default_properties,
                    allowed_names: None,
                }),
            },
        )?;

        if spec.nchildren == ChildCount::Zero
            && matches!(
                node,
                slast::Node::Displayable(_)
                    | slast::Node::Use(_)
                    | slast::Node::Transclude(_)
            )
        {
            return Err(sub.parse_error(format!("{} does not take children", spec.name)));
        }

        displayable.children.push(node);
    }

    Ok(())
}

fn parse_block(lex: &mut Lexer, ctx: BlockContext<'_>) -> Result<slast::Block> {
    let mut block = slast::Block::default();
    let mut seen_properties = HashSet::new();

    if !lex.advance() {
        return Ok(block);
    }

    while !lex.eob {
        if let Some(property_ctx) = ctx.property_context {
            if try_parse_property_line(
                lex,
                &mut block.properties,
                &mut seen_properties,
                property_ctx.allow_default_properties,
                property_ctx.owner,
                property_ctx.allowed_names,
            )? {
                continue;
            }
        }

        block.children.push(parse_node(lex, ctx)?);
    }

    Ok(block)
}

fn try_parse_property_line(
    lex: &mut Lexer,
    properties: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
    allow_default_properties: bool,
    owner: &str,
    allowed_names: Option<&[&str]>,
) -> Result<bool> {
    let state = lex.checkpoint();
    let Some(word) = lex.word() else {
        return Ok(false);
    };

    if displayable_spec(&word).is_some() || RESERVED_CHILD_NAMES.contains(&word.as_str()) {
        lex.revert(state);
        return Ok(false);
    }

    lex.revert(state);
    while !lex.eol() {
        parse_named_property(
            lex,
            properties,
            seen,
            allow_default_properties,
            owner,
            allowed_names,
        )?;
    }
    lex.expect_eol()?;
    lex.expect_noblock()?;
    lex.advance();
    Ok(true)
}

fn parse_named_property(
    lex: &mut Lexer,
    properties: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
    allow_default_properties: bool,
    owner: &str,
    allowed_names: Option<&[&str]>,
) -> Result<()> {
    let name = lex.require_or_error(
        LexerType::Type(LexerTypeOptions::Word),
        "expected property name",
    )?;
    let allowed_names = allowed_names.unwrap_or(SCREEN_PROPERTY_NAMES);
    if !allow_default_properties && !allowed_names.contains(&name.as_str()) {
        return Err(lex.parse_error(format!(
            "{name:?} is not a keyword argument or valid child of the {owner} statement."
        )));
    }
    if !seen.insert(name.clone()) {
        return Err(lex.parse_error(format!(
            "keyword argument {name:?} appears more than once in a {owner} statement"
        )));
    }

    let expr = if name == "tag" {
        lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected tag name")?
    } else if name == "arguments" {
        arguments_to_string(
            &parse_arguments(lex)?.ok_or_else(|| lex.parse_error("expected arguments"))?,
        )
    } else {
        lex.simple_expression(true, true)?.ok_or_else(|| {
            lex.parse_error(format!("the {name} keyword argument was not given a value"))
        })?
    };

    properties.push((name, expr));
    Ok(())
}

fn arguments_to_string(arguments: &ArgumentInfo) -> String {
    let mut parts = vec![];

    for (index, (keyword, expression)) in arguments.arguments.iter().enumerate() {
        let expr = expression.as_deref().unwrap_or("");
        if arguments.starred_indexes.contains(&index) {
            parts.push(format!("*{expr}"));
        } else if arguments.doublestarred_indexes.contains(&index) {
            parts.push(format!("**{expr}"));
        } else if let Some(keyword) = keyword {
            parts.push(format!("{keyword}={expr}"));
        } else {
            parts.push(expr.to_string());
        }
    }

    format!("({})", parts.join(", "))
}

fn displayable_spec(name: &str) -> Option<DisplayableSpec> {
    DISPLAYABLE_SPECS
        .iter()
        .copied()
        .find(|spec| spec.name == name)
}

fn take_python_until_keyword(lex: &mut Lexer, keyword: &str) -> Result<String> {
    lex.skip_whitespace();
    let start = lex.pos;

    while !lex.eol() {
        let probe_pos = lex.pos;
        let probe = lex.checkpoint();
        if lex.keyword(keyword.into()).is_some() {
            lex.revert(probe);
            let expr = lex.text[start..probe_pos].trim().to_string();
            if expr.is_empty() {
                return Err(lex.parse_error(format!("expected expression before '{keyword}'")));
            }
            lex.pos = probe_pos;
            return Ok(expr);
        }
        lex.revert(probe);

        let state = lex.checkpoint();
        if lex.python_string()? || lex.parenthesised_python()? {
            continue;
        }
        lex.revert(state);
        lex.pos += 1;
    }

    Err(lex.parse_error(format!("expected '{keyword}'")))
}
