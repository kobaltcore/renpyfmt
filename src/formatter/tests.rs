use crate::{
    ast::{AstNode, ImageSpecifier, Label, Say, Show, TranslateBlock},
    atl::{AtlStatement, RawBlock, RawMultipurpose},
    comments::{Comment, CommentMap},
    formatter::format_ast,
    test_support::format_script,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn assert_formats(source: &str, expected: &str) {
    assert_eq!(format_script(source), expected);
}

fn image(name: &[&str]) -> ImageSpecifier {
    ImageSpecifier {
        image_name: name.iter().map(|part| part.to_string()).collect(),
        ..Default::default()
    }
}

#[test]
fn formats_label_block_without_embedded_extra_newlines() {
    assert_formats(
        "label start:\n    e \"Hello\"\n    jump next",
        "label start:\n    e \"Hello\"\n\n    jump next",
    );
}

#[test]
fn formats_if_elif_else_blocks() {
    assert_formats(
        concat!(
            "if flag:\n",
            "    \"yes\"\n",
            "elif other:\n",
            "    jump other_label\n",
            "else:\n",
            "    call fallback"
        ),
        concat!(
            "if flag:\n",
            "    \"yes\"\n",
            "\n",
            "elif other:\n",
            "    jump other_label\n",
            "else:\n",
            "    call fallback"
        ),
    );
}

#[test]
fn formats_menu_with_caption_and_condition() {
    assert_formats(
        concat!(
            "menu:\n",
            "    \"Caption\"\n",
            "    \"Choice\" if seen:\n",
            "        jump next"
        ),
        concat!(
            "menu:\n",
            "    \"Caption\"\n",
            "    \"Choice\" if seen:\n",
            "        jump next"
        ),
    );
}

#[test]
fn formats_show_with_nested_atl() {
    assert_formats(
        concat!(
            "show eileen:\n",
            "    linear 1.0 xalign 0.5\n",
            "    parallel:\n",
            "        choice 0.5:\n",
            "            xalign 0.0"
        ),
        concat!(
            "show eileen:\n",
            "    linear 1.0 xalign 0.5\n",
            "    parallel:\n",
            "        choice 0.5:\n",
            "            xalign 0.0"
        ),
    );
}

#[test]
fn inserts_blank_line_before_top_level_scene() {
    assert_formats("\"hello\"\nscene bg room", "\"hello\"\n\nscene bg room");
}

#[test]
fn formats_supported_flow_and_init_statements() {
    assert_formats(
        concat!(
            "init 5 default persistent.answer = 42\n",
            "python early hide in mystore:\n",
            "    total = 1\n",
            "if seen_intro:\n",
            "    pass\n",
            "else:\n",
            "    while waiting:\n",
            "        pass\n",
            "IF renpy.version_tuple >= (8, 0):\n",
            "    pass\n",
            "ELSE:\n",
            "    pass"
        ),
        concat!(
            "init 5:\n",
            "    default persistent.answer = 42\n",
            "\n",
            "python early hide in mystore:\n",
            "    total = 1\n",
            "\n",
            "if seen_intro:\n",
            "    pass\n",
            "else:\n",
            "    while waiting:\n",
            "        pass\n",
            "\n",
            "IF renpy.version_tuple >= (8, 0):\n",
            "    pass\n",
            "ELSE:\n",
            "    pass"
        ),
    );
}

#[test]
fn formats_init_python_compact_form() {
    assert_formats(
        concat!(
            "init python:\n",
            "    x = 1\n",
            "\n",
            "init 5 python hide in mystore:\n",
            "    y = 2"
        ),
        concat!(
            "init python:\n",
            "    x = 1\n",
            "\n",
            "init 5 python hide in mystore:\n",
            "    y = 2"
        ),
    );
}

#[test]
fn formats_python_blocks_with_ruff() {
    assert_formats(
        concat!(
            "python:\n",
            "    numbers=[1,2,3]\n",
            "    if True: print( numbers )"
        ),
        concat!(
            "python:\n",
            "    numbers = [1, 2, 3]\n",
            "    if True:\n",
            "        print(numbers)"
        ),
    );
}

#[test]
fn formats_nested_python_blocks_with_ruff() {
    assert_formats(
        concat!(
            "label start:\n",
            "    python:\n",
            "        values=[1,2]\n",
            "        if True: print( values )"
        ),
        concat!(
            "label start:\n",
            "    python:\n",
            "        values = [1, 2]\n",
            "        if True:\n",
            "            print(values)"
        ),
    );
}

#[test]
fn formats_supported_media_and_atl_statement_variants() {
    assert_formats(
        concat!(
            "show layer overlay at left:\n",
            "    animation\n",
            "    contains icon_idle\n",
            "    on show, hide:\n",
            "        pass\n",
            "    time 1.0\n",
            "    function callback\n",
            "    event startled\n",
            "    repeat 2\n",
            "\n",
            "camera at wobble\n",
            "image logo idle = \"room.webp\""
        ),
        concat!(
            "show layer overlay at left:\n",
            "    animation\n",
            "    contains icon_idle\n",
            "    on show, hide:\n",
            "        pass\n",
            "    time 1.0\n",
            "    function callback\n",
            "    event startled\n",
            "    repeat 2\n",
            "\n",
            "camera at wobble\n",
            "image logo idle = \"room.webp\""
        ),
    );
}

#[test]
fn formats_implicit_init_statements_without_init_blocks() {
    assert_formats(
        concat!(
            "style nvl_window_badend is nvl_window:\n",
            "    background None\n",
            "    xpadding 40\n",
            "    ypadding 40\n",
            "define badnar = Character(what_color='#ffffff', what_size=40, what_outlines=[(2, '#000000')], what_text_align=0.5, kind=nvl_narrator)\n",
            "transform wobble:\n",
            "    linear xalign 0.5\n",
            "image bg room = \"room.webp\""
        ),
        concat!(
            "style nvl_window_badend is nvl_window:\n",
            "    background None\n",
            "    xpadding 40\n",
            "    ypadding 40\n",
            "define badnar = Character(what_color='#ffffff', what_size=40, what_outlines=[(2, '#000000')], what_text_align=0.5, kind=nvl_narrator)\n",
            "transform wobble:\n",
            "    linear xalign 0.5\n",
            "image bg room = \"room.webp\""
        ),
    );
}

#[test]
fn keeps_explicit_init_blocks_for_non_default_priorities() {
    assert_formats("init 5 define foo = 1", "init 5:\n    define foo = 1");
}

#[test]
fn keeps_implicit_init_statements_bare_after_init_offset() {
    assert_formats(
        concat!("init offset = -2\n", "define gui.accent_color = '#9e2c94'"),
        concat!("init offset = -2\n", "define gui.accent_color = '#9e2c94'"),
    );
}

#[test]
fn formats_basic_screen_language_blocks() {
    assert_formats(
        concat!(
            "screen say(what, who):\n",
            "    tag say\n",
            "    window:\n",
            "        text what"
        ),
        concat!(
            "screen say(what, who):\n",
            "    tag say\n",
            "    window:\n",
            "        text what"
        ),
    );
}

#[test]
fn formats_textbutton_properties_in_block_form() {
    assert_formats(
        "screen navigation():\n    textbutton _(\"Start\") action Start()",
        concat!(
            "screen navigation():\n",
            "    textbutton _(\"Start\"):\n",
            "        action Start()"
        ),
    );
}

#[test]
fn formats_nested_screen_displayables_and_use() {
    assert_formats(
        concat!(
            "screen nav():\n",
            "    viewport:\n",
            "        vbox:\n",
            "            use navigation"
        ),
        concat!(
            "screen nav():\n",
            "    viewport:\n",
            "        vbox:\n",
            "            use navigation"
        ),
    );
}

#[test]
fn formats_screen_conditionals_and_loops() {
    assert_formats(
        concat!(
            "screen listing(slots):\n",
            "    if persistent.foo:\n",
            "        text \"Yes\"\n",
            "    else:\n",
            "        pass\n",
            "    for slot in slots:\n",
            "        text slot"
        ),
        concat!(
            "screen listing(slots):\n",
            "    if persistent.foo:\n",
            "        text \"Yes\"\n",
            "    else:\n",
            "        pass\n",
            "    for slot in slots:\n",
            "        text slot"
        ),
    );
}

#[test]
fn formats_screen_python_and_transclude() {
    assert_formats(
        concat!(
            "screen tools():\n",
            "    $ count=count+1\n",
            "    python:\n",
            "        total=[1,2,3]\n",
            "    use panel:\n",
            "        transclude"
        ),
        concat!(
            "screen tools():\n",
            "    $ count=count+1\n",
            "    python:\n",
            "        total = [1, 2, 3]\n",
            "    use panel:\n",
            "        transclude"
        ),
    );
}

#[test]
fn formats_screen_fixture_from_reference_style() {
    assert_formats(
        concat!(
            "screen quick_menu():\n",
            "    hbox:\n",
            "        textbutton _(\"Back\") action Rollback()\n",
            "        textbutton _(\"Skip\") action Skip() alternate Skip(fast=True, confirm=True)\n"
        ),
        concat!(
            "screen quick_menu():\n",
            "    hbox:\n",
            "        textbutton _(\"Back\"):\n",
            "            action Rollback()\n",
            "        textbutton _(\"Skip\"):\n",
            "            action Skip()\n",
            "            alternate Skip(fast=True, confirm=True)"
        ),
    );
}

#[test]
fn keeps_jump_expression_targets_unqualified() {
    assert_formats(
        concat!(
            "label scenario_entry_point:\n",
            "    jump expression scenario.label"
        ),
        concat!(
            "label scenario_entry_point:\n",
            "    jump expression scenario.label"
        ),
    );
}

#[test]
fn separates_block_statements_from_neighbors() {
    assert_formats(
        concat!(
            "define foo = 1\n",
            "label start:\n",
            "    $ x = 1\n",
            "    if True:\n",
            "        pass\n",
            "    $ y = 2\n",
            "define bar = 2"
        ),
        concat!(
            "define foo = 1\n",
            "\n",
            "label start:\n",
            "    $ x = 1\n",
            "\n",
            "    if True:\n",
            "        pass\n",
            "\n",
            "    $ y = 2\n",
            "\n",
            "define bar = 2"
        ),
    );
}

#[test]
fn no_trailing_whitespace_on_lines() {
    let formatted = format_script(concat!(
        "\"hello\"\n",
        "python:\n",
        "    x = 1\n",
        "    y = 2\n",
        "$ z = 3  \n",
        "\"world\""
    ));
    for (i, line) in formatted.lines().enumerate() {
        assert_eq!(
            line,
            line.trim_end(),
            "trailing whitespace on line {}: {:?}",
            i + 1,
            line
        );
    }
}

#[test]
fn show_scene_hide_with_clause_on_same_line() {
    assert_formats(
        concat!(
            "show eileen happy with dissolve\n",
            "scene bg room with fade\n",
            "hide eileen\n",
            "with dissolve"
        ),
        concat!(
            "show eileen happy with dissolve\n",
            "\n",
            "scene bg room with fade\n",
            "hide eileen\n",
            "with dissolve"
        ),
    );
}

#[test]
fn show_expression_with_tag_and_atl_preserves_expression_form() {
    assert_formats(
        concat!(
            "show expression alien_particles(400, 250, 700) as particles:\n",
            "    ypos 1.15"
        ),
        concat!(
            "show expression alien_particles(400, 250, 700) as particles:\n",
            "    ypos 1.15"
        ),
    );
}

#[test]
fn keeps_comments_inside_atl_blocks() {
    let mut statement = RawMultipurpose::new((PathBuf::from("test.rpy"), 4));
    statement.warper = Some("ease".into());
    statement.duration = Some("0.5".into());
    statement.properties = vec![("zoom".into(), "2.0".into())];

    let ast = vec![AstNode::Label(Label {
        loc: (PathBuf::from("test.rpy"), 1),
        name: "test".into(),
        block: vec![AstNode::Show(Show {
            loc: (PathBuf::from("test.rpy"), 2),
            imspec: Some(image(&["image1"])),
            atl: Some(RawBlock {
                loc: (PathBuf::from("test.rpy"), 3),
                statements: vec![Some(AtlStatement::RawMultipurpose(statement))],
                ..Default::default()
            }),
            ..Default::default()
        })],
        ..Default::default()
    })];

    let comments: CommentMap = BTreeMap::from([(
        4,
        vec![Comment::Standalone {
            indent: 8,
            text: "# comment".into(),
            line_number: 3,
        }],
    )]);

    assert_eq!(
        format_ast(&ast, &comments),
        concat!(
            "label test:\n",
            "    show image1:\n",
            "        # comment\n",
            "        ease 0.5 zoom 2.0"
        )
    );
}

#[test]
fn ungrouped_with_stays_on_own_line() {
    assert_formats(
        concat!("show image1\n", "show image2\n", "with exchange"),
        concat!("show image1\n", "show image2\n", "with exchange"),
    );
}

#[test]
fn standalone_with_on_own_line() {
    assert_formats("\"hello\"\nwith dissolve", "\"hello\" with dissolve");
}

#[test]
fn formats_translate_and_raw_block_statements() {
    assert_formats(
        concat!(
            "translate None strings:\n",
            "    old \"Hello\"\n",
            "    new \"Hi\"\n",
            "\n",
            "translate english start:\n",
            "    pass\n",
            "\n",
            "translate french python:\n",
            "    count = 3\n",
            "\n",
            "testcase foo.bar:\n",
            "    assert x\n",
            "\n",
            "testsuite foo.bar:\n",
            "    testcase nested:\n",
            "        assert y"
        ),
        concat!(
            "translate None strings:\n",
            "    old \"Hello\"\n",
            "    new \"Hi\"\n",
            "\n",
            "translate english start:\n",
            "    pass\n",
            "\n",
            "translate french python:\n",
            "    count = 3\n",
            "\n",
            "testcase foo.bar:\n",
            "    assert x\n",
            "\n",
            "testsuite foo.bar:\n",
            "    testcase nested:\n",
            "        assert y"
        ),
    );
}

#[test]
fn formats_generic_translate_block_statement() {
    let ast = vec![AstNode::TranslateBlock(TranslateBlock {
        language: Some("english".into()),
        block: vec![AstNode::Label(Label {
            name: "nested".into(),
            block: vec![],
            ..Default::default()
        })],
        ..Default::default()
    })];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        "translate english:\n    label nested:"
    );
}

