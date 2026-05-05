use crate::{
    ast::{
        Call, Camera, CompileIf, Default_, Define, Hide, If, Image, Init, Jump, Label,
        LayeredImage, Menu, Pass, Python, PythonOneLine, RPY, Return, Say, Scene, Screen, Show,
        Style, Testcase, Testsuite, Transform, Translate, UserStatement, While, With,
    },
    trie::ParseTrie,
};
use once_cell::sync::Lazy;

use super::statements_media::{
    AudioStatementParser, HideScreenStatementParser, PauseStatementParser, PlayLikeMode,
    ScreenStatementParser, StopAudioStatementParser, WindowAutoStatementParser,
    WindowStatementParser,
};
use crate::ast::{AudioTarget, ScreenStatementKind, WindowKind};

static PARSER: Lazy<ParseTrie> = Lazy::new(|| {
    let mut parser = ParseTrie::new();
    register_statements(&mut parser);
    parser
});

pub(super) fn parser() -> &'static ParseTrie {
    &PARSER
}

fn register_statements(parser: &mut ParseTrie) {
    parser.add(&["label"], Box::new(Label::default()));
    parser.add(&["scene"], Box::new(Scene::default()));
    parser.add(&["with"], Box::new(With::default()));
    parser.add(&[""], Box::new(Say::default()));
    parser.add(&["show"], Box::new(Show::default()));
    parser.add(&["hide"], Box::new(Hide::default()));
    parser.add(&["$"], Box::new(PythonOneLine::default()));
    parser.add(&["jump"], Box::new(Jump::default()));
    parser.add(&["menu"], Box::new(Menu::default()));
    parser.add(&["if"], Box::new(If::default()));
    parser.add(&["IF"], Box::new(CompileIf::default()));
    parser.add(&["while"], Box::new(While::default()));
    parser.add(&["return"], Box::new(Return::default()));
    parser.add(&["style"], Box::new(Style::default()));
    parser.add(&["init"], Box::new(Init::default()));
    parser.add(&["python"], Box::new(Python::default()));
    parser.add(&["define"], Box::new(Define::default()));
    parser.add(&["default"], Box::new(Default_::default()));
    parser.add(&["call"], Box::new(Call::default()));
    parser.add(&["pass"], Box::new(Pass::default()));
    parser.add(&["transform"], Box::new(Transform::default()));
    parser.add(&["camera"], Box::new(Camera::default()));
    parser.add(&["screen"], Box::new(Screen::default()));
    parser.add(&["image"], Box::new(Image::default()));
    parser.add(&["layeredimage"], Box::new(LayeredImage::default()));
    parser.add(&["rpy"], Box::new(RPY::default()));
    parser.add(&["translate"], Box::new(Translate::default()));
    parser.add(&["testcase"], Box::new(Testcase::default()));
    parser.add(&["testsuite"], Box::new(Testsuite::default()));

    parser.add(
        &["play", "music"],
        Box::new(AudioStatementParser {
            target: AudioTarget::Music,
            mode: PlayLikeMode::Play,
        }),
    );
    parser.add(
        &["queue", "music"],
        Box::new(AudioStatementParser {
            target: AudioTarget::Music,
            mode: PlayLikeMode::Queue,
        }),
    );
    parser.add(
        &["stop", "music"],
        Box::new(StopAudioStatementParser {
            target: AudioTarget::Music,
        }),
    );
    parser.add(
        &["play", "sound"],
        Box::new(AudioStatementParser {
            target: AudioTarget::Sound,
            mode: PlayLikeMode::Play,
        }),
    );
    parser.add(
        &["queue", "sound"],
        Box::new(AudioStatementParser {
            target: AudioTarget::Sound,
            mode: PlayLikeMode::Queue,
        }),
    );
    parser.add(
        &["stop", "sound"],
        Box::new(StopAudioStatementParser {
            target: AudioTarget::Sound,
        }),
    );
    parser.add(
        &["play"],
        Box::new(AudioStatementParser {
            target: AudioTarget::Generic(String::new()),
            mode: PlayLikeMode::Play,
        }),
    );
    parser.add(
        &["queue"],
        Box::new(AudioStatementParser {
            target: AudioTarget::Generic(String::new()),
            mode: PlayLikeMode::Queue,
        }),
    );
    parser.add(
        &["stop"],
        Box::new(StopAudioStatementParser {
            target: AudioTarget::Generic(String::new()),
        }),
    );
    parser.add(&["pause"], Box::new(PauseStatementParser));
    parser.add(
        &["show", "screen"],
        Box::new(ScreenStatementParser {
            kind: ScreenStatementKind::Show,
        }),
    );
    parser.add(
        &["call", "screen"],
        Box::new(ScreenStatementParser {
            kind: ScreenStatementKind::Call,
        }),
    );
    parser.add(&["hide", "screen"], Box::new(HideScreenStatementParser));
    parser.add(
        &["window", "show"],
        Box::new(WindowStatementParser {
            kind: WindowKind::Show,
        }),
    );
    parser.add(
        &["window", "hide"],
        Box::new(WindowStatementParser {
            kind: WindowKind::Hide,
        }),
    );
    parser.add(&["window", "auto"], Box::new(WindowAutoStatementParser));

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
            &stmt.split(' ').collect::<Vec<_>>(),
            Box::new(UserStatement::default()),
        );
    }
}
