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
