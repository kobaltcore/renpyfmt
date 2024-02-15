use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct Screen {
    pub loc: (PathBuf, usize),
}
