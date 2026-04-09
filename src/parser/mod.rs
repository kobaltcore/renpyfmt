use crate::{
    ast::{
        ArgumentInfo, AstNode, Call, Camera, CompileIf, Default_, Define, EarlyPython,
        EndTranslate, Hide, If, Image, ImageSpecifier, Init, Jump, Label, Menu, Parameter,
        ParameterKind, ParameterSignature, Pass, Python, PythonOneLine, RPY, Return, Say, Scene,
        Screen, Show, ShowLayer, Style, Testcase, Testsuite, Transform, Translate, TranslateBlock,
        TranslateEarlyBlock, TranslateString, UserStatement, While, With,
    },
    atl::{
        AtlStatement, RawBlock, RawChild, RawChoice, RawContainsExpr, RawEvent, RawFunction,
        RawMultipurpose, RawOn, RawParallel, RawRepeat, RawTime,
    },
    error::Result,
    lexer::{Lexer, LexerType, LexerTypeOptions, RegexType},
};
use std::{
    collections::{HashMap, HashSet},
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
};

mod keywords;
mod registry;
mod statements_flow;
mod statements_init;
mod statements_media;
mod statements_translate;
#[cfg(test)]
mod tests;

use self::keywords::{ATL_PROPERTIES, ATL_WARPERS, STYLE_PROPERTIES};

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "parser panicked".into()
    }
}

fn with_parse_error_boundary<T>(
    lex: &mut Lexer,
    f: impl FnOnce(&mut Lexer) -> Result<T>,
) -> Result<T> {
    match catch_unwind(AssertUnwindSafe(|| f(lex))) {
        Ok(result) => result,
        Err(payload) => Err(lex.parse_error(panic_message(payload))),
    }
}

pub trait Parser {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes>;
}

#[derive(Debug, Clone)]
pub enum ParseNodes {
    None,
    One(AstNode),
    Many(Vec<AstNode>),
}

impl ParseNodes {
    pub fn into_vec(self) -> Vec<AstNode> {
        match self {
            ParseNodes::None => vec![],
            ParseNodes::One(node) => vec![node],
            ParseNodes::Many(nodes) => nodes,
        }
    }
}

impl From<AstNode> for ParseNodes {
    fn from(value: AstNode) -> Self {
        ParseNodes::One(value)
    }
}

impl From<Vec<AstNode>> for ParseNodes {
    fn from(value: Vec<AstNode>) -> Self {
        match value.len() {
            0 => ParseNodes::None,
            1 => ParseNodes::One(value.into_iter().next().expect("length checked above")),
            _ => ParseNodes::Many(value),
        }
    }
}

pub fn parse_statement(lex: &mut Lexer) -> Result<ParseNodes> {
    let parser = registry::new_parser();

    with_parse_error_boundary(lex, |lex| parser.parse(lex))
}
pub fn parse_block(lex: &mut Lexer) -> Result<Vec<AstNode>> {
    lex.advance();

    let mut result = vec![];

    let parser = registry::new_parser();

    // println!("parsing block: {:?} {} {}", lex.text, lex.pos, lex.eob);

    while !lex.eob {
        // println!("parsing: {:?}", lex.text);
        let stmt = with_parse_error_boundary(lex, |lex| parser.parse(lex))?;
        result.extend(stmt.into_vec());
    }

    Ok(result)
}

