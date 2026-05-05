use std::path::PathBuf;

pub type Loc = (PathBuf, usize);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestProperties {
    pub description: Option<String>,
    pub enabled: Option<String>,
    pub only: Option<String>,
    pub xfail: Option<String>,
    pub parameters: Vec<TestParameter>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestParameter {
    pub loc: Loc,
    pub names: Vec<String>,
    pub values_expr: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestHookProperties {
    pub xfail: Option<String>,
    pub depth: Option<String>,
    pub depth_explicit: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestCase {
    pub loc: Loc,
    pub name: String,
    pub properties: TestProperties,
    pub statements: Vec<TestNode>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestSuite {
    pub loc: Loc,
    pub name: String,
    pub properties: TestProperties,
    pub entries: Vec<TestSuiteEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestSuiteEntry {
    Hook(TestHook),
    TestCase(TestCase),
    TestSuite(TestSuite),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestHook {
    pub loc: Loc,
    pub kind: TestHookKind,
    pub properties: TestHookProperties,
    pub statements: Vec<TestNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestHookKind {
    Setup,
    BeforeTestsuite,
    BeforeTestcase,
    AfterTestcase,
    AfterTestsuite,
    Teardown,
}

impl TestHookKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TestHookKind::Setup => "setup",
            TestHookKind::BeforeTestsuite => "before testsuite",
            TestHookKind::BeforeTestcase => "before testcase",
            TestHookKind::AfterTestcase => "after testcase",
            TestHookKind::AfterTestsuite => "after testsuite",
            TestHookKind::Teardown => "teardown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestNode {
    Exit(TestExit),
    Pass(TestPass),
    If(TestIf),
    While(TestWhile),
    Advance(TestAdvance),
    Click(TestClick),
    Drag(TestDrag),
    Keysym(TestKeysym),
    Move(TestMove),
    Pause(TestPause),
    Run(TestRun),
    Scroll(TestScroll),
    Skip(TestSkip),
    Type(TestType),
    Assert(TestAssert),
    Screenshot(TestScreenshot),
    Python(TestPython),
    Until(TestUntil),
    Repeat(TestRepeat),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestExit {
    pub loc: Loc,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestPass {
    pub loc: Loc,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestIf {
    pub loc: Loc,
    pub branches: Vec<TestIfBranch>,
    pub else_block: Option<Vec<TestNode>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestIfBranch {
    pub loc: Loc,
    pub condition: TestCondition,
    pub block: Vec<TestNode>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestWhile {
    pub loc: Loc,
    pub condition: TestCondition,
    pub block: Vec<TestNode>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestAdvance {
    pub loc: Loc,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestClick {
    pub loc: Loc,
    pub selector: Option<TestSelector>,
    pub button: Option<String>,
    pub position: Option<String>,
    pub always: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestDrag {
    pub loc: Loc,
    pub button: Option<String>,
    pub steps: Option<String>,
    pub start: TestTarget,
    pub end: TestTarget,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestTarget {
    pub selector: Option<TestSelector>,
    pub position: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestKeysym {
    pub loc: Loc,
    pub keysym: String,
    pub selector: Option<TestSelector>,
    pub position: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestMove {
    pub loc: Loc,
    pub selector: Option<TestSelector>,
    pub position: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestPause {
    pub loc: Loc,
    pub delay: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestRun {
    pub loc: Loc,
    pub expr: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestScroll {
    pub loc: Loc,
    pub selector: Option<TestSelector>,
    pub position: Option<String>,
    pub amount: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestSkip {
    pub loc: Loc,
    pub fast: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestType {
    pub loc: Loc,
    pub text: String,
    pub selector: Option<TestSelector>,
    pub position: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestAssert {
    pub loc: Loc,
    pub condition: TestCondition,
    pub timeout: Option<String>,
    pub xfail: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestScreenshot {
    pub loc: Loc,
    pub name: String,
    pub max_pixel_difference: Option<String>,
    pub crop: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestPython {
    pub loc: Loc,
    pub code: String,
    pub hide: bool,
    pub block: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestUntil {
    pub loc: Loc,
    pub command: Box<TestNode>,
    pub condition: TestCondition,
    pub timeout: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRepeat {
    pub loc: Loc,
    pub command: Box<TestNode>,
    pub count: String,
    pub timeout: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestCondition {
    BoolLiteral { loc: Loc, value: bool },
    Eval { loc: Loc, expr: String },
    Label { loc: Loc, name: String },
    Selector(TestSelector),
    Not { loc: Loc, right: Box<TestCondition> },
    And {
        loc: Loc,
        left: Box<TestCondition>,
        right: Box<TestCondition>,
    },
    Or {
        loc: Loc,
        left: Box<TestCondition>,
        right: Box<TestCondition>,
    },
    Grouped { loc: Loc, inner: Box<TestCondition> },
}

impl Default for TestCondition {
    fn default() -> Self {
        Self::BoolLiteral {
            loc: Default::default(),
            value: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestSelector {
    Text(TestTextSelector),
    Displayable(TestDisplayableSelector),
}

impl TestSelector {
    pub fn loc(&self) -> &Loc {
        match self {
            TestSelector::Text(selector) => &selector.loc,
            TestSelector::Displayable(selector) => &selector.loc,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestTextSelector {
    pub loc: Loc,
    pub focused: bool,
    pub pattern: String,
    pub raw: bool,
    pub expression: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestDisplayableSelector {
    pub loc: Loc,
    pub screen: Option<String>,
    pub id: Option<String>,
    pub layer: Option<String>,
    pub focused: bool,
}
