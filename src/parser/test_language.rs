use super::ParseNodes;
use crate::{
    ast::{AstNode, Testcase, Testsuite},
    error::Result,
    lexer::{Lexer, LexerType, LexerTypeOptions, RegexType},
    testast::{
        self, TestAssert, TestCase, TestClick, TestCondition, TestDisplayableSelector, TestDrag,
        TestExit, TestHook, TestHookKind, TestHookProperties, TestIf, TestIfBranch, TestKeysym,
        TestMove, TestNode, TestParameter, TestPass, TestPause, TestProperties, TestPython,
        TestRepeat, TestRun, TestScreenshot, TestScroll, TestSelector, TestSkip, TestSuite,
        TestSuiteEntry, TestTarget, TestTextSelector, TestType, TestUntil, TestWhile,
    },
};
use std::{collections::HashMap, path::PathBuf};

type TestParserFn = fn(&mut Lexer, testast::Loc) -> Result<TestNode>;

struct TestParseTrie {
    default: Option<TestParserFn>,
    words: HashMap<&'static str, TestParseTrie>,
}

impl TestParseTrie {
    fn new() -> Self {
        Self {
            default: None,
            words: HashMap::new(),
        }
    }

    fn add(&mut self, name: &[&'static str], parser: TestParserFn) {
        if let Some((first, rest)) = name.split_first() {
            self.words
                .entry(first)
                .or_insert_with(TestParseTrie::new)
                .add(rest, parser);
        } else {
            self.default = Some(parser);
        }
    }

    fn parse(&self, lex: &mut Lexer) -> Result<TestNode> {
        let loc = lex.get_location();
        let old_pos = lex.pos;

        let word = match lex.word() {
            Some(word) => Some(word),
            None => lex.rmatch(RegexType::Simple("$")),
        };

        let Some(word) = word else {
            lex.pos = old_pos;
            return match self.default {
                Some(parser) => parser(lex, loc),
                None => Err(lex.parse_error("Expected statement.")),
            };
        };

        if let Some(next) = self.words.get(word.as_str()) {
            return next.parse(lex);
        }

        lex.pos = old_pos;
        match self.default {
            Some(parser) => parser(lex, loc),
            None => Err(lex.parse_error("Expected statement.")),
        }
    }
}

fn test_statement_trie() -> TestParseTrie {
    let mut trie = TestParseTrie::new();
    trie.add(&["exit"], parse_exit_statement);
    trie.add(&["if"], parse_if_statement);
    trie.add(&["pass"], parse_pass_statement);
    trie.add(&["while"], parse_while_statement);
    trie.add(&["advance"], parse_advance_statement);
    trie.add(&["click"], parse_click_statement);
    trie.add(&["drag"], parse_drag_statement);
    trie.add(&["keysym"], parse_keysym_statement);
    trie.add(&["move"], parse_move_statement);
    trie.add(&["pause"], parse_pause_statement);
    trie.add(&["run"], parse_run_statement);
    trie.add(&["scroll"], parse_scroll_statement);
    trie.add(&["skip"], parse_skip_statement);
    trie.add(&["type"], parse_type_statement);
    trie.add(&["assert"], parse_assert_statement);
    trie.add(&["screenshot"], parse_screenshot_statement);
    trie.add(&["python"], parse_python_statement);
    trie.add(&["$"], parse_one_line_python_statement);
    trie
}

pub(super) fn parse_testcase_statement(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
) -> Result<ParseNodes> {
    let name = lex.require_or_error(
        LexerType::Type(LexerTypeOptions::DottedName),
        "expected dotted name",
    )?;
    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;
    lex.expect_block()?;

    let (statements, properties) = parse_test_block(&mut lex.subblock_lexer(false))?;
    lex.advance();

    Ok(AstNode::Testcase(Testcase {
        loc: loc.clone(),
        test: TestCase {
            loc,
            name,
            properties,
            statements,
        },
    })
    .into())
}

pub(super) fn parse_testsuite_statement(
    lex: &mut Lexer,
    loc: (PathBuf, usize),
) -> Result<ParseNodes> {
    let name = lex.require_or_error(
        LexerType::Type(LexerTypeOptions::DottedName),
        "expected dotted name",
    )?;
    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;
    lex.expect_block()?;

    let (entries, properties) = parse_testsuite_entries(&mut lex.subblock_lexer(false))?;
    lex.advance();

    Ok(AstNode::Testsuite(Testsuite {
        loc: loc.clone(),
        suite: TestSuite {
            loc,
            name,
            properties,
            entries,
        },
    })
    .into())
}

fn parse_testsuite_entries(lex: &mut Lexer) -> Result<(Vec<TestSuiteEntry>, TestProperties)> {
    lex.advance();

    let mut entries = vec![];
    let mut properties = TestProperties::default();
    let mut body_started = false;
    let mut seen_hooks = HashMap::<TestHookKind, usize>::new();

    while !lex.eob {
        if try_parse_test_property(lex, &mut properties, body_started)? {
            continue;
        }

        body_started = true;
        let line_loc = lex.get_location();

        if lex.keyword("setup".into()).is_some() {
            let hook = parse_hook(lex, line_loc, TestHookKind::Setup)?;
            reject_duplicate_hook(lex, &mut seen_hooks, hook.kind)?;
            entries.push(TestSuiteEntry::Hook(hook));
        } else if lex.keyword("before".into()).is_some() {
            let kind = if lex.keyword("testsuite".into()).is_some() {
                TestHookKind::BeforeTestsuite
            } else if lex.keyword("testcase".into()).is_some() {
                TestHookKind::BeforeTestcase
            } else {
                return Err(lex.parse_error("Expected 'before testsuite' or 'before testcase'."));
            };
            let hook = parse_hook(lex, line_loc, kind)?;
            reject_duplicate_hook(lex, &mut seen_hooks, hook.kind)?;
            entries.push(TestSuiteEntry::Hook(hook));
        } else if lex.keyword("after".into()).is_some() {
            let kind = if lex.keyword("testcase".into()).is_some() {
                TestHookKind::AfterTestcase
            } else if lex.keyword("testsuite".into()).is_some() {
                TestHookKind::AfterTestsuite
            } else {
                return Err(lex.parse_error("Expected 'after testsuite' or 'after testcase'."));
            };
            let hook = parse_hook(lex, line_loc, kind)?;
            reject_duplicate_hook(lex, &mut seen_hooks, hook.kind)?;
            entries.push(TestSuiteEntry::Hook(hook));
        } else if lex.keyword("teardown".into()).is_some() {
            let hook = parse_hook(lex, line_loc, TestHookKind::Teardown)?;
            reject_duplicate_hook(lex, &mut seen_hooks, hook.kind)?;
            entries.push(TestSuiteEntry::Hook(hook));
        } else if lex.keyword("testcase".into()).is_some() {
            let parsed = parse_testcase_statement(lex, line_loc)?.into_vec();
            let AstNode::Testcase(node) = parsed.into_iter().next().expect("single testcase node")
            else {
                unreachable!();
            };
            entries.push(TestSuiteEntry::TestCase(node.test));
        } else if lex.keyword("testsuite".into()).is_some() {
            let parsed = parse_testsuite_statement(lex, line_loc)?.into_vec();
            let AstNode::Testsuite(node) =
                parsed.into_iter().next().expect("single testsuite node")
            else {
                unreachable!();
            };
            entries.push(TestSuiteEntry::TestSuite(node.suite));
        } else {
            return Err(lex.parse_error("Unexpected statement in testsuite."));
        }
    }

    Ok((entries, properties))
}

fn reject_duplicate_hook(
    lex: &Lexer,
    seen_hooks: &mut HashMap<TestHookKind, usize>,
    kind: TestHookKind,
) -> Result<()> {
    if seen_hooks.insert(kind, 1).is_some() {
        return Err(lex.parse_error(format!(
            "Only one '{}' block is allowed in a testsuite.",
            kind.as_str()
        )));
    }
    Ok(())
}

fn parse_hook(lex: &mut Lexer, loc: testast::Loc, kind: TestHookKind) -> Result<TestHook> {
    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;
    lex.expect_block()?;

    let (statements, mut properties) = parse_hook_block(&mut lex.subblock_lexer(false))?;
    lex.advance();

    if properties.depth.is_none() {
        properties.depth = match kind {
            TestHookKind::BeforeTestcase | TestHookKind::AfterTestcase => Some("-1".into()),
            TestHookKind::BeforeTestsuite | TestHookKind::AfterTestsuite => Some("0".into()),
            _ => None,
        };
    }

    Ok(TestHook {
        loc,
        kind,
        properties,
        statements,
    })
}

fn parse_test_block(lex: &mut Lexer) -> Result<(Vec<TestNode>, TestProperties)> {
    lex.advance();

    let mut statements = vec![];
    let mut properties = TestProperties::default();
    let mut statements_started = false;

    while !lex.eob {
        if try_parse_test_property(lex, &mut properties, statements_started)? {
            continue;
        }

        statements_started = true;
        if starts_nested_test_container(lex) {
            return Err(lex.parse_error(
                "A testsuite or testcase may not be nested inside a block. It must be at the top level, or within a testsuite.",
            ));
        }

        let stmt = parse_test_statement(lex)?;
        statements.push(stmt);
    }

    Ok((statements, properties))
}

fn parse_hook_block(lex: &mut Lexer) -> Result<(Vec<TestNode>, TestHookProperties)> {
    lex.advance();

    let mut statements = vec![];
    let mut properties = TestHookProperties::default();
    let mut statements_started = false;

    while !lex.eob {
        if try_parse_hook_property(lex, &mut properties, statements_started)? {
            continue;
        }

        statements_started = true;
        if starts_nested_test_container(lex) {
            return Err(lex.parse_error(
                "A testsuite or testcase may not be nested inside a block. It must be at the top level, or within a testsuite.",
            ));
        }

        let stmt = parse_test_statement(lex)?;
        statements.push(stmt);
    }

    Ok((statements, properties))
}

fn starts_nested_test_container(lex: &mut Lexer) -> bool {
    let state = lex.checkpoint();
    let result = matches!(lex.word().as_deref(), Some("testcase" | "testsuite"));
    lex.revert(state);
    result
}

fn try_parse_test_property(
    lex: &mut Lexer,
    properties: &mut TestProperties,
    statements_started: bool,
) -> Result<bool> {
    let state = lex.checkpoint();
    let Some(keyword) = lex.word() else {
        return Ok(false);
    };

    match keyword.as_str() {
        "description" | "enabled" | "only" | "xfail" => {
            if statements_started {
                return Err(lex.parse_error(format!(
                    "Property {keyword} must be defined before any test statements."
                )));
            }
            let expr = lex
                .simple_expression(false, true)?
                .ok_or_else(|| lex.parse_error(format!("expected expression for {keyword}")))?;
            lex.expect_eol()?;
            lex.expect_noblock()?;
            lex.advance();

            match keyword.as_str() {
                "description" => properties.description = Some(expr),
                "enabled" => properties.enabled = Some(expr),
                "only" => properties.only = Some(expr),
                "xfail" => properties.xfail = Some(expr),
                _ => unreachable!(),
            }
            Ok(true)
        }
        "parameter" => {
            if statements_started {
                return Err(lex.parse_error(
                    "Property parameter must be defined before any test statements.",
                ));
            }
            let parameter_loc = lex.get_location();
            let parameter = parse_parameter(lex, parameter_loc)?;
            properties.parameters.push(parameter);
            Ok(true)
        }
        _ => {
            lex.revert(state);
            Ok(false)
        }
    }
}

fn try_parse_hook_property(
    lex: &mut Lexer,
    properties: &mut TestHookProperties,
    statements_started: bool,
) -> Result<bool> {
    let state = lex.checkpoint();
    let Some(keyword) = lex.word() else {
        return Ok(false);
    };

    match keyword.as_str() {
        "xfail" | "depth" => {
            if statements_started {
                return Err(lex.parse_error(format!(
                    "Property {keyword} must be defined before any test statements."
                )));
            }
            let expr = lex
                .simple_expression(false, true)?
                .ok_or_else(|| lex.parse_error(format!("expected expression for {keyword}")))?;
            lex.expect_eol()?;
            lex.expect_noblock()?;
            lex.advance();

            match keyword.as_str() {
                "xfail" => properties.xfail = Some(expr),
                "depth" => {
                    properties.depth = Some(expr);
                    properties.depth_explicit = true;
                }
                _ => unreachable!(),
            }
            Ok(true)
        }
        _ => {
            lex.revert(state);
            Ok(false)
        }
    }
}

fn parse_parameter(lex: &mut Lexer, loc: testast::Loc) -> Result<TestParameter> {
    lex.expect_noblock()?;

    let mut names = vec![];
    if lex.rmatch(RegexType::Simple("(")).is_some() {
        loop {
            if lex.rmatch(RegexType::Simple(")")).is_some() {
                break;
            }
            if lex.rmatch(RegexType::Simple(",")).is_some() {
                continue;
            }
            names.push(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Name),
                "expected parameter name",
            )?);
        }
    } else {
        names.push(lex.require_or_error(
            LexerType::Type(LexerTypeOptions::Name),
            "expected parameter name",
        )?);
    }

    if names.is_empty() {
        return Err(lex.parse_error("Expected at least one name in parameter statement."));
    }

    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != names.len() {
        return Err(lex.parse_error("Parameter names in a parameter statement must be unique."));
    }

    lex.require_or_error(LexerType::String("=".into()), "expected '='")?;
    let values_expr = lex
        .simple_expression(false, true)?
        .ok_or_else(|| lex.parse_error("expected parameter value expression"))?;
    lex.expect_eol()?;
    lex.advance();

    Ok(TestParameter {
        loc,
        names,
        values_expr,
    })
}

