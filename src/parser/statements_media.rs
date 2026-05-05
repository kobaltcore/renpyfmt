use super::*;
use crate::ast::{
    AudioOperation, AudioTarget, ScreenName, ScreenStatementKind, WindowAutoKind, WindowKind,
};

pub(super) struct AudioStatementParser {
    pub(super) target: AudioTarget,
    pub(super) mode: PlayLikeMode,
}

pub(super) struct StopAudioStatementParser {
    pub(super) target: AudioTarget,
}

pub(super) struct PauseStatementParser;
pub(super) struct ScreenStatementParser {
    pub(super) kind: ScreenStatementKind,
}
pub(super) struct HideScreenStatementParser;
pub(super) struct WindowStatementParser {
    pub(super) kind: WindowKind,
}
pub(super) struct WindowAutoStatementParser;

impl Parser for Scene {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let mut layer = None;

        if lex.keyword("onlayer".into()).is_some() {
            layer = lex.require(LexerType::Type(LexerTypeOptions::Name))?;
            lex.expect_eol()?;
        }

        if layer.is_some() || lex.eol() {
            lex.advance();
            return Ok(vec![AstNode::Scene(Scene {
                loc,
                imspec: None,
                layer,
                atl: None,
            })]
            .into());
        }

        let imspec = parse_image_specifier(lex)?;
        let mut stmt = Scene {
            loc,
            imspec: Some(imspec.clone()),
            layer: imspec.layer,
            atl: None,
        };
        let rv = parse_with(lex, AstNode::Scene(stmt.clone()))?;

        if lex.rmatch(":".into()).is_some() {
            lex.expect_block()?;
            stmt.atl = Some(parse_atl(&mut lex.subblock_lexer(false))?);
        } else {
            lex.expect_noblock()?;
        }

        lex.expect_eol()?;
        lex.advance();

        Ok(parse_with_nodes_replace_primary(rv, AstNode::Scene(stmt)).into())
    }
}

impl Parser for Say {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let state = lex.checkpoint();

        let what = match lex.triple_string() {
            Some(s) => s,
            None => match lex.string() {
                Some(s) => vec![s],
                None => vec![],
            },
        };

        let rv = finish_say(lex, loc.clone(), None, what, None, None, true)?;

        if let Some(rv) = rv {
            lex.expect_noblock()?;
            lex.advance();
            return Ok(rv.into());
        }

        lex.revert(state);

        let who = lex.say_expression()?;
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

        if who.is_some() && !what.is_empty() {
            let rv = finish_say(
                lex,
                loc,
                Some(who.expect("who checked above").trim().to_string()),
                what,
                attributes,
                temporary_attributes,
                true,
            )?
            .ok_or_else(|| lex.parse_error("expected say statement"))?;

            lex.expect_eol()?;
            lex.expect_noblock()?;
            lex.advance();

            return Ok(rv.into());
        }

        Err(lex.parse_error("expected statement."))
    }
}

impl Parser for UserStatement {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let _old_subparses = lex.subparses.clone();

        lex.subparses = vec![];

        let text = lex.text.clone();
        let subblock = lex.subblock.clone();

        let code_block = None;

        lex.expect_noblock()?;

        let start_line = lex.line;

        if lex.line == start_line {
            lex.advance();
        }

        let rv = UserStatement {
            loc,
            line: text,
            block: subblock,
            parsed: true,
            code_block,
        };

        Ok(vec![AstNode::UserStatement(rv)].into())
    }
}

impl Parser for AudioStatementParser {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        parse_play_like(lex, loc, self.target.clone(), self.mode)
    }
}

impl Parser for StopAudioStatementParser {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        parse_stop_like(lex, loc, self.target.clone())
    }
}

impl Parser for PauseStatementParser {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        parse_pause_statement(lex, loc)
    }
}

impl Parser for ScreenStatementParser {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        parse_show_call_screen_statement(lex, loc, self.kind.clone())
    }
}

impl Parser for HideScreenStatementParser {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        parse_hide_screen_statement(lex, loc)
    }
}

impl Parser for WindowStatementParser {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        parse_window_show_hide_statement(lex, loc, self.kind.clone())
    }
}