fn parse_parameters(lex: &mut Lexer) -> Result<Option<ParameterSignature>> {
    if lex.rmatch(r"\(".into()).is_none() {
        return Ok(None);
    }

    let mut parameters = HashMap::new();

    let mut got_slash = false;
    let mut now_kwonly = false;
    let mut kind = ParameterKind::PositionalOrKeyword;
    let mut missing_kwonly = false;
    let mut now_default = false;

    while lex.rmatch(r"\)".into()).is_none() {
        if lex.rmatch(r"\*\*".into()).is_some() {
            let extrakw = lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Name),
                "expected parameter name",
            )?;

            if parameters.contains_key(&extrakw) {
                return Err(lex.parse_error(format!("duplicate parameter name: {}", extrakw)));
            }

            parameters.insert(
                extrakw.clone(),
                Parameter {
                    name: extrakw.clone(),
                    kind: ParameterKind::VarKeyword,
                    default: None,
                },
            );

            if lex.rmatch(r"=".into()).is_some() {
                return Err(lex.parse_error(format!(
                    "a var-keyword parameter (**{extrakw}) cannot have a default value"
                )));
            }

            lex.rmatch(r",".into());

            if lex.rmatch(r"\)".into()).is_none() {
                return Err(lex.parse_error(format!(
                    "no parameter can follow a var-keyword parameter (**{extrakw})"
                )));
            }

            break;
        } else if lex.rmatch(r"\*".into()).is_some() {
            if now_kwonly {
                return Err(lex.parse_error("* may appear only once"));
            }

            now_kwonly = true;
            kind = ParameterKind::VarPositional;
            now_default = false;

            match lex.name() {
                Some(extrapos) => {
                    if parameters.contains_key(&extrapos) {
                        return Err(
                            lex.parse_error(format!("duplicate parameter name: {extrapos}"))
                        );
                    }

                    parameters.insert(
                        extrapos.clone(),
                        Parameter {
                            name: extrapos.clone(),
                            kind: ParameterKind::VarPositional,
                            default: None,
                        },
                    );

                    if lex.rmatch(r"=".into()).is_some() {
                        return Err(lex.parse_error(format!(
                            "a var-positional parameter (*{extrapos}) cannot have a default value"
                        )));
                    }
                }
                None => {
                    missing_kwonly = true;
                }
            };
        } else if lex.rmatch(r"/\*".into()).is_some() {
            return Err(lex.parse_error("expected comma between / and *"));
        } else if lex.rmatch(r"/".into()).is_some() {
            if now_kwonly {
                return Err(lex.parse_error("/ must be ahead of *"));
            } else if got_slash {
                return Err(lex.parse_error("/ may appear only once"));
            } else if parameters.is_empty() {
                return Err(lex.parse_error("at least one parameter must precede /"));
            }

            let mut new_parameters = HashMap::new();
            for (k, v) in parameters {
                new_parameters.insert(
                    k,
                    Parameter {
                        kind: ParameterKind::PositionalOnly,
                        ..v
                    },
                );
            }
            parameters = new_parameters;

            got_slash = true;
        } else {
            let name = lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Name),
                "expected parameter name",
            )?;

            missing_kwonly = false;

            let mut default = None;

            if lex.rmatch(r"=".into()).is_some() {
                lex.skip_whitespace();
                default = lex.delimited_python("),".into(), false);
                now_default = true;

                if default.is_none() {
                    return Err(
                        lex.parse_error(format!("empty default value for parameter {name}"))
                    );
                }
            } else if now_default && !now_kwonly {
                return Err(lex.parse_error(format!(
                    "non-default parameter {name} follows a default parameter"
                )));
            }

            if parameters.contains_key(&name) {
                return Err(lex.parse_error(format!("duplicate parameter name: {}", name)));
            }

            parameters.insert(
                name.clone(),
                Parameter {
                    name,
                    kind: kind.clone(),
                    default,
                },
            );
        }

        if lex.rmatch(r"\)".into()).is_some() {
            break;
        }

        lex.require_or_error(LexerType::String(",".into()), "expected ','")?;
    }

    if missing_kwonly {
        return Err(lex.parse_error("a bare * must be followed by a parameter"));
    }

    Ok(Some(ParameterSignature { parameters }))
}

fn parse_label(lex: &mut Lexer, loc: (PathBuf, usize), init: bool) -> Result<Vec<AstNode>> {
    let name = lex.require_or_error(
        LexerType::Type(LexerTypeOptions::LabelNameDeclare),
        "expected label name",
    )?;
    lex.set_global_label(Some(name.clone()));
    let parameters = parse_parameters(lex)?;

    let hide = lex.keyword("hide".into()).is_some();

    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;

    let block = parse_block(&mut lex.subblock_lexer(init))?;

    lex.advance();

    Ok(vec![AstNode::Label(Label {
        loc,
        name,
        block,
        parameters,
        hide,
        statement_start: None,
    })])
}

