use crate::{
    ast::AstNode,
    atl::AtlStatement,
    error::{ParseError, Result},
    test_support::{block, parse, parse_script},
};
use std::path::PathBuf;

fn assert_error(result: Result<Vec<crate::ast::AstNode>>, expected: &str, line: usize) {
    let err = result.expect_err("parse should fail");
    assert_eq!(
        err,
        ParseError::at((PathBuf::from("test.rpy"), line), expected),
    );
}

fn assert_error_contains(
    result: Result<Vec<crate::ast::AstNode>>,
    expected_substring: &str,
    line: usize,
) {
    let err = result.expect_err("parse should fail");
    assert_eq!(err.location, Some((PathBuf::from("test.rpy"), line)));
    assert!(
        err.message.contains(expected_substring),
        "expected error containing {:?}, got {:?}",
        expected_substring,
        err.message
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

#[test]
fn script_label_say_menu_and_flow_statements_parse() {
    let ast = assert_parse(parse_script(
        r#"
label start(name) hide:
    e happy @ blush "Hello [name]." with dissolve nointeract id greeting
    menu:
        e "What should we do?"
        "Take the left path" if courage > 0:
            jump left_path
        "Call for help":
            call expression helper_label(pass_count) from helper_return
    if seen_intro:
        return "done"
    elif retries < 3:
        pass
    else:
        while keep_waiting:
            pass
"#,
    ));

    assert!(matches!(&ast[0], AstNode::Label(node) if node.name == "start" && node.hide));

    let AstNode::Label(label) = &ast[0] else {
        panic!("expected label");
    };

    assert!(label.parameters.is_some());
    assert_eq!(label.block.len(), 4);

    assert!(matches!(
        &label.block[0],
        AstNode::Say(node)
            if node.who.as_deref() == Some("e")
                && node.what == "Hello [name]."
                && node.with.as_deref() == Some("dissolve")
                && !node.interact
                && node.attributes.as_ref().is_some_and(|attrs| attrs == &vec!["happy".to_string()])
                && node.temporary_attributes.as_ref().is_some_and(|attrs| attrs == &vec!["blush".to_string()])
                && node.identifier.as_deref() == Some("greeting")
    ));

    assert!(matches!(
        &label.block[1],
        AstNode::Say(node)
            if node.who.as_deref() == Some("e")
                && node.what == "What should we do?"
                && !node.interact
    ));

    let AstNode::Menu(menu) = &label.block[2] else {
        panic!("expected menu");
    };

    assert_eq!(menu.items.len(), 2);
    assert!(menu.has_caption);
    assert!(matches!(
        &menu.items[0],
        (Some(label), Some(condition), Some(block))
            if label == "Take the left path"
                && condition == "courage > 0"
                && matches!(&block[0], AstNode::Jump(node) if node.target == "left_path" && !node.expression)
    ));
    assert!(matches!(
        &menu.items[1],
        (Some(label), None, Some(block))
            if label == "Call for help"
                && matches!(&block[0], AstNode::Call(node) if node.label == "helper_label(pass_count)" && node.expression)
                && matches!(&block[1], AstNode::Label(node) if node.name == "helper_return")
    ));

    let AstNode::If(if_node) = &label.block[3] else {
        panic!("expected if");
    };

    assert_eq!(if_node.entries.len(), 3);
    assert!(
        matches!(&if_node.entries[0].1[0], AstNode::Return(node) if node.expression.as_deref() == Some("\"done\""))
    );
    assert!(matches!(&if_node.entries[1].1[0], AstNode::Pass(_)));
    assert!(
        matches!(&if_node.entries[2].1[0], AstNode::While(node) if node.condition == "keep_waiting")
    );
}

#[test]
fn script_media_statements_parse_with_modifiers_and_atl() {
    let ast = assert_parse(parse_script(
        r#"
label visuals:
    scene bg lecturehall onlayer master with fade
    show eileen happy at left, center onlayer screens zorder 3 behind desk as teacher with dissolve
    show paul a_0 with ease:
        ypos 1.15
    hide teacher onlayer screens
    camera at wobble:
        xalign 0.5
"#,
    ));

    let AstNode::Label(label) = &ast[0] else {
        panic!("expected label");
    };

    assert_eq!(label.block.len(), 11);
    assert!(
        matches!(&label.block[0], AstNode::With(node) if node.expr == "None" && node.paired.as_deref() == Some("fade"))
    );
    assert!(matches!(
        &label.block[1],
        AstNode::Scene(node)
            if node.layer.as_deref() == Some("master")
                && node.imspec.as_ref().is_some_and(|imspec| imspec.image_name == vec!["bg".to_string(), "lecturehall".to_string()])
    ));
    assert!(
        matches!(&label.block[2], AstNode::With(node) if node.expr == "fade" && node.paired.is_none())
    );
    assert!(
        matches!(&label.block[3], AstNode::With(node) if node.expr == "None" && node.paired.as_deref() == Some("dissolve"))
    );
    assert!(matches!(
        &label.block[4],
        AstNode::Show(node)
            if node.imspec.as_ref().is_some_and(|imspec| {
                imspec.image_name == vec!["eileen".to_string(), "happy".to_string()]
                    && imspec.tag.as_deref() == Some("teacher")
                    && imspec.layer.as_deref() == Some("screens")
                    && imspec.at_list == vec!["left".to_string(), "center".to_string()]
                    && imspec.zorder.as_deref() == Some("3")
                    && imspec.behind == vec!["desk".to_string()]
            })
    ));
    assert!(
        matches!(&label.block[5], AstNode::With(node) if node.expr == "dissolve" && node.paired.is_none())
    );
    assert!(
        matches!(&label.block[6], AstNode::With(node) if node.expr == "None" && node.paired.as_deref() == Some("ease"))
    );
    assert!(matches!(
        &label.block[7],
        AstNode::Show(node)
            if node.imspec.as_ref().is_some_and(|imspec| imspec.image_name == vec!["paul".to_string(), "a_0".to_string()])
                && node.atl.is_some()
    ));
    assert!(
        matches!(&label.block[8], AstNode::With(node) if node.expr == "ease" && node.paired.is_none())
    );
    assert!(matches!(
        &label.block[9],
        AstNode::Hide(node)
            if node.imgspec.image_name == vec!["teacher".to_string()] && node.imgspec.layer.as_deref() == Some("screens")
    ));
    assert!(
        matches!(&label.block[10], AstNode::Camera(node) if node.layer == "master" && node.at_list == vec!["wobble"] && node.atl.is_some())
    );
}

#[test]
fn script_init_statements_parse() {
    let ast = assert_parse(parse_script(
        r#"
python early hide in mystore:
    total = 1
python hide:
    value = 2
$ points += 1
define gui.colors["accent"] |= palette.choice()
default persistent.seen_intro = False
style menu_choice is default:
    xalign 0.5
transform wobble(x, y=2):
    xalign 0.5
image bg room = "room.webp"
"#,
    ));

    assert!(matches!(
        &ast[0],
        AstNode::EarlyPython(node)
            if node.hide && node.store == "store.mystore" && node.python_code.contains("total = 1")
    ));
    assert!(matches!(
        &ast[1],
        AstNode::Python(node)
            if node.hide && node.store == "store" && node.python_code.contains("value = 2")
    ));
    assert!(matches!(&ast[2], AstNode::PythonOneLine(node) if node.python_code == "points += 1"));
    assert!(matches!(
        &ast[3],
        AstNode::Init(node)
            if node.priority == 0
                && matches!(&node.block[0], AstNode::Define(define)
                    if define.store == "store.gui"
                        && define.name == "colors"
                        && define.index.as_deref() == Some("\"accent\"")
                        && define.operator == "|="
                        && define.expr == "palette.choice()")
    ));
    assert!(matches!(
        &ast[4],
        AstNode::Init(node)
            if matches!(&node.block[0], AstNode::Default(default)
                if default.store == "store.persistent" && default.name == "seen_intro" && default.expr.as_deref() == Some("False"))
    ));
    assert!(matches!(
        &ast[5],
        AstNode::Init(node)
            if matches!(&node.block[0], AstNode::Style(style)
                if style.name == "menu_choice"
                    && style.parent.as_deref() == Some("default")
                    && style.properties.get("xalign").map(String::as_str) == Some("0.5"))
    ));
    assert!(matches!(
        &ast[6],
        AstNode::Init(node)
            if matches!(&node.block[0], AstNode::Transform(transform)
                if transform.name == "wobble" && transform.parameters.is_some() && transform.atl.is_some())
    ));
    assert!(matches!(
        &ast[7],
        AstNode::Init(node)
            if node.priority == 500
                && matches!(&node.block[0], AstNode::Image(image)
                    if image.name == vec!["bg".to_string(), "room".to_string()] && image.expr.as_deref() == Some("\"room.webp\""))
    ));
}

#[test]
fn script_translate_variants_parse() {
    let ast = assert_parse(parse_script(
        r#"
translate None strings:
    old "Hello"
    new "Hi"
translate english style say_dialogue is default
translate french python:
    count = 3
"#,
    ));

    assert!(matches!(
        &ast[0],
        AstNode::Init(node)
            if matches!(&node.block[0], AstNode::TranslateString(string)
                if string.language.is_none() && string.old == "\"Hello\"" && string.new == "\"Hi\"")
    ));
    assert!(matches!(
        &ast[1],
        AstNode::TranslateBlock(node)
            if node.language.as_deref() == Some("english")
                && matches!(&node.block[0], AstNode::Style(style)
                    if style.name == "say_dialogue" && style.parent.as_deref() == Some("default"))
    ));
    assert!(matches!(
        &ast[2],
        AstNode::TranslateEarlyBlock(node)
            if node.language.as_deref() == Some("french")
                && matches!(&node.block[0], AstNode::Python(python) if python.python_code.contains("count = 3"))
    ));
}

#[test]
fn show_atl_block_parses_diverse_statement_forms() {
    let ast = assert_parse(parse_script(
        r#"
label atl_demo:
    show eileen:
        animation
        linear 1.0 xalign 0.5
        contains icon_idle
        contains:
            pass
        parallel:
            xalign 0.0
        choice 0.5:
            yalign 1.0
        on show, hide:
            pass
        time 1.0
        function callback
        event startled
        repeat
"#,
    ));

    let AstNode::Label(label) = &ast[0] else {
        panic!("expected label");
    };
    let AstNode::Show(show) = &label.block[0] else {
        panic!("expected show");
    };
    let atl = show.atl.as_ref().expect("expected ATL block");

    assert!(atl.animation);
    assert_eq!(atl.statements.len(), 10);
    assert!(matches!(
        &atl.statements[0],
        Some(AtlStatement::RawMultipurpose(node))
            if node.warper.as_deref() == Some("linear")
                && node.duration.as_deref() == Some("1.0")
                && node.properties == vec![("xalign".to_string(), "0.5".to_string())]
    ));
    assert!(matches!(
        &atl.statements[1],
        Some(AtlStatement::RawContainsExpr(node)) if node.expr == "icon_idle"
    ));
    assert!(matches!(
        &atl.statements[2],
        Some(AtlStatement::RawChild(node)) if node.child.statements.len() == 1 && node.child.statements[0].is_none()
    ));
    assert!(matches!(
        &atl.statements[3],
        Some(AtlStatement::RawParallel(node)) if node.block.statements.len() == 1
    ));
    assert!(matches!(
        &atl.statements[4],
        Some(AtlStatement::RawChoice(node)) if node.chance == "0.5" && node.block.statements.len() == 1
    ));
    assert!(matches!(
        &atl.statements[5],
        Some(AtlStatement::RawOn(node)) if node.names == vec!["show", "hide"] && node.block.statements.len() == 1
    ));
    assert!(matches!(
        &atl.statements[6],
        Some(AtlStatement::RawTime(node)) if node.time == "1.0"
    ));
    assert!(matches!(
        &atl.statements[7],
        Some(AtlStatement::RawFunction(node)) if node.expr == "callback"
    ));
    assert!(matches!(
        &atl.statements[8],
        Some(AtlStatement::RawEvent(node)) if node.name == "startled"
    ));
    assert!(matches!(
        &atl.statements[9],
        Some(AtlStatement::RawRepeat(node)) if node.repeats.is_none()
    ));
}

#[test]
fn atl_reports_duplicate_property_conflict_and_orientation_errors() {
    assert_error(
        parse_script(
            r#"
label atl_duplicate:
    show eileen:
        xalign 0.0 xalign 1.0
"#,
        ),
        "property xalign is given a value more than once",
        4,
    );

    assert_error(
        parse_script(
            r#"
label atl_conflict:
    show eileen:
        xalign 0.0 xpos 1.0
"#,
        ),
        "properties xpos and xalign conflict with each other",
        4,
    );

    assert_error(
        parse_script(
            r#"
label atl_orientation:
    show eileen:
        orientation 0 knot 1
"#,
        ),
        "Orientation doesn't support spline.",
        4,
    );
}

#[test]
fn call_arguments_and_label_parameters_parse() {
    let ast = assert_parse(parse_script(
        r#"
label start(a, /, b=1, *rest, **kwargs):
    call target(1, count=2, *items, **named)
"#,
    ));

    let AstNode::Label(label) = &ast[0] else {
        panic!("expected label");
    };
    let params = label.parameters.as_ref().expect("expected parameters");

    assert!(matches!(
        params.parameters.get("a"),
        Some(param) if matches!(param.kind, crate::ast::ParameterKind::PositionalOnly) && param.default.is_none()
    ));
    assert!(matches!(
        params.parameters.get("b"),
        Some(param) if matches!(param.kind, crate::ast::ParameterKind::PositionalOrKeyword) && param.default.as_deref() == Some("1")
    ));
    assert!(matches!(
        params.parameters.get("rest"),
        Some(param) if matches!(param.kind, crate::ast::ParameterKind::VarPositional)
    ));
    assert!(matches!(
        params.parameters.get("kwargs"),
        Some(param) if matches!(param.kind, crate::ast::ParameterKind::VarKeyword)
    ));

    let AstNode::Call(call) = &label.block[0] else {
        panic!("expected call");
    };
    let arguments = call.arguments.as_ref().expect("expected call arguments");

    assert_eq!(arguments.arguments.len(), 4);
    assert_eq!(arguments.arguments[0], (None, Some("1".to_string())));
    assert_eq!(
        arguments.arguments[1],
        (Some("count".to_string()), Some("2".to_string()))
    );
    assert_eq!(arguments.arguments[2], (None, Some("items".to_string())));
    assert_eq!(arguments.arguments[3], (None, Some("named".to_string())));
    assert!(arguments.starred_indexes.contains(&2));
    assert!(arguments.doublestarred_indexes.contains(&3));
}

#[test]
fn invalid_argument_and_parameter_forms_return_parse_errors() {
    assert_error(
        parse_script(
            r#"
label start:
    call target(count=1, 2)
"#,
        ),
        "positional argument follows keyword argument",
        3,
    );

    assert_error(
        parse_script(
            r#"
label start:
    call target(count=1, count=2)
"#,
        ),
        "keyword argument repeated: 'count'",
        3,
    );

    assert_error(
        parse(vec![block(
            1,
            "label start(*):",
            vec![block(2, "pass", vec![])],
        )]),
        "a bare * must be followed by a parameter",
        1,
    );

    assert_error(
        parse(vec![block(
            1,
            "label start(a=1, b):",
            vec![block(2, "pass", vec![])],
        )]),
        "non-default parameter b follows a default parameter",
        1,
    );
}

#[test]
fn transform_rejects_star_args_and_star_star_kwargs() {
    assert_error_contains(
        parse_script(
            r#"
transform wobble(*args):
    pass
"#,
        ),
        "the transform statement does not take *args",
        2,
    );

    assert_error_contains(
        parse_script(
            r#"
transform wobble(**kwargs):
    pass
"#,
        ),
        "the transform statement does not take **kwargs",
        2,
    );
}

#[test]
fn menu_label_arguments_with_and_set_parse() {
    let ast = assert_parse(parse_script(
        r#"
label start:
    menu side_menu(flag, answer=2):
        with dissolve
        set seen_choices
        "Choose wisely"
        "Go left"(10, who="e") if ready:
            pass
"#,
    ));

    let AstNode::Label(label) = &ast[0] else {
        panic!("expected label");
    };

    assert!(matches!(&label.block[0], AstNode::Label(node) if node.name == "side_menu"));

    let AstNode::Menu(menu) = &label.block[1] else {
        panic!("expected menu");
    };

    assert_eq!(menu.with_.as_deref(), Some("dissolve"));
    assert_eq!(menu.set.as_deref(), Some("seen_choices"));
    assert!(menu.has_caption);
    assert_eq!(menu.items.len(), 2);
    assert!(matches!(&menu.items[0], (Some(label), None, None) if label == "Choose wisely"));
    assert!(
        matches!(&menu.items[1], (Some(label), Some(condition), Some(block)) if label == "Go left" && condition == "ready" && matches!(&block[0], AstNode::Pass(_)))
    );

    let arguments = menu.arguments.as_ref().expect("expected menu arguments");
    assert_eq!(arguments.arguments.len(), 2);
    assert_eq!(arguments.arguments[0], (None, Some("flag".to_string())));
    assert_eq!(
        arguments.arguments[1],
        (Some("answer".to_string()), Some("2".to_string()))
    );

    let item_arguments = menu.item_arguments[1]
        .as_ref()
        .expect("expected menu item arguments");
    assert_eq!(item_arguments.arguments.len(), 2);
    assert_eq!(item_arguments.arguments[0], (None, Some("10".to_string())));
    assert_eq!(
        item_arguments.arguments[1],
        (Some("who".to_string()), Some("\"e\"".to_string()))
    );
}

#[test]
fn invalid_menu_and_say_forms_return_parse_errors() {
    assert_error(
        parse_script(
            r#"
menu:
    "Caption only"
"#,
        ),
        "Menu does not contain any choices.",
        2,
    );

    assert_error(
        parse_script(
            r#"
menu:
    e "Question?"
    "Caption"
    "Answer":
        pass
"#,
        ),
        "Captions and say menuitems may not exist in the same menu.",
        2,
    );

    assert_error(
        parse_script(
            r#"
label start:
    e "Hello" with dissolve with fade
"#,
        ),
        "say can only take a single with clause",
        3,
    );

    assert_error(
        parse_script(
            r#"
label start:
    e "Hello" (1) (2)
"#,
        ),
        "say can only take a single set of arguments",
        3,
    );
}

#[test]
fn say_statement_parses_arguments_and_negative_attributes() {
    let ast = assert_parse(parse_script(
        r#"
label start:
    e happy -sad @ blush "Hello" (1, tone="warm") with dissolve id greet
    "Narration" (volume=2) with fade id nar
"#,
    ));

    let AstNode::Label(label) = &ast[0] else {
        panic!("expected label");
    };

    assert!(matches!(
        &label.block[0],
        AstNode::Say(node)
            if node.attributes.as_ref().is_some_and(|attrs| attrs == &vec!["happy".to_string(), "-sad".to_string()])
                && node.temporary_attributes.as_ref().is_some_and(|attrs| attrs == &vec!["blush".to_string()])
                && node.with.as_deref() == Some("dissolve")
                && node.identifier.as_deref() == Some("greet")
                && node.arguments.as_ref().is_some_and(|args| args.arguments == vec![
                    (None, Some("1".to_string())),
                    (Some("tone".to_string()), Some("\"warm\"".to_string())),
                ])
    ));
    assert!(matches!(
        &label.block[1],
        AstNode::Say(node)
            if node.who.is_none()
                && node.what == "Narration"
                && node.with.as_deref() == Some("fade")
                && node.identifier.as_deref() == Some("nar")
                && node.arguments.as_ref().is_some_and(|args| args.arguments == vec![
                    (Some("volume".to_string()), Some("2".to_string())),
                ])
    ));
}

#[test]
fn media_statement_variants_parse() {
    let ast = assert_parse(parse_script(
        r#"
label media_more:
    scene onlayer master
    scene bg beach:
        xalign 0.5
    show layer overlay:
        yalign 1.0
    hide eileen with dissolve
image logo idle:
    linear 1.0 alpha 0.0
"#,
    ));

    let AstNode::Label(label) = &ast[0] else {
        panic!("expected label");
    };

    assert!(matches!(
        &label.block[0],
        AstNode::Scene(node) if node.layer.as_deref() == Some("master") && node.imspec.is_none() && node.atl.is_none()
    ));
    assert!(matches!(
        &label.block[1],
        AstNode::Scene(node)
            if node.imspec.as_ref().is_some_and(|imspec| imspec.image_name == vec!["bg".to_string(), "beach".to_string()])
                && node.atl.is_some()
    ));
    assert!(matches!(
        &label.block[2],
        AstNode::ShowLayer(node) if node.layer == "overlay" && node.atl.is_some()
    ));
    assert!(matches!(
        &label.block[3],
        AstNode::With(node) if node.expr == "None" && node.paired.as_deref() == Some("dissolve")
    ));
    assert!(matches!(
        &label.block[4],
        AstNode::Hide(node) if node.imgspec.image_name == vec!["eileen".to_string()]
    ));
    assert!(matches!(
        &label.block[5],
        AstNode::With(node) if node.expr == "dissolve" && node.paired.is_none()
    ));

    assert!(matches!(
        &ast[1],
        AstNode::Init(node)
            if node.priority == 500
                && matches!(&node.block[0], AstNode::Image(image)
                    if image.name == vec!["logo".to_string(), "idle".to_string()] && image.atl.is_some() && image.expr.is_none())
    ));
}

#[test]
fn init_statement_variants_parse() {
    let ast = assert_parse(parse_script(
        r#"
init 10:
    define foo = 1
init 5 default persistent.answer = 42
"#,
    ));

    assert!(matches!(
        &ast[0],
        AstNode::Init(node)
            if node.priority == 10 && matches!(&node.block[0], AstNode::Define(define) if define.name == "foo" && define.expr == "1")
    ));
    assert!(matches!(
        &ast[1],
        AstNode::Init(node)
            if node.priority == 5
                && matches!(&node.block[0], AstNode::Default(default)
                    if default.store == "store.persistent" && default.name == "answer" && default.expr.as_deref() == Some("42"))
    ));
}

#[test]
fn media_and_init_errors_return_parse_errors() {
    assert_error(
        parse(vec![block(1, "$", vec![])]),
        "expected python code",
        1,
    );

    assert_error(
        parse_script(
            r#"
image logo idle =
"#,
        ),
        "expected expression",
        2,
    );
}

#[test]
fn style_clauses_parse_and_errors_are_reported() {
    let ast = assert_parse(parse_script(
        r#"
style fancy is base clear take other del xalign variant "small" xsize 10
"#,
    ));

    assert!(matches!(
        &ast[0],
        AstNode::Init(node)
            if matches!(&node.block[0], AstNode::Style(style)
                if style.name == "fancy"
                    && style.parent.as_deref() == Some("base")
                    && style.clear
                    && style.take.as_deref() == Some("other")
                    && style.delattr == vec!["xalign".to_string()]
                    && style.variant.as_deref() == Some("\"small\"")
                    && style.properties.get("xsize").map(String::as_str) == Some("10"))
    ));

    assert_error(
        parse_script(
            r#"
style bad is one is two
"#,
        ),
        "parent clause appears twice.",
        2,
    );

    assert_error(
        parse_script(
            r#"
style bad take one take two
"#,
        ),
        "take clause appears twice.",
        2,
    );

    assert_error(
        parse_script(
            r#"
style bad variant "a" variant "b"
"#,
        ),
        "variant clause appears twice.",
        2,
    );

    assert_error(
        parse_script(
            r#"
style bad del madeup
"#,
        ),
        "style property madeup is not known.",
        2,
    );

    assert_error(
        parse_script(
            r#"
style bad madeup 1
"#,
        ),
        "style property madeup is not known.",
        2,
    );
}

#[test]
fn translate_strings_error_cases_return_parse_errors() {
    assert_error(
        parse_script(
            r#"
translate english strings:
    old "One"
    old "Two"
"#,
        ),
        "previous string is missing a translation",
        4,
    );

    assert_error(
        parse_script(
            r#"
translate english strings:
    new "Two"
"#,
        ),
        "no string to translate",
        3,
    );

    assert_error(
        parse_script(
            r#"
translate english strings:
    old "One"
"#,
        ),
        "final string is missing a translation",
        2,
    );

    assert_error(
        parse_script(
            r#"
translate english strings:
    pass
"#,
        ),
        "unknown statement",
        3,
    );
}

#[test]
fn flow_and_rpy_variants_parse() {
    let ast = assert_parse(parse_script(
        r#"
rpy monologue single
rpy monologue double
label start:
    with dissolve
    jump expression next_target
    call expression callee_expr from return_spot
"#,
    ));

    assert!(matches!(&ast[0], AstNode::Label(node) if node.name == "start"));

    let AstNode::Label(label) = &ast[0] else {
        panic!("expected label");
    };

    assert!(matches!(&label.block[0], AstNode::With(node) if node.expr == "dissolve"));
    assert!(matches!(
        &label.block[1],
        AstNode::Jump(node)
            if node.expression && node.target == "next_target" && node.global_label.as_deref() == Some("start")
    ));
    assert!(matches!(
        &label.block[2],
        AstNode::Call(node)
            if node.expression && node.label == "callee_expr" && node.global_label.as_deref() == Some("start")
    ));
    assert!(matches!(&label.block[3], AstNode::Label(node) if node.name == "return_spot"));
}

#[test]
fn atl_additional_variants_parse_and_error() {
    let ast = assert_parse(parse_script(
        r#"
label extra_atl:
    show eileen:
        clockwise
        circles 2
        counterclockwise
        foo with dissolve
"#,
    ));

    let AstNode::Label(label) = &ast[0] else {
        panic!("expected label");
    };
    let AstNode::Show(show) = &label.block[0] else {
        panic!("expected show");
    };
    let atl = show.atl.as_ref().expect("expected ATL block");

    assert_eq!(atl.statements.len(), 4);
    assert!(matches!(
        &atl.statements[0],
        Some(AtlStatement::RawMultipurpose(_))
    ));
    assert!(matches!(
        &atl.statements[1],
        Some(AtlStatement::RawMultipurpose(_))
    ));
    assert!(matches!(
        &atl.statements[2],
        Some(AtlStatement::RawMultipurpose(_))
    ));
    assert!(
        matches!(&atl.statements[3], Some(AtlStatement::RawMultipurpose(node)) if node.expressions == vec![("foo".to_string(), Some("dissolve".to_string()))])
    );

    assert_error(
        parse_script(
            r#"
label bad_atl:
    show eileen:
        foo bar
"#,
        ),
        "ATL statement contains two expressions in a row; is one of them a misspelled property? If not, separate them with pass.",
        4,
    );
}

#[test]
fn invalid_rpy_statement_returns_parse_error() {
    assert_error(
        parse_script(
            r#"
rpy monologue weird
"#,
        ),
        "rpy monologue expects either none, single or double.",
        2,
    );
}
