use crate::{
    ast::{
        Call, Camera, CompileIf, Default_, Define, Hide, If, Image, Init, Jump, Label,
        LayeredImage, Menu, Pass, Python, PythonOneLine, RPY, Return, Say, Scene, Screen, Show,
        Style, Testcase, Testsuite, Transform, Translate, UserStatement, While, With,
    },
    trie::ParseTrie,
};

use super::statements_media::{
    AudioStatementParser, HideScreenStatementParser, PauseStatementParser, PlayLikeMode,
    ScreenStatementParser, StopAudioStatementParser, WindowAutoStatementParser,
    WindowStatementParser,
};
use crate::ast::{AudioTarget, ScreenStatementKind, WindowKind};

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
        Box::new(AudioStatementParser {
            target: AudioTarget::Music,
            mode: PlayLikeMode::Play,
        }),
    );
    parser.add(
        vec!["queue".into(), "music".into()],
        Box::new(AudioStatementParser {
            target: AudioTarget::Music,
            mode: PlayLikeMode::Queue,
        }),
    );
    parser.add(
        vec!["stop".into(), "music".into()],
        Box::new(StopAudioStatementParser {
            target: AudioTarget::Music,
        }),
    );
    parser.add(
        vec!["play".into(), "sound".into()],
        Box::new(AudioStatementParser {
            target: AudioTarget::Sound,
            mode: PlayLikeMode::Play,
        }),
    );
    parser.add(
        vec!["queue".into(), "sound".into()],
        Box::new(AudioStatementParser {
            target: AudioTarget::Sound,
            mode: PlayLikeMode::Queue,
        }),
    );
    parser.add(
        vec!["stop".into(), "sound".into()],
        Box::new(StopAudioStatementParser {
            target: AudioTarget::Sound,
        }),
    );
    parser.add(
        vec!["play".into()],
        Box::new(AudioStatementParser {
            target: AudioTarget::Generic(String::new()),
            mode: PlayLikeMode::Play,
        }),
    );
    parser.add(
        vec!["queue".into()],
        Box::new(AudioStatementParser {
            target: AudioTarget::Generic(String::new()),
            mode: PlayLikeMode::Queue,
        }),
    );
    parser.add(
        vec!["stop".into()],
        Box::new(StopAudioStatementParser {
            target: AudioTarget::Generic(String::new()),
        }),
    );
    parser.add(
        vec!["pause".into()],
        Box::new(PauseStatementParser),
    );
    parser.add(
        vec!["show".into(), "screen".into()],
        Box::new(ScreenStatementParser {
            kind: ScreenStatementKind::Show,
        }),
    );
    parser.add(
        vec!["call".into(), "screen".into()],
        Box::new(ScreenStatementParser {
            kind: ScreenStatementKind::Call,
        }),
    );
    parser.add(
        vec!["hide".into(), "screen".into()],
        Box::new(HideScreenStatementParser),
    );
    parser.add(
        vec!["window".into(), "show".into()],
        Box::new(WindowStatementParser {
            kind: WindowKind::Show,
        }),
    );
    parser.add(
        vec!["window".into(), "hide".into()],
        Box::new(WindowStatementParser {
            kind: WindowKind::Hide,
        }),
    );
    parser.add(
        vec!["window".into(), "auto".into()],
        Box::new(WindowAutoStatementParser),
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
