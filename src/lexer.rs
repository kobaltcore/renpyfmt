use lazy_static::lazy_static;
use std::{collections::HashSet, path::PathBuf};

use regex::{Regex, RegexBuilder};

#[derive(Debug, Clone)]
pub struct SubParse {
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
    filename: PathBuf,
    number: usize,
    text: String,
    subblock: Vec<Block>,
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
    pub keywords: HashSet<String>,
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
    Simple(String),
    /// Will be parsed into a Regex
    String(String),
    /// Will be matched as-is
    Regex(Regex),
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

impl Into<RegexType> for Regex {
    fn into(self) -> RegexType {
        RegexType::Regex(self)
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
            keywords: HashSet::from_iter(vec![
                "$".into(),
                "as".into(),
                "at".into(),
                "behind".into(),
                "call".into(),
                "expression".into(),
                "hide".into(),
                "if".into(),
                "in".into(),
                "image".into(),
                "init".into(),
                "jump".into(),
                "menu".into(),
                "onlayer".into(),
                "python".into(),
                "return".into(),
                "scene".into(),
                "show".into(),
                "with".into(),
                "while".into(),
                "zorder".into(),
                "transform".into(),
            ]),
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

        let block = self.block[self.line.unwrap()].clone();
        self.filename = block.filename;
        self.number = block.number;
        self.text = block.text;
        self.subblock = block.block;

        self.pos = 0;
        self.word_cache_pos = None;

        return true;
    }

