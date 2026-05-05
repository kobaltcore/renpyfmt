use crate::formatter::{ConfiguredLineEnding, PythonFormatConfig, PythonFormatterSettings};
use anyhow::{Context, Result, bail};
use etcetera::BaseStrategy;
use ruff_formatter::{IndentStyle, IndentWidth, LineWidth};
use ruff_python_ast::PythonVersion;
use ruff_python_formatter::{
    DocstringCode, DocstringCodeLineWidth, MagicTrailingComma, NestedStringQuoteStyle, PreviewMode,
    QuoteStyle,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use toml::Value;

pub(crate) fn resolve_python_format_config(
    input_dir: &Path,
    ruff_config: Option<&Path>,
) -> Result<PythonFormatConfig> {
    let input_dir = fs::canonicalize(input_dir)
        .with_context(|| format!("Failed to resolve input directory {}", input_dir.display()))?;

    let formatter_settings = load_python_formatter_settings(&input_dir, ruff_config)?;
    Ok(PythonFormatConfig::new(input_dir, formatter_settings))
}

fn load_python_formatter_settings(
    input_dir: &Path,
    ruff_config: Option<&Path>,
) -> Result<PythonFormatterSettings> {
    if let Some(ruff_config) = ruff_config {
        let ruff_config = fs::canonicalize(ruff_config).with_context(|| {
            format!(
                "Failed to resolve Ruff config path {}",
                ruff_config.display()
            )
        })?;
        return load_settings_from_path(&ruff_config)
            .with_context(|| format!("Failed to load Ruff config {}", ruff_config.display()));
    }

    if let Some(ruff_config) = find_settings_toml(input_dir).with_context(|| {
        format!(
            "Failed to discover Ruff config from {}",
            input_dir.display()
        )
    })? {
        return load_settings_from_path(&ruff_config)
            .with_context(|| format!("Failed to load Ruff config {}", ruff_config.display()));
    }

    if let Some(ruff_config) = find_user_settings_toml() {
        return load_settings_from_path(&ruff_config)
            .with_context(|| format!("Failed to load Ruff config {}", ruff_config.display()));
    }

    let mut settings = PythonFormatterSettings::default();
    if let Some(target_version) = find_fallback_target_version(input_dir) {
        settings.target_version = target_version;
    }
    Ok(settings)
}

fn load_settings_from_path(path: &Path) -> Result<PythonFormatterSettings> {
    let root = parse_toml(path)?;
    let mut settings = PythonFormatterSettings::default();

    let (ruff, project) = if path.ends_with("pyproject.toml") {
        let tool = get_table(&root, "tool");
        (
            tool.and_then(|tool| get_nested_table(tool, "ruff")),
            get_table(&root, "project"),
        )
    } else {
        (
            Some(
                root.as_table()
                    .context("Ruff config root must be a TOML table")?,
            ),
            None,
        )
    };

    let Some(ruff) = ruff else {
        return Ok(settings);
    };

    if let Some(target_version) = get_string(ruff, "target-version")
        .map(parse_target_version)
        .transpose()?
    {
        settings.target_version = target_version;
    } else if let Some(project) = project {
        if let Some(requires_python) = get_string(project, "requires-python") {
            if let Some(target_version) = infer_requires_python_version(requires_python) {
                settings.target_version = target_version;
            }
        }
    }

    if let Some(line_width) = get_integer(ruff, "line-length") {
        settings.line_width = to_line_width(line_width, "line-length")?;
    }
    if let Some(indent_width) = get_integer(ruff, "indent-width") {
        settings.indent_width = to_indent_width(indent_width, "indent-width")?;
    }
    if let Some(preview) = parse_preview_value(ruff.get("preview"))? {
        settings.preview = preview;
    }

    if let Some(format) = get_nested_table(ruff, "format") {
        if let Some(quote_style) = get_string(format, "quote-style") {
            settings.quote_style = QuoteStyle::from_str(quote_style)
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("Unsupported Ruff quote-style `{quote_style}`"))?;
        }
        if let Some(indent_style) = get_string(format, "indent-style") {
            settings.indent_style = parse_indent_style(indent_style)?;
        }
        if let Some(nested_quote_style) = get_string(format, "nested-string-quote-style") {
            settings.nested_string_quote_style =
                parse_nested_string_quote_style(nested_quote_style)?;
        }
        if let Some(skip_magic_trailing_comma) = get_bool(format, "skip-magic-trailing-comma") {
            settings.magic_trailing_comma = if skip_magic_trailing_comma {
                MagicTrailingComma::Ignore
            } else {
                MagicTrailingComma::Respect
            };
        }
        if let Some(line_ending) = get_string(format, "line-ending") {
            settings.line_ending = parse_line_ending(line_ending)?;
        }
        if let Some(docstring_code_format) = get_bool(format, "docstring-code-format") {
            settings.docstring_code = if docstring_code_format {
                DocstringCode::Enabled
            } else {
                DocstringCode::Disabled
            };
        }
        if let Some(docstring_code_line_length) = format.get("docstring-code-line-length") {
            settings.docstring_code_line_width =
                parse_docstring_code_line_width(docstring_code_line_length)?;
        }
        if let Some(preview) = parse_preview_value(format.get("preview"))? {
            settings.preview = preview;
        }
    }

    Ok(settings)
}

