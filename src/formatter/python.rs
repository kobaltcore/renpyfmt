use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ruff_python_formatter::{PyFormatOptions, format_module_source};
use ruff_workspace::FormatterSettings;

#[derive(Clone, Debug)]
pub struct PythonFormatConfig {
    formatter_settings: FormatterSettings,
    synthetic_path: PathBuf,
}

impl PythonFormatConfig {
    pub fn new(root: PathBuf, formatter_settings: FormatterSettings) -> Self {
        Self {
            formatter_settings,
            synthetic_path: root.join("__renpyfmt__.py"),
        }
    }

    fn format_options(&self, source: &str) -> PyFormatOptions {
        let source_type = PyFormatOptions::from_extension(Path::new("renpyfmt.py")).source_type();
        self.formatter_settings
            .to_format_options(source_type, source, Some(&self.synthetic_path))
    }
}

impl Default for PythonFormatConfig {
    fn default() -> Self {
        Self::new(
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            FormatterSettings::default(),
        )
    }
}

pub fn format_python_block(source: &str, config: &PythonFormatConfig) -> String {
    if source.trim().is_empty() {
        return source.to_string();
    }

    let base_indent = base_leading_indent(source);
    let dedented = strip_leading_indent(source, &base_indent);

    let formatted = match format_module_source(&dedented, config.format_options(&dedented)) {
        Ok(printed) => printed.as_code().trim_end_matches('\n').to_string(),
        Err(_) => return source.to_string(),
    };
    let formatted = normalize_standalone_multiline_strings(&formatted);

    if base_indent.is_empty() {
        formatted
    } else {
        restore_leading_indent(&formatted, &base_indent)
    }
}

pub fn format_python_file(source: &str, config: &PythonFormatConfig) -> Result<String> {
    let formatted = format_module_source(source, config.format_options(source))
        .context("Ruff could not parse or format Python source")?;

    Ok(formatted.as_code().to_string())
}

fn base_leading_indent(source: &str) -> String {
    source
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(leading_whitespace)
        .unwrap_or_default()
}

fn leading_whitespace(line: &str) -> String {
    line.chars()
        .take_while(|char| char.is_ascii_whitespace())
        .collect()
}

fn strip_leading_indent(source: &str, indent: &str) -> String {
    if indent.is_empty() {
        return source.to_string();
    }

    let mut out = String::with_capacity(source.len());

    for (index, line) in source.lines().enumerate() {
        if index > 0 {
            out.push('\n');
        }

        match line.strip_prefix(indent) {
            Some(stripped) if !line.trim().is_empty() => out.push_str(stripped),
            _ => out.push_str(line),
        }
    }

    out
}

fn restore_leading_indent(source: &str, indent: &str) -> String {
    let mut out = String::with_capacity(source.len() + indent.len() * source.lines().count());

    for (index, line) in source.lines().enumerate() {
        if index > 0 {
            out.push('\n');
        }

        if !line.is_empty() {
            out.push_str(indent);
            out.push_str(line);
        }
    }

    out
}

fn normalize_standalone_multiline_strings(source: &str) -> String {
    let mut lines = source.lines().map(str::to_string).collect::<Vec<_>>();
    let mut index = 0;

    while index < lines.len() {
        let Some(delimiter) = standalone_triple_quote_delimiter(&lines[index]) else {
            index += 1;
            continue;
        };

        let Some(end_index) =
            (index + 1..lines.len()).find(|line_index| lines[*line_index].trim() == delimiter)
        else {
            index += 1;
            continue;
        };

        let common_indent = lines[index + 1..end_index]
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| leading_whitespace(line).len())
            .min();

        if let Some(common_indent) = common_indent {
            let indent = " ".repeat(common_indent);

            for line in &mut lines[index + 1..end_index] {
                if line.trim().is_empty() {
                    continue;
                }

                *line = line.strip_prefix(&indent).unwrap_or(line).to_string();
            }

            lines[end_index] = lines[end_index]
                .strip_prefix(&indent)
                .unwrap_or(&lines[end_index])
                .to_string();
        }

        index = end_index + 1;
    }

    lines.join("\n")
}

fn standalone_triple_quote_delimiter(line: &str) -> Option<&'static str> {
    let trimmed = line.trim();

    if matches!(
        trimmed,
        "\"\"\"" | "r\"\"\"" | "R\"\"\"" | "u\"\"\"" | "U\"\"\""
    ) {
        Some("\"\"\"")
    } else if matches!(trimmed, "'''" | "r'''" | "R'''" | "u'''" | "U'''") {
        Some("'''")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{PythonFormatConfig, format_python_block, format_python_file};

    #[test]
    fn formats_python_block_with_ruff() {
        assert_eq!(
            format_python_block(
                "numbers=[1,2,3]\nif True: print( numbers )",
                &PythonFormatConfig::default(),
            ),
            "numbers = [1, 2, 3]\nif True:\n    print(numbers)"
        );
    }

    #[test]
    fn preserves_existing_base_indentation() {
        assert_eq!(
            format_python_block(
                "        values=[1,2]\n        if True: print( values )",
                &PythonFormatConfig::default(),
            ),
            "        values = [1, 2]\n        if True:\n            print(values)"
        );
    }

    #[test]
    fn preserves_multiline_string_indentation_relative_to_python_block() {
        assert_eq!(
            format_python_block(
                concat!(
                    "        import re\n",
                    "\n",
                    "        \"\"\"\n",
                    "            Scenario Mode now uses a list of locations.\n",
                    "            This allows an external scenario directory.\n",
                    "            \"\"\"\n",
                    "\n",
                    "        def update_scenario_paths():\n",
                    "            scenario_base_paths=[1,2]\n",
                ),
                &PythonFormatConfig::default(),
            ),
            concat!(
                "        import re\n",
                "\n",
                "        \"\"\"\n",
                "        Scenario Mode now uses a list of locations.\n",
                "        This allows an external scenario directory.\n",
                "        \"\"\"\n",
                "\n\n",
                "        def update_scenario_paths():\n",
                "            scenario_base_paths = [1, 2]"
            )
        );
    }

    #[test]
    fn preserves_assigned_multiline_string_contents() {
        assert_eq!(
            format_python_block(
                concat!("text = \"\"\"\n", "    keep this indent\n", "\"\"\"\n",),
                &PythonFormatConfig::default(),
            ),
            concat!("text = \"\"\"\n", "    keep this indent\n", "\"\"\"")
        );
    }

    #[test]
    fn formats_whole_python_file_with_ruff() {
        assert_eq!(
            format_python_file("x=[1,2]\n", &PythonFormatConfig::default()).unwrap(),
            "x = [1, 2]\n"
        );
    }

    #[test]
    fn returns_error_for_invalid_python_file() {
        assert!(
            format_python_file("if True print('hi')\n", &PythonFormatConfig::default()).is_err()
        );
    }
}
