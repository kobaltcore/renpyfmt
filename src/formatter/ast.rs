use crate::ast::{
    AudioOperation, AudioStatement, AudioTarget, Call, Camera, CompileIf, Default_, Define,
    EarlyPython, EndTranslate, Hide, If, Image, Init, InitOffset, Jump, Label, LayeredImage,
    LayeredImageChild, LayeredImageDisplayable, LayeredImageProperty, LayeredImagePropertyValue,
    Menu, Pass, PauseStatement, Python, PythonOneLine, RPY, Return, Say, Scene, ScreenStatement,
    ScreenStatementKind, Show, ShowLayer, Style, Transform, Translate, TranslateBlock,
    TranslateEarlyBlock, TranslateString, While, WindowAutoKind, WindowAutoStatement, WindowKind,
    WindowStatement, With,
};

use super::{
    core::{Formatter, Mode},
    inline::{
        encode_say_string, format_argument_info, format_image_specifier, format_parameter_signature,
    },
};

impl Formatter {
    fn format_say_line(node: &Say, with_suffix: Option<&With>) -> String {
        let mut parts = vec![];

        if let Some(who) = &node.who {
            parts.push(who.clone());
        }

        if let Some(attributes) = &node.attributes {
            parts.extend(attributes.clone());
        }

        if let Some(temporary_attributes) = &node.temporary_attributes {
            parts.push("@".to_string());
            parts.extend(temporary_attributes.clone());
        }

        parts.push(encode_say_string(&node.what));

        if let Some(arguments) = &node.arguments {
            parts.push(format_argument_info(arguments));
        }

        if let Some(with_clause) = &node.with {
            parts.push(format!("with {with_clause}"));
        }

        if !node.interact {
            parts.push("nointeract".to_string());
        }

        if let Some(identifier) = &node.identifier {
            parts.push(format!("id {identifier}"));
        }

        if let Some(with) = with_suffix {
            parts.push(format!("with {}", with.expr));
        }

        parts.join(" ")
    }