fn parse_test_statement(lex: &mut Lexer) -> Result<TestNode> {
    test_statement_trie().parse(lex)
}

fn parse_exit_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    lex.expect_eol()?;
    lex.advance();
    Ok(TestNode::Exit(TestExit { loc }))
}

fn parse_pass_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    lex.expect_eol()?;
    lex.advance();
    Ok(TestNode::Pass(TestPass { loc }))
}

fn parse_if_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    let condition = parse_condition(lex, loc.clone(), None)?;
    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;
    lex.expect_block()?;

    let (block, _) = parse_test_block(&mut lex.subblock_lexer(false))?;
    let mut branches = vec![TestIfBranch {
        loc: loc.clone(),
        condition,
        block,
    }];
    let mut else_block = None;

    lex.advance();

    while lex.keyword("elif".into()).is_some() {
        let branch_loc = lex.get_location();
        let condition = parse_condition(lex, branch_loc.clone(), None)?;
        lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        lex.expect_eol()?;
        lex.expect_block()?;

        let (block, _) = parse_test_block(&mut lex.subblock_lexer(false))?;
        branches.push(TestIfBranch {
            loc: branch_loc,
            condition,
            block,
        });
        lex.advance();
    }

    if lex.keyword("else".into()).is_some() {
        lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
        lex.expect_eol()?;
        lex.expect_block()?;
        let (block, _) = parse_test_block(&mut lex.subblock_lexer(false))?;
        else_block = Some(block);
        lex.advance();
    }

    Ok(TestNode::If(TestIf {
        loc,
        branches,
        else_block,
    }))
}

