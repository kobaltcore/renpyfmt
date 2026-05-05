use crate::error::{ParseError, Result};
use lazy_static::lazy_static;
use std::path::PathBuf;

use regex::{Regex, RegexBuilder};

#[derive(Debug, Clone)]
pub struct SubParse {
    // Kept for parity with the upstream Ren'Py lexer API; the port does not
    // consume stored subparse blocks yet.
    #[allow(dead_code)]
    block: Block,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub filename: PathBuf,
    pub number: usize,
    pub text: String,
    pub block: Vec<Block>,
}

#[derive(Debug, Clone)]
pub struct LexerState {
    line: Option<usize>,
    pos: usize,
}

#[derive(Debug, Clone)]
pub struct Lexer {
    pub block: Vec<Block>,
    pub init: bool,
    pub init_offset: isize,
    pub global_label: Option<String>,
    pub monologue_delimiter: Option<String>,
    pub subparses: Vec<SubParse>,
    pub eob: bool,
    pub line: Option<usize>,
    pub filename: PathBuf,
    pub text: String,
    pub number: usize,
    pub subblock: Vec<Block>,
    pub pos: usize,
    pub word_cache_pos: Option<usize>,
    pub word_cache_newpos: Option<usize>,
    pub word_cache: String,
}

pub enum LexerTypeOptions {
    Name,
    Hash,
    Integer,
    Word,
    LabelName,
    LabelNameDeclare,
    SimpleExpression,
    ImageNameComponent,
    PythonExpression,
    DottedName,
}

pub enum LexerType {
    String(String),
    Type(LexerTypeOptions),
}

