use crate::atl::{AtlStatement, RawBlock};

use super::core::{Formatter, Mode};

impl Formatter {
    pub(crate) fn emit_atl_block(&mut self, block: &RawBlock) {
        if block.animation {
            self.line("animation");
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
                    self.line(&format!("repeat {repeats}"));
                } else {
                    self.line("repeat");
                }
            }
            AtlStatement::RawBlock(node) => {
                self.with_mode(Mode::AtlNestedBlock, |formatter| {
                    formatter.emit_atl_block(node)
                });
            }
            AtlStatement::RawContainsExpr(node) => self.line(&format!("contains {}", node.expr)),
            AtlStatement::RawChild(node) => {
                self.line("contains:");
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(&node.child)
                    });
                });
            }
            AtlStatement::RawParallel(node) => {
                self.line("parallel:");
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(&node.block)
                    });
                });
            }
            AtlStatement::RawChoice(node) => {
                if node.chance.is_empty() {
                    self.line("choice:");
                } else {
                    self.line(&format!("choice {}:", node.chance));
                }
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(&node.block)
                    });
                });
            }
            AtlStatement::RawOn(node) => {
                self.line(&format!("on {}:", node.names.join(", ")));
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        formatter.emit_atl_block(&node.block)
                    });
                });
            }
            AtlStatement::RawTime(node) => self.line(&format!("time {}", node.time)),
            AtlStatement::RawFunction(node) => self.line(&format!("function {}", node.expr)),
            AtlStatement::RawEvent(node) => self.line(&format!("event {}", node.name)),
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

                self.line(&parts.join(" "));
            }
        }
    }
}