fn parse_image_name(lex: &mut Lexer, string: bool, nodash: bool) -> Result<Option<Vec<String>>> {
    let mut points = vec![lex.checkpoint()];
    let Some(first) = lex.require(LexerType::Type(LexerTypeOptions::ImageNameComponent)) else {
        return Ok(None);
    };
    let mut rv = vec![first];

    loop {
        points.push(lex.checkpoint());

        let n = lex.image_name_component();

        if n.is_none() {
            points.pop();
            break;
        }

        rv.push(n.expect("image_name_component checked above").trim().into());
    }

    if string {
        points.push(lex.checkpoint());

        match lex.simple_expression(false, true) {
            Some(s) => {
                rv.push(s);
            }
            None => {
                points.pop();
            }
        };
    }

    if nodash {
        for (i, p) in rv.iter().zip(points) {
            if i.len() > 0 && i.chars().nth(0) == Some('-') {
                lex.revert(p);
                lex.skip_whitespace();
                return Err(lex.parse_error("image name components may not begin with a '-'."));
            }
        }
    }

    Ok(Some(rv))
}

fn parse_simple_expression_list(lex: &mut Lexer) -> Result<Vec<String>> {
    let mut rv = vec![lex.require_or_error(
        LexerType::Type(LexerTypeOptions::SimpleExpression),
        "expected simple expression",
    )?];

    loop {
        if lex.rmatch(",".into()).is_none() {
            break;
        }

        let e = lex.simple_expression(false, true);

        if e.is_none() {
            break;
        }

        rv.push(e.expect("simple_expression checked above"));
    }

    Ok(rv)
}

fn parse_image_specifier(lex: &mut Lexer) -> Result<ImageSpecifier> {
    let mut tag = None;
    let mut layer = None;
    let mut at_list = vec![];
    let mut zorder = None;
    let mut behind = vec![];
    let expression;
    let image_name: Option<Vec<String>>;

    if lex.keyword("expression".into()).is_some() || lex.keyword("image".into()).is_some() {
        expression = Some(lex.require_or_error(
            LexerType::Type(LexerTypeOptions::SimpleExpression),
            "expected simple expression",
        )?);
        image_name = Some(vec![
            expression
                .clone()
                .expect("expression set above")
                .trim()
                .into(),
        ]);
    } else {
        image_name = parse_image_name(lex, true, false)?;
        expression = None;
    }

    loop {
        if lex.keyword("onlayer".into()).is_some() {
            if layer.is_some() {
                return Err(lex.parse_error("multiple onlayer clauses are prohibited."));
            } else {
                layer = Some(
                    lex.require_or_error(LexerType::Type(LexerTypeOptions::Name), "expected name")?,
                );
            }
            continue;
        }

        // println!("pos before at: {}", lex.pos);
        if lex.keyword("at".into()).is_some() {
            // println!("pos after at: {}", lex.pos);
            if at_list.len() > 0 {
                return Err(lex.parse_error("multiple at clauses are prohibited."));
            } else {
                // println!("requiring simple expression");
                at_list = parse_simple_expression_list(lex)?;
            }
            continue;
        }

        if lex.keyword("as".into()).is_some() {
            if tag.is_some() {
                return Err(lex.parse_error("multiple as clauses are prohibited."));
            } else {
                tag = Some(
                    lex.require_or_error(LexerType::Type(LexerTypeOptions::Name), "expected name")?,
                );
            }
            continue;
        }

        if lex.keyword("zorder".into()).is_some() {
            if zorder.is_some() {
                return Err(lex.parse_error("multiple zorder clauses are prohibited."));
            } else {
                zorder = Some(lex.require_or_error(
                    LexerType::Type(LexerTypeOptions::SimpleExpression),
                    "expected simple expression",
                )?);
            }
            continue;
        }

        if lex.keyword("behind".into()).is_some() {
            if behind.len() > 0 {
                return Err(lex.parse_error("multiple behind clauses are prohibited."));
            }

            loop {
                let bhtag =
                    lex.require_or_error(LexerType::Type(LexerTypeOptions::Name), "expected name")?;
                behind.push(bhtag);
                if lex.rmatch(",".into()).is_none() {
                    break;
                }
            }

            continue;
        }

        break;
    }

    Ok(ImageSpecifier {
        image_name: image_name.ok_or_else(|| lex.parse_error("expected image name"))?,
        expression,
        tag,
        at_list,
        layer,
        zorder,
        behind,
    })
}

fn parse_with(lex: &mut Lexer, node: AstNode) -> Result<Vec<AstNode>> {
    let loc = lex.get_location();

    if lex.keyword("with".into()).is_none() {
        return Ok(vec![node]);
    }

    let expr = lex.require_or_error(
        LexerType::Type(LexerTypeOptions::SimpleExpression),
        "expected simple expression",
    )?;

    Ok(vec![
        AstNode::With(With {
            loc: loc.clone(),
            expr: "None".into(),
            paired: Some(expr.clone()),
        }),
        node,
        AstNode::With(With {
            loc,
            expr,
            paired: None,
        }),
    ])
}

