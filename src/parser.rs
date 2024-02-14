use crate::{
    ast::{
        ArgumentInfo, AstNode, Call, Default_, Define, EarlyPython, Hide, If, ImageSpecifier, Init,
        Jump, Label, Menu, Parameter, ParameterKind, ParameterSignature, Pass, Python,
        PythonOneLine, Return, Say, Scene, Show, Style, UserStatement, With,
    },
    atl::{
        AtlStatement, RawBlock, RawChild, RawChoice, RawContainsExpr, RawEvent, RawFunction,
        RawMultipurpose, RawOn, RawParallel, RawRepeat, RawTime,
    },
    lexer::{Lexer, LexerType, LexerTypeOptions},
    trie::ParseTrie,
};
use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

pub trait Parser {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>>;
}

pub fn parse_statement(lex: &mut Lexer) -> Result<Vec<AstNode>> {
    let mut parser = ParseTrie::new();
    parser.init();

    parser.parse(lex)
}
pub fn parse_block(lex: &mut Lexer) -> Result<Vec<AstNode>> {
    lex.advance();

    let mut result = vec![];

    let mut parser = ParseTrie::new();
    parser.init();

    // println!("parsing block: {:?} {} {}", lex.text, lex.pos, lex.eob);

    while !lex.eob {
        // println!("parsing: {:?}", lex.text);
        let stmt = parser.parse(lex)?;

        if stmt.len() == 1 {
            result.push(stmt[0].clone());
        } else {
            result.extend(stmt);
        }
    }

    Ok(result)
}

fn parse_parameters(lex: &mut Lexer) -> Option<ParameterSignature> {
    if lex.rmatch(r"\(".into()).is_none() {
        return None;
    }

    let mut parameters = HashMap::new();

    let mut got_slash = false;
    let mut now_kwonly = false;
    let mut kind = ParameterKind::PositionalOrKeyword;
    let mut missing_kwonly = false;
    let mut now_default = false;

    while lex.rmatch(r"\)".into()).is_none() {
        if lex.rmatch(r"\*\*".into()).is_some() {
            let extrakw = lex
                .require(LexerType::Type(LexerTypeOptions::Name))
                .unwrap();

            if parameters.contains_key(&extrakw) {
                panic!("duplicate parameter name: {}", extrakw);
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
                panic!("a var-keyword parameter (**{extrakw}) cannot have a default value");
            }

            lex.rmatch(r",".into());

            if lex.rmatch(r"\)".into()).is_none() {
                panic!("no parameter can follow a var-keyword parameter (**{extrakw})");
            }

            break;
        } else if lex.rmatch(r"\*".into()).is_some() {
            if now_kwonly {
                panic!("* may appear only once");
            }

            now_kwonly = true;
            kind = ParameterKind::VarPositional;
            now_default = false;

            match lex.name() {
                Some(extrapos) => {
                    if parameters.contains_key(&extrapos) {
                        panic!("duplicate parameter name: {extrapos}");
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
                        panic!(
                            "a var-positional parameter (*{extrapos}) cannot have a default value"
                        );
                    }
                }
                None => {
                    missing_kwonly = true;
                }
            };
        } else if lex.rmatch(r"/\*".into()).is_some() {
            panic!("expected comma between / and *");
        } else if lex.rmatch(r"/".into()).is_some() {
            if now_kwonly {
                panic!("/ must be ahead of *");
            } else if got_slash {
                panic!("/ may appear only once");
            } else if parameters.is_empty() {
                panic!("at least one parameter must precede /");
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
            let name = lex
                .require(LexerType::Type(LexerTypeOptions::Name))
                .unwrap();

            missing_kwonly = false;

            let mut default = None;

            if lex.rmatch(r"=".into()).is_some() {
                lex.skip_whitespace();
                default = lex.delimited_python("),".into(), false);
                now_default = true;

                if default.is_none() {
                    panic!("empty default value for parameter {name}");
                }
            } else if now_default && !now_kwonly {
                panic!("non-default parameter {name} follows a default parameter");
            }

            if parameters.contains_key(&name) {
                panic!("duplicate parameter name: {}", name);
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

        lex.require(LexerType::String(",".into()));
    }

    if missing_kwonly {
        panic!("a bare * must be followed by a parameter");
    }

    Some(ParameterSignature { parameters })
}

impl Parser for Label {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let name = lex
            .require(LexerType::Type(LexerTypeOptions::LabelNameDeclare))
            .unwrap();
        lex.set_global_label(Some(name.clone()));
        let parameters = parse_parameters(lex);

        let hide = match lex.keyword("hide".into()) {
            Some(_) => true,
            None => false,
        };

        lex.require(LexerType::String(":".into()));
        lex.expect_eol();

        let block = parse_block(&mut lex.subblock_lexer(false))?;

        lex.advance();

        return Ok(vec![AstNode::Label(Label {
            loc,
            name,
            block,
            parameters,
            hide,
            statement_start: None,
        })]);
    }
}

fn parse_image_name(lex: &mut Lexer, string: bool, nodash: bool) -> Option<Vec<String>> {
    let mut points = vec![lex.checkpoint()];
    let mut rv = vec![lex
        .require(LexerType::Type(LexerTypeOptions::ImageNameComponent))
        .unwrap()];

    loop {
        points.push(lex.checkpoint());

        let n = lex.image_name_component();

        if n.is_none() {
            points.pop();
            break;
        }

        rv.push(n.unwrap().trim().into());
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
                panic!("image name components may not begin with a '-'.");
            }
        }
    }

    Some(rv)
}

fn parse_simple_expression_list(lex: &mut Lexer) -> Vec<String> {
    let mut rv = vec![lex
        .require(LexerType::Type(LexerTypeOptions::SimpleExpression))
        .unwrap()];

    loop {
        if lex.rmatch(",".into()).is_none() {
            break;
        }

        let e = lex.simple_expression(false, true);

        if e.is_none() {
            break;
        }

        rv.push(e.unwrap());
    }

    rv
}

fn parse_image_specifier(lex: &mut Lexer) -> ImageSpecifier {
    let mut tag = None;
    let mut layer = None;
    let mut at_list = vec![];
    let mut zorder = None;
    let mut behind = vec![];
    let expression;
    let image_name: Option<Vec<String>>;

    if lex.keyword("expression".into()).is_some() || lex.keyword("image".into()).is_some() {
        expression = lex.require(LexerType::Type(LexerTypeOptions::SimpleExpression));
        image_name = Some(vec![expression.clone().unwrap().trim().into()]);
    } else {
        image_name = parse_image_name(lex, true, false);
        expression = None;
    }

    loop {
        if lex.keyword("onlayer".into()).is_some() {
            if layer.is_some() {
                panic!("multiple onlayer clauses are prohibited.");
            } else {
                layer = lex.require(LexerType::Type(LexerTypeOptions::Name));
            }
            continue;
        }

        // println!("pos before at: {}", lex.pos);
        if lex.keyword("at".into()).is_some() {
            // println!("pos after at: {}", lex.pos);
            if at_list.len() > 0 {
                panic!("multiple at clauses are prohibited.");
            } else {
                // println!("requiring simple expression");
                at_list = parse_simple_expression_list(lex);
            }
            continue;
        }

        if lex.keyword("as".into()).is_some() {
            if tag.is_some() {
                panic!("multiple as clauses are prohibited.");
            } else {
                tag = lex.require(LexerType::Type(LexerTypeOptions::Name));
            }
            continue;
        }

        if lex.keyword("zorder".into()).is_some() {
            if zorder.is_some() {
                panic!("multiple zorder clauses are prohibited.");
            } else {
                zorder = lex.require(LexerType::Type(LexerTypeOptions::SimpleExpression));
            }
            continue;
        }

        if lex.keyword("behind".into()).is_some() {
            if behind.len() > 0 {
                panic!("multiple behind clauses are prohibited.");
            }

            loop {
                let bhtag = lex.require(LexerType::Type(LexerTypeOptions::Name));
                behind.push(bhtag.unwrap());
                if lex.rmatch(",".into()).is_none() {
                    break;
                }
            }

            continue;
        }

        break;
    }

    ImageSpecifier {
        image_name: image_name.unwrap(),
        expression,
        tag,
        at_list,
        layer,
        zorder,
        behind,
    }
}

fn parse_with(lex: &mut Lexer, node: AstNode) -> Vec<AstNode> {
    let loc = lex.get_location();

    if lex.keyword("with".into()).is_none() {
        return vec![node];
    }

    let expr = lex.require(LexerType::Type(LexerTypeOptions::SimpleExpression));

    vec![
        AstNode::With(With {
            loc: loc.clone(),
            expr: "None".into(),
            paired: expr.clone(),
        }),
        node,
        AstNode::With(With {
            loc,
            expr: expr.unwrap(),
            paired: None,
        }),
    ]
}

