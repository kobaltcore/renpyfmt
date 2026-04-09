use super::*;

impl Parser for Label {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        parse_label(lex, loc, false).map(Into::into)
    }
}

impl Parser for With {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let expr = lex.require_or_error(
            LexerType::Type(LexerTypeOptions::SimpleExpression),
            "expected simple expression",
        )?;
        lex.expect_eol()?;
        lex.expect_noblock()?;
        lex.advance();

        Ok(vec![AstNode::With(With {
            loc,
            expr,
            paired: None,
        })]
        .into())
    }
}

impl Parser for Jump {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        lex.expect_noblock()?;

        let target;
        let expression;
        if lex.keyword("expression".into()).is_some() {
            expression = true;
            target = lex.require_or_error(
                LexerType::Type(LexerTypeOptions::SimpleExpression),
                "expected simple expression",
            )?;
        } else {
            expression = false;
            target = lex.require_or_error(
                LexerType::Type(LexerTypeOptions::LabelName),
                "expected label name",
            )?;
        }

        lex.expect_eol()?;
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
        })]
        .into())
    }
}

impl Parser for Menu {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        lex.expect_block()?;
        let label = lex.label_name_declare();
        lex.set_global_label(label.clone());

        let arguments = parse_arguments(lex)?;

        lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        lex.expect_eol()?;

        let menu = parse_menu(lex, loc.clone(), arguments)?;

        lex.advance();

        let mut rv = vec![];

        if let Some(label_name) = label {
            rv.push(AstNode::Label(Label {
                loc,
                name: label_name,
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

        Ok(rv.into())
    }
}

impl Parser for If {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let mut entries = vec![];

        let condition = lex.python_expression()?;
        lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        lex.expect_eol()?;
        lex.expect_block()?;

        let block = parse_block(&mut lex.subblock_lexer(false))?;

        entries.push((Some(condition), block));

        lex.advance();

        while lex.keyword("elif".into()).is_some() {
            let condition = lex.python_expression()?;
            lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
            lex.expect_eol()?;
            lex.expect_block()?;

            let block = parse_block(&mut lex.subblock_lexer(false))?;

            entries.push((Some(condition), block));

            lex.advance();
        }

        if lex.keyword("else".into()).is_some() {
            lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
            lex.expect_eol()?;
            lex.expect_block()?;

            let block = parse_block(&mut lex.subblock_lexer(false))?;

            entries.push((None, block));

            lex.advance();
        }

        Ok(vec![AstNode::If(If { loc, entries })].into())
    }
}

impl Parser for While {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let condition = lex.python_expression()?;
        lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        lex.expect_eol()?;
        lex.expect_block()?;

        let block = parse_block(&mut lex.subblock_lexer(false))?;

        lex.advance();

        Ok(vec![AstNode::While(While {
            loc,
            condition,
            block,
        })]
        .into())
    }
}

impl Parser for CompileIf {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let mut entries = vec![];

        // Unlike upstream execution-oriented parsing, the formatter must retain
        // every compile-time branch so all source can round-trip.
        let condition = lex.python_expression()?;
        lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        lex.expect_eol()?;
        lex.expect_block()?;
        let block = parse_block(&mut lex.subblock_lexer(false))?;
        entries.push((Some(condition), block));
        lex.advance();

        while lex.keyword("ELIF".into()).is_some() {
            let condition = lex.python_expression()?;
            lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
            lex.expect_eol()?;
            lex.expect_block()?;
            let block = parse_block(&mut lex.subblock_lexer(false))?;
            entries.push((Some(condition), block));
            lex.advance();
        }

        if lex.keyword("ELSE".into()).is_some() {
            lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
            lex.expect_eol()?;
            lex.expect_block()?;
            let block = parse_block(&mut lex.subblock_lexer(false))?;
            entries.push((None, block));
            lex.advance();
        }

        Ok(vec![AstNode::CompileIf(CompileIf { loc, entries })].into())
    }
}

impl Parser for Return {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        lex.expect_noblock()?;

        let rest = lex.rest();

        lex.expect_eol()?;
        lex.advance();

        Ok(vec![AstNode::Return(Return {
            loc,
            expression: rest,
        })]
        .into())
    }
}

impl Parser for Call {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        lex.expect_noblock()?;

        let mut expression = false;
        let target = if lex.keyword("expression".into()).is_some() {
            expression = true;
            lex.require_or_error(
                LexerType::Type(LexerTypeOptions::SimpleExpression),
                "expected simple expression",
            )?
        } else {
            lex.require_or_error(
                LexerType::Type(LexerTypeOptions::LabelName),
                "expected label name",
            )?
        };

        lex.keyword("pass".into());

        let arguments = parse_arguments(lex)?;

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
            let name = lex.require_or_error(
                LexerType::Type(LexerTypeOptions::LabelNameDeclare),
                "expected label name",
            )?;
            rv.push(AstNode::Label(Label {
                loc: loc.clone(),
                name,
                block: vec![],
                parameters: None,
                hide: false,
                statement_start: None,
            }));
        }

        lex.expect_eol()?;
        lex.advance();

        Ok(rv.into())
    }
}

impl Parser for Pass {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        lex.expect_noblock()?;
        lex.expect_eol()?;
        lex.advance();

        Ok(vec![AstNode::Pass(Pass { loc })].into())
    }
}
