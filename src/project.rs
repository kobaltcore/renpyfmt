use crate::parser::parse_block;
use crate::{
    ast::AstNode,
    formatter::format_ast,
    lexer::{Block, Lexer},
};
use anyhow::{bail, Context, Result};
use indicatif::ProgressBar;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

struct LexerContext {
    input_dir: PathBuf,
}

fn ren_py_to_rpy(data: &str, filename: Option<&PathBuf>) -> Result<String> {
    let lines = data.lines().collect::<Vec<_>>();
    let mut result = vec![];
    let mut prefix = String::from("");

    // IGNORE = 0
    // RENPY = 1
    // PYTHON = 2
    let mut state = 0;
    let mut open_linenumber = 0;

    for (line_num, line) in lines.iter().enumerate() {
        if state != 1 {
            if line.starts_with("\"\"\"renpy") {
                state = 1;
                result.push("".into());
                open_linenumber = line_num;
                continue;
            }
        }
        if state == 1 {
            if *line == "\"\"\"" {
                state = 2;
                result.push("".into());
                continue;
            }

            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                result.push(line.to_string());
                continue;
            }

            if line_trimmed.starts_with('#') {
                result.push(line.to_string());
                continue;
            }

            prefix = "".into();
            for c in line.chars() {
                if c != ' ' {
                    break;
                }
                prefix = format!("{prefix} ");
            }

            if line_trimmed.ends_with(':') {
                prefix = format!("{prefix}    ");
            }

            result.push(line.to_string());
            continue;
        }
        if state == 2 {
            result.push(format!("{prefix}{line}"));
            continue;
        }
        if state == 0 {
            result.push("".into());
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

    Ok(result.join("\n"))
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
    match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => true,
        _ => false,
    }
}

fn match_logical_word(s: &Vec<char>, pos: usize) -> (String, bool, usize) {
    let mut pos = pos;
    let start = pos;
    let len_s = s.len();
    let c = s[pos];

    if c == ' ' {
        pos += 1;

        while pos < len_s {
            if s[pos] != ' ' {
                break;
            }

            pos += 1;
        }
    } else if letterlike(c) {
        pos += 1;

        while pos < len_s {
            if !letterlike(s[pos]) {
                break;
            }

            pos += 1;
        }
    } else {
        pos += 1;
    }

    let word = s[start..pos].iter().collect::<String>();

    if (pos - start) >= 3 && word.starts_with("__") {
        return (word, true, pos);
    }

    (word, false, pos)
}

fn list_logical_lines(ctx: &LexerContext, path: &PathBuf) -> Result<Vec<(PathBuf, usize, String)>> {
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
    let line_number = 1;
    let mut number = line_number;
    let mut pos = 0;

    let chars = data.chars().collect::<Vec<_>>();
    let data_len = chars.len();

    if data_len > 0 && chars[0] == '\u{feff}' {
        pos += 1;
    }

    let mut start_number;

    while pos < data_len {
        start_number = number;
        let mut line: Vec<String> = vec![];
        let mut parendepth = 0;
        let mut endpos: Option<usize> = None;

        while pos < data_len {
            let startpos = pos;
            let c = chars[pos];

            if c == '\t' {
                bail!(
                    "Tab characters are not allowed in Ren'Py scripts: {}:{}",
                    path.display(),
                    line_number
                )
            }

            if c == '\n' && parendepth == 0 {
                let final_line = line.join("");
                if !final_line.trim().is_empty() {
                    result.push((path.clone(), start_number, final_line));
                }

                if endpos.is_none() {
                    endpos = Some(pos);
                }

                while endpos > Some(0) && [' ', '\r'].contains(&chars[endpos.unwrap() - 1]) {
                    endpos = Some(endpos.unwrap() - 1);
                }

                pos += 1;
                number += 1;
                line.clear();
                break;
            }

            if c == '\n' {
                number += 1;
                endpos = None;
            }

            if c == '\r' {
                pos += 1;
                continue;
            }

            if c == '\\' && chars[pos + 1] == '\n' {
                pos += 2;
                number += 1;
                line.push("\\\n".into());
                continue;
            }

            if ['(', '[', '{'].contains(&c) {
                parendepth += 1;
            }

            if [')', ']', '}'].contains(&c) && parendepth > 0 {
                parendepth -= 1;
            }

            if c == '#' {
                endpos = Some(pos);
                while chars[pos] != '\n' {
                    pos += 1;
                }
                continue;
            }

            if ['\"', '\'', '`'].contains(&c) {
                let delim = c;
                line.push(c.into());
                pos += 1;

                let mut escape = false;
                let mut triple_quote = false;

                if (pos < data_len - 1) && chars[pos] == delim && chars[pos + 1] == delim {
                    line.push(delim.into());
                    line.push(delim.into());
                    pos += 2;
                    triple_quote = true;
                }

                let mut s: Vec<String> = vec![];

                while pos < data_len {
                    let c = chars[pos];

                    if c == '\n' {
                        number += 1;
                    }

                    if c == '\r' {
                        pos += 1;
                        continue;
                    }

                    if escape {
                        escape = false;
                        pos += 1;
                        s.push(c.into());
                        continue;
                    }

                    if c == delim {
                        if !triple_quote {
                            pos += 1;
                            s.push(c.into());
                            break;
                        }

                        if (pos < data_len - 2)
                            && chars[pos + 1] == delim
                            && chars[pos + 2] == delim
                        {
                            pos += 3;
                            s.push(delim.into());
                            s.push(delim.into());
                            s.push(delim.into());
                            break;
                        }
                    }

                    if c == '\\' {
                        escape = true;
                    }

                    s.push(c.into());
                    pos += 1;
                }

                let s = s.join("");

                if s.contains("[__") {
                    // TODO: munge substitutions
                }

                line.push(s);

                continue;
            }

            let (mut word, magic, end) = match_logical_word(&chars, pos);

            if magic {
                let rest = &word[2..];

                if !rest.contains("__") {
                    word = format!("{prefix}{rest}");
                }
            }

            line.push(word);

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

    Ok(result)
}

fn depth_split(s: String) -> Result<(usize, String)> {
    let mut depth = 0;
    let mut index = 0;

    let chars = s.chars().collect::<Vec<_>>();

    loop {
        if chars[index] == ' ' {
            depth += 1;
            index += 1;
            continue;
        }

        break;
    }

    Ok((depth, s[index..].into()))
}

fn gll_core(
    lines: &Vec<(PathBuf, usize, String)>,
    i: usize,
    min_depth: usize,
) -> Result<(Vec<Block>, usize)> {
    let mut idx = i;
    let mut result = vec![];
    let mut depth: Option<usize> = None;

    while idx < lines.len() {
        let (filename, number, text) = &lines[idx];

        let (line_depth, rest) = depth_split(text.clone())?;

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
            text: rest,
            block,
        });
    }

    Ok((result, idx))
}