impl Parser for WindowAutoStatementParser {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        parse_window_auto_statement(lex, loc)
    }
}

#[derive(Clone, Copy)]
pub(super) enum PlayLikeMode {
    Play,
    Queue,
}

fn require_simple_expression(lex: &mut Lexer, message: &str) -> Result<String> {
    lex.simple_expression(false, true)?
        .ok_or_else(|| lex.parse_error(message))
}

fn parse_optional_simple_expression(lex: &mut Lexer) -> Result<Option<String>> {
    lex.simple_expression(false, true)
}

fn parse_play_like(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
    target: AudioTarget,
    mode: PlayLikeMode,
) -> Result<ParseNodes> {
    let target = match target {
        AudioTarget::Generic(_) => AudioTarget::Generic(lex.name().ok_or_else(|| {
            lex.parse_error(match mode {
                PlayLikeMode::Play => "play requires a channel",
                PlayLikeMode::Queue => "queue requires a channel",
            })
        })?),
        target => target,
    };

    let file = require_simple_expression(
        lex,
        match mode {
            PlayLikeMode::Play => "play requires a file",
            PlayLikeMode::Queue => "queue requires a file",
        },
    )?;

    let mut channel = None;
    let mut fadeout = None;
    let mut fadein = None;
    let mut volume = None;
    let mut loop_mode = None;
    let mut if_changed = false;

    while !lex.eol() {
        if lex.keyword("channel".into()).is_some() {
            channel = Some(require_simple_expression(
                lex,
                "expected simple expression",
            )?);
            continue;
        }

        if lex.keyword("loop".into()).is_some() {
            loop_mode = Some(true);
            continue;
        }

        if lex.keyword("noloop".into()).is_some() {
            loop_mode = Some(false);
            continue;
        }

        if matches!(mode, PlayLikeMode::Play) && lex.keyword("if_changed".into()).is_some() {
            if_changed = true;
            continue;
        }

        if lex.keyword("fadeout".into()).is_some() {
            fadeout = Some(require_simple_expression(
                lex,
                "expected simple expression",
            )?);
            continue;
        }

        if lex.keyword("fadein".into()).is_some() {
            fadein = Some(require_simple_expression(
                lex,
                "expected simple expression",
            )?);
            continue;
        }

        if lex.keyword("volume".into()).is_some() {
            volume = Some(require_simple_expression(
                lex,
                "expected simple expression",
            )?);
            continue;
        }

        return Err(lex.parse_error("end of line expected"));
    }

    lex.expect_eol()?;
    lex.expect_noblock()?;
    lex.advance();

    Ok(vec![AstNode::AudioStatement(AudioStatement {
        loc,
        operation: match mode {
            PlayLikeMode::Play => AudioOperation::Play,
            PlayLikeMode::Queue => AudioOperation::Queue,
        },
        target,
        file: Some(file),
        channel,
        fadeout,
        fadein,
        volume,
        loop_mode,
        if_changed,
    })]
    .into())
}

fn parse_stop_like(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
    target: AudioTarget,
) -> Result<ParseNodes> {
    let target = match target {
        AudioTarget::Generic(_) => AudioTarget::Generic(
            lex.name()
                .ok_or_else(|| lex.parse_error("stop requires a channel"))?,
        ),
        target => target,
    };

    let mut channel = None;
    let mut fadeout = None;

    while !lex.eol() {
        if lex.keyword("fadeout".into()).is_some() {
            fadeout = Some(require_simple_expression(
                lex,
                "expected simple expression",
            )?);
            continue;
        }

        if lex.keyword("channel".into()).is_some() {
            channel = Some(require_simple_expression(
                lex,
                "expected simple expression",
            )?);
            continue;
        }

        return Err(lex.parse_error("end of line expected"));
    }

    lex.expect_eol()?;
    lex.expect_noblock()?;
    lex.advance();

    Ok(vec![AstNode::AudioStatement(AudioStatement {
        loc,
        operation: AudioOperation::Stop,
        target,
        file: None,
        channel,
        fadeout,
        fadein: None,
        volume: None,
        loop_mode: None,
        if_changed: false,
    })]
    .into())
}

