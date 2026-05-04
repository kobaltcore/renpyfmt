use super::*;

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

        let mut code_block = None;

        let block = UserStatementBlock::False;

        match block {
            UserStatementBlock::True => lex.expect_block()?,
            UserStatementBlock::False => lex.expect_noblock()?,
            UserStatementBlock::Script => {
                lex.expect_block()?;
                code_block = Some(parse_block(&mut lex.subblock_lexer(false))?);
            }
        };

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
        if lex.rmatch(RegexType::Simple(":".into())).is_some() {
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
                            let nested_block = parse_layered_image_attribute_line(
                                &mut group_sub,
                                &mut attribute,
                            )?;
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
                            sub.delimited_python(":".into(), false)?
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
                        let got_block = parse_layered_image_attribute_line(&mut branch_sub, &mut holder)?;
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

                layered_image.children.push(LayeredImageChild::ConditionGroup(
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

                layered_image.children.push(LayeredImageChild::Always(always));
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
