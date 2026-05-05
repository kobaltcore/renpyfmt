use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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

fn write_script(root: &Path, contents: &str) -> PathBuf {
    let script_path = root.join("script.rpy");
    fs::write(&script_path, contents).unwrap();
    script_path
}

fn write_file(root: &Path, name: &str, contents: &str) -> PathBuf {
    let path = root.join(name);
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn format_check_exits_zero_when_already_formatted() {
    let root = create_temp_test_dir("check-clean");
    write_script(&root, "python:\n    message = \"hi\"\n");

    let output = renpyfmt_command()
        .arg("format")
        .arg("--check")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Checked 1 file(s): 0 would change, 1 already formatted, 0 failed"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn format_check_sees_both_rpy_and_py_files() {
    let root = create_temp_test_dir("check-mixed-types");
    write_script(&root, "python:\n    message = \"hi\"\n");
    write_file(&root, "normal.py", "x=[1,2]\n");

    let output = renpyfmt_command()
        .arg("format")
        .arg("--check")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Would reformat"));
    assert!(stdout.contains("Checked 2 file(s): 1 would change, 1 already formatted, 0 failed"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn format_check_dirty_python_exits_one_without_modifying_file() {
    let root = create_temp_test_dir("check-dirty-python");
    let script_path = write_file(&root, "normal.py", "x=[1,2]\n");

    let output = renpyfmt_command()
        .arg("format")
        .arg("--check")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Checked 1 file(s): 1 would change, 0 already formatted, 0 failed"));
    assert_eq!(fs::read_to_string(&script_path).unwrap(), "x=[1,2]\n");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn format_check_clean_python_counts_as_unchanged() {
    let root = create_temp_test_dir("check-clean-python");
    write_file(&root, "normal.py", "x = [1, 2]\n");

    let output = renpyfmt_command()
        .arg("format")
        .arg("--check")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Checked 1 file(s): 0 would change, 1 already formatted, 0 failed"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn format_check_exits_one_without_modifying_files_when_changes_are_needed() {
    let root = create_temp_test_dir("check-dirty");
    let script_path = write_script(&root, "python:\n    message='hi'\n");

    let output = renpyfmt_command()
        .arg("format")
        .arg("--check")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Would reformat"));
    assert_eq!(
        fs::read_to_string(&script_path).unwrap(),
        "python:\n    message='hi'\n"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn format_check_uses_error_exit_code_for_real_failures() {
    let root = create_temp_test_dir("check-error");
    write_script(&root, "python:\n    message='hi'\n");
    let missing_config = root.join("missing-ruff.toml");

    let output = renpyfmt_command()
        .arg("format")
        .arg("--check")
        .arg("--config")
        .arg(&missing_config)
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to resolve Ruff config path"));

    let _ = fs::remove_dir_all(&root);
}