lazy_static! {
    static ref RE_OPERATOR: Regex = RegexBuilder::new(r#"^(<>|<<|<=|<|>>|>=|>|!=|==|\||\^|&|\+|\-|\*\*|\*|\/\/|\/|%|~|@|:=|\bor\b|\band\b|\bnot\b|\bin\b|\bis\b)"#).dot_matches_new_line(true).build().unwrap();
    static ref RE_WORD: Regex = RegexBuilder::new("^[a-zA-Z_\u{00a0}-\u{fffd}][0-9a-zA-Z_\u{00a0}-\u{fffd}]*").dot_matches_new_line(true).build().unwrap();
    static ref RE_WHITESPACE: Regex = RegexBuilder::new(r"^(\s+|\\\n)+").dot_matches_new_line(true).build().unwrap();
    static ref RE_STRING_DOUBLE: Regex = RegexBuilder::new("^r?\"([^\\\"]|\\.)*\"").dot_matches_new_line(true).build().unwrap();
    static ref RE_STRING_SINGLE: Regex = RegexBuilder::new(r"^r?'([^\\']|\\.)*'").dot_matches_new_line(true).build().unwrap();
    static ref RE_STRING_BACK: Regex = RegexBuilder::new(r"^r?`([^\\`]|\\.)*`").dot_matches_new_line(true).build().unwrap();
    static ref RE_STRING_TRIPLE_DOUBLE: Regex = RegexBuilder::new("^r?\"\"\"([^\\\"]|\\.|\"{1,2}[^\"])*\"\"\"").dot_matches_new_line(true).build().unwrap();
    static ref RE_STRING_TRIPLE_SINGLE: Regex = RegexBuilder::new(r"^r?'''([^\\']|\\.|'{1,2}[^'])*'''").dot_matches_new_line(true).build().unwrap();
    static ref RE_STRING_TRIPLE_BACK: Regex = RegexBuilder::new(r"^r?```([^\\`]|\\.|`{1,2}[^`])*```").dot_matches_new_line(true).build().unwrap();
    static ref RE_IMAGE_NAME: Regex = RegexBuilder::new("^[-0-9a-zA-Z_\u{00a0}-\u{fffd}][-0-9a-zA-Z_\u{00a0}-\u{fffd}]*").dot_matches_new_line(true).build().unwrap();
    static ref RE_FLOAT: Regex = RegexBuilder::new(r"^(\+|\-)?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?").dot_matches_new_line(true).build().unwrap();
    static ref RE_INTEGER: Regex = RegexBuilder::new(r"^(\+|\-)?\d+").dot_matches_new_line(true).build().unwrap();
    static ref RE_PYTHON_STRING: Regex = RegexBuilder::new("^[urfURF]*(\"\"\"|\'\'\'|\"|\')").dot_matches_new_line(true).build().unwrap();
    static ref RE_STRING_NEWLINE_REPLACE: Regex = Regex::new(r"[ \n]+").unwrap();
    static ref RE_STRING_INTERNAL_1: Regex = Regex::new(r"\\(u([0-9a-fA-F]{1,4})|.)").unwrap();
    static ref RE_NEWLINES: Regex = Regex::new(r" *\n *").unwrap();
    static ref RE_SPACES: Regex = Regex::new(r" +").unwrap();
    static ref RE_PYTHON_STRING_INTERNAL_1: Regex = Regex::new(r#"^.[^'"\\]*"#).unwrap();
}

pub enum GlobalRegex {
    Operator,
    Word,
    Whitespace,
    StringDouble,
    StringSingle,
    StringBack,
    StringTripleDouble,
    StringTripleSingle,
    StringTripleBack,
    ImageName,
    Float,
    Integer,
    PythonString,
    StringNewLineReplace,
    PythonStringInternal1,
}

pub enum RegexType {
    /// Will be matched as-is
    Simple(&'static str),
    /// Will be parsed into a Regex
    String(String),
    GlobalRegex(GlobalRegex),
}

impl Into<RegexType> for String {
    fn into(self) -> RegexType {
        RegexType::String(self)
    }
}

impl Into<RegexType> for &str {
    fn into(self) -> RegexType {
        RegexType::String(self.into())
    }
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "$" | "as"
            | "at"
            | "behind"
            | "call"
            | "expression"
            | "hide"
            | "if"
            | "in"
            | "image"
            | "init"
            | "jump"
            | "menu"
            | "onlayer"
            | "python"
            | "return"
            | "scene"
            | "show"
            | "transform"
            | "while"
            | "with"
            | "zorder"
    )
}

fn literal_pattern(pattern: &str) -> Option<&'static str> {
    match pattern {
        r"\(" => Some("("),
        r"\)" => Some(")"),
        r"\*" => Some("*"),
        r"\*\*" => Some("**"),
        r"\$" => Some("$"),
        r"\." => Some("."),
        r"\[" => Some("["),
        r"\+=" => Some("+="),
        r"\|=" => Some("|="),
        r"\@" => Some("@"),
        r"/\*" => Some("/*"),
        "=" => Some("="),
        "," => Some(","),
        ":" => Some(":"),
        "-" => Some("-"),
        "/" => Some("/"),
        "3" => Some("3"),
        _ => None,
    }
}

fn global_regex(regex: GlobalRegex) -> &'static Regex {
    match regex {
        GlobalRegex::Operator => &RE_OPERATOR,
        GlobalRegex::Word => &RE_WORD,
        GlobalRegex::Whitespace => &RE_WHITESPACE,
        GlobalRegex::StringDouble => &RE_STRING_DOUBLE,
        GlobalRegex::StringSingle => &RE_STRING_SINGLE,
        GlobalRegex::StringBack => &RE_STRING_BACK,
        GlobalRegex::StringTripleDouble => &RE_STRING_TRIPLE_DOUBLE,
        GlobalRegex::StringTripleSingle => &RE_STRING_TRIPLE_SINGLE,
        GlobalRegex::StringTripleBack => &RE_STRING_TRIPLE_BACK,
        GlobalRegex::ImageName => &RE_IMAGE_NAME,
        GlobalRegex::Float => &RE_FLOAT,
        GlobalRegex::Integer => &RE_INTEGER,
        GlobalRegex::PythonString => &RE_PYTHON_STRING,
        GlobalRegex::StringNewLineReplace => &RE_STRING_NEWLINE_REPLACE,
        GlobalRegex::PythonStringInternal1 => &RE_PYTHON_STRING_INTERNAL_1,
    }
}

impl Lexer {
    pub fn new(block: Vec<Block>) -> Lexer {
        Lexer {
            block,
            init: false,
            init_offset: 0,
            global_label: None,
            monologue_delimiter: Some("\n\n".into()),
            subparses: Vec::new(),
            // internal state
            eob: false,
            line: None,
            filename: "".into(),
            text: "".into(),
            number: 0,
            subblock: Vec::new(),
            pos: 0,
            word_cache_pos: None,
            word_cache_newpos: None,
            word_cache: "".into(),
        }
    }

    pub fn set_init(&mut self, init: bool) {
        self.init = init;
    }

    pub fn set_init_offset(&mut self, offset: isize) {
        self.init_offset = offset;
    }

    pub fn set_global_label(&mut self, label: Option<String>) {
        self.global_label = label;
    }