fn parse_atl(lex: &mut Lexer) -> Option<RawBlock> {
    lex.advance();

    let block_loc = lex.get_location();

    let mut statements: Vec<Option<AtlStatement>> = vec![];

    let mut animation = false;

    let warpers = [
        "instant".into(),
        "pause".into(),
        "linear".into(),
        "easeout".into(),
        "easein".into(),
        "ease".into(),
        "easeout_quad".into(),
        "easein_quad".into(),
        "ease_quad".into(),
        "easeout_cubic".into(),
        "easein_cubic".into(),
        "ease_cubic".into(),
        "easeout_quart".into(),
        "easein_quart".into(),
        "ease_quart".into(),
        "easeout_quint".into(),
        "easein_quint".into(),
        "ease_quint".into(),
        "easeout_expo".into(),
        "easein_expo".into(),
        "ease_expo".into(),
        "easeout_circ".into(),
        "easein_circ".into(),
        "ease_circ".into(),
        "easeout_back".into(),
        "easein_back".into(),
        "ease_back".into(),
        "easeout_elastic".into(),
        "easein_elastic".into(),
        "ease_elastic".into(),
        "easeout_bounce".into(),
        "easein_bounce".into(),
        "ease_bounce".into(),
    ];

    let properties = [
        "additive".into(),
        "alpha".into(),
        "blend".into(),
        "blur".into(),
        "corner1".into(),
        "corner2".into(),
        "crop".into(),
        "crop_relative".into(),
        "debug".into(),
        "delay".into(),
        "events".into(),
        "fit".into(),
        "matrixanchor".into(),
        "matrixcolor".into(),
        "matrixtransform".into(),
        "maxsize".into(),
        "mesh".into(),
        "mesh_pad".into(),
        "nearest".into(),
        "perspective".into(),
        "rotate".into(),
        "rotate_pad".into(),
        "point_to".into(),
        "orientation".into(),
        "xrotate".into(),
        "yrotate".into(),
        "zrotate".into(),
        "shader".into(),
        "show_cancels_hide".into(),
        "subpixel".into(),
        "transform_anchor".into(),
        "zoom".into(),
        "xanchoraround".into(),
        "xanchor".into(),
        "xaround".into(),
        "xoffset".into(),
        "xpan".into(),
        "xpos".into(),
        "xsize".into(),
        "xtile".into(),
        "xzoom".into(),
        "yanchoraround".into(),
        "yanchor".into(),
        "yaround".into(),
        "yoffset".into(),
        "ypan".into(),
        "ypos".into(),
        "ysize".into(),
        "ytile".into(),
        "yzoom".into(),
        "zpos".into(),
        "zzoom".into(),
        "gl_anisotropic".into(),
        "gl_blend_func".into(),
        "gl_color_mask".into(),
        "gl_depth".into(),
        "gl_drawable_resolution".into(),
        "gl_mipmap".into(),
        "gl_pixel_perfect".into(),
        "gl_texture_scaling".into(),
        "gl_texture_wrap".into(),
        "alignaround".into(),
        "align".into(),
        "anchor".into(),
        "anchorangle".into(),
        "anchoraround".into(),
        "anchorradius".into(),
        "angle".into(),
        "around".into(),
        "offset".into(),
        "pos".into(),
        "radius".into(),
        "size".into(),
        "xalign".into(),
        "xcenter".into(),
        "xycenter".into(),
        "xysize".into(),
        "yalign".into(),
        "ycenter".into(),
        "u_lod_bias".into(),
        "u_renpy_blur_log2".into(),
        "u_renpy_solid_color".into(),
        "u_renpy_dissolve".into(),
        "u_renpy_dissolve_offset".into(),
        "u_renpy_dissolve_multiplier".into(),
        "u_renpy_matrixcolor".into(),
        "u_renpy_alpha".into(),
        "u_renpy_over".into(),
        "u_renpy_mask_multiplier".into(),
        "u_renpy_mask_offset".into(),
    ];

    while !lex.eob {
        // println!("loop");
        let loc = lex.get_location();

        if lex.keyword("repeat".into()).is_some() {
            let repeats = lex.simple_expression(false, true);
            statements.push(Some(AtlStatement::RawRepeat(RawRepeat { loc, repeats })));
        } else if lex.keyword("block".into()).is_some() {
            lex.require(LexerType::String(":".into())).unwrap();
            lex.expect_eol();
            lex.expect_block();

            let block = parse_atl(&mut lex.subblock_lexer(false))?;
            statements.push(Some(AtlStatement::RawBlock(block)));
        } else if lex.keyword("contains".into()).is_some() {
            match lex.simple_expression(false, true) {
                Some(expr) => {
                    lex.expect_noblock();
                    statements.push(Some(AtlStatement::RawContainsExpr(RawContainsExpr {
                        loc,
                        expr,
                    })));
                }
                None => {
                    lex.require(LexerType::String(":".into())).unwrap();
                    lex.expect_eol();
                    lex.expect_block();

                    let block = parse_atl(&mut lex.subblock_lexer(false))?;
                    statements.push(Some(AtlStatement::RawChild(RawChild { loc, child: block })));
                }
            }
        } else if lex.keyword("parallel".into()).is_some() {
            lex.require(LexerType::String(":".into())).unwrap();
            lex.expect_eol();
            lex.expect_block();

            let block = parse_atl(&mut lex.subblock_lexer(false))?;
            statements.push(Some(AtlStatement::RawParallel(RawParallel { loc, block })));
        } else if lex.keyword("choice".into()).is_some() {
            let mut chance = lex.simple_expression(false, true);

            if chance.is_none() {
                chance = Some("1.0".into());
            }

            lex.require(LexerType::String(":".into())).unwrap();
            lex.expect_eol();
            lex.expect_block();

            let block = parse_atl(&mut lex.subblock_lexer(false))?;
            statements.push(Some(AtlStatement::RawChoice(RawChoice {
                loc,
                chance: chance.unwrap(),
                block,
            })));
        } else if lex.keyword("on".into()).is_some() {
            let mut names = vec![lex.require(LexerType::Type(LexerTypeOptions::Word))?];

            while lex.rmatch(",".into()).is_some() {
                let name = lex.require(LexerType::Type(LexerTypeOptions::Word));

                if name.is_none() {
                    break;
                }

                names.push(name.unwrap());
            }

            lex.require(LexerType::String(":".into())).unwrap();
            lex.expect_eol();
            lex.expect_block();

            let block = parse_atl(&mut lex.subblock_lexer(false))?;
            statements.push(Some(AtlStatement::RawOn(RawOn { loc, names, block })));
        } else if lex.keyword("time".into()).is_some() {
            let time = lex
                .require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                .unwrap();
            lex.expect_noblock();

            statements.push(Some(AtlStatement::RawTime(RawTime { loc, time })));
        } else if lex.keyword("function".into()).is_some() {
            let expr = lex
                .require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                .unwrap();
            lex.expect_noblock();

            statements.push(Some(AtlStatement::RawFunction(RawFunction { loc, expr })));
        } else if lex.keyword("event".into()).is_some() {
            let name = lex
                .require(LexerType::Type(LexerTypeOptions::Word))
                .unwrap();
            lex.expect_noblock();

            statements.push(Some(AtlStatement::RawEvent(RawEvent { loc, name })));
        } else if lex.keyword("pass".into()).is_some() {
            lex.expect_noblock();
            statements.push(None);
        } else if lex.keyword("animation".into()).is_some() {
            lex.expect_noblock();
            animation = true;
        } else {
            let mut rm = RawMultipurpose::new(loc);

            let mut last_expression = false;
            let mut this_expression = false;

            let mut cp = lex.checkpoint();
            let mut warper = lex.name();

            let duration;
            let warp_function;

            if warper.is_some() && warpers.contains(&warper.clone().unwrap()) {
                duration = Some(
                    lex.require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                        .unwrap(),
                );
                warp_function = None;
            } else if warper == Some("warp".into()) {
                warper = None;
                warp_function = Some(
                    lex.require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                        .unwrap(),
                );
                duration = Some(
                    lex.require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                        .unwrap(),
                );
            } else {
                lex.revert(cp);

                warper = None;
                warp_function = None;
                // duration = Some("0".into());
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
                    ll.expect_eol();
                    ll.expect_block();
                    has_block = true;
                    if first_lex {
                        // forward the original lexer
                        let sub_cp = ll.checkpoint();
                        lex.revert(sub_cp);
                    }
                    first_lex = false;
                    ll = lex.subblock_lexer(false);
                    ll.advance();
                    ll.expect_noblock();
                }

                if has_block && ll.eol() {
                    // println!("block end");
                    ll.advance();
                    ll.expect_noblock();
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
                    let expr = lex
                        .require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                        .unwrap();
                    rm.add_circles(expr.into());
                    continue;
                }

                cp = ll.checkpoint();

                match ll.name() {
                    Some(prop) => {
                        // println!("try parsing as property: {:?}", prop);
                        if properties.contains(&prop) || prop.starts_with("u_") {
                            let expr = ll
                                .require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                                .unwrap();

                            let mut knots = vec![];

                            while ll.keyword("knot".into()).is_some() {
                                knots.push(
                                    ll.require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                                        .unwrap(),
                                );
                            }

                            if knots.len() > 0 {
                                if prop == "orientation" {
                                    panic!("Orientation doesn't support spline.")
                                }
                                // println!("add spline");
                                knots.push(expr);
                                rm.add_spline(prop, knots);
                            } else {
                                // println!("add property");
                                let addprop_rv = rm.add_property(prop.clone(), expr);

                                if addprop_rv == Some(prop.clone()) {
                                    panic!("property {prop} is given a value more than once");
                                } else if addprop_rv.is_some() {
                                    panic!(
                                        "properties {prop} and {} conflict with each other",
                                        addprop_rv?
                                    );
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
                    panic!("ATL statement contains two expressions in a row; is one of them a misspelled property? If not, separate them with pass.");
                }

                this_expression = true;

                let mut with_expr = None;
                if ll.keyword("with".into()).is_some() {
                    // println!("with");
                    with_expr = Some(
                        ll.require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                            .unwrap(),
                    );
                }

                // println!("add expression");
                rm.add_expression(expr?, with_expr);
            }

            if !has_block {
                // println!("expect noblock");
                lex.expect_noblock();
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

        lex.require(LexerType::String(",".into()))
            .expect("comma or end of line");
    }

    // let merged = vec![];

    Some(RawBlock {
        loc: block_loc,
        statements,
        animation,
    })
}

impl Parser for Scene {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let mut layer = None;

        if lex.keyword("onlayer".into()).is_some() {
            layer = lex.require(LexerType::Type(LexerTypeOptions::Name));
            lex.expect_eol();
        }

        if layer.is_some() || lex.eol() {
            lex.advance();
            return Ok(vec![AstNode::Scene(Scene {
                loc,
                imspec: None,
                layer,
                atl: None,
            })]);
        }

        let imspec = parse_image_specifier(lex);
        let stmt = Scene {
            loc,
            imspec: Some(imspec.clone()),
            layer: imspec.layer,
            atl: None,
        };
        let mut rv = parse_with(lex, AstNode::Scene(stmt.clone()));

        if lex.rmatch(":".into()).is_some() {
            lex.expect_block();
            // println!("parsing ATL {:?}", rv);
            match &mut rv[0] {
                AstNode::Scene(node) => {
                    node.atl = parse_atl(&mut lex.subblock_lexer(false));
                    // println!("atl: {:?}", node.atl);
                }
                _ => {}
            }
        } else {
            lex.expect_noblock();
        }

        lex.expect_eol();
        lex.advance();

        Ok(rv)
    }
}

impl Parser for With {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let expr = lex
            .require(LexerType::Type(LexerTypeOptions::SimpleExpression))
            .unwrap();
        lex.expect_eol();
        lex.expect_noblock();
        lex.advance();

        Ok(vec![AstNode::With(With {
            loc,
            expr,
            paired: None,
        })])
    }
}

fn parse_arguments(lex: &mut Lexer) -> Option<ArgumentInfo> {
    if lex.rmatch(r"\(".into()).is_none() {
        return None;
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
                if names.contains(&name.clone().unwrap()) {
                    panic!("keyword argument repeated: '{}'", name.clone().unwrap());
                } else {
                    names.insert(name.clone().unwrap());
                }
                keyword_parsed = true;
            } else if keyword_parsed {
                panic!("positional argument follows keyword argument");
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

        lex.require(LexerType::String(",".into()));
        index += 1;
    }

    Some(ArgumentInfo {
        arguments,
        starred_indexes,
        doublestarred_indexes,
    })
}

fn finish_say(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
    who: Option<String>,
    what: Vec<String>,
    attributes: Option<Vec<String>>,
    temporary_attributes: Option<Vec<String>>,
    interact: bool,
) -> Option<Vec<AstNode>> {
    if what.len() == 0 {
        return None;
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
                panic!("say can only take a single with clause");
            }
            with = lex.require(LexerType::Type(LexerTypeOptions::SimpleExpression));
        } else if lex.keyword("id".into()).is_some() {
            identifier = lex.require(LexerType::Type(LexerTypeOptions::Name));
        } else {
            let args = parse_arguments(lex);

            if args.is_none() {
                break;
            }

            if arguments.is_some() {
                panic!("say can only take a single set of arguments");
            }

            arguments = args;
        }
    }

    if what.len() == 1 {
        return Some(vec![AstNode::Say(Say {
            loc,
            who,
            what: what[0].clone(),
            with,
            interact,
            attributes,
            arguments,
            temporary_attributes,
            identifier,
        })]);
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

    Some(result)
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

        attributes.push(format!("{}{}", prefix.unwrap(), component.unwrap()));
    }

    if attributes.len() > 0 {
        return Some(attributes);
    }

    None
}

impl Parser for Say {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let state = lex.checkpoint();
        // println!("{} {}", lex.pos, lex.text);

        let what = match lex.triple_string() {
            Some(s) => s,
            None => match lex.string() {
                Some(s) => vec![s],
                None => vec![],
            },
        };

        let rv = finish_say(lex, loc.clone(), None, what, None, None, true);

        if rv.is_some() {
            lex.expect_noblock();
            lex.advance();
            return Ok(rv.unwrap());
        }

        lex.revert(state);

        let who = lex.say_expression();

        // println!("who: {:?}", who);

        let attributes = say_attributes(lex);

        let temporary_attributes = if lex.rmatch(r"\@".into()).is_some() {
            say_attributes(lex)
        } else {
            None
        };

        let what = match lex.triple_string() {
            Some(s) => s,
            None => match lex.string() {
                Some(s) => vec![s],
                None => vec![],
            },
        };

        if who.is_some() && what.len() > 0 {
            let rv = finish_say(
                lex,
                loc,
                Some(who.unwrap().trim().to_string()),
                what,
                attributes,
                temporary_attributes,
                true,
            )
            .unwrap();

            lex.expect_eol();
            lex.expect_noblock();
            lex.advance();

            return Ok(rv);
        }

        panic!("expected statement.")
    }
}

