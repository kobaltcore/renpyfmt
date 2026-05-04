use crate::{
    ast::{ArgumentInfo, ParameterSignature},
    atl::RawBlock,
};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct Screen {
    pub loc: (PathBuf, usize),
    pub name: String,
    pub parameters: Option<ParameterSignature>,
    pub properties: Vec<(String, String)>,
    pub docstring: Option<String>,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum Node {
    Displayable(Displayable),
    If(If),
    ShowIf(ShowIf),
    For(For),
    Python(Python),
    Default(Default_),
    Use(Use),
    Transclude(Transclude),
    Pass(Pass),
    Break(Break),
    Continue(Continue),
}

#[derive(Debug, Clone, Default)]
pub struct Displayable {
    pub loc: (PathBuf, usize),
    pub name: String,
    pub positional: Vec<String>,
    pub properties: Vec<(String, String)>,
    pub variable: Option<String>,
    pub atl_transform: Option<RawBlock>,
    pub children: Vec<Node>,
    pub layout_child: Option<Box<Node>>,
}

#[derive(Debug, Clone, Default)]
pub struct If {
    pub loc: (PathBuf, usize),
    pub entries: Vec<(Option<String>, Vec<Node>)>,
}

#[derive(Debug, Clone, Default)]
pub struct ShowIf {
    pub loc: (PathBuf, usize),
    pub entries: Vec<(Option<String>, Vec<Node>)>,
}

#[derive(Debug, Clone, Default)]
pub struct For {
    pub loc: (PathBuf, usize),
    pub target: String,
    pub index_expression: Option<String>,
    pub iterable: String,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum UseTarget {
    Name(String),
    Expression(String),
}

impl Default for UseTarget {
    fn default() -> Self {
        Self::Name(String::new())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Use {
    pub loc: (PathBuf, usize),
    pub target: UseTarget,
    pub arguments: Option<ArgumentInfo>,
    pub id_expr: Option<String>,
    pub variable: Option<String>,
    pub pass_context: bool,
    pub block: Option<Vec<Node>>,
}

#[derive(Debug, Clone, Default)]
pub struct Python {
    pub loc: (PathBuf, usize),
    pub source: String,
    pub block: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Default_ {
    pub loc: (PathBuf, usize),
    pub name: String,
    pub expr: String,
}

#[derive(Debug, Clone, Default)]
pub struct Transclude {
    pub loc: (PathBuf, usize),
}

#[derive(Debug, Clone, Default)]
pub struct Pass {
    pub loc: (PathBuf, usize),
}

#[derive(Debug, Clone, Default)]
pub struct Break {
    pub loc: (PathBuf, usize),
}

#[derive(Debug, Clone, Default)]
pub struct Continue {
    pub loc: (PathBuf, usize),
}
