use crate::{
    ast::Say,
    error::Result,
    lexer::Lexer,
    parser::{ParseNodes, Parser},
};
use std::collections::HashMap;

pub struct ParseTrie {
    default: Option<Box<dyn Parser>>,
    words: HashMap<String, ParseTrie>,
}

impl ParseTrie {
    pub fn new() -> ParseTrie {
        let parser = ParseTrie {
            default: None,
            words: HashMap::new(),
        };

        parser
    }

    pub fn add(&mut self, name: Vec<String>, parser: Box<dyn Parser>) {
        if name.len() > 0 {
            let first = name.first().unwrap();
            let rest = name[1..].into();

            if !self.words.contains_key(first) {
                self.words.insert(first.clone(), ParseTrie::new());
            }

            self.words.entry(first.clone()).and_modify(|e| {
                e.add(rest, parser);
            });
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
            None => lex.rmatch(r"\$".into()),
        } {
            Some(word) => Some(word),
            None => Some("".into()),
        };

        if word.is_none() || !self.words.contains_key(&word.clone().unwrap()) {
            lex.pos = old_pos;
            match self.default.as_ref() {
                Some(parse_cmd) => {
                    return parse_cmd.parse(lex, loc);
                }
                None => {
                    return Say::default().parse(lex, loc);
                    // panic!("unexpected word: {}", word.unwrap());
                    // lex.advance();
                    // return Ok(vec![]);
                }
            };
        }

        // println!("match, parsing");

        let trie = self.words.get(&word.unwrap()).unwrap();
        return trie.parse(lex);
    }
}