    pub fn set_mono_delim(&mut self, delim: Option<String>) {
        self.monologue_delimiter = delim;
    }

    pub fn set_subparses(&mut self, subparses: Vec<SubParse>) {
        self.subparses = subparses;
    }

    pub fn advance(&mut self) -> bool {
        match self.line {
            Some(l) => self.line = Some(l + 1),
            None => self.line = Some(0),
        };

        // println!(
        //     "line: {} | block.len(): {}",
        //     self.line.unwrap(),
        //     self.block.len()
        // );
        if self.line.unwrap() >= self.block.len() {
            // println!("setting eob");
            self.eob = true;
            return false;
        }

        self.restore_line_state(self.line.unwrap());

        self.pos = 0;
        self.word_cache_pos = None;

        return true;
    }

    pub fn unadvance(&mut self) {
        self.line = Some(self.line.unwrap() - 1);
        self.eob = false;

        self.restore_line_state(self.line.unwrap());

        self.pos = self.text.len();
        self.word_cache_pos = None;
    }

    fn restore_line_state(&mut self, line: usize) {
        let block = &self.block[line];
        self.filename = block.filename.clone();
        self.number = block.number;
        self.text = block.text.clone();
        self.subblock = block.block.clone();
    }

    pub fn match_regexp(&mut self, regexp: RegexType) -> Option<String> {
        if self.eob {
            return None;
        }

        if self.pos == self.text.len() {
            return None;
        }

        let substr = &self.text[self.pos..];
        let pattern = match regexp {
            RegexType::Simple(s) => {
                if substr.starts_with(&s) {
                    self.pos += s.len();
                    return Some(s.to_string());
                }
                return None;
            }
            RegexType::String(s) => {
                if let Some(literal) = literal_pattern(&s) {
                    if substr.starts_with(literal) {
                        self.pos += literal.len();
                        return Some(literal.to_string());
                    }
                    return None;
                }

                return RegexBuilder::new(&format!("^{s}"))
                    .dot_matches_new_line(true)
                    .build()
                    .unwrap()
                    .find(substr)
                    .filter(|m| m.end() > 0)
                    .map(|m| {
                        self.pos += m.end();
                        m.as_str().to_string()
                    });
            }
            RegexType::GlobalRegex(r) => global_regex(r),
        };

        if let Some(m) = pattern.find(substr) {
            if m.end() == 0 {
                return None;
            }
            self.pos += m.end();
            return Some(m.as_str().into());
        }

        None
    }

    pub fn skip_whitespace(&mut self) {
        self.match_regexp(RegexType::GlobalRegex(GlobalRegex::Whitespace));
    }

    pub fn rmatch(&mut self, regexp: RegexType) -> Option<String> {
        self.skip_whitespace();
        self.match_regexp(regexp)
    }

    pub fn match_literal(&mut self, literal: &'static str) -> Option<String> {
        self.match_regexp(RegexType::Simple(literal))
    }

    pub fn rmatch_literal(&mut self, literal: &'static str) -> Option<String> {
        self.skip_whitespace();
        self.match_literal(literal)
    }

    pub fn keyword(&mut self, word: String) -> Option<String> {
        let oldpos = self.pos;
        if self.word().as_deref() == Some(word.as_str()) {
            return Some(word);
        }

        self.pos = oldpos;
        return None;
    }

    pub fn word(&mut self) -> Option<String> {
        if Some(self.pos) == self.word_cache_pos {
            self.pos = self.word_cache_newpos.unwrap();
            if self.word_cache.len() == 0 {
                return None;
            }
            return Some(self.word_cache.clone());
        }

        self.word_cache_pos = Some(self.pos);
        let rv = self.rmatch(RegexType::GlobalRegex(GlobalRegex::Word));
        self.word_cache = rv.clone().unwrap_or("".into());
        self.word_cache_newpos = Some(self.pos);

        rv
    }

    pub fn eol(&mut self) -> bool {
        self.skip_whitespace();
        return self.pos >= self.text.len();
    }

    pub fn has_block(&mut self) -> bool {
        return self.subblock.len() > 0;
    }

    pub fn subblock_lexer(&mut self, init: bool) -> Lexer {
        let mut lex = Lexer::new(self.subblock.clone());

        lex.set_init(self.init || init);
        lex.set_init_offset(self.init_offset);
        lex.set_global_label(self.global_label.clone());
        lex.set_mono_delim(self.monologue_delimiter.clone());
        lex.set_subparses(self.subparses.clone());

        lex
    }

