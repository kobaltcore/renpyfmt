use crate::comments::{Comment, CommentMap, EOF_LINE};
use crate::parser::parse_block;
use crate::{
    ast::AstNode,
    formatter::{PythonFormatConfig, format_ast_with_config_owned, format_python_file},
    lexer::{Block, Lexer},
};
use anyhow::{Context, Result, bail};
use indicatif::ProgressBar;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use ruff_workspace::Settings;
use ruff_workspace::configuration::Configuration;
use ruff_workspace::pyproject::{
    find_fallback_target_version, find_settings_toml, find_user_settings_toml,
};
use ruff_workspace::resolver::{
    ConfigurationOrigin, ConfigurationTransformer, resolve_root_settings,
};

#[derive(Clone, Debug)]
struct FormatContext {
    python_format_config: PythonFormatConfig,
}

struct NoOpTransformer;

impl ConfigurationTransformer for NoOpTransformer {
    fn transform(&self, config: Configuration) -> Configuration {
        config
    }
}

struct LexerContext {
    input_dir: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatMode {
    Write,
    Check,
}

#[derive(Debug, Default)]
pub struct FormatReport {
    pub unchanged_count: usize,
    pub changed_count: usize,
    pub failed_count: usize,
    pub changed_files: Vec<PathBuf>,
}

impl FormatReport {
    pub fn has_changes(&self) -> bool {
        self.changed_count > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileFormatOutcome {
    Unchanged,
    Changed,
}

fn ren_py_to_rpy(data: &str, filename: Option<&PathBuf>) -> Result<String> {
    let mut result = String::with_capacity(data.len());
    let mut prefix_len = 0usize;

    // IGNORE = 0
    // RENPY = 1
    // PYTHON = 2
    let mut state = 0;
    let mut open_linenumber = 0;

    for (line_num, line) in data.lines().enumerate() {
        if state != 1 {
            if line.starts_with("\"\"\"renpy") {
                state = 1;
                result.push('\n');
                open_linenumber = line_num;
                continue;
            }
        }
        if state == 1 {
            if line == "\"\"\"" {
                state = 2;
                result.push('\n');
                continue;
            }

            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                result.push_str(line);
                result.push('\n');
                continue;
            }

            if line_trimmed.starts_with('#') {
                result.push_str(line);
                result.push('\n');
                continue;
            }

            prefix_len = line.len() - line.trim_start_matches(' ').len();

            if line_trimmed.ends_with(':') {
                prefix_len += 4;
            }

            result.push_str(line);
            result.push('\n');
            continue;
        }
        if state == 2 {
            result.push_str(&" ".repeat(prefix_len));
            result.push_str(line);
            result.push('\n');
            continue;
        }
        if state == 0 {
            result.push('\n');
            continue;
        }
    }

    match filename {
        Some(path) => {
            if state == 0 {
                bail!(
                    "In {}, there are no \"\"\"renpy blocks, so every line is ignored.",
                    path.display()
                )
            } else if state == 1 {
                bail!(
                    "In {}, there is an open \"\"\"renpy block at line {} that is not terminated by \"\"\".",
                    path.display(),
                    open_linenumber
                )
            }
        }
        None => {}
    }

    result.pop();
    Ok(result)
}

fn munge_filename(path: &PathBuf) -> Result<String> {
    let mut stem = String::from_utf8(path.file_stem().unwrap().to_str().unwrap().into()).unwrap();
    if stem.ends_with("_ren") && path.extension() == Some("py".as_ref()) {
        stem = stem.strip_suffix("_ren").unwrap().into();
    }

    stem = stem.replace(" ", "_");

    let result = stem
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => c.to_string(),
            _ => format!("0x{:x}", c as u32),
        })
        .collect::<Vec<_>>()
        .join("");

    Ok(format!("_m1_{result}__"))
}

fn elide_filename(ctx: &LexerContext, path: &PathBuf) -> Result<PathBuf> {
    Ok(pathdiff::diff_paths(path, ctx.input_dir.clone()).unwrap())
}

fn letterlike(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn match_logical_word(s: &str, pos: usize) -> (&str, bool, usize) {
    let bytes = s.as_bytes();
    let start = pos;
    let mut end = pos;
    let byte = bytes[pos];

    if byte == b' ' {
        end += 1;
        while end < bytes.len() && bytes[end] == b' ' {
            end += 1;
        }
    } else if byte.is_ascii_alphanumeric() || byte == b'_' {
        end += 1;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            end += 1;
        }
    } else {
        let mut chars = s[pos..].char_indices();
        let (_, first) = chars.next().expect("position must be at a char boundary");
        end += first.len_utf8();
        if letterlike(first) {
            for (offset, c) in chars {
                if !letterlike(c) {
                    end = pos + offset;
                    break;
                }
                end = pos + offset + c.len_utf8();
            }
        }
    }

    let word = &s[start..end];

    if (end - start) >= 3 && word.starts_with("__") {
        return (word, true, end);
    }

    (word, false, end)
}

