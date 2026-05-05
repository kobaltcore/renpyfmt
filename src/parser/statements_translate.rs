use super::*;
use super::test_language::{parse_testcase_statement, parse_testsuite_statement};

impl Parser for Translate {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        let mut language = Some(lex.require_or_error(
            LexerType::Type(LexerTypeOptions::Name),
            "expected language name",
        )?);

        if language.as_deref() == Some("None") {
            language = None;
        }

        let identifier = lex.require_or_error(
            LexerType::Type(LexerTypeOptions::Hash),
            "expected translate identifier",
        )?;

        if identifier == "strings" {
            return parse_translate_strings(lex, loc, language).map(Into::into);
        }

        if identifier == "python" {
            let old_init = lex.init;
            lex.init = true;
            let block = Python::default().parse(lex, loc.clone());
            lex.init = old_init;
            let block = block?.into_vec();
            return Ok(vec![AstNode::TranslateEarlyBlock(TranslateEarlyBlock {
                loc,
                language,
                block,
            })]
            .into());
        }

        if identifier == "style" {
            let old_init = lex.init;
            lex.init = true;
            let block = Style::default().parse(lex, loc.clone());
            lex.init = old_init;
            let block = block?.into_vec();
            return Ok(vec![AstNode::TranslateBlock(TranslateBlock {
                loc,
                language,
                block,
            })]
            .into());
        }

        lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        lex.expect_eol()?;
        lex.expect_block()?;

        let block = parse_block(&mut lex.subblock_lexer(false))?;

        lex.advance();

        Ok(vec![
            AstNode::Translate(Translate {
                loc: loc.clone(),
                identifier,
                language,
                block,
            }),
            AstNode::EndTranslate(EndTranslate { loc }),
        ]
        .into())
    }
}

impl Parser for Testcase {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        parse_testcase_statement(lex, loc)
    }
}

impl Parser for Testsuite {
    fn parse(&self, lex: &mut Lexer, loc: (PathBuf, usize)) -> Result<ParseNodes> {
        parse_testsuite_statement(lex, loc)
    }
}