    pub fn string(&mut self) -> Option<String> {
        let mut s = self.rmatch(RegexType::GlobalRegex(GlobalRegex::StringDouble));

        if s.is_none() {
            s = self.rmatch(RegexType::GlobalRegex(GlobalRegex::StringSingle));
        }

        if s.is_none() {
            s = self.rmatch(RegexType::GlobalRegex(GlobalRegex::StringBack));
        }

        if let Some(s) = s {
            let mut s = s;
            let mut raw = false;
            if s.chars().nth(0) == Some('r') {
                raw = true;
                s = s[1..].into();
            }

            s = s[1..s.len() - 1].into();

            if !raw {
                let re = RE_STRING_NEWLINE_REPLACE.clone();
                re.replace(&s, " ");

                let re = RE_STRING_INTERNAL_1.clone();
                let mut caps = re.captures_iter(&s).collect::<Vec<_>>();
                caps.reverse();
                let mut s = s.clone();
                for m in caps {
                    let capture = m.get(1).unwrap();
                    let c = m.get(1).unwrap().as_str().chars().collect::<Vec<_>>();
                    if c.len() == 1 {
                        match c[0] {
                            '{' => {
                                s.replace_range(capture.range(), "{{");
                            }
                            '[' => {
                                s.replace_range(capture.range(), "[[");
                            }
                            '%' => {
                                s.replace_range(capture.range(), "%%");
                            }
                            'n' => {
                                s.replace_range(capture.range(), "\n");
                            }
                            _ => {}
                        };
                    } else if c[0] == 'u' {
                        if let Some(g2) = m.get(2) {
                            let code = u32::from_str_radix(g2.as_str(), 16).unwrap();
                            let c = char::from_u32(code).unwrap().to_string();
                            s.replace_range(capture.range(), &c);
                        }
                    }
                }
            }

            return Some(s);
        }

        None
    }

    pub fn triple_string(&mut self) -> Option<Vec<String>> {
        let mut s = self.rmatch(RegexType::GlobalRegex(GlobalRegex::StringTripleDouble));

        if s.is_none() {
            s = self.rmatch(RegexType::GlobalRegex(GlobalRegex::StringTripleSingle));
        }

        if s.is_none() {
            s = self.rmatch(RegexType::GlobalRegex(GlobalRegex::StringTripleBack));
        }

        if let Some(s) = s {
            let mut s = s;
            let mut raw = false;
            if s.chars().nth(0) == Some('r') {
                raw = true;
                s = s[1..].into();
            }

            s = s[3..s.len() - 3].into();

            if !raw {
                let re = RE_NEWLINES.clone();
                re.replace(&s, "\n");

                let sl = match &self.monologue_delimiter {
                    Some(mondel) => s.split(mondel).map(|s| s.to_string()).collect::<Vec<_>>(),
                    None => vec![s.clone()],
                };

                let mut result = vec![];

                for s in sl {
                    let s = s.trim();

                    if s.len() == 0 {
                        continue;
                    }

                    let s: String = match &self.monologue_delimiter {
                        Some(_) => RE_STRING_NEWLINE_REPLACE
                            .clone()
                            .replace_all(&s, " ")
                            .into(),
                        None => RE_SPACES.clone().replace_all(&s, " ").into(),
                    };

                    let re = RE_STRING_INTERNAL_1.clone();
                    let mut caps = re.captures_iter(&s).collect::<Vec<_>>();
                    caps.reverse();
                    let mut s = s.clone();
                    for m in caps {
                        let capture = m.get(1).unwrap();
                        let c = m.get(1).unwrap().as_str().chars().collect::<Vec<_>>();
                        if c.len() == 1 {
                            match c[0] {
                                '{' => {
                                    s.replace_range(capture.range(), "{{");
                                }
                                '[' => {
                                    s.replace_range(capture.range(), "[[");
                                }
                                '%' => {
                                    s.replace_range(capture.range(), "%%");
                                }
                                'n' => {
                                    s.replace_range(capture.range(), "\n");
                                }
                                _ => {}
                            };
                        } else if c[0] == 'u' {
                            if let Some(g2) = m.get(2) {
                                let code = u32::from_str_radix(g2.as_str(), 16).unwrap();
                                let c = char::from_digit(code, 10).unwrap().to_string();
                                s.replace_range(capture.range(), &c);
                            }
                        }
                    }
                }

                result.push(s);

                return Some(result);
            }

            return Some(vec![s]);
        }

        None
    }