fn parse_while_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    let condition = parse_condition(lex, loc.clone(), None)?;
    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;
    lex.expect_block()?;
    let (block, _) = parse_test_block(&mut lex.subblock_lexer(false))?;
    lex.advance();
    Ok(TestNode::While(TestWhile {
        loc,
        condition,
        block,
    }))
}

fn parse_advance_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    let mut node = TestNode::Advance(testast::TestAdvance { loc: loc.clone() });
    if let Some(wrapped) = parse_until_or_repeat(lex, loc.clone(), node.clone())? {
        node = wrapped;
    }
    lex.expect_eol()?;
    lex.advance();
    Ok(node)
}

fn parse_click_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    let mut click = TestClick {
        loc: loc.clone(),
        selector: None,
        button: None,
        position: None,
        always: false,
    };

    loop {
        if lex.keyword("button".into()).is_some() {
            click.button = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Integer),
                "expected button number",
            )?);
        } else if lex.keyword("pos".into()).is_some() {
            click.position = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected position expression"))?,
            );
        } else if lex.keyword("always".into()).is_some() {
            click.always = true;
        } else if let Some(selector) = parse_selector(lex, loc.clone())? {
            click.selector = Some(selector);
        } else {
            break;
        }
    }

    let mut node = TestNode::Click(click);
    if let Some(wrapped) = parse_until_or_repeat(lex, loc.clone(), node.clone())? {
        node = wrapped;
    }
    lex.expect_eol()?;
    lex.advance();
    Ok(node)
}

