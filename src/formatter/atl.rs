use crate::atl::{AtlStatement, RawBlock};

use super::core::{Formatter, Mode};

impl Formatter {
    pub(crate) fn emit_atl_block(&mut self, block: &RawBlock) {
        if block.animation {
            self.line_for_source("animation", block.loc.1);
        }

        match self.mode {
            Mode::AtlDirectChild => {
                for statement in &block.statements {
                    match statement {
                        Some(statement) => self.emit_atl_statement(statement),
                        None => self.line("pass"),
                    }
                }
            }
            Mode::AtlNestedBlock | Mode::Script => {
                self.line("block:");
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        for statement in &block.statements {
                            match statement {
                                Some(statement) => formatter.emit_atl_statement(statement),
                                None => formatter.line("pass"),
                            }
                        }
                    });
                });
            }
        }
    }

    pub(crate) fn emit_atl_statement(&mut self, statement: &AtlStatement) {
        match statement {
            AtlStatement::RawRepeat(node) => {
                if let Some(repeats) = &node.repeats {
                    self.line_for_source(&format!("repeat {repeats}"), node.loc.1);
                } else {
                    self.line_for_source("repeat", node.loc.1);
                }
            }
            AtlStatement::RawBlock(node) => {
                self.with_mode(Mode::AtlNestedBlock, |formatter| {
                    formatter.emit_atl_block(node)
                });
            }
            AtlStatement::RawContainsExpr(node) => {
                self.line_for_source(&format!("contains {}", node.expr), node.loc.1)
            }
            AtlStatement::RawChild(node) => {
                self.line_for_source("contains:", node.loc.1);
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(&node.child)
                    });
                });
            }
            AtlStatement::RawParallel(node) => {
                self.line_for_source("parallel:", node.loc.1);
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(&node.block)
                    });
                });
            }
            AtlStatement::RawChoice(node) => {
                if node.chance.is_empty() {
                    self.line_for_source("choice:", node.loc.1);
                } else {
                    self.line_for_source(&format!("choice {}:", node.chance), node.loc.1);
                }
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(&node.block)
                    });
                });
            }
            AtlStatement::RawOn(node) => {
                self.line_for_source(&format!("on {}:", node.names.join(", ")), node.loc.1);
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(&node.block)
                    });
                });
            }
            AtlStatement::RawTime(node) => {
                self.line_for_source(&format!("time {}", node.time), node.loc.1)
            }
            AtlStatement::RawFunction(node) => {
                self.line_for_source(&format!("function {}", node.expr), node.loc.1)
            }
            AtlStatement::RawEvent(node) => {
                self.line_for_source(&format!("event {}", node.name), node.loc.1)
            }
            AtlStatement::RawMultipurpose(node) => {
                let mut parts = vec![];

                if let Some(warper) = &node.warper {
                    parts.push(warper.clone());
                }

                if let Some(duration) = &node.duration {
                    parts.push(duration.clone());
                }

                for (name, with_clause) in &node.expressions {
                    if let Some(with_clause) = with_clause {
                        parts.push(format!("{name} with {with_clause}"));
                    } else {
                        parts.push(name.clone());
                    }
                }

                let mut properties = node.properties.clone();
                properties.sort_by(|a, b| a.0.cmp(&b.0));

                for (name, exprs) in properties {
                    parts.push(format!("{name} {exprs}"));
                }

                self.line_for_source(&parts.join(" "), node.loc.1);
            }
        }
    }
}