enum UserStatementBlock {
    True,
    False,
    Script,
}

impl Parser for UserStatement {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let old_subparses = lex.subparses.clone();

        lex.subparses = vec![];

        let text = lex.text.clone();
        let subblock = lex.subblock.clone();

        let mut code_block = None;

        let block = UserStatementBlock::False;

        match block {
            UserStatementBlock::True => lex.expect_block(),
            UserStatementBlock::False => lex.expect_noblock(),
            UserStatementBlock::Script => {
                lex.expect_block();
                code_block = Some(parse_block(&mut lex.subblock_lexer(false))?);
            }
        };

        let start_line = lex.line;

        // TODO: run custom parse functions here
        // let parsed = (name, parse(l));

        if lex.line == start_line {
            lex.advance();
        }

        let rv = UserStatement {
            loc,
            line: text,
            block: subblock,
            parsed: true, // TODO: store actual parsed info here
            code_block,
        };

        Ok(vec![AstNode::UserStatement(rv)])
    }
}

impl Parser for Show {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let imspec = parse_image_specifier(lex);
        let stmt = Show {
            loc,
            imspec: Some(imspec.clone()),
            atl: None,
        };
        let mut rv = parse_with(lex, AstNode::Show(stmt));

        if lex.rmatch(":".into()).is_some() {
            lex.expect_block();
            // println!("parsing ATL");
            match &mut rv[0] {
                AstNode::Show(node) => {
                    node.atl = parse_atl(&mut lex.subblock_lexer(false));
                    // println!("atl: {:?}", node.atl);
                }
                _ => {}
            }
        } else {
            lex.expect_noblock();
        }

        lex.expect_eol();
        lex.advance();

        // println!("show {} {}", lex.pos, lex.text);
        // println!("stmts: {:?}", rv);

        Ok(rv)
    }
}

impl Parser for Hide {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let imspec = parse_image_specifier(lex);
        let rv = parse_with(
            lex,
            AstNode::Hide(Hide {
                loc,
                imgspec: imspec.clone(),
            }),
        );

        lex.expect_eol();
        lex.expect_noblock();
        lex.advance();

        Ok(rv)
    }
}

impl Parser for PythonOneLine {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let python_code = lex.rest_statement();

        if python_code.is_none() {
            panic!("expected python code");
        }

        lex.expect_noblock();
        lex.advance();

        Ok(vec![AstNode::PythonOneLine(PythonOneLine {
            loc,
            python_code: python_code.unwrap().trim().into(),
        })])
    }
}

impl Parser for Jump {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        lex.expect_noblock();

        let target;
        let expression;
        if lex.keyword("expression".into()).is_some() {
            expression = true;
            target = lex
                .require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                .unwrap();
        } else {
            expression = false;
            target = lex
                .require(LexerType::Type(LexerTypeOptions::LabelName))
                .unwrap();
        }

        lex.expect_eol();
        lex.advance();

        let mut global_label = None;

        if expression && lex.global_label.is_some() {
            global_label = lex.global_label.clone();
        }

        Ok(vec![AstNode::Jump(Jump {
            loc,
            target,
            expression,
            global_label,
        })])
    }
}

fn parse_menu(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
    arguments: Option<ArgumentInfo>,
) -> Vec<AstNode> {
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
            with_ = Some(
                l.require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                    .unwrap(),
            );
            l.expect_eol();
            l.expect_noblock();
            continue;
        }

        if l.keyword("set".into()).is_some() {
            set = Some(
                l.require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                    .unwrap(),
            );
            l.expect_eol();
            l.expect_noblock();
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
                panic!("Say menuitems and captions may not exist in the same menu.");
            }

            if say_ast.is_some() {
                panic!("Only one say menuitem may exist per menu.");
            }

            say_ast = finish_say(
                &mut l,
                loc.clone(),
                who,
                what,
                attributes,
                temporary_attributes,
                false,
            );

            l.expect_eol();
            l.expect_noblock();
            continue;
        }

        l.revert(state);

        let label = l.string();

        if label.is_none() {
            panic!("expected menuitem");
        }

        if l.eol() {
            if l.subblock.len() > 0 {
                panic!("Line is followed by a block, despite not being a menu choice. Did you forget a colon at the end of the line?");
            }

            if label.is_some() && say_ast.is_some() {
                panic!("Captions and say menuitems may not exist in the same menu.");
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

        item_arguments.push(parse_arguments(&mut l));

        if l.keyword("if".into()).is_some() {
            condition = Some(
                l.require(LexerType::Type(LexerTypeOptions::PythonExpression))
                    .unwrap(),
            );
        }

        l.require(LexerType::String(":".into())).unwrap();
        l.expect_eol();
        l.expect_block();

        let block = parse_block(&mut l.subblock_lexer(false)).unwrap();

        items.push((label, condition, Some(block)));
    }

    if !has_choice {
        panic!("Menu does not contain any choices.");
    }

    let mut rv = vec![];

    if say_ast.is_some() {
        let say_ast = say_ast.clone().unwrap();
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

    rv
}

impl Parser for Menu {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        lex.expect_block();
        let label = lex.label_name_declare();
        lex.set_global_label(label.clone());

        let arguments = parse_arguments(lex);

        lex.require(LexerType::String(":".into())).unwrap();
        lex.expect_eol();

        let menu = parse_menu(lex, loc.clone(), arguments);

        lex.advance();

        let mut rv = vec![];

        if label.is_some() {
            rv.push(AstNode::Label(Label {
                loc: loc,
                name: label.unwrap(),
                block: vec![],
                parameters: None,
                hide: false,
                statement_start: None,
            }));
        }

        rv.extend(menu);

        let first = rv[0].clone();

        for i in &mut rv {
            match i {
                AstNode::Label(node) => {
                    node.statement_start = Some(Box::new(first.clone()));
                }
                AstNode::Menu(node) => {
                    node.statement_start = Some(Box::new(first.clone()));
                }
                _ => {}
            }
        }

        Ok(rv)
    }
}

impl Parser for If {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let mut entries = vec![];

        let condition = lex
            .require(LexerType::Type(LexerTypeOptions::PythonExpression))
            .unwrap();
        lex.require(LexerType::String(":".into())).unwrap();
        lex.expect_eol();
        lex.expect_block();

        let block = parse_block(&mut lex.subblock_lexer(false)).unwrap();

        entries.push((Some(condition), block));

        lex.advance();

        while lex.keyword("elif".into()).is_some() {
            let condition = lex
                .require(LexerType::Type(LexerTypeOptions::PythonExpression))
                .unwrap();
            lex.require(LexerType::String(":".into())).unwrap();
            lex.expect_eol();
            lex.expect_block();

            let block = parse_block(&mut lex.subblock_lexer(false)).unwrap();

            entries.push((Some(condition), block));

            lex.advance();
        }

        if lex.keyword("else".into()).is_some() {
            lex.require(LexerType::String(":".into())).unwrap();
            lex.expect_eol();
            lex.expect_block();

            let block = parse_block(&mut lex.subblock_lexer(false)).unwrap();

            entries.push((None, block));

            lex.advance();
        }

        Ok(vec![AstNode::If(If { loc, entries })])
    }
}

impl Parser for Return {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        lex.expect_noblock();

        let rest = lex.rest();

        lex.expect_eol();
        lex.advance();

        Ok(vec![AstNode::Return(Return {
            loc,
            expression: rest,
        })])
    }
}