fn parse_pause_statement(lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
    let delay = parse_optional_simple_expression(lex)?;
    lex.expect_eol()?;
    lex.expect_noblock()?;
    lex.advance();
    Ok(vec![AstNode::PauseStatement(PauseStatement { loc, delay })].into())
}

fn parse_screen_name(lex: &mut Lexer) -> Result<ScreenName> {
    if lex.keyword("expression".into()).is_some() {
        return Ok(ScreenName {
            value: require_simple_expression(lex, "expected screen expression")?,
            expression: true,
        });
    }

    Ok(ScreenName {
        value: lex.require_or_error(
            LexerType::Type(LexerTypeOptions::Word),
            "expected screen name",
        )?,
        expression: false,
    })
}

fn parse_show_call_screen_statement(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
    kind: ScreenStatementKind,
) -> Result<ParseNodes> {
    let screen = parse_screen_name(lex)?;
    if screen.expression {
        lex.keyword("pass".into());
    }
    let arguments = parse_arguments(lex)?;
    let mut predict = true;
    let mut with = None;
    let mut layer = None;
    let mut zorder = None;
    let mut tag = None;

    while !lex.eol() {
        if lex.keyword("nopredict".into()).is_some() {
            predict = false;
            continue;
        }
        if lex.keyword("with".into()).is_some() {
            with = Some(require_simple_expression(
                lex,
                "expected simple expression",
            )?);
            continue;
        }
        if lex.keyword("onlayer".into()).is_some() {
            layer = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Name),
                "expected layer name",
            )?);
            continue;
        }
        if lex.keyword("zorder".into()).is_some() {
            zorder = Some(require_simple_expression(
                lex,
                "expected simple expression",
            )?);
            continue;
        }
        if lex.keyword("as".into()).is_some() {
            tag = Some(
                lex.require_or_error(LexerType::Type(LexerTypeOptions::Name), "expected tag name")?,
            );
            continue;
        }

        return Err(lex.parse_error("end of line expected"));
    }

    lex.expect_eol()?;
    lex.expect_noblock()?;
    lex.advance();

    Ok(vec![AstNode::ScreenStatement(ScreenStatement {
        loc,
        kind,
        screen,
        arguments,
        predict,
        with,
        layer,
        zorder,
        tag,
    })]
    .into())
}

fn parse_hide_screen_statement(lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
    let screen = parse_screen_name(lex)?;
    let mut with = None;
    let mut layer = None;

    while !lex.eol() {
        if lex.keyword("with".into()).is_some() {
            with = Some(require_simple_expression(
                lex,
                "expected simple expression",
            )?);
            continue;
        }
        if lex.keyword("onlayer".into()).is_some() {
            layer = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Name),
                "expected layer name",
            )?);
            continue;
        }

        return Err(lex.parse_error("end of line expected"));
    }

    lex.expect_eol()?;
    lex.expect_noblock()?;
    lex.advance();

    Ok(vec![AstNode::ScreenStatement(ScreenStatement {
        loc,
        kind: ScreenStatementKind::Hide,
        screen,
        arguments: None,
        predict: true,
        with,
        layer,
        zorder: None,
        tag: None,
    })]
    .into())
}

fn parse_window_show_hide_statement(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
    kind: WindowKind,
) -> Result<ParseNodes> {
    let transition = parse_optional_simple_expression(lex)?;
    lex.expect_eol()?;
    lex.expect_noblock()?;
    lex.advance();
    Ok(vec![AstNode::WindowStatement(WindowStatement {
        loc,
        kind,
        transition,
    })]
    .into())
}

fn parse_window_auto_statement(lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
    let kind = if lex.keyword("hide".into()).is_some() {
        WindowAutoKind::Hide(parse_optional_simple_expression(lex)?)
    } else if lex.keyword("show".into()).is_some() {
        WindowAutoKind::Show(parse_optional_simple_expression(lex)?)
    } else {
        WindowAutoKind::Auto(parse_optional_simple_expression(lex)?)
    };

    lex.expect_eol()?;
    lex.expect_noblock()?;
    lex.advance();
    Ok(vec![AstNode::WindowAutoStatement(WindowAutoStatement {
        loc,
        kind,
    })]
    .into())
}