fn list_logical_lines(
    ctx: &LexerContext,
    path: &PathBuf,
) -> Result<(Vec<(PathBuf, usize, String)>, CommentMap)> {
    let mut data = fs::read_to_string(path)?;
    let stem = path.file_stem().unwrap().to_str().unwrap();

    if stem.ends_with("_ren") && path.extension() == Some("py".as_ref()) {
        data = ren_py_to_rpy(&data, Some(path))?;
    }

    let path = elide_filename(ctx, path)?;
    let prefix = munge_filename(&path)?;

    data.push('\n');
    data.push('\n');

    let mut result: Vec<(PathBuf, usize, String)> = vec![];
    let mut comment_map: CommentMap = BTreeMap::new();
    let line_number = 1;
    let mut number = line_number;
    let mut pos = 0;

    let bytes = data.as_bytes();
    let data_len = bytes.len();

    if data.starts_with('\u{feff}') {
        pos = '\u{feff}'.len_utf8();
    }

    let mut start_number;
    let mut pending_standalone: Vec<Comment> = vec![];

    while pos < data_len {
        start_number = number;
        let mut line = String::new();
        let mut parendepth = 0;
        let mut trailing_comment: Option<String> = None;

        while pos < data_len {
            let startpos = pos;
            let c = bytes[pos];

            if c == b'\t' {
                bail!(
                    "Tab characters are not allowed in Ren'Py scripts: {}:{}",
                    path.display(),
                    line_number
                )
            }

            if c == b'\n' && parendepth == 0 {
                if let Some(ref comment_text) = trailing_comment {
                    comment_map
                        .entry(start_number)
                        .or_insert_with(Vec::new)
                        .push(Comment::Trailing {
                            text: comment_text.clone(),
                            line_number: start_number,
                        });
                }

                let final_line = std::mem::take(&mut line);
                if !final_line.trim().is_empty() {
                    pending_standalone.iter().for_each(|sc| {
                        comment_map
                            .entry(start_number)
                            .or_insert_with(Vec::new)
                            .push(sc.clone());
                    });
                    if !pending_standalone.is_empty() {
                        pending_standalone.clear();
                    }
                    result.push((path.clone(), start_number, final_line));
                }

                pos += 1;
                number += 1;
                break;
            }

            if c == b'\n' {
                number += 1;
            }

            if c == b'\r' {
                pos += 1;
                continue;
            }

            if c == b'\\' && bytes[pos + 1] == b'\n' {
                pos += 2;
                number += 1;
                line.push('\\');
                line.push('\n');
                continue;
            }

            if matches!(c, b'(' | b'[' | b'{') {
                parendepth += 1;
            }

            if matches!(c, b')' | b']' | b'}') && parendepth > 0 {
                parendepth -= 1;
            }

            if c == b'#' {
                let comment_start = pos;
                while pos < data_len && bytes[pos] != b'\n' {
                    pos += 1;
                }
                let comment_text = data[comment_start..pos].to_string();

                if line.trim().is_empty() && parendepth == 0 {
                    pending_standalone.push(Comment::Standalone {
                        indent: line.len() - line.trim_start().len(),
                        text: comment_text,
                        line_number: start_number,
                    });
                } else {
                    trailing_comment = Some(comment_text);
                }

                continue;
            }

            if matches!(c, b'"' | b'\'' | b'`') {
                let delim = c;
                line.push(delim as char);
                pos += 1;

                let mut escape = false;
                let mut triple_quote = false;

                if (pos < data_len - 1) && bytes[pos] == delim && bytes[pos + 1] == delim {
                    line.push(delim as char);
                    line.push(delim as char);
                    pos += 2;
                    triple_quote = true;
                }

                let string_start = pos;

                while pos < data_len {
                    let c = bytes[pos];

                    if c == b'\n' {
                        number += 1;
                    }

                    if c == b'\r' {
                        pos += 1;
                        continue;
                    }

                    if escape {
                        escape = false;
                        pos += 1;
                        continue;
                    }

                    if c == delim {
                        if !triple_quote {
                            pos += 1;
                            break;
                        }

                        if (pos < data_len - 2)
                            && bytes[pos + 1] == delim
                            && bytes[pos + 2] == delim
                        {
                            pos += 3;
                            break;
                        }
                    }

                    if c == b'\\' {
                        escape = true;
                    }

                    pos += 1;
                }

                let s = &data[string_start..pos];

                if s.contains("[__") {
                    // TODO: munge substitutions
                }

                line.push_str(s);

                continue;
            }

            let (word, magic, end) = match_logical_word(&data, pos);

            if magic {
                let rest = &word[2..];

                if !rest.contains("__") {
                    line.push_str(&prefix);
                    line.push_str(rest);
                    pos = end;
                    continue;
                }
            }

            line.push_str(word);

            pos = end;

            if (pos - startpos) > 65536 {
                bail!(
                    "Overly long logical line. (Check strings and parenthesis): {}:{}",
                    path.display(),
                    line_number,
                )
            }
        }

        if !line.is_empty() {
            bail!(
                "Line is not terminated with a newline. (Check strings and parenthesis): {}:{}",
                path.display(),
                line_number,
            )
        }
    }

    if !pending_standalone.is_empty() {
        comment_map
            .entry(EOF_LINE)
            .or_insert_with(Vec::new)
            .extend(pending_standalone);
    }

    Ok((result, comment_map))
}