fn parse_atl(lex: &mut Lexer) -> Result<RawBlock> {
    lex.advance();

    let block_loc = lex.get_location();

    let mut statements: Vec<Option<AtlStatement>> = vec![];

    let mut animation = false;

    while !lex.eob {
        // println!("loop");
        let loc = lex.get_location();

        if lex.keyword("repeat".into()).is_some() {
            let repeats = lex.simple_expression(false, true);
            statements.push(Some(AtlStatement::RawRepeat(RawRepeat { loc, repeats })));
        } else if lex.keyword("block".into()).is_some() {
            lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
            lex.expect_eol()?;
            lex.expect_block()?;

            let block = parse_atl(&mut lex.subblock_lexer(false))?;
            statements.push(Some(AtlStatement::RawBlock(block)));
        } else if lex.keyword("contains".into()).is_some() {
            match lex.simple_expression(false, true) {
                Some(expr) => {
                    lex.expect_noblock()?;
                    statements.push(Some(AtlStatement::RawContainsExpr(RawContainsExpr {
                        loc,
                        expr,
                    })));
                }
                None => {
                    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
                    lex.expect_eol()?;
                    lex.expect_block()?;

                    let block = parse_atl(&mut lex.subblock_lexer(false))?;
                    statements.push(Some(AtlStatement::RawChild(RawChild { loc, child: block })));
                }
            }
        } else if lex.keyword("parallel".into()).is_some() {
            lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
            lex.expect_eol()?;
            lex.expect_block()?;

            let block = parse_atl(&mut lex.subblock_lexer(false))?;
            statements.push(Some(AtlStatement::RawParallel(RawParallel { loc, block })));
        } else if lex.keyword("choice".into()).is_some() {
            let mut chance = lex.simple_expression(false, true);

            if chance.is_none() {
                chance = Some("1.0".into());
            }

            lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
            lex.expect_eol()?;
            lex.expect_block()?;

            let block = parse_atl(&mut lex.subblock_lexer(false))?;
            statements.push(Some(AtlStatement::RawChoice(RawChoice {
                loc,
                chance: chance.ok_or_else(|| lex.parse_error("expected chance expression"))?,
                block,
            })));
        } else if lex.keyword("on".into()).is_some() {
            let mut names = vec![
                lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected word")?,
            ];

            while lex.rmatch(",".into()).is_some() {
                let name = lex.require(LexerType::Type(LexerTypeOptions::Word));

                if name.is_none() {
                    break;
                }

                names.push(name.expect("name checked above"));
            }

            lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
            lex.expect_eol()?;
            lex.expect_block()?;

            let block = parse_atl(&mut lex.subblock_lexer(false))?;
            statements.push(Some(AtlStatement::RawOn(RawOn { loc, names, block })));
        } else if lex.keyword("time".into()).is_some() {
            let time = lex.require_or_error(
                LexerType::Type(LexerTypeOptions::SimpleExpression),
                "expected simple expression",
            )?;
            lex.expect_noblock()?;

            statements.push(Some(AtlStatement::RawTime(RawTime { loc, time })));
        } else if lex.keyword("function".into()).is_some() {
            let expr = lex.require_or_error(
                LexerType::Type(LexerTypeOptions::SimpleExpression),
                "expected simple expression",
            )?;
            lex.expect_noblock()?;

            statements.push(Some(AtlStatement::RawFunction(RawFunction { loc, expr })));
        } else if lex.keyword("event".into()).is_some() {
            let name =
                lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected word")?;
            lex.expect_noblock()?;

            statements.push(Some(AtlStatement::RawEvent(RawEvent { loc, name })));
        } else if lex.keyword("pass".into()).is_some() {
            lex.expect_noblock()?;
            statements.push(None);
        } else if lex.keyword("animation".into()).is_some() {
            lex.expect_noblock()?;
            animation = true;
        } else {
            let mut rm = RawMultipurpose::new(loc);

            let mut last_expression = false;
            let mut this_expression = false;

            let mut cp = lex.checkpoint();
            let mut warper = lex.name();

            let duration;
            let warp_function;

            if let Some(ref warper_name) = warper {
                if ATL_WARPERS.contains(warper_name.as_str()) {
                    duration = Some(lex.require_or_error(
                        LexerType::Type(LexerTypeOptions::SimpleExpression),
                        "expected simple expression",
                    )?);
                    warp_function = None;
                } else if warper == Some("warp".into()) {
                    warper = None;
                    warp_function = Some(lex.require_or_error(
                        LexerType::Type(LexerTypeOptions::SimpleExpression),
                        "expected simple expression",
                    )?);
                    duration = Some(lex.require_or_error(
                        LexerType::Type(LexerTypeOptions::SimpleExpression),
                        "expected simple expression",
                    )?);
                } else {
                    lex.revert(cp);

                    warper = None;
                    warp_function = None;
                    duration = None;
                }
            } else {
                lex.revert(cp);

                warper = None;
                warp_function = None;
                duration = None;
            }

            rm.add_warper(warper.clone(), duration, warp_function);

            // let mut lex = lex;
            let mut ll = lex.clone();
            let mut has_block = false;
            let mut first_lex = true;

            loop {
                // println!("loop lexer {} {}", lex.text, lex.pos);
                if warper.is_some() && !has_block && ll.rmatch(":".into()).is_some() {
                    // println!("block");
                    ll.expect_eol()?;
                    ll.expect_block()?;
                    has_block = true;
                    if first_lex {
                        // forward the original lexer
                        let sub_cp = ll.checkpoint();
                        lex.revert(sub_cp);
                    }
                    first_lex = false;
                    ll = lex.subblock_lexer(false);
                    ll.advance();
                    ll.expect_noblock()?;
                }

                if has_block && ll.eol() {
                    // println!("block end");
                    ll.advance();
                    ll.expect_noblock()?;
                }

                last_expression = this_expression;
                this_expression = false;

                if ll.keyword("pass".into()).is_some() {
                    // println!("pass");
                    continue;
                }

                if ll.keyword("clockwise".into()).is_some() {
                    // println!("clockwise");
                    rm.add_revolution("clockwise".into());
                    continue;
                }

                if ll.keyword("counterclockwise".into()).is_some() {
                    // println!("counterclockwise");
                    rm.add_revolution("counterclockwise".into());
                    continue;
                }

                if ll.keyword("circles".into()).is_some() {
                    // println!("circles");
                    let expr = lex.require_or_error(
                        LexerType::Type(LexerTypeOptions::SimpleExpression),
                        "expected simple expression",
                    )?;
                    rm.add_circles(expr.into());
                    continue;
                }

                cp = ll.checkpoint();

                match ll.name() {
                    Some(prop) => {
                        // println!("try parsing as property: {:?}", prop);
                        if ATL_PROPERTIES.contains(prop.as_str()) || prop.starts_with("u_") {
                            let expr = ll.require_or_error(
                                LexerType::Type(LexerTypeOptions::SimpleExpression),
                                "expected simple expression",
                            )?;

                            let mut knots = vec![];

                            while ll.keyword("knot".into()).is_some() {
                                knots.push(ll.require_or_error(
                                    LexerType::Type(LexerTypeOptions::SimpleExpression),
                                    "expected simple expression",
                                )?);
                            }

                            if knots.len() > 0 {
                                if prop == "orientation" {
                                    return Err(
                                        lex.parse_error("Orientation doesn't support spline.")
                                    );
                                }
                                // println!("add spline");
                                knots.push(expr);
                                rm.add_spline(prop, knots);
                            } else {
                                // println!("add property");
                                let addprop_rv = rm.add_property(prop.clone(), expr);

                                if addprop_rv == Some(prop.clone()) {
                                    return Err(lex.parse_error(format!(
                                        "property {prop} is given a value more than once"
                                    )));
                                } else if let Some(conflict) = addprop_rv {
                                    return Err(lex.parse_error(format!(
                                        "properties {prop} and {} conflict with each other",
                                        conflict
                                    )));
                                }
                            }

                            // println!("continue to next iter");
                            continue;
                        }
                    }
                    None => {}
                }

                // println!("try parsing as simple expression: {:?}", &ll.text[ll.pos..]);

                ll.revert(cp);

                let expr = ll.simple_expression(false, true);

                if expr.is_none() {
                    // println!("no simple expression");
                    break;
                }

                // println!("found simple expression");

                if last_expression {
                    return Err(lex.parse_error("ATL statement contains two expressions in a row; is one of them a misspelled property? If not, separate them with pass."));
                }

                this_expression = true;

                let mut with_expr = None;
                if ll.keyword("with".into()).is_some() {
                    // println!("with");
                    with_expr = Some(ll.require_or_error(
                        LexerType::Type(LexerTypeOptions::SimpleExpression),
                        "expected simple expression",
                    )?);
                }

                // println!("add expression");
                rm.add_expression(expr.expect("expression checked above"), with_expr);
            }

            if !has_block {
                // println!("expect noblock");
                lex.expect_noblock()?;
            }

            // println!("add raw multipurpose");
            statements.push(Some(AtlStatement::RawMultipurpose(rm)));

            let sub_cp = ll.checkpoint();
            lex.revert(sub_cp);
        }

        if lex.eol() {
            lex.advance();
            continue;
        }

        lex.require_or_error(
            LexerType::String(",".into()),
            "expected comma or end of line",
        )?;
    }

    // let merged = vec![];

    Ok(RawBlock {
        loc: block_loc,
        statements,
        animation,
    })
}

