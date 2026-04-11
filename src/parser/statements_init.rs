use super::*;

impl Parser for Style {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let name =
            lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected word")?;

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

        while parse_clause(&mut style_node, lex)? {}

        if lex.rmatch(":".into()).is_some() {
            lex.expect_block()?;
            lex.expect_eol()?;

            let mut ll = lex.subblock_lexer(false);

            while ll.advance() {
                while parse_clause(&mut style_node, &mut ll)? {}
                ll.expect_eol()?;
            }
        } else {
            lex.expect_noblock()?;
            lex.expect_eol()?;
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

        Ok(vec![rv].into())
    }
}

impl Parser for Init {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        if lex.keyword("offset".into()).is_some() {
            lex.require_or_error(LexerType::String("=".into()), "expected '='")?;
            let offset = lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Integer),
                "expected integer",
            )?;

            lex.expect_eol()?;
            lex.expect_noblock()?;
            lex.advance();

            lex.init_offset = offset
                .parse()
                .map_err(|_| lex.parse_error("expected integer"))?;

            return Ok(ParseNodes::None);
        }

        if lex.keyword("label".into()).is_some() {
            return parse_label(lex, loc, true).map(Into::into);
        }

        let priority: isize = match lex.integer() {
            Some(p) => p.parse().map_err(|_| lex.parse_error("expected integer"))?,
            None => 0,
        };

        let block;

        if lex.rmatch(":".into()).is_some() {
            lex.expect_eol()?;
            lex.expect_block()?;
            block = parse_block(&mut lex.subblock_lexer(true))?;
            lex.advance();
        } else {
            let old_init = lex.init;
            lex.init = true;
            block = parse_statement(lex)?.into_vec();
            lex.init = old_init;
        }

        Ok(vec![AstNode::Init(Init {
            loc,
            block,
            priority: priority + lex.init_offset,
        })]
        .into())
    }
}

impl Parser for Python {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
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
            let s = lex.require_or_error(
                LexerType::Type(LexerTypeOptions::DottedName),
                "expected dotted name",
            )?;
            store = format!("store.{s}");
        }

        lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        lex.expect_eol()?;
        lex.expect_block()?;

        let python_code = lex
            .python_block()
            .ok_or_else(|| lex.parse_error("expected python block"))?
            .into();

        lex.advance();

        if early {
            Ok(vec![AstNode::EarlyPython(EarlyPython {
                loc,
                python_code,
                hide,
                store,
            })]
            .into())
        } else {
            Ok(vec![AstNode::Python(Python {
                loc,
                python_code,
                hide,
                store,
            })]
            .into())
        }
    }
}

impl Parser for Default_ {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let priority: isize = match lex.integer() {
            Some(p) => p.parse().map_err(|_| lex.parse_error("expected integer"))?,
            None => 0,
        };

        let mut store = "store".into();
        let mut name =
            lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected word")?;

        while lex.rmatch(r"\.".into()).is_some() {
            store = format!("{store}.{name}");
            name =
                lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected word")?;
        }

        lex.require_or_error(LexerType::String("=".into()), "expected '='")?;
        let expr = lex.rest();
        if expr.is_none() {
            return Err(lex.parse_error("expected expression"));
        }

        lex.expect_noblock()?;

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
        Ok(res.into())
    }
}

impl Parser for Define {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let priority: isize = match lex.integer() {
            Some(p) => p.parse().map_err(|_| lex.parse_error("expected integer"))?,
            None => 0,
        };

        let mut store = "store".into();
        let mut name =
            lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected word")?;

        while lex.rmatch(r"\.".into()).is_some() {
            store = format!("{store}.{name}");
            name =
                lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected word")?;
        }

        let mut index = None;
        if lex.rmatch(r"\[".into()).is_some() {
            index = lex.delimited_python("]".into(), true)?;
            lex.require_or_error(LexerType::String(r"\]".into()), "expected ']'")?;
        }