    pub(crate) fn emit_label(&mut self, node: &Label) {
        let mut line = format!("label {}", node.name);
        if let Some(parameters) = &node.parameters {
            line.push_str(&format_parameter_signature(parameters));
        }
        if node.hide {
            line.push_str(" hide");
        }
        line.push(':');

        self.line_with_trailing(&line);
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_scene(&mut self, node: &Scene, with_suffix: Option<&With>) {
        let mut line = match &node.imspec {
            Some(image) => format!("scene {}", format_image_specifier(image)),
            None => match &node.layer {
                Some(layer) => format!("scene onlayer {layer}"),
                None => String::from("scene"),
            },
        };

        if let Some(with) = with_suffix {
            line.push_str(&format!(" with {}", with.expr));
        }

        if let Some(atl) = &node.atl {
            self.line_with_trailing(&format!("{line}:"));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line_with_trailing(&line);
        }
    }

    pub(crate) fn emit_show(&mut self, node: &Show, with_suffix: Option<&With>) {
        let image = node
            .imspec
            .as_ref()
            .expect("parser should construct show image specifiers");
        let mut line = format!("show {}", format_image_specifier(image));

        if let Some(with) = with_suffix {
            line.push_str(&format!(" with {}", with.expr));
        }

        if let Some(atl) = &node.atl {
            self.line_with_trailing(&format!("{line}:"));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line_with_trailing(&line);
        }
    }

    pub(crate) fn emit_with(&mut self, node: &With) {
        if node.expr != "None" {
            self.line_with_trailing(&format!("with {}", node.expr));
        }
    }

    pub(crate) fn emit_audio_statement(&mut self, node: &AudioStatement) {
        let mut parts = vec![match (&node.operation, &node.target) {
            (AudioOperation::Play, AudioTarget::Music) => "play music".to_string(),
            (AudioOperation::Queue, AudioTarget::Music) => "queue music".to_string(),
            (AudioOperation::Stop, AudioTarget::Music) => "stop music".to_string(),
            (AudioOperation::Play, AudioTarget::Sound) => "play sound".to_string(),
            (AudioOperation::Queue, AudioTarget::Sound) => "queue sound".to_string(),
            (AudioOperation::Stop, AudioTarget::Sound) => "stop sound".to_string(),
            (AudioOperation::Play, AudioTarget::Generic(channel)) => format!("play {channel}"),
            (AudioOperation::Queue, AudioTarget::Generic(channel)) => format!("queue {channel}"),
            (AudioOperation::Stop, AudioTarget::Generic(channel)) => format!("stop {channel}"),
        }];

        if let Some(file) = &node.file {
            parts.push(file.clone());
        }

        if let Some(channel) = &node.channel {
            parts.push(format!("channel {channel}"));
        }

        if let Some(loop_mode) = node.loop_mode {
            parts.push(if loop_mode { "loop" } else { "noloop" }.to_string());
        }

        if let Some(fadeout) = &node.fadeout {
            parts.push(format!("fadeout {fadeout}"));
        }

        if let Some(fadein) = &node.fadein {
            parts.push(format!("fadein {fadein}"));
        }

        if node.if_changed {
            parts.push("if_changed".to_string());
        }

        if let Some(volume) = &node.volume {
            parts.push(format!("volume {volume}"));
        }

        self.line_with_trailing(&parts.join(" "));
    }

    pub(crate) fn emit_pause_statement(&mut self, node: &PauseStatement) {
        if let Some(delay) = &node.delay {
            self.line_with_trailing(&format!("pause {delay}"));
        } else {
            self.line_with_trailing("pause");
        }
    }

    pub(crate) fn emit_screen_statement(&mut self, node: &ScreenStatement) {
        let prefix = match node.kind {
            ScreenStatementKind::Show => "show screen",
            ScreenStatementKind::Call => "call screen",
            ScreenStatementKind::Hide => "hide screen",
        };

        let mut line = prefix.to_string();

        if node.screen.expression {
            line.push_str(" expression");
        }
        line.push(' ');
        line.push_str(&node.screen.value);

        if let Some(arguments) = &node.arguments {
            if node.screen.expression {
                line.push_str(" pass ");
            }
            line.push_str(&format_argument_info(arguments));
        }

        let mut parts = vec![line];
        if !matches!(node.kind, ScreenStatementKind::Hide) && !node.predict {
            parts.push("nopredict".to_string());
        }

        if let Some(with) = &node.with {
            parts.push(format!("with {with}"));
        }

        if let Some(layer) = &node.layer {
            parts.push(format!("onlayer {layer}"));
        }

        if let Some(zorder) = &node.zorder {
            parts.push(format!("zorder {zorder}"));
        }

        if let Some(tag) = &node.tag {
            parts.push(format!("as {tag}"));
        }

        self.line_with_trailing(&parts.join(" "));
    }

    pub(crate) fn emit_window_statement(&mut self, node: &WindowStatement) {
        let prefix = match node.kind {
            WindowKind::Show => "window show",
            WindowKind::Hide => "window hide",
        };

        if let Some(transition) = &node.transition {
            self.line_with_trailing(&format!("{prefix} {transition}"));
        } else {
            self.line_with_trailing(prefix);
        }
    }

    pub(crate) fn emit_window_auto_statement(&mut self, node: &WindowAutoStatement) {
        let line = match &node.kind {
            WindowAutoKind::Auto(Some(expr)) => format!("window auto {expr}"),
            WindowAutoKind::Auto(None) => "window auto".to_string(),
            WindowAutoKind::Show(Some(expr)) => format!("window auto show {expr}"),
            WindowAutoKind::Show(None) => "window auto show".to_string(),
            WindowAutoKind::Hide(Some(expr)) => format!("window auto hide {expr}"),
            WindowAutoKind::Hide(None) => "window auto hide".to_string(),
        };

        self.line_with_trailing(&line);
    }

    pub(crate) fn emit_say(&mut self, node: &Say, with_suffix: Option<&With>) {
        self.line_with_trailing(&Self::format_say_line(node, with_suffix));
        self.blank_line();
    }

    pub(crate) fn emit_hide(&mut self, node: &Hide, with_suffix: Option<&With>) {
        let mut line = format!("hide {}", format_image_specifier(&node.imgspec));
        if let Some(with) = with_suffix {
            line.push_str(&format!(" with {}", with.expr));
        }
        self.line_with_trailing(&line);
    }

    pub(crate) fn emit_python_one_line(&mut self, node: &PythonOneLine) {
        self.line_with_trailing(&format!("$ {}", node.python_code));
    }

    pub(crate) fn emit_jump(&mut self, node: &Jump) {
        let target = if !node.expression {
            if let Some(global_label) = &node.global_label {
                format!("{global_label}.{}", node.target)
            } else {
                node.target.clone()
            }
        } else {
            node.target.clone()
        };

        if node.expression {
            self.line_with_trailing(&format!("jump expression {target}"));
        } else {
            self.line_with_trailing(&format!("jump {target}"));
        }
    }

    pub(crate) fn emit_menu(&mut self, node: &Menu) {
        let mut header = String::from("menu");
        if let Some(arguments) = &node.arguments {
            header.push_str(&format_argument_info(arguments));
        }
        header.push(':');
        self.line_with_trailing(&header);

        self.indented(|formatter| {
            if let Some(say_caption) = &node.say_caption {
                formatter.line(&Self::format_say_line(say_caption, None));
            }

            if let Some(with_clause) = &node.with_ {
                formatter.line(&format!("with {with_clause}"));
            }

            if let Some(set) = &node.set {
                formatter.line(&format!("set {set}"));
            }

            for (index, (label, condition, block)) in node.items.iter().enumerate() {
                if node.say_caption.is_none() && node.has_caption && index == 0 {
                    formatter.line(&encode_say_string(
                        label
                            .as_ref()
                            .expect("parser should construct menu captions"),
                    ));
                } else {
                    let label = label
                        .as_ref()
                        .expect("parser should construct menu choice labels");
                    let mut line = encode_say_string(label);
                    if let Some(arguments) = node
                        .item_arguments
                        .get(index)
                        .and_then(|args| args.as_ref())
                    {
                        line.push_str(&format_argument_info(arguments));
                    }
                    if let Some(condition) = condition {
                        line.push_str(&format!(" if {condition}"));
                    }
                    line.push(':');
                    formatter.line(&line);
                }

                if let Some(block) = block {
                    formatter.indented(|formatter| formatter.nodes(block));
                }
            }
        });
    }

    pub(crate) fn emit_if(&mut self, node: &If) {
        self.emit_conditional_entries(&node.entries, false);
    }

    pub(crate) fn emit_while(&mut self, node: &While) {
        self.line_with_trailing(&format!("while {}:", node.condition));
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_compile_if(&mut self, node: &CompileIf) {
        self.emit_conditional_entries(&node.entries, true);
    }

    fn emit_conditional_entries(
        &mut self,
        entries: &[(Option<String>, Vec<crate::ast::AstNode>)],
        compile: bool,
    ) {
        let first = if compile { "IF" } else { "if" };
        let middle = if compile { "ELIF" } else { "elif" };
        let final_with_condition = if compile { "ELIF" } else { "elif" };
        let final_without_condition = if compile { "ELSE" } else { "else" };
        let last_index = entries.len().saturating_sub(1);

        for (index, (condition, block)) in entries.iter().enumerate() {
            let header = if index == 0 {
                format!(
                    "{first} {}:",
                    condition
                        .as_ref()
                        .expect("parser should construct initial conditional conditions")
                )
            } else if index == last_index {
                match condition {
                    Some(condition) => format!("{final_with_condition} {condition}:"),
                    None => format!("{final_without_condition}:"),
                }
            } else {
                format!(
                    "{middle} {}:",
                    condition
                        .as_ref()
                        .expect("parser should construct elif conditions")
                )
            };

            self.line_with_trailing(&header);
            self.indented(|formatter| formatter.nodes(block));
        }
    }

    pub(crate) fn emit_return(&mut self, node: &Return) {
        if let Some(expr) = &node.expression {
            self.line_with_trailing(&format!("return {expr}"));
        } else {
            self.line_with_trailing("return");
        }
    }

    pub(crate) fn emit_init(&mut self, node: &Init) {
        if self.try_emit_translate_strings(node) {
            return;
        }

        if self.try_emit_init_python(node) {
            return;
        }

        if self.try_emit_implicit_init(node) {
            return;
        }

        if node.priority != 0 {
            self.line_with_trailing(&format!("init {}:", node.priority));
        } else {
            self.line_with_trailing("init:");
        }

        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_init_offset(&mut self, node: &InitOffset) {
        self.current_init_offset = node.offset;
        self.line_with_trailing(&format!("init offset = {}", node.offset));
    }

    fn try_emit_translate_strings(&mut self, node: &Init) -> bool {
        if node.block.is_empty() {
            return false;
        }

        let Some(first_language) = node.block.iter().find_map(|child| match child {
            crate::ast::AstNode::TranslateString(child) => Some(child.language.clone()),
            _ => None,
        }) else {
            return false;
        };

        if !node
            .block
            .iter()
            .all(|child| matches!(child, crate::ast::AstNode::TranslateString(translate) if translate.language == first_language))
        {
            return false;
        }

        let language = first_language.as_deref().unwrap_or("None");
        self.line_with_trailing(&format!("translate {language} strings:"));
        self.indented(|formatter| {
            for child in &node.block {
                let crate::ast::AstNode::TranslateString(translate) = child else {
                    unreachable!();
                };
                formatter.line(&format!("old {}", translate.old));
                formatter.line(&format!("new {}", translate.new));
            }
        });
        true
    }

    fn try_emit_implicit_init(&mut self, node: &Init) -> bool {
        let [child] = node.block.as_slice() else {
            return false;
        };

        match child {
            crate::ast::AstNode::Define(child) if node.priority == self.current_init_offset => {
                self.emit_define(child);
                true
            }
            crate::ast::AstNode::Default(child) if node.priority == self.current_init_offset => {
                self.emit_default(child);
                true
            }
            crate::ast::AstNode::Style(child) if node.priority == self.current_init_offset => {
                self.emit_style(child);
                true
            }
            crate::ast::AstNode::Transform(child) if node.priority == self.current_init_offset => {
                self.emit_transform(child);
                true
            }
            crate::ast::AstNode::Image(child)
                if node.priority == 500 + self.current_init_offset =>
            {
                self.emit_image(child);
                true
            }
            _ => false,
        }
    }

    fn try_emit_init_python(&mut self, node: &Init) -> bool {
        let [child] = node.block.as_slice() else {
            return false;
        };

        match child {
            crate::ast::AstNode::Python(child) => {
                self.emit_init_python_block(node.priority, child, false);
                true
            }
            crate::ast::AstNode::EarlyPython(child) => {
                self.emit_init_python_block(node.priority, child, true);
                true
            }
            _ => false,
        }
    }

    pub(crate) fn emit_style(&mut self, node: &Style) {
        let mut line = format!("style {}", node.name);
        if let Some(parent) = &node.parent {
            line.push_str(&format!(" is {parent}"));
        }

        let mut clauses = vec![];
        if node.clear {
            clauses.push(String::from("clear"));
        }
        if let Some(take) = &node.take {
            clauses.push(format!("take {take}"));
        }
        for delattr in &node.delattr {
            clauses.push(format!("del {delattr}"));
        }
        if let Some(variant) = &node.variant {
            clauses.push(format!("variant {variant}"));
        }

        let mut properties = node.properties.iter().collect::<Vec<_>>();
        properties.sort_by(|a, b| a.0.cmp(b.0));
        for (name, expr) in properties {
            clauses.push(format!("{name} {expr}"));
        }

        if clauses.is_empty() {
            self.line_with_trailing(&format!("{line}:"));
        } else {
            self.line_with_trailing(&format!("{line}:"));
            self.indented(|formatter| {
                for clause in clauses {
                    formatter.line(&clause);
                }
            });
        }
    }

    pub(crate) fn emit_define(&mut self, node: &Define) {
        let name = if let Some(index) = &node.index {
            format!("{}[{index}]", node.name)
        } else {
            node.name.clone()
        };

        if node.store == "store" {
            self.line_with_trailing(&format!("define {name} {} {}", node.operator, node.expr));
        } else {
            self.line_with_trailing(&format!(
                "define {}.{} {} {}",
                node.store.trim_start_matches("store."),
                node.name,
                node.operator,
                node.expr
            ));
        }
    }

    pub(crate) fn emit_default(&mut self, node: &Default_) {
        if node.store == "store" {
            self.line_with_trailing(&format!(
                "default {} = {}",
                node.name,
                node.expr.as_deref().unwrap_or("None")
            ));
        } else {
            self.line_with_trailing(&format!(
                "default {}.{} = {}",
                node.store.trim_start_matches("store."),
                node.name,
                node.expr.as_deref().unwrap_or("None")
            ));
        }
    }

    pub(crate) fn emit_call(&mut self, node: &Call) {
        let label = if let Some(global_label) = &node.global_label {
            format!("{global_label}.{}", node.label)
        } else {
            node.label.clone()
        };

        let mut line = if node.expression {
            format!("call expression {label}")
        } else {
            format!("call {label}")
        };

        if let Some(arguments) = &node.arguments {
            line.push_str(&format_argument_info(arguments));
        }

        self.line_with_trailing(&line);
    }

    pub(crate) fn emit_pass(&mut self, _node: &Pass) {
        self.line_with_trailing("pass");
    }

    pub(crate) fn emit_transform(&mut self, node: &Transform) {
        let mut line = if node.store == "store" {
            format!("transform {}", node.name)
        } else {
            format!(
                "transform {}.{}",
                node.store.trim_start_matches("store."),
                node.name
            )
        };

        if let Some(parameters) = &node.parameters {
            line.push_str(&format_parameter_signature(parameters));
        }

        self.line_with_trailing(&format!("{line}:"));
        if let Some(atl) = &node.atl {
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        }
    }

    pub(crate) fn emit_show_layer(&mut self, node: &ShowLayer) {
        let mut line = format!("show layer {}", node.layer);
        if !node.at_list.is_empty() {
            line.push_str(&format!(" at {}", node.at_list.join(", ")));
        }

        if let Some(atl) = &node.atl {
            self.line_with_trailing(&format!("{line}:"));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line_with_trailing(&line);
        }
    }

    pub(crate) fn emit_camera(&mut self, node: &Camera) {
        let mut parts = vec![String::from("camera")];
        if !node.layer.is_empty() && node.layer != "master" {
            parts.push(node.layer.clone());
        }
        if !node.at_list.is_empty() {
            parts.push(format!("at {}", node.at_list.join(", ")));
        }
        let line = parts.join(" ");

        if let Some(atl) = &node.atl {
            self.line_with_trailing(&format!("{line}:"));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line_with_trailing(&line);
        }
    }

    pub(crate) fn emit_image(&mut self, node: &Image) {
        let line = format!("image {}", node.name.join(" "));
        if let Some(expr) = &node.expr {
            self.line_with_trailing(&format!("{line} = {expr}"));
        } else if let Some(atl) = &node.atl {
            self.line_with_trailing(&format!("{line}:"));
            self.indented(|formatter| {
                formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                    formatter.emit_atl_block(atl)
                });
            });
        } else {
            self.line_with_trailing(&line);
        }
    }

    pub(crate) fn emit_layered_image(&mut self, node: &LayeredImage) {
        self.line_with_trailing(&format!("layeredimage {}:", node.name.join(" ")));
        self.indented(|formatter| {
            for property in &node.properties {
                formatter.emit_layered_image_property(property);
            }

            for child in &node.children {
                match child {
                    LayeredImageChild::Attribute(attribute) => {
                        formatter.emit_layered_image_attribute(attribute)
                    }
                    LayeredImageChild::Group(group) => formatter.emit_layered_image_group(group),
                    LayeredImageChild::ConditionGroup(group) => {
                        formatter.emit_layered_image_condition_group(group)
                    }
                    LayeredImageChild::Always(always) => {
                        formatter.emit_layered_image_always(always)
                    }
                    LayeredImageChild::Pass => formatter.line("pass"),
                }
            }
        });
    }

    fn emit_layered_image_property(&mut self, property: &LayeredImageProperty) {
        match &property.value {
            LayeredImagePropertyValue::Flag => self.line(&property.name),
            LayeredImagePropertyValue::Expression(expr) => {
                self.line(&format!("{} {expr}", property.name))
            }
            LayeredImagePropertyValue::AtlTransform(block) => {
                self.line(&format!("{} transform:", property.name));
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(block)
                    });
                });
            }
        }
    }

    fn emit_layered_image_displayable(&mut self, displayable: &LayeredImageDisplayable) {
        match displayable {
            LayeredImageDisplayable::Expression(expr) => self.line(expr),
            LayeredImageDisplayable::Null => self.line("null"),
            LayeredImageDisplayable::Atl(block) => {
                self.line("image:");
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(block)
                    });
                });
            }
        }
    }

    fn emit_layered_image_attribute(&mut self, attribute: &crate::ast::LayeredImageAttribute) {
        if attribute.properties.is_empty() && attribute.displayable.is_none() {
            self.line(&format!("attribute {}", attribute.name));
            return;
        }

        self.line(&format!("attribute {}:", attribute.name));
        self.indented(|formatter| {
            for property in &attribute.properties {
                formatter.emit_layered_image_property(property);
            }
            if let Some(displayable) = &attribute.displayable {
                formatter.emit_layered_image_displayable(displayable);
            }
        });
    }

    fn emit_layered_image_group(&mut self, group: &crate::ast::LayeredImageGroup) {
        let mut line = String::from("group");
        if let Some(name) = &group.name {
            line.push(' ');
            line.push_str(name);
        } else {
            line.push_str(" multiple");
        }
        line.push(':');
        self.line(&line);
        self.indented(|formatter| {
            for property in &group.properties {
                formatter.emit_layered_image_property(property);
            }
            for attribute in &group.attributes {
                formatter.emit_layered_image_attribute(attribute);
            }
        });
    }

    fn emit_layered_image_condition_group(
        &mut self,
        group: &crate::ast::LayeredImageConditionGroup,
    ) {
        for branch in &group.branches {
            let mut line = branch.branch.clone();
            if let Some(condition) = &branch.condition {
                line.push(' ');
                line.push_str(condition);
            }
            line.push(':');
            self.line(&line);
            self.indented(|formatter| {
                for property in &branch.properties {
                    formatter.emit_layered_image_property(property);
                }
                if let Some(displayable) = &branch.displayable {
                    formatter.emit_layered_image_displayable(displayable);
                }
            });
        }
    }

    fn emit_layered_image_always(&mut self, always: &crate::ast::LayeredImageAlways) {
        if always.properties.is_empty()
            && matches!(
                always.displayable,
                Some(LayeredImageDisplayable::Expression(_)) | Some(LayeredImageDisplayable::Null)
            )
        {
            match always.displayable.as_ref().expect("checked above") {
                LayeredImageDisplayable::Expression(expr) => {
                    self.line(&format!("always {expr}"));
                }
                LayeredImageDisplayable::Null => self.line("always null"),
                LayeredImageDisplayable::Atl(_) => unreachable!(),
            }
            return;
        }

        self.line("always:");
        self.indented(|formatter| {
            for property in &always.properties {
                formatter.emit_layered_image_property(property);
            }
            if let Some(displayable) = &always.displayable {
                formatter.emit_layered_image_displayable(displayable);
            }
        });
    }

    pub(crate) fn emit_rpy(&mut self, node: &RPY) {
        self.line_with_trailing(&format!("rpy {}", node.rest.join(" ")));
    }

    pub(crate) fn emit_translate(&mut self, node: &Translate) {
        let language = node.language.as_deref().unwrap_or("None");
        self.line_with_trailing(&format!("translate {language} {}:", node.identifier));
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_end_translate(&mut self, _node: &EndTranslate) {}

    pub(crate) fn emit_translate_string(&mut self, node: &TranslateString) {
        self.line_with_trailing(&format!("old {}", node.old));
        self.line_with_trailing(&format!("new {}", node.new));
    }

    pub(crate) fn emit_translate_block(&mut self, node: &TranslateBlock) {
        let language = node.language.as_deref().unwrap_or("None");

        if node.block.len() == 1 {
            if let crate::ast::AstNode::Style(style) = &node.block[0] {
                let mut line = format!("translate {language} style {}", style.name);
                if let Some(parent) = &style.parent {
                    line.push_str(&format!(" is {parent}"));
                }
                line.push(':');
                self.line_with_trailing(&line);

                self.indented(|formatter| {
                    let mut clauses = vec![];
                    if style.clear {
                        clauses.push(String::from("clear"));
                    }
                    if let Some(take) = &style.take {
                        clauses.push(format!("take {take}"));
                    }
                    for delattr in &style.delattr {
                        clauses.push(format!("del {delattr}"));
                    }
                    if let Some(variant) = &style.variant {
                        clauses.push(format!("variant {variant}"));
                    }

                    let mut properties = style.properties.iter().collect::<Vec<_>>();
                    properties.sort_by(|a, b| a.0.cmp(b.0));
                    for (name, expr) in properties {
                        clauses.push(format!("{name} {expr}"));
                    }

                    for clause in clauses {
                        formatter.line(&clause);
                    }
                });
                return;
            }
        }

        self.line_with_trailing(&format!("translate {language}:"));
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_translate_early_block(&mut self, node: &TranslateEarlyBlock) {
        let language = node.language.as_deref().unwrap_or("None");

        if node.block.len() == 1 {
            if let crate::ast::AstNode::Python(python) = &node.block[0] {
                let formatted_code =
                    self.format_python_block_source(&python.python_code, python.loc.1);
                self.line_with_trailing(&format!("translate {language} python:"));
                self.indented(|formatter| {
                    for code_line in formatted_code.lines() {
                        formatter.line(code_line);
                    }
                });
                return;
            }
        }

        self.line_with_trailing(&format!("translate {language}:"));
        self.indented(|formatter| formatter.nodes(&node.block));
    }

    pub(crate) fn emit_python(&mut self, node: &Python) {
        self.emit_python_block(node, false);
    }

    pub(crate) fn emit_early_python(&mut self, node: &EarlyPython) {
        self.emit_python_block(node, true);
    }

    fn emit_init_python_block(
        &mut self,
        priority: isize,
        node: &impl PythonBlockLike,
        early: bool,
    ) {
        let mut line = String::from("init");
        if priority != 0 {
            line.push_str(&format!(" {priority}"));
        }
        line.push(' ');
        line.push_str(&self.python_block_header(node, early));
        let formatted_code =
            self.format_python_block_source(node.python_code(), node.line_number());

        self.line_with_trailing(&line);
        self.indented(|formatter| {
            for code_line in formatted_code.lines() {
                formatter.line(code_line);
            }
        });
    }

    fn emit_python_block(&mut self, node: &impl PythonBlockLike, early: bool) {
        let line = self.python_block_header(node, early);
        let formatted_code =
            self.format_python_block_source(node.python_code(), node.line_number());

        self.line_with_trailing(&line);
        self.indented(|formatter| {
            for code_line in formatted_code.lines() {
                formatter.line(code_line);
            }
        });
    }

    fn python_block_header(&self, node: &impl PythonBlockLike, early: bool) -> String {
        let mut line = String::from("python");
        if early {
            line.push_str(" early");
        }
        if node.hide() {
            line.push_str(" hide");
        }
        if !node.store().is_empty() && node.store() != "store" {
            line.push_str(&format!(
                " in {}",
                node.store().trim_start_matches("store.")
            ));
        }
        line.push(':');

        line
    }
}

trait PythonBlockLike {
    fn python_code(&self) -> &str;
    fn store(&self) -> &str;
    fn hide(&self) -> bool;
    fn line_number(&self) -> usize;
}

impl PythonBlockLike for Python {
    fn python_code(&self) -> &str {
        &self.python_code
    }

    fn store(&self) -> &str {
        &self.store
    }

    fn hide(&self) -> bool {
        self.hide
    }

    fn line_number(&self) -> usize {
        self.loc.1
    }
}

impl PythonBlockLike for EarlyPython {
    fn python_code(&self) -> &str {
        &self.python_code
    }

    fn store(&self) -> &str {
        &self.store
    }

    fn hide(&self) -> bool {
        self.hide
    }

    fn line_number(&self) -> usize {
        self.loc.1
    }
}
