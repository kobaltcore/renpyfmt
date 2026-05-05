use crate::{
    ast::{Testcase, Testsuite},
    testast::{
        TestCase, TestCondition, TestHook, TestNode, TestProperties, TestSelector, TestSuite,
        TestSuiteEntry, TestTarget,
    },
};

use super::{core::Formatter, inline::encode_say_string};

impl Formatter {
    pub(crate) fn emit_testcase(&mut self, node: &Testcase) {
        self.emit_test_case(&node.test, true);
    }

    pub(crate) fn emit_testsuite(&mut self, node: &Testsuite) {
        self.emit_test_suite(&node.suite, true);
    }

    fn emit_test_case(&mut self, node: &TestCase, trailing: bool) {
        if trailing {
            self.line_with_trailing(&format!("testcase {}:", node.name));
        } else {
            self.line(&format!("testcase {}:", node.name));
        }
        self.indented(|formatter| {
            formatter.emit_test_properties(&node.properties);
            formatter.emit_test_nodes(&node.statements);
        });
    }

    fn emit_test_suite(&mut self, node: &TestSuite, trailing: bool) {
        if trailing {
            self.line_with_trailing(&format!("testsuite {}:", node.name));
        } else {
            self.line(&format!("testsuite {}:", node.name));
        }
        self.indented(|formatter| {
            formatter.emit_test_properties(&node.properties);
            for entry in &node.entries {
                formatter.emit_test_suite_entry(entry);
            }
        });
    }

    fn emit_test_suite_entry(&mut self, entry: &TestSuiteEntry) {
        match entry {
            TestSuiteEntry::Hook(hook) => self.emit_test_hook(hook),
            TestSuiteEntry::TestCase(case) => self.emit_test_case(case, false),
            TestSuiteEntry::TestSuite(suite) => self.emit_test_suite(suite, false),
        }
    }

    fn emit_test_hook(&mut self, hook: &TestHook) {
        self.line(&format!("{}:", hook.kind.as_str()));
        self.indented(|formatter| {
            if let Some(xfail) = &hook.properties.xfail {
                formatter.line(&format!("xfail {xfail}"));
            }
            if hook.properties.depth_explicit {
                let depth = hook
                    .properties
                    .depth
                    .as_ref()
                    .expect("explicit depth should have a value");
                formatter.line(&format!("depth {depth}"));
            }
            formatter.emit_test_nodes(&hook.statements);
        });
    }

    fn emit_test_properties(&mut self, properties: &TestProperties) {
        if let Some(description) = &properties.description {
            self.line(&format!("description {description}"));
        }
        if let Some(enabled) = &properties.enabled {
            self.line(&format!("enabled {enabled}"));
        }
        if let Some(only) = &properties.only {
            self.line(&format!("only {only}"));
        }
        if let Some(xfail) = &properties.xfail {
            self.line(&format!("xfail {xfail}"));
        }
        for parameter in &properties.parameters {
            let names = if parameter.names.len() == 1 {
                parameter.names[0].clone()
            } else {
                format!("({})", parameter.names.join(", "))
            };
            self.line(&format!("parameter {names} = {}", parameter.values_expr));
        }
    }

    fn emit_test_nodes(&mut self, nodes: &[TestNode]) {
        for node in nodes {
            self.emit_test_node(node);
        }
    }

