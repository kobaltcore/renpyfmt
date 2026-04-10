use crate::{
    ast::{
        AstNode, Call, Camera, CompileIf, Default_, Define, EarlyPython, If, Image, ImageSpecifier,
        Init, Jump, Label, Menu, Pass, Python, Say, Scene, Show, ShowLayer, Style, Testcase,
        Testsuite, Transform, Translate, TranslateBlock, TranslateEarlyBlock, TranslateString,
        While, With,
    },
    atl::{
        AtlStatement, RawBlock, RawChoice, RawContainsExpr, RawEvent, RawFunction, RawMultipurpose,
        RawOn, RawParallel, RawRepeat, RawTime,
    },
    comments::CommentMap,
    formatter::format_ast,
    lexer::Block,
};
use std::path::PathBuf;

fn image(name: &[&str]) -> ImageSpecifier {
    ImageSpecifier {
        image_name: name.iter().map(|part| part.to_string()).collect(),
        ..Default::default()
    }
}

fn multipurpose(
    expressions: Vec<(&str, Option<&str>)>,
    properties: Vec<(&str, &str)>,
) -> RawMultipurpose {
    let mut node = RawMultipurpose::new((PathBuf::new(), 0));
    node.expressions = expressions
        .into_iter()
        .map(|(expr, with_clause)| (expr.into(), with_clause.map(Into::into)))
        .collect();
    node.properties = properties
        .into_iter()
        .map(|(name, expr)| (name.into(), expr.into()))
        .collect();
    node
}

#[test]
fn formats_label_block_without_embedded_extra_newlines() {
    let ast = vec![AstNode::Label(Label {
        name: "start".into(),
        block: vec![
            AstNode::Say(Say {
                who: Some("e".into()),
                what: "Hello".into(),
                interact: true,
                ..Default::default()
            }),
            AstNode::Jump(Jump {
                target: "next".into(),
                ..Default::default()
            }),
        ],
        ..Default::default()
    })];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        "label start:\n    e \"Hello\"\n\n    jump next"
    );
}

#[test]
fn formats_if_elif_else_blocks() {
    let ast = vec![AstNode::If(If {
        entries: vec![
            (
                Some("flag".into()),
                vec![AstNode::Say(Say {
                    what: "yes".into(),
                    interact: true,
                    ..Default::default()
                })],
            ),
            (
                Some("other".into()),
                vec![AstNode::Jump(Jump {
                    target: "other_label".into(),
                    ..Default::default()
                })],
            ),
            (
                None,
                vec![AstNode::Call(Call {
                    label: "fallback".into(),
                    ..Default::default()
                })],
            ),
        ],
        ..Default::default()
    })];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        concat!(
            "if flag:\n",
            "    \"yes\"\n",
            "\n",
            "elif other:\n",
            "    jump other_label\n",
            "else:\n",
            "    call fallback"
        )
    );
}

#[test]
fn formats_menu_with_caption_and_condition() {
    let ast = vec![AstNode::Menu(Menu {
        has_caption: true,
        items: vec![
            (Some("Caption".into()), None, None),
            (
                Some("Choice".into()),
                Some("seen".into()),
                Some(vec![AstNode::Jump(Jump {
                    target: "next".into(),
                    ..Default::default()
                })]),
            ),
        ],
        item_arguments: vec![None, None],
        ..Default::default()
    })];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        concat!(
            "menu:\n",
            "    \"Caption\"\n",
            "    \"Choice\" if seen:\n",
            "        jump next"
        )
    );
}

#[test]
fn formats_show_with_nested_atl() {
    let ast = vec![AstNode::Show(Show {
        imspec: Some(image(&["eileen"])),
        atl: Some(RawBlock {
            statements: vec![
                Some(AtlStatement::RawMultipurpose(multipurpose(
                    vec![("linear", None)],
                    vec![("xalign", "0.5")],
                ))),
                Some(AtlStatement::RawParallel(RawParallel {
                    block: RawBlock {
                        statements: vec![Some(AtlStatement::RawChoice(RawChoice {
                            chance: "0.5".into(),
                            block: RawBlock {
                                statements: vec![Some(AtlStatement::RawMultipurpose(
                                    multipurpose(vec![("pause", None)], vec![]),
                                ))],
                                ..Default::default()
                            },
                            ..Default::default()
                        }))],
                        ..Default::default()
                    },
                    ..Default::default()
                })),
            ],
            ..Default::default()
        }),
        ..Default::default()
    })];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        concat!(
            "show eileen:\n",
            "    linear xalign 0.5\n",
            "    parallel:\n",
            "        choice 0.5:\n",
            "            pause"
        )
    );
}

