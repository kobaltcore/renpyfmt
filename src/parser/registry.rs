use crate::{
    ast::{
        Call, Camera, CompileIf, Default_, Define, Hide, If, Image, Init, Jump, Label,
        LayeredImage, Menu, Pass, Python, PythonOneLine, RPY, Return, Say, Scene, Screen, Show,
        Style, Testcase, Testsuite, Transform, Translate, UserStatement, While, With,
    },
    trie::ParseTrie,
};

use super::statements_media::{RegisteredStatement, RegisteredStatementKind};

pub(super) fn new_parser() -> ParseTrie {
    let mut parser = ParseTrie::new();
    register_statements(&mut parser);
    parser
}

fn register_statements(parser: &mut ParseTrie) {
    parser.add(vec!["label".into()], Box::new(Label::default()));
    parser.add(vec!["scene".into()], Box::new(Scene::default()));
    parser.add(vec!["with".into()], Box::new(With::default()));
    parser.add(vec!["".into()], Box::new(Say::default()));
    parser.add(vec!["show".into()], Box::new(Show::default()));
    parser.add(vec!["hide".into()], Box::new(Hide::default()));
    parser.add(vec!["$".into()], Box::new(PythonOneLine::default()));
    parser.add(vec!["jump".into()], Box::new(Jump::default()));
    parser.add(vec!["menu".into()], Box::new(Menu::default()));
    parser.add(vec!["if".into()], Box::new(If::default()));
    parser.add(vec!["IF".into()], Box::new(CompileIf::default()));
    parser.add(vec!["while".into()], Box::new(While::default()));
    parser.add(vec!["return".into()], Box::new(Return::default()));
    parser.add(vec!["style".into()], Box::new(Style::default()));
    parser.add(vec!["init".into()], Box::new(Init::default()));
    parser.add(vec!["python".into()], Box::new(Python::default()));
    parser.add(vec!["define".into()], Box::new(Define::default()));
    parser.add(vec!["default".into()], Box::new(Default_::default()));
    parser.add(vec!["call".into()], Box::new(Call::default()));
    parser.add(vec!["pass".into()], Box::new(Pass::default()));
    parser.add(vec!["transform".into()], Box::new(Transform::default()));
    parser.add(vec!["camera".into()], Box::new(Camera::default()));
    parser.add(vec!["screen".into()], Box::new(Screen::default()));
    parser.add(vec!["image".into()], Box::new(Image::default()));
    parser.add(vec!["layeredimage".into()], Box::new(LayeredImage::default()));
    parser.add(vec!["rpy".into()], Box::new(RPY::default()));
    parser.add(vec!["translate".into()], Box::new(Translate::default()));
    parser.add(vec!["testcase".into()], Box::new(Testcase::default()));
    parser.add(vec!["testsuite".into()], Box::new(Testsuite::default()));

    parser.add(
        vec!["play".into(), "music".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::PlayMusic)),
    );
    parser.add(
        vec!["queue".into(), "music".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::QueueMusic)),
    );
    parser.add(
        vec!["stop".into(), "music".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::StopMusic)),
    );
    parser.add(
        vec!["play".into(), "sound".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::PlaySound)),
    );
    parser.add(
        vec!["queue".into(), "sound".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::QueueSound)),
    );
    parser.add(
        vec!["stop".into(), "sound".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::StopSound)),
    );
    parser.add(
        vec!["play".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::Play)),
    );
    parser.add(
        vec!["queue".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::Queue)),
    );
    parser.add(
        vec!["stop".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::Stop)),
    );
    parser.add(
        vec!["pause".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::Pause)),
    );
    parser.add(
        vec!["show".into(), "screen".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::ShowScreen)),
    );
    parser.add(
        vec!["call".into(), "screen".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::CallScreen)),
    );
    parser.add(
        vec!["hide".into(), "screen".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::HideScreen)),
    );
    parser.add(
        vec!["window".into(), "show".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::WindowShow)),
    );
    parser.add(
        vec!["window".into(), "hide".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::WindowHide)),
    );
    parser.add(
        vec!["window".into(), "auto".into()],
        Box::new(RegisteredStatement::new(RegisteredStatementKind::WindowAuto)),
    );

    let custom_statements = vec![
        "nvl show",
        "nvl hide",
        "nvl clear",
        "resumeaudio",
        "pauseaudio",
        "timedchoice",
        "gameover",
        "text",
        "msg",
        "title",
        "outfit",
        "accessory",
        "body",
        "swap",
        "clone",
        "morph",
        "exspirit",
        "possess",
        "scry",
        "placeholder",
        "routename",
        "unlock",
        "resetstate",
        "FIXME",
        "phone_call",
    ];

    for stmt in custom_statements {
        parser.add(
            stmt.split(' ').map(|s| s.to_string()).collect(),
            Box::new(UserStatement::default()),
        );
    }
}