    pub fn unadvance(&mut self) {
        self.line = Some(self.line.unwrap() - 1);
        self.eob = false;

        let block = self.block[self.line.unwrap()].clone();
        self.filename = block.filename;
        self.number = block.number;
        self.text = block.text;
        self.subblock = block.block;

        self.pos = self.text.len();
        self.word_cache_pos = None;
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
                    return Some(s.clone());
                }
                return None;
            }
            RegexType::String(s) => RegexBuilder::new(&format!("^{s}"))
                .dot_matches_new_line(true)
                .build()
                .unwrap(),
            RegexType::Regex(r) => r.clone(),
            RegexType::GlobalRegex(r) => match r {
                GlobalRegex::Operator => RE_OPERATOR.clone(),
                GlobalRegex::Word => RE_WORD.clone(),
                GlobalRegex::Whitespace => RE_WHITESPACE.clone(),
                GlobalRegex::StringDouble => RE_STRING_DOUBLE.clone(),
                GlobalRegex::StringSingle => RE_STRING_SINGLE.clone(),
                GlobalRegex::StringBack => RE_STRING_BACK.clone(),
                GlobalRegex::StringTripleDouble => RE_STRING_TRIPLE_DOUBLE.clone(),
                GlobalRegex::StringTripleSingle => RE_STRING_TRIPLE_SINGLE.clone(),
                GlobalRegex::StringTripleBack => RE_STRING_TRIPLE_BACK.clone(),
                GlobalRegex::ImageName => RE_IMAGE_NAME.clone(),
                GlobalRegex::Float => RE_FLOAT.clone(),
                GlobalRegex::PythonString => RE_PYTHON_STRING.clone(),
                GlobalRegex::StringNewLineReplace => RE_STRING_NEWLINE_REPLACE.clone(),
                GlobalRegex::PythonStringInternal1 => RE_PYTHON_STRING_INTERNAL_1.clone(),
                GlobalRegex::Integer => RE_INTEGER.clone(),
            },
        };
        // println!("matching '{}' against '{}'", substr, regexp);
        if let Some(m) = pattern.find(substr) {
            if m.end() == 0 {
                return None;
            }
            self.pos += m.end();
            // println!(
            //     "matched: '{}' from {} to {} | new substring: '{}'",
            //     m.as_str(),
            //     m.start(),
            //     m.end(),
            //     self.text[self.pos..].chars().collect::<String>()
            // );
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

    pub fn keyword(&mut self, word: String) -> Option<String> {
        let oldpos = self.pos;
        if self.word() == Some(word.clone()) {
            // println!("keyword: {:?}", word);
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

    pub fn require(&mut self, thing: LexerType) -> Option<String> {
        match thing {
            LexerType::String(s) => self.rmatch(s.into()),
            LexerType::Type(t) => match t {
                LexerTypeOptions::Name => self.name(),
                LexerTypeOptions::Hash => todo!(),
                LexerTypeOptions::Integer => todo!(),
                LexerTypeOptions::Word => self.word(),
                LexerTypeOptions::LabelNameDeclare => self.label_name_declare(),
                LexerTypeOptions::SimpleExpression => self.simple_expression(false, true),
                LexerTypeOptions::ImageNameComponent => self.image_name_component(),
                LexerTypeOptions::LabelName => self.label_name(false),
                LexerTypeOptions::PythonExpression => self.python_expression(),
                LexerTypeOptions::DottedName => self.dotted_name(),
            },
        }
    }

    pub fn expect_eol(&mut self) {
        if !self.eol() {
            panic!("end of line expected");
        }
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

                if self.keywords.contains(&rv) {
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
                if self.rmatch(RegexType::Simple(".".into())).is_some() {
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
                if self.rmatch(RegexType::Simple(".".into())).is_none()
                    || self.global_label.is_none()
                {
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

    pub fn python_string(&mut self) -> bool {
        // println!("python string");
        if self.eol() {
            return false;
        }

        let old_pos = self.pos;

        let start = self.rmatch(RegexType::GlobalRegex(GlobalRegex::PythonString));

        if start.is_none() {
            self.pos = old_pos;
            return false;
        }

        let delim: String = start.unwrap().trim_start_matches("urfURF").into();

        loop {
            if self.eol() {
                panic!("end of line reached while parsing string.");
            }

            if self.rmatch(delim.clone().into()).is_some() {
                break;
            }

            if self.rmatch(RegexType::Simple(r"\\".into())).is_some() {
                self.pos += 1;
                break;
            }

            self.rmatch(RegexType::GlobalRegex(GlobalRegex::PythonStringInternal1))
                .unwrap();
        }

        true
    }

    pub fn parenthesised_python(&mut self) -> bool {
        // println!("parenthesised python");
        let chars = self.text.chars().collect::<Vec<_>>();

        if self.pos >= chars.len() {
            return false;
        }

        let c = chars[self.pos];

        match c {
            '(' => {
                self.pos += 1;
                self.delimited_python(")".into(), false);
                self.pos += 1;
                true
            }
            '[' => {
                self.pos += 1;
                self.delimited_python("]".into(), false);
                self.pos += 1;
                true
            }
            '{' => {
                self.pos += 1;
                self.delimited_python("}".into(), false);
                self.pos += 1;
                true
            }
            _ => false,
        }
    }

    pub fn delimited_python(&mut self, delim: String, _expr: bool) -> Option<String> {
        let start = self.pos;

        let chars = self.text.chars().collect::<Vec<_>>();
        while !self.eol() {
            let c = chars[self.pos];

            if delim.contains(c) {
                return Some(self.text[start..self.pos].to_string());
            }

            if ['\'', '"'].contains(&c) {
                self.python_string();
                continue;
            }

            if self.parenthesised_python() {
                continue;
            }

            self.pos += 1;
        }

        panic!("reached end of line when expecting '{delim}'");
    }

    pub fn float(&mut self) -> Option<String> {
        // println!("float");
        self.rmatch(RegexType::GlobalRegex(GlobalRegex::Float))
    }

    pub fn simple_expression(&mut self, comma: bool, operator: bool) -> Option<String> {
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

            if !(self.python_string()
                || self.name().is_some()
                || self.float().is_some()
                || self.parenthesised_python())
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

                if self.rmatch(RegexType::Simple(".".into())).is_some() {
                    let n = self.word();
                    if n.is_none() {
                        panic!("expecting name after dot.");
                    }
                    continue;
                }

                if self.parenthesised_python() {
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

            if comma && self.rmatch(RegexType::Simple(",".into())).is_some() {
                continue;
            }

            break;
        }
        // println!("start: {} | pos: {}", start, self.pos);

        let text = self.text[start..self.pos].trim().to_string();

        // println!("text: {:?}", text);

        if text.len() == 0 {
            return None;
        }

        Some(text.into())
    }

    pub fn checkpoint(&mut self) -> LexerState {
        LexerState {
            line: self.line,
            filename: self.filename.clone(),
            number: self.number,
            text: self.text.clone(),
            subblock: self.subblock.clone(),
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

        if rv.is_some() && self.keywords.contains(rv.as_ref().unwrap()) {
            self.pos = oldpos;
            return None;
        }

        rv
    }

    pub fn revert(&mut self, state: LexerState) {
        self.line = state.line;
        self.filename = state.filename;
        self.number = state.number;
        self.text = state.text;
        self.subblock = state.subblock;
        self.pos = state.pos;

        self.word_cache_pos = None;

        if self.line < Some(self.block.len()) {
            self.eob = false;
        } else {
            self.eob = true;
        }
    }

    pub fn expect_block(&mut self) {
        if self.subblock.len() == 0 {
            panic!("expected a non-empty block.");
        }
    }

    pub fn expect_noblock(&mut self) {
        if self.subblock.len() > 0 {
            let mut ll = self.subblock_lexer(false);
            ll.advance();
            panic!("Line is indented, but the preceding statement does not expect a block. Please check this line's indentation. You may have forgotten a colon (:).");
        }
    }

    pub fn say_expression(&mut self) -> Option<String> {
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

    pub fn python_expression(&mut self) -> Option<String> {
        let pe = self.delimited_python(":".into(), false);

        match pe {
            Some(s) => Some(s.trim().into()),
            None => panic!("expected python_expression"),
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

    pub fn dotted_name(&mut self) -> Option<String> {
        let mut rv = self.name();

        if rv.is_none() {
            return None;
        }

        while self.rmatch(RegexType::Simple(".".into())).is_some() {
            let n = self.name();
            if n.is_none() {
                panic!("expecting name.");
            }
            rv = Some(format!("{}.{}", rv.unwrap(), n.unwrap()));
        }

        rv
    }

    pub fn python_block(&mut self) -> Option<String> {
        let mut rv = vec![];

        let mut line = self.number;

        process(&mut rv, &mut line, self.subblock.clone(), "".into());

        if rv.len() == 0 {
            return None;
        }

        Some(rv.join(""))
    }
}

fn process(rv: &mut Vec<String>, line: &mut usize, blocks: Vec<Block>, indent: String) {
    for b in blocks {
        let ln = b.number;
        let text = b.text;
        let subblock = b.block;

        while *line < ln {
            rv.push(format!("{indent}\n"));
            *line += 1;
        }

        let linetext = format!("{indent}{text}\n");

        rv.push(linetext.clone());
        *line += linetext.matches("\n").count();

        process(rv, line, subblock, format!("{indent}    "));
    }
}