#[test]
fn standalone_comment_before_statement() {
    use crate::comments::Comment;

    let mut comments = CommentMap::new();
    comments.insert(
        1,
        vec![Comment::Standalone {
            indent: 0,
            text: "# This is a comment".into(),
            line_number: 1,
        }],
    );

    let ast = vec![AstNode::Label(Label {
        loc: (PathBuf::from("test.rpy"), 1),
        name: "start".into(),
        block: vec![AstNode::Say(Say {
            loc: (PathBuf::from("test.rpy"), 2),
            what: "Hello".into(),
            interact: true,
            ..Default::default()
        })],
        ..Default::default()
    })];

    let result = format_ast(&ast, &comments);
    assert_eq!(result, "# This is a comment\nlabel start:\n    \"Hello\"");
}

#[test]
fn trailing_comment_on_statement() {
    use crate::comments::Comment;

    let mut comments = CommentMap::new();
    comments.insert(
        1,
        vec![Comment::Trailing {
            text: "# important".into(),
            line_number: 1,
        }],
    );

    let ast = vec![AstNode::Label(Label {
        loc: (PathBuf::from("test.rpy"), 1),
        name: "start".into(),
        block: vec![],
        ..Default::default()
    })];

    let result = format_ast(&ast, &comments);
    assert_eq!(result, "label start:  # important");
}

#[test]
fn multiple_standalone_comments_before_statement() {
    use crate::comments::Comment;

    let mut comments = CommentMap::new();
    comments.insert(
        1,
        vec![
            Comment::Standalone {
                indent: 0,
                text: "# Comment one".into(),
                line_number: 1,
            },
            Comment::Standalone {
                indent: 0,
                text: "# Comment two".into(),
                line_number: 2,
            },
        ],
    );

    let ast = vec![AstNode::Label(Label {
        loc: (PathBuf::from("test.rpy"), 1),
        name: "start".into(),
        block: vec![],
        ..Default::default()
    })];

    let result = format_ast(&ast, &comments);
    assert_eq!(result, "# Comment one\n# Comment two\nlabel start:");
}