fn parse_clause(rv: &mut Style, lex: &mut Lexer) -> bool {
    let style_prefixed_all_properties: HashSet<String, _> = HashSet::from([
        "selected_hover_xpos".into(),
        "selected_hover_ypos".into(),
        "selected_insensitive_mipmap".into(),
        "child".into(),
        "insensitive_xoffset".into(),
        "idle_line_leading".into(),
        "idle_line_spacing".into(),
        "selected_xfit".into(),
        "selected_debug".into(),
        "insensitive_yoffset".into(),
        "selected_first_spacing".into(),
        "spacing".into(),
        "selected_activate_bottom_gutter".into(),
        "selected_idle_bar_resizing".into(),
        "idle_outline_scaling".into(),
        "idle_bottom_bar".into(),
        "selected_insensitive_size_group".into(),
        "selected_insensitive_hover_sound".into(),
        "idle_bar_resizing".into(),
        "bottom_padding".into(),
        "right_bar".into(),
        "selected_idle_bottom_bar".into(),
        "selected_insensitive_textalign".into(),
        "hover_strikethrough".into(),
        "selected_idle_xpos".into(),
        "selected_idle_ypos".into(),
        "selected_bottom_margin".into(),
        "idle_textalign".into(),
        "selected_anchor".into(),
        "selected_hover_box_first_spacing".into(),
        "slow_speed".into(),
        "selected_slow_cps".into(),
        "idle_drop_shadow".into(),
        "hover_xmargin".into(),
        "selected_subpixel".into(),
        "hover_ymargin".into(),
        "idle_xpadding".into(),
        "idle_ypadding".into(),
        "activate_drop_shadow_color".into(),
        "hover_order_reverse".into(),
        "selected_activate_unscrollable".into(),
        "yoffset".into(),
        "selected_hover_line_leading".into(),
        "selected_hover_line_spacing".into(),
        "insensitive_top_padding".into(),
        "selected_insensitive_first_indent".into(),
        "selected_italic".into(),
        "selected_insensitive_focus_mask".into(),
        "hover_xsize".into(),
        "hover_ysize".into(),
        "selected_activate_box_first_spacing".into(),
        "ruby_style".into(),
        "selected_hover_emoji_font".into(),
        "first_spacing".into(),
        "selected_bottom_padding".into(),
        "selected_insensitive_foreground".into(),
        "selected_idle_hinting".into(),
        "selected_insensitive_xoffset".into(),
        "selected_insensitive_yoffset".into(),
        "selected_insensitive_enable_hover".into(),
        "aft_gutter".into(),
        "selected_idle_size_group".into(),
        "drop_shadow".into(),
        "selected_hover_bottom_gutter".into(),
        "idle_fit_first".into(),
        "selected_hover_xcenter".into(),
        "selected_hover_ycenter".into(),
        "selected_activate_prefer_emoji".into(),
        "selected_idle_subtitle_width".into(),
        "selected_hover_caret".into(),
        "xcenter".into(),
        "insensitive_xalign".into(),
        "insensitive_yalign".into(),
        "hover_xalign".into(),
        "hover_yalign".into(),
        "selected_activate_child".into(),
        "hover_xminimum".into(),
        "hover_yminimum".into(),
        "selected_idle_slow_cps_multiplier".into(),
        "activate_min_width".into(),
        "selected_box_layout".into(),
        "selected_yfill".into(),
        "activate_right_margin".into(),
        "insensitive_key_events".into(),
        "min_width".into(),
        "selected_hover_hyperlink_functions".into(),
        "activate_sound".into(),
        "black_color".into(),
        "idle_xminimum".into(),
        "idle_yminimum".into(),
        "selected_activate_xoffset".into(),
        "selected_activate_yoffset".into(),
        "language".into(),
        "selected_activate_aft_bar".into(),
        "selected_activate_xanchor".into(),
        "selected_hover_kerning".into(),
        "selected_insensitive_xmaximum".into(),
        "selected_insensitive_ymaximum".into(),
        "selected_activate_yanchor".into(),
        "selected_activate_top_bar".into(),
        "idle_text_align".into(),
        "selected_activate_justify".into(),
        "selected_activate_hinting".into(),
        "selected_activate_kerning".into(),
        "selected_activate_spacing".into(),
        "selected_activate_padding".into(),
        "xfill".into(),
        "selected_activate_xmargin".into(),
        "selected_activate_ymargin".into(),
        "idle_left_padding".into(),
        "selected_activate_maximum".into(),
        "hover_size".into(),
        "idle_text_y_fudge".into(),
        "selected_activate_minimum".into(),
        "activate_subpixel".into(),
        "selected_idle_align".into(),
        "selected_hover_xfit".into(),
        "selected_hover_yfit".into(),
        "selected_hover_left_gutter".into(),
        "activate_slow_cps".into(),
        "selected_insensitive_black_color".into(),
        "selected_foreground".into(),
        "selected_idle_mipmap".into(),
        "idle_left_gutter".into(),
        "insensitive_altruby_style".into(),
        "selected_altruby_style".into(),
        "selected_insensitive_thumb_shadow".into(),
        "selected_insensitive_thumb_offset".into(),
        "selected_idle_bar_vertical".into(),
        "hover_thumb".into(),
        "insensitive_subpixel".into(),
        "selected_activate_first_indent".into(),
        "hover_drop_shadow".into(),
        "selected_first_indent".into(),
        "selected_idle_font".into(),
        "insensitive_xpos".into(),
        "align".into(),
        "selected_activate_anchor".into(),
        "selected_hover_sound".into(),
        "idle_caret".into(),
        "text_y_fudge".into(),
        "xmaximum".into(),
        "activate_first_indent".into(),
        "selected_fore_gutter".into(),
        "selected_activate_xysize".into(),
        "hover_thumb_shadow".into(),
        "top_padding".into(),
        "alt".into(),
        "activate_xmaximum".into(),
        "activate_xminimum".into(),
        "activate_hyperlink_functions".into(),
        "activate_xspacing".into(),
        "activate_xpadding".into(),
        "selected_hover_slow_speed".into(),
        "activate_xycenter".into(),
        "idle_xcenter".into(),
        "idle_ycenter".into(),
        "selected_layout".into(),
        "activate_axis".into(),
        "selected_idle_fit_first".into(),
        "selected_hover_box_layout".into(),
        "selected_insensitive_rest_indent".into(),
        "selected_insensitive_bar_resizing".into(),
        "selected_idle_layout".into(),
        "selected_hyperlink_functions".into(),
        "hover_language".into(),
        "hover_xanchor".into(),
        "hover_yanchor".into(),
        "selected_focus_rect".into(),
        "selected_activate_bold".into(),
        "selected_hover_justify".into(),
        "activate_area".into(),
        "insensitive_focus_mask".into(),
        "selected_caret".into(),
        "activate_spacing".into(),
        "pos".into(),
        "selected_insensitive_subpixel".into(),
        "idle_subpixel".into(),
        "hover_bottom_padding".into(),
        "hover_fore_bar".into(),
        "activate_yfit".into(),
        "selected_activate_min_width".into(),
        "selected_insensitive_color".into(),
        "insensitive_shaper".into(),
        "insensitive_offset".into(),
        "selected_activate_bar_invert".into(),
        "insensitive_hyperlink_functions".into(),
        "activate_ymaximum".into(),
        "activate_yminimum".into(),
        "insensitive_thumb_offset".into(),
        "selected_insensitive_slow_abortable".into(),
        "activate_yspacing".into(),
        "activate_ypadding".into(),
        "insensitive_slow_speed".into(),
        "selected_hover_xsize".into(),
        "selected_hover_ysize".into(),
        "insensitive_line_spacing".into(),
        "selected_spacing".into(),
        "hover_xycenter".into(),
        "insensitive_time_policy".into(),
        "hover_top_gutter".into(),
        "hover_underline".into(),
        "selected_activate_underline".into(),
        "insensitive_box_spacing".into(),
        "selected_hover_keyboard_focus".into(),
        "selected_insensitive_ruby_line_leading".into(),
        "idle_left_margin".into(),
        "selected_insensitive_xspacing".into(),
        "selected_insensitive_yspacing".into(),
        "hover_base_bar".into(),
        "selected_order_reverse".into(),
        "selected_activate_ycenter".into(),
        "selected_insensitive_spacing".into(),
        "insensitive_antialias".into(),
        "hover_line_leading".into(),
        "hover_line_spacing".into(),
        "selected_hinting".into(),
        "selected_idle_bottom_padding".into(),
        "activate_antialias".into(),
        "selected_activate_drop_shadow_color".into(),
        "selected_time_policy".into(),
        "idle_fore_gutter".into(),
        "selected_insensitive_mouse".into(),
        "selected_idle_extra_alt".into(),
        "hover_antialias".into(),
        "hover_xcenter".into(),
        "hover_ycenter".into(),
        "emoji_font".into(),
        "activate_text_y_fudge".into(),
        "idle_maximum".into(),
        "idle_minimum".into(),
        "selected_hover_min_width".into(),
        "ypadding".into(),
        "insensitive_hinting".into(),
        "insensitive_kerning".into(),
        "insensitive_spacing".into(),
        "insensitive_padding".into(),
        "activate_fore_bar".into(),
        "selected_idle_line_overlap_split".into(),
        "selected_activate_yalign".into(),
        "keyboard_focus".into(),
        "idle_slow_cps_multiplier".into(),
        "hover_bar_vertical".into(),
        "insensitive_drop_shadow_color".into(),
        "selected_hover_left_bar".into(),
        "activate_left_gutter".into(),
        "selected_idle_box_wrap_spacing".into(),
        "selected_color".into(),
        "idle_padding".into(),
        "idle_xalign".into(),
        "idle_yalign".into(),
        "selected_xalign".into(),
        "activate_ysize".into(),
        "selected_hover_text_y_fudge".into(),
        "selected_idle_clipping".into(),
        "selected_activate_black_color".into(),
        "hover_justify".into(),
        "unscrollable".into(),
        "xsize".into(),
        "selected_xysize".into(),
        "selected_idle_rest_indent".into(),
        "selected_idle_modal".into(),
        "hover_align".into(),
        "activate_aft_gutter".into(),
        "offset".into(),
        "selected_rest_indent".into(),
        "selected_activate_slow_speed".into(),
        "base_bar".into(),
        "hover_bottom_gutter".into(),
        "hover_first_spacing".into(),
        "activate_caret".into(),
        "selected_activate_time_policy".into(),
        "idle_subtitle_width".into(),
        "selected_hover_background".into(),
        "selected_insensitive_alt".into(),
        "selected_activate_xpos".into(),
        "selected_insensitive_left_bar".into(),
        "selected_insensitive_vertical".into(),
        "idle_box_wrap_spacing".into(),
        "idle_xoffset".into(),
        "idle_yoffset".into(),
        "selected_hover_thumb_offset".into(),
        "insensitive_enable_hover".into(),
        "selected_insensitive_emoji_font".into(),
        "selected_hover_italic".into(),
        "selected_hover_focus_rect".into(),
        "idle_clipping".into(),
        "idle_top_padding".into(),
        "selected_idle_xycenter".into(),
        "selected_left_margin".into(),
        "selected_outline_scaling".into(),
        "selected_keyboard_focus".into(),
        "selected_hover_focus_mask".into(),
        "idle_spacing".into(),
        "insensitive_axis".into(),
        "activate_padding".into(),
        "minimum".into(),
        "insensitive_aft_bar".into(),
        "insensitive_top_bar".into(),
        "idle_bar_invert".into(),
        "selected_top_margin".into(),
        "hover_size_group".into(),
        "selected_bottom_gutter".into(),
        "fore_bar".into(),
        "selected_activate_right_padding".into(),
        "selected_activate_right_gutter".into(),
        "selected_idle_focus_rect".into(),
        "selected_hover_instance".into(),
        "selected_idle_xcenter".into(),
        "selected_idle_ycenter".into(),
        "selected_hover_font".into(),
        "idle_hinting".into(),
        "selected_insensitive_hyperlink_functions".into(),
        "xycenter".into(),
        "right_margin".into(),
        "selected_hover_newline_indent".into(),
        "extra_alt".into(),
        "activate_left_margin".into(),
        "hover_slow_speed".into(),
        "insensitive_minwidth".into(),
        "selected_hover_xminimum".into(),
        "selected_hover_yminimum".into(),
        "activate_language".into(),
        "hover_margin".into(),
        "selected_yfit".into(),
        "instance".into(),
        "hover_offset".into(),
        "activate_left_bar".into(),
        "insensitive_bold".into(),
        "selected_hover_textalign".into(),
        "idle_debug".into(),
        "hover_adjust_spacing".into(),
        "selected_hover_axis".into(),
        "selected_hover_xoffset".into(),
        "selected_hover_yoffset".into(),
        "selected_hover_bottom_bar".into(),
        "idle_black_color".into(),
        "selected_insensitive_italic".into(),
        "insensitive_line_leading".into(),
        "insensitive_language".into(),
        "activate_bottom_bar".into(),
        "ymargin".into(),
        "selected_idle_minwidth".into(),
        "selected_insensitive_language".into(),
        "selected_pos".into(),
        "selected_insensitive_anchor".into(),
        "selected_alt".into(),
        "box_layout".into(),
        "selected_idle_altruby_style".into(),
        "box_reverse".into(),
        "selected_idle_thumb_offset".into(),
        "selected_activate_minwidth".into(),
        "idle_modal".into(),
        "insensitive_min_width".into(),
        "newline_indent".into(),
        "selected_idle_strikethrough".into(),
        "selected_hover_text_align".into(),
        "italic".into(),
        "hover_child".into(),
        "selected_activate_keyboard_focus".into(),
        "selected_hover_box_reverse".into(),
        "activate_kerning".into(),
        "hover_xfill".into(),
        "hover_yfill".into(),
        "selected_activate_offset".into(),
        "selected_hover_layout".into(),
        "selected_activate_fit_first".into(),
        "idle_bottom_margin".into(),
        "selected_hover_xanchor".into(),
        "selected_hover_yanchor".into(),
        "selected_box_spacing".into(),
        "hover_slow_cps_multiplier".into(),
        "insensitive_pos".into(),
        "slow_cps".into(),
        "activate_slow_cps_multiplier".into(),
        "selected_idle_vertical".into(),
        "selected_idle_left_margin".into(),
        "selected_idle_xmargin".into(),
        "selected_idle_ymargin".into(),
        "hover_subpixel".into(),
        "selected_activate_xsize".into(),
        "selected_idle_left_gutter".into(),
        "slow_cps_multiplier".into(),
        "selected_insensitive_outline_scaling".into(),
        "selected_activate_antialias".into(),
        "activate_box_wrap_spacing".into(),
        "insensitive_xanchor".into(),
        "insensitive_yanchor".into(),
        "selected_activate_bar_vertical".into(),
        "activate_prefer_emoji".into(),
        "selected_activate_pos".into(),
        "selected_idle_xsize".into(),
        "selected_idle_ysize".into(),
        "selected_insensitive_minwidth".into(),
        "insensitive_textalign".into(),
        "selected_idle_black_color".into(),
        "selected_hover_minwidth".into(),
        "selected_idle_box_first_spacing".into(),
        "insensitive_background".into(),
        "insensitive_foreground".into(),
        "idle_justify".into(),
        "hover_time_policy".into(),
        "selected_idle_area".into(),
        "insensitive_maximum".into(),
        "insensitive_minimum".into(),
        "prefer_emoji".into(),
        "idle_xanchor".into(),
        "idle_yanchor".into(),
        "hover_modal".into(),
        "selected_insensitive_hinting".into(),
        "hover_axis".into(),
        "hover_xfit".into(),
        "hover_yfit".into(),
        "selected_hover_first_spacing".into(),
        "insensitive_adjust_spacing".into(),
        "left_gutter".into(),
        "activate_pos".into(),
        "selected_insensitive_left_margin".into(),
        "selected_insensitive_margin".into(),
        "selected_idle_key_events".into(),
        "selected_activate_color".into(),
        "bar_vertical".into(),
        "selected_insensitive_maximum".into(),
        "selected_insensitive_minimum".into(),
        "selected_insensitive_xalign".into(),
        "selected_insensitive_yalign".into(),
        "selected_activate_caret".into(),
        "insensitive_left_margin".into(),
        "selected_hover_fore_gutter".into(),
        "selected_hover_xalign".into(),
        "selected_hover_yalign".into(),
        "selected_vertical".into(),
        "selected_idle_right_margin".into(),
        "selected_insensitive_layout".into(),
        "selected_hover_minimum".into(),
        "idle_fore_bar".into(),
        "idle_mouse".into(),
        "idle_base_bar".into(),
        "activate_minimum".into(),
        "selected_activate_left_padding".into(),
        "selected_hover_ymaximum".into(),
        "selected_hover_xmaximum".into(),
        "selected_activate_extra_alt".into(),
        "selected_activate_group_alt".into(),
        "activate_thumb_offset".into(),
        "selected_ymargin".into(),
        "idle_slow_cps".into(),
        "selected_insensitive_line_overlap_split".into(),
        "insensitive_fore_gutter".into(),
        "insensitive_left_gutter".into(),
        "selected_insensitive_xysize".into(),
        "insensitive_box_reverse".into(),
        "insensitive_ypos".into(),
        "activate_xmargin".into(),
        "selected_hover_xycenter".into(),
        "selected_insensitive_bottom_padding".into(),
        "color".into(),
        "selected_box_first_spacing".into(),
        "selected_insensitive_box_spacing".into(),
        "selected_ypos".into(),
        "hover_bar_invert".into(),
        "selected_box_wrap_spacing".into(),
        "activate_size_group".into(),
        "hover_ruby_style".into(),
        "selected_activate_xmaximum".into(),
        "selected_activate_xminimum".into(),
        "selected_activate_ymaximum".into(),
        "selected_activate_yminimum".into(),
        "activate_yoffset".into(),
        "selected_idle_color".into(),
        "selected_insensitive_order_reverse".into(),
        "selected_hover_right_bar".into(),
        "insensitive_bottom_gutter".into(),
        "selected_activate_language".into(),
        "selected_xpos".into(),
        "mouse".into(),
        "selected_activate_aft_gutter".into(),
        "selected_activate_top_gutter".into(),
        "minwidth".into(),
        "selected_fit_first".into(),
        "insensitive_bottom_margin".into(),
        "selected_idle_unscrollable".into(),
        "hover_enable_hover".into(),
        "activate_bottom_margin".into(),
        "selected_idle_subpixel".into(),
        "idle_emoji_font".into(),
        "idle_vertical".into(),
        "idle_background".into(),
        "selected_xanchor".into(),
        "hyperlink_functions".into(),
        "hover_italic".into(),
        "hover_fit_first".into(),
        "hover_fore_gutter".into(),
        "selected_insensitive_xsize".into(),
        "selected_insensitive_ysize".into(),
        "hover_aft_bar".into(),
        "selected_insensitive_aft_gutter".into(),
        "selected_insensitive_top_gutter".into(),
        "kerning".into(),
        "line_overlap_split".into(),
        "selected_group_alt".into(),
        "idle_thumb_offset".into(),
        "idle_thumb_shadow".into(),
        "selected_text_align".into(),
        "hover_line_overlap_split".into(),
        "insensitive_top_margin".into(),
        "activate_ruby_style".into(),
        "selected_adjust_spacing".into(),
        "selected_hover_slow_abortable".into(),
        "selected_idle_antialias".into(),
        "insensitive_extra_alt".into(),
        "idle_line_overlap_split".into(),
        "idle_aft_bar".into(),
        "idle_top_bar".into(),
        "enable_hover".into(),
        "selected_hover_child".into(),
        "selected_top_padding".into(),
        "selected_insensitive_altruby_style".into(),
        "foreground".into(),
        "selected_hover_top_gutter".into(),
        "selected_min_width".into(),
        "selected_idle_top_padding".into(),
        "selected_idle_margin".into(),
        "hover_caret".into(),
        "selected_hover_top_margin".into(),
        "subtitle_width".into(),
        "insensitive_strikethrough".into(),
        "insensitive_box_wrap_spacing".into(),
        "idle_size_group".into(),
        "xmargin".into(),
        "selected_hover_aft_gutter".into(),
        "insensitive_right_gutter".into(),
        "selected_hover_mouse".into(),
        "order_reverse".into(),
        "hover_aft_gutter".into(),
        "selected_bold".into(),
        "insensitive_outlines".into(),
        "selected_hover_align".into(),
        "insensitive_first_indent".into(),
        "selected_idle_order_reverse".into(),
        "selected_focus_mask".into(),
        "idle_group_alt".into(),
        "activate_child".into(),
        "activate_bold".into(),
        "hover_black_color".into(),
        "hover_area".into(),
        "selected_activate_line_overlap_split".into(),
        "insensitive_left_padding".into(),
        "selected_activate_fore_gutter".into(),
        "selected_activate_left_gutter".into(),
        "activate_shaper".into(),
        "slow_abortable".into(),
        "hover_ruby_line_leading".into(),
        "idle_drop_shadow_color".into(),
        "hover_mipmap".into(),
        "activate_aft_bar".into(),
        "idle_kerning".into(),
        "selected_idle_first_indent".into(),
        "selected_insensitive_box_wrap_spacing".into(),
        "selected_activate_strikethrough".into(),
        "selected_activate_thumb_offset".into(),
        "activate_color".into(),
        "insensitive_fore_bar".into(),
        "insensitive_left_bar".into(),
        "insensitive_base_bar".into(),
        "selected_activate_debug".into(),
        "selected_justify".into(),
        "idle_right_bar".into(),
        "hover_slow_cps".into(),
        "activate_background".into(),
        "activate_bottom_padding".into(),
        "selected_activate_italic".into(),
        "idle_align".into(),
        "selected_strikethrough".into(),
        "insensitive_text_align".into(),
        "insensitive_subtitle_width".into(),
        "activate_hover_sound".into(),
        "idle_xmaximum".into(),
        "idle_ymaximum".into(),
        "selected_idle_left_bar".into(),
        "yanchor".into(),
        "selected_insensitive_top_margin".into(),
        "selected_insensitive_right_bar".into(),
        "idle_pos".into(),
        "selected_line_overlap_split".into(),
        "idle_shaper".into(),
        "activate_emoji_font".into(),
        "insensitive_prefer_emoji".into(),
        "selected_hover_box_wrap_spacing".into(),
        "idle_slow_speed".into(),
        "hover_left_padding".into(),
        "left_margin".into(),
        "selected_hover_key_events".into(),
        "insensitive_mipmap".into(),
        "selected_thumb".into(),
        "selected_left_gutter".into(),
        "selected_hover_fore_bar".into(),
        "selected_idle_xoffset".into(),
        "selected_idle_yoffset".into(),
        "activate_align".into(),
        "idle_language".into(),
        "selected_insensitive_fore_bar".into(),
        "selected_insensitive_base_bar".into(),
        "hover_debug".into(),
        "idle_xycenter".into(),
        "activate_bar_invert".into(),
        "activate_layout".into(),
        "selected_insensitive_xminimum".into(),
        "selected_insensitive_yminimum".into(),
        "selected_newline_indent".into(),
        "hover_foreground".into(),
        "selected_idle_emoji_font".into(),
        "activate_focus_mask".into(),
        "fit_first".into(),
        "selected_idle_slow_speed".into(),
        "selected_hover_group_alt".into(),
        "hover_textalign".into(),
        "activate_bar_resizing".into(),
        "insensitive_outline_scaling".into(),
        "selected_xfill".into(),
        "selected_insensitive_box_first_spacing".into(),
        "hover_shaper".into(),
        "ruby_line_leading".into(),
        "selected_bottom_bar".into(),
        "selected_xsize".into(),
        "idle_first_indent".into(),
        "activate_newline_indent".into(),
        "selected_activate_bottom_margin".into(),
        "hover_sound".into(),
        "activate_yfill".into(),
        "selected_xcenter".into(),
        "selected_idle_underline".into(),
        "textalign".into(),
        "line_leading".into(),
        "selected_activate_mouse".into(),
        "activate_xanchor".into(),
        "hover_box_wrap_spacing".into(),
        "insensitive_bottom_padding".into(),
        "activate_extra_alt".into(),
        "selected_activate_rest_indent".into(),
        "activate_text_align".into(),
        "selected_hover_ruby_style".into(),
        "selected_insensitive_top_padding".into(),
        "insensitive_caret".into(),
        "insensitive_color".into(),
        "insensitive_yfill".into(),
        "insensitive_modal".into(),
        "insensitive_xfill".into(),
        "insensitive_align".into(),
        "hover_first_indent".into(),
        "activate_line_spacing".into(),
        "insensitive_child".into(),
        "insensitive_mouse".into(),
        "insensitive_xsize".into(),
        "insensitive_debug".into(),
        "insensitive_ysize".into(),
        "insensitive_thumb".into(),
        "selected_idle_bottom_margin".into(),
        "selected_idle_top_bar".into(),
        "selected_idle_ruby_line_leading".into(),
        "selected_activate_slow_cps_multiplier".into(),
        "selected_idle_pos".into(),
        "selected_idle_alt".into(),
        "selected_idle_xanchor".into(),
        "selected_idle_yanchor".into(),
        "bar_invert".into(),
        "selected_hover_black_color".into(),
        "selected_instance".into(),
        "group_alt".into(),
        "selected_insensitive_xpadding".into(),
        "selected_insensitive_ypadding".into(),
        "adjust_spacing".into(),
        "bottom_margin".into(),
        "selected_idle_xminimum".into(),
        "selected_idle_yminimum".into(),
        "insensitive_focus_rect".into(),
        "selected_insensitive_pos".into(),
        "strikethrough".into(),
        "hover_mouse".into(),
        "hover_left_gutter".into(),
        "selected_idle_top_margin".into(),
        "selected_prefer_emoji".into(),
        "bold".into(),
        "activate_hinting".into(),
        "hover_left_margin".into(),
        "activate_enable_hover".into(),
        "hover_prefer_emoji".into(),
        "idle_alt".into(),
        "selected_hover_first_indent".into(),
        "selected_activate_slow_abortable".into(),
        "activate_box_reverse".into(),
        "selected_hover_xpadding".into(),
        "selected_hover_ypadding".into(),
        "selected_minimum".into(),
        "selected_insensitive_prefer_emoji".into(),
        "layout".into(),
        "idle_bottom_gutter".into(),
        "area".into(),
        "idle_xmargin".into(),
        "idle_ymargin".into(),
        "box_first_spacing".into(),
        "xminimum".into(),
        "selected_line_spacing".into(),
        "hover_slow_abortable".into(),
        "idle_enable_hover".into(),
        "selected_idle_offset".into(),
        "insensitive_rest_indent".into(),
        "selected_hover_underline".into(),
        "selected_hover_slow_cps".into(),
        "selected_hover_bar_resizing".into(),
        "insensitive_first_spacing".into(),
        "insensitive_right_padding".into(),
        "activate_xoffset".into(),
        "selected_idle_right_bar".into(),
        "hover_extra_alt".into(),
        "selected_insensitive_right_gutter".into(),
        "insensitive_unscrollable".into(),
        "selected_insensitive_right_margin".into(),
        "activate_left_padding".into(),
        "hover_left_bar".into(),
        "activate_right_bar".into(),
        "selected_hover_antialias".into(),
        "selected_mouse".into(),
        "hover_minwidth".into(),
        "hover_emoji_font".into(),
        "hover_xysize".into(),
        "hover_hover_sound".into(),
        "yalign".into(),
        "insensitive_order_reverse".into(),
        "selected_idle_line_leading".into(),
        "selected_idle_line_spacing".into(),
        "selected_idle_size".into(),
        "activate_bottom_gutter".into(),
        "xfit".into(),
        "activate_group_alt".into(),
        "selected_activate_right_bar".into(),
        "selected_idle_newline_indent".into(),
        "selected_hover_alt".into(),
        "selected_activate_emoji_font".into(),
        "selected_insensitive_fit_first".into(),
        "idle_focus_rect".into(),
        "selected_insensitive_instance".into(),
        "hover_bold".into(),
        "selected_idle_drop_shadow".into(),
        "activate_box_first_spacing".into(),
        "selected_activate_size_group".into(),
        "activate_foreground".into(),
        "idle_outlines".into(),
        "selected_hover_color".into(),
        "hover_keyboard_focus".into(),
        "selected_activate_area".into(),
        "selected_clipping".into(),
        "activate_top_margin".into(),
        "idle_margin".into(),
        "activate_bar_vertical".into(),
        "selected_idle_anchor".into(),
        "selected_insensitive_first_spacing".into(),
        "hover_right_margin".into(),
        "idle_antialias".into(),
        "insensitive_line_overlap_split".into(),
        "selected_activate_margin".into(),
        "insensitive_fit_first".into(),
        "idle_focus_mask".into(),
        "selected_ruby_line_leading".into(),
        "selected_insensitive_xycenter".into(),
        "selected_right_bar".into(),
        "outlines".into(),
        "selected_insensitive_background".into(),
        "selected_activate_ruby_style".into(),
        "selected_idle_thumb".into(),
        "hover_right_bar".into(),
        "insensitive_xysize".into(),
        "activate_subtitle_width".into(),
        "selected_activate_slow_cps".into(),
        "selected_activate_enable_hover".into(),
        "selected_hover_slow_cps_multiplier".into(),
        "activate_unscrollable".into(),
        "activate_base_bar".into(),
        "activate_box_wrap".into(),
        "selected_insensitive_slow_speed".into(),
        "size_group".into(),
        "activate_key_events".into(),
        "insensitive_hover_sound".into(),
        "insensitive_emoji_font".into(),
        "selected_textalign".into(),
        "activate_box_layout".into(),
        "activate_thumb".into(),
        "box_wrap".into(),
        "selected_activate_yfit".into(),
        "selected_insensitive_text_y_fudge".into(),
        "selected_insensitive_key_events".into(),
        "selected_insensitive_left_padding".into(),
        "selected_align".into(),
        "insensitive_xmaximum".into(),
        "insensitive_xminimum".into(),
        "insensitive_ymaximum".into(),
        "insensitive_yminimum".into(),
        "bottom_gutter".into(),
        "margin".into(),
        "selected_insensitive_box_reverse".into(),
        "selected_aft_gutter".into(),
        "selected_bar_invert".into(),
        "selected_activate_top_margin".into(),
        "selected_activate_drop_shadow".into(),
        "activate_top_bar".into(),
        "selected_activate_text_align".into(),
        "maximum".into(),
        "selected_insensitive_padding".into(),
        "selected_hover_outlines".into(),
        "activate_yanchor".into(),
        "selected_activate_sound".into(),
        "insensitive_bar_resizing".into(),
        "selected_idle_xmaximum".into(),
        "selected_idle_ymaximum".into(),
        "selected_maximum".into(),
        "insensitive_activate_sound".into(),
        "selected_insensitive_align".into(),
        "selected_insensitive_adjust_spacing".into(),
        "selected_hover_size".into(),
        "insensitive_bar_invert".into(),
        "selected_hover_subpixel".into(),
        "xoffset".into(),
        "activate_xcenter".into(),
        "key_events".into(),
        "selected_idle_first_spacing".into(),
        "selected_hover_bottom_padding".into(),
        "selected_idle_bottom_gutter".into(),
        "activate_clipping".into(),
        "selected_idle_kerning".into(),
        "selected_activate_bottom_padding".into(),
        "selected_insensitive_debug".into(),
        "selected_ycenter".into(),
        "selected_hover_base_bar".into(),
        "selected_hover_bar_vertical".into(),
        "focus_rect".into(),
        "selected_activate_ruby_line_leading".into(),
        "insensitive_box_wrap".into(),
        "selected_insensitive_bold".into(),
        "selected_insensitive_size".into(),
        "selected_insensitive_area".into(),
        "hover_maximum".into(),
        "selected_insensitive_font".into(),
        "selected_insensitive_xfit".into(),
        "selected_insensitive_yfit".into(),
        "selected_margin".into(),
        "selected_insensitive_axis".into(),
        "selected_insensitive_xpos".into(),
        "selected_insensitive_ypos".into(),
        "insensitive_drop_shadow".into(),
        "caret".into(),
        "selected_hover_activate_sound".into(),
        "selected_insensitive_min_width".into(),
        "activate_xysize".into(),
        "activate_underline".into(),
        "hover_key_events".into(),
        "hinting".into(),
        "selected_idle_right_padding".into(),
        "selected_minwidth".into(),
        "selected_idle_caret".into(),
        "selected_idle_xpadding".into(),
        "selected_idle_ypadding".into(),
        "activate_black_color".into(),
        "selected_insensitive_underline".into(),
        "hover_xpos".into(),
        "hover_ypos".into(),
        "idle_time_policy".into(),
        "activate_alt".into(),
        "line_spacing".into(),
        "insensitive_size_group".into(),
        "selected_idle_right_gutter".into(),
        "hover_clipping".into(),
        "selected_activate_shaper".into(),
        "selected_hover_vertical".into(),
        "idle_xfill".into(),
        "idle_yfill".into(),
        "selected_idle_left_padding".into(),
        "idle_child".into(),
        "idle_right_padding".into(),
        "axis".into(),
        "idle_rest_indent".into(),
        "insensitive_justify".into(),
        "rest_indent".into(),
        "insensitive_bottom_bar".into(),
        "insensitive_font".into(),
        "selected_idle_outlines".into(),
        "insensitive_aft_gutter".into(),
        "insensitive_top_gutter".into(),
        "selected_hover_right_padding".into(),
        "selected_activate_first_spacing".into(),
        "right_gutter".into(),
        "selected_activate_outline_scaling".into(),
        "yminimum".into(),
        "top_bar".into(),
        "selected_idle_group_alt".into(),
        "selected_insensitive_kerning".into(),
        "size".into(),
        "selected_insensitive_line_leading".into(),
        "selected_insensitive_line_spacing".into(),
        "selected_activate_right_margin".into(),
        "selected_hover_hinting".into(),
        "selected_insensitive_offset".into(),
        "top_margin".into(),
        "selected_ruby_style".into(),
        "selected_xycenter".into(),
        "selected_hover_anchor".into(),
        "insensitive_thumb_shadow".into(),
        "activate_strikethrough".into(),
        "selected_xspacing".into(),
        "selected_xpadding".into(),
        "selected_xmaximum".into(),
        "selected_xminimum".into(),
        "selected_hover_unscrollable".into(),
        "insensitive_anchor".into(),
        "selected_right_padding".into(),
        "selected_hover_xysize".into(),
        "selected_activate_instance".into(),
        "hover_xmaximum".into(),
        "hover_ymaximum".into(),
        "selected_idle_adjust_spacing".into(),
        "activate_xfit".into(),
        "selected_hover_clipping".into(),
        "left_bar".into(),
        "insensitive_xpadding".into(),
        "insensitive_ypadding".into(),
        "selected_insensitive_activate_sound".into(),
        "selected_hover_adjust_spacing".into(),
        "selected_insensitive_xanchor".into(),
        "selected_insensitive_yanchor".into(),
        "idle_box_spacing".into(),
        "hover_outline_scaling".into(),
        "selected_xmargin".into(),
        "selected_activate_bar_resizing".into(),
        "insensitive_underline".into(),
        "selected_idle_top_gutter".into(),
        "activate_mouse".into(),
        "selected_hover_thumb".into(),
        "activate_instance".into(),
        "selected_activate_bottom_bar".into(),
        "selected_activate_textalign".into(),
        "selected_activate_top_padding".into(),
        "selected_idle_mouse".into(),
        "selected_activate_yfill".into(),
        "selected_idle_focus_mask".into(),
        "shaper".into(),
        "idle_bar_vertical".into(),
        "selected_key_events".into(),
        "selected_modal".into(),
        "selected_insensitive_strikethrough".into(),
        "idle_altruby_style".into(),
        "insensitive_xspacing".into(),
        "insensitive_yspacing".into(),
        "selected_slow_cps_multiplier".into(),
        "idle_strikethrough".into(),
        "selected_idle_box_wrap".into(),
        "insensitive_text_y_fudge".into(),
        "selected_idle_keyboard_focus".into(),
        "selected_activate_altruby_style".into(),
        "selected_activate_align".into(),
        "selected_size_group".into(),
        "insensitive_bar_vertical".into(),
        "activate_slow_abortable".into(),
        "selected_thumb_shadow".into(),
        "activate_ymargin".into(),
        "idle_ruby_style".into(),
        "clipping".into(),
        "selected_hover_extra_alt".into(),
        "idle_order_reverse".into(),
        "idle_box_reverse".into(),
        "selected_idle_fore_gutter".into(),
        "insensitive_keyboard_focus".into(),
        "hover_drop_shadow_color".into(),
        "insensitive_xycenter".into(),
        "selected_hover_drop_shadow".into(),
        "modal".into(),
        "idle_xysize".into(),
        "selected_hover_bar_invert".into(),
        "hover_focus_mask".into(),
        "selected_idle_fore_bar".into(),
        "selected_insensitive_unscrollable".into(),
        "selected_insensitive_group_alt".into(),
        "debug".into(),
        "activate_vertical".into(),
        "insensitive_xmargin".into(),
        "insensitive_ymargin".into(),
        "idle_thumb".into(),
        "anchor".into(),
        "hover_hinting".into(),
        "selected_idle_xysize".into(),
        "hover_hyperlink_functions".into(),
        "underline".into(),
        "activate_time_policy".into(),
        "ymaximum".into(),
        "hover_group_alt".into(),
        "insensitive_yfit".into(),
        "selected_hover_foreground".into(),
        "selected_idle_shaper".into(),
        "activate_activate_sound".into(),
        "selected_box_wrap".into(),
        "selected_base_bar".into(),
        "hover_thumb_offset".into(),
        "activate_size".into(),
        "hover_unscrollable".into(),
        "selected_aft_bar".into(),
        "selected_idle_justify".into(),
        "hover_instance".into(),
        "hover_box_wrap".into(),
        "selected_insensitive_extra_alt".into(),
        "activate_offset".into(),
        "insensitive_box_first_spacing".into(),
        "selected_idle_time_policy".into(),
        "selected_emoji_font".into(),
        "idle_hyperlink_functions".into(),
        "selected_shaper".into(),
        "selected_activate_font".into(),
        "selected_idle_maximum".into(),
        "selected_insensitive_outlines".into(),
        "selected_idle_background".into(),
        "selected_activate_subpixel".into(),
        "hover_text_align".into(),
        "selected_activate_outlines".into(),
        "selected_activate_xycenter".into(),
        "idle_top_margin".into(),
        "selected_idle_spacing".into(),
        "selected_hover_order_reverse".into(),
        "hover_minimum".into(),
        "idle_key_events".into(),
        "idle_box_first_spacing".into(),
        "selected_idle_slow_cps".into(),
        "xysize".into(),
        "activate_xalign".into(),
        "focus_mask".into(),
        "yspacing".into(),
        "selected_black_color".into(),
        "selected_insensitive_caret".into(),
        "selected_kerning".into(),
        "selected_idle_minimum".into(),
        "insensitive_slow_cps_multiplier".into(),
        "hover_top_padding".into(),
        "selected_idle_base_bar".into(),
        "selected_insensitive_slow_cps".into(),
        "hover_anchor".into(),
        "selected_underline".into(),
        "outline_scaling".into(),
        "insensitive_group_alt".into(),
        "activate_ruby_line_leading".into(),
        "selected_drop_shadow".into(),
        "selected_idle_text_y_fudge".into(),
        "selected_yoffset".into(),
        "selected_activate_hover_sound".into(),
        "hover_min_width".into(),
        "selected_hover_top_bar".into(),
        "selected_unscrollable".into(),
        "hover_font".into(),
        "selected_hover_top_padding".into(),
        "activate_modal".into(),
        "idle_keyboard_focus".into(),
        "hover_activate_sound".into(),
        "xpos".into(),
        "selected_activate_hyperlink_functions".into(),
        "selected_idle_xspacing".into(),
        "selected_idle_yspacing".into(),
        "hover_text_y_fudge".into(),
        "selected_padding".into(),
        "insensitive_newline_indent".into(),
        "selected_activate_ypos".into(),
        "hover_box_first_spacing".into(),
        "selected_enable_hover".into(),
        "idle_mipmap".into(),
        "selected_hover_shaper".into(),
        "selected_idle_italic".into(),
        "activate_box_spacing".into(),
        "activate_textalign".into(),
        "ysize".into(),
        "selected_activate_adjust_spacing".into(),
        "altruby_style".into(),
        "selected_hover_line_overlap_split".into(),
        "selected_activate_order_reverse".into(),
        "activate_outline_scaling".into(),
        "selected_left_bar".into(),
        "background".into(),
        "ypos".into(),
        "activate_focus_rect".into(),
        "selected_subtitle_width".into(),
        "selected_language".into(),
        "selected_hover_altruby_style".into(),
        "insensitive_ruby_line_leading".into(),
        "selected_idle_aft_bar".into(),
        "selected_activate_box_reverse".into(),
        "hover_subtitle_width".into(),
        "selected_hover_aft_bar".into(),
        "selected_hover_mipmap".into(),
        "selected_axis".into(),
        "activate_xfill".into(),
        "thumb_offset".into(),
        "selected_activate_activate_sound".into(),
        "selected_antialias".into(),
        "selected_hover_fit_first".into(),
        "activate_slow_speed".into(),
        "selected_child".into(),
        "hover_bottom_margin".into(),
        "activate_minwidth".into(),
        "selected_hover_strikethrough".into(),
        "selected_insensitive_bar_invert".into(),
        "idle_min_width".into(),
        "selected_insensitive_antialias".into(),
        "insensitive_layout".into(),
        "activate_rest_indent".into(),
        "selected_idle_slow_abortable".into(),
        "selected_bar_resizing".into(),
        "selected_hover_left_padding".into(),
        "selected_hover_ruby_line_leading".into(),
        "activate_line_overlap_split".into(),
        "hover_box_spacing".into(),
        "hover_box_reverse".into(),
        "activate_yalign".into(),
        "selected_insensitive_box_wrap".into(),
        "selected_insensitive_thumb".into(),
        "selected_activate_text_y_fudge".into(),
        "subpixel".into(),
        "selected_insensitive_ruby_style".into(),
        "selected_hover_modal".into(),
        "selected_activate_left_margin".into(),
        "thumb".into(),
        "insensitive_alt".into(),
        "selected_hover_maximum".into(),
        "insensitive_vertical".into(),
        "activate_margin".into(),
        "idle_activate_sound".into(),
        "selected_idle_xalign".into(),
        "selected_idle_yalign".into(),
        "selected_bar_vertical".into(),
        "selected_xoffset".into(),
        "activate_font".into(),
        "selected_hover_hover_sound".into(),
        "hover_right_padding".into(),
        "activate_ycenter".into(),
        "hover_top_bar".into(),
        "selected_insensitive_shaper".into(),
        "insensitive_box_layout".into(),
        "idle_offset".into(),
        "xspacing".into(),
        "yfill".into(),
        "selected_slow_speed".into(),
        "selected_idle_bold".into(),
        "idle_anchor".into(),
        "selected_idle_enable_hover".into(),
        "activate_keyboard_focus".into(),
        "selected_insensitive_time_policy".into(),
        "idle_layout".into(),
        "selected_activate_ysize".into(),
        "selected_insensitive_bottom_margin".into(),
        "selected_idle_box_reverse".into(),
        "selected_idle_box_spacing".into(),
        "insensitive_slow_abortable".into(),
        "selected_fore_bar".into(),
        "selected_idle_padding".into(),
        "selected_mipmap".into(),
        "idle_xsize".into(),
        "idle_ysize".into(),
        "selected_area".into(),
        "idle_hover_sound".into(),
        "activate_anchor".into(),
        "selected_insensitive_focus_rect".into(),
        "bar_resizing".into(),
        "selected_insensitive_subtitle_width".into(),
        "idle_box_wrap".into(),
        "left_padding".into(),
        "selected_idle_hover_sound".into(),
        "activate_fit_first".into(),
        "selected_right_gutter".into(),
        "selected_idle_box_layout".into(),
        "selected_thumb_offset".into(),
        "hover_background".into(),
        "selected_hover_enable_hover".into(),
        "selected_line_leading".into(),
        "xalign".into(),
        "idle_extra_alt".into(),
        "hover_bar_resizing".into(),
        "time_policy".into(),
        "idle_newline_indent".into(),
        "fore_gutter".into(),
        "insensitive_right_bar".into(),
        "selected_activate_xfit".into(),
        "activate_top_padding".into(),
        "selected_insensitive_keyboard_focus".into(),
        "selected_hover_xmargin".into(),
        "selected_hover_ymargin".into(),
        "selected_insensitive_xcenter".into(),
        "selected_insensitive_ycenter".into(),
        "aft_bar".into(),
        "activate_drop_shadow".into(),
        "selected_idle_prefer_emoji".into(),
        "selected_activate_clipping".into(),
        "selected_activate_line_leading".into(),
        "selected_activate_mipmap".into(),
        "selected_activate_xspacing".into(),
        "selected_activate_yspacing".into(),
        "selected_activate_xpadding".into(),
        "selected_activate_ypadding".into(),
        "activate_xsize".into(),
        "selected_idle_foreground".into(),
        "hover_pos".into(),
        "hover_alt".into(),
        "bottom_bar".into(),
        "selected_activate_focus_mask".into(),
        "selected_insensitive_xfill".into(),
        "selected_insensitive_yfill".into(),
        "selected_insensitive_child".into(),
        "insensitive_xcenter".into(),
        "insensitive_ycenter".into(),
        "selected_hover_size_group".into(),
        "selected_hover_thumb_shadow".into(),
        "activate_italic".into(),
        "selected_insensitive_clipping".into(),
        "activate_maximum".into(),
        "padding".into(),
        "insensitive_black_color".into(),
        "idle_ruby_line_leading".into(),
        "box_spacing".into(),
        "selected_insensitive_justify".into(),
        "selected_insensitive_slow_cps_multiplier".into(),
        "selected_insensitive_newline_indent".into(),
        "selected_hover_pos".into(),
        "selected_idle_min_width".into(),
        "selected_hover_bold".into(),
        "selected_activate_alt".into(),
        "selected_hover_drop_shadow_color".into(),
        "idle_slow_abortable".into(),
        "selected_idle_ruby_style".into(),
        "insensitive_slow_cps".into(),
        "idle_unscrollable".into(),
        "selected_insensitive_fore_gutter".into(),
        "selected_ysize".into(),
        "activate_line_leading".into(),
        "idle_minwidth".into(),
        "font".into(),
        "selected_activate_box_layout".into(),
        "hover_rest_indent".into(),
        "idle_size".into(),
        "idle_bold".into(),
        "idle_area".into(),
        "selected_insensitive_bottom_gutter".into(),
        "selected_idle_debug".into(),
        "idle_bottom_padding".into(),
        "idle_right_gutter".into(),
        "antialias".into(),
        "activate_mipmap".into(),
        "idle_font".into(),
        "idle_right_margin".into(),
        "idle_xfit".into(),
        "idle_yfit".into(),
        "insensitive_area".into(),
        "idle_axis".into(),
        "idle_xpos".into(),
        "idle_ypos".into(),
        "selected_activate_thumb".into(),
        "idle_first_spacing".into(),
        "selected_idle_aft_gutter".into(),
        "selected_activate_line_spacing".into(),
        "selected_insensitive_left_gutter".into(),
        "selected_size".into(),
        "selected_hover_xspacing".into(),
        "selected_hover_yspacing".into(),
        "selected_insensitive_drop_shadow_color".into(),
        "selected_box_reverse".into(),
        "top_gutter".into(),
        "selected_outlines".into(),
        "selected_activate_box_wrap_spacing".into(),
        "selected_yalign".into(),
        "idle_left_bar".into(),
        "ycenter".into(),
        "hover_spacing".into(),
        "selected_activate_xcenter".into(),
        "selected_insensitive_aft_bar".into(),
        "selected_insensitive_top_bar".into(),
        "hover_focus_rect".into(),
        "idle_instance".into(),
        "right_padding".into(),
        "selected_idle_text_align".into(),
        "activate_debug".into(),
        "selected_idle_bar_invert".into(),
        "activate_order_reverse".into(),
        "hover_color".into(),
        "activate_top_gutter".into(),
        "selected_idle_language".into(),
        "selected_insensitive_text_align".into(),
        "activate_adjust_spacing".into(),
        "idle_box_layout".into(),
        "vertical".into(),
        "selected_hover_right_gutter".into(),
        "insensitive_right_margin".into(),
        "yfit".into(),
        "idle_xspacing".into(),
        "idle_yspacing".into(),
        "text_align".into(),
        "selected_activate_focus_rect".into(),
        "selected_insensitive_xmargin".into(),
        "selected_insensitive_ymargin".into(),
        "selected_extra_alt".into(),
        "selected_hover_left_margin".into(),
        "activate_altruby_style".into(),
        "selected_hover_xfill".into(),
        "selected_hover_yfill".into(),
        "selected_idle_hyperlink_functions".into(),
        "idle_adjust_spacing".into(),
        "hover_vertical".into(),
        "selected_activate_key_events".into(),
        "selected_hover_debug".into(),
        "selected_hover_time_policy".into(),
        "idle_prefer_emoji".into(),
        "hover_layout".into(),
        "xpadding".into(),
        "hover_top_margin".into(),
        "selected_activate_subtitle_width".into(),
        "selected_hover_rest_indent".into(),
        "selected_hover_prefer_emoji".into(),
        "hover_xoffset".into(),
        "hover_yoffset".into(),
        "selected_idle_drop_shadow_color".into(),
        "idle_italic".into(),
        "selected_hover_margin".into(),
        "selected_hover_padding".into(),
        "hover_right_gutter".into(),
        "selected_activate_xalign".into(),
        "insensitive_instance".into(),
        "hover_xpadding".into(),
        "hover_ypadding".into(),
        "hover_altruby_style".into(),
        "selected_idle_axis".into(),
        "selected_idle_xfit".into(),
        "selected_idle_yfit".into(),
        "selected_activate_axis".into(),
        "drop_shadow_color".into(),
        "activate_justify".into(),
        "selected_text_y_fudge".into(),
        "mipmap".into(),
        "selected_hover_offset".into(),
        "selected_hover_spacing".into(),
        "hover_outlines".into(),
        "insensitive_size".into(),
        "idle_underline".into(),
        "hover_kerning".into(),
        "selected_yanchor".into(),
        "activate_outlines".into(),
        "selected_hover_outline_scaling".into(),
        "selected_background".into(),
        "selected_idle_activate_sound".into(),
        "selected_insensitive_right_padding".into(),
        "activate_xpos".into(),
        "hover_newline_indent".into(),
        "idle_foreground".into(),
        "selected_left_padding".into(),
        "selected_activate_background".into(),
        "selected_top_gutter".into(),
        "selected_activate_foreground".into(),
        "selected_yspacing".into(),
        "selected_ypadding".into(),
        "insensitive_clipping".into(),
        "selected_idle_thumb_shadow".into(),
        "selected_ymaximum".into(),
        "selected_yminimum".into(),
        "xanchor".into(),
        "selected_drop_shadow_color".into(),
        "activate_right_padding".into(),
        "selected_activate_newline_indent".into(),
        "idle_color".into(),
        "selected_top_bar".into(),
        "selected_insensitive_box_layout".into(),
        "selected_font".into(),
        "activate_first_spacing".into(),
        "activate_ypos".into(),
        "selected_activate_xfill".into(),
        "selected_slow_abortable".into(),
        "selected_hover_box_spacing".into(),
        "hover_xspacing".into(),
        "hover_yspacing".into(),
        "selected_activate_layout".into(),
        "hover_bottom_bar".into(),
        "idle_aft_gutter".into(),
        "idle_top_gutter".into(),
        "selected_activate_box_wrap".into(),
        "selected_hover_right_margin".into(),
        "selected_activate_fore_bar".into(),
        "selected_activate_left_bar".into(),
        "selected_activate_base_bar".into(),
        "selected_hover_subtitle_width".into(),
        "insensitive_margin".into(),
        "insensitive_ruby_style".into(),
        "selected_activate_vertical".into(),
        "insensitive_italic".into(),
        "activate_right_gutter".into(),
        "selected_activate_modal".into(),
        "hover_padding".into(),
        "selected_insensitive_modal".into(),
        "selected_insensitive_bottom_bar".into(),
        "selected_insensitive_bar_vertical".into(),
        "selected_idle_outline_scaling".into(),
        "selected_activate_thumb_shadow".into(),
        "activate_fore_gutter".into(),
        "selected_activate_size".into(),
        "selected_idle_textalign".into(),
        "selected_offset".into(),
        "selected_hover_language".into(),
        "first_indent".into(),
        "selected_hover_area".into(),
        "selected_idle_child".into(),
        "selected_insensitive_drop_shadow".into(),
        "selected_idle_instance".into(),
        "selected_idle_xfill".into(),
        "selected_idle_yfill".into(),
        "activate_thumb_shadow".into(),
        "box_wrap_spacing".into(),
        "selected_right_margin".into(),
        "selected_hover_bottom_margin".into(),
        "selected_hover_box_wrap".into(),
        "hover_box_layout".into(),
        "justify".into(),
        "insensitive_xfit".into(),
        "thumb_shadow".into(),
        "selected_activate_box_spacing".into(),
    ]);

    if lex.keyword("is".into()).is_some() {
        if rv.parent.is_some() {
            panic!("parent clause appears twice.");
        }
        rv.parent = Some(
            lex.require(LexerType::Type(LexerTypeOptions::Word))
                .unwrap(),
        );
        return true;
    }

    if lex.keyword("clear".into()).is_some() {
        rv.clear = true;
        return true;
    }

    if lex.keyword("take".into()).is_some() {
        if rv.take.is_some() {
            panic!("take clause appears twice.");
        }
        rv.take = Some(
            lex.require(LexerType::Type(LexerTypeOptions::Name))
                .unwrap(),
        );
        return true;
    }

    if lex.keyword("del".into()).is_some() {
        let propname = lex
            .require(LexerType::Type(LexerTypeOptions::Name))
            .unwrap();

        if !style_prefixed_all_properties.contains(&propname) {
            panic!("style property {} is not known.", propname);
        }

        rv.delattr.push(propname);
        return true;
    }

    if lex.keyword("variant".into()).is_some() {
        if rv.variant.is_some() {
            panic!("variant clause appears twice.");
        }
        rv.variant = Some(
            lex.require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                .unwrap(),
        );
        return true;
    }

    let propname = lex.name();

    match propname {
        Some(pname) => {
            if pname != "properties" && !style_prefixed_all_properties.contains(&pname) {
                panic!("style property {} is not known.", pname);
            }

            if rv.properties.contains_key(&pname) {
                panic!("style property {} appears twice.", pname);
            }

            rv.properties.insert(
                pname,
                lex.require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                    .unwrap(),
            );

            return true;
        }
        None => {}
    }

    false
}

