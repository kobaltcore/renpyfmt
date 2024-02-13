use anyhow::{bail, Ok, Result};
use renpyfmt::ast::AstNode;
use renpyfmt::formatter::format_ast;
use renpyfmt::lexer::{Block, Lexer};
use renpyfmt::parser::parse_block;
// use ruff_python_ast::PySourceType;
// use ruff_python_formatter::{format_module_ast, PyFormatOptions};
// use ruff_python_index::tokens_and_ranges;
// use ruff_python_parser::{parse_tokens, AsMode};
use glob::glob;
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

struct LexerContext {
    // base_dir: PathBuf,
    // renpy_base: PathBuf,
    input_dir: PathBuf,
}

/*
fn _format() -> Result<()> {
    /*
    TODO:
    - find some way to parse rpy files and split them into python blocks and renpy blocks
      - maybe use VScode extension? it has a semantic token provider:
        https://github.com/LuqueDaniel/vscode-language-renpy/blob/master/src/semantics.ts
      - maybe reimplement in rust and use that
    - first step: format all python-related blocks with ruff and isort
    - second step: use semantic parse to format renpy blocks, if possible
    */

    let source_path = Path::new("main.py");

    let bytes = fs::read(source_path)?;
    let source = str::from_utf8(&bytes)?;

    let source_type = PySourceType::Python;
    let (tokens, comment_ranges) = tokens_and_ranges(source, source_type)
        .map_err(|err| format_err!("Source contains syntax errors {err:?}"))?;

    let module = parse_tokens(tokens, source, source_type.as_mode())?;

    let options = PyFormatOptions::from_source_type(source_type);

    let formatted = format_module_ast(&module, &comment_ranges, source, options)?;

    let output = formatted.print()?.as_code().to_string();

    fs::write("main.py", output)?;

    Ok(())
}
*/

