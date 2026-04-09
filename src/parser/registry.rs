use crate::{
    ast::{
        Call, Camera, CompileIf, Default_, Define, Hide, If, Image, Init, Jump, Label, Menu, Pass,
        Python, PythonOneLine, Return, Say, Scene, Screen, Show, Style, Testcase, Testsuite,
        Transform, Translate, UserStatement, While, With, RPY,
    },
    trie::ParseTrie,
};

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
    parser.add(vec!["rpy".into()], Box::new(RPY::default()));
    parser.add(vec!["translate".into()], Box::new(Translate::default()));
    parser.add(vec!["testcase".into()], Box::new(Testcase::default()));
    parser.add(vec!["testsuite".into()], Box::new(Testsuite::default()));

    let custom_statements = vec![
        "play music",
        "queue music",
        "stop music",
        "play sound",
        "queue sound",
        "stop sound",
        "play",
        "queue",
        "stop",
        "pause",
        "show screen",
        "call screen",
        "hide screen",
        "nvl show",
        "nvl hide",
        "nvl clear",
        "window show",
        "window hide",
        "window auto",
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