fn parse_drag_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;

    let mut drag = TestDrag {
        loc: loc.clone(),
        button: None,
        steps: None,
        start: TestTarget::default(),
        end: TestTarget::default(),
    };

    loop {
        if lex.keyword("button".into()).is_some() {
            drag.button = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Integer),
                "expected button number",
            )?);
        } else if lex.keyword("steps".into()).is_some() {
            drag.steps = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Integer),
                "expected drag steps",
            )?);
        } else if lex.keyword("pos".into()).is_some() {
            drag.start.position = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected position expression"))?,
            );
        } else if let Some(selector) = parse_selector(lex, loc.clone())? {
            drag.start.selector = Some(selector);
        } else if lex.keyword("to".into()).is_some() {
            break;
        } else {
            return Err(lex.parse_error("Expected 'to' or drag start specification."));
        }
    }

    loop {
        if lex.keyword("button".into()).is_some() {
            drag.button = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Integer),
                "expected button number",
            )?);
        } else if lex.keyword("steps".into()).is_some() {
            drag.steps = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Integer),
                "expected drag steps",
            )?);
        } else if lex.keyword("pos".into()).is_some() {
            drag.end.position = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected position expression"))?,
            );
        } else if let Some(selector) = parse_selector(lex, loc.clone())? {
            drag.end.selector = Some(selector);
        } else {
            break;
        }
    }

    lex.expect_eol()?;
    lex.advance();
    Ok(TestNode::Drag(drag))
}