    pub fn get_location(&mut self) -> (PathBuf, usize) {
        (self.filename.clone(), self.number)
    }

    pub fn parse_error(&self, message: impl Into<String>) -> ParseError {
        ParseError::at((self.filename.clone(), self.number), message)
    }

    pub fn require(&mut self, thing: LexerType) -> Result<Option<String>> {
        match thing {
            LexerType::String(s) => Ok(self.rmatch(s.into())),
            LexerType::Type(t) => match t {
                LexerTypeOptions::Name => Ok(self.name()),
                LexerTypeOptions::Hash => Ok(self.rmatch(RegexType::String(r"\w+".into()))),
                LexerTypeOptions::Integer => Ok(self.integer()),
                LexerTypeOptions::Word => Ok(self.word()),
                LexerTypeOptions::LabelNameDeclare => Ok(self.label_name_declare()),
                LexerTypeOptions::SimpleExpression => self.simple_expression(false, true),
                LexerTypeOptions::ImageNameComponent => Ok(self.image_name_component()),
                LexerTypeOptions::LabelName => Ok(self.label_name(false)),
                LexerTypeOptions::PythonExpression => self.python_expression().map(Some),
                LexerTypeOptions::DottedName => self.dotted_name(),
            },
        }
    }

    pub fn require_or_error(
        &mut self,
        thing: LexerType,
        message: impl Into<String>,
    ) -> Result<String> {
        self.require(thing)?
            .ok_or_else(|| self.parse_error(message))
    }

    pub fn expect_eol(&mut self) -> Result<()> {
        if !self.eol() {
            return Err(self.parse_error("end of line expected"));
        }

        Ok(())
    }

    pub fn name(&mut self) -> Option<String> {
        // println!("name");
        let old_pos = self.pos;
        let rv = self.word();

        match rv {
            Some(rv) => {
                if (rv == "r" || rv == "u" || rv == "ur")
                    && (&self.text[self.pos..self.pos + 1] == "\""
                        || &self.text[self.pos..self.pos + 1] == "'"
                        || &self.text[self.pos..self.pos + 1] == "`")
                {
                    self.pos = old_pos;
                    return None;
                }

                if is_keyword(&rv) {
                    self.pos = old_pos;
                    return None;
                }

                Some(rv)
            }
            None => rv,
        }
    }

    pub fn label_name(&mut self, declare: bool) -> Option<String> {
        let old_pos = self.pos;
        let mut local_name: Option<String> = None;
        let mut global_name = self.name();

        match global_name {
            Some(ref global_name) => {
                if self.rmatch(RegexType::Simple(".")).is_some() {
                    if declare && Some(global_name) != self.global_label.as_ref() {
                        self.pos = old_pos;
                        return None;
                    }

                    local_name = self.name();
                    if local_name.is_none() {
                        self.pos = old_pos;
                        return None;
                    }
                }
            }
            None => {
                if self.rmatch(RegexType::Simple(".")).is_none() || self.global_label.is_none() {
                    self.pos = old_pos;
                    return None;
                }
                global_name = self.global_label.clone();
                local_name = self.name();
                if local_name.is_none() {
                    self.pos = old_pos;
                    return None;
                }
            }
        };

        match local_name {
            Some(local_name) => Some(format!("{}.{local_name}", global_name.unwrap())),
            None => return Some(global_name.unwrap()),
        }
    }

    pub fn label_name_declare(&mut self) -> Option<String> {
        self.label_name(true)
    }

    pub fn python_string(&mut self) -> Result<bool> {
        // println!("python string");
        if self.eol() {
            return Ok(false);
        }

        let old_pos = self.pos;

        let start = self.rmatch(RegexType::GlobalRegex(GlobalRegex::PythonString));

        if start.is_none() {
            self.pos = old_pos;
            return Ok(false);
        }

        let delim: String = start.unwrap().trim_start_matches("urfURF").into();

        loop {
            if self.eol() {
                self.pos = old_pos;
                return Err(self.parse_error("end of line reached while parsing string."));
            }

            self.skip_whitespace();
            if self.text[self.pos..].starts_with(&delim) {
                self.pos += delim.len();
                break;
            }

            if self.rmatch(RegexType::Simple(r"\\")).is_some() {
                self.pos += 1;
                break;
            }

            self.rmatch(RegexType::GlobalRegex(GlobalRegex::PythonStringInternal1))
                .ok_or_else(|| self.parse_error("end of line reached while parsing string."))?;
        }

        Ok(true)
    }

