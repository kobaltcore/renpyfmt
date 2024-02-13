use crate::{atl::RawBlock, lexer::Block};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[derive(Debug, Clone, Default)]
pub struct ImageSpecifier {
    pub image_name: Vec<String>,
    pub expression: Option<String>,
    pub tag: Option<String>,
    pub at_list: Vec<String>,
    pub layer: Option<String>,
    pub zorder: Option<String>,
    pub behind: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub kind: ParameterKind,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ArgumentInfo {
    pub arguments: Vec<(Option<String>, Option<String>)>,
    pub starred_indexes: HashSet<usize>,
    pub doublestarred_indexes: HashSet<usize>,
}

#[derive(Debug, Clone)]
pub enum ParameterKind {
    PositionalOnly,
    PositionalOrKeyword,
    VarPositional,
    KeywordOnly,
    VarKeyword,
}

#[derive(Debug, Clone, Default)]
pub struct ParameterSignature {
    pub parameters: HashMap<String, Parameter>,
}

#[derive(Debug, Clone, Default)]
pub struct Label {
    pub loc: (PathBuf, usize),
    pub name: String,
    pub block: Vec<AstNode>,
    pub parameters: Option<ParameterSignature>,
    pub hide: bool,
    // parent property
    // TODO: this might need to become a reference later
    pub statement_start: Option<Box<AstNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub loc: (PathBuf, usize),
    pub imspec: Option<ImageSpecifier>,
    pub layer: Option<String>,
    pub atl: Option<RawBlock>,
}

#[derive(Debug, Clone, Default)]
pub struct Show {
    pub loc: (PathBuf, usize),
    pub imspec: Option<ImageSpecifier>,
    pub atl: Option<RawBlock>,
}

#[derive(Debug, Clone, Default)]
pub struct With {
    pub loc: (PathBuf, usize),
    pub expr: String,
    pub paired: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Say {
    pub loc: (PathBuf, usize),
    pub who: Option<String>,
    pub what: String,
    pub with: Option<String>,
    pub interact: bool,
    pub attributes: Option<Vec<String>>,
    pub arguments: Option<ArgumentInfo>,
    pub temporary_attributes: Option<Vec<String>>,
    pub identifier: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UserStatement {
    pub loc: (PathBuf, usize),
    pub line: String,
    pub block: Vec<Block>,
    pub parsed: bool,
    pub code_block: Option<Vec<AstNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct Hide {
    pub loc: (PathBuf, usize),
    pub imgspec: ImageSpecifier,
}

#[derive(Debug, Clone, Default)]
pub struct PythonOneLine {
    pub loc: (PathBuf, usize),
    pub python_code: String,
}

#[derive(Debug, Clone, Default)]
pub struct Python {
    pub loc: (PathBuf, usize),
    pub python_code: String,
    pub store: Option<String>,
    pub hide: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EarlyPython {
    pub loc: (PathBuf, usize),
    pub python_code: String,
    pub store: Option<String>,
    pub hide: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Jump {
    pub loc: (PathBuf, usize),
    pub target: String,
    pub expression: bool,
    pub global_label: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Menu {
    pub loc: (PathBuf, usize),
    pub items: Vec<(Option<String>, String, Option<Vec<AstNode>>)>,
    pub set: Option<String>,
    pub with_: Option<String>,
    pub has_caption: bool,
    pub arguments: Option<ArgumentInfo>,
    pub item_arguments: Vec<Option<ArgumentInfo>>,
    pub statement_start: Option<Box<AstNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct If {
    pub loc: (PathBuf, usize),
    pub entries: Vec<(String, Vec<AstNode>)>,
}

#[derive(Debug, Clone, Default)]
pub struct Return {
    pub loc: (PathBuf, usize),
    pub expression: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub loc: (PathBuf, usize),
    pub name: String,
    pub parent: Option<String>,
    pub clear: bool,
    pub take: Option<String>,
    pub delattr: Vec<String>,
    pub variant: Option<String>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct Init {
    pub loc: (PathBuf, usize),
    pub block: Vec<AstNode>,
    pub priority: isize,
}

#[derive(Debug, Clone, Default)]
pub struct Define {
    pub loc: (PathBuf, usize),
    pub store: String,
    pub name: String,
    pub index: Option<String>,
    pub operator: String,
    pub expr: String,
}

#[derive(Debug, Clone, Default)]
pub struct Default_ {
    pub loc: (PathBuf, usize),
    pub store: String,
    pub name: String,
    pub expr: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Call {
    pub loc: (PathBuf, usize),
    pub label: String,
    pub expression: bool,
    pub arguments: Option<ArgumentInfo>,
    pub global_label: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Pass {
    pub loc: (PathBuf, usize),
}

#[derive(Debug, Clone)]
pub enum AstNode {
    Label(Label),
    Scene(Scene),
    Show(Show),
    With(With),
    Say(Say),
    UserStatement(UserStatement),
    Hide(Hide),
    PythonOneLine(PythonOneLine),
    Jump(Jump),
    Menu(Menu),
    If(If),
    Return(Return),
    Style(Style),
    Init(Init),
    Python(Python),
    EarlyPython(EarlyPython),
    Define(Define),
    Default(Default_),
    Call(Call),
    Pass(Pass),
}

impl Default for AstNode {
    fn default() -> Self {
        AstNode::Say(Say::default())
    }
}