fn parse_keysym_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    let mut keysym = TestKeysym {
        loc: loc.clone(),
        keysym: lex
            .string()
            .ok_or_else(|| lex.parse_error("expected string literal"))?,
        selector: None,
        position: None,
    };

    loop {
        if lex.keyword("pos".into()).is_some() {
            keysym.position = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected position expression"))?,
            );
        } else if let Some(selector) = parse_selector(lex, loc.clone())? {
            keysym.selector = Some(selector);
        } else {
            break;
        }
    }

    let mut node = TestNode::Keysym(keysym);
    if let Some(wrapped) = parse_until_or_repeat(lex, loc.clone(), node.clone())? {
        node = wrapped;
    }
    lex.expect_eol()?;
    lex.advance();
    Ok(node)
}

fn parse_move_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    let mut move_stmt = TestMove {
        loc: loc.clone(),
        selector: None,
        position: None,
    };

    loop {
        if lex.keyword("pos".into()).is_some() {
            move_stmt.position = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected position expression"))?,
            );
        } else if let Some(selector) = parse_selector(lex, loc.clone())? {
            move_stmt.selector = Some(selector);
        } else {
            break;
        }
    }

    let mut node = TestNode::Move(move_stmt);
    if let Some(wrapped) = parse_until_or_repeat(lex, loc.clone(), node.clone())? {
        node = wrapped;
    }
    lex.expect_eol()?;
    lex.advance();
    Ok(node)
}