#[test]
fn inserts_blank_line_before_top_level_scene() {
    let ast = vec![
        AstNode::Say(Say {
            what: "hello".into(),
            interact: true,
            ..Default::default()
        }),
        AstNode::Scene(Scene {
            imspec: Some(image(&["bg", "room"])),
            ..Default::default()
        }),
    ];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        "\"hello\"\n\nscene bg room"
    );
}

#[test]
fn formats_supported_flow_and_init_statements() {
    let ast = vec![
        AstNode::Init(Init {
            priority: 5,
            block: vec![AstNode::Default(Default_ {
                store: "store.persistent".into(),
                name: "answer".into(),
                expr: Some("42".into()),
                ..Default::default()
            })],
            ..Default::default()
        }),
        AstNode::EarlyPython(EarlyPython {
            hide: true,
            store: "store.mystore".into(),
            python_code: "total = 1".into(),
            ..Default::default()
        }),
        AstNode::If(If {
            entries: vec![
                (
                    Some("seen_intro".into()),
                    vec![AstNode::Pass(Pass::default())],
                ),
                (
                    None,
                    vec![AstNode::While(While {
                        condition: "waiting".into(),
                        block: vec![AstNode::Pass(Pass::default())],
                        ..Default::default()
                    })],
                ),
            ],
            ..Default::default()
        }),
        AstNode::CompileIf(CompileIf {
            entries: vec![
                (
                    Some("renpy.version_tuple >= (8, 0)".into()),
                    vec![AstNode::Pass(Pass::default())],
                ),
                (None, vec![AstNode::Pass(Pass::default())]),
            ],
            ..Default::default()
        }),
    ];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        concat!(
            "init 5:\n",
            "    default persistent.answer = 42\n",
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
        )
    );
}

#[test]
fn formats_init_python_compact_form() {
    let ast = vec![
        AstNode::Init(Init {
            block: vec![AstNode::Python(Python {
                python_code: "x = 1".into(),
                ..Default::default()
            })],
            ..Default::default()
        }),
        AstNode::Init(Init {
            priority: 5,
            block: vec![AstNode::Python(Python {
                python_code: "y = 2".into(),
                hide: true,
                store: "store.mystore".into(),
                ..Default::default()
            })],
            ..Default::default()
        }),
    ];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        concat!(
            "init python:\n",
            "    x = 1\n",
            "init 5 python hide in mystore:\n",
            "    y = 2"
        )
    );
}

#[test]
fn formats_supported_media_and_atl_statement_variants() {
    let ast = vec![
        AstNode::ShowLayer(ShowLayer {
            layer: "overlay".into(),
            at_list: vec!["left".into()],
            atl: Some(RawBlock {
                animation: true,
                statements: vec![
                    Some(AtlStatement::RawContainsExpr(RawContainsExpr {
                        expr: "icon_idle".into(),
                        ..Default::default()
                    })),
                    Some(AtlStatement::RawOn(RawOn {
                        names: vec!["show".into(), "hide".into()],
                        block: RawBlock {
                            statements: vec![None],
                            ..Default::default()
                        },
                        ..Default::default()
                    })),
                    Some(AtlStatement::RawTime(RawTime {
                        time: "1.0".into(),
                        ..Default::default()
                    })),
                    Some(AtlStatement::RawFunction(RawFunction {
                        expr: "callback".into(),
                        ..Default::default()
                    })),
                    Some(AtlStatement::RawEvent(RawEvent {
                        name: "startled".into(),
                        ..Default::default()
                    })),
                    Some(AtlStatement::RawRepeat(RawRepeat {
                        repeats: Some("2".into()),
                        ..Default::default()
                    })),
                ],
                ..Default::default()
            }),
            ..Default::default()
        }),
        AstNode::Camera(Camera {
            at_list: vec!["wobble".into()],
            ..Default::default()
        }),
        AstNode::Init(Init {
            priority: 500,
            block: vec![AstNode::Image(Image {
                name: vec!["logo".into(), "idle".into()],
                expr: Some("\"room.webp\"".into()),
                ..Default::default()
            })],
            ..Default::default()
        }),
    ];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
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
            "camera at wobble\n",
            "image logo idle = \"room.webp\""
        )
    );
}