        let operator;
        if lex.rmatch(r"\+=".into()).is_some() {
            operator = "+=";
        } else if lex.rmatch(r"\|=".into()).is_some() {
            operator = "|=";
        } else {
            lex.require_or_error(LexerType::String("=".into()), "expected '='")?;
            operator = "=";
        }

        let expr = lex.rest();
        if expr.is_none() {
            return Err(lex.parse_error("expected expression"));
        }

        lex.expect_noblock()?;

        let rv = Define {
            loc: loc.clone(),
            store,
            name,
            index,
            operator: operator.into(),
            expr: expr.ok_or_else(|| lex.parse_error("expected expression"))?,
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
        Ok(res.into())
    }
}

impl Parser for Transform {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let priority: isize = match lex.integer() {
            Some(p) => p.parse().map_err(|_| lex.parse_error("expected integer"))?,
            None => 0,
        };

        let mut store = "store".into();
        let mut name =
            lex.require_or_error(LexerType::Type(LexerTypeOptions::Name), "expected name")?;

        while lex.rmatch(r"\.".into()).is_some() {
            store = format!("{store}.{name}");
            name =
                lex.require_or_error(LexerType::Type(LexerTypeOptions::Word), "expected word")?;
        }

        let parameters = parse_parameters(lex)?;

        if let Some(params) = parameters.clone() {
            let mut found_pos_only = false;
            for p in params.parameters.values() {
                match p.kind {
                    ParameterKind::PositionalOnly => {
                        if !found_pos_only {
                            found_pos_only = true;
                        }
                    }
                    ParameterKind::VarPositional => {
                        return Err(lex.parse_error(format!(
                            "the transform statement does not take *args ({p:?} is not allowed)"
                        )));
                    }
                    ParameterKind::VarKeyword => {
                        return Err(lex.parse_error(format!(
                            "the transform statement does not take **kwargs ({p:?} is not allowed)"
                        )));
                    }
                    ParameterKind::KeywordOnly => {
                        return Err(lex.parse_error(format!(
                            "the transform statement does not take required keyword-only parameters ({p:?} is not allowed)"
                        )));
                    }
                    _ => {}
                }
            }
        }

        lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        lex.expect_eol()?;
        lex.expect_block()?;

        let atl = Some(parse_atl(&mut lex.subblock_lexer(false))?);

        let mut rv = AstNode::Transform(Transform {
            loc: loc.clone(),
            store,
            name,
            atl,
            parameters,
        });

        if !lex.init {
            rv = AstNode::Init(Init {
                loc,
                block: vec![rv],
                priority: priority + lex.init_offset,
            });
        }

        lex.advance();
        Ok(vec![rv].into())
    }
}

impl Parser for RPY {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        if lex.keyword("monologue".into()).is_some() {
            if lex.keyword("double".into()).is_some() {
                lex.monologue_delimiter = Some("\n\n".into());
            } else if lex.keyword("single".into()).is_some() {
                lex.monologue_delimiter = Some("\n".into());
            } else if lex.keyword("none".into()).is_some() {
                lex.monologue_delimiter = Some("".into());
            } else {
                return Err(lex.parse_error("rpy monologue expects either none, single or double."));
            }

            lex.expect_eol()?;
            lex.expect_noblock()?;
            lex.advance();

            return Ok(ParseNodes::None);
        }

        if lex.keyword("python".into()).is_some() {
            let mut rv = vec![];

            loop {
                let name = match lex.rmatch("3".into()) {
                    Some(name) => name,
                    None => lex.require_or_error(
                        LexerType::Type(LexerTypeOptions::Word),
                        "expected __future__ name",
                    )?,
                };

                rv.push(AstNode::RPY(RPY {
                    loc: loc.clone(),
                    rest: vec!["python".into(), name],
                }));

                if lex.rmatch(",".into()).is_none() {
                    break;
                }
            }

            lex.expect_eol()?;
            lex.expect_noblock()?;
            lex.advance();

            return Ok(rv.into());
        }

        Err(lex.parse_error("expected rpy statement"))
    }
}