fn parse_pause_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;

    let node = if let Some(wrapped) = parse_until_or_repeat(
        lex,
        loc.clone(),
        TestNode::Pause(TestPause {
            loc: loc.clone(),
            delay: None,
        }),
    )? {
        wrapped
    } else {
        let delay = lex
            .simple_expression(false, true)?
            .ok_or_else(|| lex.parse_error("expected pause delay or until condition"))?;
        let mut base = TestNode::Pause(TestPause {
            loc: loc.clone(),
            delay: Some(delay),
        });
        if let Some(wrapped) = parse_until_or_repeat(lex, loc.clone(), base.clone())? {
            base = wrapped;
        }
        base
    };

    lex.expect_eol()?;
    lex.advance();
    Ok(node)
}

fn parse_run_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    let expr = lex
        .simple_expression(false, true)?
        .ok_or_else(|| lex.parse_error("expected action expression"))?;
    let mut node = TestNode::Run(TestRun {
        loc: loc.clone(),
        expr,
    });
    if let Some(wrapped) = parse_until_or_repeat(lex, loc.clone(), node.clone())? {
        node = wrapped;
    }
    lex.expect_eol()?;
    lex.advance();
    Ok(node)
}

fn parse_scroll_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    let mut scroll = TestScroll {
        loc: loc.clone(),
        selector: None,
        position: None,
        amount: None,
    };

    loop {
        if lex.keyword("amount".into()).is_some() {
            scroll.amount = Some(lex.require_or_error(
                LexerType::Type(LexerTypeOptions::Integer),
                "expected scroll amount",
            )?);
        } else if lex.keyword("pos".into()).is_some() {
            scroll.position = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected position expression"))?,
            );
        } else if let Some(selector) = parse_selector(lex, loc.clone())? {
            scroll.selector = Some(selector);
        } else {
            break;
        }
    }

    let mut node = TestNode::Scroll(scroll);
    if let Some(wrapped) = parse_until_or_repeat(lex, loc.clone(), node.clone())? {
        node = wrapped;
    }
    lex.expect_eol()?;
    lex.advance();
    Ok(node)
}

fn parse_skip_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    let mut node = TestNode::Skip(TestSkip {
        loc: loc.clone(),
        fast: lex.keyword("fast".into()).is_some(),
    });
    if let Some(wrapped) = parse_until_or_repeat(lex, loc.clone(), node.clone())? {
        node = wrapped;
    }
    lex.expect_eol()?;
    lex.advance();
    Ok(node)
}

fn parse_type_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    let mut type_stmt = TestType {
        loc: loc.clone(),
        text: lex
            .string()
            .ok_or_else(|| lex.parse_error("expected string literal"))?,
        selector: None,
        position: None,
    };

    loop {
        if lex.keyword("pos".into()).is_some() {
            type_stmt.position = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected position expression"))?,
            );
        } else if let Some(selector) = parse_selector(lex, loc.clone())? {
            type_stmt.selector = Some(selector);
        } else {
            break;
        }
    }

    let mut node = TestNode::Type(type_stmt);
    if let Some(wrapped) = parse_until_or_repeat(lex, loc.clone(), node.clone())? {
        node = wrapped;
    }
    lex.expect_eol()?;
    lex.advance();
    Ok(node)
}

fn parse_assert_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    let condition = parse_condition(lex, loc.clone(), None)?;
    let mut timeout = None;
    let mut xfail = None;

    loop {
        if lex.keyword("timeout".into()).is_some() {
            timeout = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected timeout expression"))?,
            );
        } else if lex.keyword("xfail".into()).is_some() {
            xfail = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected xfail expression"))?,
            );
        } else {
            break;
        }
    }

    lex.expect_noblock()?;
    lex.expect_eol()?;
    lex.advance();

    Ok(TestNode::Assert(TestAssert {
        loc,
        condition,
        timeout,
        xfail,
    }))
}