    fn emit_test_node(&mut self, node: &TestNode) {
        match node {
            TestNode::Exit(_) => self.line("exit"),
            TestNode::Pass(_) => self.line("pass"),
            TestNode::If(node) => {
                for (index, branch) in node.branches.iter().enumerate() {
                    let keyword = if index == 0 { "if" } else { "elif" };
                    self.line(&format!(
                        "{keyword} {}:",
                        self.format_test_condition(&branch.condition, 0)
                    ));
                    self.indented(|formatter| formatter.emit_test_nodes(&branch.block));
                }
                if let Some(block) = &node.else_block {
                    self.line("else:");
                    self.indented(|formatter| formatter.emit_test_nodes(block));
                }
            }
            TestNode::While(node) => {
                self.line(&format!(
                    "while {}:",
                    self.format_test_condition(&node.condition, 0)
                ));
                self.indented(|formatter| formatter.emit_test_nodes(&node.block));
            }
            TestNode::Advance(_) => self.line("advance"),
            TestNode::Click(node) => {
                let mut parts = vec!["click".to_string()];
                if let Some(selector) = &node.selector {
                    parts.push(self.format_test_selector(selector));
                }
                if let Some(button) = &node.button {
                    parts.push(format!("button {button}"));
                }
                if let Some(position) = &node.position {
                    parts.push(format!("pos {position}"));
                }
                if node.always {
                    parts.push("always".into());
                }
                self.line(&parts.join(" "));
            }
            TestNode::Drag(node) => {
                let mut parts = vec!["drag".to_string()];
                if let Some(button) = &node.button {
                    parts.push(format!("button {button}"));
                }
                if let Some(steps) = &node.steps {
                    parts.push(format!("steps {steps}"));
                }
                parts.push(self.format_test_target(&node.start));
                parts.push("to".into());
                parts.push(self.format_test_target(&node.end));
                self.line(&parts.join(" "));
            }
            TestNode::Keysym(node) => {
                let mut parts = vec!["keysym".to_string(), encode_say_string(&node.keysym)];
                if let Some(selector) = &node.selector {
                    parts.push(self.format_test_selector(selector));
                }
                if let Some(position) = &node.position {
                    parts.push(format!("pos {position}"));
                }
                self.line(&parts.join(" "));
            }
            TestNode::Move(node) => {
                let mut parts = vec!["move".to_string()];
                if let Some(selector) = &node.selector {
                    parts.push(self.format_test_selector(selector));
                }
                if let Some(position) = &node.position {
                    parts.push(format!("pos {position}"));
                }
                self.line(&parts.join(" "));
            }
            TestNode::Pause(node) => {
                if let Some(delay) = &node.delay {
                    self.line(&format!("pause {delay}"));
                } else {
                    self.line("pause");
                }
            }
            TestNode::Run(node) => self.line(&format!("run {}", node.expr)),
            TestNode::Scroll(node) => {
                let mut parts = vec!["scroll".to_string()];
                if let Some(selector) = &node.selector {
                    parts.push(self.format_test_selector(selector));
                }
                if let Some(amount) = &node.amount {
                    parts.push(format!("amount {amount}"));
                }
                if let Some(position) = &node.position {
                    parts.push(format!("pos {position}"));
                }
                self.line(&parts.join(" "));
            }
            TestNode::Skip(node) => {
                if node.fast {
                    self.line("skip fast");
                } else {
                    self.line("skip");
                }
            }
            TestNode::Type(node) => {
                let mut parts = vec!["type".to_string(), encode_say_string(&node.text)];
                if let Some(selector) = &node.selector {
                    parts.push(self.format_test_selector(selector));
                }
                if let Some(position) = &node.position {
                    parts.push(format!("pos {position}"));
                }
                self.line(&parts.join(" "));
            }
            TestNode::Assert(node) => {
                let mut line = format!("assert {}", self.format_test_condition(&node.condition, 0));
                if let Some(timeout) = &node.timeout {
                    line.push_str(&format!(" timeout {timeout}"));
                }
                if let Some(xfail) = &node.xfail {
                    line.push_str(&format!(" xfail {xfail}"));
                }
                self.line(&line);
            }
            TestNode::Screenshot(node) => {
                let mut line = format!("screenshot {}", node.name);
                if let Some(max_pixel_difference) = &node.max_pixel_difference {
                    line.push_str(&format!(" max_pixel_difference {max_pixel_difference}"));
                }
                if let Some(crop) = &node.crop {
                    line.push_str(&format!(" crop {crop}"));
                }
                self.line(&line);
            }
            TestNode::Python(node) => {
                if node.block {
                    let header = if node.hide { "python hide:" } else { "python:" };
                    let formatted = self.format_python_block_source(&node.code, node.loc.1);
                    self.line(header);
                    self.indented(|formatter| {
                        for code_line in formatted.lines() {
                            formatter.line(code_line);
                        }
                    });
                } else {
                    self.line(&format!("$ {}", node.code));
                }
            }
            TestNode::Until(node) => {
                let mut line = self.format_test_command(node.command.as_ref());
                line.push_str(&format!(
                    " until {}",
                    self.format_test_condition(&node.condition, 0)
                ));
                if let Some(timeout) = &node.timeout {
                    line.push_str(&format!(" timeout {timeout}"));
                }
                self.line(&line);
            }
            TestNode::Repeat(node) => {
                let mut line = self.format_test_command(node.command.as_ref());
                line.push_str(&format!(" repeat {}", node.count));
                if let Some(timeout) = &node.timeout {
                    line.push_str(&format!(" timeout {timeout}"));
                }
                self.line(&line);
            }
        }
    }