fn depth_split(s: &str) -> (usize, &str) {
    let depth = s.len() - s.trim_start_matches(' ').len();
    (depth, &s[depth..])
}

fn gll_core(
    lines: &[(PathBuf, usize, String)],
    i: usize,
    min_depth: usize,
) -> Result<(Vec<Block>, usize)> {
    let mut idx = i;
    let mut result = vec![];
    let mut depth: Option<usize> = None;

    while idx < lines.len() {
        let (filename, number, text) = &lines[idx];

        let (line_depth, rest) = depth_split(text);

        if line_depth < min_depth {
            break;
        }

        if depth.is_none() {
            depth = Some(line_depth);
        }

        if depth.unwrap() != line_depth {
            bail!("Indentation mismatch: {}:{}", filename.display(), number)
        }

        idx += 1;

        let (block, next_idx) = gll_core(lines, idx, depth.unwrap() + 1)?;
        idx = next_idx;

        result.push(Block {
            filename: filename.clone(),
            number: *number,
            text: rest.to_string(),
            block,
        });
    }

    Ok((result, idx))
}

pub fn group_logical_lines(lines: Vec<(PathBuf, usize, String)>) -> Result<Vec<Block>> {
    if lines.is_empty() {
        return Ok(vec![]);
    }

    let (filename, number, text) = lines.first().unwrap();

    let (depth, _) = depth_split(text);
    if depth != 0 {
        bail!(
            "Unexpected indentation at start of file: {}:{}",
            filename.display(),
            number,
        )
    }

    let (block, _) = gll_core(&lines, 0, 0)?;

    Ok(block)
}

pub fn list_logical_lines_for_path(
    input_dir: &Path,
    input_file: &PathBuf,
) -> Result<(Vec<(PathBuf, usize, String)>, CommentMap)> {
    let ctx = LexerContext {
        input_dir: input_dir.to_path_buf(),
    };

    list_logical_lines(&ctx, input_file)
}

fn collect_rpy_files(path: &Path) -> Result<Vec<PathBuf>> {
    collect_files(path, |extension| extension == "rpy")
}

fn collect_format_files(path: &Path) -> Result<Vec<PathBuf>> {
    collect_files(path, |extension| extension == "rpy" || extension == "py")
}

fn collect_files(path: &Path, mut include: impl FnMut(&str) -> bool) -> Result<Vec<PathBuf>> {
    if !path.exists() {
        bail!("Directory does not exist: {}", path.display());
    }

    if !path.is_dir() {
        bail!("Path is not a directory: {}", path.display());
    }

    let mut files = Vec::new();

    for entry in WalkDir::new(path) {
        let entry = entry.with_context(|| format!("Failed to walk {}", path.display()))?;
        let entry_path = entry.path();

        if entry.file_type().is_file()
            && entry_path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(&mut include)
        {
            files.push(entry_path.to_path_buf());
        }
    }

    files.sort();

    Ok(files)
}

pub fn parse_file_ast(
    input_dir: &Path,
    input_file: &PathBuf,
) -> Result<(Vec<AstNode>, CommentMap)> {
    let (lines, comments) = list_logical_lines_for_path(input_dir, input_file)
        .with_context(|| format!("Failed to list logical lines for {}", input_file.display()))?;
    let nested = group_logical_lines(lines)
        .with_context(|| format!("Failed to group logical lines for {}", input_file.display()))?;

    let mut lex = Lexer::new(nested);
    let ast = parse_block(&mut lex)
        .with_context(|| format!("Failed to parse {}", input_file.display()))?;
    Ok((ast, comments))
}

