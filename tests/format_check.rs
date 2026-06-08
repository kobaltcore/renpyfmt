use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn create_temp_test_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "renpyfmt-cli-{name}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn renpyfmt_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_renpyfmt"))
}

fn run_with_stdin(mut command: Command, stdin: &str) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn write_script(root: &Path, contents: &str) -> PathBuf {
    let script_path = root.join("script.rpy");
    fs::write(&script_path, contents).unwrap();
    script_path
}

fn write_file(root: &Path, name: &str, contents: &str) -> PathBuf {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn format_file_formats_one_file_in_place() {
    let root = create_temp_test_dir("format-one-file");
    let script_path = write_script(&root, "python:\n    message='hi'\n");

    let output = renpyfmt_command()
        .arg("format")
        .arg(&script_path)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(&script_path).unwrap(),
        "python:\n    message = \"hi\"\n"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn format_accepts_multiple_files_and_directories() {
    let root = create_temp_test_dir("format-multi-input");
    let dir = root.join("dir");
    let nested_dir = root.join("nested_dir");
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&nested_dir).unwrap();

    let dir_rpy = write_file(&dir, "scene.rpy", "python:\n    message='hi'\n");
    let standalone_py = write_file(&root, "standalone.py", "x=[1,2]\n");
    let nested_py = write_file(&nested_dir, "nested.py", "y=[3,4]\n");
    write_file(&dir, "notes.txt", "ignored\n");

    let output = renpyfmt_command()
        .arg("format")
        .arg(&dir)
        .arg(&standalone_py)
        .arg(&nested_dir)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(&dir_rpy).unwrap(),
        "python:\n    message = \"hi\"\n"
    );
    assert_eq!(fs::read_to_string(&standalone_py).unwrap(), "x = [1, 2]\n");
    assert_eq!(fs::read_to_string(&nested_py).unwrap(), "y = [3, 4]\n");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn check_accepts_multiple_files_and_directories() {
    let root = create_temp_test_dir("check-multi-input");
    let dir = root.join("dir");
    let nested_dir = root.join("nested_dir");
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&nested_dir).unwrap();

    write_file(&dir, "scene.rpy", "python:\n    message='hi'\n");
    let standalone_py = write_file(&root, "standalone.py", "x = [1, 2]\n");
    write_file(&nested_dir, "nested.py", "y=[3,4]\n");

    let output = renpyfmt_command()
        .arg("check")
        .arg(&dir)
        .arg(&standalone_py)
        .arg(&nested_dir)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Would reformat"));
    assert!(stdout.contains("Checked 3 file(s): 2 would change, 1 already formatted, 0 failed"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn format_stdin_writes_rpy_output_to_stdout() {
    let root = create_temp_test_dir("format-stdin-rpy");
    let output = run_with_stdin(
        {
            let mut command = renpyfmt_command();
            command
                .arg("format")
                .arg("-")
                .arg("--stdin-filename")
                .arg(root.join("script.rpy"));
            command
        },
        "python:\n    message='hi'\n",
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "python:\n    message = \"hi\"\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn check_stdin_exits_one_for_dirty_input_and_zero_for_clean_input() {
    let root = create_temp_test_dir("check-stdin-rpy");
    let stdin_filename = root.join("script.rpy");

    let dirty = run_with_stdin(
        {
            let mut command = renpyfmt_command();
            command
                .arg("check")
                .arg("-")
                .arg("--stdin-filename")
                .arg(&stdin_filename);
            command
        },
        "python:\n    message='hi'\n",
    );
    assert_eq!(dirty.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&dirty.stdout).is_empty());
    assert!(String::from_utf8_lossy(&dirty.stderr).contains("Would reformat stdin"));

    let clean = run_with_stdin(
        {
            let mut command = renpyfmt_command();
            command
                .arg("check")
                .arg("-")
                .arg("--stdin-filename")
                .arg(&stdin_filename);
            command
        },
        "python:\n    message = \"hi\"\n",
    );
    assert_eq!(clean.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&clean.stdout).is_empty());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn stdin_requires_stdin_filename() {
    let format_output = run_with_stdin(
        {
            let mut command = renpyfmt_command();
            command.arg("format").arg("-");
            command
        },
        "python:\n    message='hi'\n",
    );
    assert_eq!(format_output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&format_output.stderr).contains("--stdin-filename"));

    let check_output = run_with_stdin(
        {
            let mut command = renpyfmt_command();
            command.arg("check").arg("-");
            command
        },
        "python:\n    message='hi'\n",
    );
    assert_eq!(check_output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&check_output.stderr).contains("--stdin-filename"));
}

#[test]
fn stdin_cannot_be_combined_with_path_inputs() {
    let root = create_temp_test_dir("stdin-and-path");
    let file_path = write_script(&root, "python:\n    message='hi'\n");

    let format_output = run_with_stdin(
        {
            let mut command = renpyfmt_command();
            command
                .arg("format")
                .arg("-")
                .arg(&file_path)
                .arg("--stdin-filename")
                .arg("script.rpy");
            command
        },
        "python:\n    message='hi'\n",
    );
    assert_eq!(format_output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&format_output.stderr).contains("cannot be combined"));

    let check_output = run_with_stdin(
        {
            let mut command = renpyfmt_command();
            command
                .arg("check")
                .arg("-")
                .arg(&file_path)
                .arg("--stdin-filename")
                .arg("script.rpy");
            command
        },
        "python:\n    message='hi'\n",
    );
    assert_eq!(check_output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&check_output.stderr).contains("cannot be combined"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn unsupported_explicit_files_exit_with_error() {
    let root = create_temp_test_dir("unsupported-explicit-file");
    let text_path = write_file(&root, "notes.txt", "ignored\n");

    let output = renpyfmt_command()
        .arg("format")
        .arg(&text_path)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unsupported file type"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn single_file_config_discovery_uses_file_parent() {
    let root = create_temp_test_dir("single-file-config-parent");
    fs::write(
        root.join("ruff.toml"),
        "[format]\nquote-style = \"double\"\n",
    )
    .unwrap();

    let scripts = root.join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    fs::write(
        scripts.join("ruff.toml"),
        "[format]\nquote-style = \"single\"\n",
    )
    .unwrap();
    let script_path = write_file(&scripts, "script.rpy", "python:\n    message=\"hi\"\n");

    let output = renpyfmt_command()
        .current_dir(&root)
        .arg("format")
        .arg(&script_path)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(&script_path).unwrap(),
        "python:\n    message = 'hi'\n"
    );

    let _ = fs::remove_dir_all(&root);
}