impl Parser for Show {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        if lex.keyword("layer".into()).is_some() {
            let layer = lex.require_or_error(
                LexerType::Type(LexerTypeOptions::ImageNameComponent),
                "expected image name component",
            )?;

            let at_list = if lex.keyword("at".into()).is_some() {
                parse_simple_expression_list(lex)?
            } else {
                vec![]
            };

            let atl = if lex.rmatch(":".into()).is_some() {
                lex.expect_block()?;
                Some(parse_atl(&mut lex.subblock_lexer(false))?)
            } else {
                lex.expect_noblock()?;
                None
            };

            lex.expect_eol()?;
            lex.advance();

            return Ok(vec![AstNode::ShowLayer(ShowLayer {
                loc,
                layer,
                at_list,
                atl,
            })]
            .into());
        }

        let imspec = parse_image_specifier(lex)?;
        let mut stmt = Show {
            loc,
            imspec: Some(imspec.clone()),
            atl: None,
        };
        let rv = parse_with(lex, AstNode::Show(stmt.clone()))?;

        if lex.rmatch(":".into()).is_some() {
            lex.expect_block()?;
            stmt.atl = Some(parse_atl(&mut lex.subblock_lexer(false))?);
        } else {
            lex.expect_noblock()?;
        }

        lex.expect_eol()?;
        lex.advance();

        Ok(parse_with_nodes_replace_primary(rv, AstNode::Show(stmt)).into())
    }
}

impl Parser for Hide {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let imspec = parse_image_specifier(lex)?;
        let rv = parse_with(
            lex,
            AstNode::Hide(Hide {
                loc,
                imgspec: imspec.clone(),
            }),
        )?;

        lex.expect_eol()?;
        lex.expect_noblock()?;
        lex.advance();

        Ok(rv.into())
    }
}

impl Parser for PythonOneLine {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let python_code = lex.rest_statement();

        if python_code.is_none() {
            return Err(lex.parse_error("expected python code"));
        }

        lex.expect_noblock()?;
        lex.advance();

        Ok(vec![AstNode::PythonOneLine(PythonOneLine {
            loc,
            python_code: python_code
                .expect("python code checked above")
                .trim()
                .into(),
        })]
        .into())
    }
}

impl Parser for Camera {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let layer = lex.image_name_component().unwrap_or("master".into());

        let at_list = if lex.keyword("at".into()).is_some() {
            parse_simple_expression_list(lex)?
        } else {
            vec![]
        };

        let atl = if lex.rmatch(":".into()).is_some() {
            lex.expect_block()?;
            Some(parse_atl(&mut lex.subblock_lexer(false))?)
        } else {
            lex.expect_noblock()?;
            None
        };

        lex.expect_eol()?;
        lex.advance();

        Ok(vec![AstNode::Camera(Camera {
            loc,
            layer,
            at_list,
            atl,
        })]
        .into())
    }
}

impl Parser for Screen {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let screen = super::screen_language::parse_screen(lex, loc.clone())?;
        Ok(AstNode::Screen(Screen { loc, screen }).into())
    }
}

impl Parser for Image {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let name = parse_image_name(lex, false, false)?
            .ok_or_else(|| lex.parse_error("expected image name"))?;

        let mut atl = None;
        let mut expr = None;
        if lex.rmatch(RegexType::Simple(":")).is_some() {
            lex.expect_eol()?;
            lex.expect_block()?;
            atl = Some(parse_atl(&mut lex.subblock_lexer(false))?);
        } else {
            lex.require_or_error(LexerType::String("=".into()), "expected '='")?;

            expr = lex.rest();

            if expr.is_none() {
                return Err(lex.parse_error("expected expression"));
            }

            lex.expect_noblock()?;
        }

        let mut rv = AstNode::Image(Image {
            loc: loc.clone(),
            name,
            expr,
            atl,
        });

        if !lex.init {
            rv = AstNode::Init(Init {
                loc,
                block: vec![rv],
                priority: 500 + lex.init_offset,
            });
        }