fn parse_arguments(lex: &mut Lexer) -> Result<Option<ArgumentInfo>> {
    if lex.rmatch(r"\(".into()).is_none() {
        return Ok(None);
    }

    let mut arguments = vec![];
    let mut starred_indexes = HashSet::new();
    let mut doublestarred_indexes = HashSet::new();

    let mut index: usize = 0;
    let mut keyword_parsed = false;
    let mut names = HashSet::new();

    loop {
        let mut expect_starred = false;
        let mut expect_doublestarred = false;
        let mut name = None;

        if lex.rmatch(r"\)".into()).is_some() {
            break;
        }

        if lex.rmatch(r"\*\*".into()).is_some() {
            expect_doublestarred = true;
            doublestarred_indexes.insert(index);
        } else if lex.rmatch(r"\*".into()).is_some() {
            expect_starred = true;
            starred_indexes.insert(index);
        }

        let state = lex.checkpoint();

        if !(expect_starred || expect_doublestarred) {
            name = lex.word();

            if name.is_some()
                && lex.rmatch(r"=".into()).is_some()
                && lex.rmatch(r"=".into()).is_none()
            {
                let name_value = name.clone().expect("name checked above");
                if names.contains(&name_value) {
                    return Err(
                        lex.parse_error(format!("keyword argument repeated: '{}'", name_value))
                    );
                } else {
                    names.insert(name_value);
                }
                keyword_parsed = true;
            } else if keyword_parsed {
                return Err(lex.parse_error("positional argument follows keyword argument"));
            } else {
                lex.revert(state);
                name = None;
            }
        }

        lex.skip_whitespace();
        arguments.push((name, lex.delimited_python("),".into(), false)));

        if lex.rmatch(r"\)".into()).is_some() {
            break;
        }

        lex.require_or_error(LexerType::String(",".into()), "expected ','")?;
        index += 1;
    }

    Ok(Some(ArgumentInfo {
        arguments,
        starred_indexes,
        doublestarred_indexes,
    }))
}

