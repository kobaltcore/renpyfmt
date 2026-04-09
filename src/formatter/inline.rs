use crate::{
    ast::{ArgumentInfo, ImageSpecifier, ParameterKind, ParameterSignature},
    lexer::Block,
};

pub(crate) fn encode_say_string(s: &str) -> String {
    let mut escaped = s.replace('\\', "\\\\");
    escaped = escaped.replace('\n', "\\n");
    escaped = escaped.replace('"', "\\\"");
    format!("\"{escaped}\"")
}

pub(crate) fn format_image_specifier(image: &ImageSpecifier) -> String {
    let mut parts = vec![];

    if !image.image_name.is_empty() {
        parts.push(image.image_name.join(" "));
    }

    if let Some(expr) = &image.expression {
        parts.push(format!("expression {expr}"));
    }

    if let Some(tag) = &image.tag {
        parts.push(format!("as {tag}"));
    }

    if !image.at_list.is_empty() {
        parts.push(format!("at {}", image.at_list.join(", ")));
    }

    if let Some(layer) = &image.layer {
        parts.push(format!("onlayer {layer}"));
    }

    if let Some(zorder) = &image.zorder {
        parts.push(format!("zorder {zorder}"));
    }

    if !image.behind.is_empty() {
        parts.push(format!("behind {}", image.behind.join(", ")));
    }

    parts.join(" ")
}

pub(crate) fn format_argument_info(arguments: &ArgumentInfo) -> String {
    let mut parts = vec![];

    for (index, (keyword, expression)) in arguments.arguments.iter().enumerate() {
        let expression = expression
            .as_ref()
            .expect("parser should not construct missing argument expressions");

        if arguments.starred_indexes.contains(&index) {
            parts.push(format!("*{expression}"));
        } else if arguments.doublestarred_indexes.contains(&index) {
            parts.push(format!("**{expression}"));
        } else if let Some(keyword) = keyword {
            parts.push(format!("{keyword}={expression}"));
        } else {
            parts.push(expression.to_string());
        }
    }

    format!("({})", parts.join(", "))
}

pub(crate) fn format_parameter_signature(signature: &ParameterSignature) -> String {
    let mut positional_only = vec![];
    let mut positional_or_keyword = vec![];
    let mut keyword_only = vec![];
    let mut var_positional = vec![];
    let mut var_keyword = vec![];

    for parameter in signature.parameters.values() {
        let rendered = match parameter.kind {
            ParameterKind::VarPositional => format!("*{}", parameter.name),
            ParameterKind::VarKeyword => format!("**{}", parameter.name),
            _ => match &parameter.default {
                Some(default) => format!("{}={default}", parameter.name),
                None => parameter.name.clone(),
            },
        };

        match parameter.kind {
            ParameterKind::PositionalOnly => positional_only.push(rendered),
            ParameterKind::PositionalOrKeyword => positional_or_keyword.push(rendered),
            ParameterKind::VarPositional => var_positional.push(rendered),
            ParameterKind::KeywordOnly => keyword_only.push(rendered),
            ParameterKind::VarKeyword => var_keyword.push(rendered),
        }
    }

    positional_only.sort();
    positional_or_keyword.sort();
    keyword_only.sort();
    var_positional.sort();
    var_keyword.sort();

    let mut parts = vec![];
    parts.extend(positional_only);
    if !parts.is_empty()
        && signature
            .parameters
            .values()
            .any(|parameter| matches!(parameter.kind, ParameterKind::PositionalOnly))
    {
        parts.push("/".into());
    }
    parts.extend(positional_or_keyword);
    parts.extend(var_positional);
    parts.extend(keyword_only);
    parts.extend(var_keyword);

    format!("({})", parts.join(", "))
}

pub(crate) fn format_raw_block(blocks: &[Block], indent: usize) -> Vec<String> {
    let mut lines = vec![];

    for block in blocks {
        lines.push(format!("{}{}", " ".repeat(indent), block.text));
        lines.extend(format_raw_block(&block.block, indent + 4));
    }

    lines
}
