use crate::{
    ast::{
        AstNode, Call, Default_, Define, Hide, If, Image, Init, Jump, Label, Menu, Pass, Python,
        PythonOneLine, Return, Say, Scene, Screen, Show, Style, Transform, UserStatement, With,
    },
    lexer::Lexer,
    parser::Parser,
};
use anyhow::Result;
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

    pub fn init(&mut self) {
        self.add(vec!["label".into()], Box::new(Label::default()));
        self.add(vec!["scene".into()], Box::new(Scene::default()));
        self.add(vec!["with".into()], Box::new(With::default()));
        self.add(vec!["".into()], Box::new(Say::default()));
        self.add(vec!["show".into()], Box::new(Show::default()));
        self.add(vec!["hide".into()], Box::new(Hide::default()));
        self.add(vec!["$".into()], Box::new(PythonOneLine::default()));
        self.add(vec!["jump".into()], Box::new(Jump::default()));
        self.add(vec!["menu".into()], Box::new(Menu::default()));
        self.add(vec!["if".into()], Box::new(If::default()));
        self.add(vec!["return".into()], Box::new(Return::default()));
        self.add(vec!["style".into()], Box::new(Style::default()));
        self.add(vec!["init".into()], Box::new(Init::default()));
        self.add(vec!["python".into()], Box::new(Python::default()));
        self.add(vec!["define".into()], Box::new(Define::default()));
        self.add(vec!["default".into()], Box::new(Default_::default()));
        self.add(vec!["call".into()], Box::new(Call::default()));
        self.add(vec!["pass".into()], Box::new(Pass::default()));
        self.add(vec!["transform".into()], Box::new(Transform::default()));
        self.add(vec!["screen".into()], Box::new(Screen::default()));
        self.add(vec!["image".into()], Box::new(Image::default()));

        let custom_statements = vec![
            // built-in custom statements
            "play music",
            "queue music",
            "stop music",
            "play sound",
            "queue sound",
            "stop sound",
            "play",
            "queue",
            "stop",
            "pause",
            "show screen",
            "call screen",
            "hide screen",
            "nvl show",
            "nvl hide",
            "nvl clear",
            "window show",
            "window hide",
            "window auto",
            // user-defined custom statements, fill these in automatically somehow
            "resumeaudio",
            "pauseaudio",
            "timedchoice",
            "gameover",
            "text",
            "msg",
            "title",
            "outfit",
            "accessory",
            "body",
            "swap",
            "clone",
            "morph",
            "exspirit",
            "possess",
            "scry",
            "placeholder",
            "routename",
            "unlock",
            "resetstate",
            "FIXME",
            "phone_call",
        ];

        for stmt in custom_statements {
            self.add(
                stmt.split(" ").map(|s| s.to_string()).collect(),
                Box::new(UserStatement::default()),
            );
        }
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

    pub fn parse(&self, lex: &mut Lexer) -> Result<Vec<AstNode>> {
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

        println!("word: {:?}", word);
        // println!("keys: {:?}", self.words.keys());

        if word.is_none() || !self.words.contains_key(&word.clone().unwrap()) {
            println!("parsing {:?}", lex.text);
            println!("no match, defaulting");
            lex.pos = old_pos;
            match self.default.as_ref() {
                Some(parse_cmd) => {
                    println!("parsing {:?}", lex.text);
                    return parse_cmd.parse(lex, loc);
                }
                None => {
                    println!("defaulting to say {:?}", lex.text);
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
