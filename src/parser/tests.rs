use super::parse_block;
use crate::{
    ast::AstNode,
    error::{ParseError, Result},
    lexer::{Block, Lexer},
};
use std::path::PathBuf;

fn block(number: usize, text: &str, block: Vec<Block>) -> Block {
    Block {
        filename: PathBuf::from("test.rpy"),
        number,
        text: text.into(),
        block,
    }
}

fn parse(blocks: Vec<Block>) -> Result<Vec<crate::ast::AstNode>> {
    let mut lex = Lexer::new(blocks);
    parse_block(&mut lex)
}

fn assert_error(result: Result<Vec<crate::ast::AstNode>>, expected: &str, line: usize) {
    let err = result.expect_err("parse should fail");
    assert_eq!(
        err,
        ParseError::at((PathBuf::from("test.rpy"), line), expected),
    );
}

fn assert_parse(result: Result<Vec<AstNode>>) -> Vec<AstNode> {
    result.expect("parse should succeed")
}

#[test]
fn label_missing_colon_returns_parse_error() {
    assert_error(
        parse(vec![block(
            1,
            "label start",
            vec![block(2, "pass", vec![])],
        )]),
        "expected ':'",
        1,
    );
}

#[test]
fn duplicate_label_parameter_returns_parse_error() {
    assert_error(
        parse(vec![block(
            1,
            "label start(a, a):",
            vec![block(2, "pass", vec![])],
        )]),
        "duplicate parameter name: a",
        1,
    );
}

#[test]
fn unexpected_block_returns_parse_error() {
    assert_error(
        parse(vec![block(1, "\"hello\"", vec![block(2, "pass", vec![])])]),
        "Line is indented, but the preceding statement does not expect a block. Please check this line's indentation. You may have forgotten a colon (:).",
        1,
    );
}

#[test]
fn malformed_menu_choice_returns_parse_error() {
    assert_error(
        parse(vec![block(
            1,
            "menu:",
            vec![block(2, "\"Choice\"", vec![block(3, "pass", vec![])])],
        )]),
        "Line is followed by a block, despite not being a menu choice. Did you forget a colon at the end of the line?",
        1,
    );
}

#[test]
fn duplicate_onlayer_clause_returns_parse_error() {
    assert_error(
        parse(vec![block(
            1,
            "show eileen onlayer master onlayer front",
            vec![],
        )]),
        "multiple onlayer clauses are prohibited.",
        1,
    );
}

#[test]
fn duplicate_style_property_returns_parse_error() {
    assert_error(
        parse(vec![block(1, "style foo xalign 0.0 xalign 1.0", vec![])]),
        "style property xalign appears twice.",
        1,
    );
}

#[test]
fn duplicate_at_clause_returns_parse_error() {
    assert_error(
        parse(vec![block(1, "show eileen at left at right", vec![])]),
        "multiple at clauses are prohibited.",
        1,
    );
}

#[test]
fn empty_python_block_returns_parse_error() {
    assert_error(
        parse(vec![block(1, "python:", vec![])]),
        "expected a non-empty block.",
        1,
    );
}

#[test]
fn malformed_python_expression_returns_parse_error_instead_of_panicking() {
    assert_error(
        parse(vec![block(1, "show eileen at foo.", vec![])]),
        "expecting name after dot.",
        1,
    );
}

#[test]
fn unterminated_argument_expression_returns_parse_error_instead_of_panicking() {
    assert_error(
        parse(vec![block(1, "call expression foo(bar", vec![])]),
        "reached end of line when expecting ')'",
        1,
    );
}

#[test]
fn while_statement_parses() {
    let ast = assert_parse(parse(vec![block(
        1,
        "while flag:",
        vec![block(2, "pass", vec![])],
    )]));

    assert!(
        matches!(&ast[0], AstNode::While(node) if node.condition == "flag" && node.block.len() == 1)
    );
}

#[test]
fn show_layer_statement_parses() {
    let ast = assert_parse(parse(vec![block(
        1,
        "show layer master at left, right",
        vec![],
    )]));

    assert!(
        matches!(&ast[0], AstNode::ShowLayer(node) if node.layer == "master" && node.at_list == vec!["left", "right"])
    );
}