fn parse_toml(path: &Path) -> Result<Value> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str::<Value>(&contents)
        .with_context(|| format!("Failed to parse {}", path.display()))
}

fn settings_toml(path: &Path) -> Result<Option<PathBuf>> {
    let dot_ruff = path.join(".ruff.toml");
    if dot_ruff.is_file() {
        return Ok(Some(dot_ruff));
    }

    let ruff = path.join("ruff.toml");
    if ruff.is_file() {
        return Ok(Some(ruff));
    }

    let pyproject = path.join("pyproject.toml");
    if pyproject.is_file() && pyproject_has_ruff(&pyproject)? {
        return Ok(Some(pyproject));
    }

    Ok(None)
}

fn find_settings_toml(path: &Path) -> Result<Option<PathBuf>> {
    for directory in path.ancestors() {
        if let Some(config) = settings_toml(directory)? {
            return Ok(Some(config));
        }
    }
    Ok(None)
}

fn pyproject_has_ruff(path: &Path) -> Result<bool> {
    let root = parse_toml(path)?;
    Ok(get_table(&root, "tool")
        .and_then(|tool| get_nested_table(tool, "ruff"))
        .is_some())
}

fn find_user_settings_toml() -> Option<PathBuf> {
    let strategy = etcetera::base_strategy::choose_base_strategy().ok()?;
    let config_dir = strategy.config_dir().join("ruff");
    [".ruff.toml", "ruff.toml", "pyproject.toml"]
        .into_iter()
        .map(|name| config_dir.join(name))
        .find(|path| path.is_file())
}

fn find_fallback_target_version(path: &Path) -> Option<PythonVersion> {
    for directory in path.ancestors() {
        let pyproject = directory.join("pyproject.toml");
        if !pyproject.is_file() {
            continue;
        }

        let Ok(root) = parse_toml(&pyproject) else {
            continue;
        };
        let Some(project) = get_table(&root, "project") else {
            continue;
        };
        let Some(requires_python) = get_string(project, "requires-python") else {
            continue;
        };
        if let Some(target_version) = infer_requires_python_version(requires_python) {
            return Some(target_version);
        }
    }
    None
}

fn infer_requires_python_version(specifier: &str) -> Option<PythonVersion> {
    for segment in specifier.split(|c: char| !c.is_ascii_alphanumeric() && c != '.') {
        if segment.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            if let Ok(version) = parse_target_version(segment) {
                return Some(version);
            }
        }
    }
    None
}