fn finish_say(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
    who: Option<String>,
    what: Vec<String>,
    attributes: Option<Vec<String>>,
    temporary_attributes: Option<Vec<String>>,
    interact: bool,
) -> Result<Option<Vec<AstNode>>> {
    if what.len() == 0 {
        return Ok(None);
    }

    let mut with = None;
    let mut arguments = None;
    let mut identifier = None;
    let mut interact = interact;

    loop {
        if lex.keyword("nointeract".into()).is_some() {
            interact = false;
        } else if lex.keyword("with".into()).is_some() {
            if with.is_some() {
                return Err(lex.parse_error("say can only take a single with clause"));
            }
            with = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::SimpleExpression),
                "expected simple expression",
            )?);
        } else if lex.keyword("id".into()).is_some() {
            identifier = Some(
                lex.require_or_error(LexerType::Type(LexerTypeOptions::Name), "expected name")?,
            );
        } else {
            let args = parse_arguments(lex)?;

            if args.is_none() {
                break;
            }

            if arguments.is_some() {
                return Err(lex.parse_error("say can only take a single set of arguments"));
            }

            arguments = args;
        }
    }

    if what.len() == 1 {
        return Ok(Some(vec![AstNode::Say(Say {
            loc,
            who,
            what: what[0].clone(),
            with,
            interact,
            attributes,
            arguments,
            temporary_attributes,
            identifier,
        })]));
    }

    let mut result = vec![];

    for i in what {
        if i == "{clear}" {
            result.push(AstNode::UserStatement(UserStatement {
                loc: loc.clone(),
                line: "nvl clear".into(),
                block: vec![],
                parsed: false, // TODO: this is a placeholder, figure this out later
                code_block: None,
            }));
        } else {
            result.push(AstNode::Say(Say {
                loc: loc.clone(),
                who: who.clone(),
                what: i,
                with: with.clone(),
                interact: interact.clone(),
                attributes: attributes.clone(),
                arguments: arguments.clone(),
                temporary_attributes: temporary_attributes.clone(),
                identifier: identifier.clone(),
            }))
        }
    }

    Ok(Some(result))
}

