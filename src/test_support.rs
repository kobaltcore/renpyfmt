use crate::{
    ast::AstNode,
    comments::CommentMap,
    error::Result,
    formatter::format_ast,
    lexer::{Block, Lexer},
    parser::parse_block,
};
use std::path::PathBuf;

pub(crate) fn block(number: usize, text: &str, block: Vec<Block>) -> Block {
    Block {
        filename: PathBuf::from("test.rpy"),
        number,
        text: text.into(),
        block,
    }
}

#[derive(Clone)]
struct ScriptLine {
    number: usize,
    indent: usize,
    text: String,
}

fn parse_script_lines(lines: &[ScriptLine], index: &mut usize, indent: usize) -> Vec<Block> {
    let mut blocks = vec![];

    while *index < lines.len() {
        let line = &lines[*index];

        if line.indent < indent {
            break;
        }

        assert_eq!(
            line.indent, indent,
            "unexpected indentation on line {}",
            line.number
        );

        *index += 1;

        let child = if *index < lines.len() && lines[*index].indent > indent {
            assert_eq!(
                lines[*index].indent,
                indent + 1,
                "indentation jumps more than one level on line {}",
                lines[*index].number
            );
            parse_script_lines(lines, index, indent + 1)
        } else {
            vec![]
        };

        blocks.push(block(line.number, &line.text, child));
    }

    blocks
}

pub(crate) fn script(source: &str) -> Vec<Block> {
    let lines: Vec<_> = source
        .lines()
        .enumerate()
        .filter_map(|(index, raw)| {
            if raw.trim().is_empty() {
                return None;
            }

            let spaces = raw.chars().take_while(|c| *c == ' ').count();
            assert_eq!(spaces % 4, 0, "indentation must use 4 spaces");

            Some(ScriptLine {
                number: index + 1,
                indent: spaces / 4,
                text: raw[spaces..].to_string(),
            })
        })
        .collect();

    let mut index = 0;
    parse_script_lines(&lines, &mut index, 0)
}

pub(crate) fn parse(blocks: Vec<Block>) -> Result<Vec<AstNode>> {
    let mut lex = Lexer::new(blocks);
    parse_block(&mut lex)
}

pub(crate) fn parse_script(source: &str) -> Result<Vec<AstNode>> {
    parse(script(source))
}

pub(crate) fn format_script(source: &str) -> String {
    let ast = parse_script(source).expect("parse should succeed");
    format_ast(&ast, &CommentMap::new())
}