    fn format_test_command(&mut self, node: &TestNode) -> String {
        match node {
            TestNode::Exit(_) => "exit".into(),
            TestNode::Pass(_) => "pass".into(),
            TestNode::Advance(_) => "advance".into(),
            TestNode::Click(node) => {
                let mut parts = vec!["click".to_string()];
                if let Some(selector) = &node.selector {
                    parts.push(self.format_test_selector(selector));
                }
                if let Some(button) = &node.button {
                    parts.push(format!("button {button}"));
                }
                if let Some(position) = &node.position {
                    parts.push(format!("pos {position}"));
                }
                if node.always {
                    parts.push("always".into());
                }
                parts.join(" ")
            }
            TestNode::Drag(node) => {
                let mut parts = vec!["drag".to_string()];
                if let Some(button) = &node.button {
                    parts.push(format!("button {button}"));
                }
                if let Some(steps) = &node.steps {
                    parts.push(format!("steps {steps}"));
                }
                parts.push(self.format_test_target(&node.start));
                parts.push("to".into());
                parts.push(self.format_test_target(&node.end));
                parts.join(" ")
            }
            TestNode::Keysym(node) => {
                let mut parts = vec!["keysym".to_string(), encode_say_string(&node.keysym)];
                if let Some(selector) = &node.selector {
                    parts.push(self.format_test_selector(selector));
                }
                if let Some(position) = &node.position {
                    parts.push(format!("pos {position}"));
                }
                parts.join(" ")
            }
            TestNode::Move(node) => {
                let mut parts = vec!["move".to_string()];
                if let Some(selector) = &node.selector {
                    parts.push(self.format_test_selector(selector));
                }
                if let Some(position) = &node.position {
                    parts.push(format!("pos {position}"));
                }
                parts.join(" ")
            }
            TestNode::Pause(node) => match &node.delay {
                Some(delay) => format!("pause {delay}"),
                None => "pause".into(),
            },
            TestNode::Run(node) => format!("run {}", node.expr),
            TestNode::Scroll(node) => {
                let mut parts = vec!["scroll".to_string()];
                if let Some(selector) = &node.selector {
                    parts.push(self.format_test_selector(selector));
                }
                if let Some(amount) = &node.amount {
                    parts.push(format!("amount {amount}"));
                }
                if let Some(position) = &node.position {
                    parts.push(format!("pos {position}"));
                }
                parts.join(" ")
            }
            TestNode::Skip(node) => {
                if node.fast {
                    "skip fast".into()
                } else {
                    "skip".into()
                }
            }
            TestNode::Type(node) => {
                let mut parts = vec!["type".to_string(), encode_say_string(&node.text)];
                if let Some(selector) = &node.selector {
                    parts.push(self.format_test_selector(selector));
                }
                if let Some(position) = &node.position {
                    parts.push(format!("pos {position}"));
                }
                parts.join(" ")
            }
            TestNode::Assert(node) => {
                let mut line = format!("assert {}", self.format_test_condition(&node.condition, 0));
                if let Some(timeout) = &node.timeout {
                    line.push_str(&format!(" timeout {timeout}"));
                }
                if let Some(xfail) = &node.xfail {
                    line.push_str(&format!(" xfail {xfail}"));
                }
                line
            }
            TestNode::Screenshot(node) => {
                let mut line = format!("screenshot {}", node.name);
                if let Some(max_pixel_difference) = &node.max_pixel_difference {
                    line.push_str(&format!(" max_pixel_difference {max_pixel_difference}"));
                }
                if let Some(crop) = &node.crop {
                    line.push_str(&format!(" crop {crop}"));
                }
                line
            }
            TestNode::Python(node) => {
                if node.block {
                    if node.hide {
                        "python hide:".into()
                    } else {
                        "python:".into()
                    }
                } else {
                    format!("$ {}", node.code)
                }
            }
            TestNode::If(_) | TestNode::While(_) | TestNode::Until(_) | TestNode::Repeat(_) => {
                unreachable!()
            }
        }
    }