fn parse_file(input_dir: &Path, input_file: &PathBuf) -> Result<()> {
    parse_file_ast(input_dir, input_file).map(|_| ())?;

    Ok(())
}

fn resolve_python_format_config(
    input_dir: &Path,
    ruff_config: Option<&Path>,
) -> Result<PythonFormatConfig> {
    let input_dir = fs::canonicalize(input_dir)
        .with_context(|| format!("Failed to resolve input directory {}", input_dir.display()))?;

    let settings = resolve_ruff_settings(&input_dir, ruff_config)?;

    Ok(PythonFormatConfig::new(input_dir, settings.formatter))
}

fn resolve_ruff_settings(input_dir: &Path, ruff_config: Option<&Path>) -> Result<Settings> {
    if let Some(ruff_config) = ruff_config {
        let ruff_config = fs::canonicalize(ruff_config).with_context(|| {
            format!(
                "Failed to resolve Ruff config path {}",
                ruff_config.display()
            )
        })?;

        return resolve_root_settings(
            &ruff_config,
            &NoOpTransformer,
            ConfigurationOrigin::UserSpecified,
        )
        .with_context(|| format!("Failed to load Ruff config {}", ruff_config.display()));
    }

    if let Some(ruff_config) = find_settings_toml(input_dir).with_context(|| {
        format!(
            "Failed to discover Ruff config from {}",
            input_dir.display()
        )
    })? {
        return resolve_root_settings(
            &ruff_config,
            &NoOpTransformer,
            ConfigurationOrigin::Ancestor,
        )
        .with_context(|| format!("Failed to load Ruff config {}", ruff_config.display()));
    }

    if let Some(ruff_config) = find_user_settings_toml() {
        return resolve_root_settings(
            &ruff_config,
            &NoOpTransformer,
            ConfigurationOrigin::UserSettings,
        )
        .with_context(|| format!("Failed to load Ruff config {}", ruff_config.display()));
    }

    let mut settings = Settings::default();
    if let Some(target_version) = find_fallback_target_version(input_dir) {
        settings.formatter.unresolved_target_version = target_version.into();
    }
    Ok(settings)
}

fn format_file(
    input_dir: &Path,
    input_file: &PathBuf,
    ctx: &FormatContext,
    mode: FormatMode,
) -> Result<FileFormatOutcome> {
    let existing = fs::read_to_string(input_file)
        .with_context(|| format!("Failed to read {}", input_file.display()))?;
    let output = format_file_source(input_dir, input_file, &ctx.python_format_config)?;

    if existing == output {
        return Ok(FileFormatOutcome::Unchanged);
    }

    if mode == FormatMode::Write {
        fs::write(input_file, output)
            .with_context(|| format!("Failed to write {}", input_file.display()))?;
    }

    Ok(FileFormatOutcome::Changed)
}

pub fn format_file_source(
    input_dir: &Path,
    input_file: &PathBuf,
    python_format_config: &PythonFormatConfig,
) -> Result<String> {
    let existing = fs::read_to_string(input_file)
        .with_context(|| format!("Failed to read {}", input_file.display()))?;
    let extension = input_file
        .extension()
        .and_then(|ext| ext.to_str())
        .with_context(|| format!("Unsupported file type for {}", input_file.display()))?;

    let formatted = match extension {
        "rpy" => {
            let (ast, comments) = parse_file_ast(input_dir, input_file)?;
            format_ast_with_config_owned(&ast, comments, python_format_config.clone())
        }
        "py" => format_python_file(&existing, python_format_config)
            .with_context(|| format!("Failed to format {}", input_file.display()))?,
        _ => bail!("Unsupported file type for {}", input_file.display()),
    };

    Ok(if formatted.is_empty() {
        String::new()
    } else {
        format!("{}\n", formatted.trim_end_matches('\n'))
    })
}

