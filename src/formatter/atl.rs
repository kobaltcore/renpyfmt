use crate::atl::{AtlStatement, RawBlock};

use super::core::{Formatter, Mode};

impl Formatter {
    pub(crate) fn emit_atl_block(&mut self, block: &RawBlock) {
        match self.mode {
            Mode::AtlDirectChild => {
                for statement in block.statements.iter().flatten() {
                    self.emit_atl_statement(statement);
                }
            }
            Mode::AtlNestedBlock | Mode::Script => {
                self.line("block:");
                self.indented(|formatter| {
                    formatter.with_mode(Mode::AtlDirectChild, |formatter| {
                        for statement in block.statements.iter().flatten() {
                            formatter.emit_atl_statement(statement);
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
            AtlStatement::RawContainsExpr(_node) => todo!("raw contains expr"),
            AtlStatement::RawChild(_node) => todo!("raw child"),
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
            AtlStatement::RawOn(_node) => todo!("raw on"),
            AtlStatement::RawTime(_node) => todo!("raw time"),
            AtlStatement::RawFunction(_node) => todo!("raw function"),
            AtlStatement::RawEvent(_node) => todo!("raw event"),
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
