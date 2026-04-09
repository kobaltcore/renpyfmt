use crate::{atl::RawBlock, lexer::Block, slast};
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
    pub store: String,
    pub hide: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EarlyPython {
    pub loc: (PathBuf, usize),
    pub python_code: String,
    pub store: String,
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
    pub items: Vec<(Option<String>, Option<String>, Option<Vec<AstNode>>)>,
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
    pub entries: Vec<(Option<String>, Vec<AstNode>)>,
}

#[derive(Debug, Clone, Default)]
pub struct While {
    pub loc: (PathBuf, usize),
    pub condition: String,
    pub block: Vec<AstNode>,
}

#[derive(Debug, Clone, Default)]
pub struct CompileIf {
    pub loc: (PathBuf, usize),
    pub entries: Vec<(Option<String>, Vec<AstNode>)>,
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

#[derive(Debug, Clone, Default)]
pub struct Transform {
    pub loc: (PathBuf, usize),
    pub store: String,
    pub name: String,
    pub atl: Option<RawBlock>,
    pub parameters: Option<ParameterSignature>,
}

#[derive(Debug, Clone, Default)]
pub struct ShowLayer {
    pub loc: (PathBuf, usize),
    pub layer: String,
    pub at_list: Vec<String>,
    pub atl: Option<RawBlock>,
}

#[derive(Debug, Clone, Default)]
pub struct Camera {
    pub loc: (PathBuf, usize),
    pub layer: String,
    pub at_list: Vec<String>,
    pub atl: Option<RawBlock>,
}

#[derive(Debug, Clone, Default)]
pub struct Screen {
    pub loc: (PathBuf, usize),
    pub screen: slast::Screen,
}

#[derive(Debug, Clone, Default)]
pub struct Image {
    pub loc: (PathBuf, usize),
    pub name: Vec<String>,
    pub expr: Option<String>,
    pub atl: Option<RawBlock>,
}

#[derive(Debug, Clone, Default)]
pub struct RPY {
    pub loc: (PathBuf, usize),
    pub rest: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Translate {
    pub loc: (PathBuf, usize),
    pub identifier: String,
    pub language: Option<String>,
    pub block: Vec<AstNode>,
}

#[derive(Debug, Clone, Default)]
pub struct EndTranslate {
    pub loc: (PathBuf, usize),
}

#[derive(Debug, Clone, Default)]
pub struct TranslateString {
    pub loc: (PathBuf, usize),
    pub language: Option<String>,
    pub old: String,
    pub new: String,
    pub new_loc: (PathBuf, usize),
}

#[derive(Debug, Clone, Default)]
pub struct TranslateBlock {
    pub loc: (PathBuf, usize),
    pub language: Option<String>,
    pub block: Vec<AstNode>,
}

#[derive(Debug, Clone, Default)]
pub struct TranslateEarlyBlock {
    pub loc: (PathBuf, usize),
    pub language: Option<String>,
    pub block: Vec<AstNode>,
}

#[derive(Debug, Clone, Default)]
pub struct Testcase {
    pub loc: (PathBuf, usize),
    pub name: String,
    pub block: Vec<Block>,
}

#[derive(Debug, Clone, Default)]
pub struct Testsuite {
    pub loc: (PathBuf, usize),
    pub name: String,
    pub block: Vec<Block>,
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
    While(While),
    CompileIf(CompileIf),
    Return(Return),
    Style(Style),
    Init(Init),
    Python(Python),
    EarlyPython(EarlyPython),
    Define(Define),
    Default(Default_),
    Call(Call),
    Pass(Pass),
    Transform(Transform),
    ShowLayer(ShowLayer),
    Camera(Camera),
    Screen(Screen),
    Image(Image),
    RPY(RPY),
    Translate(Translate),
    EndTranslate(EndTranslate),
    TranslateString(TranslateString),
    TranslateBlock(TranslateBlock),
    TranslateEarlyBlock(TranslateEarlyBlock),
    Testcase(Testcase),
    Testsuite(Testsuite),
}

impl Default for AstNode {
    fn default() -> Self {
        AstNode::Say(Say::default())
    }
}

impl AstNode {
    pub fn line_number(&self) -> usize {
        match self {
            AstNode::Label(n) => n.loc.1,
            AstNode::Scene(n) => n.loc.1,
            AstNode::Show(n) => n.loc.1,
            AstNode::With(n) => n.loc.1,
            AstNode::Say(n) => n.loc.1,
            AstNode::UserStatement(n) => n.loc.1,
            AstNode::Hide(n) => n.loc.1,
            AstNode::PythonOneLine(n) => n.loc.1,
            AstNode::Jump(n) => n.loc.1,
            AstNode::Menu(n) => n.loc.1,
            AstNode::If(n) => n.loc.1,
            AstNode::While(n) => n.loc.1,
            AstNode::CompileIf(n) => n.loc.1,
            AstNode::Return(n) => n.loc.1,
            AstNode::Style(n) => n.loc.1,
            AstNode::Init(n) => n.loc.1,
            AstNode::Python(n) => n.loc.1,
            AstNode::EarlyPython(n) => n.loc.1,
            AstNode::Define(n) => n.loc.1,
            AstNode::Default(n) => n.loc.1,
            AstNode::Call(n) => n.loc.1,
            AstNode::Pass(n) => n.loc.1,
            AstNode::Transform(n) => n.loc.1,
            AstNode::ShowLayer(n) => n.loc.1,
            AstNode::Camera(n) => n.loc.1,
            AstNode::Screen(n) => n.loc.1,
            AstNode::Image(n) => n.loc.1,
            AstNode::RPY(n) => n.loc.1,
            AstNode::Translate(n) => n.loc.1,
            AstNode::EndTranslate(n) => n.loc.1,
            AstNode::TranslateString(n) => n.loc.1,
            AstNode::TranslateBlock(n) => n.loc.1,
            AstNode::TranslateEarlyBlock(n) => n.loc.1,
            AstNode::Testcase(n) => n.loc.1,
            AstNode::Testsuite(n) => n.loc.1,
        }
    }
}