fn parse_target_version(value: &str) -> Result<PythonVersion> {
    if let Some(version) = value.strip_prefix("py") {
        if version.len() == 2 {
            let minor = version
                .parse::<u8>()
                .with_context(|| format!("Unsupported Ruff target-version `{value}`"))?;
            return Ok(PythonVersion::from((3, minor)));
        }
        if version.len() == 3 {
            let major = version[0..1]
                .parse::<u8>()
                .with_context(|| format!("Unsupported Ruff target-version `{value}`"))?;
            let minor = version[1..]
                .parse::<u8>()
                .with_context(|| format!("Unsupported Ruff target-version `{value}`"))?;
            return Ok(PythonVersion::from((major, minor)));
        }
    }

    value
        .parse()
        .with_context(|| format!("Unsupported Ruff target-version `{value}`"))
}

fn parse_indent_style(value: &str) -> Result<IndentStyle> {
    match value {
        "tab" => Ok(IndentStyle::Tab),
        "space" => Ok(IndentStyle::Space),
        _ => bail!("Unsupported Ruff indent-style `{value}`"),
    }
}

fn parse_nested_string_quote_style(value: &str) -> Result<NestedStringQuoteStyle> {
    match value {
        "alternating" => Ok(NestedStringQuoteStyle::Alternating),
        "preferred" => Ok(NestedStringQuoteStyle::Preferred),
        _ => bail!("Unsupported Ruff nested-string-quote-style `{value}`"),
    }
}

fn parse_line_ending(value: &str) -> Result<ConfiguredLineEnding> {
    match value {
        "auto" => Ok(ConfiguredLineEnding::Auto),
        "lf" => Ok(ConfiguredLineEnding::Lf),
        "crlf" => Ok(ConfiguredLineEnding::CrLf),
        "native" => Ok(ConfiguredLineEnding::Native),
        _ => bail!("Unsupported Ruff line-ending `{value}`"),
    }
}

fn parse_docstring_code_line_width(value: &Value) -> Result<DocstringCodeLineWidth> {
    if let Some(width) = value.as_integer() {
        return Ok(DocstringCodeLineWidth::Fixed(to_line_width(
            width,
            "docstring-code-line-length",
        )?));
    }

    if let Some("dynamic") = value.as_str() {
        return Ok(DocstringCodeLineWidth::Dynamic);
    }

    bail!("Unsupported Ruff docstring-code-line-length `{value}`")
}

fn parse_preview_value(value: Option<&Value>) -> Result<Option<PreviewMode>> {
    let Some(value) = value else {
        return Ok(None);
    };

    if let Some(enabled) = value.as_bool() {
        return Ok(Some(if enabled {
            PreviewMode::Enabled
        } else {
            PreviewMode::Disabled
        }));
    }

    if let Some(mode) = value.as_str() {
        return Ok(Some(match mode {
            "enabled" => PreviewMode::Enabled,
            "disabled" => PreviewMode::Disabled,
            _ => bail!("Unsupported Ruff preview mode `{mode}`"),
        }));
    }

    bail!("Unsupported Ruff preview value `{value}`")
}

fn to_indent_width(value: i64, setting: &str) -> Result<IndentWidth> {
    let value =
        u8::try_from(value).with_context(|| format!("Invalid Ruff {setting} value `{value}`"))?;
    IndentWidth::try_from(value).with_context(|| format!("Invalid Ruff {setting} value `{value}`"))
}

fn to_line_width(value: i64, setting: &str) -> Result<LineWidth> {
    let value =
        u16::try_from(value).with_context(|| format!("Invalid Ruff {setting} value `{value}`"))?;
    LineWidth::try_from(value).with_context(|| format!("Invalid Ruff {setting} value `{value}`"))
}

fn get_table<'a>(value: &'a Value, key: &str) -> Option<&'a toml::value::Table> {
    value.get(key)?.as_table()
}

fn get_nested_table<'a>(
    table: &'a toml::value::Table,
    key: &str,
) -> Option<&'a toml::value::Table> {
    table.get(key)?.as_table()
}

fn get_string<'a>(table: &'a toml::value::Table, key: &str) -> Option<&'a str> {
    table.get(key)?.as_str()
}

fn get_integer(table: &toml::value::Table, key: &str) -> Option<i64> {
    table.get(key)?.as_integer()
}

fn get_bool(table: &toml::value::Table, key: &str) -> Option<bool> {
    table.get(key)?.as_bool()
}