impl Parser for Style {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let name = lex
            .require(LexerType::Type(LexerTypeOptions::Word))
            .unwrap();

        let mut style_node = Style {
            loc: loc.clone(),
            name,
            parent: None,
            clear: false,
            take: None,
            delattr: vec![],
            variant: None,
            properties: HashMap::new(),
        };

        while parse_clause(&mut style_node, lex) {}

        if lex.rmatch(":".into()).is_some() {
            lex.expect_block();
            lex.expect_eol();

            let mut ll = lex.subblock_lexer(false);

            while ll.advance() {
                while parse_clause(&mut style_node, &mut ll) {}

                ll.expect_eol();
            }
        } else {
            lex.expect_noblock();
            lex.expect_eol();
        }

        let mut rv = AstNode::Style(style_node);

        if !lex.init {
            rv = AstNode::Init(Init {
                loc,
                block: vec![rv],
                priority: lex.init_offset,
            });
        }

        lex.advance();

        Ok(vec![rv])
    }
}

impl Parser for Init {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let priority: isize = match lex.integer() {
            Some(p) => p.parse()?,
            None => 0,
        };

        let block;

        if lex.rmatch(":".into()).is_some() {
            lex.expect_eol();
            lex.expect_block();

            block = parse_block(&mut lex.subblock_lexer(true))?;

            lex.advance();
        } else {
            let old_init = lex.init;

            lex.init = true;

            block = parse_statement(lex)?;

            lex.init = old_init;
        }