#[test]
fn formats_implicit_init_statements_without_init_blocks() {
    let ast = vec![
        AstNode::Init(Init {
            block: vec![AstNode::Style(Style {
                name: "nvl_window_badend".into(),
                parent: Some("nvl_window".into()),
                properties: [("background".into(), "None".into()), ("xpadding".into(), "40".into()), ("ypadding".into(), "40".into())]
                    .into_iter()
                    .collect(),
                ..Default::default()
            })],
            ..Default::default()
        }),
        AstNode::Init(Init {
            block: vec![AstNode::Define(Define {
                store: "store".into(),
                name: "badnar".into(),
                operator: "=".into(),
                expr: "Character(what_color='#ffffff', what_size=40, what_outlines=[(2, '#000000')], what_text_align=0.5, kind=nvl_narrator)".into(),
                ..Default::default()
            })],
            ..Default::default()
        }),
        AstNode::Init(Init {
            block: vec![AstNode::Transform(Transform {
                store: "store".into(),
                name: "wobble".into(),
                atl: Some(RawBlock {
                    statements: vec![Some(AtlStatement::RawMultipurpose(multipurpose(
                        vec![("linear", None)],
                        vec![("xalign", "0.5")],
                    )))],
                    ..Default::default()
                }),
                ..Default::default()
            })],
            ..Default::default()
        }),
        AstNode::Init(Init {
            priority: 500,
            block: vec![AstNode::Image(Image {
                name: vec!["bg".into(), "room".into()],
                expr: Some("\"room.webp\"".into()),
                ..Default::default()
            })],
            ..Default::default()
        }),
    ];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        concat!(
            "style nvl_window_badend is nvl_window:\n",
            "    background None\n",
            "    xpadding 40\n",
            "    ypadding 40\n",
            "define badnar = Character(what_color='#ffffff', what_size=40, what_outlines=[(2, '#000000')], what_text_align=0.5, kind=nvl_narrator)\n",
            "transform wobble:\n",
            "    linear xalign 0.5\n",
            "image bg room = \"room.webp\""
        )
    );
}

#[test]
fn keeps_explicit_init_blocks_for_non_default_priorities() {
    let ast = vec![AstNode::Init(Init {
        priority: 5,
        block: vec![AstNode::Define(Define {
            store: "store".into(),
            name: "foo".into(),
            operator: "=".into(),
            expr: "1".into(),
            ..Default::default()
        })],
        ..Default::default()
    })];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        "init 5:\n    define foo = 1"
    );
}

#[test]
fn no_trailing_whitespace_on_lines() {
    use crate::ast::{Python, PythonOneLine, Say};

    let ast = vec![
        AstNode::Say(Say {
            what: "hello".into(),
            interact: true,
            ..Default::default()
        }),
        AstNode::Python(Python {
            python_code: "x = 1\n\ny = 2".into(),
            ..Default::default()
        }),
        AstNode::PythonOneLine(PythonOneLine {
            python_code: "z = 3  ".into(),
            ..Default::default()
        }),
        AstNode::Say(Say {
            what: "world".into(),
            interact: true,
            ..Default::default()
        }),
    ];

    let formatted = format_ast(&ast, &CommentMap::new());
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
    use crate::ast::{Hide, With};

    let ast = vec![
        AstNode::With(With {
            loc: (PathBuf::from("test.rpy"), 1),
            expr: "None".into(),
            paired: Some("dissolve".into()),
        }),
        AstNode::Show(Show {
            imspec: Some(image(&["eileen", "happy"])),
            ..Default::default()
        }),
        AstNode::With(With {
            loc: (PathBuf::from("test.rpy"), 1),
            expr: "dissolve".into(),
            paired: None,
        }),
        AstNode::With(With {
            loc: (PathBuf::from("test.rpy"), 2),
            expr: "None".into(),
            paired: Some("fade".into()),
        }),
        AstNode::Scene(Scene {
            imspec: Some(image(&["bg", "room"])),
            ..Default::default()
        }),
        AstNode::With(With {
            loc: (PathBuf::from("test.rpy"), 2),
            expr: "fade".into(),
            paired: None,
        }),
        AstNode::Hide(Hide {
            loc: (PathBuf::from("test.rpy"), 3),
            imgspec: image(&["eileen"]),
        }),
        AstNode::With(With {
            loc: (PathBuf::from("test.rpy"), 3),
            expr: "dissolve".into(),
            paired: None,
        }),
    ];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        concat!(
            "show eileen happy with dissolve\n",
            "\n",
            "scene bg room with fade\n",
            "hide eileen\n",
            "with dissolve"
        )
    );
}