fn say_attributes(lex: &mut Lexer) -> Option<Vec<String>> {
    let mut attributes = vec![];

    loop {
        let mut prefix = lex.rmatch(r"-".into());
        if prefix.is_none() {
            prefix = Some("".into());
        }

        let component = lex.image_name_component();

        if component.is_none() {
            break;
        }

        attributes.push(format!(
            "{}{}",
            prefix.expect("default prefix set above"),
            component.expect("component checked above")
        ));
    }

    if attributes.len() > 0 {
        return Some(attributes);
    }

    None
}

enum UserStatementBlock {
    True,
    False,
    Script,
}

fn parse_menu(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
    arguments: Option<ArgumentInfo>,
) -> Result<Vec<AstNode>> {
    let mut l = lex.subblock_lexer(false);

    let mut has_choice = false;

    let mut say_ast = None;
    let mut has_caption = false;

    let mut with_ = None;
    let mut set = None;

    let mut items: Vec<(Option<String>, Option<String>, Option<Vec<AstNode>>)> = vec![];
    let mut item_arguments = vec![];

    while l.advance() {
        if l.keyword("with".into()).is_some() {
            with_ = Some(l.require_or_error(
                LexerType::Type(LexerTypeOptions::SimpleExpression),
                "expected simple expression",
            )?);
            l.expect_eol()?;
            l.expect_noblock()?;
            continue;
        }

        if l.keyword("set".into()).is_some() {
            set = Some(l.require_or_error(
                LexerType::Type(LexerTypeOptions::SimpleExpression),
                "expected simple expression",
            )?);
            l.expect_eol()?;
            l.expect_noblock()?;
            continue;
        }

        let state = l.checkpoint();

        let who = l.simple_expression(false, true);

        let attributes = say_attributes(&mut l);

        let temporary_attributes = if l.rmatch(r"\@".into()).is_some() {
            say_attributes(&mut l)
        } else {
            None
        };

        let what = match l.triple_string() {
            Some(s) => s,
            None => match l.string() {
                Some(s) => vec![s],
                None => vec![],
            },
        };

        if who.is_some() && what.len() > 0 {
            if has_caption {
                return Err(
                    lex.parse_error("Say menuitems and captions may not exist in the same menu.")
                );
            }

            if say_ast.is_some() {
                return Err(lex.parse_error("Only one say menuitem may exist per menu."));
            }

            say_ast = finish_say(
                &mut l,
                loc.clone(),
                who,
                what,
                attributes,
                temporary_attributes,
                false,
            )?;

            l.expect_eol()?;
            l.expect_noblock()?;
            continue;
        }

        l.revert(state);

        let label = l.string();

        if label.is_none() {
            return Err(lex.parse_error("expected menuitem"));
        }

        if l.eol() {
            if l.subblock.len() > 0 {
                return Err(lex.parse_error("Line is followed by a block, despite not being a menu choice. Did you forget a colon at the end of the line?"));
            }

            if label.is_some() && say_ast.is_some() {
                return Err(
                    lex.parse_error("Captions and say menuitems may not exist in the same menu.")
                );
            }

            if label.is_some() {
                has_caption = true;
            }

            items.push((label, None, None));
            item_arguments.push(None);

            continue;
        }

        has_choice = true;

        let mut condition = None;

        item_arguments.push(parse_arguments(&mut l)?);

        if l.keyword("if".into()).is_some() {
            condition = Some(l.python_expression()?);
        }

        l.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        l.expect_eol()?;
        l.expect_block()?;

        let block = parse_block(&mut l.subblock_lexer(false))?;

        items.push((label, condition, Some(block)));
    }

    if !has_choice {
        return Err(lex.parse_error("Menu does not contain any choices."));
    }

    let mut rv = vec![];

    if let Some(say_ast) = say_ast.clone() {
        rv.push(say_ast[0].clone());
    }

    rv.push(AstNode::Menu(Menu {
        loc,
        items,
        set,
        with_,
        has_caption: say_ast.is_some() || has_caption,
        arguments,
        item_arguments,
        statement_start: None,
    }));

    Ok(rv)
}

