use crate::{
    ast::{AstNode, AudioOperation, AudioTarget, ScreenStatementKind, WindowAutoKind},
    atl::AtlStatement,
    error::{ParseError, Result},
    test_support::{block, parse, parse_script},
    testast::{TestCondition, TestNode, TestSelector, TestSuiteEntry},
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
fn registered_builtin_user_statements_parse() {
    let ast = assert_parse(parse_script(concat!(
        "play music \"theme.ogg\" fadeout 1.0 fadein 0.5 if_changed volume 0.8\n",
        "queue music [\"a.ogg\", \"b.ogg\"] channel music_loop loop fadein 1.25\n",
        "stop music fadeout 2.0 channel music_loop\n",
        "play sound \"click.ogg\" channel sfx noloop volume 0.4\n",
        "queue sound \"next.ogg\" channel sfx loop fadein 0.2\n",
        "stop sound fadeout 0.5 channel sfx\n",
        "play ambient \"wind.ogg\" fadein 0.1\n",
        "queue ambient \"gust.ogg\" loop\n",
        "stop ambient fadeout 0.75\n",
        "pause 0.25\n",
        "show screen preferences(page=\"audio\") nopredict with dissolve onlayer screens zorder 20 as prefs\n",
        "call screen confirm(\"Quit?\") with dissolve\n",
        "hide screen preferences with fade onlayer screens\n",
        "window show Dissolve(0.2)\n",
        "window hide\n",
        "window auto hide Dissolve(0.3)\n",
    )));

    assert_eq!(ast.len(), 16);
    assert!(matches!(
        &ast[0],
        AstNode::AudioStatement(node)
            if matches!(node.operation, AudioOperation::Play)
                && matches!(node.target, AudioTarget::Music)
                && node.file.as_deref() == Some("\"theme.ogg\"")
                && node.fadeout.as_deref() == Some("1.0")
                && node.fadein.as_deref() == Some("0.5")
                && node.if_changed
                && node.volume.as_deref() == Some("0.8")
    ));
    assert!(matches!(
        &ast[10],
        AstNode::ScreenStatement(node)
            if matches!(node.kind, ScreenStatementKind::Show)
                && node.screen.value == "preferences"
                && !node.screen.expression
                && node.arguments.is_some()
                && !node.predict
                && node.with.as_deref() == Some("dissolve")
                && node.layer.as_deref() == Some("screens")
                && node.zorder.as_deref() == Some("20")
                && node.tag.as_deref() == Some("prefs")
    ));
    assert!(matches!(
        &ast[15],
        AstNode::WindowAutoStatement(node)
            if matches!(&node.kind, WindowAutoKind::Hide(Some(expr)) if expr == "Dissolve(0.3)")
    ));
}

#[test]
fn registered_builtin_user_statements_parse_expression_screen_and_window_auto_variants() {
    let ast = assert_parse(parse_script(concat!(
        "show screen expression current_screen pass (page=selected) with dissolve onlayer overlay zorder 7 as current\n",
        "call screen expression current_screen pass (page=selected)\n",
        "hide screen expression current_screen with fade onlayer overlay\n",
        "window auto\n",
        "window auto True\n",
        "window auto show\n",
        "window auto hide Dissolve(0.3)\n",
    )));

    assert!(matches!(
        &ast[0],
        AstNode::ScreenStatement(node)
            if matches!(node.kind, ScreenStatementKind::Show)
                && node.screen.expression
                && node.screen.value == "current_screen"
                && node.arguments.is_some()
                && node.with.as_deref() == Some("dissolve")
                && node.layer.as_deref() == Some("overlay")
                && node.zorder.as_deref() == Some("7")
                && node.tag.as_deref() == Some("current")
    ));
    assert!(matches!(
        &ast[1],
        AstNode::ScreenStatement(node)
            if matches!(node.kind, ScreenStatementKind::Call)
                && node.screen.expression
                && node.arguments.is_some()
    ));
    assert!(matches!(
        &ast[2],
        AstNode::ScreenStatement(node)
            if matches!(node.kind, ScreenStatementKind::Hide)
                && node.screen.expression
                && node.with.as_deref() == Some("fade")
                && node.layer.as_deref() == Some("overlay")
    ));
    assert!(matches!(
        &ast[3],
        AstNode::WindowAutoStatement(node) if matches!(node.kind, WindowAutoKind::Auto(None))
    ));
    assert!(matches!(
        &ast[4],
        AstNode::WindowAutoStatement(node)
            if matches!(&node.kind, WindowAutoKind::Auto(Some(expr)) if expr == "True")
    ));
    assert!(matches!(
        &ast[5],
        AstNode::WindowAutoStatement(node) if matches!(node.kind, WindowAutoKind::Show(None))
    ));
    assert!(matches!(
        &ast[6],
        AstNode::WindowAutoStatement(node)
            if matches!(&node.kind, WindowAutoKind::Hide(Some(expr)) if expr == "Dissolve(0.3)")
    ));
}

#[test]
fn malformed_registered_builtin_user_statements_return_parse_errors() {
    assert_error(
        parse_script("play music \"theme.ogg\" fadein"),
        "expected simple expression",
        1,
    );
    assert_error(parse_script("queue ambient"), "queue requires a file", 1);
    assert_error(parse_script("stop"), "stop requires a channel", 1);
    assert_error(
        parse_script("show screen preferences zorder"),
        "expected simple expression",
        1,
    );
    assert_error(parse_script("hide screen"), "expected screen name", 1);
    assert_error(
        parse_script("window auto hide extra junk"),
        "end of line expected",
        1,
    );
    assert_error(parse_script("pause 1 2"), "end of line expected", 1);
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

    assert!(matches!(&ast[0], AstNode::InitOffset(node) if node.offset == 5));
    assert!(matches!(&ast[1], AstNode::Init(node) if node.priority == 5));
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
fn layeredimage_statement_parses() {
    let ast = assert_parse(parse(vec![block(
        1,
        "layeredimage eileen happy:",
        vec![
            block(2, "offer_screen False", vec![]),
            block(
                3,
                "attribute body default:",
                vec![block(4, "\"body.png\"", vec![])],
            ),
            block(
                5,
                "group face auto:",
                vec![block(6, "attribute smile", vec![])],
            ),
            block(7, "if wearing_hat:", vec![block(8, "\"hat.png\"", vec![])]),
            block(9, "elif True:", vec![block(10, "null", vec![])]),
            block(
                11,
                "always:",
                vec![block(12, "image:", vec![block(13, "pass", vec![])])],
            ),
        ],
    )]));

    let AstNode::Init(init) = &ast[0] else {
        panic!("expected implicit init node");
    };
    assert_eq!(init.priority, 0);

    let AstNode::LayeredImage(layered) = &init.block[0] else {
        panic!("expected layeredimage child");
    };
    assert_eq!(
        layered.name,
        vec!["eileen".to_string(), "happy".to_string()]
    );
    assert_eq!(layered.children.len(), 4);
}

#[test]
fn layeredimage_attribute_rejects_variant_with_displayable() {
    assert_error(
        parse(vec![block(
            1,
            "layeredimage eileen:",
            vec![block(
                2,
                "attribute happy variant alt \"happy.png\"",
                vec![],
            )],
        )]),
        "Attribute \"happy\" cannot have a variant if it is provided a displayable.",
        2,
    );
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
fn screen_with_basic_pass_parses() {
    let ast = assert_parse(parse(vec![block(
        1,
        "screen simple:",
        vec![block(2, "pass", vec![])],
    )]));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    assert_eq!(screen.screen.name, "simple");
    assert!(matches!(
        &screen.screen.children[0],
        crate::slast::Node::Pass(_)
    ));
}

#[test]
fn screen_with_parameters_and_properties_parses() {
    let ast = assert_parse(parse_script(
        "screen say(who, what):\n    tag menu\n    modal True\n    zorder 100\n    window:\n        text what",
    ));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    assert!(screen.screen.parameters.is_some());
    assert_eq!(
        screen.screen.properties,
        vec![
            ("tag".to_string(), "menu".to_string()),
            ("modal".to_string(), "True".to_string()),
            ("zorder".to_string(), "100".to_string()),
        ]
    );
}

#[test]
fn screen_displayables_and_use_parse() {
    let ast = assert_parse(parse_script(
        "screen navigation():\n    vbox:\n        textbutton _(\"Start\") action Start()\n        use extra_nav(id=\"root\")",
    ));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    let crate::slast::Node::Displayable(vbox) = &screen.screen.children[0] else {
        panic!("expected vbox");
    };
    assert_eq!(vbox.name, "vbox");
    assert_eq!(vbox.children.len(), 2);
}

#[test]
fn screen_use_block_properties_parse() {
    let ast = assert_parse(parse_script(concat!(
        "screen about():\n",
        "    use game_menu(_(\"About\"), scroll=\"viewport\"):\n",
        "        style_prefix \"about\"\n",
        "        vbox:\n",
        "            transclude\n",
    )));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    let crate::slast::Node::Use(use_node) = &screen.screen.children[0] else {
        panic!("expected use node");
    };
    let block = use_node.block.as_ref().expect("expected use block");

    assert_eq!(
        block.properties,
        vec![("style_prefix".to_string(), "\"about\"".to_string())]
    );
    assert!(matches!(
        &block.children[0],
        crate::slast::Node::Displayable(node) if node.name == "vbox"
    ));
}

#[test]
fn zero_child_displayable_allows_conditional_properties() {
    let ast = assert_parse(parse_script(concat!(
        "screen filters(ssa):\n",
        "    input value ssa prefix \" \":\n",
        "        if not renpy.variant(\"small\"):\n",
        "            line_leading 6\n",
        "        length 20\n",
        "        copypaste True\n",
    )));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    let crate::slast::Node::Displayable(input) = &screen.screen.children[0] else {
        panic!("expected input displayable");
    };
    let crate::slast::Node::If(if_node) = &input.children[0] else {
        panic!("expected conditional child");
    };

    assert_eq!(
        if_node.entries[0].1.properties,
        vec![("line_leading".to_string(), "6".to_string())]
    );
    assert!(
        input
            .properties
            .iter()
            .any(|(name, expr)| name == "length" && expr == "20")
    );
}

#[test]
fn screen_block_property_line_allows_multiple_properties() {
    let ast = assert_parse(parse_script(concat!(
        "screen musicroom(musicRoomType):\n",
        "    viewport id musicRoomType:\n",
        "        draggable True mousewheel True\n",
        "        has vbox\n",
    )));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    let crate::slast::Node::Displayable(viewport) = &screen.screen.children[0] else {
        panic!("expected viewport displayable");
    };

    assert!(
        viewport
            .properties
            .iter()
            .any(|(name, expr)| name == "draggable" && expr == "True")
    );
    assert!(
        viewport
            .properties
            .iter()
            .any(|(name, expr)| name == "mousewheel" && expr == "True")
    );
}

#[test]
fn screen_if_showif_and_for_parse() {
    let ast = assert_parse(parse_script(concat!(
        "screen complex(slots):\n",
        "    if persistent.foo:\n",
        "        text \"yes\"\n",
        "    elif persistent.bar:\n",
        "        text \"maybe\"\n",
        "    else:\n",
        "        pass\n",
        "    showif visible:\n",
        "        text \"shown\"\n",
        "    for slot index i in slots:\n",
        "        if slot.hidden:\n",
        "            continue\n",
        "        text slot.name\n",
    )));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    assert!(
        matches!(&screen.screen.children[0], crate::slast::Node::If(node) if node.entries.len() == 3)
    );
    assert!(
        matches!(&screen.screen.children[1], crate::slast::Node::ShowIf(node) if node.entries.len() == 1)
    );
    assert!(
        matches!(&screen.screen.children[2], crate::slast::Node::For(node) if node.index_expression.as_deref() == Some("i"))
    );
}

#[test]
fn screen_python_default_transclude_and_at_transform_parse() {
    let ast = assert_parse(parse_script(concat!(
        "screen tools(default_name):\n",
        "    default current = default_name\n",
        "    $ current = current.upper()\n",
        "    python:\n",
        "        current = current.lower()\n",
        "    use panel:\n",
        "        transclude\n",
        "    text current:\n",
        "        at transform:\n",
        "            linear 1.0 alpha 1.0\n",
    )));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    assert!(matches!(
        &screen.screen.children[0],
        crate::slast::Node::Default(_)
    ));
    assert!(matches!(&screen.screen.children[1], crate::slast::Node::Python(node) if !node.block));
    assert!(matches!(&screen.screen.children[2], crate::slast::Node::Python(node) if node.block));
    assert!(matches!(
        &screen.screen.children[3],
        crate::slast::Node::Use(_)
    ));
    assert!(
        matches!(&screen.screen.children[4], crate::slast::Node::Displayable(node) if node.atl_transform.is_some())
    );
}

#[test]
fn screen_window_properties_with_commas_and_add_parse() {
    let ast = assert_parse(parse_script(concat!(
        "screen physics_quiz():\n",
        "    window:\n",
        "        pos (int(1280 / 2), int(88 / 2))\n",
        "        anchor (0.5, 0.5)\n",
        "        xmargin 24\n",
        "        ymargin 16\n",
        "        yminimum 0\n",
        "        ymaximum 88\n",
        "        at quizshow_show_hide\n",
        "        add LiveMarquee(Text(u\"Speed of an Egg\", slow=True, style='quizshow')) crop 0, 0, 900, 58 xoffset -10\n",
    )));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    let crate::slast::Node::Displayable(window) = &screen.screen.children[0] else {
        panic!("expected window displayable");
    };
    assert_eq!(window.properties[0].0, "pos");
    assert!(
        window
            .properties
            .iter()
            .any(|(name, expr)| name == "at" && expr == "quizshow_show_hide")
    );
    let crate::slast::Node::Displayable(add) = &window.children[0] else {
        panic!("expected add displayable");
    };
    assert!(
        add.properties
            .iter()
            .any(|(name, expr)| name == "crop" && expr == "0, 0, 900, 58")
    );
    assert!(
        add.properties
            .iter()
            .any(|(name, expr)| name == "xoffset" && expr == "-10")
    );
}

#[test]
fn screen_displayable_at_property_before_block_parses() {
    let ast = assert_parse(parse_script(concat!(
        "screen navigation():\n",
        "    on \"hide\" action SetField(renpy.store, \"from_intro\", False)\n",
        "    vbox at (alpha_blend(1.0) if renpy.store.from_intro else None):\n",
        "        style_prefix \"navigation\"\n",
        "        xpos gui.navigation_xpos\n",
    )));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    let crate::slast::Node::Displayable(vbox) = &screen.screen.children[1] else {
        panic!("expected vbox displayable");
    };

    assert!(
        vbox.properties.iter().any(|(name, expr)| {
            name == "at" && expr == "(alpha_blend(1.0) if renpy.store.from_intro else None)"
        })
    );
    assert!(
        vbox.properties
            .iter()
            .any(|(name, expr)| name == "style_prefix" && expr == "\"navigation\"")
    );
}

#[test]
fn screen_icon_and_iconbutton_parse() {
    let ast = assert_parse(parse_script(concat!(
        "screen toolbar():\n",
        "    icon \"save\" color \"#fff\"\n",
        "    iconbutton \"prefs\":\n",
        "        caption _(\"Preferences\")\n",
        "        action ShowMenu(\"preferences\")\n",
        "        icon_color \"#8cf\"\n",
    )));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };

    let crate::slast::Node::Displayable(icon) = &screen.screen.children[0] else {
        panic!("expected icon displayable");
    };
    assert_eq!(icon.name, "icon");
    assert_eq!(icon.positional, vec!["\"save\"".to_string()]);
    assert!(
        icon.properties
            .iter()
            .any(|(name, expr)| name == "color" && expr == "\"#fff\"")
    );

    let crate::slast::Node::Displayable(iconbutton) = &screen.screen.children[1] else {
        panic!("expected iconbutton displayable");
    };
    assert_eq!(iconbutton.name, "iconbutton");
    assert_eq!(iconbutton.positional, vec!["\"prefs\"".to_string()]);
    assert!(
        iconbutton
            .properties
            .iter()
            .any(|(name, expr)| name == "caption" && expr == "_(\"Preferences\")")
    );
    assert!(
        iconbutton
            .properties
            .iter()
            .any(|(name, expr)| name == "action" && expr == "ShowMenu(\"preferences\")")
    );
    assert!(
        iconbutton
            .properties
            .iter()
            .any(|(name, expr)| name == "icon_color" && expr == "\"#8cf\"")
    );
}

#[test]
fn screen_conditional_properties_inside_displayable_parse() {
    let ast = assert_parse(parse_script(concat!(
        "screen scenario_picker(found_hash):\n",
        "    button:\n",
        "        style \"scenario_button\"\n",
        "        xfill True\n",
        "        if scenario.hooks[found_hash]:\n",
        "            ymaximum 100\n",
        "        else:\n",
        "            ymaximum 50\n",
        "        margin 5, 0\n",
        "        vbox\n",
    )));

    let AstNode::Screen(screen) = &ast[0] else {
        panic!("expected screen node");
    };
    let crate::slast::Node::Displayable(button) = &screen.screen.children[0] else {
        panic!("expected button displayable");
    };
    let crate::slast::Node::If(if_node) = &button.children[0] else {
        panic!("expected conditional child");
    };

    assert_eq!(if_node.entries.len(), 2);
    assert_eq!(
        if_node.entries[0].1.properties,
        vec![("ymaximum".to_string(), "100".to_string())]
    );
    assert_eq!(
        if_node.entries[1].1.properties,
        vec![("ymaximum".to_string(), "50".to_string())]
    );
    assert!(
        button
            .properties
            .iter()
            .any(|(name, expr)| name == "margin" && expr == "5, 0")
    );
}

#[test]
fn screen_duplicate_property_returns_parse_error() {
    assert_error_contains(
        parse_script("screen dupes:\n    text \"Hello\" xalign 0.0 xalign 1.0"),
        "appears more than once",
        2,
    );
}

#[test]
fn screen_unknown_child_returns_parse_error() {
    assert_error_contains(
        parse_script("screen odd:\n    mystery_widget:"),
        "not a valid child statement",
        2,
    );
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
fn testcase_parses_structured_body_and_properties() {
    let ast = assert_parse(parse(vec![block(
        1,
        "testcase foo.bar:",
        vec![
            block(2, "description \"desc\"", vec![]),
            block(3, "enabled flag", vec![]),
            block(4, "parameter (x, y) = [(1, 2)]", vec![]),
            block(5, "assert eval ready timeout 2.0", vec![]),
        ],
    )]));

    let AstNode::Testcase(node) = &ast[0] else {
        panic!("expected testcase");
    };

    assert_eq!(node.test.name, "foo.bar");
    assert_eq!(
        node.test.properties.description.as_deref(),
        Some("\"desc\"")
    );
    assert_eq!(node.test.properties.enabled.as_deref(), Some("flag"));
    assert_eq!(node.test.properties.parameters.len(), 1);
    assert_eq!(node.test.properties.parameters[0].names, vec!["x", "y"]);
    assert_eq!(node.test.properties.parameters[0].values_expr, "[(1, 2)]");
    assert!(matches!(
        &node.test.statements[0],
        TestNode::Assert(assert)
            if matches!(&assert.condition, TestCondition::Eval { expr, .. } if expr == "ready")
                && assert.timeout.as_deref() == Some("2.0")
    ));
}

#[test]
fn testsuite_parses_hooks_and_nested_children() {
    let ast = assert_parse(parse(vec![block(
        1,
        "testsuite foo.bar:",
        vec![
            block(2, "before testcase:", vec![block(3, "pass", vec![])]),
            block(
                4,
                "testcase inner:",
                vec![block(5, "advance until label chapter_5", vec![])],
            ),
        ],
    )]));

    let AstNode::Testsuite(node) = &ast[0] else {
        panic!("expected testsuite");
    };

    assert_eq!(node.suite.name, "foo.bar");
    assert_eq!(node.suite.entries.len(), 2);
    assert!(matches!(
        &node.suite.entries[0],
        TestSuiteEntry::Hook(hook) if hook.kind.as_str() == "before testcase"
    ));
    assert!(matches!(
        &node.suite.entries[1],
        TestSuiteEntry::TestCase(case)
            if case.name == "inner"
                && matches!(
                    &case.statements[0],
                    TestNode::Until(until)
                        if matches!(&until.condition, TestCondition::Label { name, .. } if name == "chapter_5")
                )
    ));
}

#[test]
fn testcase_rejects_property_after_statement() {
    assert_error_contains(
        parse_script("testcase foo.bar:\n    pass\n    enabled flag"),
        "Property enabled must be defined before any test statements.",
        3,
    );
}

#[test]
fn testsuite_rejects_duplicate_hook() {
    assert_error_contains(
        parse_script("testsuite foo.bar:\n    setup:\n        pass\n    setup:\n        pass"),
        "Only one 'setup' block is allowed in a testsuite.",
        4,
    );
}

#[test]
fn testsuite_rejects_invalid_before_clause() {
    assert_error_contains(
        parse_script("testsuite foo.bar:\n    before something:\n        pass"),
        "Expected 'before testsuite' or 'before testcase'.",
        2,
    );
}

#[test]
fn testcase_rejects_nested_testcase_in_block() {
    assert_error_contains(
        parse_script("testcase foo.bar:\n    testcase inner:\n        pass"),
        "may not be nested inside a block",
        2,
    );
}

#[test]
fn testsuite_hook_defaults_are_applied() {
    let ast = assert_parse(parse_script(
        "testsuite foo.bar:\n    before testcase:\n        pass\n    after testsuite:\n        pass",
    ));

    let AstNode::Testsuite(node) = &ast[0] else {
        panic!("expected testsuite");
    };

    assert!(matches!(
        &node.suite.entries[0],
        TestSuiteEntry::Hook(hook) if hook.properties.depth.as_deref() == Some("-1")
    ));
    assert!(matches!(
        &node.suite.entries[1],
        TestSuiteEntry::Hook(hook) if hook.properties.depth.as_deref() == Some("0")
    ));
}

#[test]
fn selector_and_statement_variants_parse() {
    let ast = assert_parse(parse_script(
        r#"
testcase foo.bar:
    click "Next" button 1 pos (10, 20) always
    drag pos (0, 0) to pos (100, 100) steps 10
    pause until screen "choice"
    $ print("ok")
"#,
    ));

    let AstNode::Testcase(node) = &ast[0] else {
        panic!("expected testcase");
    };

    assert!(matches!(
        &node.test.statements[0],
        TestNode::Click(click)
            if click.button.as_deref() == Some("1")
                && click.position.as_deref() == Some("(10, 20)")
                && click.always
                && matches!(
                    click.selector.as_ref(),
                    Some(TestSelector::Text(selector)) if selector.pattern == "Next"
                )
    ));
    assert!(matches!(
        &node.test.statements[1],
        TestNode::Drag(drag)
            if drag.start.position.as_deref() == Some("(0, 0)")
                && drag.end.position.as_deref() == Some("(100, 100)")
                && drag.steps.as_deref() == Some("10")
    ));
    assert!(matches!(
        &node.test.statements[2],
        TestNode::Until(until)
            if matches!(
                &until.condition,
                TestCondition::Selector(TestSelector::Displayable(selector))
                    if selector.screen.as_deref() == Some("\"choice\"")
            )
    ));
    assert!(matches!(
        &node.test.statements[3],
        TestNode::Python(python) if !python.block && python.code == "print(\"ok\")"
    ));
}

#[test]
fn selector_rejects_two_text_patterns() {
    assert_error_contains(
        parse_script("testcase foo.bar:\n    click \"One\" \"Two\""),
        "Only one text pattern may be specified in a selector.",
        2,
    );
}

#[test]
fn selector_rejects_text_with_screen() {
    assert_error_contains(
        parse_script("testcase foo.bar:\n    click \"One\" screen \"menu\""),
        "may not be specified with a screen or id",
        2,
    );
}

#[test]
fn parameter_rejects_duplicate_names() {
    assert_error_contains(
        parse_script("testcase foo.bar:\n    parameter (x, x) = [(1, 2)]"),
        "must be unique",
        2,
    );
}

#[test]
fn parameter_rejects_empty_tuple() {
    assert_error_contains(
        parse_script("testcase foo.bar:\n    parameter () = []"),
        "Expected at least one name",
        2,
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
    assert_eq!(
        params.order,
        vec![
            "a".to_string(),
            "b".to_string(),
            "rest".to_string(),
            "kwargs".to_string()
        ]
    );

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