fn parse_screenshot_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    lex.expect_noblock()?;
    let name = lex
        .simple_expression(false, true)?
        .ok_or_else(|| lex.parse_error("expected screenshot expression"))?;
    let mut screenshot = TestScreenshot {
        loc,
        name,
        max_pixel_difference: None,
        crop: None,
    };

    loop {
        if lex.keyword("max_pixel_difference".into()).is_some() {
            screenshot.max_pixel_difference = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected max_pixel_difference expression"))?,
            );
        } else if lex.keyword("crop".into()).is_some() {
            screenshot.crop = Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected crop expression"))?,
            );
        } else {
            break;
        }
    }

    lex.expect_eol()?;
    lex.advance();
    Ok(TestNode::Screenshot(screenshot))
}

fn parse_python_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    let hide = lex.keyword("hide".into()).is_some();
    lex.require_or_error(LexerType::String(":".into()), "expected ':'")?;
    lex.expect_eol()?;
    lex.expect_block()?;
    let code = lex
        .python_block()
        .ok_or_else(|| lex.parse_error("expected python block"))?;
    lex.advance();
    Ok(TestNode::Python(TestPython {
        loc,
        code,
        hide,
        block: true,
    }))
}

fn parse_one_line_python_statement(lex: &mut Lexer, loc: testast::Loc) -> Result<TestNode> {
    let code = lex
        .rest_statement()
        .map(|code| code.trim().to_string())
        .filter(|code| !code.is_empty())
        .ok_or_else(|| lex.parse_error("expected python code"))?;
    lex.expect_noblock()?;
    lex.expect_eol()?;
    lex.advance();
    Ok(TestNode::Python(TestPython {
        loc,
        code,
        hide: false,
        block: false,
    }))
}

fn parse_selector(lex: &mut Lexer, loc: testast::Loc) -> Result<Option<TestSelector>> {
    let checkpoint = lex.checkpoint();
    let mut pattern = None;
    let mut screen = None;
    let mut id = None;
    let mut layer = None;
    let mut focused = false;
    let mut raw = false;
    let mut expression = false;

    loop {
        if lex.keyword("screen".into()).is_some() {
            screen = Some(
                lex.simple_expression(false, false)?
                    .ok_or_else(|| lex.parse_error("expected screen expression"))?,
            );
        } else if lex.keyword("id".into()).is_some() {
            id = Some(
                lex.simple_expression(false, false)?
                    .ok_or_else(|| lex.parse_error("expected id expression"))?,
            );
        } else if lex.keyword("layer".into()).is_some() {
            layer = Some(
                lex.simple_expression(false, false)?
                    .ok_or_else(|| lex.parse_error("expected layer expression"))?,
            );
        } else if lex.keyword("focused".into()).is_some() {
            focused = true;
        } else if lex.keyword("expression".into()).is_some() {
            expression = true;
            if pattern.is_some() {
                return Err(
                    lex.parse_error("Only one text pattern may be specified in a selector.")
                );
            }
            pattern = Some(
                lex.simple_expression(false, false)?
                    .ok_or_else(|| lex.parse_error("expected selector expression"))?,
            );
        } else if lex.keyword("raw".into()).is_some() {
            raw = true;
        } else if let Some(text) = lex.string() {
            if pattern.is_some() {
                return Err(
                    lex.parse_error("Only one text pattern may be specified in a selector.")
                );
            }
            pattern = Some(text);
        } else {
            break;
        }
    }

    if pattern.is_none() && screen.is_none() && id.is_none() {
        lex.revert(checkpoint);
        return Ok(None);
    }

    if pattern.is_some() && (screen.is_some() || id.is_some()) {
        return Err(lex.parse_error("A text pattern may not be specified with a screen or id."));
    }

    if let Some(pattern) = pattern {
        return Ok(Some(TestSelector::Text(TestTextSelector {
            loc,
            focused,
            pattern,
            raw,
            expression,
        })));
    }

    Ok(Some(TestSelector::Displayable(TestDisplayableSelector {
        loc,
        screen,
        id,
        layer,
        focused,
    })))
}

