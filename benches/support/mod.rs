#![allow(dead_code)]

use renpyfmt::comments::CommentMap;
use renpyfmt::formatter::PythonFormatConfig;
use renpyfmt::project::{
    format_file_source, group_logical_lines, list_logical_lines_for_path, parse_file_ast,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const RPY_FIXTURES: &[&str] = &[
    "dialogue_heavy.rpy",
    "nested_control_flow.rpy",
    "screen_language.rpy",
    "atl_heavy.rpy",
    "embedded_python.rpy",
];

pub const LOGICAL_FIXTURES: &[&str] = &[
    "dialogue_heavy.rpy",
    "nested_control_flow.rpy",
    "screen_language.rpy",
    "atl_heavy.rpy",
    "embedded_python.rpy",
    "script_ren.py",
];

pub const PARSE_FIXTURES: &[&str] = &[
    "dialogue_heavy.rpy",
    "nested_control_flow.rpy",
    "screen_language.rpy",
    "atl_heavy.rpy",
    "embedded_python.rpy",
];

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benches/fixtures")
}

pub fn fixture_path(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

pub fn python_config() -> PythonFormatConfig {
    PythonFormatConfig::default()
}

pub fn parse_fixture(name: &str) -> (Vec<renpyfmt::ast::AstNode>, CommentMap) {
    parse_file_ast(&fixtures_dir(), &fixture_path(name)).expect("fixture should parse")
}

pub fn logical_lines_fixture(name: &str) -> (Vec<(PathBuf, usize, String)>, CommentMap) {
    list_logical_lines_for_path(&fixtures_dir(), &fixture_path(name)).expect("fixture should scan")
}

pub fn grouped_fixture(name: &str) -> Vec<renpyfmt::lexer::Block> {
    let (lines, _) = logical_lines_fixture(name);
    group_logical_lines(lines).expect("fixture should group")
}

pub fn create_temp_fixture_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("renpyfmt-bench-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir should be creatable");
    dir
}

pub fn copy_fixture(name: &str, dir: &Path) -> PathBuf {
    let source = fixture_path(name);
    let target = dir.join(name);
    fs::copy(&source, &target).expect("fixture should copy");
    target
}

pub fn format_fixture(name: &str) -> String {
    format_file_source(&fixtures_dir(), &fixture_path(name), &python_config())
        .expect("fixture should format")
}