    fn format_test_target(&self, target: &TestTarget) -> String {
        let mut parts = vec![];
        if let Some(selector) = &target.selector {
            parts.push(self.format_test_selector(selector));
        }
        if let Some(position) = &target.position {
            parts.push(format!("pos {position}"));
        }
        parts.join(" ")
    }

    fn format_test_selector(&self, selector: &TestSelector) -> String {
        match selector {
            TestSelector::Text(selector) => {
                let mut parts = vec![];
                if selector.focused {
                    parts.push("focused".into());
                }
                if selector.raw {
                    parts.push("raw".into());
                }
                if selector.expression {
                    parts.push(format!("expression {}", selector.pattern));
                } else {
                    parts.push(encode_say_string(&selector.pattern));
                }
                parts.join(" ")
            }
            TestSelector::Displayable(selector) => {
                let mut parts = vec![];
                if let Some(screen) = &selector.screen {
                    parts.push(format!("screen {screen}"));
                }
                if let Some(id) = &selector.id {
                    parts.push(format!("id {id}"));
                }
                if let Some(layer) = &selector.layer {
                    parts.push(format!("layer {layer}"));
                }
                if selector.focused {
                    parts.push("focused".into());
                }
                parts.join(" ")
            }
        }
    }

    fn format_test_condition(&self, condition: &TestCondition, parent_precedence: u8) -> String {
        let precedence = self.test_condition_precedence(condition);
        let rendered = match condition {
            TestCondition::BoolLiteral { value, .. } => {
                if *value {
                    "True".into()
                } else {
                    "False".into()
                }
            }
            TestCondition::Eval { expr, .. } => format!("eval {expr}"),
            TestCondition::Label { name, .. } => format!("label {name}"),
            TestCondition::Selector(selector) => self.format_test_selector(selector),
            TestCondition::Not { right, .. } => {
                format!("not {}", self.format_test_condition(right, precedence))
            }
            TestCondition::And { left, right, .. } => format!(
                "{} and {}",
                self.format_test_condition(left, precedence),
                self.format_test_condition(right, precedence)
            ),
            TestCondition::Or { left, right, .. } => format!(
                "{} or {}",
                self.format_test_condition(left, precedence),
                self.format_test_condition(right, precedence)
            ),
            TestCondition::Grouped { inner, .. } => {
                format!("({})", self.format_test_condition(inner, 0))
            }
        };

        if !matches!(condition, TestCondition::Grouped { .. }) && precedence < parent_precedence {
            format!("({rendered})")
        } else {
            rendered
        }
    }

    fn test_condition_precedence(&self, condition: &TestCondition) -> u8 {
        match condition {
            TestCondition::Or { .. } => 1,
            TestCondition::And { .. } => 2,
            TestCondition::Not { .. } => 3,
            TestCondition::Grouped { .. } => 4,
            _ => 4,
        }
    }
}