        lex.advance();

        Ok(vec![rv].into())
    }
}

impl Parser for LayeredImage {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let name = parse_image_name(lex, false, false)?
            .ok_or_else(|| lex.parse_error("expected image name"))?;

        lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        lex.expect_eol()?;
        lex.expect_block()?;

        let mut layered_image = LayeredImage {
            loc: loc.clone(),
            name,
            properties: vec![],
            children: vec![],
        };

        let mut sub = lex.subblock_lexer(false);
        while sub.advance() {
            if sub.keyword("attribute".into()).is_some() {
                let name = sub.require_or_error(
                    LexerType::Type(LexerTypeOptions::ImageNameComponent),
                    "expected attribute name",
                )?;
                let mut attribute = LayeredImageAttribute {
                    name,
                    ..Default::default()
                };

                let got_block = parse_layered_image_attribute_line(&mut sub, &mut attribute)?;
                if got_block {
                    validate_layered_image_attribute(&sub, &attribute)?;
                    layered_image
                        .children
                        .push(LayeredImageChild::Attribute(attribute));
                    continue;
                }

                if sub.rmatch(":".into()).is_some() {
                    sub.expect_eol()?;
                    sub.expect_block()?;
                    parse_layered_image_attribute_body(&mut sub, &mut attribute, "attribute")?;
                } else {
                    sub.expect_eol()?;
                    sub.expect_noblock()?;
                }

                validate_layered_image_attribute(&sub, &attribute)?;

                layered_image
                    .children
                    .push(LayeredImageChild::Attribute(attribute));
                continue;
            }

            if sub.keyword("group".into()).is_some() {
                let name = if sub.keyword("multiple".into()).is_some() {
                    None
                } else {
                    Some(sub.require_or_error(
                        LexerType::Type(LexerTypeOptions::ImageNameComponent),
                        "expected group name",
                    )?)
                };

                let mut group = LayeredImageGroup {
                    name: name.clone(),
                    ..Default::default()
                };

                let mut got_block = false;
                while parse_layered_image_property(&mut sub, &mut group.properties, true)? {
                    got_block = layered_image_has_property(&group.properties, "at", |value| {
                        matches!(value, LayeredImagePropertyValue::AtlTransform(_))
                    });
                    if got_block {
                        break;
                    }
                }

                if !got_block && sub.rmatch(":".into()).is_some() {
                    sub.expect_eol()?;
                    sub.expect_block()?;
                    let mut group_sub = sub.subblock_lexer(false);
                    while group_sub.advance() {
                        if group_sub.keyword("pass".into()).is_some() {
                            group_sub.expect_eol()?;
                            group_sub.expect_noblock()?;
                            continue;
                        }

                        if group_sub.keyword("attribute".into()).is_some() {
                            let attribute_name = group_sub.require_or_error(
                                LexerType::Type(LexerTypeOptions::ImageNameComponent),
                                "expected attribute name",
                            )?;
                            let mut attribute = LayeredImageAttribute {
                                name: attribute_name,
                                ..Default::default()
                            };
                            let nested_block =
                                parse_layered_image_attribute_line(&mut group_sub, &mut attribute)?;
                            if nested_block {
                                validate_layered_image_attribute(&group_sub, &attribute)?;
                                group.attributes.push(attribute);
                                continue;
                            }

                            if group_sub.rmatch(":".into()).is_some() {
                                group_sub.expect_eol()?;
                                group_sub.expect_block()?;
                                parse_layered_image_attribute_body(
                                    &mut group_sub,
                                    &mut attribute,
                                    "attribute",
                                )?;
                            } else {
                                group_sub.expect_eol()?;
                                group_sub.expect_noblock()?;
                            }

                            validate_layered_image_attribute(&group_sub, &attribute)?;
                            group.attributes.push(attribute);
                            continue;
                        }

                        let nested_block = parse_layered_image_property(
                            &mut group_sub,
                            &mut group.properties,
                            false,
                        )?;
                        if nested_block {
                            continue;
                        }

                        group_sub.expect_eol()?;
                        group_sub.expect_noblock()?;
                    }
                } else {
                    sub.expect_eol()?;
                    sub.expect_noblock()?;
                }

                if name.is_none() {
                    group.properties.push(LayeredImageProperty {
                        name: "multiple".into(),
                        value: LayeredImagePropertyValue::Flag,
                    });
                }

                layered_image.children.push(LayeredImageChild::Group(group));
                continue;
            }

            if sub.keyword("if".into()).is_some() {
                let mut branches = vec![];

                loop {
                    let branch = if branches.is_empty() {
                        "if".to_string()
                    } else if sub.keyword("elif".into()).is_some() {
                        "elif".to_string()
                    } else if sub.keyword("else".into()).is_some() {
                        "else".to_string()
                    } else {
                        break;
                    };

                    let condition = if branch == "else" {
                        None
                    } else {
                        Some(
                            sub.delimited_python(":", false)?
                                .ok_or_else(|| sub.parse_error("expected condition"))?
                                .trim()
                                .to_string(),
                        )
                    };

                    sub.require_or_error(LexerType::String(":".into()), "expected ':'")?;
                    sub.expect_block()?;
                    sub.expect_eol()?;

                    let mut branch_node = LayeredImageCondition {
                        branch,
                        condition,
                        ..Default::default()
                    };
                    let mut branch_sub = sub.subblock_lexer(false);
                    while branch_sub.advance() {
                        let mut holder = LayeredImageAttribute {
                            name: String::new(),
                            properties: std::mem::take(&mut branch_node.properties),
                            displayable: branch_node.displayable.take(),
                        };
                        let got_block =
                            parse_layered_image_attribute_line(&mut branch_sub, &mut holder)?;
                        branch_node.properties = holder.properties;
                        branch_node.displayable = holder.displayable;
                        if !got_block {
                            branch_sub.expect_eol()?;
                            branch_sub.expect_noblock()?;
                        }
                    }

                    if branch_node.displayable.is_none() {
                        return Err(sub.parse_error(
                            "An if, elif or else statement must have a displayable.",
                        ));
                    }

                    branches.push(branch_node);
                    if !sub.advance() {
                        break;
                    }
                }

                if !sub.eob {
                    sub.unadvance();
                }

                layered_image
                    .children
                    .push(LayeredImageChild::ConditionGroup(
                        LayeredImageConditionGroup { branches },
                    ));
                continue;
            }

            if sub.keyword("always".into()).is_some() {
                let mut always = LayeredImageAlways::default();
                let mut holder = LayeredImageAttribute {
                    name: String::new(),
                    properties: vec![],
                    displayable: None,
                };
                let got_block = parse_layered_image_attribute_line(&mut sub, &mut holder)?;
                always.properties = holder.properties;
                always.displayable = holder.displayable;

                if !got_block && sub.rmatch(":".into()).is_some() {
                    sub.expect_eol()?;
                    sub.expect_block()?;
                    let mut nested = LayeredImageAttribute {
                        name: String::new(),
                        properties: always.properties,
                        displayable: always.displayable,
                    };
                    parse_layered_image_attribute_body(&mut sub, &mut nested, "always")?;
                    always.properties = nested.properties;
                    always.displayable = nested.displayable;
                } else if !got_block {
                    sub.expect_eol()?;
                    sub.expect_noblock()?;
                }

                if always.displayable.is_none() {
                    return Err(sub.parse_error("The always statement must have a displayable."));
                }

                layered_image
                    .children
                    .push(LayeredImageChild::Always(always));
                continue;
            }

            if sub.keyword("pass".into()).is_some() {
                sub.expect_eol()?;
                sub.expect_noblock()?;
                layered_image.children.push(LayeredImageChild::Pass);
                continue;
            }

            while parse_layered_image_property(&mut sub, &mut layered_image.properties, true)? {}
            sub.expect_eol()?;
            sub.expect_noblock()?;
        }

        lex.advance();

        let mut rv = AstNode::LayeredImage(layered_image);
        if !lex.init {
            rv = AstNode::Init(Init {
                loc,
                block: vec![rv],
                priority: lex.init_offset,
            });
        }

        Ok(vec![rv].into())
    }
}