fn group_logical_lines(lines: Vec<(PathBuf, usize, String)>) -> Result<Vec<Block>> {
    if lines.is_empty() {
        return Ok(vec![]);
    }

    let (filename, number, text) = lines.first().unwrap();

    let (depth, _) = depth_split(text.clone())?;
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

fn collect_rpy_files(path: &Path) -> Result<Vec<PathBuf>> {
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

        if entry.file_type().is_file() && entry_path.extension().is_some_and(|ext| ext == "rpy") {
            files.push(entry_path.to_path_buf());
        }
    }

    files.sort();

    Ok(files)
}

fn parse_file_ast(input_dir: &Path, input_file: &PathBuf) -> Result<Vec<AstNode>> {
    let ctx = LexerContext {
        input_dir: input_dir.to_path_buf(),
    };

    let lines = list_logical_lines(&ctx, input_file)
        .with_context(|| format!("Failed to list logical lines for {}", input_file.display()))?;
    let nested = group_logical_lines(lines)
        .with_context(|| format!("Failed to group logical lines for {}", input_file.display()))?;

    let mut lex = Lexer::new(nested);
    parse_block(&mut lex).with_context(|| format!("Failed to parse {}", input_file.display()))
}

fn parse_file(input_dir: &Path, input_file: &PathBuf) -> Result<()> {
    parse_file_ast(input_dir, input_file).map(|_| ())?;

    Ok(())
}

fn format_file(input_dir: &Path, input_file: &PathBuf) -> Result<bool> {
    let ast = parse_file_ast(input_dir, input_file)?;
    let formatted = format_ast(&ast);
    let output = if formatted.is_empty() {
        String::new()
    } else {
        format!("{formatted}\n")
    };

    let existing = fs::read_to_string(input_file)
        .with_context(|| format!("Failed to read {}", input_file.display()))?;

    if existing == output {
        return Ok(false);
    }

    fs::write(input_file, output)
        .with_context(|| format!("Failed to write {}", input_file.display()))?;

    Ok(true)
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

pub fn format_directory(path: PathBuf, pb: ProgressBar) -> Result<()> {
    let files = collect_rpy_files(&path)?;

    if files.is_empty() {
        pb.finish_with_message("No .rpy files found");
        return Ok(());
    }

    pb.set_length(files.len() as u64);
    let mut unchanged_count = 0;
    let mut formatted_count = 0;
    let mut failures = vec![];

    for input_file in &files {
        pb.set_message(input_file.display().to_string());
        match format_file(&path, input_file) {
            Ok(true) => {
                formatted_count += 1;
            }
            Ok(false) => {
                unchanged_count += 1;
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
        "Formatted {} .rpy file(s): {} changed, {} unchanged, {} failed",
        files.len(),
        formatted_count,
        unchanged_count,
        failures.len()
    );

    if !failures.is_empty() {
        bail!("encountered format errors")
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::munge_filename;

    #[test]
    fn test_filename_munge() {
        let result = munge_filename(&PathBuf::from("test/foo/bar_test -*.txt")).unwrap();
        assert_eq!(result, "_m1_bar_test_0x2d0x2a__")
    }
}