#[test]
fn camera_statement_defaults_to_master() {
    let ast = assert_parse(parse(vec![block(1, "camera", vec![])]));

    assert!(
        matches!(&ast[0], AstNode::Camera(node) if node.layer == "master" && node.at_list.is_empty())
    );
}

#[test]
fn init_offset_updates_following_init_priority() {
    let ast = assert_parse(parse(vec![
        block(1, "init offset = 5", vec![]),
        block(2, "define foo = 1", vec![]),
    ]));

    assert!(matches!(&ast[0], AstNode::Init(node) if node.priority == 5));
}

#[test]
fn init_label_uses_init_subblock() {
    let ast = assert_parse(parse(vec![block(
        1,
        "init label start:",
        vec![block(2, "define foo = 1", vec![])],
    )]));

    assert!(
        matches!(&ast[0], AstNode::Label(node) if node.name == "start" && matches!(&node.block[0], AstNode::Define(_)))
    );
}

#[test]
fn rpy_python_parses_multiple_names() {
    let ast = assert_parse(parse(vec![block(
        1,
        "rpy python __future__, annotations",
        vec![],
    )]));

    assert_eq!(ast.len(), 2);
    assert!(matches!(&ast[0], AstNode::RPY(node) if node.rest == vec!["python", "__future__"]));
    assert!(matches!(&ast[1], AstNode::RPY(node) if node.rest == vec!["python", "annotations"]));
}

#[test]
fn rpy_monologue_none_parses_without_nodes() {
    let ast = assert_parse(parse(vec![block(1, "rpy monologue none", vec![])]));

    assert!(ast.is_empty());
}

#[test]
fn compile_if_statement_parses_all_clauses() {
    let ast = assert_parse(parse(vec![
        block(1, "IF flag:", vec![block(2, "pass", vec![])]),
        block(3, "ELIF other:", vec![block(4, "pass", vec![])]),
        block(5, "ELSE:", vec![block(6, "pass", vec![])]),
    ]));

    assert!(matches!(&ast[0], AstNode::CompileIf(node) if node.entries.len() == 3));
}

#[test]
fn translate_block_parses() {
    let ast = assert_parse(parse(vec![block(
        1,
        "translate english start:",
        vec![block(2, "pass", vec![])],
    )]));

    assert!(
        matches!(&ast[0], AstNode::Translate(node) if node.language.as_deref() == Some("english") && node.identifier == "start")
    );
    assert!(matches!(&ast[1], AstNode::EndTranslate(_)));
}

#[test]
fn translate_strings_parses() {
    let ast = assert_parse(parse(vec![block(
        1,
        "translate english strings:",
        vec![
            block(2, "old \"Hello\"", vec![]),
            block(3, "new \"Hi\"", vec![]),
        ],
    )]));

    assert!(matches!(&ast[0], AstNode::Init(_)));
}

#[test]
fn translate_python_parses() {
    let ast = assert_parse(parse(vec![block(
        1,
        "translate english python:",
        vec![block(2, "pass", vec![])],
    )]));

    assert!(
        matches!(&ast[0], AstNode::TranslateEarlyBlock(node) if node.language.as_deref() == Some("english") && node.block.len() == 1)
    );
}

#[test]
fn testcase_parses_header_and_raw_block() {
    let ast = assert_parse(parse(vec![block(
        1,
        "testcase foo.bar:",
        vec![block(2, "assert x", vec![])],
    )]));

    assert!(
        matches!(&ast[0], AstNode::Testcase(node) if node.name == "foo.bar" && node.block.len() == 1)
    );
}

#[test]
fn testsuite_parses_header_and_raw_block() {
    let ast = assert_parse(parse(vec![block(
        1,
        "testsuite foo.bar:",
        vec![block(
            2,
            "testcase inner:",
            vec![block(3, "assert x", vec![])],
        )],
    )]));

    assert!(
        matches!(&ast[0], AstNode::Testsuite(node) if node.name == "foo.bar" && node.block.len() == 1)
    );
}