        Ok(vec![AstNode::Init(Init {
            loc,
            block,
            priority: priority + lex.init_offset,
        })])
    }
}

impl Parser for Python {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let mut hide = false;
        let mut early = false;
        let mut store = "store".into();

        if lex.keyword("early".into()).is_some() {
            early = true;
        }

        if lex.keyword("hide".into()).is_some() {
            hide = true;
        }

        if lex.keyword("in".into()).is_some() {
            let s = lex
                .require(LexerType::Type(LexerTypeOptions::DottedName))
                .unwrap();
            store = format!("store.{s}");
        }

        lex.require(LexerType::String(":".into())).unwrap();
        lex.expect_eol();

        lex.expect_block();

        let python_code = lex.python_block().unwrap().trim().into();

        lex.advance();

        if early {
            Ok(vec![AstNode::EarlyPython(EarlyPython {
                loc,
                python_code,
                hide,
                store: store,
            })])
        } else {
            Ok(vec![AstNode::Python(Python {
                loc,
                python_code,
                hide,
                store: store,
            })])
        }
    }
}

impl Parser for Default_ {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let priority: isize = match lex.integer() {
            Some(p) => p.parse()?,
            None => 0,
        };

        let mut store = "store".into();
        let mut name = lex
            .require(LexerType::Type(LexerTypeOptions::Word))
            .unwrap();