fn ren_py_to_rpy(data: &String, filename: Option<&PathBuf>) -> Result<String> {
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

            // Ignore empty
            let line_trimmed = line.trim();
            if line_trimmed.len() == 0 {
                result.push(line.to_string());
                continue;
            }

            // Ignore comments
            if line_trimmed.starts_with("#") {
                result.push(line.to_string());
                continue;
            }

            // Determine the prefix.
            prefix = "".into();
            for c in line.chars() {
                if c != ' ' {
                    break;
                }
                prefix = format!("{prefix} ");
            }

            // If the line ends in ":", add 4 spaces to the prefix.
            if line_trimmed.ends_with(":") {
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

// fn convert_slashes(path: PathBuf) -> Result<PathBuf> {
//     let s = String::from_utf8(path.to_str().unwrap().into()).unwrap();
//     Ok(PathBuf::from(s.replace("\\", "/")))
// }

fn elide_filename(ctx: &LexerContext, path: &PathBuf) -> Result<PathBuf> {
    // let dirs = if path.starts_with(ctx.base_dir.clone()) {
    //     vec![ctx.renpy_base.clone(), ctx.base_dir.clone()]
    // } else {
    //     vec![ctx.base_dir.clone(), ctx.renpy_base.clone()]
    // };

    // for dir in dirs {
    //     if path.starts_with(&dir) {
    //         let p = PathBuf::from(path.strip_prefix(dir).unwrap());
    //         return convert_slashes(p);
    //     }
    // }

    // convert_slashes(path.clone())

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
    let mut data = fs::read_to_string(&path)?;
    let stem = path.file_stem().unwrap().to_str().unwrap();

    if stem.ends_with("_ren") && path.extension() == Some("py".as_ref()) {
        // println!("renpy file");
        data = ren_py_to_rpy(&data, Some(path))?;
    }

    let path = elide_filename(ctx, &path)?;
    let prefix = munge_filename(&path)?;

    // Add some newlines, to fix lousy editors.
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
                if final_line.trim().len() > 0 {
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
                // endpos = None;
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

                    continue;
                }

                let s = s.join("");

                if s.contains("[__") {
                    // TODO: munge subtitutions
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

        if line.len() > 0 {
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

        let (block, _i) = gll_core(lines, idx, depth.unwrap() + 1)?;
        idx = _i;

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

/*
fn print_blocks(blocks: Vec<Block>, depth: usize) {
    for block in blocks {
        for _ in 0..depth {
            print!("    ");
        }

        println!(
            "{}:{}:{}",
            block.filename.display(),
            block.number,
            block.text
        );

        print_blocks(block.block, depth + 1);
    }
}
*/

fn print_nodes(nodes: Vec<AstNode>, depth: usize) {
    for node in nodes {
        for _ in 0..depth {
            print!("    ");
        }

        match node {
            AstNode::Label(l) => {
                println!("Label: {}", l.name);
                print_nodes(l.block, depth + 1);
            }
            AstNode::Scene(s) => {
                println!("Scene: {:?}", s);
            }
            AstNode::With(w) => {
                println!("With: {:?}", w);
            }
            AstNode::Say(s) => {
                println!("Say: {:?}", s);
            }
            AstNode::UserStatement(u) => {
                println!("UserStatement: {:?}", u);
            }
            AstNode::Show(s) => {
                println!("Show: {:?}", s);
            }
            AstNode::Hide(h) => {
                println!("Hide: {:?}", h);
            }
            AstNode::PythonOneLine(p) => {
                println!("PythonOneLine: {:?}", p);
            }
            AstNode::Jump(j) => {
                println!("Jump: {:?}", j);
            }
            AstNode::Menu(m) => {
                println!("Menu: {:?}", m);
            }
            AstNode::If(i) => {
                println!("If: {:?}", i);
            }
            AstNode::Return(r) => {
                println!("Return: {:?}", r);
            }
            AstNode::Style(s) => {
                println!("Style: {:?}", s);
            }
            AstNode::Init(i) => {
                println!("Init: {:?}", i);
            }
            AstNode::Python(p) => {
                println!("Python: {:?}", p);
            }
            AstNode::EarlyPython(e) => {
                println!("EarlyPython: {:?}", e);
            }
            AstNode::Define(d) => {
                println!("Define: {:?}", d);
            }
            AstNode::Default(d) => {
                println!("Default: {:?}", d);
            }
            AstNode::Call(c) => {
                println!("Call: {:?}", c);
            }
            AstNode::Pass(p) => {
                println!("Pass: {:?}", p);
            }
        }
    }
}

fn main() -> Result<()> {
    // m = re.compile(regexp, re.DOTALL).match(self.text, self.pos)
    // let skip_whitespace = RegexBuilder::new(r"^(\s+|\\\n)+")
    //     .dot_matches_new_line(true)
    //     .build()
    //     .unwrap();
    // let word_regexp = RegexBuilder::new(r#"<>|<<|<=|<|>>|>=|>|!=|==|\||\^|&|\+|\-|\*\*|\*|\/\/|\/|%|~|@|:=|\bor\b|\band\b|\bnot\b|\bin\b|\bis\b"#)
    //     .dot_matches_new_line(true)
    //     .build()
    //     .unwrap();
    // let m = word_regexp.find("is 0.075");
    // println!("m: {:?}", m);
    // return Ok(());

    let files: Vec<PathBuf> = glob("game/**/*.rpy")
        .expect("Failed to read glob pattern")
        .into_iter()
        .filter_map(|s| s.ok())
        .collect();
    // let files = vec![PathBuf::from("game/magic/mina/middle.rpy")];

    files.par_iter().for_each(|input_file| {
        println!("Processing: {}", input_file.display());

        let ctx = LexerContext {
            // base_dir: PathBuf::from("."),
            // renpy_base: PathBuf::from("."),
            input_dir: PathBuf::from("game"),
        };

        // list logical lines
        let lines = list_logical_lines(&ctx, &input_file).unwrap();
        // for (path, line_num, line) in lines {
        //     println!("{}:{}:{}", path.display(), line_num, line);
        // }

        // group logical lines
        let nested = group_logical_lines(lines).unwrap();
        // print_blocks(nested, 0);

        let mut lex = Lexer::new(nested);

        // parse blocks
        let ast = parse_block(&mut lex).unwrap();

        // print_nodes(ast, 0);

        let lines = format_ast(&ast, 0);

        println!("{}", lines.join("\n"));
        // for (i, line) in lines.iter().enumerate() {
        //     println!("{}: {}", i, line);
        // }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::munge_filename;

    #[test]
    fn test_filename_munge() {
        let result = munge_filename(&PathBuf::from("test/foo/bar_test -*.txt")).unwrap();
        assert_eq!(result, "_m1_bar_test_0x2d0x2a__")
    }
}
