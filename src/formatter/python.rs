use std::path::{Path, PathBuf};

use ruff_python_formatter::{PyFormatOptions, format_module_source};
use ruff_workspace::FormatterSettings;

#[derive(Clone, Debug)]
pub(crate) struct PythonFormatConfig {
    root: PathBuf,
    formatter_settings: FormatterSettings,
}

impl PythonFormatConfig {
    pub(crate) fn new(root: PathBuf, formatter_settings: FormatterSettings) -> Self {
        Self {
            root,
            formatter_settings,
        }
    }

    fn format_options(&self, source: &str) -> PyFormatOptions {
        let synthetic_path = self.root.join("__renpyfmt__.py");
        let source_type = PyFormatOptions::from_extension(Path::new("renpyfmt.py")).source_type();

        self.formatter_settings
            .to_format_options(source_type, source, Some(&synthetic_path))
    }
}

impl Default for PythonFormatConfig {
    fn default() -> Self {
        Self {
            root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            formatter_settings: FormatterSettings::default(),
        }
    }
}

pub(crate) fn format_python_block(source: &str, config: &PythonFormatConfig) -> String {
    let base_indent = base_leading_indent(source);
    let dedented = strip_leading_indent(source, &base_indent);

    let formatted = match format_module_source(&dedented, config.format_options(&dedented)) {
        Ok(printed) => printed.as_code().trim_end_matches('\n').to_string(),
        Err(_) => return source.to_string(),
    };

    if base_indent.is_empty() {
        formatted
    } else {
        restore_leading_indent(&formatted, &base_indent)
    }
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

    source
        .lines()
        .map(|line| match line.strip_prefix(indent) {
            Some(stripped) if !line.trim().is_empty() => stripped,
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn restore_leading_indent(source: &str, indent: &str) -> String {
    source
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{indent}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{PythonFormatConfig, format_python_block};

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
}