fn parse_clause(rv: &mut Style, lex: &mut Lexer) -> Result<bool> {
    if lex.keyword("is".into()).is_some() {
        if rv.parent.is_some() {
            return Err(lex.parse_error("parent clause appears twice."));
        }
        rv.parent =
            Some(lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected word")?);
        return Ok(true);
    }

    if lex.keyword("clear".into()).is_some() {
        rv.clear = true;
        return Ok(true);
    }

    if lex.keyword("take".into()).is_some() {
        if rv.take.is_some() {
            return Err(lex.parse_error("take clause appears twice."));
        }
        rv.take =
            Some(lex.require_or_error(LexerType::Type(LexerTypeOptions::Name), "expected name")?);
        return Ok(true);
    }

    if lex.keyword("del".into()).is_some() {
        let propname =
            lex.require_or_error(LexerType::Type(LexerTypeOptions::Name), "expected name")?;

        if !STYLE_PROPERTIES.contains(propname.as_str()) {
            return Err(lex.parse_error(format!("style property {} is not known.", propname)));
        }

        rv.delattr.push(propname);
        return Ok(true);
    }

    if lex.keyword("variant".into()).is_some() {
        if rv.variant.is_some() {
            return Err(lex.parse_error("variant clause appears twice."));
        }
        rv.variant = Some(lex.require_or_error(
            LexerType::Type(LexerTypeOptions::SimpleExpression),
            "expected simple expression",
        )?);
        return Ok(true);
    }

    let propname = lex.name();

    match propname {
        Some(pname) => {
            if pname != "properties" && !STYLE_PROPERTIES.contains(pname.as_str()) {
                return Err(lex.parse_error(format!("style property {} is not known.", pname)));
            }

            if rv.properties.contains_key(&pname) {
                return Err(lex.parse_error(format!("style property {} appears twice.", pname)));
            }

            rv.properties.insert(
                pname,
                lex.require_or_error(
                    LexerType::Type(LexerTypeOptions::SimpleExpression),
                    "expected simple expression",
                )?,
            );

            return Ok(true);
        }
        None => {}
    }

    Ok(false)
}

fn parse_translate_strings(
    lex: &mut Lexer,
    init_loc: (PathBuf, usize),
    language: Option<String>,
) -> Result<Vec<AstNode>> {
    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;
    lex.expect_block()?;

    let mut ll = lex.subblock_lexer(false);
    let mut block = vec![];
    let mut old = None;
    let mut old_loc = None;

    while ll.advance() {
        if ll.keyword("old".into()).is_some() {
            if old.is_some() {
                return Err(ll.parse_error("previous string is missing a translation"));
            }

            old_loc = Some(ll.get_location());
            old = Some(
                ll.rest()
                    .ok_or_else(|| ll.parse_error("Could not parse string."))?,
            );
        } else if ll.keyword("new".into()).is_some() {
            if old.is_none() {
                return Err(ll.parse_error("no string to translate"));
            }

            let new_loc = ll.get_location();
            let new = ll
                .rest()
                .ok_or_else(|| ll.parse_error("Could not parse string."))?;

            block.push(AstNode::TranslateString(TranslateString {
                loc: old_loc.clone().expect("old location set above"),
                language: language.clone(),
                old: old.take().expect("old string set above"),
                new,
                new_loc,
            }));

            old_loc = None;
        } else {
            return Err(ll.parse_error("unknown statement"));
        }
    }

    if old.is_some() {
        return Err(lex.parse_error("final string is missing a translation"));
    }

    lex.advance();

    if lex.init {
        Ok(block)
    } else {
        Ok(vec![AstNode::Init(Init {
            loc: init_loc,
            block,
            priority: lex.init_offset,
        })])
    }
}
