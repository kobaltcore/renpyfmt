use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::{Position, Range, Url};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentLanguage {
    Renpy,
    Python,
    Unknown,
}

impl DocumentLanguage {
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rpy") => Self::Renpy,
            Some("py") => Self::Python,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceDocument {
    pub uri: Option<Url>,
    pub path: PathBuf,
    pub text: String,
    pub line_index: LineIndex,
}

impl SourceDocument {
    pub fn new(uri: Option<Url>, path: PathBuf, text: String) -> Self {
        let line_index = LineIndex::new(&text);
        Self {
            uri,
            path,
            text,
            line_index,
        }
    }

    pub fn language(&self) -> DocumentLanguage {
        DocumentLanguage::from_path(&self.path)
    }

    pub fn file_url(&self) -> Option<Url> {
        self.uri
            .clone()
            .or_else(|| Url::from_file_path(&self.path).ok())
    }
}

#[derive(Debug, Clone)]
pub struct LineIndex {
    line_starts: Vec<usize>,
    text_len: usize,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (offset, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(offset + 1);
            }
        }
        Self {
            line_starts,
            text_len: text.len(),
        }
    }

    pub fn offset_to_position(&self, offset: usize) -> Position {
        let offset = offset.min(self.text_len);
        let line = self
            .line_starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line];
        Position::new(line as u32, (offset - line_start) as u32)
    }

    pub fn position_to_offset(&self, position: Position) -> Option<usize> {
        let line = usize::try_from(position.line).ok()?;
        let column = usize::try_from(position.character).ok()?;
        let line_start = *self.line_starts.get(line)?;
        let line_end = self.line_end_offset(line);
        let offset = line_start.saturating_add(column);
        (offset <= line_end).then_some(offset)
    }

    pub fn range_from_offsets(&self, start: usize, end: usize) -> Range {
        Range::new(self.offset_to_position(start), self.offset_to_position(end))
    }

    pub fn line_range(&self, line: usize) -> Range {
        let start = *self.line_starts.get(line).unwrap_or(&self.text_len);
        let end = self.line_end_offset(line);
        self.range_from_offsets(start, end)
    }

    pub fn one_based_line_range(&self, line: usize) -> Range {
        self.line_range(line.saturating_sub(1))
    }

    pub fn line_text<'a>(&self, text: &'a str, one_based_line: usize) -> Option<&'a str> {
        let zero_based = one_based_line.checked_sub(1)?;
        let start = *self.line_starts.get(zero_based)?;
        let end = self.line_end_offset(zero_based);
        text.get(start..end)
    }

    fn line_end_offset(&self, line: usize) -> usize {
        match self.line_starts.get(line + 1) {
            Some(next_start) => next_start.saturating_sub(1),
            None => self.text_len,
        }
    }
}