fn parse_condition(
    lex: &mut Lexer,
    loc: testast::Loc,
    left: Option<TestCondition>,
) -> Result<TestCondition> {
    if lex.keyword("not".into()).is_some() {
        let right = parse_condition(lex, loc.clone(), None)?;
        Ok(TestCondition::Not {
            loc,
            right: Box::new(right),
        })
    } else if lex.keyword("and".into()).is_some() {
        let left = left
            .ok_or_else(|| lex.parse_error("Expected a left-hand side for \"and\" condition."))?;
        let right = parse_condition(lex, loc.clone(), None)?;
        Ok(TestCondition::And {
            loc,
            left: Box::new(left),
            right: Box::new(right),
        })
    } else if lex.keyword("or".into()).is_some() {
        let left =
            left.ok_or_else(|| lex.parse_error("Expected a left-hand side for \"or\" condition."))?;
        let right = parse_condition(lex, loc.clone(), None)?;
        Ok(TestCondition::Or {
            loc,
            left: Box::new(left),
            right: Box::new(right),
        })
    } else {
        let mut rv = if lex.rmatch(RegexType::Simple("(")).is_some() {
            let inner = parse_condition(lex, loc.clone(), None)?;
            lex.require_or_error(LexerType::String(r"\)".into()), "expected ')'")?;
            TestCondition::Grouped {
                loc: loc.clone(),
                inner: Box::new(inner),
            }
        } else if lex.keyword("True".into()).is_some() {
            TestCondition::BoolLiteral {
                loc: loc.clone(),
                value: true,
            }
        } else if lex.keyword("False".into()).is_some() {
            TestCondition::BoolLiteral {
                loc: loc.clone(),
                value: false,
            }
        } else if lex.keyword("eval".into()).is_some() {
            TestCondition::Eval {
                loc: loc.clone(),
                expr: lex
                    .simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected eval expression"))?,
            }
        } else if lex.keyword("label".into()).is_some() {
            TestCondition::Label {
                loc: loc.clone(),
                name: lex.require_or_error(
                    LexerType::Type(LexerTypeOptions::LabelName),
                    "expected label name",
                )?,
            }
        } else if let Some(selector) = parse_selector(lex, loc.clone())? {
            TestCondition::Selector(selector)
        } else {
            return Err(lex.parse_error("Invalid condition."));
        };

        let old_pos = lex.pos;
        if lex.keyword("and".into()).is_some() || lex.keyword("or".into()).is_some() {
            lex.pos = old_pos;
            rv = parse_condition(lex, loc, Some(rv))?;
        }

        Ok(rv)
    }
}

fn parse_until_or_repeat(
    lex: &mut Lexer,
    loc: testast::Loc,
    command: TestNode,
) -> Result<Option<TestNode>> {
    if lex.keyword("until".into()).is_some() {
        let condition = parse_condition(lex, loc.clone(), None)?;
        let timeout = if lex.keyword("timeout".into()).is_some() {
            Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected timeout expression"))?,
            )
        } else {
            None
        };

        Ok(Some(TestNode::Until(TestUntil {
            loc,
            command: Box::new(command),
            condition,
            timeout,
        })))
    } else if lex.keyword("repeat".into()).is_some() {
        let count = lex
            .simple_expression(false, true)?
            .ok_or_else(|| lex.parse_error("expected repeat count"))?;
        let timeout = if lex.keyword("timeout".into()).is_some() {
            Some(
                lex.simple_expression(false, true)?
                    .ok_or_else(|| lex.parse_error("expected timeout expression"))?,
            )
        } else {
            None
        };

        Ok(Some(TestNode::Repeat(TestRepeat {
            loc,
            command: Box::new(command),
            count,
            timeout,
        })))
    } else {
        Ok(None)
    }
}
