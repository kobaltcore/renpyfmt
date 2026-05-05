use crate::{
    ast::Say,
    error::Result,
    lexer::Lexer,
    parser::{ParseNodes, Parser},
};
use std::collections::HashMap;

pub struct ParseTrie {
    default: Option<Box<dyn Parser + Send + Sync>>,
    words: HashMap<&'static str, ParseTrie>,
}

impl ParseTrie {
    pub fn new() -> ParseTrie {
        let parser = ParseTrie {
            default: None,
            words: HashMap::new(),
        };

        parser
    }

    pub fn add(&mut self, name: &[&'static str], parser: Box<dyn Parser + Send + Sync>) {
        if let Some((first, rest)) = name.split_first() {
            self.words
                .entry(first)
                .or_insert_with(ParseTrie::new)
                .add(rest, parser);
        } else {
            self.default = Some(parser);
        }
    }

    pub fn parse(&self, lex: &mut Lexer) -> Result<ParseNodes> {
        // println!("parse trie call");
        let loc = lex.get_location();
        let old_pos = lex.pos;

        let word = match match lex.word() {
            Some(word) => Some(word),
            None => lex.rmatch_literal("$"),
        } {
            Some(word) => Some(word),
            None => Some("".into()),
        };

        let Some(word) = word else {
            lex.pos = old_pos;
            return match self.default.as_ref() {
                Some(parse_cmd) => parse_cmd.parse(lex, loc),
                None => Say::default().parse(lex, loc),
            };
        };

        if let Some(trie) = self.words.get(word.as_str()) {
            return trie.parse(lex);
        }

        lex.pos = old_pos;
        match self.default.as_ref() {
            Some(parse_cmd) => parse_cmd.parse(lex, loc),
            None => Say::default().parse(lex, loc),
        }
    }
}