        while lex.rmatch(r"\.".into()).is_some() {
            store = format!("{store}.{name}");
            name = lex
                .require(LexerType::Type(LexerTypeOptions::Word))
                .unwrap();
        }

        lex.require(LexerType::String("=".into())).unwrap();
        let expr = lex.rest();

        if expr.is_none() {
            panic!("expected expression");
        }

        lex.expect_noblock();

        let rv = Default_ {
            loc: loc.clone(),
            store,
            name,
            expr,
        };

        let res = if !lex.init {
            vec![AstNode::Init(Init {
                loc,
                block: vec![AstNode::Default(rv)],
                priority: priority + lex.init_offset,
            })]
        } else {
            vec![AstNode::Default(rv)]
        };

        lex.advance();

        Ok(res)
    }
}

impl Parser for Define {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        let priority: isize = match lex.integer() {
            Some(p) => p.parse()?,
            None => 0,
        };

        let mut store = "store".into();
        let mut name = lex
            .require(LexerType::Type(LexerTypeOptions::Word))
            .unwrap();

        while lex.rmatch(r"\.".into()).is_some() {
            store = format!("{store}.{name}");
            name = lex.require(LexerType::Type(LexerTypeOptions::Word))
                .unwrap();
        }

        let mut index = None;
        if lex.rmatch(r"\[".into()).is_some() {
            index = lex.delimited_python("]".into(), true);
            lex.require(LexerType::String(r"\]".into())).unwrap();
        }