#[test]
fn show_expression_with_tag_and_atl_preserves_expression_form() {
    let ast = vec![AstNode::Show(Show {
        imspec: Some(ImageSpecifier {
            image_name: vec!["alien_particles(400, 250, 700)".into()],
            expression: Some("alien_particles(400, 250, 700)".into()),
            tag: Some("particles".into()),
            ..Default::default()
        }),
        atl: Some(RawBlock {
            statements: vec![Some(AtlStatement::RawMultipurpose(multipurpose(
                vec![("ypos 1.15", None)],
                vec![],
            )))],
            ..Default::default()
        }),
        ..Default::default()
    })];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        concat!(
            "show expression alien_particles(400, 250, 700) as particles:\n",
            "    ypos 1.15"
        )
    );
}

#[test]
fn ungrouped_with_stays_on_own_line() {
    let ast = vec![
        AstNode::Show(Show {
            imspec: Some(image(&["image1"])),
            ..Default::default()
        }),
        AstNode::Show(Show {
            imspec: Some(image(&["image2"])),
            ..Default::default()
        }),
        AstNode::With(With {
            loc: (PathBuf::from("test.rpy"), 1),
            expr: "exchange".into(),
            paired: None,
        }),
    ];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        concat!("show image1\n", "show image2\n", "with exchange")
    );
}

#[test]
fn standalone_with_on_own_line() {
    use crate::ast::{Say, With};

    let ast = vec![
        AstNode::Say(Say {
            what: "hello".into(),
            interact: true,
            ..Default::default()
        }),
        AstNode::With(With {
            loc: (PathBuf::from("test.rpy"), 1),
            expr: "dissolve".into(),
            paired: None,
        }),
    ];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        "\"hello\" with dissolve"
    );
}

#[test]
fn formats_translate_and_raw_block_statements() {
    let ast = vec![
        AstNode::Init(Init {
            block: vec![AstNode::TranslateString(TranslateString {
                old: "\"Hello\"".into(),
                new: "\"Hi\"".into(),
                ..Default::default()
            })],
            ..Default::default()
        }),
        AstNode::Translate(Translate {
            language: Some("english".into()),
            identifier: "start".into(),
            block: vec![AstNode::Pass(Pass::default())],
            ..Default::default()
        }),
        AstNode::TranslateBlock(TranslateBlock {
            language: Some("english".into()),
            block: vec![AstNode::Label(Label {
                name: "nested".into(),
                block: vec![],
                ..Default::default()
            })],
            ..Default::default()
        }),
        AstNode::TranslateEarlyBlock(TranslateEarlyBlock {
            language: Some("french".into()),
            block: vec![AstNode::Python(Python {
                python_code: "count = 3".into(),
                ..Default::default()
            })],
            ..Default::default()
        }),
        AstNode::Testcase(Testcase {
            name: "foo.bar".into(),
            block: vec![Block {
                filename: PathBuf::from("test.rpy"),
                number: 1,
                text: "assert x".into(),
                block: vec![],
            }],
            ..Default::default()
        }),
        AstNode::Testsuite(Testsuite {
            name: "foo.bar".into(),
            block: vec![Block {
                filename: PathBuf::from("test.rpy"),
                number: 1,
                text: "testcase nested:".into(),
                block: vec![Block {
                    filename: PathBuf::from("test.rpy"),
                    number: 2,
                    text: "assert y".into(),
                    block: vec![],
                }],
            }],
            ..Default::default()
        }),
    ];

    assert_eq!(
        format_ast(&ast, &CommentMap::new()),
        concat!(
            "translate None strings:\n",
            "    old \"Hello\"\n",
            "    new \"Hi\"\n",
            "translate english start:\n",
            "    pass\n",
            "translate english:\n",
            "    label nested:\n",
            "translate french python:\n",
            "    count = 3\n",
            "testcase foo.bar:\n",
            "    assert x\n",
            "testsuite foo.bar:\n",
            "    testcase nested:\n",
            "        assert y"
        )
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
