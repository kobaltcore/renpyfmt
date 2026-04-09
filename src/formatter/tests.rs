use crate::{
    ast::{AstNode, Call, If, ImageSpecifier, Jump, Label, Menu, Say, Scene, Show},
    atl::{AtlStatement, RawBlock, RawChoice, RawMultipurpose, RawParallel},
    formatter::format_ast,
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
        format_ast(&ast),
        "label start:\n    e \"Hello\"\n    jump next"
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
        format_ast(&ast),
        concat!(
            "if flag:\n",
            "    \"yes\"\n",
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
        format_ast(&ast),
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
        format_ast(&ast),
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

    assert_eq!(format_ast(&ast), "\"hello\"\n\nscene bg room");
}