    pub fn parenthesised_python(&mut self) -> Result<bool> {
        // println!("parenthesised python");
        let Some(c) = self.text[self.pos..].chars().next() else {
            return Ok(false);
        };

        let old_pos = self.pos;

        match c {
            '(' => {
                self.pos += 1;
                if let Err(err) = self.delimited_python(")", false) {
                    self.pos = old_pos;
                    return Err(err);
                }
                self.pos += 1;
                Ok(true)
            }
            '[' => {
                self.pos += 1;
                if let Err(err) = self.delimited_python("]", false) {
                    self.pos = old_pos;
                    return Err(err);
                }
                self.pos += 1;
                Ok(true)
            }
            '{' => {
                self.pos += 1;
                if let Err(err) = self.delimited_python("}", false) {
                    self.pos = old_pos;
                    return Err(err);
                }
                self.pos += 1;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn delimited_python(&mut self, delim: &'static str, _expr: bool) -> Result<Option<String>> {
        let start = self.pos;

        while !self.eol() {
            let c = self.text[self.pos..]
                .chars()
                .next()
                .expect("eol checked above");

            if delim.contains(c) {
                return Ok(Some(self.text[start..self.pos].to_string()));
            }

            if ['\'', '"'].contains(&c) {
                self.python_string()?;
                continue;
            }

            if self.parenthesised_python()? {
                continue;
            }

            self.pos += c.len_utf8();
        }

        Err(self.parse_error(format!("reached end of line when expecting '{delim}'")))
    }

    pub fn float(&mut self) -> Option<String> {
        // println!("float");
        self.rmatch(RegexType::GlobalRegex(GlobalRegex::Float))
    }

    pub fn simple_expression(&mut self, comma: bool, operator: bool) -> Result<Option<String>> {
        // self.skip_whitespace();
        let start = self.pos;

        loop {
            while self
                .rmatch(RegexType::GlobalRegex(GlobalRegex::Operator))
                .is_some()
            {
                // println!("operator skip");
                continue;
            }

            // println!("after operator skip {:?}", &self.text[self.pos..]);

            if self.eol() {
                // println!("eol skip");
                break;
            }

            if !(self.python_string()?
                || self.name().is_some()
                || self.float().is_some()
                || self.parenthesised_python()?)
            {
                // println!("string/name/float/python skip {}", self.pos);
                break;
            }

            loop {
                // println!("2 pos: {}", self.pos);
                self.skip_whitespace();

                if self.eol() {
                    break;
                }

                if self.rmatch(RegexType::Simple(".")).is_some() {
                    let n = self.word();
                    if n.is_none() {
                        return Err(self.parse_error("expecting name after dot."));
                    }
                    continue;
                }

                if self.parenthesised_python()? {
                    continue;
                }

                break;
            }

            if operator
                && self
                    .rmatch(RegexType::GlobalRegex(GlobalRegex::Operator))
                    .is_some()
            {
                continue;
            }

            if comma && self.rmatch(RegexType::Simple(",")).is_some() {
                continue;
            }

            break;
        }
        // println!("start: {} | pos: {}", start, self.pos);

        let text = self.text[start..self.pos].trim().to_string();

        // println!("text: {:?}", text);

        if text.len() == 0 {
            return Ok(None);
        }

        Ok(Some(text.into()))
    }

    pub fn checkpoint(&mut self) -> LexerState {
        LexerState {
            line: self.line,
            pos: self.pos,
        }
    }

    pub fn image_name_component(&mut self) -> Option<String> {
        let oldpos = self.pos;
        let rv = self.rmatch(RegexType::GlobalRegex(GlobalRegex::ImageName));

        if rv == Some("r".into()) || rv == Some("u".into()) {
            if ['"', '\'', '`'].contains(&self.text.chars().nth(self.pos).unwrap()) {
                self.pos = oldpos;
                return None;
            }
        }

        if rv.as_deref().is_some_and(is_keyword) {
            self.pos = oldpos;
            return None;
        }

        rv
    }

    pub fn revert(&mut self, state: LexerState) {
        self.line = state.line;
        self.pos = state.pos;

        self.word_cache_pos = None;

        if let Some(line) = self.line {
            if line < self.block.len() {
                self.restore_line_state(line);
                self.eob = false;
            } else {
                self.eob = true;
            }
        } else {
            self.filename.clear();
            self.number = 0;
            self.text.clear();
            self.subblock.clear();
            self.eob = false;
        }
    }

    pub fn expect_block(&mut self) -> Result<()> {
        if self.subblock.len() == 0 {
            return Err(self.parse_error("expected a non-empty block."));
        }

        Ok(())
    }

    pub fn expect_noblock(&mut self) -> Result<()> {
        if self.subblock.len() > 0 {
            let mut ll = self.subblock_lexer(false);
            ll.advance();
            return Err(self.parse_error("Line is indented, but the preceding statement does not expect a block. Please check this line's indentation. You may have forgotten a colon (:)."));
        }

        Ok(())
    }

    pub fn say_expression(&mut self) -> Result<Option<String>> {
        self.simple_expression(false, false)
    }

    pub fn rest_statement(&mut self) -> Option<String> {
        let pos = self.pos;
        self.pos = self.text.len();
        let rv = self.text[pos..].to_string();
        if rv.len() == 0 {
            return None;
        }
        Some(rv)
    }

    pub fn python_expression(&mut self) -> Result<String> {
        let pe = self.delimited_python(":", false)?;

        match pe {
            Some(s) => Ok(s.trim().into()),
            None => Err(self.parse_error("expected python_expression")),
        }
    }

    pub fn rest(&mut self) -> Option<String> {
        self.skip_whitespace();

        let pos = self.pos;
        self.pos = self.text.len();
        let rv = self.text[pos..].trim().to_string();

        if rv.len() == 0 {
            return None;
        }

        Some(rv)
    }

    pub fn integer(&mut self) -> Option<String> {
        self.rmatch(RegexType::GlobalRegex(GlobalRegex::Integer).into())
    }

    pub fn dotted_name(&mut self) -> Result<Option<String>> {
        let mut rv = self.name();

        if rv.is_none() {
            return Ok(None);
        }

        while self.rmatch(RegexType::Simple(".")).is_some() {
            let n = self.name();
            if n.is_none() {
                return Err(self.parse_error("expecting name."));
            }
            rv = Some(format!("{}.{}", rv.unwrap(), n.unwrap()));
        }

        Ok(rv)
    }

    pub fn python_block(&mut self) -> Option<String> {
        let mut rv = vec![];

        let mut line = self.number;

        process(&mut rv, &mut line, &self.subblock, "");

        if rv.len() == 0 {
            return None;
        }

        Some(rv.join(""))
    }
}

fn process(rv: &mut Vec<String>, line: &mut usize, blocks: &[Block], indent: &str) {
    let mut next_indent = String::with_capacity(indent.len() + 4);
    next_indent.push_str(indent);
    next_indent.push_str("    ");

    for b in blocks {
        let ln = b.number;
        let text = &b.text;
        let subblock = &b.block;

        while *line < ln {
            rv.push(format!("{indent}\n"));
            *line += 1;
        }

        let linetext = format!("{indent}{text}\n");

        rv.push(linetext.clone());
        *line += linetext.matches("\n").count();

        process(rv, line, subblock, &next_indent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(number: usize, text: &str, block: Vec<Block>) -> Block {
        Block {
            filename: PathBuf::from("test.rpy"),
            number,
            text: text.into(),
            block,
        }
    }

    fn single_line_lexer(text: &str) -> Lexer {
        let mut lex = Lexer::new(vec![block(1, text, vec![])]);
        lex.advance();
        lex
    }

    #[test]
    fn string_and_triple_string_handle_escapes_and_raw_strings() {
        let mut lex = single_line_lexer("\"hello\\n\\u0041\\{\"");
        assert_eq!(lex.string().as_deref(), Some("hello\\n\\u0041\\{"));

        let mut raw = single_line_lexer(r#"r"hello\n""#);
        assert_eq!(raw.string().as_deref(), Some(r#"hello\n"#));

        let mut triple = single_line_lexer("\"\"\"First\n\nSecond\"\"\"");
        assert_eq!(
            triple.triple_string(),
            Some(vec!["First\n\nSecond".to_string()])
        );

        let mut triple_raw = single_line_lexer("r'''raw\\ntext'''");
        assert_eq!(
            triple_raw.triple_string(),
            Some(vec![r"raw\ntext".to_string()])
        );
    }

    #[test]
    fn advance_unadvance_checkpoint_and_revert_restore_state() {
        let mut lex = Lexer::new(vec![block(1, "first", vec![]), block(2, "second", vec![])]);
        assert!(lex.advance());
        assert_eq!(lex.text, "first");

        let state = lex.checkpoint();
        lex.pos = 3;
        lex.advance();
        assert_eq!(lex.text, "second");

        lex.unadvance();
        assert_eq!(lex.text, "first");
        assert_eq!(lex.pos, lex.text.len());

        lex.revert(state);
        assert_eq!(lex.text, "first");
        assert_eq!(lex.pos, 0);
        assert!(!lex.eob);
    }

    #[test]
    fn name_label_and_image_name_rules_respect_keywords_and_global_labels() {
        let mut keyword = single_line_lexer("show");
        assert_eq!(keyword.name(), None);

        let mut label = single_line_lexer("start.local");
        label.set_global_label(Some("start".into()));
        assert_eq!(label.label_name(true).as_deref(), Some("start.local"));

        let mut relative = single_line_lexer(".branch");
        relative.set_global_label(Some("start".into()));
        assert_eq!(relative.label_name(false).as_deref(), Some("start.branch"));

        let mut image = single_line_lexer("r\"not_image\"");
        assert_eq!(image.image_name_component(), None);
    }

    #[test]
    fn python_scanners_handle_strings_parentheses_and_expressions() {
        let mut lex = single_line_lexer("func(\"a\", [1, 2], {'k': value}) + other.attr");
        assert_eq!(
            lex.simple_expression(false, true),
            Ok(Some(
                "func(\"a\", [1, 2], {'k': value}) + other.attr".into()
            ))
        );

        let mut python = single_line_lexer("value[foo(\"bar\")] : rest");
        assert_eq!(
            python.python_expression().as_deref(),
            Ok("value[foo(\"bar\")]")
        );

        let mut dotted = single_line_lexer("store.module.value");
        assert_eq!(dotted.dotted_name(), Ok(Some("store.module.value".into())));
    }

    #[test]
    fn python_scanners_report_errors_instead_of_panicking() {
        let mut dotted = single_line_lexer("store.");
        assert_eq!(
            dotted.dotted_name().unwrap_err(),
            ParseError::at((PathBuf::from("test.rpy"), 1), "expecting name.")
        );

        let mut expr = single_line_lexer("foo.");
        assert_eq!(
            expr.simple_expression(false, true).unwrap_err(),
            ParseError::at((PathBuf::from("test.rpy"), 1), "expecting name after dot.")
        );

        let mut paren = single_line_lexer("foo(");
        assert_eq!(
            paren.simple_expression(false, true).unwrap_err(),
            ParseError::at(
                (PathBuf::from("test.rpy"), 1),
                "reached end of line when expecting ')'"
            )
        );

        let mut string = single_line_lexer("\"unterminated");
        assert_eq!(
            string.simple_expression(false, true).unwrap_err(),
            ParseError::at(
                (PathBuf::from("test.rpy"), 1),
                "end of line reached while parsing string."
            )
        );
    }

    #[test]
    fn python_block_and_subblock_lexer_preserve_context() {
        let blocks = vec![block(
            10,
            "python:",
            vec![
                block(11, "x = 1", vec![]),
                block(13, "if True:", vec![block(14, "pass", vec![])]),
            ],
        )];
        let mut lex = Lexer::new(blocks);
        lex.set_init(true);
        lex.set_init_offset(7);
        lex.set_global_label(Some("start".into()));
        lex.advance();

        let child = lex.subblock_lexer(false);
        assert!(child.init);
        assert_eq!(child.init_offset, 7);
        assert_eq!(child.global_label.as_deref(), Some("start"));

        let code = lex.python_block().expect("expected python block");
        assert_eq!(code, "\nx = 1\n\nif True:\n    pass\n");
    }

    #[test]
    fn block_and_rest_helpers_report_expected_errors() {
        let mut no_block = single_line_lexer("line");
        assert_eq!(
            no_block.expect_block().unwrap_err(),
            ParseError::at(
                (PathBuf::from("test.rpy"), 1),
                "expected a non-empty block."
            )
        );

        let mut with_block = Lexer::new(vec![block(1, "line", vec![block(2, "child", vec![])])]);
        with_block.advance();
        assert_eq!(
            with_block.expect_noblock().unwrap_err(),
            ParseError::at(
                (PathBuf::from("test.rpy"), 1),
                "Line is indented, but the preceding statement does not expect a block. Please check this line's indentation. You may have forgotten a colon (:)."
            )
        );

        let mut rest = single_line_lexer("   trailing words  ");
        assert_eq!(rest.rest().as_deref(), Some("trailing words"));

        let mut rest_stmt = single_line_lexer(" code here");
        assert_eq!(rest_stmt.rest_statement().as_deref(), Some(" code here"));
    }
}
