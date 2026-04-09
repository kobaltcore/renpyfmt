use crate::ast::{ArgumentInfo, ImageSpecifier};

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