        let operator;
        if lex.rmatch(r"\+=".into()).is_some() {
            operator = "+=";
        } else if lex.rmatch(r"\|=".into()).is_some() {
            operator = "|=";
        } else {
            lex.require(LexerType::String("=".into())).unwrap();
            operator = "=";
        }

        let expr = lex.rest();

        if expr.is_none() {
            panic!("expected expression");
        }

        lex.expect_noblock();

        let rv = Define {
            loc: loc.clone(),
            store,
            name,
            index,
            operator: operator.into(),
            expr: expr.unwrap(),
        };

        let res = if !lex.init {
            vec![AstNode::Init(Init {
                loc,
                block: vec![AstNode::Define(rv)],
                priority: priority + lex.init_offset,
            })]
        } else {
            vec![AstNode::Define(rv)]
        };

        lex.advance();

        Ok(res)
    }
}

impl Parser for Call {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        lex.expect_noblock();

        let mut expression = false;
        let target = if lex.keyword("expression".into()).is_some() {
            expression = true;
            lex.require(LexerType::Type(LexerTypeOptions::SimpleExpression))
                .unwrap()
        } else {
            lex.require(LexerType::Type(LexerTypeOptions::LabelName))
                .unwrap()
        };

        // optional keyword
        lex.keyword("pass".into());

        let arguments = parse_arguments(lex);

        let mut global_label = None;

        if expression && lex.global_label.is_some() {
            global_label = lex.global_label.clone();
        }

        let mut rv = vec![AstNode::Call(Call {
            loc: loc.clone(),
            label: target,
            expression,
            arguments,
            global_label,
        })];

        if lex.keyword("from".into()).is_some() {
            let name = lex
                .require(LexerType::Type(LexerTypeOptions::LabelNameDeclare))
                .unwrap();
            rv.push(AstNode::Label(Label {
                loc: loc.clone(),
                name,
                block: vec![],
                parameters: None,
                hide: false,
                statement_start: None,
            }));
        }

        // rv.push(AstNode::Pass(Pass { loc }));

        lex.expect_eol();
        lex.advance();

        Ok(rv)
    }
}

impl Parser for Pass {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<Vec<AstNode>> {
        lex.expect_noblock();
        lex.expect_eol();
        lex.advance();

        Ok(vec![AstNode::Pass(Pass { loc })])
    }
}