pub fn parse_directory(path: PathBuf, pb: ProgressBar) -> Result<()> {
    let files = collect_rpy_files(&path)?;

    if files.is_empty() {
        pb.finish_with_message("No .rpy files found");
        return Ok(());
    }

    pb.set_length(files.len() as u64);
    let mut success_count = 0;
    let mut failures = vec![];

    for input_file in &files {
        pb.set_message(input_file.display().to_string());
        match parse_file(&path, input_file) {
            Ok(()) => {
                success_count += 1;
            }
            Err(err) => failures.push((input_file.clone(), err)),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    for (path, err) in &failures {
        eprintln!("error: {}", path.display());
        eprintln!("{err:#}");
        eprintln!();
    }

    println!(
        "Parsed {} .rpy file(s): {} succeeded, {} failed",
        files.len(),
        success_count,
        failures.len()
    );

    if !failures.is_empty() {
        bail!("encountered parse errors")
    }

    Ok(())
}

pub fn format_directory(
    path: PathBuf,
    ruff_config: Option<PathBuf>,
    mode: FormatMode,
    pb: ProgressBar,
) -> Result<FormatReport> {
    let files = collect_format_files(&path)?;

    let ctx = FormatContext {
        python_format_config: resolve_python_format_config(&path, ruff_config.as_deref())?,
    };

    if files.is_empty() {
        pb.finish_with_message("No formattable files found");
        return Ok(FormatReport::default());
    }

    pb.set_length(files.len() as u64);
    let mut report = FormatReport::default();
    let mut failures = vec![];

    for input_file in &files {
        pb.set_message(input_file.display().to_string());
        match format_file(&path, input_file, &ctx, mode) {
            Ok(FileFormatOutcome::Changed) => {
                report.changed_count += 1;
                report.changed_files.push(input_file.clone());
            }
            Ok(FileFormatOutcome::Unchanged) => {
                report.unchanged_count += 1;
            }
            Err(err) => failures.push((input_file.clone(), err)),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    for (path, err) in &failures {
        eprintln!("error: {}", path.display());
        eprintln!("{err:#}");
        eprintln!();
    }

    report.failed_count = failures.len();

    if mode == FormatMode::Check {
        for path in &report.changed_files {
            println!("Would reformat {}", path.display());
        }
    }

    match mode {
        FormatMode::Write => {
            println!(
                "Formatted {} file(s): {} changed, {} unchanged, {} failed",
                files.len(),
                report.changed_count,
                report.unchanged_count,
                report.failed_count
            );
        }
        FormatMode::Check => {
            println!(
                "Checked {} file(s): {} would change, {} already formatted, {} failed",
                files.len(),
                report.changed_count,
                report.unchanged_count,
                report.failed_count
            );
        }
    }

    if !failures.is_empty() {
        bail!("encountered format errors")
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use std::path::PathBuf;

    use super::*;
    use crate::comments::Comment;

    #[test]
    fn test_filename_munge() {
        let result = munge_filename(&PathBuf::from("test/foo/bar_test -*.txt")).unwrap();
        assert_eq!(result, "_m1_bar_test_0x2d0x2a__")
    }

    #[test]
    fn test_list_logical_lines_captures_standalone_comment() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_standalone_comment.rpy");
        let content = "# This is a comment\nlabel start:\n    \"Hello\"\n";
        std::fs::write(&file_path, content).unwrap();

        let ctx = LexerContext {
            input_dir: dir.clone(),
        };
        let (lines, comments) = list_logical_lines(&ctx, &file_path).unwrap();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].2.contains("label start:"));

        // The standalone comment should be keyed to the line number of "label start:"
        let label_line_num = lines[0].1;
        assert!(comments.contains_key(&label_line_num));
        let comment_list = &comments[&label_line_num];
        assert_eq!(comment_list.len(), 1);
        assert!(
            matches!(&comment_list[0], Comment::Standalone { text, .. } if text == "# This is a comment")
        );

        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_list_logical_lines_captures_trailing_comment() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_trailing_comment.rpy");
        let content = "label start: # important\n    \"Hello\"\n";
        std::fs::write(&file_path, content).unwrap();

        let ctx = LexerContext {
            input_dir: dir.clone(),
        };
        let (lines, comments) = list_logical_lines(&ctx, &file_path).unwrap();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].2.contains("label start:"));

        let label_line_num = lines[0].1;
        assert!(comments.contains_key(&label_line_num));
        let comment_list = &comments[&label_line_num];
        assert_eq!(comment_list.len(), 1);
        assert!(
            matches!(&comment_list[0], Comment::Trailing { text, .. } if text == "# important")
        );

        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_list_logical_lines_captures_indented_standalone_comment() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_indented_comment.rpy");
        let content = "label start:\n    show eileen happy\n    # device sfx\n    \"Hello\"\n";
        std::fs::write(&file_path, content).unwrap();

        let ctx = LexerContext {
            input_dir: dir.clone(),
        };
        let (lines, comments) = list_logical_lines(&ctx, &file_path).unwrap();

        assert_eq!(lines.len(), 3);

        // Find the line with "Hello"
        let hello_line = lines.iter().find(|l| l.2.contains("Hello")).unwrap();
        let hello_line_num = hello_line.1;

        // The indented comment should be a Standalone comment keyed to the line of "Hello"
        assert!(comments.contains_key(&hello_line_num));
        let comment_list = &comments[&hello_line_num];
        assert_eq!(comment_list.len(), 1);
        assert!(
            matches!(&comment_list[0], Comment::Standalone { text, .. } if text == "# device sfx")
        );

        let _ = std::fs::remove_file(&file_path);
    }

    fn create_temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("renpyfmt-{name}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn bench_fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("benches")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn format_uses_ruff_config_discovered_from_cli_input_directory() {
        let root = create_temp_test_dir("ruff-root-discovery");
        let nested = root.join("game");
        std::fs::create_dir_all(&nested).unwrap();

        std::fs::write(
            root.join("ruff.toml"),
            "[format]\nquote-style = \"single\"\n",
        )
        .unwrap();
        std::fs::write(
            nested.join(".ruff.toml"),
            "[format]\nquote-style = \"double\"\n",
        )
        .unwrap();

        let script_path = nested.join("script.rpy");
        std::fs::write(&script_path, "python:\n    message=\"hi\"\n").unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(formatted, "python:\n    message = 'hi'\n");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn logical_lines_handle_crlf_escaped_newlines_unicode_and_magic_names() {
        let root = create_temp_test_dir("logical-lines-edge-cases");
        let script_path = root.join("unicode_script.rpy");
        std::fs::write(
            &script_path,
            concat!(
                "\u{feff}label café_start:\r\n",
                "    $ café_value = 1 + \\\n",
                "        2\r\n",
                "    \"[__voice]\"\r\n",
            ),
        )
        .unwrap();

        let (lines, _) = list_logical_lines_for_path(&root, &script_path).unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].2, "label café_start:");
        assert!(lines[1].2.contains("café_value = 1 + \\\n        2"));
        assert!(lines[2].2.contains("[__voice]"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn logical_lines_keep_eof_comments() {
        let root = create_temp_test_dir("logical-lines-eof-comment");
        let script_path = root.join("script.rpy");
        std::fs::write(
            &script_path,
            "label start:\n    \"Hello\"\n# trailing eof\n",
        )
        .unwrap();

        let (_, comments) = list_logical_lines_for_path(&root, &script_path).unwrap();
        assert!(matches!(
            comments.get(&EOF_LINE).and_then(|items| items.first()),
            Some(Comment::Standalone { text, .. }) if text == "# trailing eof"
        ));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn logical_lines_convert_ren_py_blocks() {
        let root = create_temp_test_dir("logical-lines-ren-py");
        let script_path = root.join("entry_ren.py");
        std::fs::write(
            &script_path,
            concat!(
                "\"\"\"renpy\n",
                "label start:\n",
                "    \"Hello\"\n",
                "\"\"\"\n",
                "print('ignored by parser path')\n",
            ),
        )
        .unwrap();

        let (lines, _) = list_logical_lines_for_path(&root, &script_path).unwrap();
        assert!(lines.len() >= 2);
        assert_eq!(lines[0].2, "label start:");
        assert_eq!(lines[1].2, "    \"Hello\"");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn benchmark_fixtures_format_stably() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("benches")
            .join("fixtures");

        for fixture in [
            "dialogue_heavy.rpy",
            "nested_control_flow.rpy",
            "screen_language.rpy",
            "atl_heavy.rpy",
            "embedded_python.rpy",
        ] {
            let path = bench_fixture(fixture);
            let config = resolve_python_format_config(&root, None).unwrap();
            let formatted = format_file_source(&root, &path, &config).unwrap();
            let expected = std::fs::read_to_string(&path).unwrap();
            assert_eq!(formatted, expected, "fixture {fixture} changed");
        }
    }

    #[test]
    fn format_can_use_explicit_ruff_config_override() {
        let root = create_temp_test_dir("ruff-explicit-config");
        std::fs::write(
            root.join("ruff.toml"),
            "[format]\nquote-style = \"single\"\n",
        )
        .unwrap();

        let explicit_config = root.join("explicit-ruff.toml");
        std::fs::write(&explicit_config, "[format]\nquote-style = \"double\"\n").unwrap();

        let script_path = root.join("script.rpy");
        std::fs::write(&script_path, "python:\n    message='hi'\n").unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, Some(&explicit_config))
                .unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(formatted, "python:\n    message = \"hi\"\n");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_keeps_python_block_comments_in_place() {
        let root = create_temp_test_dir("python-comments");
        let script_path = root.join("script.rpy");
        std::fs::write(
            &script_path,
            concat!(
                "label start:\n",
                "    python:\n",
                "        # before\n",
                "        value=1  # trailing\n",
                "\n",
                "        # after\n",
                "    \"done\"\n",
            ),
        )
        .unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(
            formatted,
            concat!(
                "label start:\n",
                "    python:\n",
                "        # before\n",
                "        value = 1  # trailing\n",
                "\n",
                "        # after\n",
                "\n",
                "    \"done\"\n",
            )
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_keeps_nested_python_block_comment_indentation() {
        let root = create_temp_test_dir("python-comment-indent");
        let script_path = root.join("script.rpy");
        std::fs::write(
            &script_path,
            concat!(
                "python:\n",
                "    if ready:\n",
                "        # nested\n",
                "        value=1\n",
            ),
        )
        .unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(
            formatted,
            concat!(
                "python:\n",
                "    if ready:\n",
                "        # nested\n",
                "        value = 1\n",
            )
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_keeps_nested_python_block_multiline_string_indentation() {
        let root = create_temp_test_dir("python-docstring-indent");
        let script_path = root.join("script.rpy");
        std::fs::write(
            &script_path,
            concat!(
                "label start:\n",
                "    python:\n",
                "        \"\"\"\n",
                "            Scenario Mode now uses a list of locations.\n",
                "            This allows an external scenario directory.\n",
                "            \"\"\"\n",
                "\n",
                "        def update_scenario_paths():\n",
                "            scenario_base_paths=[1,2]\n",
            ),
        )
        .unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(
            formatted,
            concat!(
                "label start:\n",
                "    python:\n",
                "        \"\"\"\n",
                "        Scenario Mode now uses a list of locations.\n",
                "        This allows an external scenario directory.\n",
                "        \"\"\"\n",
                "\n\n",
                "        def update_scenario_paths():\n",
                "            scenario_base_paths = [1, 2]\n",
            )
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_dedents_top_level_python_block_multiline_comment_after_import() {
        let root = create_temp_test_dir("python-top-level-docstring-indent");
        let script_path = root.join("script.rpy");
        std::fs::write(
            &script_path,
            concat!(
                "init -2000 python:\n",
                "    import re\n",
                "\n",
                "    \"\"\"\n",
                "        This defines a FontGroup which uses Ubuntu Regular by default\n",
                "        and falls back to Noto Sans for chinese characters.\n",
                "        This can be used anywhere a regular path to a font is otherwise used.\n",
                "            https://renpy.org/doc/html/text.html#FontGroup\n",
                "        \"\"\"\n",
                "    main_font_group = (\n",
                "        FontGroup()\n",
                "        .add(\"gui/fonts/Ubuntu-R.ttf\", 0x0020, 0x024F)\n",
                "        .add(\"gui/fonts/Ubuntu-R.ttf\", 0x2000, 0x23FF)\n",
                "        .add(\"gui/fonts/Ubuntu-R.ttf\", 0x0400, 0x04FF)\n",
                "        .add(\"gui/fonts/SourceHanSansCLite.ttf\", None, None)\n",
                "    )\n",
            ),
        )
        .unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(
            formatted,
            concat!(
                "init -2000 python:\n",
                "    import re\n",
                "\n",
                "    \"\"\"\n",
                "    This defines a FontGroup which uses Ubuntu Regular by default\n",
                "    and falls back to Noto Sans for chinese characters.\n",
                "    This can be used anywhere a regular path to a font is otherwise used.\n",
                "        https://renpy.org/doc/html/text.html#FontGroup\n",
                "    \"\"\"\n",
                "    main_font_group = (\n",
                "        FontGroup()\n",
                "        .add(\"gui/fonts/Ubuntu-R.ttf\", 0x0020, 0x024F)\n",
                "        .add(\"gui/fonts/Ubuntu-R.ttf\", 0x2000, 0x23FF)\n",
                "        .add(\"gui/fonts/Ubuntu-R.ttf\", 0x0400, 0x04FF)\n",
                "        .add(\"gui/fonts/SourceHanSansCLite.ttf\", None, None)\n",
                "    )\n",
            )
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_preserves_init_offset_comments_and_separation() {
        let root = create_temp_test_dir("init-offset-comments");
        let script_path = root.join("script.rpy");
        std::fs::write(
            &script_path,
            concat!(
                "################################################################################\n",
                "## Initialization\n",
                "################################################################################\n",
                "\n",
                "## The init offset statement causes the init code in this file to run before\n",
                "## init code in any other file.\n",
                "init offset = -2\n",
                "\n",
                "define gui.accent_color = '#9e2c94'\n",
            ),
        )
        .unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(
            formatted,
            concat!(
                "################################################################################\n",
                "## Initialization\n",
                "################################################################################\n",
                "## The init offset statement causes the init code in this file to run before\n",
                "## init code in any other file.\n",
                "init offset = -2\n",
                "define gui.accent_color = '#9e2c94'\n",
            )
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_preserves_inline_comments_on_default_statements() {
        let root = create_temp_test_dir("default-inline-comment");
        let script_path = root.join("script.rpy");
        std::fs::write(
            &script_path,
            "default eDay4Morph = \"john\"  # john, zoey, brad, rita\n",
        )
        .unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(
            formatted,
            "default eDay4Morph = \"john\"  # john, zoey, brad, rita\n"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_check_reports_changed_without_writing_file() {
        let root = create_temp_test_dir("format-check-no-write");
        let script_path = root.join("script.rpy");
        std::fs::write(&script_path, "python:\n    message='hi'\n").unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        let outcome = format_file(&root, &script_path, &ctx, FormatMode::Check).unwrap();

        assert_eq!(outcome, FileFormatOutcome::Changed);
        let existing = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(existing, "python:\n    message='hi'\n");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_check_reports_unchanged_for_formatted_file() {
        let root = create_temp_test_dir("format-check-clean");
        let script_path = root.join("script.rpy");
        std::fs::write(&script_path, "python:\n    message = \"hi\"\n").unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        let outcome = format_file(&root, &script_path, &ctx, FormatMode::Check).unwrap();

        assert_eq!(outcome, FileFormatOutcome::Unchanged);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn collect_format_files_includes_rpy_and_py_only() {
        let root = create_temp_test_dir("collect-format-files");
        std::fs::write(root.join("script.rpy"), "label start:\n    \"hi\"\n").unwrap();
        std::fs::write(root.join("normal.py"), "x=[1,2]\n").unwrap();
        std::fs::write(root.join("notes.txt"), "ignored\n").unwrap();

        let files = collect_format_files(&root).unwrap();

        assert_eq!(files, vec![root.join("normal.py"), root.join("script.rpy")]);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_python_file_formats_plain_python_source() {
        let root = create_temp_test_dir("format-python-file");
        let script_path = root.join("normal.py");
        std::fs::write(&script_path, "x=[1,2]\n").unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(formatted, "x = [1, 2]\n");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_ren_py_file_as_plain_python_without_renpy_extraction() {
        let root = create_temp_test_dir("format-ren-py-as-python");
        let script_path = root.join("store_ren.py");
        std::fs::write(
            &script_path,
            concat!(
                "\"\"\"renpy\n",
                "label start:\n",
                "    \"Hello\"\n",
                "\"\"\"\n",
                "x=[1,2]\n",
            ),
        )
        .unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(
            formatted,
            concat!(
                "\"\"\"renpy\n",
                "label start:\n",
                "    \"Hello\"\n",
                "\"\"\"\n",
                "\n",
                "x = [1, 2]\n",
            )
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_rpy_file_uses_existing_renpy_formatter_path() {
        let root = create_temp_test_dir("format-rpy-path");
        let script_path = root.join("script.rpy");
        std::fs::write(&script_path, "python:\n    x=[1,2]\n").unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        format_file(&root, &script_path, &ctx, FormatMode::Write).unwrap();

        let formatted = std::fs::read_to_string(&script_path).unwrap();
        assert_eq!(formatted, "python:\n    x = [1, 2]\n");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn format_check_reports_changed_for_dirty_python_without_writing() {
        let root = create_temp_test_dir("format-check-python-no-write");
        let script_path = root.join("normal.py");
        std::fs::write(&script_path, "x=[1,2]\n").unwrap();

        let ctx = FormatContext {
            python_format_config: resolve_python_format_config(&root, None).unwrap(),
        };
        let outcome = format_file(&root, &script_path, &ctx, FormatMode::Check).unwrap();

        assert_eq!(outcome, FileFormatOutcome::Changed);
        assert_eq!(std::fs::read_to_string(&script_path).unwrap(), "x=[1,2]\n");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn invalid_python_file_is_counted_as_format_failure() {
        let root = create_temp_test_dir("format-invalid-python");
        std::fs::write(root.join("broken.py"), "if True print('hi')\n").unwrap();

        let pb = ProgressBar::hidden();
        let err = format_directory(root.clone(), None, FormatMode::Check, pb).unwrap_err();

        assert!(err.to_string().contains("encountered format errors"));

        let _ = std::fs::remove_dir_all(&root);
    }
}
